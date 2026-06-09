use serde::{Deserialize, Serialize};

use crate::contract_whale_monitor::types::{
    ContractWhaleDirection, ContractWhaleSeverity, ContractWhaleSignal, ContractWhaleSignalType,
};
use crate::runtime::{
    score_config::{
        score_runtime_config, ContractWeights, CrossConfirmWeights,
        MarketStructureConfirmationConfig, MarketStructureRuntimeConfig, SpotWeights,
        ToxicShortDiscordConfig, ToxicShortWeights,
    },
    tof_metrics::{TofDirection, TofMetrics},
};

const MAIN_FORCE_CWM_WEIGHT: f64 = 0.25;

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CwmRiskContribution {
    pub available: bool,
    pub source: String,
    pub formula: String,
    pub contribution_weight: f64,
    pub score: Option<u8>,
    pub weighted_contribution: f64,
    pub signal_id: Option<String>,
    pub severity: Option<ContractWhaleSeverity>,
    pub signal_type: Option<ContractWhaleSignalType>,
    pub direction: Option<ContractWhaleDirection>,
    pub window_sec: Option<u64>,
    pub data_quality: Option<u8>,
    pub dominance: Option<f64>,
    pub main_exchange: Option<String>,
    pub exchange_count: Option<usize>,
    pub price_move_pct: Option<f64>,
    pub multi_exchange_confirmed: Option<bool>,
    pub liquidation_suspected: Option<bool>,
    pub liquidation_long_btc: Option<f64>,
    pub liquidation_short_btc: Option<f64>,
    pub liquidation_ratio: Option<f64>,
    pub oi_change_pct: Option<f64>,
    pub oi_bias: Option<String>,
    pub funding_rate: Option<f64>,
    pub funding_bias: Option<String>,
    pub summary: String,
    pub discord_gate_independent: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SplitRiskSystems {
    pub short_term_toxic: ShortTermToxicRisk,
    pub market_structure_score: MainForceStructureRisk,
    pub main_force_structure: MainForceStructureRisk,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ShortTermToxicRisk {
    pub ts: i64,
    pub symbol: String,
    pub toxic_score: u8,
    pub short_pressure: i16,
    pub confidence: f64,
    pub data_quality: f64,
    pub severity: String,
    pub toxic_type: String,
    pub ttl_sec: u64,
    pub expires_at: i64,
    pub half_life_sec: u64,
    pub max_ttl_sec: u64,
    pub decayed_score: f64,
    pub decay_formula: String,
    pub reasons: Vec<ToxicReason>,
    pub timeframes: Vec<String>,
    pub formula: String,
    pub discord_gate: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ToxicReason {
    pub reason_type: String,
    pub score: f64,
    pub weight: f64,
    pub window_sec: u64,
    pub direction: String,
    pub description: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct MainForceStructureRisk {
    pub ts: i64,
    pub symbol: String,
    pub main_force_score: u8,
    pub main_force_confirmed: bool,
    pub main_force_confirmation_count: u8,
    pub main_force_confirmation_total: u8,
    pub main_force_confirmation_threshold: u8,
    pub extreme_impact_score: u8,
    pub extreme_impact_confirmed: bool,
    pub structure_bias: i16,
    pub confidence: f64,
    pub data_quality: f64,
    pub severity: String,
    pub regime_type: String,
    pub structure_raw: f64,
    pub spot_contract_floor: u8,
    pub duration_score: u8,
    pub liquidation_penalty: f64,
    pub crowding_penalty: f64,
    pub spot_score: u8,
    pub spot_cvd_score: u8,
    pub spot_volume_anomaly: u8,
    pub spot_absorption: u8,
    pub spot_liquidity_shift: u8,
    pub spot_price_response: u8,
    pub contract_score: u8,
    pub cwm_aggressive_flow: u8,
    pub oi_impulse: u8,
    pub liquidation_context: u8,
    pub funding_crowding: u8,
    pub basis_premium: u8,
    pub active_exchange_confirmation: u8,
    pub cross_confirm_score: u8,
    pub spot_contract_direction_consistency: u8,
    pub multi_window_consistency: u8,
    pub price_response_consistency: u8,
    pub source_coverage: u8,
    pub signal_agreement: u8,
    pub oi_score: u8,
    pub liquidation_score: u8,
    pub funding_crowding_score: u8,
    pub cwm_score: u8,
    pub reasons: Vec<MarketStructureReason>,
    pub timeframes: Vec<String>,
    pub formula: String,
    pub cwm_contribution: CwmRiskContribution,
    pub discord_gate_independent: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct MarketStructureReason {
    pub reason_type: String,
    pub score: f64,
    pub weight: f64,
    pub timeframe: String,
    pub direction: String,
    pub description: String,
}

#[derive(Debug, Clone, Copy)]
struct MarketStructureComponents {
    spot_score: u8,
    spot_cvd_score: u8,
    spot_volume_anomaly: u8,
    spot_absorption: u8,
    spot_liquidity_shift: u8,
    spot_price_response: u8,
    contract_score: u8,
    cwm_aggressive_flow: u8,
    oi_impulse: u8,
    liquidation_context: u8,
    funding_crowding: u8,
    basis_premium: u8,
    active_exchange_confirmation: u8,
    cross_confirm_score: u8,
    spot_contract_direction_consistency: u8,
    multi_window_consistency: u8,
    price_response_consistency: u8,
    source_coverage: u8,
    signal_agreement: u8,
    spot_contract_floor: u8,
    duration_score: u8,
    liquidation_penalty: f64,
    crowding_penalty: f64,
    oi_score: u8,
    liquidation_score: u8,
    funding_crowding_score: u8,
    cwm_score: u8,
}

#[derive(Debug, Clone, Copy)]
struct SpotBehaviorComponents {
    spot_cvd_score: u8,
    spot_volume_anomaly: u8,
    spot_absorption: u8,
    spot_liquidity_shift: u8,
    spot_price_response: u8,
}

#[derive(Debug, Clone, Copy)]
struct ContractBehaviorComponents {
    cwm_aggressive_flow: u8,
    oi_impulse: u8,
    liquidation_context: u8,
    funding_crowding: u8,
    basis_premium: u8,
    active_exchange_confirmation: u8,
}

#[derive(Debug, Clone, Copy)]
struct CrossConfirmComponents {
    spot_contract_direction_consistency: u8,
    multi_window_consistency: u8,
    price_response_consistency: u8,
    source_coverage: u8,
}

#[derive(Debug, Clone, Copy)]
struct MainForceConfirmationGate {
    confirmed: bool,
    count: u8,
    total: u8,
    threshold: u8,
}

#[derive(Debug, Clone)]
pub struct SplitRiskSystemsInput<'a> {
    pub ts_ms: i64,
    pub symbol: &'a str,
    pub short_toxic_score: u8,
    pub short_tof_score: f64,
    pub short_direction: TofDirection,
    pub toxic_type: &'a str,
    pub data_quality: f64,
    pub tof_metrics: &'a TofMetrics,
    pub advanced_score: u8,
    pub perp_score: u8,
    pub metrics_direction: TofDirection,
    pub cwm_contribution: CwmRiskContribution,
}

impl CwmRiskContribution {
    pub fn unavailable(symbol: &str) -> Self {
        Self {
            available: false,
            source: "contract_whale_monitor".to_string(),
            formula: main_force_formula_label(),
            contribution_weight: MAIN_FORCE_CWM_WEIGHT,
            score: None,
            weighted_contribution: 0.0,
            signal_id: None,
            severity: None,
            signal_type: None,
            direction: None,
            window_sec: None,
            data_quality: None,
            dominance: None,
            main_exchange: None,
            exchange_count: None,
            price_move_pct: None,
            multi_exchange_confirmed: None,
            liquidation_suspected: None,
            liquidation_long_btc: None,
            liquidation_short_btc: None,
            liquidation_ratio: None,
            oi_change_pct: None,
            oi_bias: None,
            funding_rate: None,
            funding_bias: None,
            summary: format!("No recent CWM signal for {symbol}; main-force structure uses spot/perp context only."),
            discord_gate_independent: true,
        }
    }
}

pub fn build_cwm_risk_contribution(
    symbol: &str,
    signal: Option<&ContractWhaleSignal>,
) -> CwmRiskContribution {
    let Some(signal) = signal else {
        return CwmRiskContribution::unavailable(symbol);
    };
    CwmRiskContribution {
        available: true,
        source: "contract_whale_monitor".to_string(),
        formula: main_force_formula_label(),
        contribution_weight: MAIN_FORCE_CWM_WEIGHT,
        score: Some(signal.score),
        weighted_contribution: round2(signal.score as f64 * MAIN_FORCE_CWM_WEIGHT),
        signal_id: Some(signal.id.clone()),
        severity: Some(signal.severity),
        signal_type: Some(signal.signal_type),
        direction: Some(signal.direction),
        window_sec: Some(signal.window_sec),
        data_quality: Some(signal.data_quality),
        dominance: Some(round4(signal.dominance)),
        main_exchange: signal.main_exchange.clone(),
        exchange_count: Some(signal.exchanges.len()),
        price_move_pct: signal.price_move_pct.map(round4),
        multi_exchange_confirmed: Some(signal.multi_exchange_confirmed),
        liquidation_suspected: Some(signal.liquidation_suspected),
        liquidation_long_btc: Some(round4(signal.liquidation_long_btc)),
        liquidation_short_btc: Some(round4(signal.liquidation_short_btc)),
        liquidation_ratio: signal.liquidation_ratio.map(round4),
        oi_change_pct: signal.oi_change_pct.map(round4),
        oi_bias: signal.oi_bias.clone(),
        funding_rate: signal.funding_rate.map(round4),
        funding_bias: signal.funding_bias.clone(),
        summary: signal.final_result.clone(),
        discord_gate_independent: true,
    }
}

pub fn build_split_risk_systems(input: SplitRiskSystemsInput<'_>) -> SplitRiskSystems {
    let score_config = score_runtime_config();
    let toxic_config = &score_config.toxic_short;
    let market_structure_config = &score_config.market_structure;
    let ttl_sec = ttl_for_toxic_score();
    let half_life_sec = half_life_for_toxic_score();
    let short_term_toxic = ShortTermToxicRisk {
        ts: input.ts_ms,
        symbol: input.symbol.to_string(),
        toxic_score: input.short_toxic_score,
        short_pressure: pressure_from_direction(input.short_direction, input.short_toxic_score),
        confidence: round2((input.short_tof_score + input.data_quality) / 2.0),
        data_quality: round2(input.data_quality),
        severity: toxic_short_severity(input.short_toxic_score).to_string(),
        toxic_type: canonical_toxic_type(input.toxic_type, input.short_direction).to_string(),
        ttl_sec,
        expires_at: input
            .ts_ms
            .saturating_add((ttl_sec as i64).saturating_mul(1000)),
        half_life_sec,
        max_ttl_sec: toxic_config.max_ttl_sec,
        decayed_score: decayed_toxic_score(input.short_toxic_score, 0.0, half_life_sec),
        decay_formula: "decayedScore = previousScore * exp(-elapsedSec / halfLifeSec)".to_string(),
        reasons: build_toxic_reasons(&input, &toxic_config.weights),
        timeframes: toxic_short_timeframes(&toxic_config.windows_sec),
        formula: toxic_short_formula(&toxic_config.weights),
        discord_gate: toxic_short_discord_gate(&toxic_config.discord),
    };
    let spot_components = spot_behavior_components(&input);
    let spot_score = spot_behavior_score(&spot_components, &market_structure_config.spot_weights);
    let contract_components = contract_behavior_components(&input);
    let cwm_score = contract_components.cwm_aggressive_flow;
    let oi_score = contract_components.oi_impulse;
    let liquidation_score = contract_components.liquidation_context;
    let funding_crowding_score = contract_components.funding_crowding;
    let contract_score = contract_behavior_score(
        &contract_components,
        &market_structure_config.contract_weights,
    );
    let cross_components = cross_confirm_components(&input);
    let cross_confirm_score = cross_confirm_score(
        &cross_components,
        &market_structure_config.cross_confirm_weights,
    );
    let duration_score = duration_score(&input.cwm_contribution, cross_confirm_score);
    let liquidation_penalty = liquidation_penalty(&input.cwm_contribution);
    let crowding_penalty = crowding_penalty(funding_crowding_score, cross_confirm_score);
    let signal_agreement = signal_agreement(&cross_components, &contract_components, &input);
    let components = MarketStructureComponents {
        spot_score,
        spot_cvd_score: spot_components.spot_cvd_score,
        spot_volume_anomaly: spot_components.spot_volume_anomaly,
        spot_absorption: spot_components.spot_absorption,
        spot_liquidity_shift: spot_components.spot_liquidity_shift,
        spot_price_response: spot_components.spot_price_response,
        contract_score,
        cwm_aggressive_flow: contract_components.cwm_aggressive_flow,
        oi_impulse: contract_components.oi_impulse,
        liquidation_context: contract_components.liquidation_context,
        funding_crowding: contract_components.funding_crowding,
        basis_premium: contract_components.basis_premium,
        active_exchange_confirmation: contract_components.active_exchange_confirmation,
        cross_confirm_score,
        spot_contract_direction_consistency: cross_components.spot_contract_direction_consistency,
        multi_window_consistency: cross_components.multi_window_consistency,
        price_response_consistency: cross_components.price_response_consistency,
        source_coverage: cross_components.source_coverage,
        signal_agreement,
        spot_contract_floor: spot_score.min(contract_score),
        duration_score,
        liquidation_penalty,
        crowding_penalty,
        oi_score,
        liquidation_score,
        funding_crowding_score,
        cwm_score,
    };
    let structure_weights = &market_structure_config.structure_weights;
    let structure_raw = round2(
        structure_weights.spot_score * components.spot_score as f64
            + structure_weights.contract_score * components.contract_score as f64
            + structure_weights.cross_confirm_score * components.cross_confirm_score as f64,
    );
    let main_force_weights = &market_structure_config.main_force_weights;
    let base_main_force_score = clamp_score(
        main_force_weights.structure_raw * structure_raw
            + main_force_weights.spot_contract_min * components.spot_contract_floor as f64
            + main_force_weights.duration_score * components.duration_score as f64
            - components.liquidation_penalty
            - components.crowding_penalty,
    )
    .round() as u8;
    let contract_flow_shock = is_contract_flow_shock(&components, &input.cwm_contribution);
    let liquidation_driven = liquidation_is_primary_driver(&components, &input.cwm_contribution);
    let main_force_score = capped_main_force_score(
        base_main_force_score,
        &components,
        &input.cwm_contribution,
        contract_flow_shock,
        liquidation_driven,
    );
    let main_force_data_quality = market_structure_data_quality(&input, &components);
    let main_force_confidence = market_structure_confidence(main_force_data_quality, &components);
    let main_force_confirmation = main_force_confirmation_gate(
        &components,
        &input.cwm_contribution,
        main_force_score,
        main_force_confidence,
        main_force_data_quality,
        &market_structure_config.confirmation,
    );
    let extreme_impact_score = extreme_impact_score(
        input.advanced_score,
        input.perp_score,
        input.cwm_contribution.score,
        &components,
        &input.cwm_contribution,
    );
    let extreme_impact_confirmed = extreme_impact_score >= 80;
    let regime_type = regime_type(
        &components,
        &input.cwm_contribution,
        input.short_direction,
        input.metrics_direction,
        contract_flow_shock,
    )
    .to_string();
    let reasons = build_market_structure_reasons(
        &components,
        input.metrics_direction,
        &regime_type,
        main_force_confirmation,
        market_structure_config,
    );
    let main_force_structure = MainForceStructureRisk {
        ts: input.ts_ms,
        symbol: input.symbol.to_string(),
        main_force_score,
        main_force_confirmed: main_force_confirmation.confirmed,
        main_force_confirmation_count: main_force_confirmation.count,
        main_force_confirmation_total: main_force_confirmation.total,
        main_force_confirmation_threshold: main_force_confirmation.threshold,
        structure_bias: structure_bias(&input, &components),
        extreme_impact_score,
        extreme_impact_confirmed,
        data_quality: main_force_data_quality,
        confidence: main_force_confidence,
        severity: market_structure_severity(main_force_score, extreme_impact_score).to_string(),
        regime_type,
        structure_raw,
        spot_contract_floor: components.spot_contract_floor,
        duration_score: components.duration_score,
        liquidation_penalty: components.liquidation_penalty,
        crowding_penalty: components.crowding_penalty,
        spot_score: components.spot_score,
        spot_cvd_score: components.spot_cvd_score,
        spot_volume_anomaly: components.spot_volume_anomaly,
        spot_absorption: components.spot_absorption,
        spot_liquidity_shift: components.spot_liquidity_shift,
        spot_price_response: components.spot_price_response,
        contract_score: components.contract_score,
        cwm_aggressive_flow: components.cwm_aggressive_flow,
        oi_impulse: components.oi_impulse,
        liquidation_context: components.liquidation_context,
        funding_crowding: components.funding_crowding,
        basis_premium: components.basis_premium,
        active_exchange_confirmation: components.active_exchange_confirmation,
        cross_confirm_score: components.cross_confirm_score,
        spot_contract_direction_consistency: components.spot_contract_direction_consistency,
        multi_window_consistency: components.multi_window_consistency,
        price_response_consistency: components.price_response_consistency,
        source_coverage: components.source_coverage,
        signal_agreement: components.signal_agreement,
        oi_score: components.oi_score,
        liquidation_score: components.liquidation_score,
        funding_crowding_score: components.funding_crowding_score,
        cwm_score: components.cwm_score,
        reasons,
        timeframes: market_structure_timeframes(&market_structure_config.windows_min),
        formula: main_force_formula_label(),
        cwm_contribution: input.cwm_contribution,
        discord_gate_independent: true,
    };
    SplitRiskSystems {
        short_term_toxic,
        market_structure_score: main_force_structure.clone(),
        main_force_structure,
    }
}

fn main_force_formula_label() -> String {
    let config = score_runtime_config();
    let spot = &config.market_structure.spot_weights;
    let contract = &config.market_structure.contract_weights;
    let cross = &config.market_structure.cross_confirm_weights;
    let structure = &config.market_structure.structure_weights;
    let main_force = &config.market_structure.main_force_weights;
    format!(
        "MarketStructureScore: spotScore = {}*SpotCvdScore + {}*SpotVolumeAnomaly + {}*SpotAbsorption + {}*SpotLiquidityShift + {}*SpotPriceResponse; contractScore = {}*CwmAggressiveFlow + {}*OiImpulse + {}*LiquidationContext + {}*FundingCrowding + {}*BasisPremium + {}*ActiveExchangeConfirmation; crossConfirmScore = {}*SpotContractDirectionConsistency + {}*MultiWindowConsistency + {}*PriceResponseConsistency + {}*SourceCoverage; structureRaw = {}*spotScore + {}*contractScore + {}*crossConfirmScore; mainForceScore = {}*structureRaw + {}*min(spotScore, contractScore) + {}*durationScore - liquidationPenalty - crowdingPenalty; independent from toxicScore",
        fmt_weight(spot.spot_cvd),
        fmt_weight(spot.spot_volume_anomaly),
        fmt_weight(spot.spot_absorption),
        fmt_weight(spot.spot_liquidity_shift),
        fmt_weight(spot.spot_price_response),
        fmt_weight(contract.cwm_aggressive_flow),
        fmt_weight(contract.oi_impulse),
        fmt_weight(contract.liquidation_context),
        fmt_weight(contract.funding_crowding),
        fmt_weight(contract.basis_premium),
        fmt_weight(contract.active_exchange_confirmation),
        fmt_weight(cross.spot_contract_direction_consistency),
        fmt_weight(cross.multi_window_consistency),
        fmt_weight(cross.price_response_consistency),
        fmt_weight(cross.source_coverage),
        fmt_weight(structure.spot_score),
        fmt_weight(structure.contract_score),
        fmt_weight(structure.cross_confirm_score),
        fmt_weight(main_force.structure_raw),
        fmt_weight(main_force.spot_contract_min),
        fmt_weight(main_force.duration_score)
    )
}

fn ttl_for_toxic_score() -> u64 {
    score_runtime_config().toxic_short.max_ttl_sec
}

fn half_life_for_toxic_score() -> u64 {
    score_runtime_config().toxic_short.half_life_sec
}

fn toxic_short_severity(score: u8) -> &'static str {
    match score {
        90..=100 => "S",
        75..=89 => "Critical",
        60..=74 => "High",
        40..=59 => "Watch",
        _ => "Calm",
    }
}

pub fn decayed_toxic_score(previous_score: u8, elapsed_sec: f64, half_life_sec: u64) -> f64 {
    if !elapsed_sec.is_finite() || elapsed_sec <= 0.0 {
        return previous_score as f64;
    }
    if half_life_sec == 0 {
        return 0.0;
    }
    round2(previous_score as f64 * (-elapsed_sec / half_life_sec as f64).exp())
}

fn canonical_toxic_type(candidate_type: &str, direction: TofDirection) -> &'static str {
    let value = candidate_type.to_ascii_lowercase();
    if value.contains("spoof") {
        "spoofing"
    } else if value.contains("liquidity_pull") || value.contains("pull") {
        "liquidity_pull"
    } else if value.contains("thin") || value.contains("gap") {
        "micro_liquidity_gap"
    } else if value.contains("stop") || value.contains("trap") {
        "stop_hunt"
    } else if value.contains("breakout") {
        "fake_breakout"
    } else if value.contains("adverse") {
        "adverse_selection"
    } else {
        match direction {
            TofDirection::Bullish => "toxic_buy_sweep",
            TofDirection::Bearish => "toxic_sell_sweep",
            TofDirection::Mixed | TofDirection::Neutral => "adverse_selection",
        }
    }
}

fn pressure_from_direction(direction: TofDirection, score: u8) -> i16 {
    let value = score.min(100) as i16;
    match direction {
        TofDirection::Bullish => value,
        TofDirection::Bearish => -value,
        TofDirection::Mixed => 0,
        TofDirection::Neutral => 0,
    }
}

fn structure_bias(
    input: &SplitRiskSystemsInput<'_>,
    components: &MarketStructureComponents,
) -> i16 {
    let spot_direction = market_direction_score(input.metrics_direction, components.spot_score);
    let contract_direction =
        contract_direction_score(input.cwm_contribution.direction, components.contract_score);
    let oi_direction = oi_direction_score(
        input.cwm_contribution.direction,
        input.cwm_contribution.oi_change_pct,
        components.oi_impulse,
    );
    let price_response_direction = price_response_direction_score(
        &input.cwm_contribution,
        components.price_response_consistency,
    );
    let liquidation_direction =
        liquidation_direction_score(&input.cwm_contribution, components.liquidation_context);
    let weighted = 0.30 * spot_direction
        + 0.30 * contract_direction
        + 0.15 * oi_direction
        + 0.15 * price_response_direction
        + 0.10 * liquidation_direction;
    if is_downside_absorption(components, &input.cwm_contribution) {
        return clamp_signed_score(weighted.max(12.0));
    }
    if is_upside_resistance(components, &input.cwm_contribution) {
        return clamp_signed_score(weighted.min(-12.0));
    }
    clamp_signed_score(weighted)
}

fn market_direction_score(direction: TofDirection, score: u8) -> f64 {
    signed_direction_value(direction, score)
}

fn contract_direction_score(direction: Option<ContractWhaleDirection>, score: u8) -> f64 {
    match direction {
        Some(ContractWhaleDirection::Buy) => score as f64,
        Some(ContractWhaleDirection::Sell) => -(score as f64),
        Some(ContractWhaleDirection::Absorption) => score as f64 * 0.20,
        Some(ContractWhaleDirection::Suppression) => -(score as f64 * 0.20),
        None => 0.0,
    }
}

fn oi_direction_score(
    direction: Option<ContractWhaleDirection>,
    oi_change_pct: Option<f64>,
    oi_impulse: u8,
) -> f64 {
    let Some(change_pct) = oi_change_pct else {
        return 0.0;
    };
    match (change_pct > 0.0, direction) {
        (true, Some(ContractWhaleDirection::Buy)) => oi_impulse as f64,
        (true, Some(ContractWhaleDirection::Sell)) => -(oi_impulse as f64),
        (true, Some(ContractWhaleDirection::Absorption)) => oi_impulse as f64 * 0.15,
        (true, Some(ContractWhaleDirection::Suppression)) => -(oi_impulse as f64 * 0.15),
        (false, Some(ContractWhaleDirection::Buy)) => oi_impulse as f64 * 0.25,
        (false, Some(ContractWhaleDirection::Sell)) => -(oi_impulse as f64 * 0.25),
        _ => 0.0,
    }
}

fn price_response_direction_score(
    contribution: &CwmRiskContribution,
    price_response_consistency: u8,
) -> f64 {
    let price_move_pct = contribution.price_move_pct.unwrap_or(0.0);
    if price_move_pct > 0.05 {
        return price_response_consistency as f64;
    }
    if price_move_pct < -0.05 {
        return -(price_response_consistency as f64);
    }
    match contribution.direction {
        Some(ContractWhaleDirection::Absorption) => price_response_consistency as f64 * 0.20,
        Some(ContractWhaleDirection::Suppression) => -(price_response_consistency as f64 * 0.20),
        _ => 0.0,
    }
}

fn liquidation_direction_score(contribution: &CwmRiskContribution, liquidation_context: u8) -> f64 {
    if short_liquidation_dominant(contribution) {
        liquidation_context as f64 * 0.35
    } else if long_liquidation_dominant(contribution) {
        -(liquidation_context as f64 * 0.35)
    } else {
        0.0
    }
}

fn signed_direction_value(direction: TofDirection, score: u8) -> f64 {
    match direction {
        TofDirection::Bullish => score as f64,
        TofDirection::Bearish => -(score as f64),
        TofDirection::Mixed | TofDirection::Neutral => 0.0,
    }
}

fn clamp_signed_score(value: f64) -> i16 {
    if value.is_finite() {
        value.clamp(-100.0, 100.0).round() as i16
    } else {
        0
    }
}

fn build_toxic_reasons(
    input: &SplitRiskSystemsInput<'_>,
    weights: &ToxicShortWeights,
) -> Vec<ToxicReason> {
    let direction = direction_key(input.short_direction).to_string();
    let metrics = input.tof_metrics;
    let toxic_order_cluster =
        clamp_score(0.55 * input.short_toxic_score as f64 + 0.45 * input.short_tof_score);
    let aggressive_sweep = clamp_score(
        (metrics.trade_imbalance.abs() * 100.0)
            .max(metrics.trade_imbalance_score)
            .max(input.short_tof_score * 0.35),
    );
    let orderbook_deformation = clamp_score(
        metrics
            .depth_withdrawal_score
            .max(
                metrics
                    .bid_depth_withdrawal
                    .max(metrics.ask_depth_withdrawal),
            )
            .max(metrics.spread_widening_score * 0.65),
    );
    let spoof_cancel = clamp_score(metrics.order_churn_score.max(
        orderbook_deformation
            * if input.toxic_type.to_ascii_lowercase().contains("spoof") {
                0.95
            } else {
                0.65
            },
    ));
    let adverse_move = clamp_score(
        input.short_toxic_score as f64 * 0.45
            + metrics.metrics_confidence * 0.35
            + metrics.vpin_proxy * 0.20,
    );
    let liquidity_gap = clamp_score(
        metrics
            .liquidity_vacuum_score
            .max(metrics.spread_widening_score),
    );
    let micro_volatility_shock = clamp_score(
        metrics.spread_widening_score * 0.55
            + metrics.trade_rate.min(100.0) * 0.25
            + metrics.book_update_rate.min(100.0) * 0.20,
    );
    vec![
        toxic_reason(
            "ToxicOrderCluster",
            toxic_order_cluster,
            weights.toxic_order_cluster,
            5,
            &direction,
            "1s/5s/15s abnormal order concentration and direction clustering",
        ),
        toxic_reason(
            "AggressiveSweep",
            aggressive_sweep,
            weights.aggressive_sweep,
            5,
            &direction,
            "aggressive trades sweeping nearby book depth",
        ),
        toxic_reason(
            "OrderbookDeformation",
            orderbook_deformation,
            weights.orderbook_deformation,
            15,
            &direction,
            "depth withdrawal, spread widening, and book imbalance deformation",
        ),
        toxic_reason(
            "SpoofCancel",
            spoof_cancel,
            weights.spoof_cancel,
            15,
            &direction,
            "fake wall, cancel ratio, wall move frequency, and near-touch cancel count",
        ),
        toxic_reason(
            "AdverseMove",
            adverse_move,
            weights.adverse_move,
            60,
            &direction,
            "price moves against the signal soon after it appears, adding adverse-selection risk",
        ),
        toxic_reason(
            "LiquidityGap",
            liquidity_gap,
            weights.liquidity_gap,
            15,
            &direction,
            "nearby 0.1%/0.2%/0.5% depth vacuum and removed resting liquidity",
        ),
        toxic_reason(
            "MicroVolatilityShock",
            micro_volatility_shock,
            weights.micro_volatility_shock,
            1,
            &direction,
            "1s micro volatility and update-rate shock",
        ),
    ]
}

fn toxic_reason(
    reason_type: &str,
    score: f64,
    weight: f64,
    window_sec: u64,
    direction: &str,
    description: &str,
) -> ToxicReason {
    ToxicReason {
        reason_type: reason_type.to_string(),
        score: round2(score),
        weight,
        window_sec,
        direction: direction.to_string(),
        description: description.to_string(),
    }
}

fn direction_key(direction: TofDirection) -> &'static str {
    match direction {
        TofDirection::Bullish => "bullish",
        TofDirection::Bearish => "bearish",
        TofDirection::Mixed => "mixed",
        TofDirection::Neutral => "neutral",
    }
}

fn cross_confirm_components(input: &SplitRiskSystemsInput<'_>) -> CrossConfirmComponents {
    CrossConfirmComponents {
        spot_contract_direction_consistency: spot_contract_direction_consistency(input),
        multi_window_consistency: multi_window_consistency(input),
        price_response_consistency: price_response_consistency(input),
        source_coverage: source_coverage(input),
    }
}

fn cross_confirm_score(components: &CrossConfirmComponents, weights: &CrossConfirmWeights) -> u8 {
    clamp_score(
        weights.spot_contract_direction_consistency
            * components.spot_contract_direction_consistency as f64
            + weights.multi_window_consistency * components.multi_window_consistency as f64
            + weights.price_response_consistency * components.price_response_consistency as f64
            + weights.source_coverage * components.source_coverage as f64,
    )
    .round() as u8
}

fn signal_agreement(
    cross_components: &CrossConfirmComponents,
    contract_components: &ContractBehaviorComponents,
    input: &SplitRiskSystemsInput<'_>,
) -> u8 {
    let oi_alignment = if oi_direction_consistent_from_raw(
        &input.cwm_contribution,
        contract_components.oi_impulse,
    ) {
        92.0
    } else if input.cwm_contribution.oi_change_pct.is_some() {
        46.0
    } else {
        60.0
    };
    let liquidation_penalty = if input.cwm_contribution.liquidation_suspected == Some(true) {
        8.0
    } else {
        0.0
    };
    clamp_score(
        0.45 * cross_components.spot_contract_direction_consistency as f64
            + 0.35 * cross_components.price_response_consistency as f64
            + 0.20 * oi_alignment
            - liquidation_penalty,
    )
    .round() as u8
}

fn spot_contract_direction_consistency(input: &SplitRiskSystemsInput<'_>) -> u8 {
    if let Some(signal_type) = input.cwm_contribution.signal_type {
        let cwm_direction = signal_type_direction(signal_type);
        let mut score = if directions_align(cwm_direction, input.metrics_direction)
            && directions_align(cwm_direction, input.short_direction)
        {
            96.0
        } else if directions_align(cwm_direction, input.metrics_direction)
            || directions_align(cwm_direction, input.short_direction)
        {
            90.0
        } else if matches!(
            input.metrics_direction,
            TofDirection::Mixed | TofDirection::Neutral
        ) || matches!(
            input.short_direction,
            TofDirection::Mixed | TofDirection::Neutral
        ) {
            58.0
        } else {
            38.0
        };
        if input
            .cwm_contribution
            .oi_change_pct
            .is_some_and(|change_pct| change_pct > 0.0)
        {
            score += 4.0;
        }
        return clamp_score(score).round() as u8;
    }
    if directions_align(input.short_direction, input.metrics_direction) {
        78
    } else if matches!(
        input.short_direction,
        TofDirection::Mixed | TofDirection::Neutral
    ) || matches!(
        input.metrics_direction,
        TofDirection::Mixed | TofDirection::Neutral
    ) {
        50
    } else {
        35
    }
}

fn multi_window_consistency(input: &SplitRiskSystemsInput<'_>) -> u8 {
    let mut score = if input.cwm_contribution.available {
        match input.cwm_contribution.window_sec.unwrap_or(0) {
            60.. => 84.0,
            15..=59 => 74.0,
            5..=14 => 62.0,
            1..=4 => 48.0,
            _ => 45.0,
        }
    } else {
        45.0
    };
    score += match input.cwm_contribution.severity {
        Some(ContractWhaleSeverity::S) => 12.0,
        Some(ContractWhaleSeverity::Critical) => 9.0,
        Some(ContractWhaleSeverity::High) => 6.0,
        Some(ContractWhaleSeverity::Medium) => 2.0,
        _ => 0.0,
    };
    if input.advanced_score >= 80 && input.perp_score >= 80 {
        score += 6.0;
    }
    if input.cwm_contribution.oi_change_pct.is_some() {
        score += 4.0;
    }
    clamp_score(score).round() as u8
}

fn price_response_consistency(input: &SplitRiskSystemsInput<'_>) -> u8 {
    let Some(signal_type) = input.cwm_contribution.signal_type else {
        return spot_price_response(input);
    };
    let cwm_direction = signal_type_direction(signal_type);
    let price_move_pct = input.cwm_contribution.price_move_pct.unwrap_or(0.0);
    match signal_type {
        ContractWhaleSignalType::AggressiveBuy | ContractWhaleSignalType::AggressiveSell => {
            let same_direction = (matches!(cwm_direction, TofDirection::Bullish)
                && price_move_pct > 0.0)
                || (matches!(cwm_direction, TofDirection::Bearish) && price_move_pct < 0.0);
            if price_move_pct.abs() <= 0.05 {
                58
            } else if same_direction {
                clamp_score(78.0 + price_move_pct.abs().min(0.5) * 40.0).round() as u8
            } else {
                42
            }
        }
        ContractWhaleSignalType::DownsideAbsorption
        | ContractWhaleSignalType::UpsideSuppression => {
            if price_move_pct.abs() <= 0.05 {
                88
            } else {
                64
            }
        }
    }
}

fn source_coverage(input: &SplitRiskSystemsInput<'_>) -> u8 {
    let config = crate::contract_whale_monitor::config::contract_whale_runtime_config();
    let enabled_sources = config.active_exchange_count().max(1);
    let healthy_sources = active_source_count(&input.cwm_contribution);
    let coverage = healthy_sources.min(enabled_sources) as f64 / enabled_sources as f64;
    clamp_score(coverage * 100.0).round() as u8
}

fn signal_type_direction(signal_type: ContractWhaleSignalType) -> TofDirection {
    match signal_type {
        ContractWhaleSignalType::AggressiveBuy | ContractWhaleSignalType::DownsideAbsorption => {
            TofDirection::Bullish
        }
        ContractWhaleSignalType::AggressiveSell | ContractWhaleSignalType::UpsideSuppression => {
            TofDirection::Bearish
        }
    }
}

fn directions_align(left: TofDirection, right: TofDirection) -> bool {
    matches!(
        (left, right),
        (TofDirection::Bullish, TofDirection::Bullish)
            | (TofDirection::Bearish, TofDirection::Bearish)
    )
}

fn spot_behavior_components(input: &SplitRiskSystemsInput<'_>) -> SpotBehaviorComponents {
    SpotBehaviorComponents {
        spot_cvd_score: spot_cvd_score(input),
        spot_volume_anomaly: spot_volume_anomaly(input.tof_metrics),
        spot_absorption: spot_absorption(input),
        spot_liquidity_shift: spot_liquidity_shift(input.tof_metrics),
        spot_price_response: spot_price_response(input),
    }
}

fn spot_behavior_score(components: &SpotBehaviorComponents, weights: &SpotWeights) -> u8 {
    clamp_score(
        weights.spot_cvd * components.spot_cvd_score as f64
            + weights.spot_volume_anomaly * components.spot_volume_anomaly as f64
            + weights.spot_absorption * components.spot_absorption as f64
            + weights.spot_liquidity_shift * components.spot_liquidity_shift as f64
            + weights.spot_price_response * components.spot_price_response as f64,
    )
    .round() as u8
}

fn spot_cvd_score(input: &SplitRiskSystemsInput<'_>) -> u8 {
    let metrics = input.tof_metrics;
    let net_flow_strength =
        (metrics.trade_imbalance.abs() * 100.0).max(metrics.trade_imbalance_score);
    let direction_persistence =
        if directions_align(metrics.metrics_direction, input.short_direction) {
            8.0
        } else if matches!(
            metrics.metrics_direction,
            TofDirection::Mixed | TofDirection::Neutral
        ) || matches!(
            input.short_direction,
            TofDirection::Mixed | TofDirection::Neutral
        ) {
            0.0
        } else {
            -8.0
        };
    clamp_score(
        0.72 * net_flow_strength
            + 0.18 * metrics.metrics_confidence
            + 0.10 * input.short_tof_score
            + direction_persistence,
    )
    .round() as u8
}

fn spot_volume_anomaly(metrics: &TofMetrics) -> u8 {
    let bucket_intensity = (metrics.vpin_bucket_count as f64 * 12.0).min(100.0);
    let window_volume_intensity = (metrics.vpin_window_volume / 15_000.0 * 100.0).min(100.0);
    clamp_score(
        0.45 * metrics.trade_rate.min(100.0)
            + 0.25 * bucket_intensity
            + 0.20 * window_volume_intensity
            + 0.10 * metrics.vpin_proxy,
    )
    .round() as u8
}

fn spot_absorption(input: &SplitRiskSystemsInput<'_>) -> u8 {
    let metrics = input.tof_metrics;
    let sell_absorption = if metrics.trade_imbalance < -0.10 {
        0.45 * (100.0 - metrics.bid_depth_withdrawal)
            + 0.25 * metrics.ask_depth_withdrawal
            + 0.30 * metrics.liquidity_vacuum_score
    } else {
        0.0
    };
    let buy_suppression = if metrics.trade_imbalance > 0.10 {
        0.45 * (100.0 - metrics.ask_depth_withdrawal)
            + 0.25 * metrics.bid_depth_withdrawal
            + 0.30 * metrics.liquidity_vacuum_score
    } else {
        0.0
    };
    let response_conflict = if directions_align(input.short_direction, input.metrics_direction)
        || matches!(
            input.metrics_direction,
            TofDirection::Mixed | TofDirection::Neutral
        )
        || matches!(
            input.short_direction,
            TofDirection::Mixed | TofDirection::Neutral
        ) {
        0.0
    } else {
        18.0
    };
    clamp_score(sell_absorption.max(buy_suppression) + response_conflict).round() as u8
}

fn spot_liquidity_shift(metrics: &TofMetrics) -> u8 {
    clamp_score(
        0.35 * metrics.depth_withdrawal_score
            + 0.25 * metrics.liquidity_vacuum_score
            + 0.20 * metrics.spread_widening_score
            + 0.20 * metrics.order_churn_score,
    )
    .round() as u8
}

fn spot_price_response(input: &SplitRiskSystemsInput<'_>) -> u8 {
    let metrics = input.tof_metrics;
    let response_alignment = if directions_align(metrics.metrics_direction, input.short_direction) {
        90.0
    } else if matches!(
        metrics.metrics_direction,
        TofDirection::Mixed | TofDirection::Neutral
    ) || matches!(
        input.short_direction,
        TofDirection::Mixed | TofDirection::Neutral
    ) {
        55.0
    } else {
        40.0
    };
    clamp_score(
        0.55 * metrics.metrics_confidence
            + 0.25 * response_alignment
            + 0.20 * input.short_tof_score,
    )
    .round() as u8
}

fn contract_behavior_components(input: &SplitRiskSystemsInput<'_>) -> ContractBehaviorComponents {
    ContractBehaviorComponents {
        cwm_aggressive_flow: cwm_aggressive_flow(input.perp_score, &input.cwm_contribution),
        oi_impulse: oi_impulse(
            input.perp_score,
            input.advanced_score,
            &input.cwm_contribution,
        ),
        liquidation_context: liquidation_context(&input.cwm_contribution, input.advanced_score),
        funding_crowding: funding_crowding(
            input.perp_score,
            input.advanced_score,
            &input.cwm_contribution,
        ),
        basis_premium: basis_premium(input.perp_score, &input.cwm_contribution),
        active_exchange_confirmation: active_exchange_confirmation(&input.cwm_contribution),
    }
}

fn contract_behavior_score(
    components: &ContractBehaviorComponents,
    weights: &ContractWeights,
) -> u8 {
    clamp_score(
        weights.cwm_aggressive_flow * components.cwm_aggressive_flow as f64
            + weights.oi_impulse * components.oi_impulse as f64
            + weights.liquidation_context * components.liquidation_context as f64
            + weights.funding_crowding * components.funding_crowding as f64
            + weights.basis_premium * components.basis_premium as f64
            + weights.active_exchange_confirmation * components.active_exchange_confirmation as f64,
    )
    .round() as u8
}

fn cwm_aggressive_flow(perp_score: u8, contribution: &CwmRiskContribution) -> u8 {
    if let Some(score) = contribution.score {
        return cap_single_venue_cwm_score(score, contribution);
    }
    clamp_score(perp_score as f64 * 0.65).round() as u8
}

fn cap_single_venue_cwm_score(score: u8, contribution: &CwmRiskContribution) -> u8 {
    if contribution.multi_exchange_confirmed == Some(true) {
        return score;
    }
    match normalized_main_exchange(contribution).as_deref() {
        Some("binance") => score.min(89),
        Some("bitfinex") => score.min(74),
        Some(_) => score.min(74),
        None => score.min(80),
    }
}

fn oi_impulse(perp_score: u8, advanced_score: u8, contribution: &CwmRiskContribution) -> u8 {
    let fallback = 0.65 * perp_score as f64 + 0.35 * advanced_score as f64;
    let Some(oi_change_pct) = contribution.oi_change_pct else {
        return clamp_score(fallback).round() as u8;
    };
    let normalized_pct = if oi_change_pct.abs() <= 1.0 {
        oi_change_pct.abs() * 100.0
    } else {
        oi_change_pct.abs()
    };
    let impulse = 42.0 + (normalized_pct * 8.0).min(38.0);
    let direction_bonus = match (oi_change_pct > 0.0, contribution.direction) {
        (true, Some(ContractWhaleDirection::Buy | ContractWhaleDirection::Sell)) => 18.0,
        (true, Some(ContractWhaleDirection::Absorption | ContractWhaleDirection::Suppression)) => {
            10.0
        }
        (true, None) => 8.0,
        (false, _) => -10.0,
    };
    clamp_score(impulse + direction_bonus).round() as u8
}

fn liquidation_context(contribution: &CwmRiskContribution, advanced_score: u8) -> u8 {
    let cwm_score = contribution.score.unwrap_or(0);
    let base = if contribution.liquidation_suspected == Some(true) {
        0.75 * cwm_score as f64 + 0.25 * advanced_score as f64
    } else if contribution.available {
        0.80 * cwm_score as f64 + 0.20 * advanced_score as f64
    } else {
        advanced_score as f64 * 0.55
    };
    clamp_score(base).round() as u8
}

fn funding_crowding(perp_score: u8, advanced_score: u8, contribution: &CwmRiskContribution) -> u8 {
    if let Some(funding_rate) = contribution.funding_rate {
        let rate_bps = funding_rate.abs() * 10_000.0;
        let bias_bonus = match contribution.funding_bias.as_deref() {
            Some("long") | Some("short") => 8.0,
            _ => 0.0,
        };
        return clamp_score(42.0 + (rate_bps * 5.0).min(40.0) + bias_bonus).round() as u8;
    }
    clamp_score(0.55 * perp_score as f64 + 0.45 * advanced_score as f64).round() as u8
}

fn basis_premium(perp_score: u8, contribution: &CwmRiskContribution) -> u8 {
    if !contribution.available {
        return 50;
    }
    let price_move_component = contribution
        .price_move_pct
        .map(|value| value.abs().min(1.0) * 35.0)
        .unwrap_or(0.0);
    let dominance_component = contribution
        .dominance
        .map(|value| (value.clamp(0.0, 1.0) * 20.0).min(20.0))
        .unwrap_or(0.0);
    clamp_score(45.0 + price_move_component + dominance_component + perp_score as f64 * 0.05)
        .round() as u8
}

fn active_exchange_confirmation(contribution: &CwmRiskContribution) -> u8 {
    if !contribution.available {
        return 35;
    }
    if contribution.multi_exchange_confirmed == Some(true) {
        return 92;
    }
    if normalized_main_exchange(contribution).as_deref() == Some("bitfinex") {
        return 55;
    }
    let quality = contribution.data_quality.unwrap_or(0);
    let score = contribution.score.unwrap_or(0);
    match (quality, score) {
        (80..=100, 90..=100) => 70,
        (70..=100, 75..=89) => 62,
        (70..=100, _) => 55,
        _ => 45,
    }
}

fn market_structure_data_quality(
    input: &SplitRiskSystemsInput<'_>,
    components: &MarketStructureComponents,
) -> f64 {
    let source_health = enabled_source_health_quality(&input.cwm_contribution)
        .unwrap_or_else(|| fallback_enabled_source_health(input.data_quality));
    let cwm_quality = input
        .cwm_contribution
        .data_quality
        .map(|value| value as f64)
        .unwrap_or(input.data_quality);
    round2(clamp_score(
        0.65 * source_health + 0.20 * input.data_quality + 0.15 * cwm_quality
            - if components.source_coverage < 50 {
                6.0
            } else {
                0.0
            },
    ))
}

fn market_structure_confidence(data_quality: f64, components: &MarketStructureComponents) -> f64 {
    round2(clamp_score(
        0.35 * data_quality
            + 0.25 * components.source_coverage as f64
            + 0.20 * components.multi_window_consistency as f64
            + 0.20 * components.signal_agreement as f64,
    ))
}

fn enabled_source_health_quality(contribution: &CwmRiskContribution) -> Option<f64> {
    let config = crate::contract_whale_monitor::config::contract_whale_runtime_config();
    let enabled_sources = config.active_exchange_count();
    if enabled_sources == 0 {
        return Some(100.0);
    }
    if !contribution.available {
        return None;
    }
    let healthy_sources = active_source_count(contribution).min(enabled_sources);
    let score = match (enabled_sources, healthy_sources) {
        (1, 1) => 95.0,
        (1, 0) => 20.0,
        (2, 2) => 95.0,
        (2, 1) => {
            if normalized_main_exchange(contribution).as_deref() == Some("binance") {
                76.0
            } else {
                58.0
            }
        }
        (2, 0) => 20.0,
        (_, 0) => 20.0,
        _ => 20.0 + (healthy_sources as f64 / enabled_sources as f64) * 75.0,
    };
    Some(score)
}

fn fallback_enabled_source_health(input_data_quality: f64) -> f64 {
    clamp_score(0.80 * input_data_quality + 12.0)
}

fn active_source_count(contribution: &CwmRiskContribution) -> usize {
    let mut healthy_sources = contribution.exchange_count.unwrap_or(0);
    if contribution.multi_exchange_confirmed == Some(true) {
        healthy_sources = healthy_sources.max(2);
    } else if contribution.main_exchange.is_some() {
        healthy_sources = healthy_sources.max(1);
    }
    healthy_sources
}

fn normalized_main_exchange(contribution: &CwmRiskContribution) -> Option<String> {
    contribution
        .main_exchange
        .as_deref()
        .map(|exchange| exchange.trim().to_ascii_lowercase())
        .filter(|exchange| !exchange.is_empty())
}

fn duration_score(contribution: &CwmRiskContribution, cross_confirm_score: u8) -> u8 {
    if !contribution.available {
        return if cross_confirm_score >= 75 { 60 } else { 35 };
    }
    let severity_floor = match contribution.severity {
        Some(ContractWhaleSeverity::S) => 100,
        Some(ContractWhaleSeverity::Critical) => 85,
        Some(ContractWhaleSeverity::High) => 70,
        Some(ContractWhaleSeverity::Medium) => 55,
        _ => 45,
    };
    let window_floor = match contribution.window_sec.unwrap_or(0) {
        60.. => 90,
        15..=59 => 75,
        5..=14 => 60,
        1..=4 => 45,
        _ => 40,
    };
    severity_floor
        .max(window_floor)
        .max(cross_confirm_score.min(90))
}

fn liquidation_penalty(contribution: &CwmRiskContribution) -> f64 {
    if contribution.liquidation_suspected != Some(true) {
        return 0.0;
    }
    let ratio_penalty = contribution
        .liquidation_ratio
        .map(|ratio| ratio.clamp(0.0, 1.0) * 10.0)
        .unwrap_or(5.0);
    let oi_drop_penalty = if contribution
        .oi_change_pct
        .is_some_and(|change_pct| change_pct < 0.0)
    {
        8.0
    } else {
        0.0
    };
    let price_shock_penalty = if contribution
        .price_move_pct
        .is_some_and(|price_move_pct| price_move_pct.abs() >= 0.10)
    {
        6.0
    } else {
        0.0
    };
    round2(14.0 + ratio_penalty + oi_drop_penalty + price_shock_penalty)
}

fn crowding_penalty(funding_crowding_score: u8, cross_confirm_score: u8) -> f64 {
    match (funding_crowding_score, cross_confirm_score) {
        (90..=100, 0..=59) => 10.0,
        (85..=100, 60..=69) => 6.0,
        (80..=100, 0..=49) => 5.0,
        _ => 0.0,
    }
}

fn build_market_structure_reasons(
    components: &MarketStructureComponents,
    direction: TofDirection,
    regime_type: &str,
    confirmation_gate: MainForceConfirmationGate,
    config: &MarketStructureRuntimeConfig,
) -> Vec<MarketStructureReason> {
    let direction = direction_key(direction);
    let structure = &config.structure_weights;
    let spot = &config.spot_weights;
    let contract = &config.contract_weights;
    let cross = &config.cross_confirm_weights;
    let main_force = &config.main_force_weights;
    let confirmation = &config.confirmation;
    vec![
        market_structure_reason(
            "SpotScore",
            components.spot_score as f64,
            structure.spot_score,
            "5m/15m",
            direction,
            "spot behavior composite from CVD, volume anomaly, absorption, liquidity shift, and price response",
        ),
        market_structure_reason(
            "SpotCvdScore",
            components.spot_cvd_score as f64,
            spot.spot_cvd,
            "5m/15m",
            direction,
            "spot active net flow and CVD persistence proxy",
        ),
        market_structure_reason(
            "SpotVolumeAnomaly",
            components.spot_volume_anomaly as f64,
            spot.spot_volume_anomaly,
            "5m/15m",
            direction,
            "spot volume anomaly; percentile-ready, currently backed by live trade-rate and VPIN bucket/window-volume proxy",
        ),
        market_structure_reason(
            "SpotAbsorption",
            components.spot_absorption as f64,
            spot.spot_absorption,
            "5m/15m",
            direction,
            "downside absorption or upside suppression proxy from aggressive flow, nearby support/resistance, and response conflict",
        ),
        market_structure_reason(
            "SpotLiquidityShift",
            components.spot_liquidity_shift as f64,
            spot.spot_liquidity_shift,
            "5m/15m",
            direction,
            "spot book depth withdrawal, liquidity vacuum, spread widening, and churn",
        ),
        market_structure_reason(
            "SpotPriceResponse",
            components.spot_price_response as f64,
            spot.spot_price_response,
            "5m/15m",
            direction,
            "whether spot active flow has a healthy same-direction price response or stalls into absorption/suppression",
        ),
        market_structure_reason(
            "ContractScore",
            components.contract_score as f64,
            structure.contract_score,
            "5m/15m",
            direction,
            "contract behavior composite from CWM aggressive flow, OI impulse, liquidation, funding crowding, basis premium, and active exchange confirmation",
        ),
        market_structure_reason(
            "CwmAggressiveFlow",
            components.cwm_aggressive_flow as f64,
            contract.cwm_aggressive_flow,
            "5m/15m",
            direction,
            "active-exchange-aware CWM score; OKX disabled is ignored rather than treated as missing confirmation",
        ),
        market_structure_reason(
            "OiImpulse",
            components.oi_impulse as f64,
            contract.oi_impulse,
            "15m/1h",
            direction,
            "OI increase with aligned active flow supports new position building; OI decrease leans toward closeout or liquidation",
        ),
        market_structure_reason(
            "LiquidationContext",
            components.liquidation_context as f64,
            contract.liquidation_context,
            "5m/15m",
            direction,
            "raises extreme-impact context but liquidation-driven moves are not automatically main-force builds",
        ),
        market_structure_reason(
            "FundingCrowding",
            components.funding_crowding as f64,
            contract.funding_crowding,
            "1h/4h",
            direction,
            "funding crowding and squeeze background",
        ),
        market_structure_reason(
            "BasisPremium",
            components.basis_premium as f64,
            contract.basis_premium,
            "5m/15m",
            direction,
            "basis-premium-ready context; currently uses perp price-move/dominance proxy when explicit basis is unavailable",
        ),
        market_structure_reason(
            "ActiveExchangeConfirmation",
            components.active_exchange_confirmation as f64,
            contract.active_exchange_confirmation,
            "5m/15m",
            direction,
            "enabled exchanges only; disabled venues such as OKX do not reduce confirmation or data quality",
        ),
        market_structure_reason(
            "CrossConfirmScore",
            components.cross_confirm_score as f64,
            structure.cross_confirm_score,
            "15m/1h",
            direction,
            "0.40*SpotContractDirectionConsistency + 0.25*MultiWindowConsistency + 0.20*PriceResponseConsistency + 0.15*SourceCoverage",
        ),
        market_structure_reason(
            "SpotContractDirectionConsistency",
            components.spot_contract_direction_consistency as f64,
            cross.spot_contract_direction_consistency,
            "5m/15m",
            direction,
            "spot flow, contract/CWM flow, OI context, and structure direction alignment",
        ),
        market_structure_reason(
            "MultiWindowConsistency",
            components.multi_window_consistency as f64,
            cross.multi_window_consistency,
            "5m/15m/1h/4h",
            direction,
            "5m startup, 15m confirmation, 1h structure, and 4h background proxy",
        ),
        market_structure_reason(
            "PriceResponseConsistency",
            components.price_response_consistency as f64,
            cross.price_response_consistency,
            "5m/15m",
            direction,
            "separates trend-pushing flow from absorption or suppression",
        ),
        market_structure_reason(
            "SourceCoverage",
            components.source_coverage as f64,
            cross.source_coverage,
            "runtime",
            direction,
            "healthy enabled sources divided by enabled sources; disabled venues such as OKX are not counted",
        ),
        market_structure_reason(
            "SignalAgreement",
            components.signal_agreement as f64,
            0.20,
            "15m/1h",
            direction,
            "agreement across source coverage, multi-window structure, price response, and OI direction",
        ),
        market_structure_reason(
            "SpotContractFloor",
            components.spot_contract_floor as f64,
            main_force.spot_contract_min,
            "15m/1h",
            direction,
            "min(spotScore, contractScore) forces spot and contract confirmation before main-force score can become very high",
        ),
        market_structure_reason(
            "DurationScore",
            components.duration_score as f64,
            main_force.duration_score,
            "15m/1h/4h",
            direction,
            "persistence proxy from CWM severity, signal window, and cross-market confirmation",
        ),
        market_structure_reason(
            "LiquidationPenalty",
            components.liquidation_penalty,
            -1.0,
            "5m/15m",
            direction,
            "subtracts when the move looks liquidation-driven; extreme impact is not automatically main-force behavior",
        ),
        market_structure_reason(
            "CrowdingPenalty",
            components.crowding_penalty,
            -1.0,
            "1h/4h",
            direction,
            "subtracts when funding crowding is high but cross-confirmation is weak",
        ),
        market_structure_reason(
            "MainForceConfirmed",
            if confirmation_gate.confirmed {
                100.0
            } else {
                0.0
            },
            0.0,
            "gate",
            direction,
            &format!(
                "review gate requires mainForceScore>={}, confidence>={}, dataQuality>={}, and at least {}/{} confirmation checks; current {}/{}",
                confirmation.min_main_force_score,
                fmt_weight(confirmation.min_confidence),
                fmt_weight(confirmation.min_data_quality),
                confirmation.min_confirm_conditions,
                confirmation_gate.total,
                confirmation_gate.count,
                confirmation_gate.total
            ),
        ),
        market_structure_reason(
            "OIScore",
            components.oi_score as f64,
            0.0,
            "15m/1h",
            direction,
            "compatibility alias for OiImpulse",
        ),
        market_structure_reason(
            "LiquidationScore",
            components.liquidation_score as f64,
            0.0,
            "5m/15m",
            direction,
            "compatibility alias for LiquidationContext",
        ),
        market_structure_reason(
            "FundingCrowdingScore",
            components.funding_crowding_score as f64,
            0.0,
            "1h/4h",
            direction,
            "compatibility alias for FundingCrowding",
        ),
        market_structure_reason(
            "CwmScore",
            components.cwm_score as f64,
            0.0,
            "5m/15m",
            direction,
            &format!("compatibility alias for CwmAggressiveFlow: {regime_type}"),
        ),
    ]
}

fn market_structure_reason(
    reason_type: &str,
    score: f64,
    weight: f64,
    timeframe: &str,
    direction: &str,
    description: &str,
) -> MarketStructureReason {
    MarketStructureReason {
        reason_type: reason_type.to_string(),
        score: round2(score),
        weight,
        timeframe: timeframe.to_string(),
        direction: direction.to_string(),
        description: description.to_string(),
    }
}

fn market_structure_severity(main_force_score: u8, extreme_impact_score: u8) -> &'static str {
    let _ = extreme_impact_score;
    match main_force_score {
        90..=100 => "Extreme",
        75..=89 => "Major",
        60..=74 => "Confirmed",
        40..=59 => "Watch",
        _ => "Calm",
    }
}

