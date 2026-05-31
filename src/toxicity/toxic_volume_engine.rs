use crate::{
    config::thresholds::ToxicVolumeParams,
    toxicity::cross_venue_confirmation::{cross_venue_confirmed, leader_venue},
    types::{
        flow::{FlowWindow, VenueFlowBreakdown},
        liquidation::{LiquidationClusterSide, LiquidationState},
        markout::MarkoutState,
        sweep::{SweepDirection, SweepResult, SweepState},
        toxic::{
            empty_venue_toxic_breakdown, ToxicDirection, ToxicEvent, ToxicSeverity,
            ToxicVolumeResult, VenueToxicBreakdown,
        },
        vpin::VpinState,
    },
};

#[derive(Debug, Clone)]
pub struct ToxicVolumeEngine {
    params: ToxicVolumeParams,
}

impl ToxicVolumeEngine {
    pub fn new(params: ToxicVolumeParams) -> Self {
        Self { params }
    }

    pub fn params(&self) -> &ToxicVolumeParams {
        &self.params
    }

    pub fn compute_window(
        &self,
        flow_window: &FlowWindow,
        markout_state: &MarkoutState,
        sweep_state: &SweepState,
        vpin_state: &VpinState,
        liquidation_state: &LiquidationState,
    ) -> ToxicVolumeResult {
        let direction = detect_direction(flow_window.net_aggressive_btc);
        let aggressive_direction_volume_btc = match direction {
            ToxicDirection::Buy => flow_window.aggressive_buy_btc,
            ToxicDirection::Sell => flow_window.aggressive_sell_btc,
            ToxicDirection::Neutral => 0.0,
        };

        let markout_1s_bps = markout_bps(markout_state, 1000, direction);
        let markout_5s_bps = markout_bps(markout_state, 5000, direction);
        let sweep_result = sweep_result_for_window(sweep_state, flow_window.window_ms);
        let sweep_detected = sweep_result.is_some_and(|result| {
            result.sweep_detected && sweep_direction_matches(result, direction)
        });
        let liquidity = sweep_result.and_then(|result| result.liquidity.clone());
        let liquidity_thin = sweep_result
            .and_then(|result| result.liquidity.as_ref())
            .is_some_and(|liquidity| match direction {
                ToxicDirection::Buy => {
                    liquidity
                        .ask_depth_drop_ratio
                        .is_some_and(|ratio| ratio >= self.params.min_depth_drop_ratio)
                        || liquidity.spread_widened
                }
                ToxicDirection::Sell => {
                    liquidity
                        .bid_depth_drop_ratio
                        .is_some_and(|ratio| ratio >= self.params.min_depth_drop_ratio)
                        || liquidity.spread_widened
                }
                ToxicDirection::Neutral => false,
            });
        let cross_venue_confirmed = cross_venue_confirmed(
            direction,
            &flow_window.venue_breakdown,
            self.params.min_cross_venue_count,
        );
        let leader_venue_value = leader_venue(direction, &flow_window.venue_breakdown);
        let leader_venue_diffusion = leader_venue_value.is_some() && cross_venue_confirmed;
        let vpin_metrics = &vpin_state.metrics;
        let vpin_enabled = vpin_metrics.enabled;
        let vpin = vpin_metrics.vpin;
        let vpin_zscore = vpin_metrics.vpin_zscore;
        let vpin_extreme = vpin_metrics.vpin_extreme;
        let vpin_spike = vpin_metrics.vpin_spike;
        let vpin_high = vpin_metrics.vpin_high;
        let liquidation_metrics = &liquidation_state.metrics;
        let liquidation_enabled = liquidation_metrics.enabled;
        let nearest_cluster_side = liquidation_metrics.nearest_cluster_side;
        let cluster_distance_bps = liquidation_metrics.distance_bps;
        let cluster_notional_usd = liquidation_metrics.cluster_notional_usd;
        let cluster_density = liquidation_metrics.cluster_density;
        let liq_hunt_pressure = liquidation_metrics.liq_hunt_pressure;
        let liq_cluster_nearby = liquidation_metrics.liq_cluster_nearby
            && matches!(
                (direction, liquidation_metrics.nearest_cluster_side),
                (
                    ToxicDirection::Buy,
                    Some(LiquidationClusterSide::ShortAbove)
                ) | (
                    ToxicDirection::Sell,
                    Some(LiquidationClusterSide::LongBelow)
                )
            );
        let possible_liq_hunt_setup = liquidation_metrics.possible_liq_hunt_setup
            && direction == liquidation_metrics.dominant_direction;

        let mut toxic_ratio: f64 = 0.0;
        let mut reason_codes = Vec::new();

        if direction == ToxicDirection::Neutral {
            reason_codes.push("neutral_direction".to_string());
        }
        if aggressive_direction_volume_btc >= self.params.min_large_flow_btc {
            toxic_ratio += 0.25;
            reason_codes.push("large_aggressive_flow".to_string());
        }
        if markout_1s_bps.is_some_and(|bps| bps > self.params.markout_1s_bps) {
            toxic_ratio += 0.20;
            reason_codes.push("markout_1s_confirmed".to_string());
        }
        if markout_5s_bps.is_some_and(|bps| bps > self.params.markout_5s_bps) {
            toxic_ratio += 0.25;
            reason_codes.push("markout_5s_confirmed".to_string());
        }
        if sweep_detected {
            toxic_ratio += 0.25;
            reason_codes.push("sweep_detected".to_string());
        }
        if liquidity_thin {
            toxic_ratio += 0.15;
            reason_codes.push("liquidity_thin".to_string());
        }
        if cross_venue_confirmed {
            toxic_ratio += 0.15;
            reason_codes.push("cross_venue_confirmed".to_string());
        }
        if leader_venue_diffusion {
            toxic_ratio += 0.15;
            reason_codes.push("leader_venue_diffusion".to_string());
        }
        if vpin_enabled {
            if vpin_extreme {
                toxic_ratio += 0.20;
                reason_codes.push("vpin_extreme".to_string());
            } else if vpin_spike {
                toxic_ratio += 0.15;
                reason_codes.push("vpin_spike".to_string());
            } else if vpin_high {
                toxic_ratio += 0.10;
                reason_codes.push("vpin_high".to_string());
            }
        }
        if liq_cluster_nearby {
            toxic_ratio += 0.10;
            reason_codes.push("liq_cluster_nearby".to_string());
        }
        if possible_liq_hunt_setup {
            toxic_ratio += 0.10;
            reason_codes.push("possible_liq_hunt_setup".to_string());
        }

        let toxic_ratio = toxic_ratio.min(1.0_f64);
        let toxic_volume_btc = aggressive_direction_volume_btc * toxic_ratio;
        let severity =
            ToxicSeverity::from_toxic_volume(toxic_volume_btc, self.params.threshold_btc);
        let alert_triggered = toxic_volume_btc >= self.params.threshold_btc;
        if alert_triggered {
            reason_codes.push("threshold_crossed".to_string());
        } else if toxic_ratio < 1.0 {
            reason_codes.push("insufficient_toxic_ratio".to_string());
        }
        if flow_window.window_ms == 60_000 && toxic_volume_btc >= 600.0 && vpin_spike {
            reason_codes.push("tof_spike_1m".to_string());
        }

        ToxicVolumeResult {
            symbol: flow_window.symbol.clone(),
            window_ms: flow_window.window_ms,
            ts: flow_window.now_ts,
            direction,
            severity,
            toxic_ratio,
            toxic_volume_btc,
            threshold_btc: self.params.threshold_btc,
            alert_triggered,
            aggressive_buy_btc: flow_window.aggressive_buy_btc,
            aggressive_sell_btc: flow_window.aggressive_sell_btc,
            net_aggressive_btc: flow_window.net_aggressive_btc,
            abs_aggressive_btc: flow_window.abs_aggressive_btc,
            markout_1s_bps,
            markout_5s_bps,
            markout_confirmed: markout_1s_bps.is_some_and(|bps| bps > self.params.markout_1s_bps)
                || markout_5s_bps.is_some_and(|bps| bps > self.params.markout_5s_bps),
            sweep_detected,
            liquidity_thin,
            liquidity: liquidity.clone(),
            cross_venue_confirmed,
            vpin_enabled,
            vpin,
            vpin_zscore,
            vpin_spike,
            vpin_high,
            vpin_extreme,
            liquidation_enabled,
            nearest_cluster_side,
            cluster_distance_bps,
            cluster_notional_usd,
            cluster_density,
            liq_hunt_pressure,
            liq_cluster_nearby,
            possible_liq_hunt_setup,
            leader_venue: leader_venue_value,
            venue_breakdown: build_toxic_breakdown(
                direction,
                toxic_ratio,
                &flow_window.venue_breakdown,
            ),
            reason_codes,
        }
    }

