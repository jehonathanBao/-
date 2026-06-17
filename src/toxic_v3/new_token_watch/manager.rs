use std::{collections::BTreeMap, sync::Arc};

use parking_lot::RwLock;
use thiserror::Error;

use super::{
    collector::ContractFlowCollector,
    engine::NewTokenFlowEngine,
    types::{
        BehaviorProbabilities, CapitalPhase, CapitalTimeline, CapitalTimelinePhase, ContractTick,
        CostDistributionBand, DecisionOrderType, DecisionTiming, ForcedFlowAttribution,
        LiquidationZone, LiquidityForceState, LiquidityReactionMap, LiquidityVacuumZone,
        MarketDynamicsState, MarketEnergy, MarketStateVector, MarketStateVelocity,
        PhaseTimelineSegment, PositionFlowCurve, PositionFlowPoint, PriceImpactDecomposition,
        RegimeTransitionProbability, SmartLevel, SmartMoneyChartResponse,
        SmartMoneyReconstructionResponse, StabilityRegime, StopLossCascadeState, TokenChartMarker,
        TokenChartPoint, TokenWatchItem, TokenWatchListResponse, TradingDecisionEntry,
        TradingDecisionExit, TradingDecisionKernel, TradingInvalidation, TradingPositionSize,
        MAX_ACTIVE_TOKENS,
    },
};
use crate::normalizers::trade::now_ms;

#[derive(Debug, Error, PartialEq, Eq)]
pub enum TokenWatchError {
    #[error("invalid_symbol")]
    InvalidSymbol,
    #[error("max_active_tokens_reached")]
    MaxActiveTokensReached,
    #[error("token_not_found")]
    TokenNotFound,
}

#[derive(Debug, Clone, Default)]
pub struct TokenWatchManager {
    items: Arc<RwLock<BTreeMap<String, TokenWatchItem>>>,
}

impl TokenWatchManager {
    pub fn add_token(&self, raw_symbol: &str) -> Result<TokenWatchItem, TokenWatchError> {
        let symbol = normalize_symbol(raw_symbol)?;
        let mut guard = self.items.write();
        if let Some(existing) = guard.get(&symbol) {
            return Ok(existing.clone());
        }
        if guard.len() >= MAX_ACTIVE_TOKENS {
            return Err(TokenWatchError::MaxActiveTokensReached);
        }
        let now = now_ms();
        let ticks = ContractFlowCollector::deterministic_probe_ticks(&symbol, now as u64);
        let item = TokenWatchItem {
            symbol: symbol.clone(),
            added_at_ms: now,
            stream_status: "read_only_probe".to_string(),
            last_signal: NewTokenFlowEngine::analyze_ticks(&symbol, &ticks),
            read_only: true,
        };
        guard.insert(symbol, item.clone());
        Ok(item)
    }

    pub fn remove_token(&self, raw_symbol: &str) -> Result<TokenWatchItem, TokenWatchError> {
        let symbol = normalize_symbol(raw_symbol)?;
        self.items
            .write()
            .remove(&symbol)
            .ok_or(TokenWatchError::TokenNotFound)
    }

    pub fn list_active_tokens(&self) -> TokenWatchListResponse {
        let now = now_ms();
        let mut guard = self.items.write();
        for item in guard.values_mut() {
            let ticks = ContractFlowCollector::deterministic_probe_ticks(&item.symbol, now as u64);
            item.last_signal = NewTokenFlowEngine::analyze_ticks(&item.symbol, &ticks);
            item.stream_status = "read_only_probe".to_string();
        }
        let items = guard.values().cloned().collect::<Vec<_>>();
        TokenWatchListResponse {
            active_count: items.len(),
            items,
            max_active_tokens: MAX_ACTIVE_TOKENS,
            read_only: true,
        }
    }

    pub fn record_tick(&self, tick: ContractTick) -> Result<TokenWatchItem, TokenWatchError> {
        let symbol = normalize_symbol(&tick.symbol)?;
        let mut guard = self.items.write();
        let item = guard
            .get_mut(&symbol)
            .ok_or(TokenWatchError::TokenNotFound)?;
        item.last_signal = NewTokenFlowEngine::analyze_ticks(&symbol, &[tick]);
        item.stream_status = "test_tick_observed".to_string();
        Ok(item.clone())
    }

    pub fn get_reconstruction(
        &self,
        raw_symbol: &str,
        timeframe: &str,
    ) -> Result<SmartMoneyReconstructionResponse, TokenWatchError> {
        let item = self.refresh_item(raw_symbol)?;
        Ok(build_reconstruction_response(&item, timeframe))
    }

    pub fn get_chart(
        &self,
        raw_symbol: &str,
        timeframe: &str,
    ) -> Result<SmartMoneyChartResponse, TokenWatchError> {
        let item = self.refresh_item(raw_symbol)?;
        Ok(build_chart_response(&item, timeframe))
    }

    fn refresh_item(&self, raw_symbol: &str) -> Result<TokenWatchItem, TokenWatchError> {
        let symbol = normalize_symbol(raw_symbol)?;
        let now = now_ms();
        let mut guard = self.items.write();
        let item = guard
            .get_mut(&symbol)
            .ok_or(TokenWatchError::TokenNotFound)?;
        let ticks = ContractFlowCollector::deterministic_probe_ticks(&item.symbol, now as u64);
        item.last_signal = NewTokenFlowEngine::analyze_ticks(&item.symbol, &ticks);
        item.stream_status = "read_only_probe".to_string();
        Ok(item.clone())
    }

    #[cfg(test)]
    pub fn clear(&self) {
        self.items.write().clear();
    }
}

