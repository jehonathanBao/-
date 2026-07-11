use std::{collections::BTreeSet, time::Duration};

use futures_util::StreamExt;
use serde::Deserialize;
use tokio::task::JoinHandle;
use tokio_tungstenite::connect_async;

use super::{
    config::binance_alt_contract_runtime_config,
    service::BinanceAltContractService,
    symbol_universe::{build_symbol_universe, BinanceAltSymbolCandidate},
    types::{
        AltContractExchange, AltContractTrade, AltContractTradeSide, AltLiquidationEvent,
        LiquidationSide,
    },
    LOG_PREFIX, LOG_TARGET,
};

const BINANCE_FUTURES_STREAM_BASE: &str = "wss://fstream.binance.com/market/stream?streams=";
const BINANCE_ALL_MARKET_CONTEXT_STREAM_URL: &str =
    "wss://fstream.binance.com/market/stream?streams=!markPrice@arr@1s/!ticker@arr";
const BINANCE_FORCE_ORDER_STREAM_URL: &str = "wss://fstream.binance.com/ws/!forceOrder@arr";
const BINANCE_FUTURES_REST_BASE: &str = "https://fapi.binance.com";
const RECONNECT_DELAY_MS: u64 = 1_000;
const MAX_STREAMS_PER_CONNECTION: usize = 200;

#[derive(Debug, Deserialize)]
struct Combined {
    data: serde_json::Value,
}

#[derive(Debug, Deserialize)]
pub struct BinanceFuturesAggTrade {
    #[serde(rename = "T")]
    pub trade_time_ms: i64,
    #[serde(rename = "s")]
    pub symbol: String,
    #[serde(rename = "a")]
    pub agg_trade_id: Option<i64>,
    #[serde(rename = "p")]
    pub price: String,
    #[serde(rename = "q")]
    pub qty: String,
    #[serde(rename = "m")]
    pub buyer_is_market_maker: bool,
}

#[derive(Debug, Deserialize)]
struct BinanceOpenInterest {
    #[serde(rename = "symbol")]
    symbol: String,
    #[serde(rename = "openInterest")]
    open_interest: String,
    #[serde(rename = "time")]
    time_ms: Option<i64>,
}

#[derive(Debug, Deserialize)]
struct BinancePremiumIndex {
    #[serde(rename = "symbol")]
    symbol: String,
    #[serde(rename = "lastFundingRate")]
    last_funding_rate: String,
}

#[derive(Debug, Deserialize)]
struct BinanceExchangeInfo {
    symbols: Vec<BinanceExchangeInfoSymbol>,
}

#[derive(Debug, Deserialize)]
struct BinanceExchangeInfoSymbol {
    symbol: String,
    #[serde(rename = "quoteAsset")]
    quote_asset: String,
    #[serde(rename = "contractType")]
    contract_type: String,
    status: String,
}

#[derive(Debug, Deserialize)]
struct BinanceTicker24h {
    symbol: String,
    #[serde(rename = "quoteVolume")]
    quote_volume: String,
}

#[derive(Debug, Deserialize)]
struct BinanceMarkPriceEvent {
    #[serde(rename = "E")]
    event_time_ms: Option<i64>,
    #[serde(rename = "s")]
    symbol: String,
    #[serde(rename = "p")]
    mark_price: String,
    #[serde(rename = "r")]
    funding_rate: Option<String>,
}

#[derive(Debug, Deserialize)]
struct BinanceTickerEvent {
    #[serde(rename = "E")]
    event_time_ms: Option<i64>,
    #[serde(rename = "s")]
    symbol: String,
    #[serde(rename = "c")]
    last_price: String,
    #[serde(rename = "q")]
    quote_volume: String,
    #[serde(rename = "P")]
    price_change_percent: Option<String>,
}

#[derive(Debug, Deserialize)]
struct BinanceForceOrderEvent {
    #[serde(rename = "o")]
    order: BinanceForceOrder,
}