fn main_force_confirmation_gate(
    components: &MarketStructureComponents,
    contribution: &CwmRiskContribution,
    main_force_score: u8,
    confidence: f64,
    data_quality: f64,
    config: &MarketStructureConfirmationConfig,
) -> MainForceConfirmationGate {
    let total = 7;
    let threshold = config.min_confirm_conditions.min(total);
    let checks = [
        components.spot_score >= 60,
        components.contract_score >= 70,
        components.cross_confirm_score >= 60,
        oi_direction_consistent(components, contribution),
        price_response_or_absorption_clear(components, contribution),
        !liquidation_is_primary_driver(components, contribution),
        components.multi_window_consistency >= 70,
    ];
    let count = checks.into_iter().filter(|passed| *passed).count() as u8;
    let confirmed = main_force_score >= config.min_main_force_score
        && confidence >= config.min_confidence
        && data_quality >= config.min_data_quality
        && count >= threshold;
    MainForceConfirmationGate {
        confirmed,
        count,
        total,
        threshold,
    }
}

fn oi_direction_consistent(
    components: &MarketStructureComponents,
    contribution: &CwmRiskContribution,
) -> bool {
    oi_direction_consistent_from_raw(contribution, components.oi_impulse)
}

fn oi_direction_consistent_from_raw(contribution: &CwmRiskContribution, oi_impulse: u8) -> bool {
    let oi_change_pct = contribution.oi_change_pct.unwrap_or(0.0);
    match contribution.direction {
        Some(ContractWhaleDirection::Buy | ContractWhaleDirection::Sell) => {
            oi_change_pct > 0.0 && oi_impulse >= 70
        }
        Some(ContractWhaleDirection::Absorption | ContractWhaleDirection::Suppression) => {
            oi_change_pct >= 0.0 && oi_impulse >= 65
        }
        None => false,
    }
}

