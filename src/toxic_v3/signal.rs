use serde::{Deserialize, Serialize};

use super::{
    btc_liquidation::{BTCLiquidationEngine, BTCLiquidationState},
    enrichment::{enrich_signal, ToxicV3SignalInput},
    feature_store::FeatureVector,
    flow_reality::directional_strength,
    gex::{GEXEngine, GammaExposureState},
    glce::{GLCEEngine, GLCEState},
    hazard::HazardEngine,
    intent::IntentEngine,
    lhcs::{LHCSEngine, LHCSState},
    mff::{MarketForceField, MarketForceFieldEngine},
    stealth::StealthEngine,
    types::{
        clamp100, Direction, HazardState, IntentState, IntentType, MarketFlowTick, SignalSource,
        StealthState, ToxicV3Enrichment,
    },
};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "snake_case")]
pub enum SignalType {
    TofAnomaly,
    WhaleAccumulation,
    LiquidationCascade,
    StealthEntry,
    StealthExit,
    MarketManipulationRisk,
    #[default]
    FlowInference,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DecisionEngine {
    pub alert_threshold: f64,
    pub stealth_weight: f64,
    pub hazard_weight: f64,
    pub intent_weight: f64,
    pub min_hazard_lambda: f64,
    pub min_stealth_score: f64,
    pub min_confidence: f64,
    pub min_data_quality: f64,
    pub external_dispatch_enabled: bool,
}

impl Default for DecisionEngine {
    fn default() -> Self {
        Self {
            alert_threshold: 80.0,
            stealth_weight: 0.30,
            hazard_weight: 0.40,
            intent_weight: 0.30,
            min_hazard_lambda: 0.60,
            min_stealth_score: 40.0,
            min_confidence: 70.0,
            min_data_quality: 70.0,
            external_dispatch_enabled: false,
        }
    }
}

impl DecisionEngine {
    pub fn should_alert(&self, signal: &SignalEvent) -> bool {
        signal.risk_score >= self.alert_threshold
            && signal.hazard_lambda >= self.min_hazard_lambda
            && signal.stealth_score >= self.min_stealth_score
            && signal.confidence >= self.min_confidence
            && signal.data_quality >= self.min_data_quality
    }

