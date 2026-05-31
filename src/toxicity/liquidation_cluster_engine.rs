use std::collections::BTreeMap;

use crate::{
    config::thresholds::LiquidationClusterParams,
    market_data::price_index::PriceSnapshot,
    types::{
        flow::FlowState,
        liquidation::{
            empty_liquidation_state, EstimatedLiquidationCluster, LiquidationClusterSide,
            LiquidationState,
        },
        sweep::{SweepDirection, SweepState},
        toxic::ToxicDirection,
        vpin::VpinState,
    },
};

#[derive(Debug, Clone)]
pub struct LiquidationClusterEngine {
    params: LiquidationClusterParams,
}

#[derive(Debug, Clone, Copy)]
struct FlowSignal {
    buy_usd: f64,
    sell_usd: f64,
    net_btc: f64,
    abs_btc: f64,
}

#[derive(Debug, Clone)]
struct ClusterAccumulator {
    side: LiquidationClusterSide,
    sum_price: f64,
    count: usize,
    first_seen_ts: i64,
    last_seen_ts: i64,
}

impl LiquidationClusterEngine {
    pub fn new(params: LiquidationClusterParams) -> Self {
        Self { params }
    }

    pub fn compute(
        &self,
        now_ts: i64,
        flow_state: &FlowState,
        sweep_state: &SweepState,
        vpin_state: &VpinState,
        snapshots: &[PriceSnapshot],
    ) -> LiquidationState {
        if !self.params.enabled {
            return empty_liquidation_state(now_ts);
        }

        let mut state = empty_liquidation_state(now_ts);
        state.metrics.enabled = true;
        state.metrics.lookback_ms = self.params.lookback_ms;
        state.metrics.cluster_band_bps = self.params.cluster_band_bps;
        state.metrics.proximity_threshold_bps = self.params.proximity_threshold_bps;
        state.metrics.reason_codes.clear();

        let Some(current_snapshot) = snapshots.last() else {
            state
                .metrics
                .reason_codes
                .push("no_price_history".to_string());
            return state;
        };
        let current_mid = current_snapshot.index_mid;
        state.metrics.current_mid = Some(current_mid);

        let dominant_direction = detect_dominant_direction(flow_state);
        state.metrics.dominant_direction = dominant_direction;

        let band_width = (current_mid * self.params.cluster_band_bps / 10_000.0).max(0.5);
        let mut buckets: BTreeMap<(LiquidationClusterSide, i64), ClusterAccumulator> =
            BTreeMap::new();

        for snapshot in snapshots {
            let distance_bps = ((snapshot.index_mid - current_mid) / current_mid) * 10_000.0;
            let abs_distance_bps = distance_bps.abs();
            if abs_distance_bps < self.params.min_cluster_distance_bps
                || abs_distance_bps > self.params.max_cluster_distance_bps
            {
                continue;
            }

            let side = if distance_bps > 0.0 {
                LiquidationClusterSide::ShortAbove
            } else {
                LiquidationClusterSide::LongBelow
            };
            let bucket_key = (side, (snapshot.index_mid / band_width).round() as i64);
            let entry = buckets
                .entry(bucket_key)
                .or_insert_with(|| ClusterAccumulator {
                    side,
                    sum_price: 0.0,
                    count: 0,
                    first_seen_ts: snapshot.ts,
                    last_seen_ts: snapshot.ts,
                });
            entry.sum_price += snapshot.index_mid;
            entry.count += 1;
            entry.first_seen_ts = entry.first_seen_ts.min(snapshot.ts);
            entry.last_seen_ts = entry.last_seen_ts.max(snapshot.ts);
        }

        let side_total_counts = side_counts(&buckets);
        let flow_5s = flow_state.windows.get("5000");
        let flow_60s = flow_state.windows.get("60000");
        let flow_signal = FlowSignal {
            buy_usd: flow_5s.map_or(0.0, |w| w.aggressive_buy_usd)
                + flow_60s.map_or(0.0, |w| w.aggressive_buy_usd * 0.5),
            sell_usd: flow_5s.map_or(0.0, |w| w.aggressive_sell_usd)
                + flow_60s.map_or(0.0, |w| w.aggressive_sell_usd * 0.5),
            net_btc: flow_5s.map_or(0.0, |w| w.net_aggressive_btc),
            abs_btc: flow_5s.map_or(0.0, |w| w.abs_aggressive_btc),
        };

        let mut clusters = buckets
            .into_values()
            .filter(|bucket| bucket.count >= self.params.min_touches)
            .map(|bucket| {
                let avg_price = bucket.sum_price / bucket.count as f64;
                let distance_bps = ((avg_price - current_mid) / current_mid).abs() * 10_000.0;
                let side_count = side_total_counts
                    .get(&bucket.side)
                    .copied()
                    .unwrap_or(bucket.count)
                    .max(bucket.count);
                let cluster_density = (bucket.count as f64 / side_count as f64).clamp(0.0, 1.0);
                let cluster_notional_usd = match bucket.side {
                    LiquidationClusterSide::ShortAbove => flow_signal.buy_usd * cluster_density,
                    LiquidationClusterSide::LongBelow => flow_signal.sell_usd * cluster_density,
                };
                EstimatedLiquidationCluster {
                    side: bucket.side,
                    price: avg_price,
                    distance_bps,
                    cluster_notional_usd,
                    cluster_density,
                    touched_snapshots: bucket.count,
                    first_seen_ts: bucket.first_seen_ts,
                    last_seen_ts: bucket.last_seen_ts,
                    reason_codes: vec![
                        "price_cluster_detected".to_string(),
                        match bucket.side {
                            LiquidationClusterSide::ShortAbove => "short_liq_cluster_above",
                            LiquidationClusterSide::LongBelow => "long_liq_cluster_below",
                        }
                        .to_string(),
                    ],
                }
            })
            .collect::<Vec<_>>();

        clusters.sort_by(|left, right| left.distance_bps.total_cmp(&right.distance_bps));

        let nearest_short = clusters
            .iter()
            .filter(|cluster| cluster.side == LiquidationClusterSide::ShortAbove)
            .min_by(|left, right| left.distance_bps.total_cmp(&right.distance_bps))
            .cloned();
        let nearest_long = clusters
            .iter()
            .filter(|cluster| cluster.side == LiquidationClusterSide::LongBelow)
            .min_by(|left, right| left.distance_bps.total_cmp(&right.distance_bps))
            .cloned();

        state.metrics.nearest_short_liq_cluster_above = nearest_short.clone();
        state.metrics.nearest_long_liq_cluster_below = nearest_long.clone();
        state.recent_clusters = clusters.iter().take(20).cloned().collect();

        let target_cluster = match dominant_direction {
            ToxicDirection::Buy => nearest_short,
            ToxicDirection::Sell => nearest_long,
            ToxicDirection::Neutral => None,
        };

        if let Some(cluster) = target_cluster {
            state.metrics.nearest_cluster_side = Some(cluster.side);
            state.metrics.distance_bps = Some(cluster.distance_bps);
            state.metrics.cluster_notional_usd = Some(cluster.cluster_notional_usd);
            state.metrics.cluster_density = Some(cluster.cluster_density);
            state.metrics.liq_cluster_nearby =
                cluster.distance_bps <= self.params.proximity_threshold_bps;

            let proximity_score = (1.0
                - (cluster.distance_bps / self.params.proximity_threshold_bps).min(1.0))
            .clamp(0.0, 1.0);
            let imbalance_score = if flow_signal.abs_btc > 0.0 {
                (flow_signal.net_btc.abs() / flow_signal.abs_btc).clamp(0.0, 1.0)
            } else {
                0.0
            };
            let sweep_score = matching_sweep_score(sweep_state, dominant_direction);
            let vpin_score = if vpin_state.metrics.vpin_extreme {
                1.0
            } else if vpin_state.metrics.vpin_spike {
                0.75
            } else if vpin_state.metrics.vpin_high {
                0.5
            } else {
                0.0
            };

            state.metrics.liq_hunt_pressure = (0.45 * proximity_score
                + 0.25 * imbalance_score
                + 0.15 * sweep_score
                + 0.15 * vpin_score)
                .clamp(0.0, 1.0);
            state.metrics.possible_liq_hunt_setup = state.metrics.liq_cluster_nearby
                && state.metrics.liq_hunt_pressure >= self.params.pressure_threshold;

            if state.metrics.liq_cluster_nearby {
                state
                    .metrics
                    .reason_codes
                    .push("liq_cluster_nearby".to_string());
            }
            if state.metrics.possible_liq_hunt_setup {
                state
                    .metrics
                    .reason_codes
                    .push("possible_liq_hunt_setup".to_string());
            }
            if cluster.cluster_density >= 0.5 {
                state
                    .metrics
                    .reason_codes
                    .push("cluster_density_high".to_string());
            }
        } else {
            state
                .metrics
                .reason_codes
                .push("no_cluster_target".to_string());
        }

        state
    }
}