fn build_reconstruction_response(
    item: &TokenWatchItem,
    timeframe: &str,
) -> SmartMoneyReconstructionResponse {
    let tf = normalize_timeframe(timeframe);
    let signal = &item.last_signal;
    let capital = &signal.capital_structure;
    let reconstruction = &signal.position_reconstruction;
    let cost = &capital.cost_basis;
    let position = &capital.estimated_position;
    let current_price = current_price(item).max(0.0);
    let estimated_net_position_base = reconstruction
        .latent_position
        .last()
        .map(|point| point.impact_adjusted_position)
        .unwrap_or_else(|| {
            if current_price > 0.0 {
                position.lower_usd / current_price
            } else {
                0.0
            }
        });
    let estimated_net_position_usdt = estimated_net_position_base * current_price;
    let floating_pnl_low_pct = pct_change(current_price, cost.lower);
    let floating_pnl_high_pct = pct_change(current_price, cost.upper);
    let phase_timeline = build_phase_timeline(item);
    let capital_timeline = build_capital_timeline(item, &phase_timeline, current_price);
    let position_flow_curve = build_position_flow_curve(item);
    let liquidity_reaction_map = build_liquidity_reaction_map(item, current_price);
    let market_dynamics = build_market_dynamics(
        item,
        current_price,
        &position_flow_curve,
        &liquidity_reaction_map,
    );
    let liquidity_force = build_liquidity_force(
        item,
        current_price,
        &liquidity_reaction_map,
        &market_dynamics,
    );
    let trading_decision = build_trading_decision(
        item,
        current_price,
        &liquidity_reaction_map,
        &market_dynamics,
        &liquidity_force,
    );
    SmartMoneyReconstructionResponse {
        symbol: item.symbol.clone(),
        timeframe: tf,
        current_phase: capital.phase,
        current_price,
        change_24h_pct: None,
        volume_24h_usd: None,
        high_24h: None,
        low_24h: None,
        market_cap_usd: None,
        cost_basis_low: cost.lower,
        cost_basis_high: cost.upper,
        vwap_anchor: cost.vwap_anchor,
        density_peak: cost.density_peak,
        estimated_total_position_usdt_low: position.lower_usd,
        estimated_total_position_usdt_high: position.upper_usd,
        estimated_net_position_usdt,
        floating_pnl_low_pct,
        floating_pnl_high_pct,
        accumulation_path: reconstruction.accumulation_path.clone(),
        last_accumulation_node: reconstruction.last_accumulation_node.clone(),
        distribution_path: reconstruction.distribution_path.clone(),
        distribution_completion_pct: distribution_completion(reconstruction),
        distribution_intensity_score: capital.distribution_risk.score * 100.0,
        short_term_behavior_probabilities: build_behavior_probabilities(item),
        behavior_windows: capital.behavior_windows.clone(),
        capital_timeline,
        position_flow_curve,
        liquidity_reaction_map,
        market_dynamics,
        liquidity_force,
        trading_decision,
        phase_timeline,
        cost_distribution: build_cost_distribution(item),
        smart_levels: build_smart_levels(item),
        confidence: reconstruction.confidence.max(capital.phase_confidence),
        read_only: true,
    }
}

fn build_chart_response(item: &TokenWatchItem, timeframe: &str) -> SmartMoneyChartResponse {
    let reconstruction = &item.last_signal.position_reconstruction;
    let mut previous_position = 0.0;
    let points = reconstruction
        .latent_position
        .iter()
        .map(|point| {
            let volume = (point.impact_adjusted_position - previous_position).abs();
            previous_position = point.impact_adjusted_position;
            TokenChartPoint {
                ts: point.timestamp,
                price: point.price,
                volume,
                net_position: point.impact_adjusted_position,
            }
        })
        .collect::<Vec<_>>();
    let markers = build_chart_markers(item);
    SmartMoneyChartResponse {
        symbol: item.symbol.clone(),
        timeframe: normalize_timeframe(timeframe),
        points,
        phase_segments: build_phase_timeline(item),
        markers,
        read_only: true,
    }
}

fn normalize_timeframe(timeframe: &str) -> String {
    match timeframe {
        "1m" | "5m" | "15m" | "1h" | "4h" => timeframe.to_string(),
        _ => "15m".to_string(),
    }
}

fn current_price(item: &TokenWatchItem) -> f64 {
    item.last_signal
        .position_reconstruction
        .latent_position
        .last()
        .map(|point| point.price)
        .filter(|price| *price > 0.0)
        .unwrap_or(item.last_signal.capital_structure.cost_basis.vwap_anchor)
}

fn pct_change(current: f64, basis: f64) -> f64 {
    if basis <= 0.0 || current <= 0.0 {
        0.0
    } else {
        ((current - basis) / basis) * 100.0
    }
}

fn distribution_completion(reconstruction: &super::types::SmartMoneyPositionReconstruction) -> f64 {
    let accumulation_volume = reconstruction
        .accumulation_path
        .iter()
        .map(|segment| segment.cumulative_delta.abs())
        .sum::<f64>();
    let distribution_volume = reconstruction
        .distribution_path
        .iter()
        .map(|segment| segment.cumulative_delta.abs())
        .sum::<f64>();
    let total = accumulation_volume + distribution_volume;
    if total <= 0.0 {
        0.0
    } else {
        ((distribution_volume / total) * 100.0).clamp(0.0, 100.0)
    }
}

fn build_behavior_probabilities(item: &TokenWatchItem) -> BehaviorProbabilities {
    let phase = item.last_signal.capital_structure.phase;
    let distribution = item
        .last_signal
        .capital_structure
        .distribution_risk
        .score
        .clamp(0.0, 1.0);
    let confidence = item
        .last_signal
        .capital_structure
        .phase_confidence
        .clamp(0.0, 1.0);
    let (distribution_bias, range_bias, rebound_bias, accumulation_bias) = match phase {
        CapitalPhase::Distribution | CapitalPhase::Breakdown => {
            (0.55 + distribution * 0.35, 0.15, 0.1, 0.2)
        }
        CapitalPhase::Markup => (0.15 + distribution * 0.2, 0.15, 0.5, 0.2),
        CapitalPhase::Accumulation => (distribution * 0.2, 0.2, 0.25, 0.55),
        CapitalPhase::Neutral => (distribution * 0.25, 0.45, 0.15, 0.15 + confidence * 0.2),
    };
    let total = distribution_bias + range_bias + rebound_bias + accumulation_bias;
    BehaviorProbabilities {
        continue_distribution: distribution_bias / total,
        range_consolidation: range_bias / total,
        rebound_markup: rebound_bias / total,
        secondary_accumulation: accumulation_bias / total,
    }
}

