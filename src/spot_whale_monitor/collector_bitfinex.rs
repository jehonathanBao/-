use std::{collections::BTreeMap, time::Duration};

use futures_util::{SinkExt, StreamExt};
use tokio_tungstenite::{connect_async, tungstenite::Message};

use super::{
    config::spot_whale_runtime_config, normalizer::normalize_bitfinex_trade_value,
    service::SpotWhaleService, types::SpotExchange, LOG_PREFIX, LOG_TARGET,
};

const URL: &str = "wss://api-pub.bitfinex.com/ws/2";
const RECONNECT_DELAY_MS: u64 = 1_500;

pub async fn run(service: SpotWhaleService) {
    loop {
        service.set_exchange_status(SpotExchange::Bitfinex, "connecting", false, None);
        match connect_async(URL).await {
            Ok((ws, _)) => {
                tracing::info!(target: LOG_TARGET, "{} bitfinex spot connected", LOG_PREFIX);
                service.mark_connected(SpotExchange::Bitfinex);
                let (mut write, mut read) = ws.split();
                for symbol in enabled_symbols() {
                    let subscribe = serde_json::json!({
                        "event": "subscribe",
                        "channel": "trades",
                        "symbol": symbol
                    });
                    if let Err(error) = write.send(Message::Text(subscribe.to_string())).await {
                        service.mark_reconnecting(
                            SpotExchange::Bitfinex,
                            Some(format!("bitfinex subscribe failed: {error}")),
                        );
                        break;
                    }
                }
                drop(write);

                let mut channels = BTreeMap::<u64, String>::new();
                while let Some(message) = read.next().await {
                    match message {
                        Ok(message) => {
                            if let Ok(text) = message.to_text() {
                                handle_message(text, &service, &mut channels);
                            }
                        }
                        Err(error) => {
                            service
                                .mark_reconnecting(SpotExchange::Bitfinex, Some(error.to_string()));
                            break;
                        }
                    }
                }
            }
            Err(error) => {
                service.mark_reconnecting(SpotExchange::Bitfinex, Some(error.to_string()));
            }
        }
        tokio::time::sleep(Duration::from_millis(RECONNECT_DELAY_MS)).await;
    }
}

fn enabled_symbols() -> Vec<&'static str> {
    let config = spot_whale_runtime_config();
    let mut symbols = Vec::new();
    if config.symbol_enabled("BTC") {
        symbols.push("tBTCUSD");
    }
    if config.symbol_enabled("ETH") {
        symbols.push("tETHUSD");
    }
    symbols
}

fn handle_message(text: &str, service: &SpotWhaleService, channels: &mut BTreeMap<u64, String>) {
    let Ok(value) = serde_json::from_str::<serde_json::Value>(text) else {
        service.set_exchange_status(
            SpotExchange::Bitfinex,
            "degraded",
            true,
            Some("bitfinex spot json parse error".to_string()),
        );
        return;
    };

    if let Some(event) = value.get("event").and_then(|item| item.as_str()) {
        if event == "subscribed"
            && value.get("channel").and_then(|item| item.as_str()) == Some("trades")
        {
            if let (Some(chan_id), Some(symbol)) = (
                value.get("chanId").and_then(|item| item.as_u64()),
                value.get("symbol").and_then(|item| item.as_str()),
            ) {
                channels.insert(chan_id, symbol.to_string());
                service.mark_connected(SpotExchange::Bitfinex);
            }
        } else if event == "error" {
            service.set_exchange_status(
                SpotExchange::Bitfinex,
                "degraded",
                false,
                Some(
                    value
                        .get("msg")
                        .and_then(|item| item.as_str())
                        .unwrap_or("bitfinex spot subscription error")
                        .to_string(),
                ),
            );
        }
        return;
    }

    let Some(items) = value.as_array() else {
        return;
    };
    let Some(chan_id) = items.first().and_then(|item| item.as_u64()) else {
        return;
    };
    if items.get(1).and_then(|item| item.as_str()) == Some("hb") {
        service.mark_connected(SpotExchange::Bitfinex);
        return;
    }
    let Some(symbol) = channels.get(&chan_id).cloned() else {
        return;
    };
    let Some(payload) = items.get(1) else {
        return;
    };

    if let Some(snapshot) = payload
        .as_array()
        .filter(|items| items.first().and_then(|first| first.as_array()).is_some())
    {
        for item in snapshot {
            ingest_trade(symbol.as_str(), item, service);
        }
        return;
    }

    if matches!(payload.as_str(), Some("te" | "tu")) {
        if let Some(item) = items.get(2) {
            ingest_trade(symbol.as_str(), item, service);
        }
    }
}

fn ingest_trade(symbol: &str, item: &serde_json::Value, service: &SpotWhaleService) {
    let Some(trade) = normalize_bitfinex_trade_value(symbol, item) else {
        service.set_exchange_status(
            SpotExchange::Bitfinex,
            "degraded",
            true,
            Some("bitfinex spot trade schema error".to_string()),
        );
        return;
    };
    if spot_whale_runtime_config().symbol_enabled(&trade.symbol) {
        service.mark_connected(SpotExchange::Bitfinex);
        service.ingest_live_trade(trade);
    }
}

#[cfg(test)]
mod tests {
    use crate::{
        normalizers::trade::now_ms,
        spot_whale_monitor::types::{SpotExchange, SpotTradeSide},
    };

    use super::*;

    #[test]
    fn parses_bitfinex_spot_trade_update() {
        let service = SpotWhaleService::new(true, true, now_ms().saturating_sub(120_000), None);
        let mut channels = BTreeMap::new();

        handle_message(
            r#"{"event":"subscribed","channel":"trades","chanId":7,"symbol":"tBTCUSD"}"#,
            &service,
            &mut channels,
        );
        let ts = now_ms();
        let message = format!(r#"[7,"tu",[11,{ts},-0.25,70000]]"#);
        handle_message(&message, &service, &mut channels);

        let summary = service.summary("BTC");
        let bitfinex = summary.exchanges.get("bitfinex").expect("bitfinex status");
        assert!(bitfinex.connected);
        assert_eq!(bitfinex.status, "connected");

        let latest = service.latest("BTC", 10);
        assert_eq!(latest.summary.trend60s.sell_volume_base, 0.25);
    }

    #[test]
    fn bitfinex_spot_normalizer_maps_positive_amount_to_buy() {
        let trade = normalize_bitfinex_trade_value(
            "tETHUSD",
            &serde_json::json!([12, 1712400000000_i64, 2.5, 3500.0]),
        )
        .expect("trade");

        assert_eq!(trade.exchange, SpotExchange::Bitfinex);
        assert_eq!(trade.symbol, "ETH");
        assert_eq!(trade.side, SpotTradeSide::Buy);
        assert_eq!(trade.notional_usd, 8_750.0);
    }
}
