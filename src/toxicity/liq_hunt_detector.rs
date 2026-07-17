use crate::{
    config::thresholds::LiqHuntParams,
    types::{
        flow::{FlowState, FlowWindow},
        liq_hunt::{empty_liq_hunt_result, LiqHuntDirection, LiqHuntResult, LiqHuntSignalLevel},
        liquidation::{LiquidationClusterSide, LiquidationState},
        sweep::{SweepDirection, SweepState},
        toxic::{ToxicDirection, ToxicSeverity, ToxicState, ToxicVolumeResult},
        vpin::VpinState,
    },
};

#[derive(Debug, Clone)]
pub struct LiqHuntDetector {
    params: LiqHuntParams,
}

#[derive(Debug, Clone)]
pub struct LiqHuntDetectorInput {
    pub now_ts: i64,
    pub symbol: String,
    pub toxic_state: ToxicState,
    pub vpin_state: Option<VpinState>,
    pub sweep_state: SweepState,
    pub liquidation_state: LiquidationState,
    pub flow_state: FlowState,
}

impl LiqHuntDetector {
    pub fn new(params: LiqHuntParams) -> Self {
        Self { params }
    }

    pub fn with_params(&self, params: LiqHuntParams) -> Self {
        Self { params }
    }

    pub fn detect(&self, input: LiqHuntDetectorInput) -> LiqHuntResult {
        let mut result = empty_liq_hunt_result(input.now_ts);
        result.symbol = input.symbol.clone();

        let Some(toxic_result) = strongest_toxic_result(&input.toxic_state) else {
            result.reason_codes.push("no_toxic_signal".to_string());
            return result;
        };

        result.toxic_volume_btc = Some(toxic_result.toxic_volume_btc);
        result.toxic_severity = Some(severity_key(toxic_result.severity).to_string());
        result.toxic_direction = Some(direction_key(toxic_result.direction).to_string());

        let vpin_metrics = input.vpin_state.as_ref().map(|state| &state.metrics);
        result.vpin = vpin_metrics.and_then(|metrics| metrics.vpin);
        result.vpin_spike = vpin_metrics.is_some_and(|metrics| metrics.vpin_spike);
        result.vpin_high = vpin_metrics.is_some_and(|metrics| metrics.vpin_high);
        result.vpin_extreme = vpin_metrics.is_some_and(|metrics| metrics.vpin_extreme);

        let flow_window = preferred_flow_window(&input.flow_state);
        let sweep_window = input.sweep_state.results.get("5000");
        result.sweep_detected = sweep_window.is_some_and(|window| {
            window.sweep_detected
                && matches!(
                    (window.direction, toxic_result.direction),
                    (SweepDirection::Buy, ToxicDirection::Buy)
                        | (SweepDirection::Sell, ToxicDirection::Sell)
                )
        });
        result.local_liquidity_drain = toxic_result.liquidity_thin;
        result.spread_widened = toxic_result
            .liquidity
            .as_ref()
            .is_some_and(|liquidity| liquidity.spread_widened);

        result.nearest_cluster_side = input
            .liquidation_state
            .metrics
            .nearest_cluster_side
            .map(cluster_side_key)
            .map(str::to_string);
        result.nearest_cluster_distance_bps = input.liquidation_state.metrics.distance_bps;
        result.nearest_cluster_notional_usd = input.liquidation_state.metrics.cluster_notional_usd;

        let matched_direction = detect_liq_hunt_direction(
            toxic_result.direction,
            input.liquidation_state.metrics.nearest_cluster_side,
        );
        result.direction = matched_direction;

        result.liq_cluster_nearby = matches!(
            matched_direction,
            LiqHuntDirection::ShortSqueeze | LiqHuntDirection::LongSqueeze
        ) && input.liquidation_state.metrics.liq_cluster_nearby;
        result.possible_liq_hunt_setup =
            matches!(
                matched_direction,
                LiqHuntDirection::ShortSqueeze | LiqHuntDirection::LongSqueeze
            ) && input.liquidation_state.metrics.possible_liq_hunt_setup;

        let price_move_toward_cluster_bps = flow_window.and_then(|window| window.price_move_bps);
        result.price_move_toward_cluster_bps = price_move_toward_cluster_bps;
        result.price_distance_closing =
            price_distance_closing(matched_direction, price_move_toward_cluster_bps);

        let mut score: f64 = 0.0;
        let mut reasons = Vec::new();

        if toxic_result.toxic_volume_btc >= 600.0 {
            score += 15.0;
            reasons.push("toxic_volume_watch".to_string());
        }
        if toxic_result.toxic_volume_btc >= 1000.0 {
            score += 25.0;
            reasons.push("toxic_alert".to_string());
        }
        if toxic_result.severity.is_at_least(ToxicSeverity::Alert) {
            score += 20.0;
            reasons.push("toxic_severity_alert".to_string());
        }
        if result.vpin_spike {
            score += 15.0;
            reasons.push("vpin_spike".to_string());
        }
        if result.vpin_extreme {
            score += 20.0;
            reasons.push("vpin_extreme".to_string());
        }
        if result.sweep_detected {
            score += 15.0;
            reasons.push("sweep_detected".to_string());
        }
        if result.local_liquidity_drain {
            score += 15.0;
            reasons.push("local_liquidity_drain".to_string());
        }
        if result.liq_cluster_nearby {
            score += 20.0;
            reasons.push(
                match matched_direction {
                    LiqHuntDirection::ShortSqueeze => "short_cluster_above_nearby",
                    LiqHuntDirection::LongSqueeze => "long_cluster_below_nearby",
                    LiqHuntDirection::None => "liq_cluster_nearby",
                }
                .to_string(),
            );
        }
        if result.possible_liq_hunt_setup {
            score += 25.0;
            reasons.push(
                match matched_direction {
                    LiqHuntDirection::ShortSqueeze => "possible_short_squeeze",
                    LiqHuntDirection::LongSqueeze => "possible_long_squeeze",
                    LiqHuntDirection::None => "possible_liq_hunt_setup",
                }
                .to_string(),
            );
        }
        if result
            .nearest_cluster_distance_bps
            .is_some_and(|distance| distance <= self.params.near_distance_bps)
        {
            score += 15.0;
            reasons.push("cluster_distance_near".to_string());
        }
        if result
            .nearest_cluster_notional_usd
            .is_some_and(|notional| notional >= self.params.cluster_large_notional_usd)
        {
            score += 10.0;
            reasons.push("cluster_notional_large".to_string());
        }
        if result.price_distance_closing {
            score += 20.0;
            reasons.push(
                match matched_direction {
                    LiqHuntDirection::ShortSqueeze => "price_moving_toward_short_cluster",
                    LiqHuntDirection::LongSqueeze => "price_moving_toward_long_cluster",
                    LiqHuntDirection::None => "price_moving_toward_cluster",
                }
                .to_string(),
            );
        }
        if toxic_result.cross_venue_confirmed {
            score += 10.0;
            reasons.push("cross_venue_confirmed".to_string());
        }

        if matched_direction == LiqHuntDirection::ShortSqueeze {
            reasons.push("buy_toxic_into_short_cluster".to_string());
        } else if matched_direction == LiqHuntDirection::LongSqueeze {
            reasons.push("sell_toxic_into_long_cluster".to_string());
        }

        score = score.min(100.0);
        let mut level = level_for_score(score, &self.params);

        if matched_direction == LiqHuntDirection::None {
            level = cap_level(level, LiqHuntSignalLevel::Watch);
            reasons.push("direction_mismatch".to_string());
        }
        if !result.liq_cluster_nearby {
            level = cap_level(level, LiqHuntSignalLevel::Watch);
            reasons.push("no_liquidation_cluster_confirmation".to_string());
        }
        if toxic_result.direction == ToxicDirection::Neutral
            || toxic_result.toxic_volume_btc < 600.0
        {
            level = cap_level(level, LiqHuntSignalLevel::Watch);
            reasons.push("insufficient_toxic_flow".to_string());
        }
        if !result.price_distance_closing {
            level = cap_level(level, LiqHuntSignalLevel::Likely);
        }
        if input.liquidation_state.metrics.current_mid.is_none() {
            level = cap_level(level, LiqHuntSignalLevel::Watch);
            reasons.push("liquidation_state_incomplete".to_string());
        }

        result.level = level;
        result.score = score;
        result.reason_codes = reasons;
        result
    }
}