fn price_response_or_absorption_clear(
    components: &MarketStructureComponents,
    contribution: &CwmRiskContribution,
) -> bool {
    components.price_response_consistency >= 70
        || components.spot_absorption >= 65
        || matches!(
            contribution.signal_type,
            Some(
                ContractWhaleSignalType::DownsideAbsorption
                    | ContractWhaleSignalType::UpsideSuppression
            )
        )
}

fn liquidation_is_primary_driver(
    components: &MarketStructureComponents,
    contribution: &CwmRiskContribution,
) -> bool {
    contribution.liquidation_suspected == Some(true)
        || components.liquidation_penalty >= 5.0
        || contribution
            .liquidation_ratio
            .is_some_and(|ratio| ratio >= 0.50)
}

fn extreme_impact_score(
    advanced_score: u8,
    perp_score: u8,
    cwm_score: Option<u8>,
    components: &MarketStructureComponents,
    contribution: &CwmRiskContribution,
) -> u8 {
    let base = [advanced_score, perp_score, cwm_score.unwrap_or(0)]
        .into_iter()
        .max()
        .unwrap_or(0) as f64;
    if liquidation_is_primary_driver(components, contribution)
        && contribution
            .oi_change_pct
            .is_some_and(|change_pct| change_pct < 0.0)
        && contribution
            .price_move_pct
            .is_some_and(|price_move_pct| price_move_pct.abs() >= 0.10)
    {
        return clamp_score(
            base.max(85.0)
                .max(components.liquidation_context as f64 + 6.0),
        )
        .round() as u8;
    }
    base.round() as u8
}

