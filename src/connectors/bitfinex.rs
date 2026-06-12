use std::{collections::BTreeMap, sync::Arc, time::Duration};

use futures_util::{SinkExt, StreamExt};
use parking_lot::RwLock;
use tokio_tungstenite::{connect_async, tungstenite::Message};

use crate::{
    market_data::event_bus::{MarketDataBus, MarketDataEvent},
    normalizers::trade::{normalize_bitfinex_trade, BitfinexTrade},
    types::market::{Venue, VenueConnectionStatus, VenueHealth},
};

use super::manager::{
    mark_message, mark_parse_error, mark_subscription_acked, mark_trade, set_status,
};

const URL: &str = "wss://api-pub.bitfinex.com/ws/2";
const RECONNECT_DELAY_MS: u64 = 1_500;
const CHANNELS: [&str; 2] = ["tBTCF0:USTF0", "tETHF0:USTF0"];

pub async fn run(bus: MarketDataBus, health: Arc<RwLock<BTreeMap<String, VenueHealth>>>) {
    loop {
        set_status(
            &bus,
            &health,
            Venue::Bitfinex,
            VenueConnectionStatus::Connecting,
            None,
        );
        match connect_async(URL).await {
            Ok((ws, _)) => {
                set_status(
                    &bus,
                    &health,
                    Venue::Bitfinex,
                    VenueConnectionStatus::Connected,
                    None,
                );
                let (mut write, mut read) = ws.split();
                for symbol in CHANNELS {
                    let subscribe = serde_json::json!({
                        "event": "subscribe",
                        "channel": "trades",
                        "symbol": symbol
                    });
                    if let Err(error) = write.send(Message::Text(subscribe.to_string())).await {
                        set_status(
                            &bus,
                            &health,
                            Venue::Bitfinex,
                            VenueConnectionStatus::Degraded,
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
                                mark_message(&bus, &health, Venue::Bitfinex);
                                handle_message(text, &bus, &health, &mut channels);
                            }
                        }
                        Err(error) => {
                            set_status(
                                &bus,
                                &health,
                                Venue::Bitfinex,
                                VenueConnectionStatus::Degraded,
                                Some(error.to_string()),
                            );
                            break;
                        }
                    }
                }
            }
            Err(error) => {
                set_status(
                    &bus,
                    &health,
                    Venue::Bitfinex,
                    VenueConnectionStatus::Error,
                    Some(error.to_string()),
                );
            }
        }
        set_status(
            &bus,
            &health,
            Venue::Bitfinex,
            VenueConnectionStatus::Reconnecting,
            None,
        );
        tokio::time::sleep(Duration::from_millis(RECONNECT_DELAY_MS)).await;
    }
}