    pub fn build_event_if_triggered(&self, result: &ToxicVolumeResult) -> Option<ToxicEvent> {
        result.alert_triggered.then(|| ToxicEvent {
            id: uuid::Uuid::new_v4().to_string(),
            ts: result.ts,
            symbol: result.symbol.clone(),
            direction: result.direction,
            severity: result.severity,
            toxic_volume_btc: result.toxic_volume_btc,
            threshold_btc: result.threshold_btc,
            window_ms: result.window_ms,
            leader_venue: result.leader_venue,
            aggressive_buy_btc: result.aggressive_buy_btc,
            aggressive_sell_btc: result.aggressive_sell_btc,
            net_aggressive_btc: result.net_aggressive_btc,
            abs_aggressive_btc: result.abs_aggressive_btc,
            markout_1s_bps: result.markout_1s_bps,
            markout_5s_bps: result.markout_5s_bps,
            sweep_detected: result.sweep_detected,
            liquidity_thin: result.liquidity_thin,
            liquidity: result.liquidity.clone(),
            cross_venue_confirmed: result.cross_venue_confirmed,
            vpin_enabled: result.vpin_enabled,
            vpin: result.vpin,
            vpin_zscore: result.vpin_zscore,
            vpin_spike: result.vpin_spike,
            vpin_high: result.vpin_high,
            vpin_extreme: result.vpin_extreme,
            liquidation_enabled: result.liquidation_enabled,
            nearest_cluster_side: result.nearest_cluster_side,
            cluster_distance_bps: result.cluster_distance_bps,
            cluster_notional_usd: result.cluster_notional_usd,
            cluster_density: result.cluster_density,
            liq_hunt_pressure: result.liq_hunt_pressure,
            liq_cluster_nearby: result.liq_cluster_nearby,
            possible_liq_hunt_setup: result.possible_liq_hunt_setup,
            reason_codes: result.reason_codes.clone(),
        })
    }
}

