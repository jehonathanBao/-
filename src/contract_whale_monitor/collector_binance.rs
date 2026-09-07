use std::collections::BTreeMap;
use std::time::Duration;

use futures_util::{SinkExt, StreamExt};
use parking_lot::RwLock;
use std::sync::OnceLock;
use tokio::sync::mpsc;
use tokio_tungstenite::connect_async;
use tokio_tungstenite::tungstenite::Message;

use super::{
    log_events,
    normalizer::{
        binance_usdt_perp_symbol, normalize_binance_force_order_json_for_symbol,
        normalize_binance_funding_rate_json_for_symbol,
        normalize_binance_open_interest_json_for_symbol,
    },
    types::{
        ContractExchange, ContractFundingSnapshot, ContractLiquidationOrder, ContractOiSnapshot,
        ContractReferencePriceSnapshot,
    },
    LOG_PREFIX, LOG_TARGET,
};

pub const BINANCE_BTC_USDT_PERP_AGG_TRADE_STREAM: &str =
    "wss://fstream.binance.com/market/ws/btcusdt@aggTrade";
pub const BINANCE_BTC_USDT_PERP_FORCE_ORDER_STREAM: &str =
    "wss://fstream.binance.com/market/ws/btcusdt@forceOrder";
pub const BINANCE_BTC_USDT_PERP_OPEN_INTEREST_URL: &str =
    "https://fapi.binance.com/fapi/v1/openInterest?symbol=BTCUSDT";
pub const BINANCE_BTC_USDT_PERP_PREMIUM_INDEX_URL: &str =
    "https://fapi.binance.com/fapi/v1/premiumIndex?symbol=BTCUSDT";
const RECONNECT_MAX_DELAY_MS: u64 = 30_000;

#[derive(Debug, Clone, Default, serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct LiquidationStreamHealth {
    pub connected: bool,
    pub last_frame_at_ms: Option<i64>,
    pub last_order_at_ms: Option<i64>,
    pub order_count: u64,
}

impl LiquidationStreamHealth {
    fn status(&self, at: i64) -> &'static str {
        if !self.connected {
            "disconnected"
        } else if self
            .last_frame_at_ms
            .is_none_or(|ts| ts > at || at.saturating_sub(ts) > 90_000)
        {
            "stale"
        } else if self.order_count == 0 {
            "healthy_empty"
        } else {
            "healthy"
        }
    }
}

fn health_registry() -> &'static RwLock<BTreeMap<String, LiquidationStreamHealth>> {
    static HEALTH: OnceLock<RwLock<BTreeMap<String, LiquidationStreamHealth>>> = OnceLock::new();
    HEALTH.get_or_init(|| RwLock::new(BTreeMap::new()))
}

pub fn collector_health() -> serde_json::Value {
    let at = crate::normalizers::trade::now_ms();
    serde_json::json!({
        "volumeSemantics": "sampled_liquidation_orders_not_total_market_liquidations",
        "symbols": health_registry().read().iter().map(|(symbol, health)|
            (symbol.clone(), serde_json::json!({"status":health.status(at),"health":health})))
            .collect::<BTreeMap<_,_>>()
    })
}

pub fn collector_status() -> &'static str {
    let health = health_registry().read();
    if health.is_empty() {
        return "not_started";
    }
    if health.values().all(|item| {
        matches!(
            item.status(crate::normalizers::trade::now_ms()),
            "healthy" | "healthy_empty"
        )
    }) {
        "healthy"
    } else {
        "degraded"
    }
}

pub async fn run_binance_force_order_collector(sender: mpsc::Sender<ContractLiquidationOrder>) {
    run_binance_force_order_collector_for_symbol("BTC", sender).await;
}

