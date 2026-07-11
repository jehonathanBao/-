//! Per-symbol lifecycle registry for public Binance USD-M L2 sessions.
//!
//! The registry owns no trading capability. It is intentionally separate from
//! the older flow reconstruction so each watched symbol can be started,
//! stopped, and diagnosed without mutating the broad alt-contract monitor.

use std::{
    collections::{BTreeMap, VecDeque},
    sync::Arc,
};

use parking_lot::RwLock;
use serde::{Deserialize, Serialize};

use super::{
    intent::{IntentAssessment, IntentFsm},
    l2::{DepthDiff, DepthSnapshot, LocalOrderBook, OrderBookMetrics, OrderBookReadiness},
    walls::{WallEvidence, WallTracker},
};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum L2SessionStatus {
    Disabled,
    Connecting,
    Syncing,
    Ready,
    Gap,
    Stale,
    Error,
    Stopped,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct L2TradeFlowMetrics {
    pub buy_notional_1s: f64,
    pub sell_notional_1s: f64,
    pub buy_notional_5s: f64,
    pub sell_notional_5s: f64,
    pub buy_notional_15s: f64,
    pub sell_notional_15s: f64,
    pub buy_notional_60s: f64,
    pub sell_notional_60s: f64,
    pub last_trade_at_ms: Option<i64>,
    pub mark_price: Option<f64>,
    pub last_mark_price_at_ms: Option<i64>,
    pub reason: String,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct L2OpenInterestMetrics {
    pub current_contracts: Option<f64>,
    pub delta_15s_pct: Option<f64>,
    pub last_update_at_ms: Option<i64>,
    pub available: bool,
    pub reason: String,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct L2SessionSnapshot {
    pub symbol: String,
    pub status: L2SessionStatus,
    pub listing_phase: String,
    pub activated_at_ms: i64,
    pub book_update_samples: u64,
    pub trade_samples: u64,
    pub data_age_ms: Option<u64>,
    pub evidence_mode: String,
    pub orderbook_evidence_available: bool,
    pub intent_assessment_available: bool,
    pub orderbook: OrderBookMetrics,
    pub intent: IntentAssessment,
    pub wall_evidence: Vec<WallEvidence>,
    pub trade_flow: L2TradeFlowMetrics,
    pub open_interest: L2OpenInterestMetrics,
    pub reconnect_count: u64,
    pub book_ticker_crosscheck_ok: Option<bool>,
    pub book_ticker_mismatch_count: u32,
    pub last_book_ticker_at_ms: Option<i64>,
    pub last_error: Option<String>,
    pub read_only: bool,
}

#[derive(Debug)]
struct L2Session {
    symbol: String,
    status: L2SessionStatus,
    activated_at_ms: i64,
    book: LocalOrderBook,
    wall_tracker: WallTracker,
    intent_fsm: IntentFsm,
    intent: IntentAssessment,
    reconnect_count: u64,
    book_ticker_crosscheck_ok: Option<bool>,
    book_ticker_mismatch_count: u32,
    last_book_ticker_at_ms: Option<i64>,
    last_error: Option<String>,
    trades: VecDeque<AggTradeObservation>,
    mark_price: Option<(f64, i64)>,
    open_interest: Option<(f64, i64)>,
    previous_open_interest: Option<f64>,
    book_update_samples: u64,
    trade_samples: u64,
}

#[derive(Debug, Clone, Copy)]
struct AggTradeObservation {
    ts_ms: i64,
    notional: f64,
    aggressive_buy: bool,
}

impl L2Session {
    fn snapshot(&self) -> L2SessionSnapshot {
        let metrics = self.book.metrics(20);
        let intent = self.intent.clone();
        let orderbook_evidence_available = metrics.orderbook_evidence_available;
        let intent_assessment_available = intent.intent_assessment_available;
        let now_ms = crate::normalizers::trade::now_ms();
        let age_ms = metrics
            .last_event_time_ms
            .and_then(|last| (last > 0).then_some(now_ms.saturating_sub(last).max(0) as u64));
        L2SessionSnapshot {
            symbol: self.symbol.clone(),
            status: self.status,
            listing_phase: listing_phase(self.activated_at_ms, now_ms),
            activated_at_ms: self.activated_at_ms,
            book_update_samples: self.book_update_samples,
            trade_samples: self.trade_samples,
            data_age_ms: age_ms,
            evidence_mode: if orderbook_evidence_available {
                "l2_ready".to_string()
            } else {
                "flow_only".to_string()
            },
            orderbook_evidence_available,
            intent_assessment_available,
            orderbook: metrics,
            intent,
            wall_evidence: self.wall_tracker.evidence(),
            trade_flow: self.trade_flow_metrics(),
            open_interest: self.open_interest_metrics(),
            reconnect_count: self.reconnect_count,
            book_ticker_crosscheck_ok: self.book_ticker_crosscheck_ok,
            book_ticker_mismatch_count: self.book_ticker_mismatch_count,
            last_book_ticker_at_ms: self.last_book_ticker_at_ms,
            last_error: self.last_error.clone(),
            read_only: true,
        }
    }

    fn open_interest_metrics(&self) -> L2OpenInterestMetrics {
        let Some((current, updated_at)) = self.open_interest else {
            return L2OpenInterestMetrics {
                reason: "open_interest_not_observed".to_string(),
                ..Default::default()
            };
        };
        let delta_15s_pct = self.previous_open_interest.and_then(|previous| {
            (previous.abs() > f64::EPSILON).then_some((current - previous) / previous.abs() * 100.0)
        });
        L2OpenInterestMetrics {
            current_contracts: Some(current),
            delta_15s_pct,
            last_update_at_ms: Some(updated_at),
            available: true,
            reason: "binance_open_interest".to_string(),
        }
    }
}

impl L2Session {
    fn trade_flow_metrics(&self) -> L2TradeFlowMetrics {
        let newest = self.trades.back().map(|trade| trade.ts_ms);
        let sum_window = |window_ms: i64| {
            let lower = newest.unwrap_or_default().saturating_sub(window_ms);
            self.trades
                .iter()
                .filter(|trade| trade.ts_ms >= lower)
                .fold((0.0, 0.0), |(buy, sell), trade| {
                    if trade.aggressive_buy {
                        (buy + trade.notional, sell)
                    } else {
                        (buy, sell + trade.notional)
                    }
                })
        };
        let (buy_1s, sell_1s) = sum_window(1_000);
        let (buy_5s, sell_5s) = sum_window(5_000);
        let (buy_15s, sell_15s) = sum_window(15_000);
        let (buy_60s, sell_60s) = sum_window(60_000);
        L2TradeFlowMetrics {
            buy_notional_1s: buy_1s,
            sell_notional_1s: sell_1s,
            buy_notional_5s: buy_5s,
            sell_notional_5s: sell_5s,
            buy_notional_15s: buy_15s,
            sell_notional_15s: sell_15s,
            buy_notional_60s: buy_60s,
            sell_notional_60s: sell_60s,
            last_trade_at_ms: newest,
            mark_price: self.mark_price.map(|(price, _)| price),
            last_mark_price_at_ms: self.mark_price.map(|(_, ts)| ts),
            reason: if newest.is_some() {
                "binance_agg_trade".to_string()
            } else {
                "trade_flow_not_observed".to_string()
            },
        }
    }
}

#[derive(Debug, Clone, Default)]
pub struct L2SessionRegistry {
    sessions: Arc<RwLock<BTreeMap<String, L2Session>>>,
}

impl L2SessionRegistry {
    pub fn register(&self, raw_symbol: &str) -> L2SessionSnapshot {
        self.register_at(raw_symbol, crate::normalizers::trade::now_ms())
    }

    pub fn register_at(&self, raw_symbol: &str, activated_at_ms: i64) -> L2SessionSnapshot {
        let symbol = canonical_symbol(raw_symbol);
        let mut sessions = self.sessions.write();
        let session = sessions.entry(symbol.clone()).or_insert_with(|| L2Session {
            symbol,
            status: L2SessionStatus::Connecting,
            activated_at_ms,
            book: LocalOrderBook::default(),
            wall_tracker: WallTracker::default(),
            intent_fsm: IntentFsm::default(),
            intent: IntentAssessment {
                state: super::intent::IntentState::Unavailable,
                confidence: 0.0,
                intent_assessment_available: false,
                reason: "l2_session_not_ready".to_string(),
                evidence: vec!["l2_evidence_unavailable".to_string()],
                read_only: true,
            },
            reconnect_count: 0,
            book_ticker_crosscheck_ok: None,
            book_ticker_mismatch_count: 0,
            last_book_ticker_at_ms: None,
            last_error: None,
            trades: VecDeque::new(),
            mark_price: None,
            open_interest: None,
            previous_open_interest: None,
            book_update_samples: 0,
            trade_samples: 0,
        });
        session.snapshot()
    }

    pub fn remove(&self, raw_symbol: &str) -> bool {
        self.sessions
            .write()
            .remove(&canonical_symbol(raw_symbol))
            .is_some()
    }

    pub fn session(&self, raw_symbol: &str) -> Option<L2SessionSnapshot> {
        self.sessions
            .read()
            .get(&canonical_symbol(raw_symbol))
            .map(L2Session::snapshot)
    }

    pub fn sessions(&self) -> Vec<L2SessionSnapshot> {
        self.sessions
            .read()
            .values()
            .map(L2Session::snapshot)
            .collect()
    }

    pub fn active_count(&self) -> usize {
        self.sessions.read().len()
    }

    pub fn set_syncing(&self, raw_symbol: &str) {
        self.update(raw_symbol, |session| {
            session.status = L2SessionStatus::Syncing;
            session.last_error = None;
        });
    }

    pub fn mark_disabled(&self, raw_symbol: &str) {
        self.update(raw_symbol, |session| {
            session.status = L2SessionStatus::Disabled;
            session.last_error = Some("new_token_l2_disabled".to_string());
        });
    }

    pub fn install_snapshot(&self, raw_symbol: &str, snapshot: DepthSnapshot) {
        self.update(raw_symbol, |session| {
            session.book.install_snapshot(snapshot);
            let observed_at = session
                .book
                .metrics(1)
                .last_event_time_ms
                .unwrap_or_default();
            session.wall_tracker.observe(&session.book, observed_at);
            session.intent = session.intent_fsm.observe(&session.book.metrics(20));
            session.status = status_from_readiness(session.book.readiness());
            session.last_error = None;
        });
    }

    pub fn registry_buffer_diff(&self, raw_symbol: &str, diff: DepthDiff) {
        self.update(raw_symbol, |session| {
            session.book.buffer_diff(diff);
            session.status = L2SessionStatus::Syncing;
        });
    }

    pub fn apply_diff(&self, raw_symbol: &str, diff: DepthDiff) -> Result<(), String> {
        let symbol = canonical_symbol(raw_symbol);
        let mut sessions = self.sessions.write();
        let Some(session) = sessions.get_mut(&symbol) else {
            return Err("session_not_found".to_string());
        };
        match session.book.apply_diff(diff) {
            Ok(()) => {
                session.book_update_samples = session.book_update_samples.saturating_add(1);
                let observed_at = session
                    .book
                    .metrics(1)
                    .last_event_time_ms
                    .unwrap_or_default();
                session.wall_tracker.observe(&session.book, observed_at);
                session.intent = session.intent_fsm.observe(&session.book.metrics(20));
                session.status = status_from_readiness(session.book.readiness());
                Ok(())
            }
            Err(error) => {
                session.status = status_from_readiness(session.book.readiness());
                session.intent = session.intent_fsm.observe(&session.book.metrics(20));
                session.last_error = Some(error.as_str().to_string());
                Err(error.as_str().to_string())
            }
        }
    }

    pub fn mark_reconnecting(&self, raw_symbol: &str, reason: Option<String>) {
        self.update(raw_symbol, |session| {
            session.status = L2SessionStatus::Connecting;
            session.reconnect_count = session.reconnect_count.saturating_add(1);
            session.last_error = reason;
        });
    }

    pub fn mark_stale(&self, raw_symbol: &str) {
        self.update(raw_symbol, |session| {
            session.book.mark_stale();
            session.status = L2SessionStatus::Stale;
            session.intent = session.intent_fsm.observe(&session.book.metrics(20));
        });
    }

    pub fn record_agg_trade(
        &self,
        raw_symbol: &str,
        price: f64,
        quantity: f64,
        buyer_is_maker: bool,
        event_time_ms: i64,
    ) {
        self.update(raw_symbol, |session| {
            let notional = price * quantity;
            if !notional.is_finite() || notional <= 0.0 {
                return;
            }
            session.trades.push_back(AggTradeObservation {
                ts_ms: event_time_ms,
                notional,
                // In Binance aggTrade, buyerIsMaker means the buyer was
                // passive, so the taker initiated a sell.
                aggressive_buy: !buyer_is_maker,
            });
            session.trade_samples = session.trade_samples.saturating_add(1);
            let cutoff = event_time_ms.saturating_sub(60_000);
            while session
                .trades
                .front()
                .is_some_and(|trade| trade.ts_ms < cutoff)
            {
                session.trades.pop_front();
            }
        });
    }

    pub fn record_mark_price(&self, raw_symbol: &str, price: f64, event_time_ms: i64) {
        self.update(raw_symbol, |session| {
            if price.is_finite() && price > 0.0 {
                session.mark_price = Some((price, event_time_ms));
            }
        });
    }

    pub fn record_open_interest(&self, raw_symbol: &str, value: f64, event_time_ms: i64) {
        self.update(raw_symbol, |session| {
            if value.is_finite() && value >= 0.0 {
                session.previous_open_interest = session.open_interest.map(|(current, _)| current);
                session.open_interest = Some((value, event_time_ms));
            }
        });
    }

    /// Returns true when repeated cross-check mismatches invalidate the local
    /// book and the owning runtime must reconnect and resynchronise.
    pub fn record_book_ticker(
        &self,
        raw_symbol: &str,
        bid: f64,
        ask: f64,
        event_time_ms: i64,
    ) -> bool {
        let mut requires_resync = false;
        self.update(raw_symbol, |session| {
            let metrics = session.book.metrics(1);
            session.book_ticker_crosscheck_ok = match (metrics.best_bid, metrics.best_ask) {
                (Some(local_bid), Some(local_ask)) if bid > 0.0 && ask >= bid => {
                    let tolerance = (local_ask - local_bid).abs().max(local_ask * 0.001);
                    Some(
                        (local_bid - bid).abs() <= tolerance
                            && (local_ask - ask).abs() <= tolerance,
                    )
                }
                _ => None,
            };
            match session.book_ticker_crosscheck_ok {
                Some(true) => session.book_ticker_mismatch_count = 0,
                Some(false) => {
                    session.book_ticker_mismatch_count =
                        session.book_ticker_mismatch_count.saturating_add(1);
                    if session.book_ticker_mismatch_count >= 2 {
                        session.book.invalidate_for_resync();
                        session.status = L2SessionStatus::Gap;
                        session.intent = IntentAssessment {
                            state: super::intent::IntentState::Unavailable,
                            confidence: 0.0,
                            intent_assessment_available: false,
                            reason: "book_ticker_mismatch_resync_required".to_string(),
                            evidence: vec!["local_book_crosscheck_failed".to_string()],
                            read_only: true,
                        };
                        session.last_error = Some("book_ticker_mismatch".to_string());
                        requires_resync = true;
                    }
                }
                None => {}
            }
            session.last_book_ticker_at_ms = Some(event_time_ms);
        });
        requires_resync
    }

    pub fn mark_error(&self, raw_symbol: &str, reason: impl Into<String>) {
        self.update(raw_symbol, |session| {
            session.status = L2SessionStatus::Error;
            session.last_error = Some(reason.into());
        });
    }

    fn update(&self, raw_symbol: &str, update: impl FnOnce(&mut L2Session)) {
        if let Some(session) = self.sessions.write().get_mut(&canonical_symbol(raw_symbol)) {
            update(session);
        }
    }
}

fn canonical_symbol(raw_symbol: &str) -> String {
    raw_symbol.trim().to_ascii_uppercase()
}

fn listing_phase(activated_at_ms: i64, now_ms: i64) -> String {
    let age_ms = now_ms.saturating_sub(activated_at_ms.max(0));
    if age_ms < 60_000 {
        "syncing".to_string()
    } else if age_ms < 10 * 60_000 {
        "opening".to_string()
    } else if age_ms < 60 * 60_000 {
        "early".to_string()
    } else if age_ms < 24 * 60 * 60_000 {
        "stabilizing".to_string()
    } else {
        "mature".to_string()
    }
}

fn status_from_readiness(readiness: OrderBookReadiness) -> L2SessionStatus {
    match readiness {
        OrderBookReadiness::Ready => L2SessionStatus::Ready,
        OrderBookReadiness::Gap => L2SessionStatus::Gap,
        OrderBookReadiness::Stale => L2SessionStatus::Stale,
        OrderBookReadiness::Syncing | OrderBookReadiness::Unavailable => L2SessionStatus::Syncing,
    }
}
