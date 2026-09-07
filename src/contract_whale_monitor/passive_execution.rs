//! Causal, anonymous Binance execution evidence, not account or hidden-order identity.
use crate::{
    market_data::event_bus::{MarketDataBus, MarketDataEvent},
    types::market::{AggressorSide, NormalizedBook, NormalizedTrade, Venue, VenueConnectionStatus},
};
use parking_lot::RwLock;
use std::{
    collections::{BTreeMap, BTreeSet, VecDeque},
    sync::Arc,
};

const RETAIN_MS: i64 = 300_000;
const MAX_PAIR_GAP_MS: i64 = 250;
const MIN_MATCHED_USD: f64 = 1_000_000.0;

#[derive(Debug, Clone, Default, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PassiveExecutionEvidence {
    pub version: String,
    pub status: String,
    pub side: String,
    pub window_sec: u64,
    pub assessed_at_ms: i64,
    pub coverage: f64,
    pub independent_bins: usize,
    pub matched_notional_usd: f64,
    pub replenished_notional_usd: f64,
    pub reasons: Vec<String>,
}

#[derive(Default)]
struct SymbolState {
    book: Option<NormalizedBook>,
    pending: Vec<NormalizedTrade>,
    seen: BTreeMap<String, i64>,
    intervals: VecDeque<Interval>,
    pruned_at: i64,
}

struct Interval {
    start: i64,
    end: i64,
    // Index 0 = passive bid / aggressive sell; index 1 = passive ask / aggressive buy.
    total: [f64; 2],
    matched: [f64; 2],
    replenished: [f64; 2],
}

#[derive(Default)]
pub struct PassiveExecutionTracker {
    symbols: BTreeMap<String, SymbolState>,
}

fn symbol_key(raw: &str) -> Option<&'static str> {
    match raw.to_ascii_uppercase().as_str() {
        "BTC" | "BTCUSDT" | "BTC-PERP" => Some("BTC"),
        "ETH" | "ETHUSDT" | "ETH-PERP" => Some("ETH"),
        _ => None,
    }
}

fn valid_book(book: &NormalizedBook) -> bool {
    book.ts > 0
        && book.best_bid.is_finite()
        && book.best_ask.is_finite()
        && book.best_bid > 0.0
        && book.best_ask > book.best_bid
        && book.bids.first().is_some_and(|row| row.0 == book.best_bid)
        && book.asks.first().is_some_and(|row| row.0 == book.best_ask)
        && book.bids.len() <= 20
        && book.asks.len() <= 20
        && book
            .bids
            .iter()
            .chain(&book.asks)
            .all(|(p, q)| p.is_finite() && *p > 0.0 && q.is_finite() && *q > 0.0)
        && book.bids.windows(2).all(|pair| pair[0].0 > pair[1].0)
        && book.asks.windows(2).all(|pair| pair[0].0 < pair[1].0)
}

impl PassiveExecutionTracker {
    pub fn on_book(&mut self, book: &NormalizedBook) {
        if book.venue != Venue::Binance {
            return;
        }
        let Some(key) = symbol_key(&book.symbol) else {
            return;
        };
        let state = self.symbols.entry(key.into()).or_default();
        if state
            .book
            .as_ref()
            .is_some_and(|previous| book.ts <= previous.ts)
        {
            return;
        }
        if !valid_book(book) {
            *state = SymbolState::default();
            return;
        }
        if let Some(previous) = state
            .book
            .as_ref()
            .filter(|previous| book.ts - previous.ts <= MAX_PAIR_GAP_MS)
        {
            let mut interval = Interval {
                start: previous.ts,
                end: book.ts,
                total: [0.0; 2],
                matched: [0.0; 2],
                replenished: [0.0; 2],
            };
            for trade in state
                .pending
                .iter()
                .filter(|trade| trade.ts > previous.ts && trade.ts <= book.ts)
            {
                let side = usize::from(trade.aggressor_side == AggressorSide::Buy);
                interval.total[side] += trade.size_usd;
            }
            for side in 0..2 {
                let (old_levels, new_levels, held) = if side == 0 {
                    (
                        &previous.bids,
                        &book.bids,
                        book.best_bid >= previous.best_bid,
                    )
                } else {
                    (
                        &previous.asks,
                        &book.asks,
                        book.best_ask <= previous.best_ask,
                    )
                };
                if !held {
                    continue;
                }
                // Only a price visible on both snapshots can support replenishment.
                // Missing top-20 levels are unknown, never classified as cancelled.
                for &(price, old_qty) in old_levels {
                    let Some(&(_, new_qty)) = new_levels.iter().find(|row| row.0 == price) else {
                        continue;
                    };
                    let executed = state
                        .pending
                        .iter()
                        .filter(|trade| {
                            trade.ts > previous.ts
                                && trade.ts <= book.ts
                                && trade.price == price
                                && usize::from(trade.aggressor_side == AggressorSide::Buy) == side
                        })
                        .map(|trade| trade.size_btc)
                        .sum::<f64>();
                    if executed <= 0.0 {
                        continue;
                    }
                    interval.matched[side] += executed * price;
                    // Net visible replenishment after observed executions. This is
                    // anonymous market-level support, not a hidden-order identity.
                    interval.replenished[side] +=
                        (new_qty - old_qty + executed).max(0.0).min(executed) * price;
                }
            }
            state.intervals.push_back(interval);
        }
        state
            .pending
            .retain(|trade| trade.ts > book.ts && trade.ts <= book.ts + 1_000);
        state.book = Some(book.clone());
        while state
            .intervals
            .front()
            .is_some_and(|row| row.end < book.ts - RETAIN_MS)
            || state.intervals.len() > 3500
        {
            state.intervals.pop_front();
        }
        if book.ts - state.pruned_at >= 1000 {
            // Older event timestamps are already rejected against the latest
            // book. Keep only the short reorder horizon, not every fill for 5m.
            state.seen.retain(|_, ts| *ts >= book.ts - 2_000);
            state.pruned_at = book.ts;
        }
    }