#[derive(Debug, Deserialize)]
struct BinanceForceOrder {
    #[serde(rename = "s")]
    symbol: String,
    #[serde(rename = "p")]
    price: String,
    #[serde(rename = "ap")]
    avg_price: String,
    #[serde(rename = "q")]
    qty: String,
    #[serde(rename = "l")]
    last_filled_qty: Option<String>,
    #[serde(rename = "z")]
    accumulated_filled_qty: Option<String>,
    #[serde(rename = "T")]
    trade_time_ms: Option<i64>,
    #[serde(rename = "S")]
    side: Option<String>,
    #[serde(rename = "i")]
    order_id: Option<i64>,
}

pub async fn run(service: BinanceAltContractService) {
    let client = reqwest::Client::new();
    loop {
        let config = binance_alt_contract_runtime_config();
        if !config.enabled || !config.exchange.binance_enabled {
            service.set_exchange_status(AltContractExchange::Binance, "disabled", false, None);
            tokio::time::sleep(Duration::from_millis(RECONNECT_DELAY_MS)).await;
            continue;
        }
        let symbols = match refresh_symbol_universe(&client, &service).await {
            Ok(symbols) if !symbols.is_empty() => symbols,
            Ok(_) => config.enabled_symbols(),
            Err(error) => {
                service.record_error();
                tracing::warn!(
                    target: LOG_TARGET,
                    "{} binance alt universe refresh failed: {error}",
                    LOG_PREFIX
                );
                config.enabled_symbols()
            }
        };
        if symbols.is_empty() {
            service.set_exchange_status(
                AltContractExchange::Binance,
                "disabled",
                false,
                Some("no alt symbols configured".to_string()),
            );
            tokio::time::sleep(Duration::from_millis(RECONNECT_DELAY_MS)).await;
            continue;
        }
        let shards = shard_symbols(&symbols, MAX_STREAMS_PER_CONNECTION);
        tracing::info!(
            target: LOG_TARGET,
            symbol_count = symbols.len(),
            shard_count = shards.len(),
            "{} binance alt aggTrade shards starting",
            LOG_PREFIX
        );
        let mut active_shards = shards;
        service.begin_shard_supervision(active_shards.len());
        service.reconcile_shard_supervision(&active_shards);
        let mut shard_tasks = spawn_shard_tasks(&service, &active_shards);
        let universe_config = binance_alt_contract_runtime_config().universe;
        let mut refresh =
            tokio::time::interval(Duration::from_secs(universe_config.refresh_seconds.max(1)));
        let mut watchdog = tokio::time::interval(Duration::from_secs(5));
        refresh.tick().await;
        loop {
            tokio::select! {
                _ = refresh.tick(), if universe_config.dynamic_reconcile_enabled => {
                    match refresh_symbol_universe(&client, &service).await {
                        Ok(next_symbols) if !next_symbols.is_empty() => {
                            let next_shards = reconcile_shard_layout(&active_shards, &next_symbols);
                            if next_shards == active_shards {
                                continue;
                            }
                            tracing::info!(
                                target: LOG_TARGET,
                                previous_symbol_count = active_shards.iter().map(Vec::len).sum::<usize>(),
                                next_symbol_count = next_symbols.len(),
                                "{} binance alt universe changed; reconciling affected shards",
                                LOG_PREFIX
                            );
                            reconcile_shard_tasks(
                                &service,
                                &mut shard_tasks,
                                &active_shards,
                                &next_shards,
                            )
                            .await;
                            active_shards = next_shards;
                        }
                        Ok(_) => {}
                        Err(error) => {
                            service.record_error();
                            tracing::warn!(target: LOG_TARGET, "{} binance alt universe refresh failed: {error}", LOG_PREFIX);
                        }
                    }
                }
                _ = watchdog.tick() => {
                    restart_finished_shards(&service, &active_shards, &mut shard_tasks).await;
                }
            }
            if shard_tasks.is_empty() {
                tracing::warn!(target: LOG_TARGET, "{} all aggTrade shards stopped; retrying supervision", LOG_PREFIX);
                break;
            }
        }
        for (_, task) in shard_tasks {
            task.1.abort();
        }
        tokio::time::sleep(Duration::from_millis(RECONNECT_DELAY_MS)).await;
    }
}

