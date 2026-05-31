use crate::types::{liquidation::LiquidationState, liquidation_replay::LiquidationReplayEvidence};

#[derive(Debug, Clone)]
pub struct LiquidationReplaySummary {
    pub snapshots_seen: usize,
    pub clusters_detected: usize,
    pub nearby_cluster_events: usize,
    pub possible_hunt_setups: usize,
    pub max_cluster_intensity: f64,
    pub strongest_side: String,
    pub evidence: Vec<LiquidationReplayEvidence>,
}

#[derive(Debug, Clone, Default)]
pub struct LiquidationReplayAccumulator {
    snapshots_seen: usize,
    clusters_detected: usize,
    nearby_cluster_events: usize,
    possible_hunt_setups: usize,
    max_cluster_intensity: f64,
    short_side_count: usize,
    long_side_count: usize,
    evidence: Vec<LiquidationReplayEvidence>,
}

impl LiquidationReplayAccumulator {
    pub fn observe(
        &mut self,
        state: &LiquidationState,
        snapshots_seen: usize,
        symbol: &str,
        sample_threshold: f64,
    ) {
        self.snapshots_seen = self.snapshots_seen.max(snapshots_seen);
        self.clusters_detected = self.clusters_detected.max(state.recent_clusters.len());

        if let Some(side) = state.metrics.nearest_cluster_side {
            match side {
                crate::types::liquidation::LiquidationClusterSide::ShortAbove => {
                    self.short_side_count += 1;
                }
                crate::types::liquidation::LiquidationClusterSide::LongBelow => {
                    self.long_side_count += 1;
                }
            }
        }
        if state.metrics.liq_cluster_nearby {
            self.nearby_cluster_events += 1;
        }
        if state.metrics.possible_liq_hunt_setup {
            self.possible_hunt_setups += 1;
        }
        self.max_cluster_intensity = self
            .max_cluster_intensity
            .max(state.metrics.liq_hunt_pressure);

        if state.metrics.liq_cluster_nearby
            || state.metrics.possible_liq_hunt_setup
            || state.metrics.liq_hunt_pressure >= sample_threshold
        {
            self.evidence.push(LiquidationReplayEvidence {
                ts_ms: state.updated_at,
                symbol: symbol.to_string(),
                mark_price: state.metrics.current_mid.unwrap_or_default(),
                nearest_cluster_price: nearest_cluster_price(state),
                nearest_cluster_distance_bps: state.metrics.distance_bps,
                nearest_cluster_side: state.metrics.nearest_cluster_side,
                cluster_intensity: state.metrics.liq_hunt_pressure,
                nearby_cluster: state.metrics.liq_cluster_nearby,
                possible_liq_hunt_setup: state.metrics.possible_liq_hunt_setup,
                explanation: state.metrics.reason_codes.clone(),
            });
        }
    }

    pub fn finalize(mut self) -> Option<LiquidationReplaySummary> {
        if self.snapshots_seen == 0 && self.evidence.is_empty() && self.clusters_detected == 0 {
            return None;
        }
        self.evidence
            .sort_by(|left, right| right.cluster_intensity.total_cmp(&left.cluster_intensity));
        self.evidence.truncate(20);

        Some(LiquidationReplaySummary {
            snapshots_seen: self.snapshots_seen,
            clusters_detected: self.clusters_detected,
            nearby_cluster_events: self.nearby_cluster_events,
            possible_hunt_setups: self.possible_hunt_setups,
            max_cluster_intensity: self.max_cluster_intensity,
            strongest_side: strongest_side(self.long_side_count, self.short_side_count),
            evidence: self.evidence,
        })
    }
}

fn nearest_cluster_price(state: &LiquidationState) -> Option<f64> {
    match state.metrics.nearest_cluster_side {
        Some(crate::types::liquidation::LiquidationClusterSide::ShortAbove) => state
            .metrics
            .nearest_short_liq_cluster_above
            .as_ref()
            .map(|cluster| cluster.price),
        Some(crate::types::liquidation::LiquidationClusterSide::LongBelow) => state
            .metrics
            .nearest_long_liq_cluster_below
            .as_ref()
            .map(|cluster| cluster.price),
        None => None,
    }
}

fn strongest_side(long_side_count: usize, short_side_count: usize) -> String {
    match long_side_count.cmp(&short_side_count) {
        std::cmp::Ordering::Greater => "long_liq".to_string(),
        std::cmp::Ordering::Less => "short_liq".to_string(),
        std::cmp::Ordering::Equal => {
            if long_side_count == 0 && short_side_count == 0 {
                "none".to_string()
            } else {
                "mixed".to_string()
            }
        }
    }
}
