use std::collections::{BTreeMap, VecDeque};

use crate::{
    config::thresholds::VpinParams,
    types::{
        market::{AggressorSide, NormalizedTrade},
        vpin::{VpinBucket, VpinDirection, VpinMetrics, VpinState, VpinVenueBreakdown},
    },
};

const EPSILON: f64 = 1e-9;

#[derive(Debug, Clone)]
struct ActiveBucket {
    id: u64,
    start_ts: i64,
    end_ts: i64,
    total_btc: f64,
    buy_btc: f64,
    sell_btc: f64,
    venue_breakdown: BTreeMap<String, VpinVenueBreakdown>,
}

impl ActiveBucket {
    fn new(id: u64, ts: i64) -> Self {
        Self {
            id,
            start_ts: ts,
            end_ts: ts,
            total_btc: 0.0,
            buy_btc: 0.0,
            sell_btc: 0.0,
            venue_breakdown: BTreeMap::new(),
        }
    }
}

#[derive(Debug, Clone)]
pub struct VpinBucketEngine {
    params: VpinParams,
    active_bucket: Option<ActiveBucket>,
    completed_buckets: VecDeque<VpinBucket>,
    next_bucket_id: u64,
}

impl VpinBucketEngine {
    pub fn new(params: VpinParams) -> Self {
        Self {
            params,
            active_bucket: None,
            completed_buckets: VecDeque::new(),
            next_bucket_id: 1,
        }
    }

    pub fn params(&self) -> &VpinParams {
        &self.params
    }

    pub fn on_trade(&mut self, trade: &NormalizedTrade) -> Vec<VpinBucket> {
        if !self.params.enabled || trade.size_btc <= 0.0 || trade.price <= 0.0 {
            return Vec::new();
        }

        let mut remaining = trade.size_btc;
        let mut completed = Vec::new();

        while remaining > EPSILON {
            if self.active_bucket.is_none() {
                let bucket = self.new_active_bucket(trade.ts);
                self.active_bucket = Some(bucket);
            }
            let available = self
                .active_bucket
                .as_ref()
                .map(|active| (self.params.bucket_size_btc - active.total_btc).max(0.0))
                .unwrap_or(self.params.bucket_size_btc);
            let fill = remaining.min(available.max(EPSILON));

            if let Some(active) = &mut self.active_bucket {
                active.end_ts = trade.ts;
                active.total_btc += fill;
                match trade.aggressor_side {
                    AggressorSide::Buy => active.buy_btc += fill,
                    AggressorSide::Sell => active.sell_btc += fill,
                }
                let entry = active
                    .venue_breakdown
                    .entry(trade.venue.as_key().to_string())
                    .or_default();
                match trade.aggressor_side {
                    AggressorSide::Buy => entry.buy_btc += fill,
                    AggressorSide::Sell => entry.sell_btc += fill,
                }
                entry.total_btc += fill;
                entry.net_btc = entry.buy_btc - entry.sell_btc;
            }

            remaining -= fill;

            let should_finalize = self
                .active_bucket
                .as_ref()
                .is_some_and(|bucket| bucket.total_btc + EPSILON >= self.params.bucket_size_btc);

            if should_finalize {
                if let Some(bucket) = self.finalize_active_bucket() {
                    completed.push(bucket);
                }
            }
        }

        completed
    }

    pub fn get_state(&self, now_ts: i64) -> VpinState {
        let completed = self.completed_buckets.iter().cloned().collect::<Vec<_>>();
        let lookback = completed
            .iter()
            .rev()
            .take(self.params.lookback_buckets)
            .cloned()
            .collect::<Vec<_>>();
        let lookback = lookback.into_iter().rev().collect::<Vec<_>>();

        let latest_bucket = completed.last().cloned();
        let active_progress_btc = self
            .active_bucket
            .as_ref()
            .map(|bucket| bucket.total_btc)
            .unwrap_or(0.0);
        let active_progress_ratio = if self.params.bucket_size_btc > 0.0 {
            active_progress_btc / self.params.bucket_size_btc
        } else {
            0.0
        };

        let mut reason_codes = Vec::new();
        let vpin = if lookback.len() >= self.params.min_buckets {
            Some(mean(
                &lookback
                    .iter()
                    .map(|bucket| bucket.imbalance_ratio)
                    .collect::<Vec<_>>(),
            ))
        } else {
            reason_codes.push("insufficient_buckets".to_string());
            None
        };

        let latest_bucket_imbalance_ratio =
            latest_bucket.as_ref().map(|bucket| bucket.imbalance_ratio);
        let avg_bucket_imbalance_ratio = vpin;
        let vpin_zscore = if lookback.len() >= self.params.min_buckets {
            zscore(
                latest_bucket_imbalance_ratio,
                &lookback
                    .iter()
                    .map(|bucket| bucket.imbalance_ratio)
                    .collect::<Vec<_>>(),
            )
        } else {
            None
        };
        let vpin_percentile = if lookback.len() >= self.params.min_buckets {
            percentile(
                latest_bucket_imbalance_ratio,
                &lookback
                    .iter()
                    .map(|bucket| bucket.imbalance_ratio)
                    .collect::<Vec<_>>(),
            )
        } else {
            None
        };

        let vpin_extreme = vpin.is_some_and(|value| value >= self.params.extreme_threshold);
        let vpin_spike = vpin_zscore.is_some_and(|value| value >= self.params.spike_zscore);
        let vpin_high =
            !vpin_extreme && vpin.is_some_and(|value| value >= self.params.high_threshold);

        if vpin_extreme {
            reason_codes.push("vpin_extreme".to_string());
        } else if vpin_spike {
            reason_codes.push("vpin_spike".to_string());
        } else if vpin_high {
            reason_codes.push("vpin_high".to_string());
        }

        let dominant_direction = dominant_direction(&lookback);
        VpinState {
            symbol: "BTC-PERP".to_string(),
            updated_at: now_ts,
            metrics: VpinMetrics {
                symbol: "BTC-PERP".to_string(),
                updated_at: now_ts,
                enabled: self.params.enabled,
                bucket_size_btc: self.params.bucket_size_btc,
                lookback_buckets: self.params.lookback_buckets,
                min_buckets: self.params.min_buckets,
                completed_bucket_count: completed.len(),
                active_bucket_progress_btc: active_progress_btc,
                active_bucket_progress_ratio: active_progress_ratio,
                latest_bucket,
                vpin,
                vpin_zscore,
                vpin_percentile,
                latest_bucket_imbalance_ratio,
                avg_bucket_imbalance_ratio,
                vpin_high,
                vpin_extreme,
                vpin_spike,
                dominant_direction,
                reason_codes,
            },
            recent_buckets: completed,
        }
    }

