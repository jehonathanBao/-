use std::time::Duration;

use futures_util::StreamExt;
use serde::Deserialize;
use tokio_tungstenite::connect_async;

use super::{
    config::binance_alt_contract_runtime_config,
    service::BinanceAltContractService,
    types::{AltContractExchange, AltContractTrade, AltContractTradeSide},
    LOG_PREFIX, LOG_TARGET,
};

const BINANCE_FUTURES_STREAM_BASE: &str = "wss://fstream.binance.com/stream?streams=";
const BINANCE_FORCE_ORDER_STREAM_URL: &str = "wss://fstream.binance.com/ws/!forceOrder@arr";
const BINANCE_FUTURES_REST_BASE: &str = "https://fapi.binance.com";
const RECONNECT_DELAY_MS: u64 = 1_000;
const CONTEXT_POLL_INTERVAL_MS: u64 = 30_000;

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
}

pub async fn run(service: BinanceAltContractService) {
    loop {
        let symbols = binance_alt_contract_runtime_config().enabled_symbols();
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
        let url = stream_url(&symbols);
        service.set_exchange_status(AltContractExchange::Binance, "connecting", false, None);
        match connect_async(&url).await {
            Ok((ws, _)) => {
                tracing::info!(target: LOG_TARGET, "{} binance alt futures connected", LOG_PREFIX);
                service.mark_connected(AltContractExchange::Binance);
                let (_, mut read) = ws.split();
                while let Some(message) = read.next().await {
                    match message {
                        Ok(message) => {
                            if let Ok(text) = message.to_text() {
                                handle_message(text, &service);
                            }
                        }
                        Err(error) => {
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
                service.mark_reconnecting(AltContractExchange::Binance, Some(error.to_string()));
            }
        }
        tokio::time::sleep(Duration::from_millis(RECONNECT_DELAY_MS)).await;
    }
}

pub async fn run_context_polling(service: BinanceAltContractService) {
    let client = reqwest::Client::new();
    loop {
        let config = binance_alt_contract_runtime_config();
        if !config.enabled || !config.exchange.binance_enabled {
            tokio::time::sleep(Duration::from_millis(CONTEXT_POLL_INTERVAL_MS)).await;
            continue;
        }
        for symbol in config.enabled_symbols() {
            if let Err(error) = poll_symbol_context(&client, &service, &symbol).await {
                service.record_error();
                tracing::warn!(
                    target: LOG_TARGET,
                    symbol = symbol.as_str(),
                    "{} binance alt context poll skipped: {error}",
                    LOG_PREFIX
                );
            }
        }
        tokio::time::sleep(Duration::from_millis(CONTEXT_POLL_INTERVAL_MS)).await;
    }
}

pub async fn run_force_order_stream(service: BinanceAltContractService) {
    loop {
        let config = binance_alt_contract_runtime_config();
        if !config.enabled || !config.exchange.binance_enabled {
            tokio::time::sleep(Duration::from_millis(RECONNECT_DELAY_MS)).await;
            continue;
        }
        match connect_async(BINANCE_FORCE_ORDER_STREAM_URL).await {
            Ok((ws, _)) => {
                tracing::info!(
                    target: LOG_TARGET,
                    "{} binance alt forceOrder snapshot stream connected",
                    LOG_PREFIX
                );
                let (_, mut read) = ws.split();
                while let Some(message) = read.next().await {
                    match message {
                        Ok(message) => {
                            if let Ok(text) = message.to_text() {
                                handle_force_order_message(text, &service);
                            }
                        }
                        Err(error) => {
                            service.record_error();
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
                service.record_error();
                tracing::warn!(
                    target: LOG_TARGET,
                    "{} binance alt forceOrder connect failed: {error}",
                    LOG_PREFIX
                );
            }
        }
        tokio::time::sleep(Duration::from_millis(RECONNECT_DELAY_MS)).await;
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
    if binance_alt_contract_runtime_config().symbol_enabled(&trade.product_id) {
        service.mark_connected(AltContractExchange::Binance);
        service.ingest_live_trade(trade);
    }
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
        if !binance_alt_contract_runtime_config().symbol_enabled(&product_id) {
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
            service.update_liquidation_context(
                &product_id,
                event
                    .order
                    .trade_time_ms
                    .unwrap_or_else(crate::normalizers::trade::now_ms),
                notional_usd,
            );
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