    pub fn on_trade(&mut self, trade: &NormalizedTrade) {
        if trade.venue != Venue::Binance
            || trade.ts <= 0
            || !trade.price.is_finite()
            || trade.price <= 0.0
            || !trade.size_btc.is_finite()
            || trade.size_btc <= 0.0
            || !trade.size_usd.is_finite()
            || (trade.size_usd - trade.price * trade.size_btc).abs() > trade.size_usd.abs() * 0.001
        {
            return;
        }
        let Some(key) = symbol_key(&trade.symbol) else {
            return;
        };
        let Some(id) = trade
            .trade_id
            .as_ref()
            .filter(|id| !id.is_empty() && id.len() <= 96)
        else {
            return;
        };
        let state = self.symbols.entry(key.into()).or_default();
        if state
            .book
            .as_ref()
            .is_none_or(|book| trade.ts <= book.ts || trade.ts > book.ts + 1_000)
            || state.seen.contains_key(id)
        {
            return;
        }
        if state.pending.len() >= 4096 || state.seen.len() >= 20_000 {
            // Overload invalidates continuity, instead of dropping fills and
            // later presenting the remaining observations as a complete window.
            *state = SymbolState::default();
            return;
        }
        state.seen.insert(id.clone(), trade.ts);
        state.pending.push(trade.clone());
    }

    pub fn reset(&mut self) {
        self.symbols.clear();
    }

    pub fn evidence(&self, symbol: &str, at: i64, window_sec: u64) -> PassiveExecutionEvidence {
        let mut value = PassiveExecutionEvidence {
            version: "binance_passive_v1".into(),
            status: "insufficient_evidence".into(),
            side: "unknown".into(),
            assessed_at_ms: at,
            window_sec,
            reasons: vec!["anonymous_top20_execution_evidence_not_account_identity".into()],
            ..Default::default()
        };
        if window_sec == 0 || window_sec > 300 {
            value.reasons.push("window_outside_retained_books".into());
            return value;
        }
        let Some(state) = symbol_key(symbol).and_then(|key| self.symbols.get(key)) else {
            return value;
        };
        let start = at.saturating_sub(window_sec as i64 * 1000);
        let rows = state
            .intervals
            .iter()
            .filter(|row| row.start >= start && row.end <= at)
            .collect::<Vec<_>>();
        let mut total = [0.0; 2];
        let mut matched = [0.0; 2];
        let mut refill = [0.0; 2];
        for row in &rows {
            for side in 0..2 {
                total[side] += row.total[side];
                matched[side] += row.matched[side];
                refill[side] += row.replenished[side];
            }
        }
        value.coverage = (rows.iter().map(|row| row.end - row.start).sum::<i64>() as f64
            / (window_sec as f64 * 1000.0))
            .clamp(0.0, 1.0);
        let side = usize::from(total[1] > total[0]);
        value.matched_notional_usd = matched[side];
        value.replenished_notional_usd = refill[side];
        value.independent_bins = rows
            .iter()
            .filter(|row| row.replenished[side] > 0.0)
            .map(|row| (row.end - 1).div_euclid(10_000))
            .collect::<BTreeSet<_>>()
            .len();
        let fresh = rows
            .last()
            .is_some_and(|row| at - row.end <= MAX_PAIR_GAP_MS);
        let supported = fresh
            && value.coverage >= 0.9
            && value.independent_bins >= 3
            && rows
                .first()
                .zip(rows.last())
                .is_some_and(|(first, last)| last.end - first.start >= 20_000)
            && total[side] / (total[0] + total[1]).max(1.0) >= 0.65
            && matched[side] >= MIN_MATCHED_USD
            && refill[side] >= matched[side] * 0.25;
        if supported {
            value.status = "supported".into();
            value.side = if side == 0 { "buy" } else { "sell" }.into();
            value.reasons.push("matched_execution_replenishment".into());
        } else {
            value
                .reasons
                .push("continuity_direction_or_matched_execution_insufficient".into());
        }
        value
    }
}

