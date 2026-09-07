//! Deterministic, read-only interpretation of contract-whale event evidence.
//!
//! This module deliberately describes a behaviour hypothesis, not an actor
//! identity or a trading signal.  Detection-time fields never consume future
//! markout/outcome data.

use serde::{Deserialize, Serialize};

use super::{
    impact_grade::{AssessmentStatus, ContractEventImpactAssessment},
    types::{
        ContractWhaleActiveFlowDirection, ContractWhaleOiContextTag,
        ContractWhalePriceResponseType, ContractWhaleSignal, ContractWhaleStructureInterpretation,
    },
};

pub const CONTRACT_WHALE_BEHAVIOR_VERSION: &str = "cwm_behavior_v2";

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct BehaviorOutcomeMarkouts {
    pub markout_30s_bps: Option<f64>,
    pub markout_2m_bps: Option<f64>,
    pub markout_5m_bps: Option<f64>,
    pub evaluated_at: i64,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ContractWhaleBehaviorHypothesis {
    InitiativeLongBuild,
    InitiativeShortBuild,
    ShortCovering,
    LongUnwind,
    DownsideAbsorption,
    UpsideSuppression,
    LongLiquidationCascade,
    ShortSqueeze,
    ActiveBuyPressure,
    ActiveSellPressure,
    Unclear,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum BehaviorDirectionBias {
    Bullish,
    Bearish,
    Neutral,
    Unknown,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum BehaviorAttribution {
    VoluntaryPositionBuild,
    PositionClose,
    PassiveAbsorption,
    ForcedFlow,
    ActiveFlowUnattributed,
    Unclear,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum BehaviorConfidenceLevel {
    Low,
    Medium,
    High,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum BehaviorDecisionState {
    Observe,
    AwaitingConfirmation,
    Confirmed,
    Invalidated,
    ExpiredUnconfirmed,
    NoTrade,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct BehaviorEvidenceItem {
    pub code: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub value: Option<f64>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct BehaviorRule {
    pub code: String,
    pub threshold_bps: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct BehaviorPostEventValidation {
    pub horizon: String,
    pub markout_bps: f64,
    pub signed: bool,
    pub state: BehaviorDecisionState,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ContractWhaleBehaviorAssessment {
    pub behavior_version: String,
    pub hypothesis: ContractWhaleBehaviorHypothesis,
    pub direction_bias: BehaviorDirectionBias,
    pub attribution: BehaviorAttribution,
    pub confidence_score: u8,
    pub confidence_level: BehaviorConfidenceLevel,
    pub confidence_semantics: String,
    pub decision_state: BehaviorDecisionState,
    pub supporting_evidence: Vec<BehaviorEvidenceItem>,
    pub contradicting_evidence: Vec<BehaviorEvidenceItem>,
    pub missing_evidence: Vec<BehaviorEvidenceItem>,
    pub confirmation_rule: BehaviorRule,
    pub invalidation_rule: BehaviorRule,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub post_event_validation: Option<BehaviorPostEventValidation>,
    pub assessed_at_ms: i64,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub outcome_evaluated_at_ms: Option<i64>,
}

pub fn build_detection_behavior(
    signal: &ContractWhaleSignal,
    impact: Option<&ContractEventImpactAssessment>,
    assessed_at_ms: i64,
) -> ContractWhaleBehaviorAssessment {
    let (hypothesis, direction_bias, attribution) = classify(signal);
    let mut supporting = Vec::new();
    let mut contradicting = Vec::new();
    let mut missing = Vec::new();

    add_signal_evidence(
        signal,
        &hypothesis,
        &mut supporting,
        &mut contradicting,
        &mut missing,
    );
    let impact_score = impact
        .and_then(|item| item.evidence.confidence_score)
        .unwrap_or(0.0)
        .clamp(0.0, 100.0);
    let impact_graded = impact.is_some_and(|item| item.status == AssessmentStatus::Graded);
    let binance_perp = signal
        .active_contract_sources
        .iter()
        .any(|source| source.eq_ignore_ascii_case("binance"))
        || signal
            .main_exchange
            .as_deref()
            .is_some_and(|source| source.eq_ignore_ascii_case("binance"));
    let binance_spot = aligned_spot_evidence(signal);
    let source_strength = 100.0
        * ((if binance_perp { 1 } else { 0 })
            + (if binance_spot { 1 } else { 0 })
            + (if signal.classification_v2.oi_available {
                1
            } else {
                0
            })) as f64
        / 3.0;
    let attribution_completeness = attribution_completeness(signal);
    let mut confidence = (0.30 * f64::from(signal.data_quality)
        + 0.30 * f64::from(signal.classification_v2.intent_confidence)
        + 0.20 * impact_score
        + 0.10 * source_strength
        + 0.10 * attribution_completeness)
        .round()
        .clamp(0.0, 100.0) as u8;

    if !impact_graded {
        confidence = confidence.min(69);
        missing.push(evidence("v3_grade_not_confirmed", None));
    }
    if signal.classification_v2.evidence.evidence_degraded
        || signal.classification_v2.oi_evidence_degraded
        || signal.data_quality < 65
    {
        confidence = confidence.min(49);
        contradicting.push(evidence(
            "evidence_degraded",
            Some(f64::from(signal.data_quality)),
        ));
    }
    if signal.liquidation_suspected
        && signal.liquidation_long_btc <= 0.0
        && signal.liquidation_short_btc <= 0.0
    {
        confidence = confidence.min(59);
        contradicting.push(evidence("liquidation_inferred_only", None));
    }
    if matches!(hypothesis, ContractWhaleBehaviorHypothesis::Unclear) {
        confidence = confidence.min(39);
    }

    let follow_bps = signal
        .classification_v2
        .dynamic_thresholds
        .follow_pct
        .max(0.05)
        * 100.0;
    let decision_state = initial_decision_state(hypothesis, confidence, impact_graded);
    ContractWhaleBehaviorAssessment {
        behavior_version: CONTRACT_WHALE_BEHAVIOR_VERSION.to_string(),
        hypothesis,
        direction_bias,
        attribution,
        confidence_score: confidence,
        confidence_level: confidence_level(confidence),
        confidence_semantics: "heuristic_evidence_strength_not_probability".to_string(),
        decision_state,
        supporting_evidence: supporting,
        contradicting_evidence: contradicting,
        missing_evidence: missing,
        confirmation_rule: BehaviorRule {
            code: "closed_price_break_with_oi_and_binance_spot_perp_alignment".to_string(),
            threshold_bps: follow_bps,
        },
        invalidation_rule: BehaviorRule {
            code: "closed_price_reenters_event_range_or_evidence_diverges".to_string(),
            threshold_bps: -follow_bps,
        },
        post_event_validation: None,
        assessed_at_ms,
        outcome_evaluated_at_ms: None,
    }
}

pub fn apply_post_event_validation(
    mut assessment: ContractWhaleBehaviorAssessment,
    markouts: &[(String, Option<f64>)],
    evaluated_at_ms: i64,
) -> ContractWhaleBehaviorAssessment {
    if matches!(assessment.decision_state, BehaviorDecisionState::NoTrade) {
        return assessment;
    }
    let threshold = assessment.confirmation_rule.threshold_bps.abs();
    let latest = markouts.iter().rev().find_map(|(horizon, markout)| {
        markout
            .filter(|value| value.is_finite())
            .map(|value| (horizon.as_str(), value))
    });
    let Some((horizon, markout)) = latest else {
        return assessment;
    };
    let state = if markout >= threshold {
        BehaviorDecisionState::Confirmed
    } else if markout <= -threshold {
        BehaviorDecisionState::Invalidated
    } else if horizon == "5m" {
        BehaviorDecisionState::ExpiredUnconfirmed
    } else {
        BehaviorDecisionState::AwaitingConfirmation
    };
    // Price follow-through validates the price hypothesis, never missing actor evidence.
    let can_confirm = matches!(
        assessment.decision_state,
        BehaviorDecisionState::AwaitingConfirmation | BehaviorDecisionState::Confirmed
    ) && assessment.contradicting_evidence.is_empty()
        && ["oi_available", "spot_confirmation"].iter().all(|code| {
            assessment
                .supporting_evidence
                .iter()
                .any(|item| item.code == *code)
        });
    if (state != BehaviorDecisionState::Confirmed || can_confirm)
        && (assessment.decision_state != BehaviorDecisionState::Observe
            || state == BehaviorDecisionState::Invalidated)
    {
        assessment.decision_state = state;
    }
    assessment.post_event_validation = Some(BehaviorPostEventValidation {
        horizon: horizon.to_string(),
        markout_bps: markout,
        signed: true,
        state,
    });
    assessment.outcome_evaluated_at_ms = Some(evaluated_at_ms);
    assessment
}

fn classify(
    signal: &ContractWhaleSignal,
) -> (
    ContractWhaleBehaviorHypothesis,
    BehaviorDirectionBias,
    BehaviorAttribution,
) {
    let long_liq = signal.liquidation_long_btc.max(0.0);
    let short_liq = signal.liquidation_short_btc.max(0.0);
    let liquidation_total = long_liq + short_liq;
    let forced_ratio = if signal.total_volume_btc > 0.0 {
        liquidation_total / signal.total_volume_btc
    } else {
        0.0
    };
    if forced_ratio >= 0.25 && liquidation_total > 0.0 {
        if long_liq >= short_liq {
            return (
                ContractWhaleBehaviorHypothesis::LongLiquidationCascade,
                BehaviorDirectionBias::Bearish,
                BehaviorAttribution::ForcedFlow,
            );
        }
        return (
            ContractWhaleBehaviorHypothesis::ShortSqueeze,
            BehaviorDirectionBias::Bullish,
            BehaviorAttribution::ForcedFlow,
        );
    }

    match signal.classification_v2.structure_interpretation {
        ContractWhaleStructureInterpretation::DownsideAbsorption => {
            return (
                ContractWhaleBehaviorHypothesis::DownsideAbsorption,
                BehaviorDirectionBias::Bullish,
                BehaviorAttribution::PassiveAbsorption,
            );
        }
        ContractWhaleStructureInterpretation::UpsideSuppression => {
            return (
                ContractWhaleBehaviorHypothesis::UpsideSuppression,
                BehaviorDirectionBias::Bearish,
                BehaviorAttribution::PassiveAbsorption,
            );
        }
        _ => {}
    }

    let buy =
        signal.classification_v2.flow_direction == ContractWhaleActiveFlowDirection::BuyDominant;
    let sell =
        signal.classification_v2.flow_direction == ContractWhaleActiveFlowDirection::SellDominant;
    match (buy, sell, signal.classification_v2.oi_context) {
        (true, false, ContractWhaleOiContextTag::NewLongBuild) => (
            ContractWhaleBehaviorHypothesis::InitiativeLongBuild,
            BehaviorDirectionBias::Bullish,
            BehaviorAttribution::VoluntaryPositionBuild,
        ),
        (false, true, ContractWhaleOiContextTag::NewShortBuild) => (
            ContractWhaleBehaviorHypothesis::InitiativeShortBuild,
            BehaviorDirectionBias::Bearish,
            BehaviorAttribution::VoluntaryPositionBuild,
        ),
        (true, false, ContractWhaleOiContextTag::ShortCovering) => (
            ContractWhaleBehaviorHypothesis::ShortCovering,
            BehaviorDirectionBias::Bullish,
            BehaviorAttribution::PositionClose,
        ),
        (false, true, ContractWhaleOiContextTag::LongUnwind) => (
            ContractWhaleBehaviorHypothesis::LongUnwind,
            BehaviorDirectionBias::Bearish,
            BehaviorAttribution::PositionClose,
        ),
        (true, false, _) => (
            ContractWhaleBehaviorHypothesis::ActiveBuyPressure,
            BehaviorDirectionBias::Bullish,
            BehaviorAttribution::ActiveFlowUnattributed,
        ),
        (false, true, _) => (
            ContractWhaleBehaviorHypothesis::ActiveSellPressure,
            BehaviorDirectionBias::Bearish,
            BehaviorAttribution::ActiveFlowUnattributed,
        ),
        _ => (
            ContractWhaleBehaviorHypothesis::Unclear,
            BehaviorDirectionBias::Unknown,
            BehaviorAttribution::Unclear,
        ),
    }
}

fn initial_decision_state(
    hypothesis: ContractWhaleBehaviorHypothesis,
    confidence: u8,
    impact_graded: bool,
) -> BehaviorDecisionState {
    if matches!(hypothesis, ContractWhaleBehaviorHypothesis::Unclear)
        || matches!(
            hypothesis,
            ContractWhaleBehaviorHypothesis::LongLiquidationCascade
                | ContractWhaleBehaviorHypothesis::ShortSqueeze
        )
    {
        return BehaviorDecisionState::NoTrade;
    }
    if !impact_graded || confidence < 60 {
        BehaviorDecisionState::Observe
    } else {
        BehaviorDecisionState::AwaitingConfirmation
    }
}

fn attribution_completeness(signal: &ContractWhaleSignal) -> f64 {
    let oi = f64::from(signal.classification_v2.oi_available) * 35.0;
    let liquidation = if signal.liquidation_long_btc > 0.0 || signal.liquidation_short_btc > 0.0 {
        25.0
    } else {
        0.0
    };
    let price = if signal.price_response_type != ContractWhalePriceResponseType::NoClearResponse {
        25.0
    } else {
        0.0
    };
    let spot = if aligned_spot_evidence(signal) {
        15.0
    } else {
        0.0
    };
    oi + liquidation + price + spot
}

fn add_signal_evidence(
    signal: &ContractWhaleSignal,
    hypothesis: &ContractWhaleBehaviorHypothesis,
    supporting: &mut Vec<BehaviorEvidenceItem>,
    contradicting: &mut Vec<BehaviorEvidenceItem>,
    missing: &mut Vec<BehaviorEvidenceItem>,
) {
    if signal
        .active_contract_sources
        .iter()
        .any(|source| source.eq_ignore_ascii_case("binance"))
        || signal
            .main_exchange
            .as_deref()
            .is_some_and(|source| source.eq_ignore_ascii_case("binance"))
    {
        supporting.push(evidence("binance_perp_flow", Some(signal.net_volume_btc)));
    } else {
        missing.push(evidence("binance_perp_flow_unavailable", None));
    }
    if aligned_spot_evidence(signal) {
        supporting.push(evidence(
            "binance_spot_flow",
            Some(f64::from(signal.spot_confirmation.score)),
        ));
    } else {
        missing.push(evidence("binance_spot_flow_unavailable", None));
    }
    if signal.classification_v2.oi_available {
        supporting.push(evidence(
            "oi_available",
            signal.classification_v2.oi_delta_pct,
        ));
    } else {
        missing.push(evidence("oi_unavailable", None));
    }
    if signal.classification_v2.price_response_type_v2
        != ContractWhalePriceResponseType::NoClearResponse
    {
        supporting.push(evidence("price_response_classified", signal.price_move_pct));
    } else {
        missing.push(evidence("price_response_unclear", None));
    }
    if aligned_spot_evidence(signal) {
        supporting.push(evidence(
            "spot_confirmation",
            Some(f64::from(signal.spot_confirmation.score)),
        ));
    } else if signal.spot_confirmation.status == "divergent" {
        contradicting.push(evidence(
            "spot_divergence",
            Some(f64::from(signal.spot_confirmation.score)),
        ));
    } else {
        missing.push(evidence("spot_confirmation", None));
    }
    match hypothesis {
        ContractWhaleBehaviorHypothesis::DownsideAbsorption => {
            supporting.push(evidence("sell_flow_absorbed", None));
            supporting.push(evidence(
                "low_price_efficiency",
                Some(signal.classification_v2.price_efficiency),
            ));
        }
        ContractWhaleBehaviorHypothesis::UpsideSuppression => {
            supporting.push(evidence("buy_flow_suppressed", None));
            supporting.push(evidence(
                "low_price_efficiency",
                Some(signal.classification_v2.price_efficiency),
            ));
        }
        ContractWhaleBehaviorHypothesis::LongLiquidationCascade
        | ContractWhaleBehaviorHypothesis::ShortSqueeze => {
            supporting.push(evidence(
                "direct_liquidation_volume",
                Some(signal.liquidation_notional_usd),
            ));
        }
        _ => {}
    }
    if signal.classification_v2.oi_evidence_degraded {
        contradicting.push(evidence("oi_evidence_degraded", None));
    }
}

fn aligned_spot_evidence(signal: &ContractWhaleSignal) -> bool {
    signal.spot_confirmation.status == "confirmed"
        && signal.spot_confirmation.confirmation_type == "confirms_contract_direction"
        && signal.spot_confirmation.score > 0
        && signal
            .spot_confirmation
            .latest_signal_at
            .is_some_and(|ts| ts <= signal.ts && signal.ts.saturating_sub(ts) <= 60_000)
}

fn confidence_level(score: u8) -> BehaviorConfidenceLevel {
    match score {
        0..=49 => BehaviorConfidenceLevel::Low,
        50..=74 => BehaviorConfidenceLevel::Medium,
        _ => BehaviorConfidenceLevel::High,
    }
}

fn evidence(code: &str, value: Option<f64>) -> BehaviorEvidenceItem {
    BehaviorEvidenceItem {
        code: code.to_string(),
        value,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::contract_whale_monitor::types::ContractWhaleSignal;

    fn signal() -> ContractWhaleSignal {
        let value = serde_json::json!({
            "id":"test",
            "ts":1,
            "symbol":"BTC",
            "windowSec":15,
            "signalType":"aggressive_sell",
            "direction":"sell",
            "severity":"high",
            "score":80,
            "totalVolumeBtc":100.0,
            "netVolumeBtc":-80.0,
            "totalNotionalUsd":8000000.0,
            "dominance":0.8,
            "priceMovePct":-0.2,
            "priceResponseType":"trend_follow_down",
            "mainExchange":null,
            "exchanges":[],
            "dataQuality":90,
            "exchanges":[],
            "discordEligible":false,
            "discordSent":false,
            "discordReason":"test",
            "finalResult":"test",
            "readOnly":true,
            "analysisOnly":true,
            "executionEnabled":false,
            "flowDirection":"sell_dominant",
            "oiContext":"new_short_build",
            "oiContextLabel":"新空开仓",
            "oiAvailable":true,
            "intentConfidence":80,
            "priceResponseTypeV2":"trend_follow_down",
            "priceEfficiency":0.8,
            "dynamicThresholds":{"followPct":0.12},
            "spotConfirmation":{"score":0},
            "activeContractSources":["binance"]
        });
        serde_json::from_value(value).expect("test signal")
    }

    #[test]
    fn classifies_new_short_build() {
        let result = build_detection_behavior(&signal(), None, 10);
        assert_eq!(
            result.hypothesis,
            ContractWhaleBehaviorHypothesis::InitiativeShortBuild
        );
        assert_eq!(result.direction_bias, BehaviorDirectionBias::Bearish);
        assert_eq!(result.decision_state, BehaviorDecisionState::Observe);
    }

    #[test]
    fn price_follow_through_does_not_confirm_missing_behavior_evidence() {
        let original = build_detection_behavior(&signal(), None, 10);
        let result =
            apply_post_event_validation(original.clone(), &[("30s".to_string(), Some(20.0))], 40);
        assert_eq!(result.decision_state, BehaviorDecisionState::Observe);
        assert_eq!(
            result.post_event_validation.as_ref().unwrap().state,
            BehaviorDecisionState::Confirmed
        );
        assert_eq!(result.hypothesis, original.hypothesis);
        assert_eq!(result.confidence_score, original.confidence_score);
    }

    #[test]
    fn high_quality_buy_without_oi_is_only_active_pressure() {
        let mut value = serde_json::to_value(signal()).expect("serialize");
        value["flowDirection"] = serde_json::json!("buy_dominant");
        value["oiContext"] = serde_json::json!("oi_unavailable");
        value["oiAvailable"] = serde_json::json!(false);
        let signal: ContractWhaleSignal = serde_json::from_value(value).expect("signal");
        let result = build_detection_behavior(&signal, None, 10);
        assert_eq!(
            result.hypothesis,
            ContractWhaleBehaviorHypothesis::ActiveBuyPressure
        );
    }
}
