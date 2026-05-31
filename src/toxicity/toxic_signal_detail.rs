use crate::types::{
    toxic_governance_ledger::{ToxicGovernanceDecision, ToxicGovernanceLedgerSummaryResponse},
    toxic_markout::{ToxicMarkoutRecentResponse, ToxicMarkoutSignal},
    toxic_quality_scorecard::{ToxicQualityScorecardBucket, ToxicQualityScorecardSummaryResponse},
    toxic_replay::ToxicReplayRecentResponse,
    toxic_signal::ToxicSignalRecentResponse,
    toxic_signal_detail::{
        ToxicSignalDetailEvidence, ToxicSignalDetailGroupResponse, ToxicSignalDetailMemberSummary,
        ToxicSignalDetailOperatorAction, ToxicSignalDetailOperatorNarrative,
        ToxicSignalDetailPayload, ToxicSignalDetailResponse, ToxicSignalDetailSource,
        ToxicSignalDetailStatusResponse, ToxicSignalDetailTimelineStage,
        ToxicSignalGroupDrilldownPayload,
    },
    toxic_signal_group::{ToxicSignalGroup, ToxicSignalGroupRecentResponse},
    toxic_signal_inbox::{
        ToxicSignalInboxItem, ToxicSignalInboxOperatorAction, ToxicSignalInboxRecentResponse,
    },
    toxic_weight_recommendation::{
        ToxicWeightRecommendationItem, ToxicWeightRecommendationSummaryResponse,
    },
};

pub struct ToxicSignalDetailContext<'a> {
    pub fusion_recent: &'a ToxicSignalRecentResponse,
    pub replay_recent: &'a ToxicReplayRecentResponse,
    pub markout_recent: &'a ToxicMarkoutRecentResponse,
    pub quality_summary: &'a ToxicQualityScorecardSummaryResponse,
    pub recommendation_summary: &'a ToxicWeightRecommendationSummaryResponse,
    pub governance_summary: &'a ToxicGovernanceLedgerSummaryResponse,
    pub inbox_recent: &'a ToxicSignalInboxRecentResponse,
    pub group_recent: &'a ToxicSignalGroupRecentResponse,
}

pub fn build_toxic_signal_detail_status(
    requested_symbol: &str,
    context: &ToxicSignalDetailContext<'_>,
) -> ToxicSignalDetailStatusResponse {
    ToxicSignalDetailStatusResponse {
        read_only: true,
        runtime_modified: false,
        analysis_only: true,
        execution_enabled: false,
        manual_review_required: true,
        runtime_weight_modified: false,
        config_modified: false,
        enabled: true,
        mode: "analysis_only".to_string(),
        selected_symbol: requested_symbol.to_string(),
        status: if context.inbox_recent.items.is_empty() {
            "empty_signal_detail".to_string()
        } else {
            "signal_detail_ready".to_string()
        },
        signal_count: context.inbox_recent.items.len(),
        group_count: context.group_recent.groups.len(),
        last_signal_at_ms: context
            .inbox_recent
            .items
            .iter()
            .map(|item| item.created_at_ms)
            .max(),
        safety_boundary: vec![
            "readOnly=true".to_string(),
            "runtimeModified=false".to_string(),
            "analysisOnly=true".to_string(),
            "executionEnabled=false".to_string(),
            "Manual review required".to_string(),
            "No order placement".to_string(),
            "No cancel/amend".to_string(),
            "No wallet/signing".to_string(),
            "No transaction construction".to_string(),
            "No live trading".to_string(),
        ],
    }
}

pub fn build_toxic_signal_detail(
    requested_symbol: &str,
    signal_id: &str,
    context: &ToxicSignalDetailContext<'_>,
) -> ToxicSignalDetailResponse {
    let detail = context
        .inbox_recent
        .items
        .iter()
        .find(|item| item.signal_id == signal_id)
        .map(|item| build_signal_payload(item, context));

    ToxicSignalDetailResponse {
        read_only: true,
        runtime_modified: false,
        analysis_only: true,
        execution_enabled: false,
        manual_review_required: true,
        runtime_weight_modified: false,
        config_modified: false,
        mode: "analysis_only".to_string(),
        selected_symbol: requested_symbol.to_string(),
        available: detail.is_some(),
        reason: detail
            .is_none()
            .then(|| "signal_id_not_found_in_read_only_signal_detail".to_string()),
        detail,
    }
}

