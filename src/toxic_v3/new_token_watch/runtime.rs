//! Dedicated public Binance USD-M L2 session runtime.
//!
//! Every selected symbol owns a separate public websocket lifecycle. The
//! runtime keeps only in-memory depth state and compact metrics; it does not
//! persist raw high-frequency depth updates or perform any exchange action.

use std::{collections::BTreeMap, sync::Arc, time::Duration};

use futures_util::StreamExt;
use parking_lot::RwLock;
use serde::Deserialize;
use tokio::sync::watch;
use tokio_tungstenite::connect_async;

use crate::storage::{new_token_l2_repo::NewTokenL2Repo, SqliteStore};

use super::{
    l2::{DepthDiff, DepthLevel, DepthSnapshot},
    session::{L2SessionRegistry, L2SessionSnapshot},
    shadow::ShadowOutcomeTracker,
};

const BINANCE_FUTURES_REST: &str = "https://fapi.binance.com";
const BINANCE_FUTURES_WS: &str = "wss://fstream.binance.com/stream?streams=";
const RECONNECT_DELAY: Duration = Duration::from_secs(2);

#[derive(Debug, Clone)]
pub struct BinanceL2Runtime {
    registry: L2SessionRegistry,
    stops: Arc<RwLock<BTreeMap<String, watch::Sender<bool>>>>,
    store: Arc<RwLock<Option<SqliteStore>>>,
    last_metric_persisted_ms: Arc<RwLock<BTreeMap<String, i64>>>,
    enabled: bool,
}

impl Default for BinanceL2Runtime {
    fn default() -> Self {
        Self {
            registry: L2SessionRegistry::default(),
            stops: Arc::new(RwLock::new(BTreeMap::new())),
            store: Arc::new(RwLock::new(None)),
            last_metric_persisted_ms: Arc::new(RwLock::new(BTreeMap::new())),
            // Tests and explicitly constructed runtimes opt into L2; the
            // production global is created through `from_env` below.
            enabled: true,
        }
    }
}

impl BinanceL2Runtime {
    pub fn from_env() -> Self {
        let mut runtime = Self::default();
        runtime.enabled = std::env::var("NEW_TOKEN_L2_ENABLED")
            .ok()
            .is_some_and(|value| value.eq_ignore_ascii_case("true"));
        runtime
    }
    pub fn registry(&self) -> L2SessionRegistry {
        self.registry.clone()
    }

    pub fn configure_store(&self, store: Option<SqliteStore>) {
        *self.store.write() = store;
    }

    pub fn session(&self, symbol: &str) -> Option<L2SessionSnapshot> {
        self.registry.session(symbol)
    }

    pub fn sessions(&self) -> Vec<L2SessionSnapshot> {
        self.registry.sessions()
    }

    pub fn start_symbol(&self, raw_symbol: &str) -> L2SessionSnapshot {
        self.start_symbol_at(raw_symbol, crate::normalizers::trade::now_ms())
    }

    pub fn start_symbol_at(&self, raw_symbol: &str, activated_at_ms: i64) -> L2SessionSnapshot {
        let symbol = raw_symbol.trim().to_ascii_uppercase();
        let snapshot = self.registry.register_at(&symbol, activated_at_ms);
        if !self.enabled {
            self.registry.mark_disabled(&symbol);
            return self.registry.session(&symbol).unwrap_or(snapshot);
        }
        if self.stops.read().contains_key(&symbol) {
            return snapshot;
        }
        let (stop_tx, stop_rx) = watch::channel(false);
        self.stops.write().insert(symbol.clone(), stop_tx);
        let registry = self.registry.clone();
        let stops = Arc::clone(&self.stops);
        let store = Arc::clone(&self.store);
        let last_metric_persisted_ms = Arc::clone(&self.last_metric_persisted_ms);
        tokio::spawn(async move {
            run_symbol_session(
                symbol.clone(),
                registry,
                stop_rx,
                store,
                last_metric_persisted_ms,
            )
            .await;
            stops.write().remove(&symbol);
        });
        snapshot
    }

    pub fn stop_symbol(&self, raw_symbol: &str) -> bool {
        let symbol = raw_symbol.trim().to_ascii_uppercase();
        if let Some(stop_tx) = self.stops.write().remove(&symbol) {
            let _ = stop_tx.send(true);
        }
        self.registry.remove(&symbol)
    }

    pub fn restore_symbols<I>(&self, symbols: I)
    where
        I: IntoIterator,
        I::Item: AsRef<str>,
    {
        for symbol in symbols {
            self.start_symbol(symbol.as_ref());
        }
    }

