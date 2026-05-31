use std::{cmp::Reverse, collections::BTreeMap, fs, path::Path};

use anyhow::Context;

use crate::types::toxic_governance_ledger::{
    ToxicGovernanceDecision, ToxicGovernanceDecisionKind, ToxicGovernanceLedgerExportResponse,
    ToxicGovernanceLedgerStatusResponse, ToxicGovernanceLedgerSummaryResponse,
    ToxicGovernanceSignalTypeSummary, ToxicGovernanceSymbolSummary,
};

pub fn load_toxic_governance_decisions(
    path: &Path,
) -> anyhow::Result<Vec<ToxicGovernanceDecision>> {
    if !path.exists() {
        return Ok(Vec::new());
    }

    let raw = fs::read_to_string(path)
        .with_context(|| format!("failed to read governance ledger {}", path.display()))?;

    let mut decisions = Vec::new();
    for (line_number, line) in raw.lines().enumerate() {
        let trimmed = line.trim();
        if trimmed.is_empty() {
            continue;
        }
        let decision =
            serde_json::from_str::<ToxicGovernanceDecision>(trimmed).with_context(|| {
                format!(
                    "failed to parse governance ledger {} line {}",
                    path.display(),
                    line_number + 1
                )
            })?;
        decisions.push(decision);
    }

    decisions.sort_by_key(|decision| Reverse(decision.created_at_ms));
    Ok(decisions)
}

pub fn build_toxic_governance_ledger_summary(
    selected_symbol: &str,
    decisions: &[ToxicGovernanceDecision],
    warnings: Vec<String>,
) -> ToxicGovernanceLedgerSummaryResponse {
    let filtered_decisions = filter_decisions(selected_symbol, decisions);
    let by_symbol = build_symbol_summaries(&filtered_decisions);
    let by_signal_type = build_signal_type_summaries(&filtered_decisions);
    let recent_governance_notes = filtered_decisions
        .iter()
        .filter_map(|decision| {
            if decision.notes.trim().is_empty() {
                None
            } else {
                Some(format!(
                    "{} / {} / {}: {}",
                    decision.symbol, decision.signal_type, decision.reviewer, decision.notes
                ))
            }
        })
        .take(5)
        .collect::<Vec<_>>();

    let counts = count_decisions(&filtered_decisions);
    let status = if filtered_decisions.is_empty() {
        "no_governance_decisions_recorded".to_string()
    } else {
        "governance_ledger_ready".to_string()
    };

    ToxicGovernanceLedgerSummaryResponse {
        read_only: true,
        analysis_only: true,
        execution_enabled: false,
        manual_review_required: true,
        governance_ledger_only: true,
        runtime_weight_modified: false,
        config_modified: false,
        runtime_modified: false,
        auto_apply_enabled: false,
        strategy_reloaded: false,
        mode: "governance_ledger_only".to_string(),
        selected_symbol: selected_symbol.to_string(),
        governance_status: status.clone(),
        status,
        manual_review_decision_placeholder: "manual_review_decision_placeholder_only_no_write_path"
            .to_string(),
        evidence_lineage: build_evidence_lineage(),
        warnings,
        total_decisions: filtered_decisions.len(),
        accept_count: counts.accept_count,
        reject_count: counts.reject_count,
        watch_more_count: counts.watch_more_count,
        needs_more_samples_count: counts.needs_more_samples_count,
        suppress_for_now_count: counts.suppress_for_now_count,
        escalate_review_count: counts.escalate_review_count,
        consensus_status: derive_consensus_status(&counts),
        recent_governance_notes,
        decisions: filtered_decisions,
        by_symbol,
        by_signal_type,
    }
}