fn build_phase_timeline(item: &TokenWatchItem) -> Vec<PhaseTimelineSegment> {
    let reconstruction = &item.last_signal.position_reconstruction;
    let mut cursor = item.added_at_ms.max(0) as u64;
    let mut segments = reconstruction
        .accumulation_path
        .iter()
        .chain(reconstruction.distribution_path.iter())
        .map(|segment| {
            let start_ms = cursor;
            let duration_ms = segment.duration_sec.saturating_mul(1000).max(1000);
            cursor = cursor.saturating_add(duration_ms);
            PhaseTimelineSegment {
                phase: segment.phase,
                label: segment.label.clone(),
                start_ms,
                end_ms: cursor,
                duration_sec: segment.duration_sec.max(1),
                lower: segment.start_price.min(segment.end_price),
                upper: segment.start_price.max(segment.end_price),
            }
        })
        .collect::<Vec<_>>();
    if segments.is_empty() {
        let price = current_price(item);
        segments.push(PhaseTimelineSegment {
            phase: item.last_signal.capital_structure.phase,
            label: item.last_signal.capital_structure.phase_label.clone(),
            start_ms: cursor,
            end_ms: cursor.saturating_add(60_000),
            duration_sec: 60,
            lower: price * 0.998,
            upper: price * 1.002,
        });
    }
    segments
}

fn build_capital_timeline(
    item: &TokenWatchItem,
    phase_timeline: &[PhaseTimelineSegment],
    current_price: f64,
) -> CapitalTimeline {
    let reconstruction = &item.last_signal.position_reconstruction;
    let source_segments = reconstruction
        .accumulation_path
        .iter()
        .chain(reconstruction.distribution_path.iter())
        .collect::<Vec<_>>();
    let phases = phase_timeline
        .iter()
        .map(|timeline| {
            let source = source_segments
                .iter()
                .copied()
                .find(|segment| segment.phase == timeline.phase && segment.label == timeline.label);
            let avg_price = source
                .map(|segment| (segment.start_price + segment.end_price) / 2.0)
                .unwrap_or((timeline.lower + timeline.upper) / 2.0)
                .max(current_price)
                .max(0.0);
            CapitalTimelinePhase {
                phase: timeline.phase,
                label: timeline.label.clone(),
                start_ms: timeline.start_ms,
                end_ms: timeline.end_ms,
                duration_sec: timeline.duration_sec,
                net_flow_usd: source
                    .map(|segment| segment.cumulative_delta * avg_price)
                    .unwrap_or_default(),
                transition_reason: source
                    .map(transition_reason_for_segment)
                    .unwrap_or_else(|| transition_reason_for_phase(timeline.phase).to_string()),
            }
        })
        .collect::<Vec<_>>();
    let total_duration_sec = phases.iter().map(|phase| phase.duration_sec).sum::<u64>();
    let dominant_phase = phases
        .iter()
        .max_by_key(|phase| phase.duration_sec)
        .map(|phase| phase.phase)
        .unwrap_or(item.last_signal.capital_structure.phase);

    CapitalTimeline {
        phases,
        dominant_phase,
        total_duration_sec,
        narrative: timeline_narrative(item.last_signal.capital_structure.phase, dominant_phase),
    }
}

fn transition_reason_for_segment(segment: &super::types::PositionPathSegment) -> String {
    if segment
        .characteristics
        .iter()
        .any(|item| item == "volatility_compression" || item == "minimal_impact_flow")
    {
        "low volatility absorption".to_string()
    } else if segment
        .characteristics
        .iter()
        .any(|item| item == "bid_liquidity_depletion")
    {
        "liquidity exhaustion".to_string()
    } else if segment.cumulative_delta < 0.0 {
        "negative delta persistence".to_string()
    } else if segment.cumulative_delta > 0.0 {
        "positive delta persistence".to_string()
    } else {
        transition_reason_for_phase(segment.phase).to_string()
    }
}

fn transition_reason_for_phase(phase: CapitalPhase) -> &'static str {
    match phase {
        CapitalPhase::Accumulation => "low volatility absorption",
        CapitalPhase::Markup => "volatility expansion",
        CapitalPhase::Distribution => "delta divergence",
        CapitalPhase::Breakdown => "liquidity exhaustion",
        CapitalPhase::Neutral => "mixed flow",
    }
}

fn timeline_narrative(current: CapitalPhase, dominant: CapitalPhase) -> String {
    if current == dominant {
        format!("dominant_{}_phase", phase_label(current))
    } else {
        format!(
            "{}_inside_{}_timeline",
            phase_label(current),
            phase_label(dominant)
        )
    }
}

fn phase_label(phase: CapitalPhase) -> &'static str {
    match phase {
        CapitalPhase::Accumulation => "accumulation",
        CapitalPhase::Markup => "markup",
        CapitalPhase::Distribution => "distribution",
        CapitalPhase::Breakdown => "breakdown",
        CapitalPhase::Neutral => "neutral",
    }
}

fn build_position_flow_curve(item: &TokenWatchItem) -> PositionFlowCurve {
    let latent = &item.last_signal.position_reconstruction.latent_position;
    let mut previous_ts = None;
    let mut previous_position_usd = 0.0;
    let mut positive_speed_sum = 0.0;
    let mut positive_speed_count = 0.0;
    let mut negative_speed_sum = 0.0;
    let mut negative_speed_count = 0.0;

    let points = latent
        .iter()
        .map(|point| {
            let position_usd = point.impact_adjusted_position * point.price.max(0.0);
            let delta_position = position_usd - previous_position_usd;
            let elapsed_min = previous_ts
                .map(|ts| elapsed_minutes(ts, point.timestamp))
                .unwrap_or(5.0 / 60.0);
            let speed_usd_per_min = delta_position / elapsed_min.max(0.000_001);
            if speed_usd_per_min > 0.0 {
                positive_speed_sum += speed_usd_per_min;
                positive_speed_count += 1.0;
            } else if speed_usd_per_min < 0.0 {
                negative_speed_sum += speed_usd_per_min.abs();
                negative_speed_count += 1.0;
            }
            previous_ts = Some(point.timestamp);
            previous_position_usd = position_usd;
            PositionFlowPoint {
                ts: point.timestamp,
                position_usd,
                speed_usd_per_min,
            }
        })
        .collect::<Vec<_>>();

    PositionFlowCurve {
        latest_position_usd: points
            .last()
            .map(|point| point.position_usd)
            .unwrap_or_default(),
        accumulation_slope_usd_per_min: positive_speed_sum / f64::max(positive_speed_count, 1.0),
        distribution_slope_usd_per_min: negative_speed_sum / f64::max(negative_speed_count, 1.0),
        points,
    }
}

