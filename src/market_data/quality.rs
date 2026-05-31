use std::sync::{
    atomic::{AtomicI64, AtomicU64, Ordering},
    Arc,
};

use crate::normalizers::trade::now_ms;

#[derive(Debug, Clone, Default)]
pub struct MarketDataQualityTracker {
    inner: Arc<MarketDataQualityCounters>,
}

#[derive(Debug, Default)]
struct MarketDataQualityCounters {
    event_bus_dropped_events: AtomicU64,
    event_bus_send_errors: AtomicU64,
    flow_window_lagged_events: AtomicU64,
    markout_lagged_events: AtomicU64,
    vpin_lagged_events: AtomicU64,
    last_lagged_at_ms: AtomicI64,
}

#[derive(Debug, Clone, Copy, Default)]
pub struct MarketDataQualitySnapshot {
    pub event_bus_dropped_events: u64,
    pub event_bus_send_errors: u64,
    pub flow_window_lagged_events: u64,
    pub markout_lagged_events: u64,
    pub vpin_lagged_events: u64,
    pub last_lagged_at_ms: Option<i64>,
}

impl MarketDataQualityTracker {
    pub fn record_send_error(&self) {
        self.inner
            .event_bus_send_errors
            .fetch_add(1, Ordering::Relaxed);
        self.inner
            .event_bus_dropped_events
            .fetch_add(1, Ordering::Relaxed);
    }

    pub fn record_flow_window_lagged(&self, skipped: u64) {
        self.record_lagged(&self.inner.flow_window_lagged_events, skipped);
    }

    pub fn record_markout_lagged(&self, skipped: u64) {
        self.record_lagged(&self.inner.markout_lagged_events, skipped);
    }

    pub fn record_vpin_lagged(&self, skipped: u64) {
        self.record_lagged(&self.inner.vpin_lagged_events, skipped);
    }

    pub fn snapshot(&self) -> MarketDataQualitySnapshot {
        let last_lagged_at_ms = self.inner.last_lagged_at_ms.load(Ordering::Relaxed);
        MarketDataQualitySnapshot {
            event_bus_dropped_events: self.inner.event_bus_dropped_events.load(Ordering::Relaxed),
            event_bus_send_errors: self.inner.event_bus_send_errors.load(Ordering::Relaxed),
            flow_window_lagged_events: self.inner.flow_window_lagged_events.load(Ordering::Relaxed),
            markout_lagged_events: self.inner.markout_lagged_events.load(Ordering::Relaxed),
            vpin_lagged_events: self.inner.vpin_lagged_events.load(Ordering::Relaxed),
            last_lagged_at_ms: (last_lagged_at_ms > 0).then_some(last_lagged_at_ms),
        }
    }

    fn record_lagged(&self, counter: &AtomicU64, skipped: u64) {
        counter.fetch_add(skipped.max(1), Ordering::Relaxed);
        self.inner
            .last_lagged_at_ms
            .store(now_ms(), Ordering::Relaxed);
    }
}