pub fn build_toxic_signal_group_detail(
    requested_symbol: &str,
    group_id: &str,
    context: &ToxicSignalDetailContext<'_>,
) -> ToxicSignalDetailGroupResponse {
    let detail = context
        .group_recent
        .groups
        .iter()
        .find(|group| group.group_id == group_id)
        .and_then(|group| {
            context
                .inbox_recent
                .items
                .iter()
                .find(|item| item.signal_id == group.representative_signal_id)
                .map(|item| ToxicSignalGroupDrilldownPayload {
                    representative_signal: build_signal_payload(item, context),
                    group: group.clone(),
                    members: group
                        .member_signal_ids
                        .iter()
                        .filter_map(|signal_id| {
                            context
                                .inbox_recent
                                .items
                                .iter()
                                .find(|item| item.signal_id == *signal_id)
                                .map(build_member_summary)
                        })
                        .collect(),
                })
        });

    ToxicSignalDetailGroupResponse {
        read_only: true,
        runtime_modified: false,
        analysis_only: true,
        execution_enabled: false,
        manual_review_required: true,
        runtime_weight_modified: false,
        config_modified: false,
        mode: "analysis_only".to_string(),
        selected_symbol: requested_symbol.to_string(),
        available: detail.is_some(),
        reason: detail
            .is_none()
            .then(|| "group_id_not_found_in_read_only_signal_detail".to_string()),
        detail,
    }
}

fn build_signal_payload(
    item: &ToxicSignalInboxItem,
    context: &ToxicSignalDetailContext<'_>,
) -> ToxicSignalDetailPayload {
    let fusion = context
        .fusion_recent
        .signals
        .iter()
        .find(|signal| signal.signal_id == item.signal_id)
        .cloned();
    let replay = context
        .replay_recent
        .signals
        .iter()
        .find(|signal| signal.signal_id == item.signal_id)
        .cloned();
    let markout = context
        .markout_recent
        .signals
        .iter()
        .find(|signal| signal.signal_id == item.signal_id)
        .cloned();
    let quality = context
        .quality_summary
        .by_signal_type
        .iter()
        .find(|bucket| bucket.key == item.signal_kind)
        .cloned();
    let recommendation = context
        .recommendation_summary
        .recommendations
        .iter()
        .find(|candidate| candidate.signal_type == item.signal_kind)
        .cloned();
    let governance = latest_governance_decision(context.governance_summary, item);
    let group = context
        .group_recent
        .groups
        .iter()
        .find(|group| {
            group
                .member_signal_ids
                .iter()
                .any(|member| member == &item.signal_id)
        })
        .cloned();
    let evidence = ToxicSignalDetailEvidence {
        fusion,
        replay,
        markout,
        quality: quality.clone(),
        recommendation: recommendation.clone(),
        governance,
    };

    ToxicSignalDetailPayload {
        signal_id: item.signal_id.clone(),
        symbol: item.symbol.clone(),
        signal_kind: item.signal_kind.clone(),
        direction_bias: item.direction_bias.clone(),
        severity: item.severity.clone(),
        confidence: item.confidence,
        created_at_ms: item.created_at_ms,
        source: ToxicSignalDetailSource {
            inbox_available: true,
            group_available: group.is_some(),
            group_id: group.as_ref().map(|entry| entry.group_id.clone()),
        },
        timeline: build_timeline(item, &evidence, group.as_ref()),
        evidence,
        operator_narrative: build_operator_narrative(
            item,
            quality.as_ref(),
            recommendation.as_ref(),
            group.as_ref(),
        ),
        operator_action: map_operator_action(item.operator_action),
        no_execution_reason: "Signal-only analysis. No trading action is available.".to_string(),
    }
}

