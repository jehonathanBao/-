use crate::binance_alt_contract_monitor::types::{AltContractSeverity, AltContractSignal};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct AltContractExposureDecision {
    pub allowed: bool,
    pub reason: &'static str,
}

pub fn evaluate_exposure_gate(
    signal: &AltContractSignal,
    warmup: bool,
) -> AltContractExposureDecision {
    if warmup {
        return decision(false, "semantic_warmup");
    }
    let confirmed_windows = signal
        .window_confirmations
        .iter()
        .filter(|window| window.confirmed)
        .count();
    let strong_s_grade_evidence = has_strong_s_grade_evidence(signal, confirmed_windows);
    if confirmed_windows < 2 || signal.evidence_count < 4 {
        return decision(false, "semantic_interpretation_only");
    }
    if semantic_confidence(signal) < 80.0 && !strong_s_grade_evidence {
        return decision(false, "semantic_low_confidence");
    }
    let price_move = signal.price_move_pct.unwrap_or_default().abs();
    if price_move < 0.30 {
        return decision(false, "semantic_no_price_follow_through");
    }
    if is_chop_like_regime(signal.market_regime.regime.as_str()) && !strong_s_grade_evidence {
        return decision(false, "semantic_chop_regime");
    }
    if supporting_dimension_count(signal) < 2 {
        return decision(false, "semantic_single_metric_spike");
    }
    decision(true, "semantic_exposure_ready")
}

fn has_strong_s_grade_evidence(signal: &AltContractSignal, confirmed_windows: usize) -> bool {
    signal.s_grade_eligible
        && signal.severity == AltContractSeverity::S
        && confirmed_windows >= 2
        && signal.evidence_count >= 4
}

fn supporting_dimension_count(signal: &AltContractSignal) -> usize {
    let mut count = 0usize;
    if signal.abnormal_score >= 85 {
        count += 1;
    }
    if signal.build_score >= 80 {
        count += 1;
    }
    if signal.main_force_confidence >= 80.0 {
        count += 1;
    }
    if signal.alt_impact_score.final_score >= signal.alt_impact_score.discord_threshold.max(85.0) {
        count += 1;
    }
    if signal
        .window_confirmations
        .iter()
        .filter(|window| window.confirmed)
        .count()
        >= 2
    {
        count += 1;
    }
    count
}

fn semantic_confidence(signal: &AltContractSignal) -> f64 {
    signal
        .signal_confidence
        .confidence_score
        .max(signal.main_force_confidence)
        .max(signal.market_regime.confidence)
        .max(signal.smart_money_lifecycle.state_confidence)
        .max(signal.smart_money_prediction.confidence)
}

fn is_chop_like_regime(regime: &str) -> bool {
    let normalized = regime.trim().to_ascii_lowercase();
    normalized.contains("chop")
        || normalized.contains("range")
        || normalized.contains("ranging")
        || normalized.contains("unclear")
}

fn decision(allowed: bool, reason: &'static str) -> AltContractExposureDecision {
    AltContractExposureDecision { allowed, reason }
}
