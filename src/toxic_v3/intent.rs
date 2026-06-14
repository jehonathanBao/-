use super::{
    feature_store::FeatureVector,
    flow_reality::directional_strength,
    types::{
        clamp01, Direction, HazardState, IntentState, IntentType, MarketFlowTick, StealthState,
    },
};

pub fn infer_intent(
    flow: &MarketFlowTick,
    direction: Direction,
    stealth: &StealthState,
    hazard: &HazardState,
) -> IntentState {
    let directional = directional_strength(flow);
    let oi_delta = flow.open_interest_delta;
    let price_move = flow.price_move_pct;
    let liquidation = hazard.liquidation_risk;

    let intent = if liquidation >= 0.40 && oi_delta < 0.0 {
        if price_move.abs() >= 0.25 {
            IntentType::PanicExit
        } else {
            IntentType::StopHunt
        }
    } else if stealth.is_camouflaging && oi_delta > 0.0 && directional < 0.70 {
        IntentType::StealthBuildUp
    } else if matches!(direction, Direction::Buy) && oi_delta > 0.0 {
        IntentType::Accumulation
    } else if matches!(direction, Direction::Sell) && oi_delta < 0.0 {
        IntentType::Distribution
    } else if hazard.lambda_t >= 0.70 && directional >= 0.70 {
        IntentType::LiquidityHunting
    } else {
        IntentType::Unknown
    };

    let confidence = match intent {
        IntentType::PanicExit | IntentType::StopHunt => {
            55.0 + liquidation * 35.0 + clamp01(price_move.abs() / 2.0) * 10.0
        }
        IntentType::StealthBuildUp => {
            45.0 + stealth.stealth_score * 0.35 + clamp01(oi_delta.abs() / 1000.0) * 20.0
        }
        IntentType::Accumulation | IntentType::Distribution => {
            45.0 + directional * 25.0
                + hazard.lambda_t * 15.0
                + clamp01(oi_delta.abs() / 1000.0) * 15.0
        }
        IntentType::LiquidityHunting => 45.0 + hazard.lambda_t * 35.0 + directional * 20.0,
        IntentType::Unknown => 20.0 + hazard.lambda_t * 20.0,
    }
    .clamp(0.0, 100.0);

    let expected_horizon_sec = match intent {
        IntentType::StealthBuildUp | IntentType::Accumulation | IntentType::Distribution => 900.0,
        IntentType::LiquidityHunting | IntentType::StopHunt => 180.0,
        IntentType::PanicExit => 60.0,
        IntentType::Unknown => 0.0,
    };

    IntentState {
        intent,
        confidence,
        expected_horizon_sec,
        aggression_level: (hazard.lambda_t * 100.0).clamp(0.0, 100.0),
    }
}

pub struct IntentEngine;

impl IntentEngine {
    pub fn infer(
        flow: &MarketFlowTick,
        direction: Direction,
        stealth: &StealthState,
        hazard: &HazardState,
    ) -> IntentState {
        infer_intent(flow, direction, stealth, hazard)
    }

    pub fn infer_vector(
        vector: &FeatureVector,
        direction: Direction,
        stealth: &StealthState,
        hazard: &HazardState,
    ) -> IntentState {
        infer_intent(&vector.tick, direction, stealth, hazard)
    }
}
