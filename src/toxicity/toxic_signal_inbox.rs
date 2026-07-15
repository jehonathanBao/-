use serde::Serialize;

use crate::types::{
    toxic_flow::ToxicConfidence,
    toxic_governance_ledger::{ToxicGovernanceDecisionKind, ToxicGovernanceLedgerSummaryResponse},
    toxic_markout::{ToxicMarkoutOutcome, ToxicMarkoutRecentResponse, ToxicMarkoutSignal},
    toxic_quality_scorecard::{ToxicQualityScorecardBucket, ToxicQualityScorecardSummaryResponse},
    toxic_replay::ToxicReplayRecentResponse,
    toxic_signal::{ToxicSignal, ToxicSignalDirection, ToxicSignalRecentResponse},
    toxic_signal_inbox::{
        ToxicSignalInboxDetailResponse, ToxicSignalInboxFusionSummary,
        ToxicSignalInboxGovernanceSummary, ToxicSignalInboxItem, ToxicSignalInboxMarkoutSummary,
        ToxicSignalInboxOperatorAction, ToxicSignalInboxQualitySummary,
        ToxicSignalInboxRecentResponse, ToxicSignalInboxRecommendationSummary,
        ToxicSignalInboxReplaySummary, ToxicSignalInboxStatusResponse,
    },
    toxic_weight_recommendation::{
        ToxicWeightRecommendationItem, ToxicWeightRecommendationKind,
        ToxicWeightRecommendationSummaryResponse,
    },
};

pub fn build_toxic_signal_inbox_recent(
    requested_symbol: &str,
    fusion_recent: &ToxicSignalRecentResponse,
    replay_recent: &ToxicReplayRecentResponse,
    markout_recent: &ToxicMarkoutRecentResponse,
    quality_summary: &ToxicQualityScorecardSummaryResponse,
    recommendation_summary: &ToxicWeightRecommendationSummaryResponse,
    governance_summary: &ToxicGovernanceLedgerSummaryResponse,
) -> ToxicSignalInboxRecentResponse {
    let items = fusion_recent
        .signals
        .iter()
        .filter(|signal| signal.symbol.eq_ignore_ascii_case(requested_symbol))
        .map(|signal| {
            build_item(
                signal,
                fusion_recent,
                replay_recent,
                markout_recent,
                quality_summary,
                recommendation_summary,
                governance_summary,
            )
        })
        .collect::<Vec<_>>();

    ToxicSignalInboxRecentResponse {
        read_only: fusion_recent.read_only,
        runtime_modified: fusion_recent.runtime_modified,
        analysis_only: fusion_recent.analysis_only,
        execution_enabled: fusion_recent.execution_enabled,
        manual_review_required: true,
        runtime_weight_modified: false,
        config_modified: false,
        mode: fusion_recent.mode.clone(),
        selected_symbol: requested_symbol.to_string(),
        status: if items.is_empty() {
            "empty_signal_inbox".to_string()
        } else {
            "signal_inbox_ready".to_string()
        },
        warnings: collect_warnings(
            fusion_recent,
            markout_recent,
            quality_summary,
            recommendation_summary,
            governance_summary,
        ),
        items,
    }
}

pub fn build_toxic_signal_inbox_status(
    recent: &ToxicSignalInboxRecentResponse,
) -> ToxicSignalInboxStatusResponse {
    ToxicSignalInboxStatusResponse {
        read_only: recent.read_only,
        runtime_modified: recent.runtime_modified,
        analysis_only: recent.analysis_only,
        execution_enabled: recent.execution_enabled,
        manual_review_required: true,
        runtime_weight_modified: false,
        config_modified: false,
        enabled: true,
        mode: recent.mode.clone(),
        selected_symbol: recent.selected_symbol.clone(),
        status: recent.status.clone(),
        item_count: recent.items.len(),
        last_signal_at_ms: recent.items.iter().map(|item| item.created_at_ms).max(),
        safety_boundary: vec![
            format!("readOnly={}", recent.read_only),
            format!("runtimeModified={}", recent.runtime_modified),
            format!("analysisOnly={}", recent.analysis_only),
            format!("executionEnabled={}", recent.execution_enabled),
            "manualReviewRequired=true".to_string(),
            "No order placement".to_string(),
            "No cancel/amend".to_string(),
            "No wallet/signing".to_string(),
            "No transaction construction".to_string(),
            "No live trading".to_string(),
        ],
    }
}

pub fn build_toxic_signal_inbox_detail(
    requested_symbol: &str,
    signal_id: &str,
    recent: &ToxicSignalInboxRecentResponse,
) -> ToxicSignalInboxDetailResponse {
    let item = recent
        .items
        .iter()
        .find(|item| item.signal_id == signal_id)
        .cloned();
    ToxicSignalInboxDetailResponse {
        read_only: recent.read_only,
        runtime_modified: recent.runtime_modified,
        analysis_only: recent.analysis_only,
        execution_enabled: recent.execution_enabled,
        manual_review_required: true,
        runtime_weight_modified: false,
        config_modified: false,
        mode: recent.mode.clone(),
        selected_symbol: requested_symbol.to_string(),
        available: item.is_some(),
        reason: item
            .is_none()
            .then(|| "signal_id_not_found_in_read_only_inbox".to_string()),
        item,
    }
}