fn regime_type(
    components: &MarketStructureComponents,
    contribution: &CwmRiskContribution,
    short_direction: TofDirection,
    metrics_direction: TofDirection,
    contract_flow_shock: bool,
) -> &'static str {
    if is_long_liquidation_cascade(components, contribution, metrics_direction) {
        return "long_liquidation_cascade";
    }
    if is_contract_short_squeeze(components, contribution, metrics_direction) {
        return "contract_short_squeeze";
    }
    if is_downside_absorption(components, contribution) {
        return "downside_absorption";
    }
    if is_upside_resistance(components, contribution) {
        return "upside_resistance";
    }
    if contract_flow_shock {
        return "contract_flow_shock";
    }
    if is_main_force_long_build(components, contribution, metrics_direction) {
        return "main_force_long_build";
    }
    if is_main_force_short_build(components, contribution, metrics_direction) {
        return "main_force_short_build";
    }
    if is_spot_accumulation(components, contribution, short_direction, metrics_direction) {
        return "spot_accumulation";
    }
    if is_spot_distribution(components, contribution, short_direction, metrics_direction) {
        return "spot_distribution";
    }
    if is_range_rotation(components, contribution, short_direction, metrics_direction) {
        return "range_rotation";
    }
    "unclear"
}

fn is_long_liquidation_cascade(
    components: &MarketStructureComponents,
    contribution: &CwmRiskContribution,
    metrics_direction: TofDirection,
) -> bool {
    matches!(metrics_direction, TofDirection::Bearish)
        && contribution.price_move_pct.unwrap_or(0.0) <= -0.10
        && contribution.oi_change_pct.unwrap_or(0.0) < 0.0
        && long_liquidation_dominant(contribution)
        && (contribution.liquidation_suspected == Some(true)
            || components.liquidation_context >= 80)
}