fn build_timeline(
    item: &ToxicSignalInboxItem,
    evidence: &ToxicSignalDetailEvidence,
    group: Option<&ToxicSignalGroup>,
) -> Vec<ToxicSignalDetailTimelineStage> {
    vec![
        ToxicSignalDetailTimelineStage {
            stage: "grouping".to_string(),
            label: "S2 Signal Groups".to_string(),
            available: group.is_some(),
            summary: group.map_or_else(
                || "No grouped burst context for this signal.".to_string(),
                |group| {
                    format!(
                        "Grouped burst summary available. Count={} within {}ms cooldown.",
                        group.count, group.cooldown_window_ms
                    )
                },
            ),
            timestamp_ms: group.map_or(item.created_at_ms, |group| group.last_seen_at_ms),
        },
        ToxicSignalDetailTimelineStage {
            stage: "fusion".to_string(),
            label: "T6 Fusion".to_string(),
            available: evidence.fusion.is_some(),
            summary: evidence
                .fusion
                .as_ref()
                .map(|signal| signal.primary_reason.clone())
                .unwrap_or_else(|| "Fused toxic flow is unavailable.".to_string()),
            timestamp_ms: evidence
                .fusion
                .as_ref()
                .map_or(item.created_at_ms, |signal| signal.ts_ms),
        },
        ToxicSignalDetailTimelineStage {
            stage: "replay".to_string(),
            label: "T7 Replay / Evidence".to_string(),
            available: evidence.replay.is_some(),
            summary: evidence.replay.as_ref().map_or_else(
                || "Evidence breakdown unavailable.".to_string(),
                |replay| {
                    format!(
                        "Evidence breakdown available. Severity={} confidence={:.2}.",
                        replay.severity, replay.confidence
                    )
                },
            ),
            timestamp_ms: evidence
                .replay
                .as_ref()
                .map_or(item.created_at_ms, |replay| replay.created_at),
        },
        ToxicSignalDetailTimelineStage {
            stage: "markout".to_string(),
            label: "T8 Markout".to_string(),
            available: evidence.markout.is_some(),
            summary: evidence.markout.as_ref().map_or_else(
                || "Markout evaluation unavailable.".to_string(),
                markout_summary,
            ),
            timestamp_ms: evidence
                .markout
                .as_ref()
                .map_or(item.created_at_ms, |markout| markout.created_at_ms),
        },
        ToxicSignalDetailTimelineStage {
            stage: "quality".to_string(),
            label: "T9 Quality".to_string(),
            available: evidence.quality.is_some(),
            summary: evidence.quality.as_ref().map_or_else(
                || "Quality scorecard unavailable.".to_string(),
                |quality| format!("Quality bucket: {}.", quality.label),
            ),
            timestamp_ms: item.created_at_ms,
        },
        ToxicSignalDetailTimelineStage {
            stage: "recommendation".to_string(),
            label: "T10 Recommendation".to_string(),
            available: evidence.recommendation.is_some(),
            summary: evidence.recommendation.as_ref().map_or_else(
                || "No recommendation available.".to_string(),
                |recommendation| {
                    format!(
                        "Recommendation: {}.",
                        snake_label(&recommendation.recommendation)
                    )
                },
            ),
            timestamp_ms: item.created_at_ms,
        },
        ToxicSignalDetailTimelineStage {
            stage: "governance".to_string(),
            label: "T12 Governance Ledger".to_string(),
            available: evidence.governance.is_some(),
            summary: evidence.governance.as_ref().map_or_else(
                || "No governance ledger entry available yet.".to_string(),
                |governance| {
                    format!(
                        "Governance decision: {}.",
                        snake_label(&governance.decision)
                    )
                },
            ),
            timestamp_ms: evidence
                .governance
                .as_ref()
                .map_or(0, |governance| governance.created_at_ms),
        },
    ]
}