fn spawn_shard_tasks(
    service: &BinanceAltContractService,
    shards: &[Vec<String>],
) -> std::collections::BTreeMap<usize, (Vec<String>, JoinHandle<()>)> {
    let total_shards = shards.iter().filter(|symbols| !symbols.is_empty()).count();
    shards
        .iter()
        .enumerate()
        .filter(|(_, symbols)| !symbols.is_empty())
        .map(|(shard_id, symbols)| {
            let shard_service = service.clone();
            let shard_symbols = symbols.clone();
            let task = tokio::spawn(async move {
                run_agg_trade_shard(shard_service, shard_id, total_shards, shard_symbols).await;
            });
            (shard_id, (symbols.clone(), task))
        })
        .collect()
}

async fn reconcile_shard_tasks(
    service: &BinanceAltContractService,
    tasks: &mut std::collections::BTreeMap<usize, (Vec<String>, JoinHandle<()>)>,
    current: &[Vec<String>],
    desired: &[Vec<String>],
) {
    let max_len = current.len().max(desired.len());
    for shard_id in 0..max_len {
        let desired_symbols = desired.get(shard_id).cloned().unwrap_or_default();
        let needs_restart = tasks
            .get(&shard_id)
            .map(|(symbols, task)| symbols != &desired_symbols || task.is_finished())
            .unwrap_or(!desired_symbols.is_empty());
        if !needs_restart {
            continue;
        }
        if let Some((_, task)) = tasks.remove(&shard_id) {
            task.abort();
        }
        if !desired_symbols.is_empty() {
            service.update_shard_status(shard_id, desired.len(), false);
            let shard_service = service.clone();
            let task_symbols = desired_symbols.clone();
            let total_shards = desired.iter().filter(|items| !items.is_empty()).count();
            let task = tokio::spawn(async move {
                run_agg_trade_shard(shard_service, shard_id, total_shards, task_symbols).await;
            });
            tasks.insert(shard_id, (desired_symbols, task));
        }
    }
    tasks.retain(|shard_id, _| {
        desired
            .get(*shard_id)
            .is_some_and(|symbols| !symbols.is_empty())
    });
    service.reconcile_shard_supervision(desired);
}

async fn restart_finished_shards(
    service: &BinanceAltContractService,
    shards: &[Vec<String>],
    tasks: &mut std::collections::BTreeMap<usize, (Vec<String>, JoinHandle<()>)>,
) {
    let finished = tasks
        .iter()
        .filter(|(_, (_, task))| task.is_finished())
        .map(|(shard_id, _)| *shard_id)
        .collect::<Vec<_>>();
    for shard_id in finished {
        if let Some((symbols, _)) = tasks.remove(&shard_id) {
            let shard_service = service.clone();
            let task_symbols = symbols.clone();
            let total_shards = shards.iter().filter(|items| !items.is_empty()).count();
            let task = tokio::spawn(async move {
                run_agg_trade_shard(shard_service, shard_id, total_shards, task_symbols).await;
            });
            tasks.insert(shard_id, (symbols, task));
        }
    }
}

