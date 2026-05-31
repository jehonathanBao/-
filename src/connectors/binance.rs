use std::{collections::BTreeMap, sync::Arc, time::Duration};

use futures_util::StreamExt;
use parking_lot::RwLock;
use serde::Deserialize;
use tokio_tungstenite::connect_async;

use crate::{
    market_data::event_bus::{MarketDataBus, MarketDataEvent},
    normalizers::{
        book::{normalize_book, RawBookInput},
        trade::{normalize_binance_agg_trade, BinanceAggTrade},
    },
    types::market::{Venue, VenueConnectionStatus, VenueHealth},
};

use super::manager::{mark_book, mark_message, mark_parse_error, mark_trade, set_status};

const URL: &str = "wss://fstream.binance.com/stream?streams=btcusdt@aggTrade/btcusdt@depth20@100ms";
const REST_AGG_TRADES_URL: &str =
    "https://fapi.binance.com/fapi/v1/aggTrades?symbol=BTCUSDT&limit=100";
const REST_DEPTH_URL: &str = "https://fapi.binance.com/fapi/v1/depth?symbol=BTCUSDT&limit=20";
const CONNECT_TIMEOUT_SECS: u64 = 8;
const REST_POLL_INTERVAL_MS: u64 = 1000;

#[derive(Debug, Deserialize)]
struct Combined {
    data: serde_json::Value,
}

#[derive(Debug, Deserialize)]
struct Depth {
    #[serde(rename = "E")]
    event_time: Option<i64>,
    s: Option<String>,
    bids: Option<Vec<[String; 2]>>,
    asks: Option<Vec<[String; 2]>>,
}

#[derive(Debug, Deserialize)]
struct RestAggTrade {
    a: serde_json::Value,
    p: String,
    q: String,
    #[serde(rename = "T")]
    trade_time: Option<i64>,
    m: bool,
}

pub async fn run(bus: MarketDataBus, health: Arc<RwLock<BTreeMap<String, VenueHealth>>>) {
    let rest_client = reqwest::Client::new();
    loop {
        set_status(
            &bus,
            &health,
            Venue::Binance,
            VenueConnectionStatus::Connecting,
            None,
        );
        match tokio::time::timeout(
            Duration::from_secs(CONNECT_TIMEOUT_SECS),
            connect_async(URL),
        )
        .await
        {
            Ok(Ok((ws, _))) => {
                set_status(
                    &bus,
                    &health,
                    Venue::Binance,
                    VenueConnectionStatus::Connected,
                    None,
                );
                let (_, mut read) = ws.split();
                while let Some(message) = read.next().await {
                    match message {
                        Ok(message) => {
                            if let Ok(text) = message.to_text() {
                                mark_message(&bus, &health, Venue::Binance);
                                handle_message(text, &bus, &health);
                            }
                        }
                        Err(error) => {
                            set_status(
                                &bus,
                                &health,
                                Venue::Binance,
                                VenueConnectionStatus::Degraded,
                                Some(error.to_string()),
                            );
                            break;
                        }
                    }
                }
            }
            Ok(Err(error)) => {
                set_status(
                    &bus,
                    &health,
                    Venue::Binance,
                    VenueConnectionStatus::Error,
                    Some(error.to_string()),
                );
                if let Err(error) = run_rest_polling(&rest_client, &bus, &health).await {
                    set_status(
                        &bus,
                        &health,
                        Venue::Binance,
                        VenueConnectionStatus::Error,
                        Some(format!("binance rest fallback failed: {error}")),
                    );
                }
            }
            Err(_) => {
                set_status(
                    &bus,
                    &health,
                    Venue::Binance,
                    VenueConnectionStatus::Degraded,
                    Some(format!(
                        "binance websocket connect timed out after {CONNECT_TIMEOUT_SECS}s; using REST fallback"
                    )),
                );
                if let Err(error) = run_rest_polling(&rest_client, &bus, &health).await {
                    set_status(
                        &bus,
                        &health,
                        Venue::Binance,
                        VenueConnectionStatus::Error,
                        Some(format!("binance rest fallback failed: {error}")),
                    );
                }
            }
        }
        set_status(
            &bus,
            &health,
            Venue::Binance,
            VenueConnectionStatus::Reconnecting,
            None,
        );
        tokio::time::sleep(Duration::from_secs(2)).await;
    }
}