impl PassiveExecutionEvidence {
    pub fn supports(&self, signal: &super::types::ContractWhaleSignal) -> bool {
        self.status == "supported"
            && self.version == "binance_passive_v1"
            && (20..=300).contains(&self.window_sec)
            && self.assessed_at_ms == signal.ts
            && self.window_sec == signal.window_sec
            && self.coverage.is_finite()
            && (0.9..=1.0).contains(&self.coverage)
            && self.independent_bins >= 3
            && self.matched_notional_usd.is_finite()
            && self.matched_notional_usd >= MIN_MATCHED_USD
            && self.replenished_notional_usd.is_finite()
            && self.replenished_notional_usd >= self.matched_notional_usd * 0.25
            && self.replenished_notional_usd <= self.matched_notional_usd
            && matches!(
                (
                    signal.classification_v2.structure_interpretation,
                    self.side.as_str()
                ),
                (
                    super::types::ContractWhaleStructureInterpretation::DownsideAbsorption,
                    "buy"
                ) | (
                    super::types::ContractWhaleStructureInterpretation::UpsideSuppression,
                    "sell"
                )
            )
    }
}

#[derive(Clone)]
pub struct PassiveExecutionService {
    bus: MarketDataBus,
    engine: Arc<RwLock<PassiveExecutionTracker>>,
    task: Arc<RwLock<Option<tokio::task::JoinHandle<()>>>>,
}