pub fn reconcile_shard_layout(current: &[Vec<String>], desired: &[String]) -> Vec<Vec<String>> {
    let mut normalized = desired
        .iter()
        .map(|symbol| symbol.to_ascii_uppercase())
        .collect::<Vec<_>>();
    normalized.sort();
    normalized.dedup();
    let desired_set = normalized.iter().cloned().collect::<BTreeSet<_>>();
    let mut shards = current
        .iter()
        .map(|symbols| {
            let mut retained = symbols
                .iter()
                .map(|symbol| symbol.to_ascii_uppercase())
                .filter(|symbol| desired_set.contains(symbol))
                .collect::<Vec<_>>();
            retained.sort();
            retained.dedup();
            retained
        })
        .collect::<Vec<_>>();
    let assigned = shards.iter().flatten().cloned().collect::<BTreeSet<_>>();
    for symbol in normalized
        .into_iter()
        .filter(|symbol| !assigned.contains(symbol))
    {
        if let Some(shard) = shards
            .iter_mut()
            .find(|shard| shard.len() < MAX_STREAMS_PER_CONNECTION)
        {
            shard.push(symbol);
        } else {
            shards.push(vec![symbol]);
        }
    }
    shards.retain(|shard| !shard.is_empty());
    shards
}

pub fn universe_changed(current: &[String], desired: &[String]) -> bool {
    let normalized = |symbols: &[String]| {
        let mut items = symbols
            .iter()
            .map(|symbol| symbol.to_ascii_uppercase())
            .collect::<Vec<_>>();
        items.sort();
        items.dedup();
        items
    };
    normalized(current) != normalized(desired)
}

fn reconnect_delay(attempt: u32) -> Duration {
    let exponent = attempt.saturating_sub(1).min(5);
    let base_ms = RECONNECT_DELAY_MS
        .saturating_mul(1_u64 << exponent)
        .min(30_000);
    let jitter_ms = base_ms / 5;
    let delay_ms = if attempt.is_multiple_of(2) {
        base_ms.saturating_add(jitter_ms)
    } else {
        base_ms.saturating_sub(jitter_ms)
    };
    Duration::from_millis(delay_ms)
}

async fn run_agg_trade_shard(
    service: BinanceAltContractService,
    shard_id: usize,
    total_shards: usize,
    symbols: Vec<String>,
) {
    let mut reconnect_attempt = 0_u32;
    loop {
        let url = stream_url(&symbols);
        service.update_shard_status(shard_id, total_shards, false);
        service.set_exchange_status(AltContractExchange::Binance, "connecting", false, None);
        match connect_async(&url).await {
            Ok((ws, _)) => {
                reconnect_attempt = 0;
                tracing::info!(
                    target: LOG_TARGET,
                    shard_id,
                    stream_count = symbols.len(),
                    "{} binance alt futures shard connected",
                    LOG_PREFIX
                );
                service.mark_connected(AltContractExchange::Binance);
                service.update_shard_status(shard_id, total_shards, true);
                let (_, mut read) = ws.split();
                while let Some(message) = read.next().await {
                    match message {
                        Ok(message) => {
                            if let Ok(text) = message.to_text() {
                                handle_message(text, &service);
                            }
                        }
                        Err(error) => {
                            service.update_shard_status(shard_id, total_shards, false);
                            service.mark_reconnecting(
                                AltContractExchange::Binance,
                                Some(error.to_string()),
                            );
                            break;
                        }
                    }
                }
            }
            Err(error) => {
                reconnect_attempt = reconnect_attempt.saturating_add(1);
                service.update_shard_status(shard_id, total_shards, false);
                service.mark_reconnecting(AltContractExchange::Binance, Some(error.to_string()));
            }
        }
        tokio::time::sleep(reconnect_delay(reconnect_attempt)).await;
    }
}

