use std::collections::BTreeMap;

use crate::types::{
    toxic_governance_ledger::ToxicGovernanceDecisionKind,
    toxic_governance_proposal::{ToxicGovernanceProposalAction, ToxicGovernanceProposalByAction},
    toxic_governance_review_pack::{
        ToxicGovernanceReviewPackDecisionSummary, ToxicGovernanceReviewPackItem,
        ToxicGovernanceReviewPackSummaryResponse,
    },
    toxic_governance_signoff_pack::{
        ToxicGovernanceSignoffPackExportResponse, ToxicGovernanceSignoffPackItem,
        ToxicGovernanceSignoffPackSignalTypeSummary, ToxicGovernanceSignoffPackStatusResponse,
        ToxicGovernanceSignoffPackSummaryResponse, ToxicGovernanceSignoffPackSymbolSummary,
    },
};

pub fn build_toxic_governance_signoff_pack_summary(
    review_pack: &ToxicGovernanceReviewPackSummaryResponse,
) -> ToxicGovernanceSignoffPackSummaryResponse {
    let items = review_pack
        .items
        .iter()
        .map(build_signoff_item)
        .collect::<Vec<_>>();
    let ready_for_signoff_count = items
        .iter()
        .filter(|item| item.signoff_recommendation == "ready_for_manual_signoff")
        .count();
    let hold_count = items.len().saturating_sub(ready_for_signoff_count);
    let mut blocked_reasons = Vec::new();
    if !review_pack.ready_for_manual_review {
        blocked_reasons.push("review_pack_not_ready".to_string());
    }
    if review_pack.by_decision.pending_governance_review_count > 0 {
        blocked_reasons.push("pending_governance_review_items".to_string());
    }
    if review_pack.by_decision.escalate_review_count > 0 {
        blocked_reasons.push("escalated_review_items_present".to_string());
    }
    if review_pack.by_decision.watch_more_count > 0 {
        blocked_reasons.push("watch_more_items_present".to_string());
    }
    if review_pack.by_decision.needs_more_samples_count > 0 {
        blocked_reasons.push("needs_more_samples_items_present".to_string());
    }
    if review_pack.by_decision.rejected_count > 0 {
        blocked_reasons.push("rejected_governance_items_present".to_string());
    }
    let ready_for_manual_signoff = !items.is_empty() && blocked_reasons.is_empty();
    let by_signal_type = build_signal_type_summaries(&items);
    let by_symbol = build_symbol_summaries(&items);
    let status = if items.is_empty() {
        "no_governance_signoff_items".to_string()
    } else if ready_for_manual_signoff {
        "governance_signoff_pack_ready".to_string()
    } else {
        "governance_signoff_pack_blocked".to_string()
    };

    ToxicGovernanceSignoffPackSummaryResponse {
        read_only: true,
        proposal_only: true,
        review_pack_only: true,
        signoff_pack_only: true,
        runtime_weight_modified: false,
        config_modified: false,
        runtime_modified: false,
        auto_apply_enabled: false,
        strategy_reloaded: false,
        mode: "governance_signoff_pack_only".to_string(),
        selected_symbol: review_pack.selected_symbol.clone(),
        status,
        ready_for_manual_signoff,
        blocked_reasons,
        warnings: review_pack.warnings.clone(),
        total_items: items.len(),
        ready_for_signoff_count,
        hold_count,
        by_action: review_pack.by_action.clone(),
        by_decision: review_pack.by_decision.clone(),
        recent_governance_notes: review_pack.recent_governance_notes.clone(),
        items,
        by_signal_type,
        by_symbol,
    }
}

pub fn build_toxic_governance_signoff_pack_status(
    summary: &ToxicGovernanceSignoffPackSummaryResponse,
) -> ToxicGovernanceSignoffPackStatusResponse {
    ToxicGovernanceSignoffPackStatusResponse {
        read_only: true,
        proposal_only: true,
        review_pack_only: true,
        signoff_pack_only: true,
        runtime_weight_modified: false,
        config_modified: false,
        runtime_modified: false,
        auto_apply_enabled: false,
        strategy_reloaded: false,
        enabled: true,
        mode: "governance_signoff_pack_only".to_string(),
        selected_symbol: summary.selected_symbol.clone(),
        status: summary.status.clone(),
        ready_for_manual_signoff: summary.ready_for_manual_signoff,
        blocked_reasons: summary.blocked_reasons.clone(),
        total_items: summary.total_items,
        safety_boundary: vec![
            "readOnly=true".to_string(),
            "proposalOnly=true".to_string(),
            "reviewPackOnly=true".to_string(),
            "signoffPackOnly=true".to_string(),
            "runtimeWeightModified=false".to_string(),
            "configModified=false".to_string(),
            "runtimeModified=false".to_string(),
            "autoApplyEnabled=false".to_string(),
            "strategyReloaded=false".to_string(),
            "No automatic weight update".to_string(),
            "No runtime config mutation".to_string(),
            "No strategy reload".to_string(),
            "No calibration_runner".to_string(),
        ],
    }
}