fn build_item(
    signal: &ToxicSignal,
    fusion_recent: &ToxicSignalRecentResponse,
    replay_recent: &ToxicReplayRecentResponse,
    markout_recent: &ToxicMarkoutRecentResponse,
    quality_summary: &ToxicQualityScorecardSummaryResponse,
    recommendation_summary: &ToxicWeightRecommendationSummaryResponse,
    governance_summary: &ToxicGovernanceLedgerSummaryResponse,
) -> ToxicSignalInboxItem {
    let signal_kind = snake_label(&signal.signal_type);
    let markout = markout_recent
        .signals
        .iter()
        .find(|markout| markout.signal_id == signal.signal_id);
    let quality_bucket = quality_summary
        .by_signal_type
        .iter()
        .find(|bucket| bucket.key == signal_kind);
    let recommendation = recommendation_summary
        .recommendations
        .iter()
        .find(|item| item.signal_type == signal_kind);
    let governance = governance_summary
        .decisions
        .iter()
        .filter(|decision| {
            decision.symbol.eq_ignore_ascii_case(&signal.symbol)
                && decision.signal_type == signal_kind
        })
        .max_by_key(|decision| decision.created_at_ms);

    let markout_summary = build_markout_summary(markout);
    let quality = build_quality_summary(quality_bucket);
    let recommendation = build_recommendation_summary(recommendation);
    let governance = build_governance_summary(governance);
    let operator_action =
        operator_action_for(&markout_summary, &quality, &recommendation, &governance);

    ToxicSignalInboxItem {
        signal_id: signal.signal_id.clone(),
        symbol: signal.symbol.clone(),
        signal_kind: signal_kind.clone(),
        direction_bias: direction_bias(signal.direction),
        severity: severity_for(signal.toxicity_score),
        risk_score: signal.toxicity_score,
        data_quality_score: signal.data_quality,
        confidence: toxic_confidence_score(signal.confidence),
        created_at_ms: signal.ts_ms,
        fusion: ToxicSignalInboxFusionSummary {
            available: true,
            summary: signal.primary_reason.clone(),
        },
        replay: ToxicSignalInboxReplaySummary {
            available: replay_recent
                .signals
                .iter()
                .any(|summary| summary.signal_id == signal.signal_id),
            evidence_count: signal.supporting_evidence.len(),
        },
        markout: markout_summary,
        quality,
        recommendation,
        governance,
        operator_action,
        read_only: fusion_recent.read_only,
        runtime_modified: fusion_recent.runtime_modified,
        analysis_only: fusion_recent.analysis_only,
        execution_enabled: fusion_recent.execution_enabled,
    }
}

fn build_markout_summary(markout: Option<&ToxicMarkoutSignal>) -> ToxicSignalInboxMarkoutSummary {
    let mut summary = ToxicSignalInboxMarkoutSummary {
        available: markout.is_some(),
        one_minute: "not_enough_data".to_string(),
        five_minute: "not_enough_data".to_string(),
        fifteen_minute: "not_enough_data".to_string(),
        one_hour: "not_enough_data".to_string(),
    };

    if let Some(markout) = markout {
        for window in &markout.windows {
            let outcome = outcome_label(window.outcome);
            match window.label.as_str() {
                "+1m" => summary.one_minute = outcome,
                "+5m" => summary.five_minute = outcome,
                "+15m" => summary.fifteen_minute = outcome,
                "+1h" => summary.one_hour = outcome,
                _ => {}
            }
        }
    }

    summary
}

fn build_quality_summary(
    bucket: Option<&ToxicQualityScorecardBucket>,
) -> ToxicSignalInboxQualitySummary {
    match bucket {
        Some(bucket) => ToxicSignalInboxQualitySummary {
            available: true,
            quality_bucket: quality_bucket_label(bucket),
            aligned_ratio: bucket.aligned_ratio,
            adverse_ratio: bucket.adverse_ratio,
        },
        None => ToxicSignalInboxQualitySummary {
            available: false,
            quality_bucket: "not_enough_data".to_string(),
            aligned_ratio: 0.0,
            adverse_ratio: 0.0,
        },
    }
}

fn build_recommendation_summary(
    recommendation: Option<&ToxicWeightRecommendationItem>,
) -> ToxicSignalInboxRecommendationSummary {
    match recommendation {
        Some(recommendation) => ToxicSignalInboxRecommendationSummary {
            available: true,
            action: snake_label(&recommendation.recommendation),
            no_trade_only: recommendation.recommendation
                == ToxicWeightRecommendationKind::NoTradeOnlyCandidate,
            manual_review_required: recommendation.manual_review_required,
        },
        None => ToxicSignalInboxRecommendationSummary {
            available: false,
            action: "insufficient_data".to_string(),
            no_trade_only: false,
            manual_review_required: true,
        },
    }
}

