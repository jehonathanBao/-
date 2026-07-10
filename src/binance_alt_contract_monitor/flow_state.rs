use std::collections::{BTreeMap, VecDeque};

use super::types::{AltContractTrade, AltContractTradeSide};

#[derive(Debug, Clone, Default)]
pub struct AltFlowBucket1s {
    pub second_ts: i64,
    pub buy_notional_usd: f64,
    pub sell_notional_usd: f64,
    pub trade_count: u64,
}

#[derive(Debug, Clone, Default)]
pub struct PerSymbolFlowWindow {
    pub total_notional_usd: f64,
    pub buy_notional_usd: f64,
    pub sell_notional_usd: f64,
    pub trade_count: u64,
}

#[derive(Debug, Clone, Default)]
pub struct PerSymbolFlowState {
    pub product_id: String,
    pub buckets_1s: VecDeque<AltFlowBucket1s>,
}

#[derive(Debug, Clone)]
pub struct PerSymbolFlowBook {
    retention_seconds: i64,
    states: BTreeMap<String, PerSymbolFlowState>,
}

impl PerSymbolFlowBook {
    pub fn new(retention_seconds: u64) -> Self {
        Self {
            retention_seconds: i64::try_from(retention_seconds).unwrap_or(i64::MAX),
            states: BTreeMap::new(),
        }
    }

    pub fn ingest(&mut self, trade: AltContractTrade) {
        let second_ts = trade.ts.div_euclid(1_000) * 1_000;
        let state = self
            .states
            .entry(trade.product_id.clone())
            .or_insert_with(|| PerSymbolFlowState {
                product_id: trade.product_id.clone(),
                ..PerSymbolFlowState::default()
            });
        if state
            .buckets_1s
            .back()
            .is_none_or(|bucket| bucket.second_ts != second_ts)
        {
            state.buckets_1s.push_back(AltFlowBucket1s {
                second_ts,
                ..AltFlowBucket1s::default()
            });
        }
        let bucket = state.buckets_1s.back_mut().expect("bucket inserted");
        match trade.side {
            AltContractTradeSide::Buy => bucket.buy_notional_usd += trade.notional_usd,
            AltContractTradeSide::Sell => bucket.sell_notional_usd += trade.notional_usd,
        }
        bucket.trade_count = bucket.trade_count.saturating_add(1);
        let oldest_allowed = second_ts.saturating_sub(self.retention_seconds.saturating_mul(1_000));
        while state
            .buckets_1s
            .front()
            .is_some_and(|bucket| bucket.second_ts < oldest_allowed)
        {
            state.buckets_1s.pop_front();
        }
    }

    pub fn window(
        &self,
        product_id: &str,
        window_seconds: u64,
        now_ms: i64,
    ) -> Option<PerSymbolFlowWindow> {
        let state = self.states.get(product_id)?;
        let start = now_ms.saturating_sub(
            i64::try_from(window_seconds)
                .unwrap_or(i64::MAX)
                .saturating_mul(1_000),
        );
        let mut result = PerSymbolFlowWindow::default();
        for bucket in state
            .buckets_1s
            .iter()
            .filter(|bucket| bucket.second_ts >= start)
        {
            result.buy_notional_usd += bucket.buy_notional_usd;
            result.sell_notional_usd += bucket.sell_notional_usd;
            result.trade_count = result.trade_count.saturating_add(bucket.trade_count);
        }
        result.total_notional_usd = result.buy_notional_usd + result.sell_notional_usd;
        Some(result)
    }

    pub fn symbol_count(&self) -> usize {
        self.states.len()
    }

    pub fn has_symbol(&self, product_id: &str) -> bool {
        self.states.contains_key(product_id)
    }
}
