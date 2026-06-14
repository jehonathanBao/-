use super::{
    feature_store::FeatureVector,
    flow_reality::total_volume,
    types::{clamp01, HazardState, HazardStateKind, MarketFlowTick, StealthState},
};

pub fn compute_hazard_state(flow: &MarketFlowTick, stealth: &StealthState) -> HazardState {
    let components = hazard_components(flow, stealth);
    let lambda_t = compute_lambda(flow, stealth);

    let state = match lambda_t {
        lambda if lambda >= 0.75 => HazardStateKind::Critical,
        lambda if lambda >= 0.50 => HazardStateKind::Elevated,
        lambda if lambda >= 0.25 => HazardStateKind::Building,
        _ => HazardStateKind::Calm,
    };

    HazardState {
        lambda_t,
        detection_pressure: lambda_t,
        regulatory_sensitivity: components.dynamic_pressure,
        anomaly_persistence: components.persistence,
        flow_irregularity: components.flow_irregularity,
        liquidation_risk: components.liquidation_risk,
        state,
    }
}

pub fn compute_lambda(flow: &MarketFlowTick, stealth: &StealthState) -> f64 {
    let components = hazard_components(flow, stealth);
    clamp01(
        components.acceleration * 0.20
            + clamp01(flow.large_trade_ratio) * 0.18
            + (stealth.stealth_score / 100.0) * 0.18
            + components.oi_pressure * 0.12
            + clamp01(flow.realized_vol) * 0.08
            + components.liquidation_risk * 0.09
            + components.persistence * 0.08
            + components.dynamic_pressure * 0.07,
    )
}

#[derive(Debug, Clone, Copy)]
struct HazardComponents {
    acceleration: f64,
    oi_pressure: f64,
    persistence: f64,
    dynamic_pressure: f64,
    liquidation_risk: f64,
    flow_irregularity: f64,
}

fn hazard_components(flow: &MarketFlowTick, _stealth: &StealthState) -> HazardComponents {
    let total = total_volume(flow);
    let acceleration = if total <= f64::EPSILON {
        0.0
    } else {
        clamp01(flow.flow_acceleration.abs() / total)
    };
    let oi_pressure = clamp01(flow.open_interest_delta.abs() / (total + 1.0));
    let persistence = clamp01(flow.anomaly_persistence_sec / 600.0);
    let dynamic_pressure = clamp01(flow.dynamic_multiple / 10.0);
    let liquidation_risk = clamp01(flow.liquidation_pressure);
    let flow_irregularity = (acceleration * 0.60 + dynamic_pressure * 0.40).clamp(0.0, 1.0);

    HazardComponents {
        acceleration,
        oi_pressure,
        persistence,
        dynamic_pressure,
        liquidation_risk,
        flow_irregularity,
    }
}

pub struct HazardEngine;

impl HazardEngine {
    pub fn compute(flow: &MarketFlowTick, stealth: &StealthState) -> HazardState {
        compute_hazard_state(flow, stealth)
    }

    pub fn compute_vector(vector: &FeatureVector, stealth: &StealthState) -> HazardState {
        let mut hazard = compute_hazard_state(&vector.tick, stealth);
        let zscore_pressure = clamp01(vector.flow_zscore.abs() / 10.0);
        hazard.lambda_t = clamp01(hazard.lambda_t + zscore_pressure * 0.05);
        hazard.detection_pressure = hazard.lambda_t;
        hazard.state = match hazard.lambda_t {
            lambda if lambda >= 0.75 => HazardStateKind::Critical,
            lambda if lambda >= 0.50 => HazardStateKind::Elevated,
            lambda if lambda >= 0.25 => HazardStateKind::Building,
            _ => HazardStateKind::Calm,
        };
        hazard
    }

    pub fn compute_lambda(flow: &MarketFlowTick, stealth: &StealthState) -> f64 {
        compute_lambda(flow, stealth)
    }
}