fn elapsed_minutes(previous_ts: u64, current_ts: u64) -> f64 {
    let delta = current_ts.saturating_sub(previous_ts);
    if delta >= 1000 {
        delta as f64 / 60_000.0
    } else {
        5.0 / 60.0
    }
}

fn build_liquidity_reaction_map(item: &TokenWatchItem, current_price: f64) -> LiquidityReactionMap {
    let signal = &item.last_signal;
    let impact = &signal.impact_response;
    let depletion = &signal.liquidity_depletion;
    let cost = &signal.capital_structure.cost_basis;
    let distribution_risk = signal.capital_structure.distribution_risk.score;
    let impact_efficiency = (impact.thin_liquidity_score * 0.55
        + (impact.price_move_pct.abs().min(0.08) / 0.08) * 0.45)
        .clamp(0.0, 1.0);
    let absorption_ratio = impact.absorption_score
        / (impact.absorption_score + impact.thin_liquidity_score + 0.000_001);
    let liquidity_response = if absorption_ratio >= 0.62 && impact.absorption_score >= 0.45 {
        "absorption_dominant"
    } else if impact.thin_liquidity_score >= 0.55 {
        "liquidity_vacuum"
    } else if distribution_risk >= 0.45 {
        "distribution_pressure"
    } else {
        "balanced_liquidity"
    };

    let mut vacuum_zones = Vec::new();
    if impact.thin_liquidity_score >= 0.25 && current_price > 0.0 {
        let width = current_price * (0.002 + impact.thin_liquidity_score.min(1.0) * 0.006);
        vacuum_zones.push(LiquidityVacuumZone {
            lower: (current_price - width).max(0.0),
            upper: current_price + width,
            intensity: impact.thin_liquidity_score.clamp(0.0, 1.0),
            reason: "thin liquidity around current price".to_string(),
        });
    }
    if distribution_risk >= 0.33 && cost.upper > 0.0 {
        vacuum_zones.push(LiquidityVacuumZone {
            lower: cost.upper,
            upper: cost.upper * 1.006,
            intensity: distribution_risk.clamp(0.0, 1.0),
            reason: "upper cost band distribution pressure".to_string(),
        });
    }
    if depletion.bid_depletion_rate > depletion.replenishment_rate && cost.lower > 0.0 {
        vacuum_zones.push(LiquidityVacuumZone {
            lower: cost.lower * 0.994,
            upper: cost.lower,
            intensity: depletion
                .bid_depletion_rate
                .max(depletion.depletion_pressure)
                .clamp(0.0, 1.0),
            reason: "bid depletion below cost band".to_string(),
        });
    }

    let mut evidence = vec![
        format!("impact_efficiency={impact_efficiency:.2}"),
        format!("absorption_ratio={absorption_ratio:.2}"),
        format!("liquidity_response={liquidity_response}"),
    ];
    if vacuum_zones.is_empty() {
        evidence.push("no_liquidity_vacuum_zone_confirmed".to_string());
    } else {
        evidence.push(format!("vacuum_zones={}", vacuum_zones.len()));
    }

    LiquidityReactionMap {
        impact_efficiency,
        absorption_ratio: absorption_ratio.clamp(0.0, 1.0),
        liquidity_response: liquidity_response.to_string(),
        vacuum_zones,
        evidence,
    }
}

fn build_market_dynamics(
    item: &TokenWatchItem,
    current_price: f64,
    position_flow_curve: &PositionFlowCurve,
    liquidity_reaction_map: &LiquidityReactionMap,
) -> MarketDynamicsState {
    let signal = &item.last_signal;
    let compression = &signal.signal_compression;
    let stable = &compression.stable_signals;
    let regime_state = &compression.regime_state;
    let capital = &signal.capital_structure;
    let cost = &capital.cost_basis;

    let flow_strength = stable.smp_stable.abs().max(stable.mfe_stable.abs());
    let liquidity_availability = (liquidity_reaction_map.absorption_ratio * 0.55
        + (1.0 - liquidity_reaction_map.impact_efficiency) * 0.45)
        .clamp(0.0, 1.0);
    let regime_stability = regime_state.stability.clamp(0.0, 1.0);
    let market_energy_score =
        (flow_strength * liquidity_availability * regime_stability).clamp(0.0, 1.0);

    let cost_pressure = if current_price > 0.0 && cost.vwap_anchor > 0.0 {
        ((current_price - cost.vwap_anchor) / cost.vwap_anchor)
            * (position_flow_curve.latest_position_usd.abs()
                / signal
                    .capital_structure
                    .estimated_position
                    .upper_usd
                    .max(1.0))
            .clamp(0.0, 1.5)
    } else {
        0.0
    };
    let flow_acceleration = (stable.smp_stable * 0.45 + stable.mfe_stable * 0.35
        - signal.impact_response.thin_liquidity_score * 0.20)
        .clamp(-1.0, 1.0);
    let liquidity_shift_rate = (liquidity_reaction_map.absorption_ratio
        - liquidity_reaction_map.impact_efficiency
        - signal.liquidity_depletion.depletion_pressure * 0.35)
        .clamp(-1.0, 1.0);
    let regime_transition_speed =
        ((1.0 - regime_state.stability) * (flow_strength + stable.lsm_stable.max(0.0)) * 0.5)
            .clamp(0.0, 1.0);
    let position_velocity_usd_per_min = if position_flow_curve.accumulation_slope_usd_per_min
        >= position_flow_curve.distribution_slope_usd_per_min
    {
        position_flow_curve.accumulation_slope_usd_per_min
    } else {
        -position_flow_curve.distribution_slope_usd_per_min
    };

    let state_vector = MarketStateVector {
        smp: stable.smp_stable,
        mfe: stable.mfe_stable,
        lsm: stable.lsm_stable,
        regime: regime_state.current,
        position_usd: position_flow_curve.latest_position_usd,
        cost_basis: cost.vwap_anchor,
        liquidity: liquidity_availability,
    };
    let state_velocity = MarketStateVelocity {
        flow_acceleration,
        liquidity_shift_rate,
        regime_transition_speed,
        position_velocity_usd_per_min,
    };
    let market_energy = MarketEnergy {
        score: market_energy_score,
        level: market_energy_level(market_energy_score).to_string(),
        flow_strength,
        liquidity_availability,
        regime_stability,
    };

    let transition_matrix = build_transition_matrix(
        capital.phase,
        market_energy_score,
        cost_pressure,
        &state_velocity,
    );
    let trajectory_summary = trajectory_summary(capital.phase, &state_velocity, &market_energy);

    MarketDynamicsState {
        state_vector,
        state_velocity,
        transition_matrix,
        market_energy,
        trajectory_summary,
        read_only: true,
    }
}

