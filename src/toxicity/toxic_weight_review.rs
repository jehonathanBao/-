use std::collections::BTreeMap;

use crate::types::{
    toxic_weight_recommendation::{
        ToxicWeightRecommendationItem, ToxicWeightRecommendationKind,
        ToxicWeightRecommendationSummaryResponse,
    },
    toxic_weight_review::{
        ToxicWeightReviewExportResponse, ToxicWeightReviewItem, ToxicWeightReviewStatusResponse,
        ToxicWeightReviewSummaryResponse, ToxicWeightReviewSymbolSummary,
    },
};

pub fn build_toxic_weight_review_summary(
    recommendations: &ToxicWeightRecommendationSummaryResponse,
) -> ToxicWeightReviewSummaryResponse {
    let review_items = recommendations
        .recommendations
        .iter()
        .map(build_review_item)
        .collect::<Vec<_>>();
    let by_symbol = build_symbol_summaries(&review_items);
    let governance_notes = build_governance_notes();

    ToxicWeightReviewSummaryResponse {
        read_only: true,
        analysis_only: true,
        execution_enabled: false,
        manual_review_required: true,
        export_only: true,
        runtime_modified: false,
        runtime_weight_modified: false,
        config_modified: false,
        auto_apply_enabled: false,
        mode: "review_export_only".to_string(),
        selected_symbol: recommendations.selected_symbol.clone(),
        status: if review_items.is_empty() {
            "no_weight_review_items".to_string()
        } else {
            "weight_review_ready".to_string()
        },
        warnings: recommendations.warnings.clone(),
        total_items: review_items.len(),
        manual_review_required_count: review_items.len(),
        keep_count: count_review_items(&review_items, ToxicWeightRecommendationKind::Keep),
        upgrade_candidate_count: count_review_items(
            &review_items,
            ToxicWeightRecommendationKind::SlightUpgradeCandidate,
        ),
        downgrade_candidate_count: review_items
            .iter()
            .filter(|item| {
                matches!(
                    item.recommended_action,
                    ToxicWeightRecommendationKind::SlightDowngradeCandidate
                        | ToxicWeightRecommendationKind::DowngradeCandidate
                )
            })
            .count(),
        no_trade_only_count: count_review_items(
            &review_items,
            ToxicWeightRecommendationKind::NoTradeOnlyCandidate,
        ),
        disable_candidate_count: count_review_items(
            &review_items,
            ToxicWeightRecommendationKind::DisableCandidate,
        ),
        insufficient_data_count: count_review_items(
            &review_items,
            ToxicWeightRecommendationKind::InsufficientData,
        ),
        governance_notes,
        review_items,
        by_symbol,
    }
}

pub fn build_toxic_weight_review_status(
    summary: &ToxicWeightReviewSummaryResponse,
) -> ToxicWeightReviewStatusResponse {
    ToxicWeightReviewStatusResponse {
        read_only: true,
        analysis_only: true,
        execution_enabled: false,
        manual_review_required: true,
        export_only: true,
        runtime_modified: false,
        runtime_weight_modified: false,
        config_modified: false,
        auto_apply_enabled: false,
        enabled: true,
        mode: "review_export_only".to_string(),
        selected_symbol: summary.selected_symbol.clone(),
        status: summary.status.clone(),
        total_items: summary.total_items,
        manual_review_required_count: summary.manual_review_required_count,
        safety_boundary: vec![
            "readOnly=true".to_string(),
            "analysisOnly=true".to_string(),
            "exportOnly=true".to_string(),
            "runtimeModified=false".to_string(),
            "runtimeWeightModified=false".to_string(),
            "configModified=false".to_string(),
            "autoApplyEnabled=false".to_string(),
            "No runtime config mutation".to_string(),
            "No strategy reload".to_string(),
            "No calibration_runner".to_string(),
        ],
    }
}

