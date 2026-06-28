use crate::binance_alt_contract_monitor::types::{
    AltContractSemanticLayer, AltContractSemanticView, AltContractSignal,
};

use super::{
    exposure_gate::AltContractExposureDecision,
    sanitizer::{semantic_label, semantic_summary, semantic_title},
    severity::descriptive_intensity_label,
};

pub fn seed_semantic_view(signal: &AltContractSignal) -> AltContractSemanticView {
    AltContractSemanticView {
        layer: AltContractSemanticLayer::Interpretation,
        label: semantic_label(signal.signal_type).to_string(),
        intensity_label: descriptive_intensity_label(signal.severity).to_string(),
        exposure_allowed: false,
        exposure_reason: "semantic_interpretation_only".to_string(),
        title: semantic_title(signal.signal_type).to_string(),
        summary: semantic_summary(signal),
        severity_descriptive_only: true,
    }
}

pub fn apply_semantic_boundary(
    signal: &mut AltContractSignal,
    exposure_decision: AltContractExposureDecision,
) {
    signal.semantic = seed_semantic_view(signal);
    signal.semantic.exposure_allowed = exposure_decision.allowed;
    signal.semantic.exposure_reason = exposure_decision.reason.to_string();
    if exposure_decision.allowed {
        signal.semantic.layer = AltContractSemanticLayer::Exposure;
    }
    signal.final_result = signal.semantic.summary.clone();
}