fn build_transition_matrix(
    current_phase: CapitalPhase,
    energy: f64,
    cost_pressure: f64,
    velocity: &MarketStateVelocity,
) -> Vec<RegimeTransitionProbability> {
    let markup_prob = (0.18 + energy * 0.52 + velocity.flow_acceleration.max(0.0) * 0.22
        - cost_pressure.max(0.0) * 0.12)
        .clamp(0.0, 1.0);
    let distribution_prob = (0.12
        + cost_pressure.max(0.0) * 0.45
        + (-velocity.liquidity_shift_rate).max(0.0) * 0.25
        + velocity.regime_transition_speed * 0.18)
        .clamp(0.0, 1.0);
    let accumulation_prob = (0.16
        + velocity.liquidity_shift_rate.max(0.0) * 0.36
        + (-cost_pressure).max(0.0) * 0.20
        + (1.0 - energy).max(0.0) * 0.18)
        .clamp(0.0, 1.0);
    let stress_prob = (0.08
        + (-velocity.liquidity_shift_rate).max(0.0) * 0.34
        + velocity.regime_transition_speed * 0.30)
        .clamp(0.0, 1.0);

    match current_phase {
        CapitalPhase::Accumulation => vec![
            transition(
                current_phase,
                CapitalPhase::Markup,
                markup_prob,
                "flow acceleration plus stable liquidity",
            ),
            transition(
                current_phase,
                CapitalPhase::Neutral,
                1.0 - markup_prob,
                "accumulation inertia",
            ),
        ],
        CapitalPhase::Markup => vec![
            transition(
                current_phase,
                CapitalPhase::Distribution,
                distribution_prob,
                "cost pressure and liquidity shift",
            ),
            transition(
                current_phase,
                CapitalPhase::Markup,
                1.0 - distribution_prob,
                "trend persistence",
            ),
        ],
        CapitalPhase::Distribution => vec![
            transition(
                current_phase,
                CapitalPhase::Breakdown,
                stress_prob,
                "liquidity depletion stress",
            ),
            transition(
                current_phase,
                CapitalPhase::Accumulation,
                accumulation_prob,
                "absorption rebuild",
            ),
        ],
        CapitalPhase::Breakdown => vec![
            transition(
                current_phase,
                CapitalPhase::Accumulation,
                accumulation_prob,
                "post stress re-accumulation",
            ),
            transition(
                current_phase,
                CapitalPhase::Neutral,
                1.0 - accumulation_prob,
                "breakdown inertia",
            ),
        ],
        CapitalPhase::Neutral => vec![
            transition(
                current_phase,
                CapitalPhase::Accumulation,
                accumulation_prob,
                "liquidity rebuild",
            ),
            transition(
                current_phase,
                CapitalPhase::Markup,
                markup_prob,
                "flow expansion",
            ),
            transition(
                current_phase,
                CapitalPhase::Distribution,
                distribution_prob,
                "cost pressure",
            ),
        ],
    }
}

fn transition(
    from: CapitalPhase,
    to: CapitalPhase,
    probability: f64,
    reason: &str,
) -> RegimeTransitionProbability {
    RegimeTransitionProbability {
        from,
        to,
        probability: probability.clamp(0.0, 1.0),
        reason: reason.to_string(),
    }
}

fn market_energy_level(score: f64) -> &'static str {
    if score >= 0.72 {
        "overheating"
    } else if score >= 0.48 {
        "high"
    } else if score >= 0.24 {
        "medium"
    } else {
        "low"
    }
}

fn trajectory_summary(
    phase: CapitalPhase,
    velocity: &MarketStateVelocity,
    energy: &MarketEnergy,
) -> String {
    if energy.score >= 0.48 && velocity.flow_acceleration > 0.15 {
        format!("{}_energy_expanding", phase_label(phase))
    } else if velocity.liquidity_shift_rate < -0.25 {
        format!("{}_liquidity_deteriorating", phase_label(phase))
    } else if velocity.regime_transition_speed > 0.45 {
        format!("{}_transition_risk_rising", phase_label(phase))
    } else {
        format!("{}_trajectory_stable", phase_label(phase))
    }
}