pub fn build_toxic_governance_ledger_status(
    summary: &ToxicGovernanceLedgerSummaryResponse,
) -> ToxicGovernanceLedgerStatusResponse {
    ToxicGovernanceLedgerStatusResponse {
        read_only: true,
        analysis_only: true,
        execution_enabled: false,
        manual_review_required: true,
        governance_ledger_only: true,
        runtime_weight_modified: false,
        config_modified: false,
        runtime_modified: false,
        auto_apply_enabled: false,
        strategy_reloaded: false,
        enabled: true,
        mode: "governance_ledger_only".to_string(),
        selected_symbol: summary.selected_symbol.clone(),
        status: summary.status.clone(),
        total_decisions: summary.total_decisions,
        consensus_status: summary.consensus_status.clone(),
        safety_boundary: vec![
            "readOnly=true".to_string(),
            "analysisOnly=true".to_string(),
            "manualReviewRequired=true".to_string(),
            "governanceLedgerOnly=true".to_string(),
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

pub fn build_toxic_governance_ledger_export(
    summary: &ToxicGovernanceLedgerSummaryResponse,
) -> ToxicGovernanceLedgerExportResponse {
    ToxicGovernanceLedgerExportResponse {
        read_only: true,
        analysis_only: true,
        execution_enabled: false,
        manual_review_required: true,
        governance_ledger_only: true,
        runtime_weight_modified: false,
        config_modified: false,
        runtime_modified: false,
        auto_apply_enabled: false,
        strategy_reloaded: false,
        mode: "governance_ledger_only".to_string(),
        selected_symbol: summary.selected_symbol.clone(),
        status: summary.status.clone(),
        governance_status: summary.governance_status.clone(),
        manual_review_decision_placeholder: summary.manual_review_decision_placeholder.clone(),
        evidence_lineage: summary.evidence_lineage.clone(),
        total_decisions: summary.total_decisions,
        consensus_status: summary.consensus_status.clone(),
        recent_governance_notes: summary.recent_governance_notes.clone(),
        decisions: summary.decisions.clone(),
        markdown_report: build_markdown_report(summary),
    }
}

fn build_evidence_lineage() -> Vec<String> {
    vec![
        "T10 weight recommendation".to_string(),
        "T11 manual weight review export".to_string(),
        "manual review decision placeholder".to_string(),
        "governance ledger decision history if present".to_string(),
    ]
}

fn filter_decisions(
    selected_symbol: &str,
    decisions: &[ToxicGovernanceDecision],
) -> Vec<ToxicGovernanceDecision> {
    if selected_symbol.eq_ignore_ascii_case("ALL") {
        return decisions.to_vec();
    }

    decisions
        .iter()
        .filter(|decision| decision.symbol == selected_symbol)
        .cloned()
        .collect()
}

fn build_symbol_summaries(
    decisions: &[ToxicGovernanceDecision],
) -> Vec<ToxicGovernanceSymbolSummary> {
    let mut buckets = BTreeMap::<String, Vec<ToxicGovernanceDecision>>::new();
    for decision in decisions {
        buckets
            .entry(decision.symbol.clone())
            .or_default()
            .push(decision.clone());
    }

    buckets
        .into_iter()
        .map(|(symbol, entries)| {
            let counts = count_decisions(&entries);
            ToxicGovernanceSymbolSummary {
                symbol,
                total_decisions: entries.len(),
                accept_count: counts.accept_count,
                reject_count: counts.reject_count,
                watch_more_count: counts.watch_more_count,
                needs_more_samples_count: counts.needs_more_samples_count,
                suppress_for_now_count: counts.suppress_for_now_count,
                escalate_review_count: counts.escalate_review_count,
                consensus_status: derive_consensus_status(&counts),
            }
        })
        .collect()
}

fn build_signal_type_summaries(
    decisions: &[ToxicGovernanceDecision],
) -> Vec<ToxicGovernanceSignalTypeSummary> {
    let mut buckets = BTreeMap::<String, Vec<ToxicGovernanceDecision>>::new();
    for decision in decisions {
        buckets
            .entry(decision.signal_type.clone())
            .or_default()
            .push(decision.clone());
    }

    buckets
        .into_iter()
        .map(|(signal_type, entries)| {
            let counts = count_decisions(&entries);
            ToxicGovernanceSignalTypeSummary {
                signal_type,
                total_decisions: entries.len(),
                accept_count: counts.accept_count,
                reject_count: counts.reject_count,
                watch_more_count: counts.watch_more_count,
                needs_more_samples_count: counts.needs_more_samples_count,
                suppress_for_now_count: counts.suppress_for_now_count,
                escalate_review_count: counts.escalate_review_count,
                consensus_status: derive_consensus_status(&counts),
            }
        })
        .collect()
}

#[derive(Default)]
struct DecisionCounts {
    accept_count: usize,
    reject_count: usize,
    watch_more_count: usize,
    needs_more_samples_count: usize,
    suppress_for_now_count: usize,
    escalate_review_count: usize,
}

fn count_decisions(decisions: &[ToxicGovernanceDecision]) -> DecisionCounts {
    let mut counts = DecisionCounts::default();
    for decision in decisions {
        match decision.decision {
            ToxicGovernanceDecisionKind::AcceptRecommendation => counts.accept_count += 1,
            ToxicGovernanceDecisionKind::RejectRecommendation => counts.reject_count += 1,
            ToxicGovernanceDecisionKind::WatchMore => counts.watch_more_count += 1,
            ToxicGovernanceDecisionKind::NeedsMoreSamples => {
                counts.needs_more_samples_count += 1;
            }
            ToxicGovernanceDecisionKind::SuppressForNow => counts.suppress_for_now_count += 1,
            ToxicGovernanceDecisionKind::EscalateReview => counts.escalate_review_count += 1,
        }
    }
    counts
}

fn derive_consensus_status(counts: &DecisionCounts) -> String {
    let total = counts.accept_count
        + counts.reject_count
        + counts.watch_more_count
        + counts.needs_more_samples_count
        + counts.suppress_for_now_count
        + counts.escalate_review_count;
    if total == 0 {
        return "no_decisions_recorded".to_string();
    }

    let insufficient = counts.watch_more_count + counts.needs_more_samples_count;
    let mut candidates = [
        ("governance_consensus_accept", counts.accept_count),
        ("governance_consensus_reject", counts.reject_count),
        ("evidence_insufficient", insufficient),
        ("temporarily_suppressed", counts.suppress_for_now_count),
        ("manual_escalation_required", counts.escalate_review_count),
    ];
    candidates.sort_by_key(|candidate| Reverse(candidate.1));

    if candidates.len() > 1 && candidates[0].1 == candidates[1].1 {
        "mixed_governance_state".to_string()
    } else {
        candidates[0].0.to_string()
    }
}

fn build_markdown_report(summary: &ToxicGovernanceLedgerSummaryResponse) -> String {
    let mut lines = vec![
        "# Toxic Governance Ledger".to_string(),
        String::new(),
        "## Summary".to_string(),
        format!("- Selected Symbol: {}", summary.selected_symbol),
        format!("- Status: {}", summary.status),
        format!("- Consensus Status: {}", summary.consensus_status),
        format!("- Total Decisions: {}", summary.total_decisions),
        format!("- Accepted Recommendations: {}", summary.accept_count),
        format!("- Rejected Recommendations: {}", summary.reject_count),
        format!("- Watch More: {}", summary.watch_more_count),
        format!("- Needs More Samples: {}", summary.needs_more_samples_count),
        format!("- Suppressed For Now: {}", summary.suppress_for_now_count),
        format!("- Escalated Review: {}", summary.escalate_review_count),
        String::new(),
        "## Recent Governance Notes".to_string(),
    ];

    if summary.recent_governance_notes.is_empty() {
        lines.push("- None".to_string());
    } else {
        for note in &summary.recent_governance_notes {
            lines.push(format!("- {note}"));
        }
    }

    lines.push(String::new());
    lines.push("## Decisions".to_string());
    if summary.decisions.is_empty() {
        lines.push("- None".to_string());
    } else {
        for decision in &summary.decisions {
            lines.push(format!(
                "- {} / {} / {:?}: {:?} by {} at {}",
                decision.symbol,
                decision.signal_type,
                decision.recommendation,
                decision.decision,
                decision.reviewer,
                decision.created_at_ms
            ));
            lines.push(format!("  - Reason: {}", decision.reason));
            lines.push(format!("  - Notes: {}", decision.notes));
            lines.push(format!(
                "  - Evidence Summary: {}",
                if decision.evidence_summary.is_empty() {
                    "none".to_string()
                } else {
                    decision.evidence_summary.join("; ")
                }
            ));
        }
    }

    lines.join("\n")
}