pub fn build_toxic_weight_review_export(
    summary: &ToxicWeightReviewSummaryResponse,
) -> ToxicWeightReviewExportResponse {
    let recommendation_summary = ToxicWeightReviewSymbolSummary {
        symbol: summary.selected_symbol.clone(),
        total_items: summary.total_items,
        manual_review_required_count: summary.manual_review_required_count,
        keep_count: summary.keep_count,
        upgrade_candidate_count: summary.upgrade_candidate_count,
        downgrade_candidate_count: summary.downgrade_candidate_count,
        no_trade_only_count: summary.no_trade_only_count,
        disable_candidate_count: summary.disable_candidate_count,
        insufficient_data_count: summary.insufficient_data_count,
    };
    let manual_review_checklist = build_manual_review_checklist();
    let do_not_apply_conditions = build_do_not_apply_conditions();
    let rollback_notes = build_rollback_notes();
    let evidence_sources = build_evidence_sources();
    let governance_notes_markdown = build_governance_notes_markdown(summary);

    ToxicWeightReviewExportResponse {
        read_only: true,
        analysis_only: true,
        execution_enabled: false,
        manual_review_required: true,
        export_only: true,
        runtime_modified: false,
        runtime_weight_modified: false,
        config_modified: false,
        auto_apply_enabled: false,
        mode: "review_export_only".to_string(),
        selected_symbol: summary.selected_symbol.clone(),
        status: summary.status.clone(),
        total_items: summary.total_items,
        recommendation_summary,
        manual_review_checklist,
        governance_notes: summary.governance_notes.clone(),
        governance_notes_markdown,
        do_not_apply_conditions,
        rollback_notes,
        evidence_sources,
        review_items: summary.review_items.clone(),
        markdown_report: build_markdown_report(summary),
    }
}

fn build_review_item(item: &ToxicWeightRecommendationItem) -> ToxicWeightReviewItem {
    ToxicWeightReviewItem {
        symbol: item.symbol.clone(),
        signal_type: item.signal_type.clone(),
        sample_count: item.sample_count,
        aligned_ratio: item.aligned_ratio,
        adverse_ratio: item.adverse_ratio,
        neutral_ratio: item.neutral_ratio,
        best_window: item.best_window.clone(),
        worst_window: item.worst_window.clone(),
        recommended_action: item.recommendation,
        confidence: item.confidence.clone(),
        evidence_summary: item.evidence.clone(),
        reason_codes: item.reason_codes.clone(),
        governance_notes: governance_notes_for(item),
        manual_review_required: true,
        export_only: true,
        runtime_weight_modified: false,
        config_modified: false,
        runtime_modified: false,
        auto_apply_enabled: false,
    }
}

fn governance_notes_for(item: &ToxicWeightRecommendationItem) -> Vec<String> {
    let mut notes = vec!["Review and export only. Do not auto-apply.".to_string()];
    match item.recommendation {
        ToxicWeightRecommendationKind::Keep => notes.push(
            "Keep looks stable, but operator review should confirm broader cross-signal context."
                .to_string(),
        ),
        ToxicWeightRecommendationKind::SlightUpgradeCandidate => notes.push(
            "Only consider a modest upgrade after manual review of recent markout quality."
                .to_string(),
        ),
        ToxicWeightRecommendationKind::SlightDowngradeCandidate => notes.push(
            "Use a cautious downgrade only if this weakness persists across the next review cycle."
                .to_string(),
        ),
        ToxicWeightRecommendationKind::DowngradeCandidate => notes.push(
            "Treat this as a downgrade candidate before the next parameter review export."
                .to_string(),
        ),
        ToxicWeightRecommendationKind::NoTradeOnlyCandidate => notes.push(
            "Prefer converting this signal into a no-trade gate instead of a directional weight."
                .to_string(),
        ),
        ToxicWeightRecommendationKind::DisableCandidate => notes.push(
            "Disable is a manual-only recommendation and needs explicit operator sign-off."
                .to_string(),
        ),
        ToxicWeightRecommendationKind::InsufficientData => notes.push(
            "Do not adjust until the markout sample set grows beyond the minimum threshold."
                .to_string(),
        ),
    }
    if let Some(best_window) = &item.best_window {
        notes.push(format!("Best supporting markout window: {best_window}."));
    }
    if let Some(worst_window) = &item.worst_window {
        notes.push(format!("Worst supporting markout window: {worst_window}."));
    }
    notes
}

fn build_symbol_summaries(items: &[ToxicWeightReviewItem]) -> Vec<ToxicWeightReviewSymbolSummary> {
    let mut buckets = BTreeMap::<String, ToxicWeightReviewSymbolSummary>::new();
    for item in items {
        let bucket =
            buckets
                .entry(item.symbol.clone())
                .or_insert_with(|| ToxicWeightReviewSymbolSummary {
                    symbol: item.symbol.clone(),
                    total_items: 0,
                    manual_review_required_count: 0,
                    keep_count: 0,
                    upgrade_candidate_count: 0,
                    downgrade_candidate_count: 0,
                    no_trade_only_count: 0,
                    disable_candidate_count: 0,
                    insufficient_data_count: 0,
                });
        bucket.total_items += 1;
        bucket.manual_review_required_count += 1;
        match item.recommended_action {
            ToxicWeightRecommendationKind::Keep => bucket.keep_count += 1,
            ToxicWeightRecommendationKind::SlightUpgradeCandidate => {
                bucket.upgrade_candidate_count += 1;
            }
            ToxicWeightRecommendationKind::SlightDowngradeCandidate
            | ToxicWeightRecommendationKind::DowngradeCandidate => {
                bucket.downgrade_candidate_count += 1;
            }
            ToxicWeightRecommendationKind::NoTradeOnlyCandidate => {
                bucket.no_trade_only_count += 1;
            }
            ToxicWeightRecommendationKind::DisableCandidate => {
                bucket.disable_candidate_count += 1;
            }
            ToxicWeightRecommendationKind::InsufficientData => {
                bucket.insufficient_data_count += 1;
            }
        }
    }
    buckets.into_values().collect()
}

