use std::collections::BTreeMap;

use crate::types::{
    toxic_governance_ledger::ToxicGovernanceDecisionKind,
    toxic_governance_proposal::{
        ToxicGovernanceProposalByAction, ToxicGovernanceProposalSummaryResponse,
    },
    toxic_governance_review_pack::{
        ToxicGovernanceReviewPackDecisionSummary, ToxicGovernanceReviewPackExportResponse,
        ToxicGovernanceReviewPackItem, ToxicGovernanceReviewPackSignalTypeSummary,
        ToxicGovernanceReviewPackStatusResponse, ToxicGovernanceReviewPackSummaryResponse,
        ToxicGovernanceReviewPackSymbolSummary,
    },
};

pub fn build_toxic_governance_review_pack_summary(
    proposal_summary: &ToxicGovernanceProposalSummaryResponse,
) -> ToxicGovernanceReviewPackSummaryResponse {
    let items = proposal_summary
        .items
        .iter()
        .map(|item| ToxicGovernanceReviewPackItem {
            symbol: item.symbol.clone(),
            signal_type: item.signal_type.clone(),
            recommended_action: item.recommended_action,
            governance_decision: item.governance_decision,
            proposed_action: item.proposed_action,
            proposal_status: item.proposal_status.clone(),
            sample_count: item.sample_count,
            aligned_ratio: item.aligned_ratio,
            adverse_ratio: item.adverse_ratio,
            neutral_ratio: item.neutral_ratio,
            confidence: item.confidence.clone(),
            reason_codes: item.reason_codes.clone(),
            evidence_summary: item.evidence_summary.clone(),
            governance_notes: item.governance_notes.clone(),
            manual_review_required: true,
            read_only: true,
            review_pack_only: true,
            proposal_only: true,
            runtime_weight_modified: false,
            config_modified: false,
            runtime_modified: false,
            auto_apply_enabled: false,
            strategy_reloaded: false,
        })
        .collect::<Vec<_>>();
    let by_action = proposal_summary.by_action.clone();
    let by_decision = count_by_decision(&items);
    let by_signal_type = build_signal_type_summaries(&items);
    let by_symbol = build_symbol_summaries(&items);
    let recent_governance_notes = items
        .iter()
        .flat_map(|item| item.governance_notes.iter().cloned())
        .take(8)
        .collect::<Vec<_>>();
    let ready_for_manual_review = !items.is_empty();
    let status = if items.is_empty() {
        "no_governance_review_pack_items".to_string()
    } else if proposal_summary.warnings.is_empty() {
        "governance_review_pack_ready".to_string()
    } else {
        "governance_review_pack_ready_with_warnings".to_string()
    };

    ToxicGovernanceReviewPackSummaryResponse {
        read_only: true,
        proposal_only: true,
        review_pack_only: true,
        runtime_weight_modified: false,
        config_modified: false,
        runtime_modified: false,
        auto_apply_enabled: false,
        strategy_reloaded: false,
        mode: "governance_review_pack_only".to_string(),
        selected_symbol: proposal_summary.selected_symbol.clone(),
        status,
        ready_for_manual_review,
        warnings: proposal_summary.warnings.clone(),
        total_items: items.len(),
        by_action,
        by_decision,
        recent_governance_notes,
        items,
        by_signal_type,
        by_symbol,
    }
}