    pub fn risk_score(&self, hazard_lambda: f64, stealth_score: f64, aggression_level: f64) -> f64 {
        let total_weight =
            (self.stealth_weight + self.hazard_weight + self.intent_weight).max(0.01);
        clamp100(
            (hazard_lambda.clamp(0.0, 1.0) * 100.0 * self.hazard_weight
                + stealth_score.clamp(0.0, 100.0) * self.stealth_weight
                + aggression_level.clamp(0.0, 100.0) * self.intent_weight)
                / total_weight,
        )
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SignalEvent {
    pub ts: i64,
    pub symbol: String,
    pub source: SignalSource,
    pub risk_score: f64,
    pub stealth_score: f64,
    pub hazard_lambda: f64,
    pub signal_type: SignalType,
    pub direction: Direction,
    pub confidence: f64,
    pub data_quality: f64,
    pub glce_state: GLCEState,
    pub lhcs_state: LHCSState,
    pub gex_state: GammaExposureState,
    pub market_force_field: MarketForceField,
    pub btc_liquidation_state: Option<BTCLiquidationState>,
    pub should_alert: bool,
    pub external_dispatch_enabled: bool,
    pub enrichment: ToxicV3Enrichment,
}

pub struct SignalAggregator;

impl SignalAggregator {
    pub fn evaluate_tick(
        tick: &MarketFlowTick,
        source: SignalSource,
        data_quality: f64,
        decision: &DecisionEngine,
    ) -> SignalEvent {
        let direction = direction_from_tick(tick);
        let stealth = StealthEngine::analyze(tick);
        let hazard = HazardEngine::compute(tick, &stealth);
        let glce = GLCEEngine::compute(tick, &stealth);
        let lhcs = LHCSEngine::compute(tick, &glce);
        let gex = GEXEngine::compute_from_tick(tick, &glce, &lhcs);
        let mff = MarketForceFieldEngine::compute(tick, &glce, &lhcs, &gex);
        let intent = IntentEngine::infer(tick, direction, &stealth, &hazard);
        Self::evaluate_with_market_field(
            tick,
            source,
            data_quality,
            direction,
            &stealth,
            &hazard,
            &glce,
            &lhcs,
            &gex,
            &mff,
            &intent,
            decision,
        )
    }

    pub fn evaluate_vector(
        vector: &FeatureVector,
        source: SignalSource,
        data_quality: f64,
        decision: &DecisionEngine,
    ) -> SignalEvent {
        let direction = direction_from_tick(&vector.tick);
        let stealth = StealthEngine::analyze_vector(vector);
        let hazard = HazardEngine::compute_vector(vector, &stealth);
        let glce = GLCEEngine::compute_vector(vector, &stealth);
        let lhcs = LHCSEngine::compute(&vector.tick, &glce);
        let gex = GEXEngine::compute_from_tick(&vector.tick, &glce, &lhcs);
        let mff = MarketForceFieldEngine::compute(&vector.tick, &glce, &lhcs, &gex);
        let intent = IntentEngine::infer_vector(vector, direction, &stealth, &hazard);
        Self::evaluate_with_market_field(
            &vector.tick,
            source,
            data_quality,
            direction,
            &stealth,
            &hazard,
            &glce,
            &lhcs,
            &gex,
            &mff,
            &intent,
            decision,
        )
    }

    #[allow(clippy::too_many_arguments)]
    pub fn evaluate(
        tick: &MarketFlowTick,
        source: SignalSource,
        data_quality: f64,
        direction: Direction,
        stealth: &StealthState,
        hazard: &HazardState,
        intent: &IntentState,
        decision: &DecisionEngine,
    ) -> SignalEvent {
        let glce = GLCEEngine::compute(tick, stealth);
        let lhcs = LHCSEngine::compute(tick, &glce);
        let gex = GEXEngine::compute_from_tick(tick, &glce, &lhcs);
        let mff = MarketForceFieldEngine::compute(tick, &glce, &lhcs, &gex);
        Self::evaluate_with_market_field(
            tick,
            source,
            data_quality,
            direction,
            stealth,
            hazard,
            &glce,
            &lhcs,
            &gex,
            &mff,
            intent,
            decision,
        )
    }

    #[allow(clippy::too_many_arguments)]
    pub fn evaluate_with_glce(
        tick: &MarketFlowTick,
        source: SignalSource,
        data_quality: f64,
        direction: Direction,
        stealth: &StealthState,
        hazard: &HazardState,
        glce: &GLCEState,
        intent: &IntentState,
        decision: &DecisionEngine,
    ) -> SignalEvent {
        let lhcs = LHCSEngine::compute(tick, glce);
        let gex = GEXEngine::compute_from_tick(tick, glce, &lhcs);
        let mff = MarketForceFieldEngine::compute(tick, glce, &lhcs, &gex);
        Self::evaluate_with_market_field(
            tick,
            source,
            data_quality,
            direction,
            stealth,
            hazard,
            glce,
            &lhcs,
            &gex,
            &mff,
            intent,
            decision,
        )
    }

    #[allow(clippy::too_many_arguments)]
    pub fn evaluate_with_physics(
        tick: &MarketFlowTick,
        source: SignalSource,
        data_quality: f64,
        direction: Direction,
        stealth: &StealthState,
        hazard: &HazardState,
        glce: &GLCEState,
        lhcs: &LHCSState,
        intent: &IntentState,
        decision: &DecisionEngine,
    ) -> SignalEvent {
        let gex = GEXEngine::compute_from_tick(tick, glce, lhcs);
        let mff = MarketForceFieldEngine::compute(tick, glce, lhcs, &gex);
        Self::evaluate_with_market_field(
            tick,
            source,
            data_quality,
            direction,
            stealth,
            hazard,
            glce,
            lhcs,
            &gex,
            &mff,
            intent,
            decision,
        )
    }

    #[allow(clippy::too_many_arguments)]
    pub fn evaluate_with_force_layers(
        tick: &MarketFlowTick,
        source: SignalSource,
        data_quality: f64,
        direction: Direction,
        stealth: &StealthState,
        hazard: &HazardState,
        glce: &GLCEState,
        lhcs: &LHCSState,
        gex: &GammaExposureState,
        intent: &IntentState,
        decision: &DecisionEngine,
    ) -> SignalEvent {
        let mff = MarketForceFieldEngine::compute(tick, glce, lhcs, gex);
        Self::evaluate_with_market_field(
            tick,
            source,
            data_quality,
            direction,
            stealth,
            hazard,
            glce,
            lhcs,
            gex,
            &mff,
            intent,
            decision,
        )
    }

    #[allow(clippy::too_many_arguments)]
    pub fn evaluate_with_market_field(
        tick: &MarketFlowTick,
        source: SignalSource,
        data_quality: f64,
        direction: Direction,
        stealth: &StealthState,
        hazard: &HazardState,
        glce: &GLCEState,
        lhcs: &LHCSState,
        gex: &GammaExposureState,
        mff: &MarketForceField,
        intent: &IntentState,
        decision: &DecisionEngine,
    ) -> SignalEvent {
        let confidence = confidence_from_intent(&intent, data_quality);
        let risk_score = decision.risk_score(
            hazard.lambda_t,
            stealth.stealth_score,
            intent.aggression_level,
        );
        let signal_type = classify_signal(&intent, tick);
        let btc_liquidation_state = BTCLiquidationEngine::compute(tick, glce, lhcs, gex);
        let enrichment = enrich_signal(&ToxicV3SignalInput {
            source,
            direction,
            risk_score,
            data_quality,
            flow: tick.clone(),
        });

        let mut event = SignalEvent {
            ts: tick.ts,
            symbol: tick.symbol.clone(),
            source,
            risk_score,
            stealth_score: stealth.stealth_score,
            hazard_lambda: hazard.lambda_t,
            signal_type,
            direction,
            confidence,
            data_quality,
            glce_state: glce.clone(),
            lhcs_state: lhcs.clone(),
            gex_state: gex.clone(),
            market_force_field: mff.clone(),
            btc_liquidation_state,
            should_alert: false,
            external_dispatch_enabled: decision.external_dispatch_enabled,
            enrichment,
        };
        event.should_alert = decision.should_alert(&event);
        event
    }
}

fn confidence_from_intent(intent: &IntentState, data_quality: f64) -> f64 {
    clamp100(intent.confidence * 0.70 + data_quality * 0.30)
}

fn classify_signal(intent: &IntentState, tick: &MarketFlowTick) -> SignalType {
    match intent.intent {
        IntentType::StealthBuildUp => SignalType::StealthEntry,
        IntentType::Accumulation => SignalType::WhaleAccumulation,
        IntentType::Distribution => SignalType::StealthExit,
        IntentType::PanicExit | IntentType::StopHunt => SignalType::LiquidationCascade,
        IntentType::LiquidityHunting => SignalType::MarketManipulationRisk,
        IntentType::Unknown if directional_strength(tick) >= 0.70 => SignalType::TofAnomaly,
        IntentType::Unknown => SignalType::FlowInference,
    }
}

fn direction_from_tick(tick: &MarketFlowTick) -> Direction {
    if tick.net_flow > 0.0 {
        Direction::Buy
    } else if tick.net_flow < 0.0 {
        Direction::Sell
    } else {
        Direction::Neutral
    }
}