/// Collect liquidation orders for one USDT-margined perpetual symbol.  The
/// Binance stream is symbol-scoped, so callers can run this concurrently for
/// BTC, ETH and any subsequently configured symbol.
pub async fn run_binance_force_order_collector_for_symbol(
    symbol: &str,
    sender: mpsc::Sender<ContractLiquidationOrder>,
) {
    let stream_url = format!(
        "wss://fstream.binance.com/market/ws/{}@forceOrder",
        binance_usdt_perp_symbol(symbol).to_ascii_lowercase()
    );
    let mut reconnect_attempt = 0_u32;
    let health_key = symbol.to_ascii_uppercase();
    health_registry()
        .write()
        .entry(health_key.clone())
        .or_default();
    loop {
        if sender.is_closed() {
            return;
        }
        tracing::info!(
            target: LOG_TARGET,
            event = log_events::WS_CONNECTED,
            "{} connecting binance forceOrder stream",
            LOG_PREFIX
        );
        match tokio::time::timeout(Duration::from_secs(10), connect_async(&stream_url)).await {
            Ok(Ok((ws, _))) => {
                reconnect_attempt = 0;
                tracing::info!(
                    target: LOG_TARGET,
                    event = log_events::WS_CONNECTED,
                    "{} binance {} forceOrder stream connected",
                    symbol,
                    LOG_PREFIX
                );
                health_registry().write().insert(
                    health_key.clone(),
                    LiquidationStreamHealth {
                        connected: true,
                        last_frame_at_ms: Some(crate::normalizers::trade::now_ms()),
                        ..Default::default()
                    },
                );
                let (mut write, mut read) = ws.split();
                let mut heartbeat = tokio::time::interval(Duration::from_secs(30));
                heartbeat.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Skip);
                loop {
                    let message = tokio::select! {
                        _ = sender.closed() => { health_registry().write().entry(health_key.clone()).or_default().connected = false; return; },
                        _ = heartbeat.tick() => {
                            let stale = health_registry().read().get(&health_key).is_none_or(|health|
                                health.status(crate::normalizers::trade::now_ms()) == "stale");
                            if stale || write.send(Message::Ping(Vec::new())).await.is_err() { break; }
                            continue;
                        },
                        message = read.next() => match message { Some(message) => message, None => break },
                    };
                    match message {
                        Ok(message) => {
                            health_registry()
                                .write()
                                .entry(health_key.clone())
                                .or_default()
                                .last_frame_at_ms = Some(crate::normalizers::trade::now_ms());
                            if let Message::Ping(payload) = message {
                                if write.send(Message::Pong(payload)).await.is_err() {
                                    break;
                                }
                                continue;
                            }
                            if matches!(message, Message::Close(_)) {
                                break;
                            }
                            if let Ok(text) = message.to_text() {
                                if let Some(order) =
                                    handle_force_order_message_for_symbol(symbol, text)
                                {
                                    {
                                        let mut health = health_registry().write();
                                        let item = health.entry(health_key.clone()).or_default();
                                        item.last_order_at_ms = Some(order.ts);
                                        item.order_count = item.order_count.saturating_add(1);
                                    }
                                    if tokio::time::timeout(
                                        Duration::from_secs(5),
                                        sender.send(order),
                                    )
                                    .await
                                    .is_err()
                                    {
                                        tracing::warn!(target: LOG_TARGET, "liquidation consumer backpressure; reconnecting");
                                        break;
                                    }
                                    if sender.is_closed() {
                                        tracing::warn!(
                                            target: LOG_TARGET,
                                            event = log_events::WS_DISCONNECTED,
                                            "{} binance forceOrder receiver dropped",
                                            LOG_PREFIX
                                        );
                                        health_registry()
                                            .write()
                                            .entry(health_key.clone())
                                            .or_default()
                                            .connected = false;
                                        return;
                                    }
                                }
                            }
                        }
                        Err(error) => {
                            tracing::warn!(
                                target: LOG_TARGET,
                                event = log_events::WS_DISCONNECTED,
                                error = %error,
                                "{} binance forceOrder stream disconnected",
                                LOG_PREFIX
                            );
                            break;
                        }
                    }
                }
                health_registry()
                    .write()
                    .entry(health_key.clone())
                    .or_default()
                    .connected = false;
            }
            Ok(Err(error)) => {
                tracing::warn!(
                    target: LOG_TARGET,
                    event = log_events::WS_DISCONNECTED,
                    error = %error,
                    "{} binance forceOrder connect failed",
                    LOG_PREFIX
                );
            }
            Err(_) => {
                tracing::warn!(target: LOG_TARGET, "binance forceOrder connect timed out");
            }
        }
        reconnect_attempt = reconnect_attempt.saturating_add(1);
        let next_delay_ms = reconnect_delay_ms(reconnect_attempt, 17);
        tracing::warn!(
            target: LOG_TARGET,
            event = log_events::WS_DISCONNECTED,
            exchange = "binance",
            attempt = reconnect_attempt,
            next_delay_ms,
            "{} binance forceOrder reconnect scheduled",
            LOG_PREFIX
        );
        tokio::time::sleep(Duration::from_millis(next_delay_ms)).await;
    }
}