pub fn detect_direction(net_aggressive_btc: f64) -> ToxicDirection {
    if net_aggressive_btc > 0.0 {
        ToxicDirection::Buy
    } else if net_aggressive_btc < 0.0 {
        ToxicDirection::Sell
    } else {
        ToxicDirection::Neutral
    }
}

pub fn map_sweep_window(window_ms: u64) -> u64 {
    match window_ms {
        1000 => 1000,
        5000 => 5000,
        15000 => 15000,
        60000 => 15000,
        _ => 15000,
    }
}

fn markout_bps(
    markout_state: &MarkoutState,
    horizon_ms: u64,
    direction: ToxicDirection,
) -> Option<f64> {
    let summary = markout_state.summaries.get(&horizon_ms.to_string())?;
    match direction {
        ToxicDirection::Buy => summary.buy.volume_weighted_markout_bps,
        ToxicDirection::Sell => summary.sell.volume_weighted_markout_bps,
        ToxicDirection::Neutral => None,
    }
}

fn sweep_result_for_window(sweep_state: &SweepState, toxic_window_ms: u64) -> Option<&SweepResult> {
    sweep_state
        .results
        .get(&map_sweep_window(toxic_window_ms).to_string())
}

fn sweep_direction_matches(result: &SweepResult, direction: ToxicDirection) -> bool {
    matches!(
        (result.direction, direction),
        (SweepDirection::Buy, ToxicDirection::Buy) | (SweepDirection::Sell, ToxicDirection::Sell)
    )
}

fn build_toxic_breakdown(
    direction: ToxicDirection,
    toxic_ratio: f64,
    flow_breakdown: &std::collections::BTreeMap<String, VenueFlowBreakdown>,
) -> std::collections::BTreeMap<String, VenueToxicBreakdown> {
    let mut output = empty_venue_toxic_breakdown();
    for (venue, flow) in flow_breakdown {
        let mut toxic = VenueToxicBreakdown {
            aggressive_buy_btc: flow.aggressive_buy_btc,
            aggressive_sell_btc: flow.aggressive_sell_btc,
            net_aggressive_btc: flow.net_aggressive_btc,
            trade_count: flow.trade_count,
            ..VenueToxicBreakdown::default()
        };
        match direction {
            ToxicDirection::Buy => toxic.toxic_buy_btc = flow.aggressive_buy_btc * toxic_ratio,
            ToxicDirection::Sell => toxic.toxic_sell_btc = flow.aggressive_sell_btc * toxic_ratio,
            ToxicDirection::Neutral => {}
        }
        toxic.toxic_volume_btc = toxic.toxic_buy_btc + toxic.toxic_sell_btc;
        output.insert(venue.clone(), toxic);
    }
    output
}