    pub fn recent_buckets(&self, limit: usize) -> Vec<VpinBucket> {
        let limit = limit.min(self.completed_buckets.len());
        self.completed_buckets
            .iter()
            .rev()
            .take(limit)
            .cloned()
            .collect()
    }

    pub fn clear(&mut self) {
        self.active_bucket = None;
        self.completed_buckets.clear();
        self.next_bucket_id = 1;
    }

    fn new_active_bucket(&mut self, ts: i64) -> ActiveBucket {
        let bucket = ActiveBucket::new(self.next_bucket_id, ts);
        self.next_bucket_id += 1;
        bucket
    }

    fn finalize_active_bucket(&mut self) -> Option<VpinBucket> {
        let active = self.active_bucket.take()?;
        let total_btc = active.total_btc.max(EPSILON);
        let net_btc = active.buy_btc - active.sell_btc;
        let imbalance_btc = net_btc.abs();
        let bucket = VpinBucket {
            id: active.id,
            symbol: "BTC-PERP".to_string(),
            start_ts: active.start_ts,
            end_ts: active.end_ts,
            bucket_size_btc: self.params.bucket_size_btc,
            total_btc: active.total_btc,
            buy_btc: active.buy_btc,
            sell_btc: active.sell_btc,
            net_btc,
            imbalance_btc,
            imbalance_ratio: imbalance_btc / total_btc,
            direction: direction_for(active.buy_btc, active.sell_btc),
            venue_breakdown: active.venue_breakdown,
        };
        self.completed_buckets.push_back(bucket.clone());
        while self.completed_buckets.len() > self.params.max_recent_buckets {
            self.completed_buckets.pop_front();
        }
        Some(bucket)
    }
}

fn direction_for(buy_btc: f64, sell_btc: f64) -> VpinDirection {
    if buy_btc > sell_btc {
        VpinDirection::Buy
    } else if sell_btc > buy_btc {
        VpinDirection::Sell
    } else {
        VpinDirection::Balanced
    }
}

fn dominant_direction(buckets: &[VpinBucket]) -> VpinDirection {
    let net = buckets.iter().map(|bucket| bucket.net_btc).sum::<f64>();
    if net > 0.0 {
        VpinDirection::Buy
    } else if net < 0.0 {
        VpinDirection::Sell
    } else {
        VpinDirection::Balanced
    }
}

fn mean(values: &[f64]) -> f64 {
    if values.is_empty() {
        0.0
    } else {
        values.iter().sum::<f64>() / values.len() as f64
    }
}

fn stddev(values: &[f64], mean: f64) -> f64 {
    if values.is_empty() {
        return 0.0;
    }
    let variance = values
        .iter()
        .map(|value| {
            let diff = value - mean;
            diff * diff
        })
        .sum::<f64>()
        / values.len() as f64;
    variance.sqrt()
}

fn zscore(latest: Option<f64>, values: &[f64]) -> Option<f64> {
    let latest = latest?;
    if values.is_empty() {
        return None;
    }
    let avg = mean(values);
    let std = stddev(values, avg);
    if std <= EPSILON {
        None
    } else {
        Some((latest - avg) / std)
    }
}

fn percentile(latest: Option<f64>, values: &[f64]) -> Option<f64> {
    let latest = latest?;
    if values.is_empty() {
        return None;
    }
    let count = values.iter().filter(|value| **value <= latest).count();
    Some(count as f64 / values.len() as f64)
}