pub async fn run_context_polling(service: BinanceAltContractService) {
    let client = reqwest::Client::new();
    let mut last_all_poll_at = 0_i64;
    loop {
        let config = binance_alt_contract_runtime_config();
        let hot_interval = Duration::from_secs(config.oi_scheduler.hot_symbols_interval_sec.max(1));
        if !config.enabled || !config.exchange.binance_enabled || !config.oi_scheduler.enabled {
            tokio::time::sleep(hot_interval).await;
            continue;
        }
        let throttle = Duration::from_millis(
            1_000_u64.saturating_div(config.oi_scheduler.max_oi_requests_per_sec.max(1)),
        );
        let now = crate::normalizers::trade::now_ms();
        let all_interval_ms = i64::try_from(config.oi_scheduler.all_symbols_interval_sec)
            .unwrap_or(300)
            .saturating_mul(1000);
        let all_due =
            last_all_poll_at == 0 || now.saturating_sub(last_all_poll_at) >= all_interval_ms;
        let symbols = if all_due {
            last_all_poll_at = now;
            service.monitored_product_ids()
        } else {
            service.hot_oi_product_ids()
        };
        for symbol in symbols {
            if let Err(error) = poll_symbol_context(&client, &service, &symbol).await {
                service.record_error();
                tracing::warn!(
                    target: LOG_TARGET,
                    symbol = symbol.as_str(),
                    "{} binance alt context poll skipped: {error}",
                    LOG_PREFIX
                );
            }
            tokio::time::sleep(throttle).await;
        }
        tokio::time::sleep(hot_interval).await;
    }
}

pub async fn run_all_market_context_stream(service: BinanceAltContractService) {
    let mut reconnect_attempt = 0_u32;
    loop {
        let config = binance_alt_contract_runtime_config();
        if !config.enabled || !config.exchange.binance_enabled {
            tokio::time::sleep(Duration::from_millis(RECONNECT_DELAY_MS)).await;
            continue;
        }
        match connect_async(BINANCE_ALL_MARKET_CONTEXT_STREAM_URL).await {
            Ok((ws, _)) => {
                reconnect_attempt = 0;
                tracing::info!(
                    target: LOG_TARGET,
                    "{} binance alt all-market markPrice/ticker stream connected",
                    LOG_PREFIX
                );
                service.mark_all_market_context_connected();
                let (_, mut read) = ws.split();
                while let Some(message) = read.next().await {
                    match message {
                        Ok(message) => {
                            if let Ok(text) = message.to_text() {
                                handle_all_market_context_message(text, &service);
                            }
                        }
                        Err(error) => {
                            service.mark_all_market_context_disconnected(Some(error.to_string()));
                            tracing::warn!(
                                target: LOG_TARGET,
                                "{} binance alt all-market context reconnecting: {error}",
                                LOG_PREFIX
                            );
                            break;
                        }
                    }
                }
            }
            Err(error) => {
                reconnect_attempt = reconnect_attempt.saturating_add(1);
                service.mark_all_market_context_disconnected(Some(error.to_string()));
                tracing::warn!(
                    target: LOG_TARGET,
                    "{} binance alt all-market context connect failed: {error}",
                    LOG_PREFIX
                );
            }
        }
        tokio::time::sleep(reconnect_delay(reconnect_attempt)).await;
    }
}

pub async fn run_force_order_stream(service: BinanceAltContractService) {
    let mut reconnect_attempt = 0_u32;
    loop {
        let config = binance_alt_contract_runtime_config();
        if !config.enabled || !config.exchange.binance_enabled {
            tokio::time::sleep(Duration::from_millis(RECONNECT_DELAY_MS)).await;
            continue;
        }
        match connect_async(BINANCE_FORCE_ORDER_STREAM_URL).await {
            Ok((ws, _)) => {
                reconnect_attempt = 0;
                tracing::info!(
                    target: LOG_TARGET,
                    "{} binance alt forceOrder snapshot stream connected",
                    LOG_PREFIX
                );
                service.mark_force_order_stream_connected();
                let (_, mut read) = ws.split();
                while let Some(message) = read.next().await {
                    match message {
                        Ok(message) => {
                            if let Ok(text) = message.to_text() {
                                handle_force_order_message(text, &service);
                            }
                        }
                        Err(error) => {
                            service.mark_force_order_stream_disconnected(Some(error.to_string()));
                            tracing::warn!(
                                target: LOG_TARGET,
                                "{} binance alt forceOrder stream reconnecting: {error}",
                                LOG_PREFIX
                            );
                            break;
                        }
                    }
                }
            }
            Err(error) => {
                reconnect_attempt = reconnect_attempt.saturating_add(1);
                service.mark_force_order_stream_disconnected(Some(error.to_string()));
                tracing::warn!(
                    target: LOG_TARGET,
                    "{} binance alt forceOrder connect failed: {error}",
                    LOG_PREFIX
                );
            }
        }
        tokio::time::sleep(reconnect_delay(reconnect_attempt)).await;
    }
}

