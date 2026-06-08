use std::time::Duration;

use futures_util::{SinkExt, StreamExt};
use tokio::sync::mpsc;
use tokio_tungstenite::{connect_async, tungstenite::Message};

use super::{
    log_events,
    normalizer::{
        normalize_okx_funding_rate_json, normalize_okx_liquidation_order_json,
        normalize_okx_open_interest_json,
    },
    types::{ContractFundingSnapshot, ContractLiquidationOrder, ContractOiSnapshot},
    LOG_PREFIX, LOG_TARGET,
};

pub const OKX_BTC_USDT_SWAP_TRADES_CHANNEL: &str = "trades:BTC-USDT-SWAP";
pub const OKX_BTC_USDT_SWAP_LIQUIDATION_ORDERS_CHANNEL: &str = "liquidation-orders:BTC-USDT-SWAP";
pub const OKX_PUBLIC_WS_URL: &str = "wss://ws.okx.com:8443/ws/v5/public";
pub const OKX_BTC_USDT_SWAP_OPEN_INTEREST_URL: &str =
    "https://www.okx.com/api/v5/public/open-interest?instType=SWAP&instId=BTC-USDT-SWAP";
pub const OKX_BTC_USDT_SWAP_FUNDING_RATE_URL: &str =
    "https://www.okx.com/api/v5/public/funding-rate?instId=BTC-USDT-SWAP";
const OKX_LIQUIDATION_RECONNECT_DELAY_MS: u64 = 1_000;

pub fn collector_status() -> &'static str {
    "defined_not_started"
}

pub async fn run_okx_liquidation_collector(
    sender: mpsc::Sender<ContractLiquidationOrder>,
    ct_val_btc: f64,
) {
    loop {
        tracing::info!(
            target: LOG_TARGET,
            event = log_events::WS_CONNECTED,
            "{} connecting okx liquidation-orders stream",
            LOG_PREFIX
        );
        match connect_async(OKX_PUBLIC_WS_URL).await {
            Ok((ws, _)) => {
                tracing::info!(
                    target: LOG_TARGET,
                    event = log_events::WS_CONNECTED,
                    "{} okx liquidation-orders stream connected",
                    LOG_PREFIX
                );
                let (mut write, mut read) = ws.split();
                let subscribe = serde_json::json!({
                    "op": "subscribe",
                    "args": [{
                        "channel": "liquidation-orders",
                        "instType": "SWAP",
                        "uly": "BTC-USDT"
                    }]
                });
                if let Err(error) = write.send(Message::Text(subscribe.to_string())).await {
                    tracing::warn!(
                        target: LOG_TARGET,
                        event = log_events::WS_DISCONNECTED,
                        error = %error,
                        "{} okx liquidation subscribe failed",
                        LOG_PREFIX
                    );
                }
                while let Some(message) = read.next().await {
                    match message {
                        Ok(message) => {
                            if let Ok(text) = message.to_text() {
                                for order in handle_liquidation_order_message(text, ct_val_btc) {
                                    if sender.send(order).await.is_err() {
                                        tracing::warn!(
                                            target: LOG_TARGET,
                                            event = log_events::WS_DISCONNECTED,
                                            "{} okx liquidation receiver dropped",
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
                                "{} okx liquidation stream disconnected",
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
                    "{} okx liquidation connect failed",
                    LOG_PREFIX
                );
            }
        }
        tokio::time::sleep(Duration::from_millis(OKX_LIQUIDATION_RECONNECT_DELAY_MS)).await;
    }
}

pub fn handle_liquidation_order_message(
    text: &str,
    ct_val_btc: f64,
) -> Vec<ContractLiquidationOrder> {
    let Ok(payload) = serde_json::from_str::<serde_json::Value>(text) else {
        return Vec::new();
    };
    normalize_okx_liquidation_order_json(&payload, ct_val_btc)
}

pub async fn fetch_okx_open_interest_snapshot(
    client: &reqwest::Client,
    ct_val_btc: f64,
) -> anyhow::Result<Option<ContractOiSnapshot>> {
    let payload = client
        .get(OKX_BTC_USDT_SWAP_OPEN_INTEREST_URL)
        .send()
        .await?
        .json::<serde_json::Value>()
        .await?;
    Ok(normalize_okx_open_interest_json(&payload, ct_val_btc))
}

pub async fn fetch_okx_funding_snapshot(
    client: &reqwest::Client,
) -> anyhow::Result<Option<ContractFundingSnapshot>> {
    let payload = client
        .get(OKX_BTC_USDT_SWAP_FUNDING_RATE_URL)
        .send()
        .await?
        .json::<serde_json::Value>()
        .await?;
    Ok(normalize_okx_funding_rate_json(&payload))
}
