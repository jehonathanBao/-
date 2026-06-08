use std::time::Duration;

use futures_util::StreamExt;
use tokio::sync::mpsc;
use tokio_tungstenite::connect_async;

use super::{
    log_events,
    normalizer::{
        normalize_binance_force_order_json, normalize_binance_funding_rate_json,
        normalize_binance_open_interest_json,
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
const FORCE_ORDER_RECONNECT_DELAY_MS: u64 = 1_000;

pub fn collector_status() -> &'static str {
    "defined_not_started"
}

pub async fn run_binance_force_order_collector(sender: mpsc::Sender<ContractLiquidationOrder>) {
    loop {
        tracing::info!(
            target: LOG_TARGET,
            event = log_events::WS_CONNECTED,
            "{} connecting binance forceOrder stream",
            LOG_PREFIX
        );
        match connect_async(BINANCE_BTC_USDT_PERP_FORCE_ORDER_STREAM).await {
            Ok((ws, _)) => {
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
        tokio::time::sleep(Duration::from_millis(FORCE_ORDER_RECONNECT_DELAY_MS)).await;
    }
}

pub fn handle_force_order_message(text: &str) -> Option<ContractLiquidationOrder> {
    let payload = serde_json::from_str::<serde_json::Value>(text).ok()?;
    normalize_binance_force_order_json(&payload)
}

pub async fn fetch_binance_open_interest_snapshot(
    client: &reqwest::Client,
    mark_price: Option<f64>,
    fallback_ts: i64,
) -> anyhow::Result<Option<ContractOiSnapshot>> {
    let payload = client
        .get(BINANCE_BTC_USDT_PERP_OPEN_INTEREST_URL)
        .send()
        .await?
        .json::<serde_json::Value>()
        .await?;
    Ok(normalize_binance_open_interest_json(
        &payload,
        mark_price,
        fallback_ts,
    ))
}

pub async fn fetch_binance_funding_snapshot(
    client: &reqwest::Client,
    fallback_ts: i64,
) -> anyhow::Result<Option<ContractFundingSnapshot>> {
    let payload = client
        .get(BINANCE_BTC_USDT_PERP_PREMIUM_INDEX_URL)
        .send()
        .await?
        .json::<serde_json::Value>()
        .await?;
    Ok(normalize_binance_funding_rate_json(&payload, fallback_ts))
}
