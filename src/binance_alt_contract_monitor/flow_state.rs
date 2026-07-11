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
pub struct PerSymbolFlowBaseline {
    pub median_notional_usd: f64,
    pub mad_notional_usd: f64,
    pub mean_notional_usd: f64,
    pub dynamic_multiple: f64,
    pub dynamic_zscore: Option<f64>,
    pub sample_count: usize,
    pub available: bool,
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
        let bucket_index = state
            .buckets_1s
            .iter()
            .position(|bucket| bucket.second_ts >= second_ts);
        let bucket_index = match bucket_index {
            Some(index) if state.buckets_1s[index].second_ts == second_ts => index,
            Some(index) => {
                state.buckets_1s.insert(
                    index,
                    AltFlowBucket1s {
                        second_ts,
                        ..AltFlowBucket1s::default()
                    },
                );
                index
            }
            None => {
                state.buckets_1s.push_back(AltFlowBucket1s {
                    second_ts,
                    ..AltFlowBucket1s::default()
                });
                state.buckets_1s.len().saturating_sub(1)
            }
        };
        let bucket = state
            .buckets_1s
            .get_mut(bucket_index)
            .expect("bucket inserted");
        match trade.side {
            AltContractTradeSide::Buy => bucket.buy_notional_usd += trade.notional_usd,
            AltContractTradeSide::Sell => bucket.sell_notional_usd += trade.notional_usd,
        }
        bucket.trade_count = bucket.trade_count.saturating_add(1);
        let newest_second_ts = state
            .buckets_1s
            .back()
            .map(|bucket| bucket.second_ts)
            .unwrap_or(second_ts);
        let oldest_allowed =
            newest_second_ts.saturating_sub(self.retention_seconds.saturating_mul(1_000));
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
            .filter(|bucket| bucket.second_ts >= start && bucket.second_ts <= now_ms)
        {
            result.buy_notional_usd += bucket.buy_notional_usd;
            result.sell_notional_usd += bucket.sell_notional_usd;
            result.trade_count = result.trade_count.saturating_add(bucket.trade_count);
        }
        result.total_notional_usd = result.buy_notional_usd + result.sell_notional_usd;
        Some(result)
    }

    pub fn baseline(
        &self,
        product_id: &str,
        window_seconds: u64,
        now_ms: i64,
        lookback_seconds: u64,
        min_samples: usize,
    ) -> Option<PerSymbolFlowBaseline> {
        let state = self.states.get(product_id)?;
        let window_ms = i64::try_from(window_seconds).ok()?.saturating_mul(1_000);
        if window_ms <= 0 {
            return None;
        }
        let lookback_ms = i64::try_from(lookback_seconds)
            .ok()?
            .saturating_mul(1_000)
            .max(window_ms);
        let current_start = now_ms.saturating_sub(window_ms);
        let history_start = current_start.saturating_sub(lookback_ms);
        let history_end = current_start;
        let first_observed = state.buckets_1s.front()?.second_ts.max(history_start);
        if first_observed >= history_end {
            return Some(PerSymbolFlowBaseline::default());
        }
        let first_sample = first_observed
            .saturating_sub(history_start)
            .div_euclid(window_ms)
            .max(0);
        let last_sample = history_end
            .saturating_sub(1)
            .saturating_sub(history_start)
            .div_euclid(window_ms)
            .max(first_sample);
        let sample_count = usize::try_from(last_sample.saturating_sub(first_sample) + 1).ok()?;
        let mut samples = vec![0.0; sample_count];
        for bucket in state
            .buckets_1s
            .iter()
            .filter(|bucket| bucket.second_ts >= history_start && bucket.second_ts < history_end)
        {
            let sample = bucket
                .second_ts
                .saturating_sub(history_start)
                .div_euclid(window_ms);
            if sample >= first_sample && sample <= last_sample {
                let index = usize::try_from(sample.saturating_sub(first_sample)).ok()?;
                samples[index] += bucket.buy_notional_usd + bucket.sell_notional_usd;
            }
        }
        let median_notional_usd = median(&samples);
        let mean_notional_usd = samples.iter().sum::<f64>() / samples.len() as f64;
        let deviations = samples
            .iter()
            .map(|sample| (sample - median_notional_usd).abs())
            .collect::<Vec<_>>();
        let mad_notional_usd = median(&deviations);
        let current_notional_usd = self
            .window(product_id, window_seconds, now_ms)
            .map(|window| window.total_notional_usd)
            .unwrap_or_default();
        let available = sample_count >= min_samples && median_notional_usd > 0.0;
        let dynamic_multiple = available
            .then_some(current_notional_usd / median_notional_usd)
            .unwrap_or_default();
        let dynamic_zscore = available
            .then_some(mad_notional_usd * 1.4826)
            .filter(|scale| *scale > 0.0)
            .map(|scale| (current_notional_usd - median_notional_usd) / scale);
        Some(PerSymbolFlowBaseline {
            median_notional_usd,
            mad_notional_usd,
            mean_notional_usd,
            dynamic_multiple,
            dynamic_zscore,
            sample_count,
            available,
        })
    }

    pub fn symbol_count(&self) -> usize {
        self.states.len()
    }

    pub fn has_symbol(&self, product_id: &str) -> bool {
        self.states.contains_key(product_id)
    }

    pub fn symbols(&self) -> impl Iterator<Item = &str> {
        self.states.keys().map(String::as_str)
    }
}

fn median(values: &[f64]) -> f64 {
    if values.is_empty() {
        return 0.0;
    }
    let mut sorted = values.to_vec();
    sorted.sort_by(|left, right| left.total_cmp(right));
    let middle = sorted.len() / 2;
    if sorted.len().is_multiple_of(2) {
        (sorted[middle - 1] + sorted[middle]) / 2.0
    } else {
        sorted[middle]
    }
}
