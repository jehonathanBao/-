use serde::{Deserialize, Serialize};

use super::{
    flow_reality::derive_stealth_features,
    hazard::compute_hazard_state,
    intent::infer_intent,
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
        explanation_tags: explanation_tags(
            stealth.regime,
            hazard.state,
            intent.intent,
            trajectory.state,
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
    tags
}
