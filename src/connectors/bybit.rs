use std::{collections::BTreeMap, sync::Arc, time::Duration};

use futures_util::{SinkExt, StreamExt};
use parking_lot::RwLock;
use serde::Deserialize;
use tokio_tungstenite::{connect_async, tungstenite::Message};

use crate::{
    market_data::event_bus::{MarketDataBus, MarketDataEvent},
    normalizers::{
        book::{normalize_book, RawBookInput},
        trade::{normalize_bybit_trade, BybitTrade},
    },
    types::market::{Venue, VenueConnectionStatus, VenueHealth},
};

use super::manager::{
    mark_book, mark_message, mark_parse_error, mark_subscription_acked, mark_trade, set_status,
};

const URL: &str = "wss://stream.bybit.com/v5/public/linear";

#[derive(Debug, Deserialize)]
struct MessageEnvelope {
    topic: Option<String>,
    #[serde(rename = "type")]
    kind: Option<String>,
    ts: Option<i64>,
    data: Option<serde_json::Value>,
}

#[derive(Debug, Deserialize)]
struct OrderbookData {
    s: String,
    b: Vec<[String; 2]>,
    a: Vec<[String; 2]>,
    ts: Option<i64>,
}

#[derive(Default)]
struct LocalBook {
    bids: Vec<(f64, f64)>,
    asks: Vec<(f64, f64)>,
}

impl LocalBook {
    fn reset(&mut self, bids: Vec<(f64, f64)>, asks: Vec<(f64, f64)>) {
        self.bids = bids;
        self.asks = asks;
    }

    fn apply_delta(&mut self, bids: Vec<(f64, f64)>, asks: Vec<(f64, f64)>) {
        apply_levels(&mut self.bids, bids);
        apply_levels(&mut self.asks, asks);
    }
}

pub async fn run(bus: MarketDataBus, health: Arc<RwLock<BTreeMap<String, VenueHealth>>>) {
    loop {
        set_status(
            &bus,
            &health,
            Venue::Bybit,
            VenueConnectionStatus::Connecting,
            None,
        );
        match connect_async(URL).await {
            Ok((mut ws, _)) => {
                set_status(
                    &bus,
                    &health,
                    Venue::Bybit,
                    VenueConnectionStatus::Connected,
                    None,
                );
                let subscribe = serde_json::json!({
                    "op": "subscribe",
                    "args": ["publicTrade.BTCUSDT", "orderbook.50.BTCUSDT"]
                });
                let _ = ws.send(Message::Text(subscribe.to_string())).await;
                let (_, mut read) = ws.split();
                let mut local_book = LocalBook::default();
                while let Some(message) = read.next().await {
                    match message {
                        Ok(message) => {
                            if let Ok(text) = message.to_text() {
                                mark_message(&bus, &health, Venue::Bybit);
                                handle_message(text, &bus, &health, &mut local_book);
                            }
                        }
                        Err(error) => {
                            set_status(
                                &bus,
                                &health,
                                Venue::Bybit,
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
                    Venue::Bybit,
                    VenueConnectionStatus::Error,
                    Some(error.to_string()),
                );
            }
        }
        set_status(
            &bus,
            &health,
            Venue::Bybit,
            VenueConnectionStatus::Reconnecting,
            None,
        );
        tokio::time::sleep(Duration::from_secs(2)).await;
    }
}

fn handle_message(
    text: &str,
    bus: &MarketDataBus,
    health: &Arc<RwLock<BTreeMap<String, VenueHealth>>>,
    local_book: &mut LocalBook,
) {
    if let Ok(value) = serde_json::from_str::<serde_json::Value>(text) {
        if value.get("success").and_then(|value| value.as_bool()) == Some(true) {
            mark_subscription_acked(bus, health, Venue::Bybit, true, true);
            return;
        }
    }
    let Ok(envelope) = serde_json::from_str::<MessageEnvelope>(text) else {
        mark_parse_error(bus, health, Venue::Bybit, "bybit message json parse error");
        return;
    };
    let Some(topic) = envelope.topic else {
        return;
    };

    if topic.starts_with("publicTrade.") {
        if let Some(data) = envelope.data.and_then(|v| v.as_array().cloned()) {
            for item in data {
                if let Ok(raw) = serde_json::from_value::<BybitTrade>(item) {
                    if let Some(trade) = normalize_bybit_trade(raw) {
                        mark_trade(bus, health, Venue::Bybit, trade.ts);
                        bus.publish(MarketDataEvent::Trade(trade));
                    } else {
                        mark_parse_error(bus, health, Venue::Bybit, "bybit trade normalize failed");
                    }
                } else {
                    mark_parse_error(bus, health, Venue::Bybit, "bybit trade schema error");
                }
            }
        }
        return;
    }

    if topic.starts_with("orderbook.") {
        let Some(data) = envelope.data else {
            return;
        };
        if let Ok(raw) = serde_json::from_value::<OrderbookData>(data) {
            let bids = parse_levels(raw.b);
            let asks = parse_levels(raw.a);
            if envelope.kind.as_deref() == Some("snapshot") {
                local_book.reset(bids, asks);
            } else {
                local_book.apply_delta(bids, asks);
            }
            let book = normalize_book(RawBookInput {
                venue: Venue::Bybit,
                symbol: raw.s,
                ts: raw
                    .ts
                    .or(envelope.ts)
                    .unwrap_or_else(crate::normalizers::trade::now_ms),
                bids: local_book.bids.clone(),
                asks: local_book.asks.clone(),
            });
            if let Some(book) = book {
                mark_book(bus, health, Venue::Bybit, book.ts);
                bus.publish(MarketDataEvent::Book(book));
            }
        } else {
            mark_parse_error(bus, health, Venue::Bybit, "bybit orderbook schema error");
        }
    }
}

fn parse_levels(levels: Vec<[String; 2]>) -> Vec<(f64, f64)> {
    levels
        .into_iter()
        .filter_map(|[price, size]| Some((price.parse().ok()?, size.parse().ok()?)))
        .collect()
}

fn apply_levels(current: &mut Vec<(f64, f64)>, updates: Vec<(f64, f64)>) {
    for (price, size) in updates {
        if size == 0.0 {
            current.retain(|(existing, _)| *existing != price);
        } else if let Some((_, existing_size)) =
            current.iter_mut().find(|(existing, _)| *existing == price)
        {
            *existing_size = size;
        } else {
            current.push((price, size));
        }
    }
}
