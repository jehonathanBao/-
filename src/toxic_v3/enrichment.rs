use serde::{Deserialize, Serialize};

use super::{
    btc_liquidation::BTCLiquidationEngine,
    flow_reality::derive_stealth_features,
    gex::GEXEngine,
    glce::GLCEEngine,
    hazard::compute_hazard_state,
    intent::infer_intent,
    lhcs::LHCSEngine,
    mff::MarketForceFieldEngine,
    stealth::compute_stealth_state,
    trajectory::compute_trajectory_state,
    types::{
        Direction, HazardStateKind, IntentType, MarketFlowTick, SignalSource, StealthRegime,
        ToxicV3Enrichment, TrajectoryStateKind,
    },
};

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ToxicV3SignalInput {
    pub source: SignalSource,
    pub direction: Direction,
    pub risk_score: f64,
    pub data_quality: f64,
    pub flow: MarketFlowTick,
}

impl Default for ToxicV3SignalInput {
    fn default() -> Self {
        Self {
            source: SignalSource::Unknown,
            direction: Direction::Neutral,
            risk_score: 0.0,
            data_quality: 0.0,
            flow: MarketFlowTick::default(),
        }
    }
}

pub fn enrich_signal(input: &ToxicV3SignalInput) -> ToxicV3Enrichment {
    let features = derive_stealth_features(&input.flow);
    let stealth = compute_stealth_state(&features);
    let hazard = compute_hazard_state(&input.flow, &stealth);
    let glce = GLCEEngine::compute(&input.flow, &stealth);
    let lhcs = LHCSEngine::compute(&input.flow, &glce);
    let gex = GEXEngine::compute_from_tick(&input.flow, &glce, &lhcs);
    let mff = MarketForceFieldEngine::compute(&input.flow, &glce, &lhcs, &gex);
    let btc_liquidation = BTCLiquidationEngine::compute(&input.flow, &glce, &lhcs, &gex);
    let intent = infer_intent(&input.flow, input.direction, &stealth, &hazard);
    let trajectory = compute_trajectory_state(&input.flow, &hazard);

    ToxicV3Enrichment {
        symbol: input.flow.symbol.clone(),
        ts: input.flow.ts,
        source: input.source,
        stealth_score: stealth.stealth_score,
        stealth_regime: stealth.regime,
        hazard_lambda: hazard.lambda_t,
        hazard_state: hazard.state,
        intent: intent.intent,
        intent_confidence: intent.confidence,
        trajectory_score: trajectory.score,
        trajectory_state: trajectory.state,
        glce_squeeze_probability: glce.squeeze_probability,
        glce_liquidation_risk: glce.liquidation_risk,
        glce_gamma_pressure: glce.gamma_pressure,
        glce_breakout_bias: glce_breakout_bias_label(glce.breakout_bias).to_string(),
        lhcs_cascade_probability: lhcs.cascade_state.cascade_probability,
        lhcs_direction_bias: lhcs_direction_bias_label(lhcs.cascade_state.direction_bias)
            .to_string(),
        lhcs_trigger_level_count: lhcs.trigger_levels.len(),
        lhcs_liquidity_void_count: lhcs.liquidity_void_zones.len(),
        gex_total: gex.total_gex,
        gex_dealer_position_bias: dealer_bias_label(gex.dealer_position_bias).to_string(),
        gex_squeeze_probability: gex.squeeze_probability,
        gex_price_pin_pressure_index: gex.price_pin_pressure_index,
        gex_gamma_wall_count: gex.gamma_wall_levels.len(),
        mff_total_stress: mff.total_stress,
        mff_liquidity_field: mff.liquidity_field,
        mff_gamma_field: mff.gamma_field,
        mff_liquidation_field: mff.liquidation_field,
        mff_cascade_field: mff.cascade_field,
        mff_directional_bias: direction_label(mff.directional_bias).to_string(),
        mff_instability_index: mff.instability_index,
        mff_regime_state: market_regime_label(mff.regime_state).to_string(),
        btc_liquidation_active: btc_liquidation.is_some(),
        btc_long_liquidation_pressure: btc_liquidation
            .as_ref()
            .map(|state| state.long_liquidation_pressure)
            .unwrap_or_default(),
        btc_short_liquidation_pressure: btc_liquidation
            .as_ref()
            .map(|state| state.short_liquidation_pressure)
            .unwrap_or_default(),
        btc_net_liquidation_bias: btc_liquidation
            .as_ref()
            .map(|state| state.net_liquidation_bias)
            .unwrap_or_default(),
        btc_squeeze_up_probability: btc_liquidation
            .as_ref()
            .map(|state| state.squeeze_up_probability)
            .unwrap_or_default(),
        btc_squeeze_down_probability: btc_liquidation
            .as_ref()
            .map(|state| state.squeeze_down_probability)
            .unwrap_or_default(),
        btc_liquidation_cluster_count: btc_liquidation
            .as_ref()
            .map(|state| state.liquidation_clusters.len())
            .unwrap_or_default(),
        btc_cascade_risk: btc_liquidation
            .as_ref()
            .map(|state| state.cascade_risk)
            .unwrap_or_default(),
        btc_gamma_pressure: btc_liquidation
            .as_ref()
            .map(|state| state.gamma_pressure)
            .unwrap_or_default(),
        explanation_tags: explanation_tags(
            stealth.regime,
            hazard.state,
            intent.intent,
            trajectory.state,
            &glce,
            &lhcs,
            &gex,
            &mff,
            btc_liquidation.as_ref(),
            input.data_quality,
        ),
        read_only: true,
        analysis_only: true,
        direct_discord_gate: false,
    }
}

