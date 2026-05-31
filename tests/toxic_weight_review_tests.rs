use btc_toxic_flow_monitor_rs::{
    toxicity::toxic_weight_review::{
        build_toxic_weight_review_export, build_toxic_weight_review_status,
        build_toxic_weight_review_summary,
    },
    types::{
        toxic_weight_recommendation::{
            ToxicWeightRecommendationItem, ToxicWeightRecommendationKind,
            ToxicWeightRecommendationReviewFlagSummary, ToxicWeightRecommendationSummaryResponse,
            ToxicWeightRecommendationSymbolSummary,
        },
        toxic_weight_review::ToxicWeightReviewSummaryResponse,
    },
};

#[test]
fn weight_review_wraps_recommendations_as_manual_export_only_items() {
    let recommendation_summary = ToxicWeightRecommendationSummaryResponse {
        read_only: true,
        analysis_only: true,
        execution_enabled: false,
        manual_review_required: true,
        runtime_modified: false,
        runtime_weight_modified: false,
        config_modified: false,
        mode: "analysis_only".to_string(),
        selected_symbol: "BTC-PERP".to_string(),
        status: "weight_recommendations_ready".to_string(),
        warnings: vec!["review warning".to_string()],
        total_recommendations: 3,
        keep_count: 1,
        slight_upgrade_candidate_count: 0,
        slight_downgrade_candidate_count: 0,
        downgrade_candidate_count: 1,
        no_trade_only_candidate_count: 0,
        disable_candidate_count: 0,
        insufficient_data_count: 1,
        recommendations: vec![
            recommendation_item("keep_signal", ToxicWeightRecommendationKind::Keep, "medium"),
            recommendation_item(
                "downgrade_signal",
                ToxicWeightRecommendationKind::DowngradeCandidate,
                "high",
            ),
            recommendation_item(
                "insufficient_signal",
                ToxicWeightRecommendationKind::InsufficientData,
                "low",
            ),
        ],
        by_signal_type: Vec::new(),
        by_symbol: vec![ToxicWeightRecommendationSymbolSummary {
            symbol: "BTC-PERP".to_string(),
            total_recommendations: 3,
            keep_count: 1,
            slight_upgrade_candidate_count: 0,
            slight_downgrade_candidate_count: 0,
            downgrade_candidate_count: 1,
            no_trade_only_candidate_count: 0,
            disable_candidate_count: 0,
            insufficient_data_count: 1,
            manual_review_required_count: 2,
        }],
        review_flags: vec![ToxicWeightRecommendationReviewFlagSummary {
            review_flag: "downgrade_manual_review".to_string(),
            count: 1,
            severity: "medium".to_string(),
            manual_review_required: true,
        }],
    };

    let summary = build_toxic_weight_review_summary(&recommendation_summary);
    let status = build_toxic_weight_review_status(&summary);
    let export = build_toxic_weight_review_export(&summary);

    assert!(summary.read_only);
    assert!(summary.analysis_only);
    assert!(summary.export_only);
    assert!(!summary.runtime_modified);
    assert!(!summary.runtime_weight_modified);
    assert!(!summary.config_modified);
    assert!(!summary.auto_apply_enabled);
    assert_eq!(summary.total_items, 3);
    assert_eq!(summary.manual_review_required_count, 3);
    assert_eq!(summary.keep_count, 1);
    assert_eq!(summary.downgrade_candidate_count, 1);
    assert_eq!(summary.insufficient_data_count, 1);
    assert!(!summary.governance_notes.is_empty());

    let keep = find_review_item(&summary, "keep_signal");
    assert!(keep.manual_review_required);
    assert!(keep.export_only);
    assert!(!keep.runtime_weight_modified);
    assert!(!keep.config_modified);
    assert!(!keep.runtime_modified);
    assert!(!keep.auto_apply_enabled);

    assert!(status.read_only);
    assert!(status.analysis_only);
    assert!(status.export_only);
    assert_eq!(status.total_items, 3);
    assert_eq!(status.manual_review_required_count, 3);

    assert!(export.read_only);
    assert!(export.export_only);
    assert_eq!(export.recommendation_summary.symbol, "BTC-PERP");
    assert!(!export.manual_review_checklist.is_empty());
    assert!(!export.do_not_apply_conditions.is_empty());
    assert!(!export.rollback_notes.is_empty());
    assert!(!export.evidence_sources.is_empty());
    assert!(export
        .governance_notes_markdown
        .contains("## Governance Notes"));
    assert!(export
        .markdown_report
        .contains("# Toxic Weight Manual Review"));
    assert!(export.markdown_report.contains("## Governance Notes"));
    assert!(!export.markdown_report.is_empty());
}

fn recommendation_item(
    signal_type: &str,
    recommendation: ToxicWeightRecommendationKind,
    confidence: &str,
) -> ToxicWeightRecommendationItem {
    ToxicWeightRecommendationItem {
        symbol: "BTC-PERP".to_string(),
        signal_type: signal_type.to_string(),
        sample_count: 24,
        aligned_ratio: 0.55,
        adverse_ratio: 0.20,
        neutral_ratio: 0.25,
        best_window: Some("+5m".to_string()),
        worst_window: Some("+15m".to_string()),
        recommendation,
        current_weight_hint: "read_only_not_loaded".to_string(),
        suggested_weight_hint: "keep_current_weight".to_string(),
        confidence: confidence.to_string(),
        reason_codes: vec![match recommendation {
            ToxicWeightRecommendationKind::Keep => "balanced_keep_zone".to_string(),
            ToxicWeightRecommendationKind::DowngradeCandidate => "downgrade_candidate".to_string(),
            ToxicWeightRecommendationKind::InsufficientData => "insufficient_data".to_string(),
            _ => "operator_review".to_string(),
        }],
        evidence: vec!["sample_count=24".to_string()],
        manual_review_required: recommendation != ToxicWeightRecommendationKind::Keep,
        runtime_weight_modified: false,
        config_modified: false,
    }
}

fn find_review_item<'a>(
    summary: &'a ToxicWeightReviewSummaryResponse,
    signal_type: &str,
) -> &'a btc_toxic_flow_monitor_rs::types::toxic_weight_review::ToxicWeightReviewItem {
    summary
        .review_items
        .iter()
        .find(|item| item.signal_type == signal_type)
        .expect("review item")
}