#[cfg(test)]
mod health_tests {
    use super::*;
    #[test]
    fn empty_stream_requires_live_heartbeat_not_a_hardcoded_ready_label() {
        let mut health = LiquidationStreamHealth::default();
        assert_eq!(health.status(100_000), "disconnected");
        health.connected = true;
        assert_eq!(health.status(100_000), "stale");
        health.last_frame_at_ms = Some(99_000);
        assert_eq!(health.status(100_000), "healthy_empty");
        assert_eq!(health.status(200_000), "stale");
        health.order_count = 1;
        assert_eq!(health.status(100_000), "healthy");
    }
}

pub fn reconnect_delay_ms(attempt: u32, jitter_seed: u64) -> u64 {
    let base = 1_000_u64.saturating_mul(1_u64 << attempt.saturating_sub(1).min(5));
    let capped = base.min(RECONNECT_MAX_DELAY_MS);
    let jitter = (jitter_seed.wrapping_mul(31).wrapping_add(attempt as u64) % 401) as i64 - 200;
    (capped as i64 + capped as i64 * jitter / 1_000).max(1_000) as u64
}

pub fn handle_force_order_message(text: &str) -> Option<ContractLiquidationOrder> {
    handle_force_order_message_for_symbol("BTC", text)
}

pub fn handle_force_order_message_for_symbol(
    symbol: &str,
    text: &str,
) -> Option<ContractLiquidationOrder> {
    let payload = serde_json::from_str::<serde_json::Value>(text).ok()?;
    normalize_binance_force_order_json_for_symbol(symbol, &payload)
}

pub fn binance_open_interest_url(symbol: &str) -> String {
    format!(
        "https://fapi.binance.com/fapi/v1/openInterest?symbol={}",
        binance_usdt_perp_symbol(symbol)
    )
}

pub fn binance_premium_index_url(symbol: &str) -> String {
    format!(
        "https://fapi.binance.com/fapi/v1/premiumIndex?symbol={}",
        binance_usdt_perp_symbol(symbol)
    )
}

pub fn binance_futures_ticker_url(symbol: &str) -> String {
    format!(
        "https://fapi.binance.com/fapi/v1/ticker/price?symbol={}",
        binance_usdt_perp_symbol(symbol)
    )
}

pub fn binance_spot_ticker_url(symbol: &str) -> String {
    format!(
        "https://api.binance.com/api/v3/ticker/price?symbol={}",
        binance_usdt_perp_symbol(symbol)
    )
}