fn is_contract_short_squeeze(
    components: &MarketStructureComponents,
    contribution: &CwmRiskContribution,
    metrics_direction: TofDirection,
) -> bool {
    matches!(metrics_direction, TofDirection::Bullish)
        && contribution.price_move_pct.unwrap_or(0.0) >= 0.10
        && contribution.oi_change_pct.unwrap_or(0.0) < 0.0
        && short_liquidation_dominant(contribution)
        && (contribution.liquidation_suspected == Some(true)
            || components.liquidation_context >= 80)
}

fn is_downside_absorption(
    components: &MarketStructureComponents,
    contribution: &CwmRiskContribution,
) -> bool {
    matches!(
        contribution.signal_type,
        Some(ContractWhaleSignalType::DownsideAbsorption)
    ) || (components.spot_absorption >= 70
        && contribution.price_move_pct.unwrap_or(0.0) > -0.05
        && matches!(
            contribution.direction,
            Some(ContractWhaleDirection::Sell | ContractWhaleDirection::Absorption)
        ))
}

fn is_upside_resistance(
    components: &MarketStructureComponents,
    contribution: &CwmRiskContribution,
) -> bool {
    matches!(
        contribution.signal_type,
        Some(ContractWhaleSignalType::UpsideSuppression)
    ) || (components.price_response_consistency < 65
        && contribution.price_move_pct.unwrap_or(0.0) < 0.05
        && matches!(
            contribution.direction,
            Some(ContractWhaleDirection::Buy | ContractWhaleDirection::Suppression)
        ))
}