pub fn normalize_binance_futures_agg_trade(
    raw: BinanceFuturesAggTrade,
) -> Option<AltContractTrade> {
    let price = raw.price.parse::<f64>().ok()?;
    let qty_base = raw.qty.parse::<f64>().ok()?;
    if !price.is_finite() || !qty_base.is_finite() || price <= 0.0 || qty_base <= 0.0 {
        return None;
    }
    let product_id = raw.symbol.to_ascii_uppercase();
    let symbol = product_id.trim_end_matches("USDT").to_string();
    let side = if raw.buyer_is_market_maker {
        AltContractTradeSide::Sell
    } else {
        AltContractTradeSide::Buy
    };
    Some(AltContractTrade {
        ts: raw.trade_time_ms,
        exchange: AltContractExchange::Binance,
        symbol,
        product_id,
        price,
        qty_base,
        notional_usd: price * qty_base,
        side,
        trade_id: raw.agg_trade_id.map(|id| id.to_string()),
    })
}

pub fn stream_url(symbols: &[String]) -> String {
    let streams = symbols
        .iter()
        .map(|symbol| format!("{}@aggTrade", symbol.to_ascii_lowercase()))
        .collect::<Vec<_>>()
        .join("/");
    format!("{BINANCE_FUTURES_STREAM_BASE}{streams}")
}

pub fn shard_symbols(symbols: &[String], max_streams_per_connection: usize) -> Vec<Vec<String>> {
    let chunk_size = max_streams_per_connection.max(1);
    symbols
        .chunks(chunk_size)
        .map(|chunk| chunk.to_vec())
        .collect()
}

fn handle_message(text: &str, service: &BinanceAltContractService) {
    let Ok(value) = serde_json::from_str::<serde_json::Value>(text) else {
        service.set_exchange_status(
            AltContractExchange::Binance,
            "degraded",
            true,
            Some("binance alt json parse error".to_string()),
        );
        return;
    };
    let Some(payload) = trade_payload(value) else {
        return;
    };
    let Ok(raw) = serde_json::from_value::<BinanceFuturesAggTrade>(payload) else {
        service.set_exchange_status(
            AltContractExchange::Binance,
            "degraded",
            true,
            Some("binance alt trade schema error".to_string()),
        );
        return;
    };
    let Some(trade) = normalize_binance_futures_agg_trade(raw) else {
        return;
    };
    let config = binance_alt_contract_runtime_config();
    if service.product_enabled(&trade.product_id, &config) {
        service.mark_connected(AltContractExchange::Binance);
        service.ingest_live_trade(trade);
    }
}