fn handle_message(
    text: &str,
    bus: &MarketDataBus,
    health: &Arc<RwLock<BTreeMap<String, VenueHealth>>>,
    channels: &mut BTreeMap<u64, String>,
) {
    let Ok(value) = serde_json::from_str::<serde_json::Value>(text) else {
        mark_parse_error(
            bus,
            health,
            Venue::Bitfinex,
            "bitfinex message json parse error",
        );
        return;
    };

    if let Some(event) = value.get("event").and_then(|item| item.as_str()) {
        if event == "subscribed" {
            if value.get("channel").and_then(|item| item.as_str()) == Some("trades") {
                if let (Some(chan_id), Some(symbol)) = (
                    value.get("chanId").and_then(|item| item.as_u64()),
                    value.get("symbol").and_then(|item| item.as_str()),
                ) {
                    channels.insert(chan_id, symbol.to_string());
                    mark_subscription_acked(bus, health, Venue::Bitfinex, true, false);
                }
            }
        } else if event == "error" {
            mark_parse_error(
                bus,
                health,
                Venue::Bitfinex,
                value
                    .get("msg")
                    .and_then(|item| item.as_str())
                    .unwrap_or("bitfinex subscription error"),
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
            publish_trade_value(&symbol, item, bus, health);
        }
        return;
    }

    if matches!(payload.as_str(), Some("te" | "tu")) {
        if let Some(item) = items.get(2) {
            publish_trade_value(&symbol, item, bus, health);
        }
    }
}

fn publish_trade_value(
    symbol: &str,
    item: &serde_json::Value,
    bus: &MarketDataBus,
    health: &Arc<RwLock<BTreeMap<String, VenueHealth>>>,
) {
    let Some(trade) = bitfinex_trade_from_value(symbol, item) else {
        mark_parse_error(bus, health, Venue::Bitfinex, "bitfinex trade schema error");
        return;
    };
    if let Some(trade) = normalize_bitfinex_trade(trade) {
        mark_trade(bus, health, Venue::Bitfinex, trade.ts);
        bus.publish(MarketDataEvent::Trade(trade));
    } else {
        mark_parse_error(
            bus,
            health,
            Venue::Bitfinex,
            "bitfinex trade normalize failed",
        );
    }
}

fn bitfinex_trade_from_value(symbol: &str, item: &serde_json::Value) -> Option<BitfinexTrade> {
    let values = item.as_array()?;
    let trade_id = values.first()?.clone();
    let ts = values.get(1)?.as_i64()?;
    let amount = values.get(2)?.as_f64()?;
    let price = values.get(3)?.as_f64()?;
    Some(BitfinexTrade {
        symbol: symbol.to_string(),
        trade_id,
        ts,
        amount,
        price,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_bitfinex_subscribe_and_trade_update() {
        let bus = MarketDataBus::new(8);
        let mut rx = bus.subscribe();
        let mut map = BTreeMap::new();
        map.insert(
            Venue::Bitfinex.as_key().to_string(),
            VenueHealth::start_attempted_with_symbol(Venue::Bitfinex, "BTC-PERP"),
        );
        let health = Arc::new(RwLock::new(map));
        let mut channels = BTreeMap::new();

        handle_message(
            r#"{"event":"subscribed","channel":"trades","chanId":42,"symbol":"tBTCF0:USTF0"}"#,
            &bus,
            &health,
            &mut channels,
        );
        handle_message(
            r#"[42,"tu",[991,1712400000000,0.5,70000]]"#,
            &bus,
            &health,
            &mut channels,
        );

        let trade = loop {
            match rx.try_recv() {
                Ok(MarketDataEvent::Trade(trade)) => break trade,
                Ok(_) => continue,
                Err(error) => panic!("expected trade event, got {error}"),
            }
        };

        assert_eq!(trade.venue, Venue::Bitfinex);
        assert_eq!(trade.symbol, "BTC-PERP");
        assert_eq!(trade.size_btc, 0.5);
        assert_eq!(
            trade.aggressor_side,
            crate::types::market::AggressorSide::Buy
        );
    }

    #[test]
    fn parses_bitfinex_snapshot_trade_rows() {
        let bus = MarketDataBus::new(8);
        let mut rx = bus.subscribe();
        let mut map = BTreeMap::new();
        map.insert(
            Venue::Bitfinex.as_key().to_string(),
            VenueHealth::start_attempted_with_symbol(Venue::Bitfinex, "ETH-PERP"),
        );
        let health = Arc::new(RwLock::new(map));
        let mut channels = BTreeMap::new();
        channels.insert(9, "tETHF0:USTF0".to_string());

        handle_message(
            r#"[9,[[1,1712400000000,-2.0,3500]]]"#,
            &bus,
            &health,
            &mut channels,
        );

        let trade = loop {
            match rx.try_recv() {
                Ok(MarketDataEvent::Trade(trade)) => break trade,
                Ok(_) => continue,
                Err(error) => panic!("expected trade event, got {error}"),
            }
        };

        assert_eq!(trade.venue, Venue::Bitfinex);
        assert_eq!(trade.symbol, "ETH-PERP");
        assert_eq!(trade.size_btc, 2.0);
        assert_eq!(
            trade.aggressor_side,
            crate::types::market::AggressorSide::Sell
        );
    }
}
