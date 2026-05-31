use btc_toxic_flow_monitor_rs::{
    toxicity::toxic_governance_signoff_pack::{
        build_toxic_governance_signoff_pack_export, build_toxic_governance_signoff_pack_status,
        build_toxic_governance_signoff_pack_summary,
    },
    types::{
        toxic_governance_ledger::ToxicGovernanceDecisionKind,
        toxic_governance_proposal::{
            ToxicGovernanceProposalAction, ToxicGovernanceProposalByAction,
        },
        toxic_governance_review_pack::{
            ToxicGovernanceReviewPackDecisionSummary, ToxicGovernanceReviewPackItem,
            ToxicGovernanceReviewPackSummaryResponse,
        },
        toxic_weight_recommendation::ToxicWeightRecommendationKind,
    },
};

#[test]
fn governance_signoff_pack_stays_read_only_and_blocks_pending_items() {
    let review_pack = ToxicGovernanceReviewPackSummaryResponse {
        read_only: true,
        proposal_only: true,
        review_pack_only: true,
        runtime_weight_modified: false,
        config_modified: false,
        runtime_modified: false,
        auto_apply_enabled: false,
        strategy_reloaded: false,
        mode: "governance_review_pack_only".to_string(),
        selected_symbol: "BTC-PERP".to_string(),
        status: "governance_review_pack_ready".to_string(),
        ready_for_manual_review: true,
        warnings: Vec::new(),
        total_items: 3,
        by_action: ToxicGovernanceProposalByAction {
            keep: 1,
            slight_upgrade_candidate: 0,
            slight_downgrade_candidate: 0,
            downgrade_candidate: 1,
            no_trade_only_candidate: 1,
            disable_candidate: 0,
            needs_more_samples: 0,
        },
        by_decision: ToxicGovernanceReviewPackDecisionSummary {
            accepted_count: 1,
            rejected_count: 0,
            watch_more_count: 1,
            needs_more_samples_count: 0,
            suppress_for_now_count: 1,
            escalate_review_count: 0,
            pending_governance_review_count: 0,
        },
        recent_governance_notes: vec!["operator note".to_string()],
        items: vec![
            review_item(
                "BTC-PERP",
                "keep_signal",
                Some(ToxicGovernanceDecisionKind::AcceptRecommendation),
                ToxicGovernanceProposalAction::Keep,
                "accepted_by_governance",
            ),
            review_item(
                "BTC-PERP",
                "watch_signal",
                Some(ToxicGovernanceDecisionKind::WatchMore),
                ToxicGovernanceProposalAction::NeedsMoreSamples,
                "watch_more",
            ),
            review_item(
                "ETH-PERP",
                "suppress_signal",
                Some(ToxicGovernanceDecisionKind::SuppressForNow),
                ToxicGovernanceProposalAction::NoTradeOnlyCandidate,
                "suppressed_for_now",
            ),
        ],
        by_signal_type: Vec::new(),
        by_symbol: Vec::new(),
    };

    let summary = build_toxic_governance_signoff_pack_summary(&review_pack);
    let status = build_toxic_governance_signoff_pack_status(&summary);
    let export = build_toxic_governance_signoff_pack_export(&summary);

    assert!(summary.read_only);
    assert!(summary.proposal_only);
    assert!(summary.review_pack_only);
    assert!(summary.signoff_pack_only);
    assert!(!summary.runtime_weight_modified);
    assert!(!summary.config_modified);
    assert!(!summary.runtime_modified);
    assert!(!summary.auto_apply_enabled);
    assert!(!summary.strategy_reloaded);
    assert!(!summary.ready_for_manual_signoff);
    assert_eq!(summary.total_items, 3);
    assert_eq!(summary.ready_for_signoff_count, 2);
    assert_eq!(summary.hold_count, 1);
    assert!(summary
        .blocked_reasons
        .contains(&"watch_more_items_present".to_string()));

    let keep = summary
        .items
        .iter()
        .find(|item| item.signal_type == "keep_signal")
        .expect("keep item");
    assert_eq!(keep.signoff_recommendation, "ready_for_manual_signoff");
    assert!(keep.blocked_reason.is_none());

    let watch = summary
        .items
        .iter()
        .find(|item| item.signal_type == "watch_signal")
        .expect("watch item");
    assert_eq!(watch.signoff_recommendation, "hold_for_review");
    assert_eq!(watch.blocked_reason.as_deref(), Some("watch_more"));

    assert!(status.read_only);
    assert!(status.signoff_pack_only);
    assert!(!status.ready_for_manual_signoff);
    assert!(status
        .blocked_reasons
        .contains(&"watch_more_items_present".to_string()));

    assert!(export.read_only);
    assert!(export.signoff_pack_only);
    assert!(export
        .markdown_report
        .contains("# Toxic Governance Signoff Pack"));
}

fn review_item(
    symbol: &str,
    signal_type: &str,
    governance_decision: Option<ToxicGovernanceDecisionKind>,
    proposed_action: ToxicGovernanceProposalAction,
    proposal_status: &str,
) -> ToxicGovernanceReviewPackItem {
    ToxicGovernanceReviewPackItem {
        symbol: symbol.to_string(),
        signal_type: signal_type.to_string(),
        recommended_action: ToxicWeightRecommendationKind::Keep,
        governance_decision,
        proposed_action,
        proposal_status: proposal_status.to_string(),
        sample_count: 20,
        aligned_ratio: 0.62,
        adverse_ratio: 0.20,
        neutral_ratio: 0.18,
        confidence: "medium".to_string(),
        reason_codes: vec!["reason_code".to_string()],
        evidence_summary: vec!["evidence".to_string()],
        governance_notes: vec!["note".to_string()],
        manual_review_required: true,
        read_only: true,
        review_pack_only: true,
        proposal_only: true,
        runtime_weight_modified: false,
        config_modified: false,
        runtime_modified: false,
        auto_apply_enabled: false,
        strategy_reloaded: false,
    }
}