fn build_liquidity_force(
    item: &TokenWatchItem,
    current_price: f64,
    liquidity_reaction_map: &LiquidityReactionMap,
    market_dynamics: &MarketDynamicsState,
) -> LiquidityForceState {
    let signal = &item.last_signal;
    let stable = &signal.signal_compression.stable_signals;
    let actors = &signal.actor_decomposition;
    let impact = &signal.impact_response;
    let depletion = &signal.liquidity_depletion;
    let velocity = &market_dynamics.state_velocity;
    let energy = &market_dynamics.market_energy;

    let volatility_proxy = signal
        .capital_structure
        .behavior_windows
        .iter()
        .map(|window| (window.volatility_pct.abs() / 5.0).clamp(0.0, 1.0))
        .fold(0.0_f64, f64::max);
    let stress = stable
        .lsm_stable
        .max(0.0)
        .max(depletion.depletion_pressure)
        .max(impact.thin_liquidity_score)
        .clamp(0.0, 1.0);
    let upward_force = (stable.smp_stable.max(0.0) * 0.35
        + stable.mfe_stable.max(0.0) * 0.20
        + impact.thin_liquidity_score * 0.25
        + energy.score * 0.20)
        .clamp(0.0, 1.0);
    let downward_force = ((-stable.smp_stable).max(0.0) * 0.35
        + (-stable.mfe_stable).max(0.0) * 0.20
        + depletion.bid_depletion_rate * 0.25
        + stress * 0.20)
        .clamp(0.0, 1.0);

    let long_intensity =
        (downward_force * 0.55 + stress * 0.25 + volatility_proxy * 0.20).clamp(0.0, 1.0);
    let short_intensity =
        (upward_force * 0.55 + stress * 0.25 + volatility_proxy * 0.20).clamp(0.0, 1.0);
    let zone_width = current_price.max(1.0) * (0.003 + stress * 0.009 + volatility_proxy * 0.004);
    let zones = vec![
        LiquidationZone {
            side: "long_liquidation".to_string(),
            lower: (current_price - zone_width * 2.2).max(0.0),
            upper: (current_price - zone_width * 0.65).max(0.0),
            intensity: long_intensity,
            leverage_density: (long_intensity * 0.55 + stress * 0.45).clamp(0.0, 1.0),
            reason: "downside stop-loss and long liquidation proxy".to_string(),
        },
        LiquidationZone {
            side: "short_liquidation".to_string(),
            lower: current_price + zone_width * 0.65,
            upper: current_price + zone_width * 2.2,
            intensity: short_intensity,
            leverage_density: (short_intensity * 0.55 + stress * 0.45).clamp(0.0, 1.0),
            reason: "upside stop-loss and short liquidation proxy".to_string(),
        },
    ];

    let stop_hunt_probability = (impact.thin_liquidity_score * 0.32
        + stable.lsm_stable.max(0.0) * 0.24
        + velocity.regime_transition_speed * 0.18
        + volatility_proxy * 0.16
        + (1.0 - market_dynamics.state_vector.liquidity).clamp(0.0, 1.0) * 0.10)
        .clamp(0.0, 1.0);
    let cascade_intensity =
        (long_intensity.max(short_intensity) * 0.55 + stop_hunt_probability * 0.45).clamp(0.0, 1.0);
    let sweep_direction = if short_intensity > long_intensity + 0.08 {
        super::types::AdvisoryDirection::Long
    } else if long_intensity > short_intensity + 0.08 {
        super::types::AdvisoryDirection::Short
    } else {
        super::types::AdvisoryDirection::NoTrade
    };
    let liquidity_sweep = match sweep_direction {
        super::types::AdvisoryDirection::Long => "upside_short_sweep",
        super::types::AdvisoryDirection::Short => "downside_long_sweep",
        super::types::AdvisoryDirection::NoTrade => "balanced_sweep_risk",
    }
    .to_string();

    let whale_raw = (actors.smart_money_probability * 0.75
        + liquidity_reaction_map.absorption_ratio * 0.25)
        .max(0.0);
    let retail_raw = (actors.momentum_chaser_probability * 0.70
        + liquidity_reaction_map.impact_efficiency * 0.30)
        .max(0.0);
    let liquidation_raw = (cascade_intensity * 0.70 + stable.lsm_stable.max(0.0) * 0.30).max(0.0);
    let total = (whale_raw + retail_raw + liquidation_raw).max(0.000_001);
    let whale_pct = whale_raw / total;
    let retail_pct = retail_raw / total;
    let liquidation_pct = liquidation_raw / total;
    let dominant_driver = if liquidation_pct >= whale_pct.max(retail_pct) {
        "liquidation_cascade"
    } else if whale_pct >= retail_pct {
        "whale_initiated_flow"
    } else {
        "retail_chasing_flow"
    }
    .to_string();

    let primary_driver = dominant_driver.clone();
    let active_zone = if cascade_intensity < 0.30 {
        "neutral_zone"
    } else if matches!(sweep_direction, super::types::AdvisoryDirection::Long) {
        "short_squeeze_zone"
    } else if matches!(sweep_direction, super::types::AdvisoryDirection::Short) {
        "long_liquidation_zone"
    } else {
        "two_sided_stop_hunt_zone"
    }
    .to_string();

    LiquidityForceState {
        liquidation_zones: zones,
        stop_loss_cascade: StopLossCascadeState {
            stop_hunt_probability,
            cascade_intensity,
            sweep_direction,
            liquidity_sweep,
        },
        forced_flow_attribution: ForcedFlowAttribution {
            whale_pct,
            retail_pct,
            liquidation_pct,
            dominant_driver,
        },
        price_impact_decomposition: PriceImpactDecomposition {
            whale_impact: (whale_pct * (1.0 - impact.thin_liquidity_score * 0.35)).clamp(0.0, 1.0),
            liquidation_cascade: (liquidation_pct * cascade_intensity).clamp(0.0, 1.0),
            stop_loss_sweep: stop_hunt_probability,
            passive_absorption: liquidity_reaction_map.absorption_ratio,
        },
        primary_driver,
        active_zone,
        read_only: true,
    }
}

