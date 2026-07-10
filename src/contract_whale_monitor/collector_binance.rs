use std::time::Duration;

use futures_util::StreamExt;
use tokio::sync::mpsc;
use tokio_tungstenite::connect_async;

use super::{
    log_events,
    normalizer::{
        binance_usdt_perp_symbol, normalize_binance_force_order_json_for_symbol,
        normalize_binance_funding_rate_json_for_symbol,
        normalize_binance_open_interest_json_for_symbol,
    },
    types::{ContractFundingSnapshot, ContractLiquidationOrder, ContractOiSnapshot},
    LOG_PREFIX, LOG_TARGET,
};

pub const BINANCE_BTC_USDT_PERP_AGG_TRADE_STREAM: &str =
    "wss://fstream.binance.com/ws/btcusdt@aggTrade";
pub const BINANCE_BTC_USDT_PERP_FORCE_ORDER_STREAM: &str =
    "wss://fstream.binance.com/ws/btcusdt@forceOrder";
pub const BINANCE_BTC_USDT_PERP_OPEN_INTEREST_URL: &str =
    "https://fapi.binance.com/fapi/v1/openInterest?symbol=BTCUSDT";
pub const BINANCE_BTC_USDT_PERP_PREMIUM_INDEX_URL: &str =
    "https://fapi.binance.com/fapi/v1/premiumIndex?symbol=BTCUSDT";
const RECONNECT_MAX_DELAY_MS: u64 = 30_000;

pub fn collector_status() -> &'static str {
    "defined_not_started"
}

pub async fn run_binance_force_order_collector(sender: mpsc::Sender<ContractLiquidationOrder>) {
    let mut reconnect_attempt = 0_u32;
    loop {
        tracing::info!(
            target: LOG_TARGET,
            event = log_events::WS_CONNECTED,
            "{} connecting binance forceOrder stream",
            LOG_PREFIX
        );
        match connect_async(BINANCE_BTC_USDT_PERP_FORCE_ORDER_STREAM).await {
            Ok((ws, _)) => {
                reconnect_attempt = 0;
                tracing::info!(
                    target: LOG_TARGET,
                    event = log_events::WS_CONNECTED,
                    "{} binance forceOrder stream connected",
                    LOG_PREFIX
                );
                let (_, mut read) = ws.split();
                while let Some(message) = read.next().await {
                    match message {
                        Ok(message) => {
                            if let Ok(text) = message.to_text() {
                                if let Some(order) = handle_force_order_message(text) {
                                    if sender.send(order).await.is_err() {
                                        tracing::warn!(
                                            target: LOG_TARGET,
                                            event = log_events::WS_DISCONNECTED,
                                            "{} binance forceOrder receiver dropped",
                                            LOG_PREFIX
                                        );
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
            }
            Err(error) => {
                tracing::warn!(
                    target: LOG_TARGET,
                    event = log_events::WS_DISCONNECTED,
                    error = %error,
                    "{} binance forceOrder connect failed",
                    LOG_PREFIX
                );
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
