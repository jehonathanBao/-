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

const URL: &str = "wss://fstream.binance.com/stream?streams=btcusdt@trade/btcusdt@depth20@100ms/ethusdt@trade/ethusdt@depth20@100ms";
const REST_SYMBOLS: [&str; 2] = ["BTCUSDT", "ETHUSDT"];
const CONNECT_TIMEOUT_SECS: u64 = 8;
const REST_POLL_INTERVAL_MS: u64 = 1000;
const REST_FALLBACK_MAX_MS: u64 = 30_000;

#[derive(Debug, Deserialize)]
struct Combined {
    data: serde_json::Value,
}

#[derive(Debug, Deserialize)]
struct Depth {
    #[serde(rename = "E")]
    event_time: Option<i64>,
    s: Option<String>,
    #[serde(alias = "b")]
    bids: Option<Vec<[String; 2]>>,
    #[serde(alias = "a")]
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
        VenueConnectionStatus::Degraded,
        Some("binance websocket unavailable; reading public REST polling fallback".to_string()),
    );
    let fallback_deadline =
        tokio::time::Instant::now() + Duration::from_millis(REST_FALLBACK_MAX_MS);
    let mut interval = tokio::time::interval(Duration::from_millis(REST_POLL_INTERVAL_MS));
    loop {
        interval.tick().await;
        if tokio::time::Instant::now() >= fallback_deadline {
            return Ok(());
        }
        fetch_rest_trades(client, bus, health).await?;
        fetch_rest_depth(client, bus, health).await?;
    }
}

async fn fetch_rest_trades(
    client: &reqwest::Client,
    bus: &MarketDataBus,
    health: &Arc<RwLock<BTreeMap<String, VenueHealth>>>,
) -> anyhow::Result<()> {
    for symbol in REST_SYMBOLS {
        let url = format!("https://fapi.binance.com/fapi/v1/aggTrades?symbol={symbol}&limit=100");
        let trades = client
            .get(url)
            .timeout(Duration::from_secs(5))
            .send()
            .await?
            .error_for_status()?
            .json::<Vec<RestAggTrade>>()
            .await?;
        for raw in trades {
            let raw = BinanceAggTrade {
                s: symbol.to_string(),
                a: Some(raw.a),
                t: None,
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
    }
    Ok(())
}

async fn fetch_rest_depth(
    client: &reqwest::Client,
    bus: &MarketDataBus,
    health: &Arc<RwLock<BTreeMap<String, VenueHealth>>>,
) -> anyhow::Result<()> {
    for symbol in REST_SYMBOLS {
        let url = format!("https://fapi.binance.com/fapi/v1/depth?symbol={symbol}&limit=20");
        let mut raw = client
            .get(url)
            .timeout(Duration::from_secs(5))
            .send()
            .await?
            .error_for_status()?
            .json::<Depth>()
            .await?;
        raw.s = Some(symbol.to_string());
        handle_depth(raw, bus, health);
    }
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
    if matches!(
        combined.data.get("e").and_then(|v| v.as_str()),
        Some("aggTrade" | "trade")
    ) {
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

#[cfg(test)]
mod tests {
    use super::*;

    fn test_health() -> Arc<RwLock<BTreeMap<String, VenueHealth>>> {
        let mut map = BTreeMap::new();
        map.insert(
            Venue::Binance.as_key().to_string(),
            VenueHealth::start_attempted_with_symbol(Venue::Binance, "BTC-PERP"),
        );
        Arc::new(RwLock::new(map))
    }

    #[test]
    fn combined_trade_message_marks_trade_activity() {
        let bus = MarketDataBus::new(8);
        let health = test_health();
        let mut rx = bus.subscribe();

        handle_message(
            r#"{"stream":"btcusdt@trade","data":{"e":"trade","E":1780252674690,"T":1780252674690,"s":"BTCUSDT","t":7705835712,"p":"73628.00","q":"0.034","X":"MARKET","m":false}}"#,
            &bus,
            &health,
        );

        let trade = loop {
            match rx.try_recv() {
                Ok(MarketDataEvent::Trade(trade)) => break trade,
                Ok(_) => continue,
                Err(error) => panic!("expected trade event, got {error}"),
            }
        };

        assert_eq!(trade.venue, Venue::Binance);
        assert_eq!(trade.symbol, "BTC-PERP");
        assert_eq!(trade.trade_id.as_deref(), Some("7705835712"));

        let snapshot = health.read();
        let binance = snapshot
            .get(Venue::Binance.as_key())
            .expect("binance health");
        assert_eq!(binance.trade_message_count, 1);
        assert!(binance.last_trade_ts.is_some());
        assert!(binance.trade_active);
        assert!(binance.last_parse_error.is_none());
    }

    #[test]
    fn combined_depth_message_marks_book_activity() {
        let bus = MarketDataBus::new(8);
        let health = test_health();
        let mut rx = bus.subscribe();

        handle_message(
            r#"{"stream":"btcusdt@depth20@100ms","data":{"e":"depthUpdate","E":1780252543679,"T":1780252543678,"s":"BTCUSDT","U":10673052221739,"u":10673052230343,"pu":10673052221569,"b":[["73635.80","22.287"],["73635.70","0.010"]],"a":[["73635.90","6.199"],["73636.00","0.003"]]}}"#,
            &bus,
            &health,
        );

        let book = loop {
            match rx.try_recv() {
                Ok(MarketDataEvent::Book(book)) => break book,
                Ok(_) => continue,
                Err(error) => panic!("expected book event, got {error}"),
            }
        };

        assert_eq!(book.venue, Venue::Binance);
        assert_eq!(book.symbol, "BTC-PERP");
        assert_eq!(book.best_bid, 73635.80);
        assert_eq!(book.best_ask, 73635.90);

        let snapshot = health.read();
        let binance = snapshot
            .get(Venue::Binance.as_key())
            .expect("binance health");
        assert_eq!(binance.book_message_count, 1);
        assert!(binance.last_book_ts.is_some());
        assert!(binance.book_active);
        assert!(binance.last_parse_error.is_none());
    }
}