fn handle_all_market_context_message(text: &str, service: &BinanceAltContractService) {
    let Ok(value) = serde_json::from_str::<serde_json::Value>(text) else {
        service.mark_all_market_context_disconnected(Some(
            "binance alt all-market json parse error".to_string(),
        ));
        return;
    };
    let config = binance_alt_contract_runtime_config();
    for payload in market_payloads(value) {
        let event_type = payload
            .get("e")
            .and_then(serde_json::Value::as_str)
            .unwrap_or_default();
        if event_type == "markPriceUpdate" || payload.get("r").is_some() {
            let Ok(event) = serde_json::from_value::<BinanceMarkPriceEvent>(payload) else {
                continue;
            };
            let product_id = event.symbol.to_ascii_uppercase();
            if !service.product_enabled(&product_id, &config) {
                continue;
            }
            service.update_mark_price_context(
                &product_id,
                event
                    .event_time_ms
                    .unwrap_or_else(crate::normalizers::trade::now_ms),
                parse_number(&event.mark_price),
                event.funding_rate.as_deref().and_then(parse_number),
            );
        } else if event_type == "24hrTicker" || payload.get("q").is_some() {
            let Ok(event) = serde_json::from_value::<BinanceTickerEvent>(payload) else {
                continue;
            };
            let product_id = event.symbol.to_ascii_uppercase();
            if !service.product_enabled(&product_id, &config) {
                continue;
            }
            service.update_ticker_context(
                &product_id,
                event
                    .event_time_ms
                    .unwrap_or_else(crate::normalizers::trade::now_ms),
                parse_number(&event.last_price),
                parse_number(&event.quote_volume),
                event.price_change_percent.as_deref().and_then(parse_number),
            );
        }
    }
}

async fn refresh_symbol_universe(
    client: &reqwest::Client,
    service: &BinanceAltContractService,
) -> Result<Vec<String>, reqwest::Error> {
    let config = binance_alt_contract_runtime_config();
    let exchange_info = client
        .get(format!("{BINANCE_FUTURES_REST_BASE}/fapi/v1/exchangeInfo"))
        .send()
        .await?
        .error_for_status()?
        .json::<BinanceExchangeInfo>()
        .await?;
    let tickers = client
        .get(format!("{BINANCE_FUTURES_REST_BASE}/fapi/v1/ticker/24hr"))
        .send()
        .await?
        .error_for_status()?
        .json::<Vec<BinanceTicker24h>>()
        .await?;
    let volumes = tickers
        .into_iter()
        .map(|ticker| {
            (
                ticker.symbol.to_ascii_uppercase(),
                ticker.quote_volume.parse::<f64>().unwrap_or_default(),
            )
        })
        .collect::<std::collections::BTreeMap<_, _>>();
    let candidates = exchange_info
        .symbols
        .into_iter()
        .map(|symbol| BinanceAltSymbolCandidate {
            quote_volume_24h_usd: volumes
                .get(&symbol.symbol.to_ascii_uppercase())
                .copied()
                .unwrap_or_default(),
            symbol: symbol.symbol,
            quote_asset: symbol.quote_asset,
            contract_type: symbol.contract_type,
            status: symbol.status,
        })
        .collect::<Vec<_>>();
    let metas = build_symbol_universe(&candidates, &config);
    let symbols = metas
        .iter()
        .map(|meta| meta.product_id.clone())
        .collect::<Vec<_>>();
    service.update_symbol_universe(metas);
    Ok(symbols)
}

async fn poll_symbol_context(
    client: &reqwest::Client,
    service: &BinanceAltContractService,
    symbol: &str,
) -> Result<(), reqwest::Error> {
    let open_interest = client
        .get(format!("{BINANCE_FUTURES_REST_BASE}/fapi/v1/openInterest"))
        .query(&[("symbol", symbol)])
        .send()
        .await?
        .error_for_status()?
        .json::<BinanceOpenInterest>()
        .await?;
    let oi = open_interest
        .open_interest
        .parse::<f64>()
        .ok()
        .filter(|value| value.is_finite() && *value > 0.0);
    if let Some(oi) = oi {
        service.update_open_interest(
            &open_interest.symbol,
            open_interest
                .time_ms
                .unwrap_or_else(crate::normalizers::trade::now_ms),
            oi,
        );
    }

    let premium = client
        .get(format!("{BINANCE_FUTURES_REST_BASE}/fapi/v1/premiumIndex"))
        .query(&[("symbol", symbol)])
        .send()
        .await?
        .error_for_status()?
        .json::<BinancePremiumIndex>()
        .await?;
    let funding_rate = premium.last_funding_rate.parse::<f64>().ok();
    service.update_funding_context(&premium.symbol, funding_rate);
    Ok(())
}

