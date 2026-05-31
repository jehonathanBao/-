use std::collections::BTreeMap;

use crate::types::{
    toxic_governance_ledger::{ToxicGovernanceDecision, ToxicGovernanceDecisionKind},
    toxic_governance_proposal::{
        ToxicGovernanceProposalAction, ToxicGovernanceProposalByAction,
        ToxicGovernanceProposalExportResponse, ToxicGovernanceProposalItem,
        ToxicGovernanceProposalSignalTypeSummary, ToxicGovernanceProposalStatusResponse,
        ToxicGovernanceProposalSummaryResponse, ToxicGovernanceProposalSymbolSummary,
    },
    toxic_weight_recommendation::ToxicWeightRecommendationKind,
    toxic_weight_review::ToxicWeightReviewSummaryResponse,
};

pub fn build_toxic_governance_proposal_summary(
    review_summary: &ToxicWeightReviewSummaryResponse,
    decisions: &[ToxicGovernanceDecision],
    warnings: Vec<String>,
) -> ToxicGovernanceProposalSummaryResponse {
    let latest_decisions = latest_decisions_by_signal(decisions);
    let items = review_summary
        .review_items
        .iter()
        .map(|item| {
            build_proposal_item(
                item,
                latest_decisions.get(&(item.symbol.clone(), item.signal_type.clone())),
            )
        })
        .collect::<Vec<_>>();
    let by_action = count_by_action(&items);
    let by_signal_type = build_signal_type_summaries(&items);
    let by_symbol = build_symbol_summaries(&items);

    ToxicGovernanceProposalSummaryResponse {
        read_only: true,
        proposal_only: true,
        runtime_weight_modified: false,
        config_modified: false,
        runtime_modified: false,
        auto_apply_enabled: false,
        strategy_reloaded: false,
        mode: "proposal_draft_only".to_string(),
        selected_symbol: review_summary.selected_symbol.clone(),
        status: if items.is_empty() {
            "no_governance_proposals".to_string()
        } else {
            "governance_proposals_ready".to_string()
        },
        warnings,
        total_proposals: items.len(),
        by_action,
        items,
        by_signal_type,
        by_symbol,
    }
}