fn side_counts(
    buckets: &BTreeMap<(LiquidationClusterSide, i64), ClusterAccumulator>,
) -> BTreeMap<LiquidationClusterSide, usize> {
    let mut counts = BTreeMap::new();
    for bucket in buckets.values() {
        *counts.entry(bucket.side).or_insert(0) += bucket.count;
    }
    counts
}

fn detect_dominant_direction(flow_state: &FlowState) -> ToxicDirection {
    flow_state
        .windows
        .get("5000")
        .map(|window| {
            if window.net_aggressive_btc > 0.0 {
                ToxicDirection::Buy
            } else if window.net_aggressive_btc < 0.0 {
                ToxicDirection::Sell
            } else {
                ToxicDirection::Neutral
            }
        })
        .unwrap_or(ToxicDirection::Neutral)
}

fn matching_sweep_score(sweep_state: &SweepState, direction: ToxicDirection) -> f64 {
    let Some(result) = sweep_state.results.get("5000") else {
        return 0.0;
    };
    if !result.sweep_detected {
        return 0.0;
    }
    match (result.direction, direction) {
        (SweepDirection::Buy, ToxicDirection::Buy)
        | (SweepDirection::Sell, ToxicDirection::Sell) => 1.0,
        _ => 0.0,
    }
}
