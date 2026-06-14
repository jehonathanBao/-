use super::{
    feature_store::FeatureVector,
    flow_reality::derive_stealth_features,
    types::{clamp100, MarketFlowTick, StealthFeatures, StealthRegime, StealthState},
};

pub fn compute_stealth_state(features: &StealthFeatures) -> StealthState {
    let raw = features.fragmentation_index * 0.24
        + features.execution_entropy * 0.18
        + features.cross_exchange_sync * 0.12
        + features.order_size_variance * 0.14
        + features.timing_jitter * 0.12
        + features.impact_dilution_ratio * 0.14
        + features.cross_exchange_dispersion * 0.06;
    let stealth_score = clamp100(raw * 100.0);
    let gamma = (stealth_score / 100.0 * 0.5).min(0.49);
    let regime = match stealth_score {
        score if score >= 80.0 => StealthRegime::ExtremeStealth,
        score if score >= 55.0 => StealthRegime::ActiveCamouflage,
        score if score >= 25.0 => StealthRegime::PartialStealth,
        _ => StealthRegime::NonStealth,
    };

    StealthState {
        gamma,
        stealth_score,
        is_camouflaging: stealth_score >= 55.0,
        regime,
    }
}

pub struct StealthEngine;

impl StealthEngine {
    pub fn analyze(tick: &MarketFlowTick) -> StealthState {
        let features = derive_stealth_features(tick);
        compute_stealth_state(&features)
    }

    pub fn analyze_vector(vector: &FeatureVector) -> StealthState {
        let mut features = derive_stealth_features(&vector.tick);
        features.execution_entropy = features.execution_entropy.max(vector.rolling_entropy);
        features.timing_jitter =
            (features.timing_jitter + (1.0 / (1.0 + vector.flow_zscore.abs()))).clamp(0.0, 1.0)
                / 2.0;
        compute_stealth_state(&features)
    }

    pub fn analyze_features(features: &StealthFeatures) -> StealthState {
        compute_stealth_state(features)
    }
}