    pub fn restore_symbols_at<I>(&self, symbols: I)
    where
        I: IntoIterator<Item = (String, i64)>,
    {
        for (symbol, activated_at_ms) in symbols {
            self.start_symbol_at(&symbol, activated_at_ms);
        }
    }
}

pub async fn validate_binance_usdm_symbol(raw_symbol: &str) -> Result<(), String> {
    let symbol = raw_symbol.trim().to_ascii_uppercase();
    let endpoint = format!("{BINANCE_FUTURES_REST}/fapi/v1/exchangeInfo?symbol={symbol}");
    let response = reqwest::Client::new()
        .get(endpoint)
        .send()
        .await
        .map_err(|_| "binance_exchange_info_unavailable".to_string())?;
    if !response.status().is_success() {
        return Err("binance_usdm_symbol_not_found".to_string());
    }
    let info = response
        .json::<BinanceExchangeInfo>()
        .await
        .map_err(|_| "binance_exchange_info_invalid".to_string())?;
    let valid = info.symbols.into_iter().any(|candidate| {
        candidate.symbol.eq_ignore_ascii_case(&symbol)
            && candidate.quote_asset.eq_ignore_ascii_case("USDT")
            && candidate.contract_type.eq_ignore_ascii_case("PERPETUAL")
            && candidate.status.eq_ignore_ascii_case("TRADING")
    });
    valid
        .then_some(())
        .ok_or_else(|| "binance_usdm_contract_not_trading".to_string())
}

async fn run_symbol_session(
    symbol: String,
    registry: L2SessionRegistry,
    mut stop: watch::Receiver<bool>,
    store: Arc<RwLock<Option<SqliteStore>>>,
    last_metric_persisted_ms: Arc<RwLock<BTreeMap<String, i64>>>,
) {
    loop {
        if *stop.borrow() {
            return;
        }
        registry.mark_reconnecting(&symbol, None);
        let stream_symbol = symbol.to_ascii_lowercase();
        let url =
            format!("{BINANCE_FUTURES_WS}{stream_symbol}@depth@100ms/{stream_symbol}@bookTicker/{stream_symbol}@aggTrade/{stream_symbol}@markPrice@1s");
        let connection = tokio::select! {
            _ = stop.changed() => return,
            result = connect_async(&url) => result,
        };
        let Ok((socket, _)) = connection else {
            registry.mark_reconnecting(&symbol, Some("binance_l2_connect_failed".to_string()));
            if wait_or_stop(&mut stop).await {
                return;
            }
            continue;
        };
        registry.set_syncing(&symbol);
        let (_, mut reader) = socket.split();
        let client = reqwest::Client::new();
        let snapshot_symbol = symbol.clone();
        let mut snapshot_task =
            tokio::spawn(async move { fetch_snapshot(&client, &snapshot_symbol).await });
        let mut snapshot_installed = false;
        let mut shadow_tracker = ShadowOutcomeTracker::default();
        let mut oi_poll = tokio::time::interval(Duration::from_secs(15));
        loop {
            tokio::select! {
                _ = stop.changed() => return,
                _ = oi_poll.tick(), if snapshot_installed => {
                    if let Ok(oi) = fetch_open_interest(&symbol).await {
                        registry.record_open_interest(&symbol, oi.value, oi.updated_at_ms);
                    } else {
                        tracing::debug!(symbol = %symbol, "new-token OI context unavailable; keeping L2 evidence degraded");
                    }
                }
                result = &mut snapshot_task, if !snapshot_installed => {
                    snapshot_installed = true;
                    match result {
                        Ok(Ok(snapshot)) => registry.install_snapshot(&symbol, snapshot),
                        _ => {
                            registry.mark_error(&symbol, "binance_l2_snapshot_failed");
                            break;
                        }
                    }
                }
                message = reader.next() => {
                    let Some(message) = message else { break; };
                    let Ok(message) = message else { break; };
                    let payload = match message {
                        tokio_tungstenite::tungstenite::Message::Text(text) => text.to_string(),
                        tokio_tungstenite::tungstenite::Message::Binary(bytes) => match String::from_utf8(bytes.to_vec()) {
                            Ok(text) => text,
                            Err(_) => continue,
                        },
                        tokio_tungstenite::tungstenite::Message::Close(_) => { break; }
                        _ => continue,
                    };
                    match parse_stream_message(&payload) {
                        Ok(StreamMessage::Depth(diff)) if !snapshot_installed => registry.registry_buffer_diff(&symbol, diff),
                        Ok(StreamMessage::Depth(diff)) => {
                            if registry.apply_diff(&symbol, diff).is_err() {
                                break;
                            }
                            record_shadow_outcomes_if_due(
                                &symbol,
                                &registry,
                                &mut shadow_tracker,
                                &store,
                            );
                            persist_compact_metric_if_due(
                                &symbol,
                                &registry,
                                &store,
                                &last_metric_persisted_ms,
                            );
                        }
                        Ok(StreamMessage::BookTicker { bid, ask, event_time_ms }) => {
                            if registry.record_book_ticker(&symbol, bid, ask, event_time_ms) {
                                break;
                            }
                        }
                        Ok(StreamMessage::AggTrade { price, quantity, buyer_is_maker, event_time_ms }) => {
                            registry.record_agg_trade(&symbol, price, quantity, buyer_is_maker, event_time_ms);
                        }
                        Ok(StreamMessage::MarkPrice { price, event_time_ms }) => {
                            registry.record_mark_price(&symbol, price, event_time_ms);
                        }
                        Err(_) => continue,
                    }
                }
            }
        }
        registry.mark_reconnecting(&symbol, Some("binance_l2_reconnect".to_string()));
        if wait_or_stop(&mut stop).await {
            return;
        }
    }
}