async fn run_rest_polling(
    client: &reqwest::Client,
    bus: &MarketDataBus,
    health: &Arc<RwLock<BTreeMap<String, VenueHealth>>>,
) -> anyhow::Result<()> {
    set_status(
        bus,
        health,
        Venue::Binance,
        VenueConnectionStatus::Connected,
        Some("binance websocket unavailable; reading public REST polling fallback".to_string()),
    );
    let mut interval = tokio::time::interval(Duration::from_millis(REST_POLL_INTERVAL_MS));
    loop {
        interval.tick().await;
        fetch_rest_trades(client, bus, health).await?;
        fetch_rest_depth(client, bus, health).await?;
    }
}

async fn fetch_rest_trades(
    client: &reqwest::Client,
    bus: &MarketDataBus,
    health: &Arc<RwLock<BTreeMap<String, VenueHealth>>>,
) -> anyhow::Result<()> {
    let trades = client
        .get(REST_AGG_TRADES_URL)
        .timeout(Duration::from_secs(5))
        .send()
        .await?
        .error_for_status()?
        .json::<Vec<RestAggTrade>>()
        .await?;
    for raw in trades {
        let raw = BinanceAggTrade {
            s: "BTCUSDT".to_string(),
            a: Some(raw.a),
            p: raw.p,
            q: raw.q,
            trade_time: raw.trade_time,
            event_time: raw.trade_time,
            m: raw.m,
        };
        if let Some(trade) = normalize_binance_agg_trade(raw) {
            mark_trade(bus, health, Venue::Binance, trade.ts);
            bus.publish(MarketDataEvent::Trade(trade));
        }
    }
    Ok(())
}

async fn fetch_rest_depth(
    client: &reqwest::Client,
    bus: &MarketDataBus,
    health: &Arc<RwLock<BTreeMap<String, VenueHealth>>>,
) -> anyhow::Result<()> {
    let raw = client
        .get(REST_DEPTH_URL)
        .timeout(Duration::from_secs(5))
        .send()
        .await?
        .error_for_status()?
        .json::<Depth>()
        .await?;
    handle_depth(raw, bus, health);
    Ok(())
}

fn handle_message(
    text: &str,
    bus: &MarketDataBus,
    health: &Arc<RwLock<BTreeMap<String, VenueHealth>>>,
) {
    let Ok(combined) = serde_json::from_str::<Combined>(text) else {
        mark_parse_error(
            bus,
            health,
            Venue::Binance,
            "binance combined stream json parse error",
        );
        return;
    };
    if combined.data.get("e").and_then(|v| v.as_str()) == Some("aggTrade") {
        if let Ok(raw) = serde_json::from_value::<BinanceAggTrade>(combined.data) {
            if let Some(trade) = normalize_binance_agg_trade(raw) {
                mark_trade(bus, health, Venue::Binance, trade.ts);
                bus.publish(MarketDataEvent::Trade(trade));
            } else {
                mark_parse_error(
                    bus,
                    health,
                    Venue::Binance,
                    "binance aggTrade normalize failed",
                );
            }
        } else {
            mark_parse_error(bus, health, Venue::Binance, "binance aggTrade schema error");
        }
        return;
    }

    if let Ok(raw) = serde_json::from_value::<Depth>(combined.data) {
        handle_depth(raw, bus, health);
    } else {
        mark_parse_error(bus, health, Venue::Binance, "binance depth schema error");
    }
}

fn handle_depth(
    raw: Depth,
    bus: &MarketDataBus,
    health: &Arc<RwLock<BTreeMap<String, VenueHealth>>>,
) {
    let book = normalize_book(RawBookInput {
        venue: Venue::Binance,
        symbol: raw.s.unwrap_or_else(|| "BTCUSDT".to_string()),
        ts: raw
            .event_time
            .unwrap_or_else(crate::normalizers::trade::now_ms),
        bids: parse_levels(raw.bids.unwrap_or_default()),
        asks: parse_levels(raw.asks.unwrap_or_default()),
    });
    if let Some(book) = book {
        mark_book(bus, health, Venue::Binance, book.ts);
        bus.publish(MarketDataEvent::Book(book));
    }
}

fn parse_levels(levels: Vec<[String; 2]>) -> Vec<(f64, f64)> {
    levels
        .into_iter()
        .filter_map(|[price, size]| Some((price.parse().ok()?, size.parse().ok()?)))
        .collect()
}