fn build_governance_summary(
    decision: Option<&crate::types::toxic_governance_ledger::ToxicGovernanceDecision>,
) -> ToxicSignalInboxGovernanceSummary {
    match decision {
        Some(decision) => ToxicSignalInboxGovernanceSummary {
            ledger_available: true,
            latest_decision: decision_label(decision.decision),
        },
        None => ToxicSignalInboxGovernanceSummary {
            ledger_available: false,
            latest_decision: "missing_ledger_evidence".to_string(),
        },
    }
}

fn operator_action_for(
    markout: &ToxicSignalInboxMarkoutSummary,
    quality: &ToxicSignalInboxQualitySummary,
    recommendation: &ToxicSignalInboxRecommendationSummary,
    governance: &ToxicSignalInboxGovernanceSummary,
) -> ToxicSignalInboxOperatorAction {
    if recommendation.no_trade_only {
        return ToxicSignalInboxOperatorAction::NoTradeWarning;
    }
    if !quality.available || quality.quality_bucket == "not_enough_data" {
        return ToxicSignalInboxOperatorAction::NeedsMoreData;
    }
    if markout.one_minute == "not_enough_data"
        || markout.five_minute == "not_enough_data"
        || markout.fifteen_minute == "not_enough_data"
        || markout.one_hour == "not_enough_data"
    {
        return ToxicSignalInboxOperatorAction::ReviewMarkout;
    }
    if !governance.ledger_available {
        return ToxicSignalInboxOperatorAction::ReviewEvidence;
    }
    if matches!(quality.quality_bucket.as_str(), "bad" | "weak" | "mixed") {
        return ToxicSignalInboxOperatorAction::ReviewQuality;
    }
    ToxicSignalInboxOperatorAction::WatchSignalOnly
}

fn collect_warnings(
    fusion_recent: &ToxicSignalRecentResponse,
    markout_recent: &ToxicMarkoutRecentResponse,
    quality_summary: &ToxicQualityScorecardSummaryResponse,
    recommendation_summary: &ToxicWeightRecommendationSummaryResponse,
    governance_summary: &ToxicGovernanceLedgerSummaryResponse,
) -> Vec<String> {
    let mut warnings = Vec::new();
    warnings.extend(fusion_recent.warnings.clone());
    warnings.extend(markout_recent.warnings.clone());
    warnings.extend(quality_summary.warnings.clone());
    warnings.extend(recommendation_summary.warnings.clone());
    warnings.extend(governance_summary.warnings.clone());
    warnings.sort();
    warnings.dedup();
    warnings
}

fn quality_bucket_label(bucket: &ToxicQualityScorecardBucket) -> String {
    if bucket.total_evaluations == 0 {
        "not_enough_data".to_string()
    } else if bucket.aligned_ratio >= 0.70 && bucket.adverse_ratio <= 0.15 {
        "excellent".to_string()
    } else if bucket.aligned_ratio >= 0.58 && bucket.adverse_ratio <= 0.25 {
        "good".to_string()
    } else if bucket.aligned_ratio >= 0.45 && bucket.adverse_ratio <= 0.35 {
        "mixed".to_string()
    } else if bucket.adverse_ratio > 0.45 {
        "bad".to_string()
    } else {
        "weak".to_string()
    }
}

fn outcome_label(outcome: ToxicMarkoutOutcome) -> String {
    snake_label(&outcome)
}

fn decision_label(decision: ToxicGovernanceDecisionKind) -> String {
    snake_label(&decision)
}

fn direction_bias(direction: ToxicSignalDirection) -> String {
    match direction {
        ToxicSignalDirection::ShortBias => "short_bias".to_string(),
        ToxicSignalDirection::LongBias => "long_bias".to_string(),
        ToxicSignalDirection::TrapRisk => "trap_risk".to_string(),
        ToxicSignalDirection::Neutral => "neutral".to_string(),
    }
}

fn severity_for(score: u8) -> String {
    if score >= 85 {
        "high".to_string()
    } else if score >= 70 {
        "medium".to_string()
    } else {
        "low".to_string()
    }
}

pub fn toxic_confidence_score(confidence: ToxicConfidence) -> f64 {
    match confidence {
        ToxicConfidence::Low => 0.35,
        ToxicConfidence::Medium => 0.62,
        ToxicConfidence::High => 0.82,
    }
}

fn snake_label<T>(value: &T) -> String
where
    T: Serialize,
{
    serde_json::to_value(value)
        .ok()
        .and_then(|value| value.as_str().map(ToOwned::to_owned))
        .unwrap_or_else(|| "unknown".to_string())
}