fn is_contract_flow_shock(
    components: &MarketStructureComponents,
    contribution: &CwmRiskContribution,
) -> bool {
    let directional_confirmation_count = directional_confirmation_count(components, contribution);
    components.contract_score >= 78
        && components.cwm_aggressive_flow >= 80
        && components.spot_score < 60
        && components.spot_contract_floor < 60
        && components.spot_contract_direction_consistency < 65
        && components.price_response_consistency < 70
        && components.signal_agreement < 70
        && directional_confirmation_count < 3
        && !oi_direction_consistent(components, contribution)
        && !liquidation_is_primary_driver(components, contribution)
        && !is_downside_absorption(components, contribution)
        && !is_upside_resistance(components, contribution)
}

fn capped_main_force_score(
    base_main_force_score: u8,
    components: &MarketStructureComponents,
    contribution: &CwmRiskContribution,
    contract_flow_shock: bool,
    liquidation_driven: bool,
) -> u8 {
    let mut capped = base_main_force_score;
    if contract_flow_shock {
        capped = capped.min(68);
    }
    if liquidation_driven {
        capped = capped.min(liquidation_main_force_cap(components, contribution));
    }
    if directional_confirmation_count(components, contribution) < 3
        && !is_downside_absorption(components, contribution)
        && !is_upside_resistance(components, contribution)
    {
        capped = capped.min(74);
    }
    capped
}

