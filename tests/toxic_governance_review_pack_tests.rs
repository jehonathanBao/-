use btc_toxic_flow_monitor_rs::{
    toxicity::toxic_governance_review_pack::{
        build_toxic_governance_review_pack_export, build_toxic_governance_review_pack_status,
        build_toxic_governance_review_pack_summary,
    },
    types::{
        toxic_governance_ledger::ToxicGovernanceDecisionKind,
        toxic_governance_proposal::{
            ToxicGovernanceProposalAction, ToxicGovernanceProposalByAction,
            ToxicGovernanceProposalItem, ToxicGovernanceProposalSummaryResponse,
        },
    },
};

#[test]
fn governance_review_pack_stays_read_only_and_tracks_decision_counts() {
    let proposal_summary = ToxicGovernanceProposalSummaryResponse {
        read_only: true,
        proposal_only: true,
        runtime_weight_modified: false,
        config_modified: false,
        runtime_modified: false,
        auto_apply_enabled: false,
        strategy_reloaded: false,
        mode: "proposal_draft_only".to_string(),
        selected_symbol: "BTC-PERP".to_string(),
        status: "governance_proposals_ready".to_string(),
        warnings: vec!["proposal_warning".to_string()],
        total_proposals: 3,
        by_action: ToxicGovernanceProposalByAction {
            keep: 1,
            slight_upgrade_candidate: 0,
            slight_downgrade_candidate: 0,
            downgrade_candidate: 1,
            no_trade_only_candidate: 0,
            disable_candidate: 0,
            needs_more_samples: 1,
        },
        items: vec![
            proposal_item(
                "BTC-PERP",
                "keep_signal",
                ToxicGovernanceProposalAction::Keep,
                Some(ToxicGovernanceDecisionKind::AcceptRecommendation),
                "accepted_by_governance",
            ),
            proposal_item(
                "BTC-PERP",
                "watch_signal",
                ToxicGovernanceProposalAction::NeedsMoreSamples,
                Some(ToxicGovernanceDecisionKind::WatchMore),
                "watch_more",
            ),
            proposal_item(
                "ETH-PERP",
                "downgrade_signal",
                ToxicGovernanceProposalAction::DowngradeCandidate,
                None,
                "pending_governance_review",
            ),
        ],
        by_signal_type: Vec::new(),
        by_symbol: Vec::new(),
    };

    let summary = build_toxic_governance_review_pack_summary(&proposal_summary);
    let status = build_toxic_governance_review_pack_status(&summary);
    let export = build_toxic_governance_review_pack_export(&summary);

    assert!(summary.read_only);
    assert!(summary.proposal_only);
    assert!(summary.review_pack_only);
    assert!(summary.ready_for_manual_review);
    assert!(!summary.runtime_weight_modified);
    assert!(!summary.config_modified);
    assert!(!summary.runtime_modified);
    assert!(!summary.auto_apply_enabled);
    assert!(!summary.strategy_reloaded);
    assert_eq!(summary.total_items, 3);
    assert_eq!(summary.by_decision.accepted_count, 1);
    assert_eq!(summary.by_decision.watch_more_count, 1);
    assert_eq!(summary.by_decision.pending_governance_review_count, 1);
    assert_eq!(summary.by_signal_type.len(), 3);
    assert_eq!(summary.by_symbol.len(), 2);

    let pending = summary
        .items
        .iter()
        .find(|item| item.signal_type == "downgrade_signal")
        .expect("pending item");
    assert!(pending.review_pack_only);
    assert_eq!(pending.proposal_status, "pending_governance_review");

    assert!(status.read_only);
    assert!(status.proposal_only);
    assert!(status.review_pack_only);
    assert!(!status.runtime_weight_modified);
    assert!(!status.config_modified);
    assert!(!status.runtime_modified);
    assert!(!status.auto_apply_enabled);
    assert!(!status.strategy_reloaded);

    assert!(export.read_only);
    assert!(export.proposal_only);
    assert!(export.review_pack_only);
    assert!(export
        .markdown_report
        .contains("# Toxic Governance Review Pack"));
}

fn proposal_item(
    symbol: &str,
    signal_type: &str,
    proposed_action: ToxicGovernanceProposalAction,
    governance_decision: Option<ToxicGovernanceDecisionKind>,
    proposal_status: &str,
) -> ToxicGovernanceProposalItem {
    ToxicGovernanceProposalItem {
        symbol: symbol.to_string(),
        signal_type: signal_type.to_string(),
        recommended_action: btc_toxic_flow_monitor_rs::types::toxic_weight_recommendation::ToxicWeightRecommendationKind::Keep,
        governance_decision,
        proposed_action,
        sample_count: 24,
        aligned_ratio: 0.58,
        adverse_ratio: 0.26,
        neutral_ratio: 0.16,
        confidence: "medium".to_string(),
        reason_codes: vec!["reason_code".to_string()],
        evidence_summary: vec!["evidence".to_string()],
        governance_notes: vec!["note".to_string()],
        proposal_status: proposal_status.to_string(),
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