fn persist_compact_metric_if_due(
    symbol: &str,
    registry: &L2SessionRegistry,
    store: &Arc<RwLock<Option<SqliteStore>>>,
    last_metric_persisted_ms: &Arc<RwLock<BTreeMap<String, i64>>>,
) {
    const METRIC_INTERVAL_MS: i64 = 1_000;
    let now_ms = crate::normalizers::trade::now_ms();
    {
        let mut last_seen = last_metric_persisted_ms.write();
        if last_seen
            .get(symbol)
            .is_some_and(|previous| now_ms.saturating_sub(*previous) < METRIC_INTERVAL_MS)
        {
            return;
        }
        last_seen.insert(symbol.to_string(), now_ms);
    }
    let Some(store) = store.read().clone() else {
        return;
    };
    let Some(session) = registry.session(symbol) else {
        return;
    };
    tokio::task::spawn_blocking(move || {
        if let Err(error) = store.insert_new_token_l2_metric(now_ms, &session) {
            tracing::warn!(symbol = %session.symbol, error = %error, "new-token compact L2 metric persistence failed");
        }
    });
}

fn record_shadow_outcomes_if_due(
    symbol: &str,
    registry: &L2SessionRegistry,
    tracker: &mut ShadowOutcomeTracker,
    store: &Arc<RwLock<Option<SqliteStore>>>,
) {
    let Some(session) = registry.session(symbol) else {
        return;
    };
    let Some(entry_price) = midpoint(&session) else {
        return;
    };
    let observed_at_ms = session.orderbook.last_event_time_ms.unwrap_or_default();
    tracker.observe_intent(symbol, observed_at_ms, entry_price, &session.intent);
    let outcomes = tracker.observe_price(symbol, observed_at_ms, entry_price);
    if outcomes.is_empty() {
        return;
    }
    let Some(store) = store.read().clone() else {
        return;
    };
    tokio::task::spawn_blocking(move || {
        if let Err(error) = store.upsert_new_token_l2_shadow_outcomes(&outcomes) {
            tracing::warn!(error = %error, "new-token L2 shadow outcome persistence failed");
        }
    });
}

fn midpoint(session: &L2SessionSnapshot) -> Option<f64> {
    match (session.orderbook.best_bid, session.orderbook.best_ask) {
        (Some(bid), Some(ask)) if bid > 0.0 && ask >= bid => Some((bid + ask) / 2.0),
        _ => None,
    }
}

async fn wait_or_stop(stop: &mut watch::Receiver<bool>) -> bool {
    tokio::select! {
        _ = stop.changed() => true,
        _ = tokio::time::sleep(RECONNECT_DELAY) => false,
    }
}

async fn fetch_snapshot(client: &reqwest::Client, symbol: &str) -> Result<DepthSnapshot, String> {
    let endpoint = format!("{BINANCE_FUTURES_REST}/fapi/v1/depth?symbol={symbol}&limit=1000");
    let response = client
        .get(endpoint)
        .send()
        .await
        .map_err(|_| "binance_l2_snapshot_unavailable".to_string())?;
    if !response.status().is_success() {
        return Err("binance_l2_snapshot_rejected".to_string());
    }
    let payload = response
        .json::<BinanceDepthSnapshot>()
        .await
        .map_err(|_| "binance_l2_snapshot_invalid".to_string())?;
    Ok(DepthSnapshot {
        last_update_id: payload.last_update_id,
        bids: parse_levels(payload.bids),
        asks: parse_levels(payload.asks),
        fetched_at_ms: crate::normalizers::trade::now_ms(),
    })
}

