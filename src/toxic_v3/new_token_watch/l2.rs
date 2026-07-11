//! Local Binance USD-M depth book primitives.
//!
//! These types deliberately model only public L2 evidence.  They do not infer
//! participant identity and a book is unusable until its REST snapshot and
//! websocket sequence are contiguous.

use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};

const PRICE_SCALE: f64 = 100_000_000.0;

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DepthLevel {
    pub price: f64,
    pub quantity: f64,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum BookSide {
    Bid,
    Ask,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct BookLevelView {
    pub side: BookSide,
    pub price: f64,
    pub quantity: f64,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DepthSnapshot {
    pub last_update_id: u64,
    pub bids: Vec<DepthLevel>,
    pub asks: Vec<DepthLevel>,
    pub fetched_at_ms: i64,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DepthDiff {
    pub first_update_id: u64,
    pub final_update_id: u64,
    pub previous_final_update_id: Option<u64>,
    pub bids: Vec<DepthLevel>,
    pub asks: Vec<DepthLevel>,
    pub event_time_ms: i64,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "snake_case")]
pub enum OrderBookReadiness {
    #[default]
    Unavailable,
    Syncing,
    Ready,
    Gap,
    Stale,
}

impl OrderBookReadiness {
    pub fn is_ready(self) -> bool {
        self == Self::Ready
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct OrderBookMetrics {
    pub readiness: OrderBookReadiness,
    pub orderbook_evidence_available: bool,
    pub best_bid: Option<f64>,
    pub best_ask: Option<f64>,
    pub spread_bps: f64,
    pub microprice: Option<f64>,
    pub top_n_bid_quantity: f64,
    pub top_n_ask_quantity: f64,
    pub imbalance: f64,
    pub bid_added_quantity: f64,
    pub bid_removed_quantity: f64,
    pub ask_added_quantity: f64,
    pub ask_removed_quantity: f64,
    pub visible_cancel_to_add_ratio: f64,
    pub last_update_id: Option<u64>,
    pub last_event_time_ms: Option<i64>,
    pub reason: String,
}

impl Default for OrderBookMetrics {
    fn default() -> Self {
        Self {
            readiness: OrderBookReadiness::Unavailable,
            orderbook_evidence_available: false,
            best_bid: None,
            best_ask: None,
            spread_bps: 0.0,
            microprice: None,
            top_n_bid_quantity: 0.0,
            top_n_ask_quantity: 0.0,
            imbalance: 0.0,
            bid_added_quantity: 0.0,
            bid_removed_quantity: 0.0,
            ask_added_quantity: 0.0,
            ask_removed_quantity: 0.0,
            visible_cancel_to_add_ratio: 0.0,
            last_update_id: None,
            last_event_time_ms: None,
            reason: "orderbook_not_ready".to_string(),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct OrderBookError(&'static str);

impl OrderBookError {
    pub fn as_str(&self) -> &'static str {
        self.0
    }
}

/// A Binance futures depth book with explicit sequence-gap protection.
#[derive(Debug, Default)]
pub struct LocalOrderBook {
    bids: BTreeMap<i64, f64>,
    asks: BTreeMap<i64, f64>,
    buffered_diffs: Vec<DepthDiff>,
    last_update_id: Option<u64>,
    last_event_time_ms: Option<i64>,
    readiness: OrderBookReadiness,
    last_delta: DepthDelta,
    synced_after_snapshot: bool,
}

impl LocalOrderBook {
    pub fn buffer_diff(&mut self, diff: DepthDiff) {
        if self.last_update_id.is_some() {
            let _ = self.apply_diff(diff);
            return;
        }
        self.buffered_diffs.push(diff);
        self.readiness = OrderBookReadiness::Syncing;
    }

    pub fn install_snapshot(&mut self, snapshot: DepthSnapshot) {
        self.bids.clear();
        self.asks.clear();
        apply_levels(&mut self.bids, snapshot.bids);
        apply_levels(&mut self.asks, snapshot.asks);
        self.last_delta = DepthDelta::default();
        self.last_update_id = Some(snapshot.last_update_id);
        self.last_event_time_ms = Some(snapshot.fetched_at_ms);
        self.readiness = OrderBookReadiness::Syncing;
        self.synced_after_snapshot = false;

        let mut pending = std::mem::take(&mut self.buffered_diffs);
        pending.sort_by_key(|diff| diff.final_update_id);
        pending.retain(|diff| diff.final_update_id > snapshot.last_update_id);
        for diff in pending {
            if self.apply_diff(diff).is_err() {
                break;
            }
        }
        // A REST snapshot is only a baseline. Keep the book in Syncing until
        // at least one websocket diff proves continuity after the snapshot.
    }

    pub fn apply_diff(&mut self, diff: DepthDiff) -> Result<(), OrderBookError> {
        let Some(last) = self.last_update_id else {
            self.buffer_diff(diff);
            return Err(OrderBookError("snapshot_required"));
        };
        if self.readiness == OrderBookReadiness::Gap {
            return Err(OrderBookError("resync_required"));
        }
        if diff.final_update_id <= last {
            return Ok(());
        }
        let expected = last.saturating_add(1);
        let contiguous = if !self.synced_after_snapshot {
            // Binance's first buffered depth event after a REST snapshot only
            // needs to cover `lastUpdateId + 1`; its `pu` may predate the
            // snapshot because the diff began before the snapshot response.
            diff.first_update_id <= expected && diff.final_update_id >= expected
        } else {
            match diff.previous_final_update_id {
                Some(previous) => previous == last && diff.first_update_id <= expected,
                None => diff.first_update_id <= expected && diff.final_update_id >= expected,
            }
        };
        if !contiguous {
            self.readiness = OrderBookReadiness::Gap;
            self.buffered_diffs.clear();
            return Err(OrderBookError("sequence_gap"));
        }

        let bid_delta = apply_levels(&mut self.bids, diff.bids);
        let ask_delta = apply_levels(&mut self.asks, diff.asks);
        self.last_delta = DepthDelta {
            bid_added_quantity: bid_delta.added_quantity,
            bid_removed_quantity: bid_delta.removed_quantity,
            ask_added_quantity: ask_delta.added_quantity,
            ask_removed_quantity: ask_delta.removed_quantity,
        };
        self.last_update_id = Some(diff.final_update_id);
        self.last_event_time_ms = Some(diff.event_time_ms);
        self.synced_after_snapshot = true;
        self.readiness = OrderBookReadiness::Ready;
        Ok(())
    }

    pub fn mark_stale(&mut self) {
        if self.readiness.is_ready() {
            self.readiness = OrderBookReadiness::Stale;
        }
    }

    /// Invalidates all locally derived L2 evidence. A caller must fetch a
    /// fresh snapshot and re-establish sequence continuity before reuse.
    pub fn invalidate_for_resync(&mut self) {
        self.bids.clear();
        self.asks.clear();
        self.buffered_diffs.clear();
        self.last_update_id = None;
        self.last_event_time_ms = None;
        self.last_delta = DepthDelta::default();
        self.synced_after_snapshot = false;
        self.readiness = OrderBookReadiness::Gap;
    }

    pub fn readiness(&self) -> OrderBookReadiness {
        self.readiness
    }

    pub fn last_update_id(&self) -> Option<u64> {
        self.last_update_id
    }

    pub fn metrics(&self, depth: usize) -> OrderBookMetrics {
        if !self.readiness.is_ready() {
            return OrderBookMetrics {
                readiness: self.readiness,
                last_update_id: self.last_update_id,
                last_event_time_ms: self.last_event_time_ms,
                reason: match self.readiness {
                    OrderBookReadiness::Gap => "sequence_gap_resync_required",
                    OrderBookReadiness::Stale => "orderbook_stale",
                    _ => "orderbook_not_ready",
                }
                .to_string(),
                ..Default::default()
            };
        }
        let bids = self
            .bids
            .iter()
            .rev()
            .take(depth)
            .map(|(price, quantity)| (price_from_key(*price), *quantity))
            .collect::<Vec<_>>();
        let asks = self
            .asks
            .iter()
            .take(depth)
            .map(|(price, quantity)| (price_from_key(*price), *quantity))
            .collect::<Vec<_>>();
        let best_bid = bids.first().map(|(price, _)| *price);
        let best_ask = asks.first().map(|(price, _)| *price);
        let bid_quantity = bids.iter().map(|(_, quantity)| quantity).sum::<f64>();
        let ask_quantity = asks.iter().map(|(_, quantity)| quantity).sum::<f64>();
        let total = bid_quantity + ask_quantity;
        let imbalance = if total > 0.0 {
            (bid_quantity - ask_quantity) / total
        } else {
            0.0
        };
        let (spread_bps, microprice) = match (best_bid, best_ask) {
            (Some(bid), Some(ask)) if bid > 0.0 && ask >= bid => {
                let mid = (bid + ask) / 2.0;
                let top_bid = bids
                    .first()
                    .map(|(_, quantity)| *quantity)
                    .unwrap_or_default();
                let top_ask = asks
                    .first()
                    .map(|(_, quantity)| *quantity)
                    .unwrap_or_default();
                let micro = if top_bid + top_ask > 0.0 {
                    (ask * top_bid + bid * top_ask) / (top_bid + top_ask)
                } else {
                    mid
                };
                (((ask - bid) / mid * 10_000.0), Some(micro))
            }
            _ => (0.0, None),
        };
        OrderBookMetrics {
            readiness: self.readiness,
            orderbook_evidence_available: true,
            best_bid,
            best_ask,
            spread_bps,
            microprice,
            top_n_bid_quantity: bid_quantity,
            top_n_ask_quantity: ask_quantity,
            imbalance,
            bid_added_quantity: self.last_delta.bid_added_quantity,
            bid_removed_quantity: self.last_delta.bid_removed_quantity,
            ask_added_quantity: self.last_delta.ask_added_quantity,
            ask_removed_quantity: self.last_delta.ask_removed_quantity,
            visible_cancel_to_add_ratio: self.last_delta.cancel_to_add_ratio(),
            last_update_id: self.last_update_id,
            last_event_time_ms: self.last_event_time_ms,
            reason: "orderbook_ready".to_string(),
        }
    }

    pub fn top_levels(&self, depth: usize) -> Vec<BookLevelView> {
        if !self.readiness.is_ready() {
            return vec![];
        }
        let mut levels = self
            .bids
            .iter()
            .rev()
            .take(depth)
            .map(|(price, quantity)| BookLevelView {
                side: BookSide::Bid,
                price: price_from_key(*price),
                quantity: *quantity,
            })
            .collect::<Vec<_>>();
        levels.extend(
            self.asks
                .iter()
                .take(depth)
                .map(|(price, quantity)| BookLevelView {
                    side: BookSide::Ask,
                    price: price_from_key(*price),
                    quantity: *quantity,
                }),
        );
        levels
    }
}

#[derive(Debug, Clone, Copy, Default)]
struct SideDelta {
    added_quantity: f64,
    removed_quantity: f64,
}

#[derive(Debug, Clone, Copy, Default)]
struct DepthDelta {
    bid_added_quantity: f64,
    bid_removed_quantity: f64,
    ask_added_quantity: f64,
    ask_removed_quantity: f64,
}

impl DepthDelta {
    fn cancel_to_add_ratio(self) -> f64 {
        let added = self.bid_added_quantity + self.ask_added_quantity;
        let removed = self.bid_removed_quantity + self.ask_removed_quantity;
        if added > 0.0 {
            removed / added
        } else if removed > 0.0 {
            removed
        } else {
            0.0
        }
    }
}

fn apply_levels(book: &mut BTreeMap<i64, f64>, levels: Vec<DepthLevel>) -> SideDelta {
    let mut delta = SideDelta::default();
    for level in levels {
        if !level.price.is_finite() || !level.quantity.is_finite() || level.price <= 0.0 {
            continue;
        }
        let key = price_key(level.price);
        let previous = book.get(&key).copied().unwrap_or_default();
        if level.quantity <= 0.0 {
            book.remove(&key);
            delta.removed_quantity += previous.max(0.0);
        } else {
            book.insert(key, level.quantity);
            if level.quantity >= previous {
                delta.added_quantity += level.quantity - previous;
            } else {
                delta.removed_quantity += previous - level.quantity;
            }
        }
    }
    delta
}

fn price_key(price: f64) -> i64 {
    (price * PRICE_SCALE).round() as i64
}

fn price_from_key(key: i64) -> f64 {
    key as f64 / PRICE_SCALE
}