/// Load closed Binance 1m mark/index candles for deterministic V4.1 replay.
/// Pagination, bounded retries and key-based de-duplication make this safe to
/// resume after a partial historical import.
pub async fn fetch_binance_reference_history_for_symbol(
    client: &reqwest::Client,
    symbol: &str,
    start_ts: i64,
    end_ts: i64,
) -> anyhow::Result<Vec<ContractReferencePriceSnapshot>> {
    let pair = binance_usdt_perp_symbol(symbol);
    let mut merged = BTreeMap::<(i64, String), ContractReferencePriceSnapshot>::new();
    for (source, endpoint, parameter) in [
        ("mark", "markPriceKlines", "symbol"),
        ("index", "indexPriceKlines", "pair"),
    ] {
        let mut cursor = start_ts.div_euclid(60_000).saturating_mul(60_000);
        while cursor <= end_ts {
            let url = format!(
                "https://fapi.binance.com/fapi/v1/{endpoint}?{parameter}={pair}&interval=1m&startTime={cursor}&endTime={end_ts}&limit=1500"
            );
            let mut payload = None;
            let mut last_error = None;
            for attempt in 0..3_u64 {
                match client.get(&url).send().await {
                    Ok(response) => match response.error_for_status() {
                        Ok(response) => match response.json::<serde_json::Value>().await {
                            Ok(value) => {
                                payload = Some(value);
                                break;
                            }
                            Err(error) => last_error = Some(error.to_string()),
                        },
                        Err(error) => last_error = Some(error.to_string()),
                    },
                    Err(error) => last_error = Some(error.to_string()),
                }
                tokio::time::sleep(Duration::from_millis(250 * (attempt + 1))).await;
            }
            let payload = payload.ok_or_else(|| {
                anyhow::anyhow!(
                    "binance {source} history request failed: {}",
                    last_error.unwrap_or_else(|| "unknown error".to_string())
                )
            })?;
            let rows = payload.as_array().ok_or_else(|| {
                anyhow::anyhow!("binance {source} history response is not an array")
            })?;
            if rows.is_empty() {
                break;
            }
            let mut newest = cursor;
            for row in rows {
                let Some(values) = row.as_array() else {
                    continue;
                };
                let Some(open_ts) = values.first().and_then(serde_json::Value::as_i64) else {
                    continue;
                };
                let close = values
                    .get(4)
                    .and_then(|value| value.as_f64().or_else(|| value.as_str()?.parse().ok()));
                let Some(price) = close.filter(|value: &f64| value.is_finite() && *value > 0.0)
                else {
                    continue;
                };
                let close_ts = values
                    .get(6)
                    .and_then(serde_json::Value::as_i64)
                    .unwrap_or_else(|| open_ts.saturating_add(59_999));
                let ts_bucket = open_ts.div_euclid(60_000).saturating_mul(60_000);
                newest = newest.max(ts_bucket);
                merged.insert(
                    (ts_bucket, source.to_string()),
                    ContractReferencePriceSnapshot {
                        ts_bucket,
                        exchange: ContractExchange::Binance,
                        symbol: symbol.trim().to_ascii_uppercase(),
                        price_source: source.to_string(),
                        price,
                        premium_bps: None,
                        event_time_ms: close_ts,
                        received_at_ms: crate::normalizers::trade::now_ms(),
                    },
                );
            }
            let next = newest.saturating_add(60_000);
            if next <= cursor || rows.len() < 1500 {
                break;
            }
            cursor = next;
            tokio::time::sleep(Duration::from_millis(150)).await;
        }
    }
    Ok(merged.into_values().collect())
}

/// Fetch the four Binance reference-price legs used by V4.1. Rows are
/// minute-bucketed in storage, while event and local receive timestamps remain
/// available for freshness and degradation decisions.
pub async fn fetch_binance_reference_prices_for_symbol(
    client: &reqwest::Client,
    symbol: &str,
    fallback_ts: i64,
) -> anyhow::Result<Vec<ContractReferencePriceSnapshot>> {
    let premium = client
        .get(binance_premium_index_url(symbol))
        .send()
        .await?
        .error_for_status()?
        .json::<serde_json::Value>()
        .await?;
    let futures = client
        .get(binance_futures_ticker_url(symbol))
        .send()
        .await?
        .error_for_status()?
        .json::<serde_json::Value>()
        .await?;
    let spot = client
        .get(binance_spot_ticker_url(symbol))
        .send()
        .await?
        .error_for_status()?
        .json::<serde_json::Value>()
        .await?;
    Ok(normalize_binance_reference_prices(
        symbol,
        &premium,
        &futures,
        &spot,
        fallback_ts,
    ))
}