pub fn build_toxic_governance_proposal_status(
    summary: &ToxicGovernanceProposalSummaryResponse,
) -> ToxicGovernanceProposalStatusResponse {
    ToxicGovernanceProposalStatusResponse {
        read_only: true,
        proposal_only: true,
        runtime_weight_modified: false,
        config_modified: false,
        runtime_modified: false,
        auto_apply_enabled: false,
        strategy_reloaded: false,
        enabled: true,
        mode: "proposal_draft_only".to_string(),
        selected_symbol: summary.selected_symbol.clone(),
        status: summary.status.clone(),
        total_proposals: summary.total_proposals,
        safety_boundary: vec![
            "readOnly=true".to_string(),
            "proposalOnly=true".to_string(),
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

pub fn build_toxic_governance_proposal_export(
    summary: &ToxicGovernanceProposalSummaryResponse,
) -> ToxicGovernanceProposalExportResponse {
    ToxicGovernanceProposalExportResponse {
        read_only: true,
        proposal_only: true,
        runtime_weight_modified: false,
        config_modified: false,
        runtime_modified: false,
        auto_apply_enabled: false,
        strategy_reloaded: false,
        mode: "proposal_draft_only".to_string(),
        selected_symbol: summary.selected_symbol.clone(),
        status: summary.status.clone(),
        total_proposals: summary.total_proposals,
        by_action: summary.by_action.clone(),
        items: summary.items.clone(),
        markdown_report: build_markdown_report(summary),
    }
}

fn build_proposal_item(
    item: &crate::types::toxic_weight_review::ToxicWeightReviewItem,
    decision: Option<&ToxicGovernanceDecision>,
) -> ToxicGovernanceProposalItem {
    let governance_decision = decision.map(|entry| entry.decision);
    let proposed_action = map_proposed_action(item.recommended_action, governance_decision);
    let proposal_status = proposal_status(governance_decision);
    let mut governance_notes = item.governance_notes.clone();
    if let Some(entry) = decision {
        governance_notes.push(format!(
            "Governance ledger decision by {}: {:?}.",
            entry.reviewer, entry.decision
        ));
        if !entry.reason.trim().is_empty() {
            governance_notes.push(format!("Governance reason: {}.", entry.reason));
        }
    } else {
        governance_notes.push("Governance decision not recorded yet.".to_string());
    }

    ToxicGovernanceProposalItem {
        symbol: item.symbol.clone(),
        signal_type: item.signal_type.clone(),
        recommended_action: item.recommended_action,
        governance_decision,
        proposed_action,
        sample_count: item.sample_count,
        aligned_ratio: item.aligned_ratio,
        adverse_ratio: item.adverse_ratio,
        neutral_ratio: item.neutral_ratio,
        confidence: item.confidence.clone(),
        reason_codes: item.reason_codes.clone(),
        evidence_summary: item.evidence_summary.clone(),
        governance_notes,
        proposal_status,
        manual_review_required: true,
        read_only: true,
        proposal_only: true,
        runtime_weight_modified: false,
        config_modified: false,
        runtime_modified: false,
        auto_apply_enabled: false,
        strategy_reloaded: false,
    }
}

fn latest_decisions_by_signal(
    decisions: &[ToxicGovernanceDecision],
) -> BTreeMap<(String, String), ToxicGovernanceDecision> {
    let mut map = BTreeMap::new();
    for decision in decisions {
        map.entry((decision.symbol.clone(), decision.signal_type.clone()))
            .or_insert_with(|| decision.clone());
    }
    map
}

fn map_proposed_action(
    recommended_action: ToxicWeightRecommendationKind,
    governance_decision: Option<ToxicGovernanceDecisionKind>,
) -> ToxicGovernanceProposalAction {
    match governance_decision {
        Some(ToxicGovernanceDecisionKind::WatchMore)
        | Some(ToxicGovernanceDecisionKind::NeedsMoreSamples)
        | Some(ToxicGovernanceDecisionKind::RejectRecommendation)
        | Some(ToxicGovernanceDecisionKind::EscalateReview) => {
            ToxicGovernanceProposalAction::NeedsMoreSamples
        }
        Some(ToxicGovernanceDecisionKind::SuppressForNow) => {
            ToxicGovernanceProposalAction::NoTradeOnlyCandidate
        }
        Some(ToxicGovernanceDecisionKind::AcceptRecommendation) | None => {
            map_recommendation_action(recommended_action)
        }
    }
}

fn map_recommendation_action(
    recommended_action: ToxicWeightRecommendationKind,
) -> ToxicGovernanceProposalAction {
    match recommended_action {
        ToxicWeightRecommendationKind::Keep => ToxicGovernanceProposalAction::Keep,
        ToxicWeightRecommendationKind::SlightUpgradeCandidate => {
            ToxicGovernanceProposalAction::SlightUpgradeCandidate
        }
        ToxicWeightRecommendationKind::SlightDowngradeCandidate => {
            ToxicGovernanceProposalAction::SlightDowngradeCandidate
        }
        ToxicWeightRecommendationKind::DowngradeCandidate => {
            ToxicGovernanceProposalAction::DowngradeCandidate
        }
        ToxicWeightRecommendationKind::NoTradeOnlyCandidate => {
            ToxicGovernanceProposalAction::NoTradeOnlyCandidate
        }
        ToxicWeightRecommendationKind::DisableCandidate => {
            ToxicGovernanceProposalAction::DisableCandidate
        }
        ToxicWeightRecommendationKind::InsufficientData => {
            ToxicGovernanceProposalAction::NeedsMoreSamples
        }
    }
}

fn proposal_status(governance_decision: Option<ToxicGovernanceDecisionKind>) -> String {
    match governance_decision {
        Some(ToxicGovernanceDecisionKind::AcceptRecommendation) => {
            "accepted_by_governance".to_string()
        }
        Some(ToxicGovernanceDecisionKind::RejectRecommendation) => {
            "rejected_by_governance".to_string()
        }
        Some(ToxicGovernanceDecisionKind::WatchMore) => "watch_more".to_string(),
        Some(ToxicGovernanceDecisionKind::NeedsMoreSamples) => "needs_more_samples".to_string(),
        Some(ToxicGovernanceDecisionKind::SuppressForNow) => "suppressed_for_now".to_string(),
        Some(ToxicGovernanceDecisionKind::EscalateReview) => "escalated_review".to_string(),
        None => "pending_governance_review".to_string(),
    }
}

fn build_signal_type_summaries(
    items: &[ToxicGovernanceProposalItem],
) -> Vec<ToxicGovernanceProposalSignalTypeSummary> {
    let mut buckets = BTreeMap::<String, Vec<ToxicGovernanceProposalItem>>::new();
    for item in items {
        buckets
            .entry(item.signal_type.clone())
            .or_default()
            .push(item.clone());
    }

    buckets
        .into_iter()
        .map(
            |(signal_type, entries)| ToxicGovernanceProposalSignalTypeSummary {
                signal_type,
                total_proposals: entries.len(),
                by_action: count_by_action(&entries),
            },
        )
        .collect()
}

fn build_symbol_summaries(
    items: &[ToxicGovernanceProposalItem],
) -> Vec<ToxicGovernanceProposalSymbolSummary> {
    let mut buckets = BTreeMap::<String, Vec<ToxicGovernanceProposalItem>>::new();
    for item in items {
        buckets
            .entry(item.symbol.clone())
            .or_default()
            .push(item.clone());
    }

    buckets
        .into_iter()
        .map(|(symbol, entries)| ToxicGovernanceProposalSymbolSummary {
            symbol,
            total_proposals: entries.len(),
            by_action: count_by_action(&entries),
        })
        .collect()
}

fn count_by_action(items: &[ToxicGovernanceProposalItem]) -> ToxicGovernanceProposalByAction {
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
                by_action.slight_upgrade_candidate += 1;
            }
            ToxicGovernanceProposalAction::SlightDowngradeCandidate => {
                by_action.slight_downgrade_candidate += 1;
            }
            ToxicGovernanceProposalAction::DowngradeCandidate => {
                by_action.downgrade_candidate += 1;
            }
            ToxicGovernanceProposalAction::NoTradeOnlyCandidate => {
                by_action.no_trade_only_candidate += 1;
            }
            ToxicGovernanceProposalAction::DisableCandidate => {
                by_action.disable_candidate += 1;
            }
            ToxicGovernanceProposalAction::NeedsMoreSamples => {
                by_action.needs_more_samples += 1;
            }
        }
    }

    by_action
}

fn build_markdown_report(summary: &ToxicGovernanceProposalSummaryResponse) -> String {
    let mut lines = vec![
        "# Toxic Governance Proposals".to_string(),
        String::new(),
        "## Summary".to_string(),
        format!("- Selected Symbol: {}", summary.selected_symbol),
        format!("- Status: {}", summary.status),
        format!("- Total Proposals: {}", summary.total_proposals),
        format!("- Keep: {}", summary.by_action.keep),
        format!(
            "- Slight Upgrade Candidates: {}",
            summary.by_action.slight_upgrade_candidate
        ),
        format!(
            "- Slight Downgrade Candidates: {}",
            summary.by_action.slight_downgrade_candidate
        ),
        format!(
            "- Downgrade Candidates: {}",
            summary.by_action.downgrade_candidate
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
        String::new(),
        "## Proposal Items".to_string(),
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