fn build_operator_narrative(
    item: &ToxicSignalInboxItem,
    quality: Option<&ToxicQualityScorecardBucket>,
    recommendation: Option<&ToxicWeightRecommendationItem>,
    group: Option<&ToxicSignalGroup>,
) -> ToxicSignalDetailOperatorNarrative {
    let mut what_confirmed_it = Vec::new();
    what_confirmed_it.push(item.fusion.summary.clone());
    if item.replay.available {
        what_confirmed_it.push(format!(
            "Replay evidence count: {}.",
            item.replay.evidence_count
        ));
    }
    if let Some(group) = group {
        what_confirmed_it.push(format!(
            "Grouped burst context preserved with {} member signals.",
            group.count
        ));
    }

    let mut what_conflicted = Vec::new();
    if item.markout.one_minute == "not_enough_data"
        || item.markout.five_minute == "not_enough_data"
        || item.markout.fifteen_minute == "not_enough_data"
        || item.markout.one_hour == "not_enough_data"
    {
        what_conflicted.push(
            "Markout still contains not_enough_data windows and cannot be overstated.".to_string(),
        );
    }
    if let Some(quality) = quality {
        if quality.no_trade_candidate {
            what_conflicted.push(
                "Quality scorecard currently flags this signal as a no-trade candidate."
                    .to_string(),
            );
        } else if quality.downgrade_candidate {
            what_conflicted.push(
                "Quality scorecard currently flags this signal as a downgrade candidate."
                    .to_string(),
            );
        }
    }
    if let Some(recommendation) = recommendation {
        if recommendation.recommendation
            == crate::types::toxic_weight_recommendation::ToxicWeightRecommendationKind::NoTradeOnlyCandidate
        {
            what_conflicted.push(
                "Recommendation layer keeps this signal in no-trade-only territory."
                    .to_string(),
            );
        }
    }
    if !item.governance.ledger_available {
        what_conflicted.push("Governance ledger entry is not available yet.".to_string());
    }

    ToxicSignalDetailOperatorNarrative {
        why_signal_fired: vec![item.fusion.summary.clone()],
        what_confirmed_it,
        what_conflicted,
        why_no_execution: vec![
            "Signal only. No order placement.".to_string(),
            "Execution is disabled.".to_string(),
            "Manual review required.".to_string(),
        ],
    }
}

fn build_member_summary(item: &ToxicSignalInboxItem) -> ToxicSignalDetailMemberSummary {
    ToxicSignalDetailMemberSummary {
        signal_id: item.signal_id.clone(),
        severity: item.severity.clone(),
        confidence: item.confidence,
        created_at_ms: item.created_at_ms,
        operator_action: map_operator_action(item.operator_action),
    }
}

fn latest_governance_decision(
    governance_summary: &ToxicGovernanceLedgerSummaryResponse,
    item: &ToxicSignalInboxItem,
) -> Option<ToxicGovernanceDecision> {
    governance_summary
        .decisions
        .iter()
        .filter(|decision| {
            decision.symbol.eq_ignore_ascii_case(&item.symbol)
                && decision.signal_type == item.signal_kind
        })
        .max_by_key(|decision| decision.created_at_ms)
        .cloned()
}

fn map_operator_action(action: ToxicSignalInboxOperatorAction) -> ToxicSignalDetailOperatorAction {
    match action {
        ToxicSignalInboxOperatorAction::ReviewEvidence => {
            ToxicSignalDetailOperatorAction::ReviewEvidence
        }
        ToxicSignalInboxOperatorAction::ReviewMarkout => {
            ToxicSignalDetailOperatorAction::ReviewMarkout
        }
        ToxicSignalInboxOperatorAction::ReviewQuality => {
            ToxicSignalDetailOperatorAction::ReviewQuality
        }
        ToxicSignalInboxOperatorAction::WatchSignalOnly => {
            ToxicSignalDetailOperatorAction::WatchSignalOnly
        }
        ToxicSignalInboxOperatorAction::NoTradeWarning => {
            ToxicSignalDetailOperatorAction::NoTradeWarning
        }
        ToxicSignalInboxOperatorAction::NeedsMoreData => {
            ToxicSignalDetailOperatorAction::NeedsMoreData
        }
    }
}

fn markout_summary(markout: &ToxicMarkoutSignal) -> String {
    format!(
        "1m {}, 5m {}, 15m {}, 1h {}.",
        window_outcome(markout, "+1m"),
        window_outcome(markout, "+5m"),
        window_outcome(markout, "+15m"),
        window_outcome(markout, "+1h"),
    )
}

fn window_outcome(markout: &ToxicMarkoutSignal, label: &str) -> String {
    markout
        .windows
        .iter()
        .find(|window| window.label == label)
        .map(|window| snake_label(&window.outcome))
        .unwrap_or_else(|| "not_enough_data".to_string())
}

fn snake_label<T>(value: &T) -> String
where
    T: serde::Serialize,
{
    serde_json::to_value(value)
        .ok()
        .and_then(|value| value.as_str().map(ToOwned::to_owned))
        .unwrap_or_else(|| "unknown".to_string())
}