fn directional_confirmation_count(
    components: &MarketStructureComponents,
    contribution: &CwmRiskContribution,
) -> u8 {
    let checks = [
        components.spot_score >= 60 && components.spot_contract_direction_consistency >= 70,
        components.contract_score >= 70,
        oi_direction_consistent(components, contribution),
        components.price_response_consistency >= 70
            || is_downside_absorption(components, contribution)
            || is_upside_resistance(components, contribution),
        components.multi_window_consistency >= 70,
    ];
    checks.into_iter().filter(|passed| *passed).count() as u8
}

fn liquidation_main_force_cap(
    components: &MarketStructureComponents,
    contribution: &CwmRiskContribution,
) -> u8 {
    let ratio = contribution.liquidation_ratio.unwrap_or(0.0);
    let price_shock = contribution
        .price_move_pct
        .is_some_and(|price_move_pct| price_move_pct.abs() >= 0.20);
    if ratio >= 0.75 || price_shock {
        58
    } else if ratio >= 0.50 || components.liquidation_context >= 85 {
        64
    } else {
        69
    }
}

fn is_main_force_long_build(
    components: &MarketStructureComponents,
    contribution: &CwmRiskContribution,
    metrics_direction: TofDirection,
) -> bool {
    matches!(metrics_direction, TofDirection::Bullish)
        && components.spot_score >= 60
        && components.contract_score >= 70
        && components.cross_confirm_score >= 60
        && oi_direction_consistent(components, contribution)
        && !liquidation_is_primary_driver(components, contribution)
        && contribution.price_move_pct.unwrap_or(0.0) >= -0.03
}