#[derive(Debug)]
struct OpenInterestValue {
    value: f64,
    updated_at_ms: i64,
}

async fn fetch_open_interest(symbol: &str) -> Result<OpenInterestValue, String> {
    let endpoint = format!("{BINANCE_FUTURES_REST}/fapi/v1/openInterest?symbol={symbol}");
    let payload = reqwest::Client::new()
        .get(endpoint)
        .send()
        .await
        .map_err(|_| "binance_open_interest_unavailable".to_string())?
        .error_for_status()
        .map_err(|_| "binance_open_interest_rejected".to_string())?
        .json::<BinanceOpenInterest>()
        .await
        .map_err(|_| "binance_open_interest_invalid".to_string())?;
    Ok(OpenInterestValue {
        value: payload
            .open_interest
            .parse()
            .map_err(|_| "binance_open_interest_value_invalid".to_string())?,
        updated_at_ms: payload.time,
    })
}

enum StreamMessage {
    Depth(DepthDiff),
    BookTicker {
        bid: f64,
        ask: f64,
        event_time_ms: i64,
    },
    AggTrade {
        price: f64,
        quantity: f64,
        buyer_is_maker: bool,
        event_time_ms: i64,
    },
    MarkPrice {
        price: f64,
        event_time_ms: i64,
    },
}

fn parse_stream_message(payload: &str) -> Result<StreamMessage, String> {
    let wrapped = serde_json::from_str::<BinanceCombinedStream>(payload)
        .map_err(|_| "binance_l2_ws_invalid".to_string())?;
    let event = wrapped
        .data
        .get("e")
        .and_then(|value| value.as_str())
        .unwrap_or_default();
    match event {
        "depthUpdate" => {
            let depth: BinanceDepthUpdate = serde_json::from_value(wrapped.data)
                .map_err(|_| "binance_l2_depth_invalid".to_string())?;
            Ok(StreamMessage::Depth(DepthDiff {
                first_update_id: depth.first_update_id,
                final_update_id: depth.final_update_id,
                previous_final_update_id: depth.previous_final_update_id,
                bids: parse_levels(depth.bids),
                asks: parse_levels(depth.asks),
                event_time_ms: depth.event_time_ms,
            }))
        }
        "bookTicker" => {
            let ticker: BinanceBookTicker = serde_json::from_value(wrapped.data)
                .map_err(|_| "binance_l2_bookticker_invalid".to_string())?;
            Ok(StreamMessage::BookTicker {
                bid: ticker
                    .bid
                    .parse()
                    .map_err(|_| "binance_l2_bid_invalid".to_string())?,
                ask: ticker
                    .ask
                    .parse()
                    .map_err(|_| "binance_l2_ask_invalid".to_string())?,
                event_time_ms: ticker.event_time_ms,
            })
        }
        "aggTrade" => {
            let trade: BinanceAggTrade = serde_json::from_value(wrapped.data)
                .map_err(|_| "binance_l2_agg_trade_invalid".to_string())?;
            Ok(StreamMessage::AggTrade {
                price: trade
                    .price
                    .parse()
                    .map_err(|_| "binance_l2_trade_price_invalid".to_string())?,
                quantity: trade
                    .quantity
                    .parse()
                    .map_err(|_| "binance_l2_trade_quantity_invalid".to_string())?,
                buyer_is_maker: trade.buyer_is_maker,
                event_time_ms: trade.event_time_ms,
            })
        }
        "markPriceUpdate" => {
            let mark: BinanceMarkPrice = serde_json::from_value(wrapped.data)
                .map_err(|_| "binance_l2_mark_price_invalid".to_string())?;
            Ok(StreamMessage::MarkPrice {
                price: mark
                    .mark_price
                    .parse()
                    .map_err(|_| "binance_l2_mark_price_value_invalid".to_string())?,
                event_time_ms: mark.event_time_ms,
            })
        }
        _ => Err("binance_l2_event_ignored".to_string()),
    }
}

fn parse_levels(levels: Vec<(String, String)>) -> Vec<DepthLevel> {
    levels
        .into_iter()
        .filter_map(|(price, quantity)| {
            Some(DepthLevel {
                price: price.parse().ok()?,
                quantity: quantity.parse().ok()?,
            })
        })
        .collect()
}

