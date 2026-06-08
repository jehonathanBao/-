use std::time::Duration;

use futures_util::{SinkExt, StreamExt};
use serde_json::json;
use tokio_tungstenite::{connect_async, tungstenite::Message};

use super::{
    config::spot_whale_runtime_config, normalizer::normalize_coinbase_market_trades_json,
    service::SpotWhaleService, types::SpotExchange, LOG_PREFIX, LOG_TARGET,
};

const URL: &str = "wss://advanced-trade-ws.coinbase.com";
const RECONNECT_DELAY_MS: u64 = 1_500;

pub async fn run(service: SpotWhaleService) {
    loop {
        service.set_exchange_status(SpotExchange::Coinbase, "connecting", false, None);
        match connect_async(URL).await {
            Ok((ws, _)) => {
                tracing::info!(target: LOG_TARGET, "{} coinbase spot connected", LOG_PREFIX);
                service.mark_connected(SpotExchange::Coinbase);
                let (mut write, mut read) = ws.split();
                let products = enabled_products();
                let market_subscribe = json!({
                    "type": "subscribe",
                    "product_ids": products,
                    "channel": "market_trades"
                });
                let heartbeat_subscribe = json!({
                    "type": "subscribe",
                    "product_ids": products,
                    "channel": "heartbeats"
                });
                if write
                    .send(Message::Text(market_subscribe.to_string()))
                    .await
                    .is_err()
                    || write
                        .send(Message::Text(heartbeat_subscribe.to_string()))
                        .await
                        .is_err()
                {
                    service.mark_reconnecting(
                        SpotExchange::Coinbase,
                        Some("coinbase subscribe failed".to_string()),
                    );
                    tokio::time::sleep(Duration::from_millis(RECONNECT_DELAY_MS)).await;
                    continue;
                }
                while let Some(message) = read.next().await {
                    match message {
                        Ok(message) => {
                            if let Ok(text) = message.to_text() {
                                handle_message(text, &service);
                            }
                        }
                        Err(error) => {
                            service
                                .mark_reconnecting(SpotExchange::Coinbase, Some(error.to_string()));
                            break;
                        }
                    }
                }
            }
            Err(error) => {
                service.mark_reconnecting(SpotExchange::Coinbase, Some(error.to_string()));
            }
        }
        tokio::time::sleep(Duration::from_millis(RECONNECT_DELAY_MS)).await;
    }
}

fn enabled_products() -> Vec<&'static str> {
    let config = spot_whale_runtime_config();
    let mut products = Vec::new();
    if config.symbol_enabled("BTC") {
        products.push("BTC-USD");
    }
    if config.symbol_enabled("ETH") {
        products.push("ETH-USD");
    }
    products
}

fn handle_message(text: &str, service: &SpotWhaleService) {
    let Ok(payload) = serde_json::from_str::<serde_json::Value>(text) else {
        service.set_exchange_status(
            SpotExchange::Coinbase,
            "degraded",
            false,
            Some("coinbase json parse error".to_string()),
        );
        return;
    };
    if payload.get("channel").and_then(|value| value.as_str()) == Some("heartbeats") {
        service.mark_connected(SpotExchange::Coinbase);
        return;
    }
    if payload.get("channel").and_then(|value| value.as_str()) != Some("market_trades") {
        return;
    }
    for trade in normalize_coinbase_market_trades_json(&payload) {
        if spot_whale_runtime_config().symbol_enabled(&trade.symbol) {
            service.ingest_live_trade(trade);
        }
    }
}
