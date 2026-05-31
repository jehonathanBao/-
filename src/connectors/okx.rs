use std::{collections::BTreeMap, sync::Arc, time::Duration};

use futures_util::{SinkExt, StreamExt};
use parking_lot::RwLock;
use serde::Deserialize;
use tokio_tungstenite::{connect_async, tungstenite::Message};

use crate::{
    market_data::event_bus::{MarketDataBus, MarketDataEvent},
    normalizers::{
        book::{normalize_book, RawBookInput},
        trade::{normalize_okx_trade, OkxTrade},
    },
    types::market::{Venue, VenueConnectionStatus, VenueHealth},
};

use super::manager::{
    mark_book, mark_message, mark_parse_error, mark_subscription_acked, mark_trade, set_status,
};

const URL: &str = "wss://ws.okx.com:8443/ws/v5/public";

#[derive(Debug, Deserialize)]
struct Envelope {
    arg: Option<Arg>,
    data: Option<Vec<serde_json::Value>>,
}

#[derive(Debug, Deserialize)]
struct Arg {
    channel: String,
    #[serde(rename = "instId")]
    inst_id: String,
}

#[derive(Debug, Deserialize)]
struct BookData {
    asks: Vec<[String; 4]>,
    bids: Vec<[String; 4]>,
    ts: Option<String>,
}

pub async fn run(bus: MarketDataBus, health: Arc<RwLock<BTreeMap<String, VenueHealth>>>) {
    loop {
        set_status(
            &bus,
            &health,
            Venue::Okx,
            VenueConnectionStatus::Connecting,
            None,
        );
        match connect_async(URL).await {
            Ok((mut ws, _)) => {
                set_status(
                    &bus,
                    &health,
                    Venue::Okx,
                    VenueConnectionStatus::Connected,
                    None,
                );
                let subscribe = serde_json::json!({
                    "op": "subscribe",
                    "args": [
                        { "channel": "trades", "instId": "BTC-USDT-SWAP" },
                        { "channel": "books5", "instId": "BTC-USDT-SWAP" }
                    ]
                });
                let _ = ws.send(Message::Text(subscribe.to_string())).await;
                let (_, mut read) = ws.split();
                while let Some(message) = read.next().await {
                    match message {
                        Ok(message) => {
                            if let Ok(text) = message.to_text() {
                                mark_message(&bus, &health, Venue::Okx);
                                handle_message(text, &bus, &health);
                            }
                        }
                        Err(error) => {
                            set_status(
                                &bus,
                                &health,
                                Venue::Okx,
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
                    Venue::Okx,
                    VenueConnectionStatus::Error,
                    Some(error.to_string()),
                );
            }
        }
        set_status(
            &bus,
            &health,
            Venue::Okx,
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
) {
    if text == "pong" {
        return;
    }
    if let Ok(value) = serde_json::from_str::<serde_json::Value>(text) {
        if value.get("event").and_then(|value| value.as_str()) == Some("subscribe") {
            let channel = value
                .get("arg")
                .and_then(|arg| arg.get("channel"))
                .and_then(|channel| channel.as_str());
            mark_subscription_acked(
                bus,
                health,
                Venue::Okx,
                channel == Some("trades"),
                channel == Some("books5"),
            );
            return;
        }
    }
    let Ok(envelope) = serde_json::from_str::<Envelope>(text) else {
        mark_parse_error(bus, health, Venue::Okx, "okx message json parse error");
        return;
    };
    let Some(arg) = envelope.arg else {
        return;
    };
    let Some(data) = envelope.data else {
        return;
    };

    if arg.channel == "trades" {
        for mut item in data {
            if let Some(obj) = item.as_object_mut() {
                obj.insert(
                    "instId".to_string(),
                    serde_json::Value::String(arg.inst_id.clone()),
                );
            }
            if let Ok(raw) = serde_json::from_value::<OkxTrade>(item) {
                if let Some(trade) = normalize_okx_trade(raw) {
                    mark_trade(bus, health, Venue::Okx, trade.ts);
                    bus.publish(MarketDataEvent::Trade(trade));
                } else {
                    mark_parse_error(bus, health, Venue::Okx, "okx trade normalize failed");
                }
            } else {
                mark_parse_error(bus, health, Venue::Okx, "okx trade schema error");
            }
        }
        return;
    }

    if arg.channel == "books5" {
        if let Some(item) = data.into_iter().next() {
            if let Ok(raw) = serde_json::from_value::<BookData>(item) {
                let book = normalize_book(RawBookInput {
                    venue: Venue::Okx,
                    symbol: arg.inst_id,
                    ts: raw
                        .ts
                        .and_then(|ts| ts.parse().ok())
                        .unwrap_or_else(crate::normalizers::trade::now_ms),
                    bids: parse_book_levels(raw.bids),
                    asks: parse_book_levels(raw.asks),
                });
                if let Some(book) = book {
                    mark_book(bus, health, Venue::Okx, book.ts);
                    bus.publish(MarketDataEvent::Book(book));
                }
            } else {
                mark_parse_error(bus, health, Venue::Okx, "okx book schema error");
            }
        }
    }
}

fn parse_book_levels(levels: Vec<[String; 4]>) -> Vec<(f64, f64)> {
    levels
        .into_iter()
        .filter_map(|level| Some((level[0].parse().ok()?, level[1].parse().ok()?)))
        .collect()
}