fn is_main_force_short_build(
    components: &MarketStructureComponents,
    contribution: &CwmRiskContribution,
    metrics_direction: TofDirection,
) -> bool {
    matches!(metrics_direction, TofDirection::Bearish)
        && components.spot_score >= 60
        && components.contract_score >= 70
        && components.cross_confirm_score >= 60
        && oi_direction_consistent(components, contribution)
        && !liquidation_is_primary_driver(components, contribution)
        && contribution.price_move_pct.unwrap_or(0.0) <= 0.03
}

fn is_spot_accumulation(
    components: &MarketStructureComponents,
    contribution: &CwmRiskContribution,
    short_direction: TofDirection,
    metrics_direction: TofDirection,
) -> bool {
    matches!(metrics_direction, TofDirection::Bullish)
        && matches!(
            short_direction,
            TofDirection::Bullish | TofDirection::Neutral
        )
        && components.spot_score >= 70
        && components.contract_score < 70
        && components.spot_absorption >= 60
        && contribution.price_move_pct.unwrap_or(0.0) >= -0.02
}

fn is_spot_distribution(
    components: &MarketStructureComponents,
    contribution: &CwmRiskContribution,
    short_direction: TofDirection,
    metrics_direction: TofDirection,
) -> bool {
    matches!(short_direction, TofDirection::Bearish)
        && matches!(
            metrics_direction,
            TofDirection::Bearish | TofDirection::Mixed
        )
        && components.spot_score >= 70
        && components.price_response_consistency < 70
        && contribution.price_move_pct.unwrap_or(0.0) <= 0.05
        && matches!(
            contribution.direction,
            Some(ContractWhaleDirection::Buy | ContractWhaleDirection::Suppression) | None
        )
}

fn is_range_rotation(
    components: &MarketStructureComponents,
    contribution: &CwmRiskContribution,
    short_direction: TofDirection,
    metrics_direction: TofDirection,
) -> bool {
    components.cross_confirm_score < 60
        && matches!(
            (short_direction, metrics_direction),
            (TofDirection::Mixed, _)
                | (_, TofDirection::Mixed)
                | (TofDirection::Bullish, TofDirection::Bearish)
                | (TofDirection::Bearish, TofDirection::Bullish)
        )
        && contribution
            .oi_change_pct
            .is_none_or(|change_pct| change_pct.abs() < 0.10)
}

fn long_liquidation_dominant(contribution: &CwmRiskContribution) -> bool {
    contribution.liquidation_long_btc.unwrap_or(0.0)
        > contribution.liquidation_short_btc.unwrap_or(0.0)
}

fn short_liquidation_dominant(contribution: &CwmRiskContribution) -> bool {
    contribution.liquidation_short_btc.unwrap_or(0.0)
        > contribution.liquidation_long_btc.unwrap_or(0.0)
}

fn clamp_score(value: f64) -> f64 {
    if value.is_finite() {
        value.clamp(0.0, 100.0)
    } else {
        0.0
    }
}

fn round2(value: f64) -> f64 {
    (value * 100.0).round() / 100.0
}

fn round4(value: f64) -> f64 {
    (value * 10_000.0).round() / 10_000.0
}

fn toxic_short_timeframes(windows_sec: &[u64]) -> Vec<String> {
    windows_sec
        .iter()
        .map(|window_sec| format!("{window_sec}s"))
        .collect()
}

fn market_structure_timeframes(windows_min: &[u64]) -> Vec<String> {
    windows_min
        .iter()
        .map(|window_min| match *window_min {
            60 => "1h".to_string(),
            120 => "2h".to_string(),
            180 => "3h".to_string(),
            240 => "4h".to_string(),
            value if value % 60 == 0 && value >= 60 => format!("{}h", value / 60),
            value => format!("{value}m"),
        })
        .collect()
}

fn toxic_short_formula(weights: &ToxicShortWeights) -> String {
    format!(
        "toxicScore = {}*ToxicOrderCluster + {}*AggressiveSweep + {}*OrderbookDeformation + {}*SpoofCancel + {}*AdverseMove + {}*LiquidityGap + {}*MicroVolatilityShock; CWM is not fused",
        fmt_weight(weights.toxic_order_cluster),
        fmt_weight(weights.aggressive_sweep),
        fmt_weight(weights.orderbook_deformation),
        fmt_weight(weights.spoof_cancel),
        fmt_weight(weights.adverse_move),
        fmt_weight(weights.liquidity_gap),
        fmt_weight(weights.micro_volatility_shock)
    )
}

fn toxic_short_discord_gate(config: &ToxicShortDiscordConfig) -> String {
    format!(
        "Short toxic Discord only, toxicScore>={}, confidence>={}, dataQuality>={}, cooldown>={}s",
        config.min_score,
        fmt_weight(config.min_confidence),
        fmt_weight(config.min_data_quality),
        config.cooldown_sec
    )
}

fn fmt_weight(value: f64) -> String {
    let formatted = format!("{value:.2}");
    formatted
        .trim_end_matches('0')
        .trim_end_matches('.')
        .to_string()
}
