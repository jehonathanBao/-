use std::time::Duration;

use futures_util::StreamExt;
use serde::Deserialize;
use tokio_tungstenite::connect_async;

use super::{
    config::spot_whale_runtime_config,
    normalizer::{normalize_binance_spot_trade, BinanceSpotAggTrade},
    service::SpotWhaleService,
    types::SpotExchange,
    LOG_PREFIX, LOG_TARGET,
};

const URL: &str = "wss://stream.binance.com:9443/stream?streams=btcusdt@aggTrade/ethusdt@aggTrade";
const RECONNECT_DELAY_MS: u64 = 1_000;

#[derive(Debug, Deserialize)]
struct Combined {
    data: serde_json::Value,
}

pub async fn run(service: SpotWhaleService) {
    loop {
        service.set_exchange_status(SpotExchange::Binance, "connecting", false, None);
        match connect_async(URL).await {
            Ok((ws, _)) => {
                tracing::info!(target: LOG_TARGET, "{} binance spot connected", LOG_PREFIX);
                service.mark_connected(SpotExchange::Binance);
                let (_, mut read) = ws.split();
                while let Some(message) = read.next().await {
                    match message {
                        Ok(message) => {
                            if let Ok(text) = message.to_text() {
                                handle_message(text, &service);
                            }
                        }
                        Err(error) => {
                            service
                                .mark_reconnecting(SpotExchange::Binance, Some(error.to_string()));
                            break;
                        }
                    }
                }
            }
            Err(error) => {
                service.mark_reconnecting(SpotExchange::Binance, Some(error.to_string()));
            }
        }
        tokio::time::sleep(Duration::from_millis(RECONNECT_DELAY_MS)).await;
    }
}

fn handle_message(text: &str, service: &SpotWhaleService) {
    let Ok(value) = serde_json::from_str::<serde_json::Value>(text) else {
        service.set_exchange_status(
            SpotExchange::Binance,
            "degraded",
            true,
            Some("binance spot json parse error".to_string()),
        );
        return;
    };
    let Some(payload) = binance_trade_payload(value) else {
        return;
    };
    let Ok(raw) = serde_json::from_value::<BinanceSpotAggTrade>(payload) else {
        service.set_exchange_status(
            SpotExchange::Binance,
            "degraded",
            true,
            Some("binance spot trade schema error".to_string()),
        );
        return;
    };
    let Some(trade) = normalize_binance_spot_trade(raw) else {
        return;
    };
    if spot_whale_runtime_config().symbol_enabled(&trade.symbol) {
        service.mark_connected(SpotExchange::Binance);
        service.ingest_live_trade(trade);
    }
}

fn binance_trade_payload(value: serde_json::Value) -> Option<serde_json::Value> {
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_combined_binance_spot_trade() {
        let raw = r#"{"stream":"btcusdt@aggTrade","data":{"e":"aggTrade","E":1712400000000,"s":"BTCUSDT","a":1,"p":"70000.00","q":"0.50","T":1712400000000,"m":false}}"#;
        let combined = serde_json::from_str::<Combined>(raw).expect("combined");
        let trade = normalize_binance_spot_trade(
            serde_json::from_value::<BinanceSpotAggTrade>(combined.data).expect("raw"),
        )
        .expect("trade");
        assert_eq!(trade.symbol, "BTC");
        assert_eq!(trade.exchange, SpotExchange::Binance);
    }

    #[test]
    fn parses_top_level_binance_spot_trade_payload() {
        let raw = serde_json::json!({
            "e": "aggTrade",
            "E": 1712400000000_i64,
            "s": "BTCUSDT",
            "a": 1,
            "p": "70000.00",
            "q": "0.50",
            "T": 1712400000000_i64,
            "m": false
        });
        let payload = binance_trade_payload(raw).expect("payload");
        let trade = normalize_binance_spot_trade(
            serde_json::from_value::<BinanceSpotAggTrade>(payload).expect("raw"),
        )
        .expect("trade");
        assert_eq!(trade.symbol, "BTC");
    }

    #[test]
    fn ignores_non_trade_control_payload_without_marking_disconnected() {
        let service = SpotWhaleService::new(true, true, 1712400000000, None);
        service.mark_connected(SpotExchange::Binance);

        handle_message(r#"{"result":null,"id":1}"#, &service);

        let summary = service.summary("BTC");
        let binance = summary.exchanges.get("binance").expect("binance status");
        assert!(binance.connected);
        assert_eq!(binance.status, "connected");
    }

    #[test]
    fn valid_trade_recovers_after_malformed_binance_payload() {
        let service = SpotWhaleService::new(true, true, 1712400000000, None);
        service.mark_connected(SpotExchange::Binance);

        handle_message("not json", &service);
        let summary = service.summary("BTC");
        let binance = summary.exchanges.get("binance").expect("binance status");
        assert!(binance.connected);
        assert_eq!(binance.status, "degraded");

        let ts = crate::normalizers::trade::now_ms();
        let message = format!(
            r#"{{"stream":"btcusdt@aggTrade","data":{{"e":"aggTrade","E":{ts},"s":"BTCUSDT","a":1,"p":"70000.00","q":"0.50","T":{ts},"m":false}}}}"#
        );
        handle_message(&message, &service);

        let summary = service.summary("BTC");
        let binance = summary.exchanges.get("binance").expect("binance status");
        assert!(binance.connected);
        assert_eq!(binance.status, "connected");
        assert!(binance.last_error.is_none());
    }
}
