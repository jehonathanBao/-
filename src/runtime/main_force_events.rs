use crate::{
    api::toxic_signal_ws_routes::ToxicSignalWsItem,
    types::main_force_event::MainForceEventObservation,
};

pub fn best_main_force_event_observation(
    signals: &[ToxicSignalWsItem],
    symbol: &str,
) -> Option<MainForceEventObservation> {
    signals
        .iter()
        .filter(|signal| signal.symbol.eq_ignore_ascii_case(symbol))
        .max_by(|left, right| {
            score_signal(left)
                .partial_cmp(&score_signal(right))
                .unwrap_or(std::cmp::Ordering::Equal)
                .then_with(|| {
                    left.market_structure_score
                        .ts
                        .cmp(&right.market_structure_score.ts)
                })
        })
        .map(|signal| MainForceEventObservation {
            symbol: signal.symbol.clone(),
            observed_at: signal.market_structure_score.ts,
            regime_type: signal.regime_type.clone(),
            severity: signal.market_structure_severity.clone(),
            main_force_score: signal.main_force_score as f64,
            extreme_impact_score: signal.extreme_impact_score as f64,
            structure_bias: signal.structure_bias as f64,
            confidence: signal.market_structure_confidence,
            spot_score: Some(signal.spot_score as f64),
            contract_score: Some(signal.contract_score as f64),
            cross_confirm_score: Some(signal.cross_confirm_score as f64),
            cwm_score: Some(signal.cwm_score as f64),
            oi_score: Some(signal.oi_score as f64),
            liquidation_score: Some(signal.liquidation_score as f64),
            funding_crowding_score: Some(signal.funding_crowding_score as f64),
            main_force_confirmed: signal.main_force_confirmed,
            extreme_impact_confirmed: signal.extreme_impact_confirmed,
            liquidation_driven: signal.cwm_contribution.liquidation_suspected == Some(true)
                || matches!(
                    signal.regime_type.as_str(),
                    "long_liquidation_cascade" | "contract_short_squeeze"
                ),
            reasons_json: serde_json::json!({
                "coreReason": signal.core_reason,
                "finalResult": signal.final_result,
                "explainTags": signal.explain_tags,
                "regimeType": signal.regime_type,
                "mainForceConfirmed": signal.main_force_confirmed,
                "extremeImpactConfirmed": signal.extreme_impact_confirmed,
            }),
        })
}

fn score_signal(signal: &ToxicSignalWsItem) -> f64 {
    (signal.main_force_score.max(signal.extreme_impact_score)) as f64
        + signal.market_structure_confidence * 0.01
}