fn explanation_tags(
    stealth: StealthRegime,
    hazard: HazardStateKind,
    intent: IntentType,
    trajectory: TrajectoryStateKind,
    glce: &super::glce::GLCEState,
    lhcs: &super::lhcs::LHCSState,
    gex: &super::gex::GammaExposureState,
    mff: &super::mff::MarketForceField,
    btc_liquidation: Option<&super::btc_liquidation::BTCLiquidationState>,
    data_quality: f64,
) -> Vec<String> {
    let mut tags = Vec::new();
    match stealth {
        StealthRegime::ActiveCamouflage | StealthRegime::ExtremeStealth => {
            tags.push("stealth_camouflage".to_string());
        }
        StealthRegime::PartialStealth => tags.push("partial_stealth".to_string()),
        StealthRegime::NonStealth | StealthRegime::Unknown => {}
    }
    match hazard {
        HazardStateKind::Critical => tags.push("hazard_critical".to_string()),
        HazardStateKind::Elevated => tags.push("hazard_elevated".to_string()),
        HazardStateKind::Building => tags.push("hazard_building".to_string()),
        HazardStateKind::Calm | HazardStateKind::Unknown => {}
    }
    match intent {
        IntentType::Accumulation => tags.push("intent_accumulation".to_string()),
        IntentType::Distribution => tags.push("intent_distribution".to_string()),
        IntentType::LiquidityHunting => tags.push("intent_liquidity_hunting".to_string()),
        IntentType::StopHunt => tags.push("intent_stop_hunt".to_string()),
        IntentType::StealthBuildUp => tags.push("intent_stealth_build_up".to_string()),
        IntentType::PanicExit => tags.push("intent_panic_exit".to_string()),
        IntentType::Unknown => tags.push("intent_unclear".to_string()),
    }
    match trajectory {
        TrajectoryStateKind::Persistent => tags.push("trajectory_persistent".to_string()),
        TrajectoryStateKind::Building => tags.push("trajectory_building".to_string()),
        TrajectoryStateKind::Reversal => tags.push("trajectory_reversal".to_string()),
        TrajectoryStateKind::Decaying => tags.push("trajectory_decaying".to_string()),
        TrajectoryStateKind::SinglePoint | TrajectoryStateKind::Unknown => {}
    }
    if data_quality < 70.0 {
        tags.push("data_quality_low".to_string());
    }
    if glce.squeeze_probability >= 0.70 {
        tags.push("glce_squeeze_probability_high".to_string());
    }
    if glce.liquidation_risk >= 0.60 {
        tags.push("glce_liquidation_risk_high".to_string());
    }
    if glce.gamma_pressure >= 0.60 {
        tags.push("glce_gamma_pressure_high".to_string());
    }
    if lhcs.cascade_state.cascade_probability >= 0.70 {
        tags.push("lhcs_cascade_probability_high".to_string());
    }
    if !lhcs.cascade_state.propagation_chain.is_empty() {
        tags.push("lhcs_propagation_chain_present".to_string());
    }
    if !lhcs.liquidity_void_zones.is_empty() {
        tags.push("lhcs_liquidity_void_detected".to_string());
    }
    if gex.squeeze_probability >= 0.70 {
        tags.push("gex_squeeze_probability_high".to_string());
    }
    if gex.price_pin_pressure_index >= 0.70 {
        tags.push("gex_price_pin_pressure_high".to_string());
    }
    if !gex.gamma_wall_levels.is_empty() {
        tags.push("gex_gamma_wall_present".to_string());
    }
    if mff.total_stress >= 0.70 {
        tags.push("mff_total_stress_high".to_string());
    }
    if mff.instability_index >= 0.75 {
        tags.push("mff_instability_high".to_string());
    }
    match mff.regime_state {
        super::mff::MarketRegime::CriticalInstability => {
            tags.push("mff_regime_critical_instability".to_string());
        }
        super::mff::MarketRegime::FragileAccumulation => {
            tags.push("mff_regime_fragile_accumulation".to_string());
        }
        super::mff::MarketRegime::FragileDistribution => {
            tags.push("mff_regime_fragile_distribution".to_string());
        }
        super::mff::MarketRegime::Compression => {
            tags.push("mff_regime_compression".to_string());
        }
        super::mff::MarketRegime::Stable | super::mff::MarketRegime::Unknown => {}
    }
    if let Some(state) = btc_liquidation {
        tags.push("btc_liquidation_engine_active".to_string());
        if state.squeeze_up_probability >= 0.65 {
            tags.push("btc_squeeze_up_probability_high".to_string());
        }
        if state.squeeze_down_probability >= 0.65 {
            tags.push("btc_squeeze_down_probability_high".to_string());
        }
        if state.cascade_risk >= 0.65 {
            tags.push("btc_cascade_risk_high".to_string());
        }
    }
    tags
}

