use btc_toxic_flow_monitor_rs::{
    toxicity::toxic_governance_ledger::{
        build_toxic_governance_ledger_export, build_toxic_governance_ledger_status,
        build_toxic_governance_ledger_summary,
    },
    types::{
        toxic_governance_ledger::{ToxicGovernanceDecision, ToxicGovernanceDecisionKind},
        toxic_weight_recommendation::ToxicWeightRecommendationKind,
    },
};

#[test]
fn governance_ledger_aggregates_decisions_without_runtime_mutation() {
    let decisions = vec![
        decision(
            "btc-accept",
            "BTC-PERP",
            "fusion_signal",
            ToxicWeightRecommendationKind::Keep,
            ToxicGovernanceDecisionKind::AcceptRecommendation,
            "alice",
            1_700_000_000_000,
        ),
        decision(
            "btc-reject",
            "BTC-PERP",
            "trap_signal",
            ToxicWeightRecommendationKind::DowngradeCandidate,
            ToxicGovernanceDecisionKind::RejectRecommendation,
            "bob",
            1_700_000_000_100,
        ),
        decision(
            "btc-watch",
            "BTC-PERP",
            "markout_signal",
            ToxicWeightRecommendationKind::SlightDowngradeCandidate,
            ToxicGovernanceDecisionKind::WatchMore,
            "alice",
            1_700_000_000_200,
        ),
        decision(
            "btc-needs-more",
            "BTC-PERP",
            "vpin_signal",
            ToxicWeightRecommendationKind::InsufficientData,
            ToxicGovernanceDecisionKind::NeedsMoreSamples,
            "carol",
            1_700_000_000_300,
        ),
        decision(
            "eth-suppress",
            "ETH-PERP",
            "wall_signal",
            ToxicWeightRecommendationKind::NoTradeOnlyCandidate,
            ToxicGovernanceDecisionKind::SuppressForNow,
            "dave",
            1_700_000_000_400,
        ),
        decision(
            "eth-escalate",
            "ETH-PERP",
            "flow_signal",
            ToxicWeightRecommendationKind::DisableCandidate,
            ToxicGovernanceDecisionKind::EscalateReview,
            "erin",
            1_700_000_000_500,
        ),
    ];

    let summary = build_toxic_governance_ledger_summary("ALL", &decisions, Vec::new());
    let status = build_toxic_governance_ledger_status(&summary);
    let export = build_toxic_governance_ledger_export(&summary);

    assert!(summary.read_only);
    assert!(summary.analysis_only);
    assert!(summary.manual_review_required);
    assert!(summary.governance_ledger_only);
    assert!(!summary.runtime_weight_modified);
    assert!(!summary.config_modified);
    assert!(!summary.runtime_modified);
    assert!(!summary.auto_apply_enabled);
    assert!(!summary.strategy_reloaded);
    assert_eq!(summary.total_decisions, 6);
    assert_eq!(
        summary.manual_review_decision_placeholder,
        "manual_review_decision_placeholder_only_no_write_path"
    );
    assert!(!summary.evidence_lineage.is_empty());
    assert_eq!(summary.accept_count, 1);
    assert_eq!(summary.reject_count, 1);
    assert_eq!(summary.watch_more_count, 1);
    assert_eq!(summary.needs_more_samples_count, 1);
    assert_eq!(summary.suppress_for_now_count, 1);
    assert_eq!(summary.escalate_review_count, 1);
    assert_eq!(summary.consensus_status, "evidence_insufficient");
    assert_eq!(summary.by_symbol.len(), 2);
    assert_eq!(summary.by_signal_type.len(), 6);

    assert!(status.read_only);
    assert!(status.analysis_only);
    assert!(status.manual_review_required);
    assert!(status.governance_ledger_only);
    assert!(!status.runtime_weight_modified);
    assert!(!status.config_modified);
    assert!(!status.runtime_modified);
    assert!(!status.auto_apply_enabled);
    assert!(!status.strategy_reloaded);
    assert_eq!(status.total_decisions, 6);

    assert!(export.read_only);
    assert!(export.analysis_only);
    assert!(export.manual_review_required);
    assert!(export.governance_ledger_only);
    assert!(!export.runtime_weight_modified);
    assert!(!export.config_modified);
    assert!(!export.runtime_modified);
    assert!(!export.auto_apply_enabled);
    assert!(!export.strategy_reloaded);
    assert!(export.markdown_report.contains("# Toxic Governance Ledger"));
    assert!(!export.evidence_lineage.is_empty());
    assert!(export
        .markdown_report
        .contains("## Recent Governance Notes"));
}

#[test]
fn governance_ledger_detects_evidence_insufficient_consensus() {
    let decisions = vec![
        decision(
            "btc-watch",
            "BTC-PERP",
            "fusion_signal",
            ToxicWeightRecommendationKind::Keep,
            ToxicGovernanceDecisionKind::WatchMore,
            "alice",
            1_700_000_000_000,
        ),
        decision(
            "btc-needs-more",
            "BTC-PERP",
            "vpin_signal",
            ToxicWeightRecommendationKind::InsufficientData,
            ToxicGovernanceDecisionKind::NeedsMoreSamples,
            "bob",
            1_700_000_000_100,
        ),
        decision(
            "btc-needs-more-2",
            "BTC-PERP",
            "wall_signal",
            ToxicWeightRecommendationKind::InsufficientData,
            ToxicGovernanceDecisionKind::NeedsMoreSamples,
            "carol",
            1_700_000_000_200,
        ),
    ];

    let summary = build_toxic_governance_ledger_summary("BTC-PERP", &decisions, Vec::new());
    assert_eq!(summary.consensus_status, "evidence_insufficient");
    assert_eq!(summary.total_decisions, 3);
    assert_eq!(summary.watch_more_count, 1);
    assert_eq!(summary.needs_more_samples_count, 2);
}

fn decision(
    id: &str,
    symbol: &str,
    signal_type: &str,
    recommendation: ToxicWeightRecommendationKind,
    decision: ToxicGovernanceDecisionKind,
    reviewer: &str,
    created_at_ms: u64,
) -> ToxicGovernanceDecision {
    ToxicGovernanceDecision {
        id: id.to_string(),
        symbol: symbol.to_string(),
        signal_type: signal_type.to_string(),
        recommendation,
        decision,
        reviewer: reviewer.to_string(),
        reason: format!("{reviewer}_reason"),
        notes: format!("{reviewer}_notes"),
        confidence: 0.75,
        evidence_summary: vec!["markout_summary".to_string()],
        created_at_ms,
        read_only: true,
        governance_ledger_only: true,
        runtime_weight_modified: false,
        config_modified: false,
        runtime_modified: false,
        auto_apply_enabled: false,
    }
}