pub fn build_toxic_governance_signoff_pack_export(
    summary: &ToxicGovernanceSignoffPackSummaryResponse,
) -> ToxicGovernanceSignoffPackExportResponse {
    ToxicGovernanceSignoffPackExportResponse {
        read_only: true,
        proposal_only: true,
        review_pack_only: true,
        signoff_pack_only: true,
        runtime_weight_modified: false,
        config_modified: false,
        runtime_modified: false,
        auto_apply_enabled: false,
        strategy_reloaded: false,
        mode: "governance_signoff_pack_only".to_string(),
        selected_symbol: summary.selected_symbol.clone(),
        status: summary.status.clone(),
        ready_for_manual_signoff: summary.ready_for_manual_signoff,
        blocked_reasons: summary.blocked_reasons.clone(),
        total_items: summary.total_items,
        ready_for_signoff_count: summary.ready_for_signoff_count,
        hold_count: summary.hold_count,
        by_action: summary.by_action.clone(),
        by_decision: summary.by_decision.clone(),
        recent_governance_notes: summary.recent_governance_notes.clone(),
        items: summary.items.clone(),
        markdown_report: build_markdown_report(summary),
    }
}

fn build_signoff_item(item: &ToxicGovernanceReviewPackItem) -> ToxicGovernanceSignoffPackItem {
    let blocked_reason = signoff_blocked_reason(item);
    let signoff_recommendation = if blocked_reason.is_none() {
        "ready_for_manual_signoff".to_string()
    } else {
        "hold_for_review".to_string()
    };

    ToxicGovernanceSignoffPackItem {
        symbol: item.symbol.clone(),
        signal_type: item.signal_type.clone(),
        recommended_action: item.recommended_action,
        governance_decision: item.governance_decision,
        proposed_action: item.proposed_action,
        proposal_status: item.proposal_status.clone(),
        signoff_recommendation,
        blocked_reason,
        sample_count: item.sample_count,
        aligned_ratio: item.aligned_ratio,
        adverse_ratio: item.adverse_ratio,
        neutral_ratio: item.neutral_ratio,
        confidence: item.confidence.clone(),
        reason_codes: item.reason_codes.clone(),
        evidence_summary: item.evidence_summary.clone(),
        governance_notes: item.governance_notes.clone(),
        read_only: true,
        proposal_only: true,
        review_pack_only: true,
        signoff_pack_only: true,
        runtime_weight_modified: false,
        config_modified: false,
        runtime_modified: false,
        auto_apply_enabled: false,
        strategy_reloaded: false,
    }
}

fn signoff_blocked_reason(item: &ToxicGovernanceReviewPackItem) -> Option<String> {
    match item.governance_decision {
        Some(ToxicGovernanceDecisionKind::AcceptRecommendation) => None,
        Some(ToxicGovernanceDecisionKind::SuppressForNow)
            if item.proposed_action == ToxicGovernanceProposalAction::NoTradeOnlyCandidate =>
        {
            None
        }
        Some(ToxicGovernanceDecisionKind::RejectRecommendation) => {
            Some("rejected_governance_decision".to_string())
        }
        Some(ToxicGovernanceDecisionKind::WatchMore) => Some("watch_more".to_string()),
        Some(ToxicGovernanceDecisionKind::NeedsMoreSamples) => {
            Some("needs_more_samples".to_string())
        }
        Some(ToxicGovernanceDecisionKind::EscalateReview) => Some("escalated_review".to_string()),
        Some(ToxicGovernanceDecisionKind::SuppressForNow) => Some("suppressed_for_now".to_string()),
        None => Some("pending_governance_review".to_string()),
    }
}

fn build_signal_type_summaries(
    items: &[ToxicGovernanceSignoffPackItem],
) -> Vec<ToxicGovernanceSignoffPackSignalTypeSummary> {
    let mut buckets = BTreeMap::<String, Vec<ToxicGovernanceSignoffPackItem>>::new();
    for item in items {
        buckets
            .entry(item.signal_type.clone())
            .or_default()
            .push(item.clone());
    }

    buckets
        .into_iter()
        .map(
            |(signal_type, entries)| ToxicGovernanceSignoffPackSignalTypeSummary {
                signal_type,
                total_items: entries.len(),
                ready_for_signoff_count: entries
                    .iter()
                    .filter(|item| item.signoff_recommendation == "ready_for_manual_signoff")
                    .count(),
                hold_count: entries
                    .iter()
                    .filter(|item| item.signoff_recommendation != "ready_for_manual_signoff")
                    .count(),
                by_action: count_by_action(&entries),
                by_decision: count_by_decision(&entries),
            },
        )
        .collect()
}