fn strongest_toxic_result(state: &ToxicState) -> Option<&ToxicVolumeResult> {
    state
        .results
        .values()
        .max_by(|left, right| left.toxic_volume_btc.total_cmp(&right.toxic_volume_btc))
}

fn preferred_flow_window(state: &FlowState) -> Option<&FlowWindow> {
    state
        .windows
        .get("5000")
        .or_else(|| state.windows.values().next())
}

fn detect_liq_hunt_direction(
    toxic_direction: ToxicDirection,
    cluster_side: Option<LiquidationClusterSide>,
) -> LiqHuntDirection {
    match (toxic_direction, cluster_side) {
        (ToxicDirection::Buy, Some(LiquidationClusterSide::ShortAbove)) => {
            LiqHuntDirection::ShortSqueeze
        }
        (ToxicDirection::Sell, Some(LiquidationClusterSide::LongBelow)) => {
            LiqHuntDirection::LongSqueeze
        }
        _ => LiqHuntDirection::None,
    }
}

fn price_distance_closing(
    direction: LiqHuntDirection,
    price_move_toward_cluster_bps: Option<f64>,
) -> bool {
    match (direction, price_move_toward_cluster_bps) {
        (LiqHuntDirection::ShortSqueeze, Some(move_bps)) => move_bps > 0.0,
        (LiqHuntDirection::LongSqueeze, Some(move_bps)) => move_bps < 0.0,
        _ => false,
    }
}

fn level_for_score(score: f64, params: &LiqHuntParams) -> LiqHuntSignalLevel {
    if score >= params.active_score {
        LiqHuntSignalLevel::Active
    } else if score >= params.likely_score {
        LiqHuntSignalLevel::Likely
    } else if score >= params.watch_score {
        LiqHuntSignalLevel::Watch
    } else {
        LiqHuntSignalLevel::None
    }
}

fn cap_level(current: LiqHuntSignalLevel, max_allowed: LiqHuntSignalLevel) -> LiqHuntSignalLevel {
    if current.rank() > max_allowed.rank() {
        max_allowed
    } else {
        current
    }
}

fn severity_key(severity: ToxicSeverity) -> &'static str {
    match severity {
        ToxicSeverity::Normal => "normal",
        ToxicSeverity::Watch => "watch",
        ToxicSeverity::Warning => "warning",
        ToxicSeverity::Alert => "alert",
        ToxicSeverity::Extreme => "extreme",
    }
}

fn direction_key(direction: ToxicDirection) -> &'static str {
    match direction {
        ToxicDirection::Buy => "buy",
        ToxicDirection::Sell => "sell",
        ToxicDirection::Neutral => "neutral",
    }
}

fn cluster_side_key(side: LiquidationClusterSide) -> &'static str {
    match side {
        LiquidationClusterSide::ShortAbove => "short_above",
        LiquidationClusterSide::LongBelow => "long_below",
    }
}