fn count_review_items(
    items: &[ToxicWeightReviewItem],
    action: ToxicWeightRecommendationKind,
) -> usize {
    items
        .iter()
        .filter(|item| item.recommended_action == action)
        .count()
}

fn build_governance_notes() -> Vec<String> {
    vec![
        "This layer is review/export only and does not apply weights automatically.".to_string(),
        "Review flags and recommendation strength should guide manual parameter discussion, not runtime mutation."
            .to_string(),
        "Manual review is required before any config patch, runtime reload, or strategy change."
            .to_string(),
    ]
}

fn build_manual_review_checklist() -> Vec<String> {
    vec![
        "Confirm the selected symbol and review window match the intended governance scope."
            .to_string(),
        "Review T9 quality scorecard ratios before accepting any recommendation bucket."
            .to_string(),
        "Review T10 recommendation confidence, evidence summary, and reason codes for each item."
            .to_string(),
        "Verify no do-not-apply condition is active before preparing a manual parameter patch."
            .to_string(),
        "Record operator approval or rejection notes before any downstream export or governance handoff."
            .to_string(),
    ]
}

fn build_do_not_apply_conditions() -> Vec<String> {
    vec![
        "Insufficient data remains unresolved for the affected signal kind.".to_string(),
        "Recommendation confidence is low or evidence is directionally conflicted.".to_string(),
        "Recent markout quality has degraded into weak or bad buckets.".to_string(),
        "Manual reviewer has not signed off on the recommendation summary.".to_string(),
        "Runtime mutation, auto weight update, reload, or calibration_runner would be required."
            .to_string(),
    ]
}

fn build_rollback_notes() -> Vec<String> {
    vec![
        "This export does not modify runtime or config, so rollback is documentation-only at this stage."
            .to_string(),
        "If a later manual patch is created, preserve the previous weight values in the review packet."
            .to_string(),
        "Any future applied change should include a manual rollback patch and operator sign-off note."
            .to_string(),
    ]
}

fn build_evidence_sources() -> Vec<String> {
    vec![
        "T9 Toxic Signal Quality Scorecard / Markout Summary".to_string(),
        "T10 Toxic Signal Weight Recommendation / Read-only Parameter Suggestion".to_string(),
    ]
}

fn build_governance_notes_markdown(summary: &ToxicWeightReviewSummaryResponse) -> String {
    let mut lines = vec!["## Governance Notes".to_string(), String::new()];
    for note in &summary.governance_notes {
        lines.push(format!("- {note}"));
    }
    lines.join("\n")
}

fn build_markdown_report(summary: &ToxicWeightReviewSummaryResponse) -> String {
    let mut lines = vec![
        "# Toxic Weight Manual Review".to_string(),
        String::new(),
        "## Summary".to_string(),
        format!("- Selected Symbol: {}", summary.selected_symbol),
        format!("- Status: {}", summary.status),
        format!("- Total Items: {}", summary.total_items),
        format!(
            "- Manual Review Required: {}",
            summary.manual_review_required_count
        ),
        format!("- Keep: {}", summary.keep_count),
        format!("- Upgrade Candidates: {}", summary.upgrade_candidate_count),
        format!(
            "- Downgrade Candidates: {}",
            summary.downgrade_candidate_count
        ),
        format!("- No-trade Only: {}", summary.no_trade_only_count),
        format!("- Disable Candidates: {}", summary.disable_candidate_count),
        format!("- Insufficient Data: {}", summary.insufficient_data_count),
        String::new(),
        "## Governance Notes".to_string(),
    ];
    for note in &summary.governance_notes {
        lines.push(format!("- {note}"));
    }
    lines.push(String::new());
    lines.push("## Review Items".to_string());
    for item in &summary.review_items {
        lines.push(format!(
            "- {} / {}: {:?}, confidence {}, samples {}, aligned {:.2}%, adverse {:.2}%",
            item.symbol,
            item.signal_type,
            item.recommended_action,
            item.confidence,
            item.sample_count,
            item.aligned_ratio * 100.0,
            item.adverse_ratio * 100.0
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
    lines.join("\n")
}