impl PassiveExecutionService {
    pub fn new(bus: MarketDataBus) -> Self {
        Self {
            bus,
            engine: Arc::new(RwLock::new(PassiveExecutionTracker::default())),
            task: Arc::new(RwLock::new(None)),
        }
    }
    pub fn start(&self) {
        let mut task = self.task.write();
        if task.is_some() {
            return;
        }
        self.engine.write().reset();
        let mut rx = self.bus.subscribe();
        let engine = self.engine.clone();
        *task = Some(tokio::spawn(async move {
            loop {
                match rx.recv().await {
                    Ok(MarketDataEvent::Book(book)) => {
                        let now = crate::normalizers::trade::now_ms();
                        if book.ts <= now + 1000 && book.ts >= now - 5000 {
                            engine.write().on_book(&book);
                        }
                    }
                    Ok(MarketDataEvent::Trade(trade)) => {
                        let now = crate::normalizers::trade::now_ms();
                        if trade.ts <= now + 1000 && trade.ts >= now - 5000 {
                            engine.write().on_trade(&trade);
                        }
                    }
                    Ok(MarketDataEvent::VenueHealth(health)) => {
                        if health.venue == Venue::Binance
                            && !matches!(health.status, VenueConnectionStatus::Connected)
                        {
                            engine.write().reset();
                        }
                    }
                    Err(tokio::sync::broadcast::error::RecvError::Lagged(_)) => {
                        engine.write().reset()
                    }
                    Err(tokio::sync::broadcast::error::RecvError::Closed) => break,
                }
            }
        }));
    }
    pub fn stop(&self) {
        if let Some(task) = self.task.write().take() {
            task.abort();
        }
        self.engine.write().reset();
    }
    pub fn evidence(&self, symbol: &str, at: i64, window_sec: u64) -> PassiveExecutionEvidence {
        self.engine.read().evidence(symbol, at, window_sec)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::types::market::{AggressorSide, Venue};

    fn book(ts: i64, quantity: f64) -> NormalizedBook {
        NormalizedBook {
            venue: Venue::Binance,
            symbol: "BTC-PERP".into(),
            ts,
            best_bid: 60_000.0,
            best_ask: 60_001.0,
            mid: 60_000.5,
            spread_bps: 0.17,
            bids: vec![(60_000.0, quantity)],
            asks: vec![(60_001.0, quantity)],
            bid_depth_btc_10bps: quantity,
            ask_depth_btc_10bps: quantity,
            bid_depth_usd_10bps: quantity * 60_000.0,
            ask_depth_usd_10bps: quantity * 60_001.0,
            imbalance_10bps: 0.0,
        }
    }

    fn trade(ts: i64, side: AggressorSide) -> NormalizedTrade {
        let price = if side == AggressorSide::Sell {
            60_000.0
        } else {
            60_001.0
        };
        NormalizedTrade {
            venue: Venue::Binance,
            symbol: "BTC-PERP".into(),
            ts,
            price,
            size_btc: 1.0,
            size_usd: price,
            aggressor_side: side,
            trade_id: Some(format!("{ts}:{side:?}")),
        }
    }

    fn feed(side: AggressorSide, duplicate: bool, balanced: bool) -> PassiveExecutionTracker {
        let mut engine = PassiveExecutionTracker::default();
        engine.on_book(&book(1_000_000, 10.0));
        for tick in 1..=600 {
            let at = 1_000_000 + tick * 100;
            let fill = trade(at - 10, side);
            engine.on_trade(&fill);
            if duplicate {
                engine.on_trade(&fill);
            }
            if balanced {
                engine.on_trade(&trade(
                    at - 10,
                    if side == AggressorSide::Sell {
                        AggressorSide::Buy
                    } else {
                        AggressorSide::Sell
                    },
                ));
            }
            engine.on_book(&book(at, 10.0));
        }
        engine
    }

    #[test]
    fn main_force_passive_replenishment_requires_matched_directional_executions() {
        for (side, passive) in [(AggressorSide::Sell, "buy"), (AggressorSide::Buy, "sell")] {
            let engine = feed(side, false, false);
            let value = engine.evidence("BTC", 1_060_000, 60);
            assert_eq!(value.status, "supported");
            assert_eq!(value.side, passive);
            assert!(value.independent_bins >= 3 && value.coverage >= 0.9);
            assert_eq!(
                value.matched_notional_usd,
                600.0 * if passive == "buy" { 60_000.0 } else { 60_001.0 }
            );
        }
        assert_ne!(
            feed(AggressorSide::Sell, false, true)
                .evidence("BTC", 1_060_000, 60)
                .status,
            "supported"
        );
    }

    #[test]
    fn main_force_passive_duplicates_future_and_reset_cannot_invent_evidence() {
        let mut engine = feed(AggressorSide::Sell, true, false);
        let plain = feed(AggressorSide::Sell, false, false);
        assert_eq!(
            engine.evidence("BTC", 1_060_000, 60).matched_notional_usd,
            plain.evidence("BTC", 1_060_000, 60).matched_notional_usd
        );
        assert_ne!(engine.evidence("BTC", 999_000, 60).status, "supported");
        assert_ne!(engine.evidence("ETH", 1_060_000, 60).status, "supported");
        assert_ne!(engine.evidence("BTC", 1_060_000, 900).status, "supported");
        engine.reset();
        assert_ne!(engine.evidence("BTC", 1_060_000, 60).status, "supported");
    }

    #[test]
    fn main_force_invalid_book_invalidates_prior_support_immediately() {
        let mut engine = feed(AggressorSide::Sell, false, false);
        let mut invalid = book(1_060_010, 10.0);
        invalid.bids.clear();
        engine.on_book(&invalid);
        assert_ne!(engine.evidence("BTC", 1_060_010, 60).status, "supported");
    }

    #[tokio::test]
    async fn main_force_passive_service_disconnect_and_lag_clear_evidence() {
        let bus = MarketDataBus::new(1);
        let service = PassiveExecutionService::new(bus.clone());
        service.start();
        *service.engine.write() = feed(AggressorSide::Sell, false, false);
        bus.publish(MarketDataEvent::VenueHealth(
            crate::types::market::VenueHealth::disconnected(Venue::Binance, true),
        ));
        tokio::task::yield_now().await;
        assert!(service.engine.read().symbols.is_empty());
        *service.engine.write() = feed(AggressorSide::Sell, false, false);
        // Two records before the receiver runs exceed the capacity: loss must reset.
        bus.publish(MarketDataEvent::Trade(trade(1, AggressorSide::Sell)));
        bus.publish(MarketDataEvent::Trade(trade(2, AggressorSide::Sell)));
        tokio::task::yield_now().await;
        assert!(service.engine.read().symbols.is_empty());
        service.stop();
        assert!(service.task.read().is_none());
    }

    #[test]
    fn main_force_passive_cancel_missing_level_and_feed_gaps_are_not_refills() {
        for case in 0..3 {
            let mut engine = PassiveExecutionTracker::default();
            engine.on_book(&book(1_000_000, 10.0));
            for tick in 1..=600 {
                let at = 1_000_000 + tick * 100;
                if case != 0 {
                    engine.on_trade(&trade(at - 10, AggressorSide::Sell));
                }
                let mut next = book(at, if tick % 2 == 0 { 10.0 } else { 5.0 });
                if case == 1 {
                    next.bids.clear();
                }
                if case != 2 || tick % 30 == 0 {
                    engine.on_book(&next);
                }
            }
            assert_ne!(
                engine.evidence("BTC", 1_060_000, 60).status,
                "supported",
                "case={case}"
            );
        }
    }
}
