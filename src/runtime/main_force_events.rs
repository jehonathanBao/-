use crate::{
    api::toxic_signal_ws_routes::ToxicSignalWsItem,
    runtime::cwm_risk_fusion::MainForceStructureRisk,
    types::main_force_event::MainForceEventObservation,
};

pub fn best_main_force_event_observation(
    signals: &[ToxicSignalWsItem],
    symbol: &str,
) -> Option<MainForceEventObservation> {
    signals
        .iter()
        .filter(|signal| signal.symbol.eq_ignore_ascii_case(symbol))
        .filter_map(|signal| {
            signal
                .market_structure_score
                .as_ref()
                .map(|score| (signal, score))
        })
        .max_by(|left, right| {
            score_signal(left.1)
                .partial_cmp(&score_signal(right.1))
                .unwrap_or(std::cmp::Ordering::Equal)
                .then_with(|| left.1.ts.cmp(&right.1.ts))
        })
        .map(|(signal, score)| MainForceEventObservation {
            symbol: signal.symbol.clone(),
            observed_at: score.ts,
            regime_type: score.regime_type.clone(),
            severity: score.severity.clone(),
            main_force_score: score.main_force_score as f64,
            extreme_impact_score: score.extreme_impact_score as f64,
            structure_bias: score.structure_bias as f64,
            confidence: score.confidence,
            spot_score: Some(score.spot_score as f64),
            contract_score: Some(score.contract_score as f64),
            cross_confirm_score: Some(score.cross_confirm_score as f64),
            cwm_score: Some(score.cwm_score as f64),
            oi_score: Some(score.oi_score as f64),
            liquidation_score: Some(score.liquidation_score as f64),
            funding_crowding_score: Some(score.funding_crowding_score as f64),
            main_force_confirmed: score.main_force_confirmed,
            extreme_impact_confirmed: score.extreme_impact_confirmed,
            liquidation_driven: signal.cwm_contribution.liquidation_suspected == Some(true)
                || matches!(
                    score.regime_type.as_str(),
                    "long_liquidation_cascade" | "contract_short_squeeze"
                ),
            reasons_json: serde_json::json!({
                "coreReason": signal.core_reason,
                "finalResult": signal.final_result,
                "explainTags": signal.explain_tags,
                "regimeType": score.regime_type,
                "mainForceConfirmed": score.main_force_confirmed,
                "extremeImpactConfirmed": score.extreme_impact_confirmed,
            }),
        })
}

fn score_signal(score: &MainForceStructureRisk) -> f64 {
    (score.main_force_score.max(score.extreme_impact_score)) as f64 + score.confidence * 0.01
}