#[derive(Debug, Deserialize)]
struct BinanceExchangeInfo {
    symbols: Vec<BinanceExchangeInfoSymbol>,
}

#[derive(Debug, Deserialize)]
struct BinanceExchangeInfoSymbol {
    symbol: String,
    #[serde(rename = "quoteAsset")]
    quote_asset: String,
    #[serde(rename = "contractType")]
    contract_type: String,
    status: String,
}

#[derive(Debug, Deserialize)]
struct BinanceDepthSnapshot {
    #[serde(rename = "lastUpdateId")]
    last_update_id: u64,
    bids: Vec<(String, String)>,
    asks: Vec<(String, String)>,
}

#[derive(Debug, Deserialize)]
struct BinanceCombinedStream {
    data: serde_json::Value,
}

#[derive(Debug, Deserialize)]
struct BinanceDepthUpdate {
    #[serde(rename = "U")]
    first_update_id: u64,
    #[serde(rename = "u")]
    final_update_id: u64,
    #[serde(rename = "pu")]
    previous_final_update_id: Option<u64>,
    #[serde(rename = "E")]
    event_time_ms: i64,
    #[serde(rename = "b")]
    bids: Vec<(String, String)>,
    #[serde(rename = "a")]
    asks: Vec<(String, String)>,
}

#[derive(Debug, Deserialize)]
struct BinanceBookTicker {
    #[serde(rename = "E")]
    event_time_ms: i64,
    #[serde(rename = "b")]
    bid: String,
    #[serde(rename = "a")]
    ask: String,
}

#[derive(Debug, Deserialize)]
struct BinanceAggTrade {
    #[serde(rename = "E")]
    event_time_ms: i64,
    #[serde(rename = "p")]
    price: String,
    #[serde(rename = "q")]
    quantity: String,
    #[serde(rename = "m")]
    buyer_is_maker: bool,
}

#[derive(Debug, Deserialize)]
struct BinanceMarkPrice {
    #[serde(rename = "E")]
    event_time_ms: i64,
    #[serde(rename = "p")]
    mark_price: String,
}

#[derive(Debug, Deserialize)]
struct BinanceOpenInterest {
    #[serde(rename = "openInterest")]
    open_interest: String,
    time: i64,
}

#[cfg(test)]
mod tests {
    use super::{parse_stream_message, StreamMessage};

    #[test]
    fn parses_binance_depth_book_ticker_trade_and_mark_messages() {
        let depth = parse_stream_message(
            r#"{"stream":"asterusdt@depth@100ms","data":{"e":"depthUpdate","E":100,"U":5,"u":6,"pu":4,"b":[["1.00","3"]],"a":[["1.01","2"]]}}"#,
        )
        .expect("depth message");
        match depth {
            StreamMessage::Depth(diff) => {
                assert_eq!(diff.first_update_id, 5);
                assert_eq!(diff.final_update_id, 6);
                assert_eq!(diff.previous_final_update_id, Some(4));
            }
            _ => panic!("expected depth message"),
        }

        let ticker = parse_stream_message(
            r#"{"stream":"asterusdt@bookTicker","data":{"e":"bookTicker","E":101,"b":"1.00","a":"1.01"}}"#,
        )
        .expect("book ticker message");
        match ticker {
            StreamMessage::BookTicker {
                bid,
                ask,
                event_time_ms,
            } => {
                assert_eq!(bid, 1.0);
                assert_eq!(ask, 1.01);
                assert_eq!(event_time_ms, 101);
            }
            _ => panic!("expected book ticker message"),
        }

        let trade = parse_stream_message(
            r#"{"stream":"asterusdt@aggTrade","data":{"e":"aggTrade","E":102,"p":"1.02","q":"3","m":false}}"#,
        )
        .expect("agg trade message");
        match trade {
            StreamMessage::AggTrade {
                price,
                quantity,
                buyer_is_maker,
                event_time_ms,
            } => {
                assert_eq!(price, 1.02);
                assert_eq!(quantity, 3.0);
                assert!(!buyer_is_maker);
                assert_eq!(event_time_ms, 102);
            }
            _ => panic!("expected agg trade message"),
        }

        let mark = parse_stream_message(
            r#"{"stream":"asterusdt@markPrice@1s","data":{"e":"markPriceUpdate","E":103,"p":"1.015"}}"#,
        )
        .expect("mark price message");
        match mark {
            StreamMessage::MarkPrice {
                price,
                event_time_ms,
            } => {
                assert_eq!(price, 1.015);
                assert_eq!(event_time_ms, 103);
            }
            _ => panic!("expected mark price message"),
        }
    }
}