fn build_trading_decision(
    item: &TokenWatchItem,
    current_price: f64,
    liquidity_reaction_map: &LiquidityReactionMap,
    market_dynamics: &MarketDynamicsState,
    liquidity_force: &LiquidityForceState,
) -> TradingDecisionKernel {
    let signal = &item.last_signal;
    let compression = &signal.signal_compression;
    let stable = &compression.stable_signals;
    let regime_state = &compression.regime_state;
    let gate = &compression.position_validity_gate;
    let cost = &signal.capital_structure.cost_basis;
    let dynamics_vector = &market_dynamics.state_vector;
    let dynamics_velocity = &market_dynamics.state_velocity;
    let energy = &market_dynamics.market_energy;
    let forced = &liquidity_force.forced_flow_attribution;
    let cascade = &liquidity_force.stop_loss_cascade;

    let regime_trend =
        regime_direction_weight(regime_state.current, stable.smp_stable, stable.mfe_stable);
    let forced_flow_bias = match cascade.sweep_direction {
        super::types::AdvisoryDirection::Long => forced.liquidation_pct * 0.18,
        super::types::AdvisoryDirection::Short => -forced.liquidation_pct * 0.18,
        super::types::AdvisoryDirection::NoTrade => 0.0,
    };
    let directional_bias = (stable.smp_stable * 0.40 + stable.mfe_stable * 0.28
        - stable.lsm_stable * 0.22
        + regime_trend * 0.20
        + forced_flow_bias)
        .clamp(-1.0, 1.0);

    let invalidation_active = !gate.trade_permission
        || (stable.lsm_stable > 0.70 && stable.mfe_stable < -0.50)
        || (cascade.stop_hunt_probability > 0.72 && cascade.cascade_intensity > 0.55)
        || forced.liquidation_pct > 0.62
        || dynamics_velocity.liquidity_shift_rate < -0.65
        || matches!(
            regime_state.current,
            StabilityRegime::LiquidityStress | StabilityRegime::Manipulation
        ) && regime_state.stability < 0.45;

    let raw_direction = if directional_bias >= 0.28 {
        super::types::AdvisoryDirection::Long
    } else if directional_bias <= -0.28 {
        super::types::AdvisoryDirection::Short
    } else {
        super::types::AdvisoryDirection::NoTrade
    };
    let direction = if invalidation_active {
        super::types::AdvisoryDirection::NoTrade
    } else {
        raw_direction
    };

    let liquidity_supportive =
        dynamics_vector.liquidity >= 0.35 && liquidity_reaction_map.impact_efficiency <= 0.72;
    let regime_aligned = regime_state.stability >= 0.45
        && !matches!(regime_state.current, StabilityRegime::Manipulation);
    let cost_favorable = cost_pressure_favorable(
        direction,
        current_price,
        cost.vwap_anchor,
        cost.lower,
        cost.upper,
    );
    let entry_valid = !matches!(direction, super::types::AdvisoryDirection::NoTrade)
        && stable.stability_score >= 0.45
        && regime_aligned
        && liquidity_supportive
        && cost_favorable;

    let confidence = if entry_valid {
        ((directional_bias.abs() * 0.30)
            + stable.stability_score * 0.25
            + regime_state.stability * 0.20
            + dynamics_vector.liquidity * 0.15
            + energy.score * 0.10)
            .clamp(0.0, 1.0)
    } else {
        ((directional_bias.abs() * 0.25)
            + stable.stability_score * 0.20
            + regime_state.stability * 0.15)
            .clamp(0.0, 0.49)
    };

    let zone_deviation = current_price.max(cost.vwap_anchor).max(1.0)
        * (0.003 + (1.0 - dynamics_vector.liquidity).clamp(0.0, 1.0) * 0.004);
    let entry = build_decision_entry(
        direction,
        entry_valid,
        confidence,
        energy.score,
        cost.lower,
        cost.vwap_anchor,
        cost.upper,
        zone_deviation,
    );
    let exit = build_decision_exit(
        direction,
        current_price,
        cost.lower,
        cost.vwap_anchor,
        cost.upper,
        zone_deviation,
    );
    let invalidation = build_invalidation(
        direction,
        current_price,
        cost.lower,
        cost.upper,
        stable.smp_stable,
        stable.lsm_stable,
        regime_state.current,
        invalidation_active,
    );
    let position_size = build_position_size(
        direction,
        entry_valid,
        confidence,
        regime_state.stability,
        dynamics_vector.liquidity,
        energy.score,
        gate.position_size_multiplier,
        forced.liquidation_pct,
    );

    TradingDecisionKernel {
        direction,
        entry,
        exit,
        position_size,
        invalidation,
        confidence,
        advisory_only: true,
        read_only: true,
    }
}

fn regime_direction_weight(regime: StabilityRegime, smp: f64, mfe: f64) -> f64 {
    let directional_sign = if smp.abs() >= mfe.abs() {
        smp.signum()
    } else {
        mfe.signum()
    };
    match regime {
        StabilityRegime::Trend => 0.45 * directional_sign,
        StabilityRegime::LiquidityExpansion => 0.35 * directional_sign,
        StabilityRegime::Chop => 0.08 * directional_sign,
        StabilityRegime::LiquidityStress | StabilityRegime::Manipulation => {
            -0.25 * directional_sign
        }
        StabilityRegime::Neutral => 0.0,
    }
}

fn cost_pressure_favorable(
    direction: super::types::AdvisoryDirection,
    current_price: f64,
    vwap: f64,
    lower: f64,
    upper: f64,
) -> bool {
    if current_price <= 0.0 || vwap <= 0.0 {
        return false;
    }
    match direction {
        super::types::AdvisoryDirection::Long => current_price <= upper.max(vwap) * 1.035,
        super::types::AdvisoryDirection::Short => current_price >= lower.min(vwap) * 0.965,
        super::types::AdvisoryDirection::NoTrade => false,
    }
}

fn build_decision_entry(
    direction: super::types::AdvisoryDirection,
    entry_valid: bool,
    confidence: f64,
    energy: f64,
    lower: f64,
    vwap: f64,
    upper: f64,
    deviation: f64,
) -> TradingDecisionEntry {
    if !entry_valid {
        return TradingDecisionEntry {
            order_type: DecisionOrderType::None,
            zone_low: 0.0,
            zone_high: 0.0,
            timing: DecisionTiming::Invalid,
            condition: "wait_for_alignment_or_invalidation_clear".to_string(),
        };
    }

    let order_type = if confidence >= 0.72 && energy >= 0.36 {
        DecisionOrderType::Market
    } else {
        DecisionOrderType::Limit
    };
    let timing = if confidence >= 0.72 && energy >= 0.36 {
        DecisionTiming::Immediate
    } else {
        DecisionTiming::Wait
    };
    let (zone_low, zone_high, condition) = match direction {
        super::types::AdvisoryDirection::Long => (
            (lower - deviation).max(0.0),
            (vwap + deviation).max(0.0),
            "enter_near_cost_basis_when_smp_regime_liquidity_align",
        ),
        super::types::AdvisoryDirection::Short => (
            (vwap - deviation).max(0.0),
            (upper + deviation).max(0.0),
            "enter_near_upper_cost_band_when_distribution_pressure_persists",
        ),
        super::types::AdvisoryDirection::NoTrade => (0.0, 0.0, "no_entry"),
    };

    TradingDecisionEntry {
        order_type,
        zone_low,
        zone_high,
        timing,
        condition: condition.to_string(),
    }
}

