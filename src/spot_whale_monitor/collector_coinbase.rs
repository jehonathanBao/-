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
                                if !handle_message(text, &service) {
                                    break;
                                }
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

fn handle_message(text: &str, service: &SpotWhaleService) -> bool {
    let Ok(payload) = serde_json::from_str::<serde_json::Value>(text) else {
        service.set_exchange_status(
            SpotExchange::Coinbase,
            "degraded",
            false,
            Some("coinbase json parse error".to_string()),
        );
        return true;
    };
    if payload.get("channel").and_then(|value| value.as_str()) == Some("heartbeats") {
        let config = spot_whale_runtime_config();
        if service.exchange_trade_stale(
            SpotExchange::Coinbase,
            config.data_quality.heartbeat_stale_ms,
        ) {
            service.mark_reconnecting(
                SpotExchange::Coinbase,
                Some("coinbase market_trades stale".to_string()),
            );
            return false;
        }
        service.mark_connected(SpotExchange::Coinbase);
        return true;
    }
    if payload.get("channel").and_then(|value| value.as_str()) != Some("market_trades") {
        return true;
    }
    for trade in normalize_coinbase_market_trades_json(&payload) {
        if spot_whale_runtime_config().symbol_enabled(&trade.symbol) {
            service.ingest_live_trade(trade);
        }
    }
    true
}

#[cfg(test)]
mod tests {
    use crate::{
        normalizers::trade::now_ms,
        spot_whale_monitor::{
            service::SpotWhaleService,
            types::{SpotExchange, SpotTrade, SpotTradeSide},
        },
    };

    use super::handle_message;

    #[test]
    fn coinbase_heartbeat_reconnects_when_market_trades_are_stale() {
        let service = SpotWhaleService::new(true, true, now_ms().saturating_sub(120_000), None);
        service.ingest_trade(SpotTrade {
            ts: now_ms().saturating_sub(120_000),
            exchange: SpotExchange::Coinbase,
            symbol: "BTC".to_string(),
            market: "spot".to_string(),
            price: 70_000.0,
            qty_base: 0.1,
            notional_usd: 7_000.0,
            side: SpotTradeSide::Buy,
            trade_id: Some("old".to_string()),
        });

        let keep_reading = handle_message(r#"{"channel":"heartbeats"}"#, &service);

        assert!(!keep_reading);
        let summary = service.summary("BTC");
        let coinbase = summary.exchanges.get("coinbase").expect("coinbase status");
        assert_eq!(coinbase.status, "stale");
        assert!(!coinbase.connected);
    }
}