pub fn normalize_binance_reference_prices(
    symbol: &str,
    premium: &serde_json::Value,
    futures: &serde_json::Value,
    spot: &serde_json::Value,
    received_at_ms: i64,
) -> Vec<ContractReferencePriceSnapshot> {
    fn positive(value: Option<&serde_json::Value>) -> Option<f64> {
        let value = value.and_then(|value| {
            value
                .as_f64()
                .or_else(|| value.as_str().and_then(|raw| raw.parse::<f64>().ok()))
        })?;
        (value.is_finite() && value > 0.0).then_some(value)
    }
    let event_time_ms = premium
        .get("time")
        .and_then(serde_json::Value::as_i64)
        .unwrap_or(received_at_ms);
    let ts_bucket = event_time_ms.div_euclid(60_000).saturating_mul(60_000);
    let mark = positive(premium.get("markPrice"));
    let index = positive(premium.get("indexPrice"));
    let premium_bps = mark
        .zip(index)
        .map(|(mark, index)| ((mark / index) - 1.0) * 10_000.0)
        .filter(|value| value.is_finite());
    let mut rows = Vec::with_capacity(4);
    let mut push = |source: &str, price: Option<f64>, event_ts: i64| {
        if let Some(price) = price {
            rows.push(ContractReferencePriceSnapshot {
                ts_bucket,
                exchange: ContractExchange::Binance,
                symbol: symbol.trim().to_ascii_uppercase(),
                price_source: source.to_string(),
                price,
                premium_bps,
                event_time_ms: event_ts,
                received_at_ms,
            });
        }
    };
    push("mark", mark, event_time_ms);
    push("index", index, event_time_ms);
    push(
        "futures_last",
        positive(futures.get("price")),
        received_at_ms,
    );
    push("spot", positive(spot.get("price")), received_at_ms);
    rows
}

pub async fn fetch_binance_open_interest_snapshot(
    client: &reqwest::Client,
    mark_price: Option<f64>,
    fallback_ts: i64,
) -> anyhow::Result<Option<ContractOiSnapshot>> {
    fetch_binance_open_interest_snapshot_for_symbol(client, "BTC", mark_price, fallback_ts).await
}

pub async fn fetch_binance_open_interest_snapshot_for_symbol(
    client: &reqwest::Client,
    symbol: &str,
    mark_price: Option<f64>,
    fallback_ts: i64,
) -> anyhow::Result<Option<ContractOiSnapshot>> {
    let payload = client
        .get(binance_open_interest_url(symbol))
        .send()
        .await?
        .json::<serde_json::Value>()
        .await?;
    Ok(normalize_binance_open_interest_json_for_symbol(
        symbol,
        &payload,
        mark_price,
        fallback_ts,
    ))
}

pub async fn fetch_binance_funding_snapshot(
    client: &reqwest::Client,
    fallback_ts: i64,
) -> anyhow::Result<Option<ContractFundingSnapshot>> {
    fetch_binance_funding_snapshot_for_symbol(client, "BTC", fallback_ts).await
}

pub async fn fetch_binance_funding_snapshot_for_symbol(
    client: &reqwest::Client,
    symbol: &str,
    fallback_ts: i64,
) -> anyhow::Result<Option<ContractFundingSnapshot>> {
    let payload = client
        .get(binance_premium_index_url(symbol))
        .send()
        .await?
        .json::<serde_json::Value>()
        .await?;
    Ok(normalize_binance_funding_rate_json_for_symbol(
        symbol,
        &payload,
        fallback_ts,
    ))
}