fn build_decision_exit(
    direction: super::types::AdvisoryDirection,
    current_price: f64,
    lower: f64,
    vwap: f64,
    upper: f64,
    deviation: f64,
) -> TradingDecisionExit {
    match direction {
        super::types::AdvisoryDirection::Long => TradingDecisionExit {
            zone_low: upper.max(current_price),
            zone_high: upper.max(current_price) + deviation * 3.0,
            condition: "exit_on_distribution_transition_or_mfe_exhaustion".to_string(),
            timing: DecisionTiming::Wait,
        },
        super::types::AdvisoryDirection::Short => TradingDecisionExit {
            zone_low: (lower.min(current_price) - deviation * 3.0).max(0.0),
            zone_high: lower.min(vwap).max(0.0),
            condition: "exit_on_reaccumulation_transition_or_smp_reversal".to_string(),
            timing: DecisionTiming::Wait,
        },
        super::types::AdvisoryDirection::NoTrade => TradingDecisionExit::default(),
    }
}

fn build_position_size(
    direction: super::types::AdvisoryDirection,
    entry_valid: bool,
    confidence: f64,
    regime_stability: f64,
    liquidity: f64,
    energy: f64,
    gate_multiplier: f64,
    liquidation_pct: f64,
) -> TradingPositionSize {
    if !entry_valid || matches!(direction, super::types::AdvisoryDirection::NoTrade) {
        return TradingPositionSize::default();
    }
    let multiplier = (confidence
        * regime_stability.clamp(0.0, 1.0)
        * liquidity.clamp(0.0, 1.0)
        * (0.55 + energy.clamp(0.0, 1.0) * 0.45)
        * gate_multiplier.clamp(0.0, 1.0)
        * (1.0 - liquidation_pct.clamp(0.0, 0.55) * 0.45))
        .clamp(0.0, 1.0);
    TradingPositionSize {
        pct: multiplier * 100.0,
        multiplier,
        reason: "confidence_x_regime_stability_x_liquidity_x_market_energy_x_pvg".to_string(),
    }
}

fn build_invalidation(
    direction: super::types::AdvisoryDirection,
    current_price: f64,
    lower: f64,
    upper: f64,
    smp: f64,
    lsm: f64,
    regime: StabilityRegime,
    active: bool,
) -> TradingInvalidation {
    let price_level = match direction {
        super::types::AdvisoryDirection::Long => lower
            .min(current_price * (1.0 - 0.012 - lsm.max(0.0) * 0.010))
            .max(0.0),
        super::types::AdvisoryDirection::Short => {
            upper.max(current_price * (1.0 + 0.012 + lsm.max(0.0) * 0.010))
        }
        super::types::AdvisoryDirection::NoTrade => current_price.max(0.0),
    };
    TradingInvalidation {
        active,
        price_level,
        regime_condition: if active {
            "regime_not_aligned_or_stress_unstable".to_string()
        } else {
            "regime_flip_against_direction".to_string()
        },
        flow_condition: if smp.abs() < 0.15 {
            "smp_lacks_directional_pressure".to_string()
        } else {
            "smp_reversal_against_direction".to_string()
        },
        liquidity_condition: if matches!(
            regime,
            StabilityRegime::LiquidityStress | StabilityRegime::Manipulation
        ) {
            "liquidity_stress_or_manipulation".to_string()
        } else {
            "liquidity_collapse_or_vacuum_expansion".to_string()
        },
    }
}

fn build_cost_distribution(item: &TokenWatchItem) -> Vec<CostDistributionBand> {
    let cost = &item.last_signal.capital_structure.cost_basis;
    let width = (cost.upper - cost.lower)
        .abs()
        .max(cost.vwap_anchor.abs() * 0.002);
    vec![
        CostDistributionBand {
            label: "核心成本区".to_string(),
            lower: cost.lower,
            upper: cost.upper,
            pct: 0.62,
        },
        CostDistributionBand {
            label: "早期吸筹区".to_string(),
            lower: (cost.lower - width).max(0.0),
            upper: cost.lower,
            pct: 0.23,
        },
        CostDistributionBand {
            label: "浮动追仓区".to_string(),
            lower: cost.upper,
            upper: cost.upper + width,
            pct: 0.15,
        },
    ]
}

fn build_smart_levels(item: &TokenWatchItem) -> Vec<SmartLevel> {
    let cost = &item.last_signal.capital_structure.cost_basis;
    let mut levels = vec![
        SmartLevel {
            label: "成本下沿".to_string(),
            price: cost.lower,
            role: "support".to_string(),
        },
        SmartLevel {
            label: "VWAP锚点".to_string(),
            price: cost.vwap_anchor,
            role: "anchor".to_string(),
        },
        SmartLevel {
            label: "成本上沿".to_string(),
            price: cost.upper,
            role: "resistance".to_string(),
        },
    ];
    if let Some(node) = &item
        .last_signal
        .position_reconstruction
        .last_accumulation_node
    {
        levels.push(SmartLevel {
            label: "最后吸筹点".to_string(),
            price: (node.lower + node.upper) / 2.0,
            role: "last_accumulation".to_string(),
        });
    }
    levels
}

fn build_chart_markers(item: &TokenWatchItem) -> Vec<TokenChartMarker> {
    let reconstruction = &item.last_signal.position_reconstruction;
    let mut markers = Vec::new();
    if let Some(node) = &reconstruction.last_accumulation_node {
        markers.push(TokenChartMarker {
            ts: item.added_at_ms.max(0) as u64,
            price: (node.lower + node.upper) / 2.0,
            label: "最后吸筹点".to_string(),
            kind: "last_accumulation".to_string(),
        });
    }
    if let Some(segment) = reconstruction.distribution_path.first() {
        markers.push(TokenChartMarker {
            ts: item.added_at_ms.max(0) as u64 + segment.duration_sec.saturating_mul(1000),
            price: segment.end_price,
            label: "出货确认".to_string(),
            kind: "distribution".to_string(),
        });
    }
    markers
}

pub fn normalize_symbol(raw_symbol: &str) -> Result<String, TokenWatchError> {
    let compact = raw_symbol
        .trim()
        .chars()
        .filter(|c| c.is_ascii_alphanumeric())
        .collect::<String>()
        .to_ascii_uppercase();
    if compact.len() < 2 || compact.len() > 24 {
        return Err(TokenWatchError::InvalidSymbol);
    }
    let symbol = if compact.ends_with("USDT") {
        compact
    } else {
        format!("{compact}USDT")
    };
    if symbol.len() > 28 {
        return Err(TokenWatchError::InvalidSymbol);
    }
    Ok(symbol)
}
