use btc_toxic_flow_monitor_rs::{
    toxicity::toxic_governance_proposal::{
        build_toxic_governance_proposal_export, build_toxic_governance_proposal_status,
        build_toxic_governance_proposal_summary,
    },
    types::{
        toxic_governance_ledger::{ToxicGovernanceDecision, ToxicGovernanceDecisionKind},
        toxic_governance_proposal::ToxicGovernanceProposalAction,
        toxic_weight_recommendation::ToxicWeightRecommendationKind,
        toxic_weight_review::{
            ToxicWeightReviewItem, ToxicWeightReviewSummaryResponse, ToxicWeightReviewSymbolSummary,
        },
    },
};

#[test]
fn governance_proposal_merges_recommendations_and_decisions_without_runtime_mutation() {
    let review_summary = ToxicWeightReviewSummaryResponse {
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
        selected_symbol: "BTC-PERP".to_string(),
        status: "weight_review_ready".to_string(),
        warnings: vec!["review_warning".to_string()],
        total_items: 4,
        manual_review_required_count: 4,
        keep_count: 1,
        upgrade_candidate_count: 1,
        downgrade_candidate_count: 1,
        no_trade_only_count: 1,
        disable_candidate_count: 0,
        insufficient_data_count: 0,
        governance_notes: vec!["manual review required".to_string()],
        review_items: vec![
            review_item(
                "BTC-PERP",
                "keep_signal",
                ToxicWeightRecommendationKind::Keep,
            ),
            review_item(
                "BTC-PERP",
                "upgrade_signal",
                ToxicWeightRecommendationKind::SlightUpgradeCandidate,
            ),
            review_item(
                "BTC-PERP",
                "downgrade_signal",
                ToxicWeightRecommendationKind::DowngradeCandidate,
            ),
            review_item(
                "ETH-PERP",
                "no_trade_signal",
                ToxicWeightRecommendationKind::NoTradeOnlyCandidate,
            ),
        ],
        by_symbol: vec![
            ToxicWeightReviewSymbolSummary {
                symbol: "BTC-PERP".to_string(),
                total_items: 3,
                manual_review_required_count: 3,
                keep_count: 1,
                upgrade_candidate_count: 1,
                downgrade_candidate_count: 1,
                no_trade_only_count: 0,
                disable_candidate_count: 0,
                insufficient_data_count: 0,
            },
            ToxicWeightReviewSymbolSummary {
                symbol: "ETH-PERP".to_string(),
                total_items: 1,
                manual_review_required_count: 1,
                keep_count: 0,
                upgrade_candidate_count: 0,
                downgrade_candidate_count: 0,
                no_trade_only_count: 1,
                disable_candidate_count: 0,
                insufficient_data_count: 0,
            },
        ],
    };

    let decisions = vec![
        decision(
            "BTC-PERP",
            "keep_signal",
            ToxicWeightRecommendationKind::Keep,
            ToxicGovernanceDecisionKind::AcceptRecommendation,
        ),
        decision(
            "BTC-PERP",
            "upgrade_signal",
            ToxicWeightRecommendationKind::SlightUpgradeCandidate,
            ToxicGovernanceDecisionKind::WatchMore,
        ),
        decision(
            "ETH-PERP",
            "no_trade_signal",
            ToxicWeightRecommendationKind::NoTradeOnlyCandidate,
            ToxicGovernanceDecisionKind::SuppressForNow,
        ),
    ];

    let summary = build_toxic_governance_proposal_summary(&review_summary, &decisions, Vec::new());
    let status = build_toxic_governance_proposal_status(&summary);
    let export = build_toxic_governance_proposal_export(&summary);

    assert!(summary.read_only);
    assert!(summary.proposal_only);
    assert!(!summary.runtime_weight_modified);
    assert!(!summary.config_modified);
    assert!(!summary.runtime_modified);
    assert!(!summary.auto_apply_enabled);
    assert!(!summary.strategy_reloaded);
    assert_eq!(summary.total_proposals, 4);
    assert_eq!(summary.by_action.keep, 1);
    assert_eq!(summary.by_action.slight_upgrade_candidate, 0);
    assert_eq!(summary.by_action.downgrade_candidate, 1);
    assert_eq!(summary.by_action.no_trade_only_candidate, 1);
    assert_eq!(summary.by_action.needs_more_samples, 1);

    let upgrade = summary
        .items
        .iter()
        .find(|item| item.signal_type == "upgrade_signal")
        .expect("upgrade proposal");
    assert_eq!(
        upgrade.proposed_action,
        ToxicGovernanceProposalAction::NeedsMoreSamples
    );
    assert_eq!(upgrade.proposal_status, "watch_more");

    let keep = summary
        .items
        .iter()
        .find(|item| item.signal_type == "keep_signal")
        .expect("keep proposal");
    assert_eq!(keep.proposed_action, ToxicGovernanceProposalAction::Keep);
    assert_eq!(keep.proposal_status, "accepted_by_governance");

    assert!(status.read_only);
    assert!(status.proposal_only);
    assert!(!status.runtime_weight_modified);
    assert!(!status.config_modified);
    assert!(!status.runtime_modified);
    assert!(!status.auto_apply_enabled);
    assert!(!status.strategy_reloaded);

    assert!(export.read_only);
    assert!(export.proposal_only);
    assert!(export
        .markdown_report
        .contains("# Toxic Governance Proposals"));
}

fn review_item(
    symbol: &str,
    signal_type: &str,
    recommended_action: ToxicWeightRecommendationKind,
) -> ToxicWeightReviewItem {
    ToxicWeightReviewItem {
        symbol: symbol.to_string(),
        signal_type: signal_type.to_string(),
        sample_count: 32,
        aligned_ratio: 0.61,
        adverse_ratio: 0.21,
        neutral_ratio: 0.18,
        best_window: Some("+5m".to_string()),
        worst_window: Some("+15m".to_string()),
        recommended_action,
        confidence: "medium".to_string(),
        evidence_summary: vec!["sample_count=32".to_string()],
        reason_codes: vec!["reason_code".to_string()],
        governance_notes: vec!["governance_note".to_string()],
        manual_review_required: true,
        export_only: true,
        runtime_weight_modified: false,
        config_modified: false,
        runtime_modified: false,
        auto_apply_enabled: false,
    }
}

fn decision(
    symbol: &str,
    signal_type: &str,
    recommendation: ToxicWeightRecommendationKind,
    decision: ToxicGovernanceDecisionKind,
) -> ToxicGovernanceDecision {
    ToxicGovernanceDecision {
        id: format!("{symbol}-{signal_type}"),
        symbol: symbol.to_string(),
        signal_type: signal_type.to_string(),
        recommendation,
        decision,
        reviewer: "operator".to_string(),
        reason: "governance_reason".to_string(),
        notes: "governance_notes".to_string(),
        confidence: 0.74,
        evidence_summary: vec!["ledger_evidence".to_string()],
        created_at_ms: 1_700_000_000_000,
        read_only: true,
        governance_ledger_only: true,
        runtime_weight_modified: false,
        config_modified: false,
        runtime_modified: false,
        auto_apply_enabled: false,
    }
}
