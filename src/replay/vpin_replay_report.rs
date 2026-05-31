use crate::types::vpin::{VpinBucket, VpinDirection, VpinMetrics};

#[derive(Debug, Clone)]
pub struct VpinReplaySummary {
    pub bucket_size_btc: f64,
    pub lookback_buckets: usize,
    pub completed_buckets: usize,
    pub max_vpin: Option<f64>,
    pub max_vpin_zscore: Option<f64>,
    pub vpin_high_count: usize,
    pub vpin_spike_count: usize,
    pub vpin_extreme_count: usize,
    pub top_buckets: Vec<VpinBucket>,
    pub dominant_direction: VpinDirection,
}

#[derive(Debug, Clone, Default)]
pub struct VpinReplayAccumulator {
    max_vpin: Option<f64>,
    max_vpin_zscore: Option<f64>,
    vpin_high_count: usize,
    vpin_spike_count: usize,
    vpin_extreme_count: usize,
    last_metrics: Option<VpinMetrics>,
}

impl VpinReplayAccumulator {
    pub fn observe(&mut self, metrics: &VpinMetrics) {
        self.max_vpin = max_optional(self.max_vpin, metrics.vpin);
        self.max_vpin_zscore = max_optional(self.max_vpin_zscore, metrics.vpin_zscore);
        if metrics.vpin_high {
            self.vpin_high_count += 1;
        }
        if metrics.vpin_spike {
            self.vpin_spike_count += 1;
        }
        if metrics.vpin_extreme {
            self.vpin_extreme_count += 1;
        }
        self.last_metrics = Some(metrics.clone());
    }

    pub fn finalize(&self, buckets: Vec<VpinBucket>) -> Option<VpinReplaySummary> {
        let metrics = self.last_metrics.clone()?;
        let mut top_buckets = buckets;
        top_buckets.sort_by(|left, right| {
            right
                .imbalance_ratio
                .total_cmp(&left.imbalance_ratio)
                .then_with(|| right.end_ts.cmp(&left.end_ts))
        });
        top_buckets.truncate(5);

        Some(VpinReplaySummary {
            bucket_size_btc: metrics.bucket_size_btc,
            lookback_buckets: metrics.lookback_buckets,
            completed_buckets: metrics.completed_bucket_count,
            max_vpin: self.max_vpin,
            max_vpin_zscore: self.max_vpin_zscore,
            vpin_high_count: self.vpin_high_count,
            vpin_spike_count: self.vpin_spike_count,
            vpin_extreme_count: self.vpin_extreme_count,
            top_buckets,
            dominant_direction: metrics.dominant_direction,
        })
    }
}

fn max_optional(current: Option<f64>, next: Option<f64>) -> Option<f64> {
    match (current, next) {
        (Some(left), Some(right)) => Some(left.max(right)),
        (None, Some(value)) | (Some(value), None) => Some(value),
        (None, None) => None,
    }
}