fn handle_force_order_message(text: &str, service: &BinanceAltContractService) {
    let Ok(value) = serde_json::from_str::<serde_json::Value>(text) else {
        return;
    };
    for payload in force_order_payloads(value) {
        let Ok(event) = serde_json::from_value::<BinanceForceOrderEvent>(payload) else {
            continue;
        };
        let product_id = event.order.symbol.to_ascii_uppercase();
        let config = binance_alt_contract_runtime_config();
        if !service.product_enabled(&product_id, &config) {
            continue;
        }
        let price = event
            .order
            .avg_price
            .parse::<f64>()
            .ok()
            .filter(|value| *value > 0.0)
            .or_else(|| event.order.price.parse::<f64>().ok())
            .unwrap_or_default();
        let qty = event
            .order
            .accumulated_filled_qty
            .as_deref()
            .and_then(|value| value.parse::<f64>().ok())
            .or_else(|| {
                event
                    .order
                    .last_filled_qty
                    .as_deref()
                    .and_then(|value| value.parse::<f64>().ok())
            })
            .or_else(|| event.order.qty.parse::<f64>().ok())
            .unwrap_or_default();
        let notional_usd = price * qty;
        if notional_usd.is_finite() && notional_usd > 0.0 {
            service.update_liquidation_event(AltLiquidationEvent {
                product_id,
                ts: event
                    .order
                    .trade_time_ms
                    .unwrap_or_else(crate::normalizers::trade::now_ms),
                side: match event.order.side.as_deref() {
                    Some("SELL") => LiquidationSide::LongLiquidation,
                    Some("BUY") => LiquidationSide::ShortLiquidation,
                    _ => LiquidationSide::Unknown,
                },
                notional_usd,
                price: (price > 0.0).then_some(price),
                quantity: (qty > 0.0).then_some(qty),
                source_event_id: event.order.order_id.map(|value| value.to_string()),
            });
        }
    }
}

fn trade_payload(value: serde_json::Value) -> Option<serde_json::Value> {
    if value.get("e").and_then(serde_json::Value::as_str) == Some("aggTrade") {
        return Some(value);
    }
    if let Ok(combined) = serde_json::from_value::<Combined>(value) {
        let is_agg_trade =
            combined.data.get("e").and_then(serde_json::Value::as_str) == Some("aggTrade");
        if is_agg_trade {
            return Some(combined.data);
        }
    }
    None
}

fn force_order_payloads(value: serde_json::Value) -> Vec<serde_json::Value> {
    if value.get("e").and_then(serde_json::Value::as_str) == Some("forceOrder") {
        return vec![value];
    }
    if let Some(data) = value.get("data") {
        if data.get("e").and_then(serde_json::Value::as_str) == Some("forceOrder") {
            return vec![data.clone()];
        }
        if let Some(items) = data.as_array() {
            return items
                .iter()
                .filter(|item| {
                    item.get("e").and_then(serde_json::Value::as_str) == Some("forceOrder")
                })
                .cloned()
                .collect();
        }
    }
    if let Some(items) = value.as_array() {
        return items
            .iter()
            .filter(|item| item.get("e").and_then(serde_json::Value::as_str) == Some("forceOrder"))
            .cloned()
            .collect();
    }
    Vec::new()
}

fn market_payloads(value: serde_json::Value) -> Vec<serde_json::Value> {
    if let Some(data) = value.get("data") {
        if let Some(items) = data.as_array() {
            return items.to_vec();
        }
        return vec![data.clone()];
    }
    if let Some(items) = value.as_array() {
        return items.to_vec();
    }
    vec![value]
}

fn parse_number(value: &str) -> Option<f64> {
    value
        .parse::<f64>()
        .ok()
        .filter(|number| number.is_finite())
}