fn glce_breakout_bias_label(value: super::glce::BreakoutBias) -> &'static str {
    match value {
        super::glce::BreakoutBias::LongSqueeze => "long_squeeze",
        super::glce::BreakoutBias::ShortSqueeze => "short_squeeze",
        super::glce::BreakoutBias::Neutral => "neutral",
    }
}

fn lhcs_direction_bias_label(value: super::lhcs::CascadeDirection) -> &'static str {
    match value {
        super::lhcs::CascadeDirection::UpwardSqueeze => "upward_squeeze",
        super::lhcs::CascadeDirection::DownwardSqueeze => "downward_squeeze",
        super::lhcs::CascadeDirection::Neutral => "neutral",
    }
}

fn dealer_bias_label(value: super::gex::DealerBias) -> &'static str {
    match value {
        super::gex::DealerBias::BuyDips => "buy_dips",
        super::gex::DealerBias::SellRallies => "sell_rallies",
        super::gex::DealerBias::Neutral => "neutral",
    }
}

fn direction_label(value: Direction) -> &'static str {
    match value {
        Direction::Buy => "buy",
        Direction::Sell => "sell",
        Direction::Absorption => "absorption",
        Direction::Suppression => "suppression",
        Direction::Neutral => "neutral",
    }
}

fn market_regime_label(value: super::mff::MarketRegime) -> &'static str {
    match value {
        super::mff::MarketRegime::Stable => "stable",
        super::mff::MarketRegime::Compression => "compression",
        super::mff::MarketRegime::FragileAccumulation => "fragile_accumulation",
        super::mff::MarketRegime::FragileDistribution => "fragile_distribution",
        super::mff::MarketRegime::CriticalInstability => "critical_instability",
        super::mff::MarketRegime::Unknown => "unknown",
    }
}