pub fn build_toxic_governance_review_pack_status(
    summary: &ToxicGovernanceReviewPackSummaryResponse,
) -> ToxicGovernanceReviewPackStatusResponse {
    ToxicGovernanceReviewPackStatusResponse {
        read_only: true,
        proposal_only: true,
        review_pack_only: true,
        runtime_weight_modified: false,
        config_modified: false,
        runtime_modified: false,
        auto_apply_enabled: false,
        strategy_reloaded: false,
        enabled: true,
        mode: "governance_review_pack_only".to_string(),
        selected_symbol: summary.selected_symbol.clone(),
        status: summary.status.clone(),
        ready_for_manual_review: summary.ready_for_manual_review,
        total_items: summary.total_items,
        safety_boundary: vec![
            "readOnly=true".to_string(),
            "proposalOnly=true".to_string(),
            "reviewPackOnly=true".to_string(),
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

pub fn build_toxic_governance_review_pack_export(
    summary: &ToxicGovernanceReviewPackSummaryResponse,
) -> ToxicGovernanceReviewPackExportResponse {
    ToxicGovernanceReviewPackExportResponse {
        read_only: true,
        proposal_only: true,
        review_pack_only: true,
        runtime_weight_modified: false,
        config_modified: false,
        runtime_modified: false,
        auto_apply_enabled: false,
        strategy_reloaded: false,
        mode: "governance_review_pack_only".to_string(),
        selected_symbol: summary.selected_symbol.clone(),
        status: summary.status.clone(),
        ready_for_manual_review: summary.ready_for_manual_review,
        total_items: summary.total_items,
        by_action: summary.by_action.clone(),
        by_decision: summary.by_decision.clone(),
        recent_governance_notes: summary.recent_governance_notes.clone(),
        items: summary.items.clone(),
        markdown_report: build_markdown_report(summary),
    }
}

fn build_signal_type_summaries(
    items: &[ToxicGovernanceReviewPackItem],
) -> Vec<ToxicGovernanceReviewPackSignalTypeSummary> {
    let mut buckets = BTreeMap::<String, Vec<ToxicGovernanceReviewPackItem>>::new();
    for item in items {
        buckets
            .entry(item.signal_type.clone())
            .or_default()
            .push(item.clone());
    }

    buckets
        .into_iter()
        .map(
            |(signal_type, entries)| ToxicGovernanceReviewPackSignalTypeSummary {
                signal_type,
                total_items: entries.len(),
                by_action: count_by_action(&entries),
                by_decision: count_by_decision(&entries),
            },
        )
        .collect()
}

fn build_symbol_summaries(
    items: &[ToxicGovernanceReviewPackItem],
) -> Vec<ToxicGovernanceReviewPackSymbolSummary> {
    let mut buckets = BTreeMap::<String, Vec<ToxicGovernanceReviewPackItem>>::new();
    for item in items {
        buckets
            .entry(item.symbol.clone())
            .or_default()
            .push(item.clone());
    }

    buckets
        .into_iter()
        .map(|(symbol, entries)| ToxicGovernanceReviewPackSymbolSummary {
            symbol,
            total_items: entries.len(),
            by_action: count_by_action(&entries),
            by_decision: count_by_decision(&entries),
        })
        .collect()
}

fn count_by_action(items: &[ToxicGovernanceReviewPackItem]) -> ToxicGovernanceProposalByAction {
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
            crate::types::toxic_governance_proposal::ToxicGovernanceProposalAction::Keep => {
                by_action.keep += 1
            }
            crate::types::toxic_governance_proposal::ToxicGovernanceProposalAction::SlightUpgradeCandidate => {
                by_action.slight_upgrade_candidate += 1
            }
            crate::types::toxic_governance_proposal::ToxicGovernanceProposalAction::SlightDowngradeCandidate => {
                by_action.slight_downgrade_candidate += 1
            }
            crate::types::toxic_governance_proposal::ToxicGovernanceProposalAction::DowngradeCandidate => {
                by_action.downgrade_candidate += 1
            }
            crate::types::toxic_governance_proposal::ToxicGovernanceProposalAction::NoTradeOnlyCandidate => {
                by_action.no_trade_only_candidate += 1
            }
            crate::types::toxic_governance_proposal::ToxicGovernanceProposalAction::DisableCandidate => {
                by_action.disable_candidate += 1
            }
            crate::types::toxic_governance_proposal::ToxicGovernanceProposalAction::NeedsMoreSamples => {
                by_action.needs_more_samples += 1
            }
        }
    }

    by_action
}

fn count_by_decision(
    items: &[ToxicGovernanceReviewPackItem],
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

fn build_markdown_report(summary: &ToxicGovernanceReviewPackSummaryResponse) -> String {
    let mut lines = vec![
        "# Toxic Governance Review Pack".to_string(),
        String::new(),
        "## Summary".to_string(),
        format!("- Selected Symbol: {}", summary.selected_symbol),
        format!("- Status: {}", summary.status),
        format!(
            "- Ready For Manual Review: {}",
            summary.ready_for_manual_review
        ),
        format!("- Total Items: {}", summary.total_items),
        format!("- Keep: {}", summary.by_action.keep),
        format!(
            "- Upgrade Candidates: {}",
            summary.by_action.slight_upgrade_candidate
        ),
        format!(
            "- Downgrade Candidates: {}",
            summary.by_action.slight_downgrade_candidate + summary.by_action.downgrade_candidate
        ),
        format!(
            "- No-trade Only: {}",
            summary.by_action.no_trade_only_candidate
        ),
        format!(
            "- Disable Candidates: {}",
            summary.by_action.disable_candidate
        ),
        format!(
            "- Needs More Samples: {}",
            summary.by_action.needs_more_samples
        ),
        format!(
            "- Accepted Decisions: {}",
            summary.by_decision.accepted_count
        ),
        format!(
            "- Rejected Decisions: {}",
            summary.by_decision.rejected_count
        ),
        format!("- Watch More: {}", summary.by_decision.watch_more_count),
        format!(
            "- Pending Governance Review: {}",
            summary.by_decision.pending_governance_review_count
        ),
        String::new(),
        "## Review Items".to_string(),
    ];

    if summary.items.is_empty() {
        lines.push("- None".to_string());
    } else {
        for item in &summary.items {
            lines.push(format!(
                "- {} / {}: {:?} -> {:?} ({})",
                item.symbol,
                item.signal_type,
                item.recommended_action,
                item.proposed_action,
                item.proposal_status
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
            lines.push(format!(
                "  - Evidence Summary: {}",
                if item.evidence_summary.is_empty() {
                    "none".to_string()
                } else {
                    item.evidence_summary.join("; ")
                }
            ));
        }
    }

    lines.join("\n")
}