fn build_symbol_summaries(
    items: &[ToxicGovernanceSignoffPackItem],
) -> Vec<ToxicGovernanceSignoffPackSymbolSummary> {
    let mut buckets = BTreeMap::<String, Vec<ToxicGovernanceSignoffPackItem>>::new();
    for item in items {
        buckets
            .entry(item.symbol.clone())
            .or_default()
            .push(item.clone());
    }

    buckets
        .into_iter()
        .map(
            |(symbol, entries)| ToxicGovernanceSignoffPackSymbolSummary {
                symbol,
                total_items: entries.len(),
                ready_for_signoff_count: entries
                    .iter()
                    .filter(|item| item.signoff_recommendation == "ready_for_manual_signoff")
                    .count(),
                hold_count: entries
                    .iter()
                    .filter(|item| item.signoff_recommendation != "ready_for_manual_signoff")
                    .count(),
                by_action: count_by_action(&entries),
                by_decision: count_by_decision(&entries),
            },
        )
        .collect()
}

fn count_by_action(items: &[ToxicGovernanceSignoffPackItem]) -> ToxicGovernanceProposalByAction {
    let mut by_action = ToxicGovernanceProposalByAction {
        keep: 0,
        slight_upgrade_candidate: 0,
        slight_downgrade_candidate: 0,
        downgrade_candidate: 0,
        no_trade_only_candidate: 0,
        disable_candidate: 0,
        needs_more_samples: 0,
    };

    for item in items {
        match item.proposed_action {
            ToxicGovernanceProposalAction::Keep => by_action.keep += 1,
            ToxicGovernanceProposalAction::SlightUpgradeCandidate => {
                by_action.slight_upgrade_candidate += 1
            }
            ToxicGovernanceProposalAction::SlightDowngradeCandidate => {
                by_action.slight_downgrade_candidate += 1
            }
            ToxicGovernanceProposalAction::DowngradeCandidate => by_action.downgrade_candidate += 1,
            ToxicGovernanceProposalAction::NoTradeOnlyCandidate => {
                by_action.no_trade_only_candidate += 1
            }
            ToxicGovernanceProposalAction::DisableCandidate => by_action.disable_candidate += 1,
            ToxicGovernanceProposalAction::NeedsMoreSamples => by_action.needs_more_samples += 1,
        }
    }

    by_action
}

fn count_by_decision(
    items: &[ToxicGovernanceSignoffPackItem],
) -> ToxicGovernanceReviewPackDecisionSummary {
    let mut by_decision = ToxicGovernanceReviewPackDecisionSummary {
        accepted_count: 0,
        rejected_count: 0,
        watch_more_count: 0,
        needs_more_samples_count: 0,
        suppress_for_now_count: 0,
        escalate_review_count: 0,
        pending_governance_review_count: 0,
    };

    for item in items {
        match item.governance_decision {
            Some(ToxicGovernanceDecisionKind::AcceptRecommendation) => {
                by_decision.accepted_count += 1
            }
            Some(ToxicGovernanceDecisionKind::RejectRecommendation) => {
                by_decision.rejected_count += 1
            }
            Some(ToxicGovernanceDecisionKind::WatchMore) => by_decision.watch_more_count += 1,
            Some(ToxicGovernanceDecisionKind::NeedsMoreSamples) => {
                by_decision.needs_more_samples_count += 1
            }
            Some(ToxicGovernanceDecisionKind::SuppressForNow) => {
                by_decision.suppress_for_now_count += 1
            }
            Some(ToxicGovernanceDecisionKind::EscalateReview) => {
                by_decision.escalate_review_count += 1
            }
            None => by_decision.pending_governance_review_count += 1,
        }
    }

    by_decision
}

fn build_markdown_report(summary: &ToxicGovernanceSignoffPackSummaryResponse) -> String {
    let mut lines = vec![
        "# Toxic Governance Signoff Pack".to_string(),
        String::new(),
        "## Summary".to_string(),
        format!("- Selected Symbol: {}", summary.selected_symbol),
        format!("- Status: {}", summary.status),
        format!(
            "- Ready For Manual Signoff: {}",
            summary.ready_for_manual_signoff
        ),
        format!("- Total Items: {}", summary.total_items),
        format!("- Ready For Signoff: {}", summary.ready_for_signoff_count),
        format!("- Hold For Review: {}", summary.hold_count),
        format!(
            "- Blocked Reasons: {}",
            if summary.blocked_reasons.is_empty() {
                "none".to_string()
            } else {
                summary.blocked_reasons.join(", ")
            }
        ),
        String::new(),
        "## Signoff Items".to_string(),
    ];

    if summary.items.is_empty() {
        lines.push("- None".to_string());
    } else {
        for item in &summary.items {
            lines.push(format!(
                "- {} / {}: {} ({})",
                item.symbol, item.signal_type, item.signoff_recommendation, item.proposal_status
            ));
            lines.push(format!(
                "  - blocked_reason: {}",
                item.blocked_reason
                    .clone()
                    .unwrap_or_else(|| "none".to_string())
            ));
            lines.push(format!(
                "  - aligned {:.2}%, adverse {:.2}%, neutral {:.2}%, samples {}",
                item.aligned_ratio * 100.0,
                item.adverse_ratio * 100.0,
                item.neutral_ratio * 100.0,
                item.sample_count
            ));
            lines.push(format!(
                "  - Reason Codes: {}",
                if item.reason_codes.is_empty() {
                    "none".to_string()
                } else {
                    item.reason_codes.join(", ")
                }
            ));
        }
    }

    lines.join("\n")
}
