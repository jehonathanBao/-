use btc_toxic_flow_monitor_rs::{
    toxicity::toxic_signal_inbox::{
        build_toxic_signal_inbox_detail, build_toxic_signal_inbox_recent,
        build_toxic_signal_inbox_status,
    },
    types::{
        toxic_flow::ToxicConfidence,
        toxic_governance_ledger::{
            ToxicGovernanceDecision, ToxicGovernanceDecisionKind,
            ToxicGovernanceLedgerSummaryResponse,
        },
        toxic_markout::{
            ToxicMarkoutOutcome, ToxicMarkoutRecentResponse, ToxicMarkoutSignal, ToxicMarkoutWindow,
        },
        toxic_quality_scorecard::{
            ToxicQualityScorecardBucket, ToxicQualityScorecardSummaryResponse,
        },
        toxic_replay::{ToxicReplayRecentResponse, ToxicReplaySignalSummary},
        toxic_signal::{
            ToxicChaseRisk, ToxicSignal, ToxicSignalDirection, ToxicSignalRecentResponse,
            ToxicSignalType, ToxicSupportingEvidence,
        },
        toxic_signal_inbox::ToxicSignalInboxOperatorAction,
        toxic_weight_recommendation::{
            ToxicWeightRecommendationItem, ToxicWeightRecommendationKind,
            ToxicWeightRecommendationSummaryResponse,
        },
    },
};

#[test]
fn signal_inbox_builds_items_with_safety_contract() {
    let fusion = fusion_recent(vec![fusion_signal(
        "fusion-1",
        "BTC-PERP",
        ToxicSignalType::ShortBiasToxicFlow,
    )]);
    let recent = build_toxic_signal_inbox_recent(
        "BTC-PERP",
        &fusion,
        &replay_recent(vec!["fusion-1"]),
        &markout_recent(vec![markout_signal(
            "fusion-1",
            "short_bias_toxic_flow",
            [
                ToxicMarkoutOutcome::Aligned,
                ToxicMarkoutOutcome::Neutral,
                ToxicMarkoutOutcome::NotEnoughData,
                ToxicMarkoutOutcome::NotEnoughData,
            ],
        )]),
        &quality_summary(vec![quality_bucket(
            "short_bias_toxic_flow",
            20,
            0.62,
            0.18,
        )]),
        &recommendation_summary(vec![recommendation_item(
            "short_bias_toxic_flow",
            ToxicWeightRecommendationKind::Keep,
        )]),
        &governance_summary(vec![decision("short_bias_toxic_flow")]),
    );
    let status = build_toxic_signal_inbox_status(&recent);

    assert!(recent.read_only);
    assert!(!recent.runtime_modified);
    assert!(recent.analysis_only);
    assert!(!recent.execution_enabled);
    assert!(recent.manual_review_required);
    assert!(!recent.runtime_weight_modified);
    assert!(!recent.config_modified);
    assert_eq!(recent.items.len(), 1);
    assert_eq!(
        recent.items[0].operator_action,
        ToxicSignalInboxOperatorAction::ReviewMarkout
    );
    assert_eq!(recent.items[0].markout.fifteen_minute, "not_enough_data");
    assert_eq!(recent.items[0].quality.quality_bucket, "good");

    assert!(status.read_only);
    assert!(!status.runtime_modified);
    assert!(status.analysis_only);
    assert!(!status.execution_enabled);
    assert_eq!(status.item_count, 1);
}

#[test]
fn signal_inbox_filters_by_symbol_and_handles_empty_signals() {
    let fusion = fusion_recent(vec![
        fusion_signal(
            "fusion-btc",
            "BTC-PERP",
            ToxicSignalType::ShortBiasToxicFlow,
        ),
        fusion_signal("fusion-eth", "ETH-PERP", ToxicSignalType::LongBiasToxicFlow),
    ]);
    let recent = build_toxic_signal_inbox_recent(
        "ETH-PERP",
        &fusion,
        &replay_recent(Vec::new()),
        &markout_recent(Vec::new()),
        &quality_summary(Vec::new()),
        &recommendation_summary(Vec::new()),
        &governance_summary(Vec::new()),
    );
    let empty = build_toxic_signal_inbox_recent(
        "SOL-PERP",
        &fusion,
        &replay_recent(Vec::new()),
        &markout_recent(Vec::new()),
        &quality_summary(Vec::new()),
        &recommendation_summary(Vec::new()),
        &governance_summary(Vec::new()),
    );

    assert_eq!(recent.items.len(), 1);
    assert_eq!(recent.items[0].symbol, "ETH-PERP");
    assert_eq!(empty.status, "empty_signal_inbox");
    assert!(empty.items.is_empty());
}

#[test]
fn no_trade_recommendation_maps_to_no_trade_warning() {
    let fusion = fusion_recent(vec![fusion_signal(
        "fusion-trap",
        "BTC-PERP",
        ToxicSignalType::TrapRisk,
    )]);
    let recent = build_toxic_signal_inbox_recent(
        "BTC-PERP",
        &fusion,
        &replay_recent(Vec::new()),
        &markout_recent(vec![markout_signal(
            "fusion-trap",
            "trap_risk",
            [
                ToxicMarkoutOutcome::Neutral,
                ToxicMarkoutOutcome::Neutral,
                ToxicMarkoutOutcome::Neutral,
                ToxicMarkoutOutcome::Neutral,
            ],
        )]),
        &quality_summary(vec![quality_bucket("trap_risk", 20, 0.20, 0.20)]),
        &recommendation_summary(vec![recommendation_item(
            "trap_risk",
            ToxicWeightRecommendationKind::NoTradeOnlyCandidate,
        )]),
        &governance_summary(Vec::new()),
    );

    assert_eq!(
        recent.items[0].operator_action,
        ToxicSignalInboxOperatorAction::NoTradeWarning
    );
    assert!(recent.items[0].recommendation.no_trade_only);
}

#[test]
fn missing_governance_ledger_is_not_an_error() {
    let fusion = fusion_recent(vec![fusion_signal(
        "fusion-1",
        "BTC-PERP",
        ToxicSignalType::ShortBiasToxicFlow,
    )]);
    let recent = build_toxic_signal_inbox_recent(
        "BTC-PERP",
        &fusion,
        &replay_recent(Vec::new()),
        &markout_recent(Vec::new()),
        &quality_summary(vec![quality_bucket(
            "short_bias_toxic_flow",
            20,
            0.62,
            0.18,
        )]),
        &recommendation_summary(vec![recommendation_item(
            "short_bias_toxic_flow",
            ToxicWeightRecommendationKind::Keep,
        )]),
        &governance_summary(Vec::new()),
    );
    let detail = build_toxic_signal_inbox_detail("BTC-PERP", "fusion-1", &recent);

    assert!(detail.available);
    assert!(!recent.items[0].governance.ledger_available);
    assert_eq!(
        recent.items[0].governance.latest_decision,
        "missing_ledger_evidence"
    );
}

#[test]
fn inbox_item_json_does_not_expose_execution_fields() {
    let fusion = fusion_recent(vec![fusion_signal(
        "fusion-1",
        "BTC-PERP",
        ToxicSignalType::ShortBiasToxicFlow,
    )]);
    let recent = build_toxic_signal_inbox_recent(
        "BTC-PERP",
        &fusion,
        &replay_recent(Vec::new()),
        &markout_recent(Vec::new()),
        &quality_summary(Vec::new()),
        &recommendation_summary(Vec::new()),
        &governance_summary(Vec::new()),
    );
    let json = serde_json::to_value(&recent.items[0]).expect("item json");

    assert!(json.get("operatorAction").is_some());
    assert!(json.get("executionAction").is_none());
    assert!(json.get("orderInstruction").is_none());
    assert!(json.get("tradeAction").is_none());
    assert!(json.get("walletAction").is_none());
}

fn fusion_recent(signals: Vec<ToxicSignal>) -> ToxicSignalRecentResponse {
    ToxicSignalRecentResponse {
        read_only: true,
        runtime_modified: false,
        analysis_only: true,
        execution_enabled: false,
        mode: "analysis_only".to_string(),
        selected_symbol: "BTC-PERP".to_string(),
        status: "fusion_ready".to_string(),
        warnings: Vec::new(),
        no_trade_reasons: Vec::new(),
        signals,
    }
}

fn fusion_signal(signal_id: &str, symbol: &str, signal_type: ToxicSignalType) -> ToxicSignal {
    ToxicSignal {
        signal_id: signal_id.to_string(),
        symbol: symbol.to_string(),
        ts_ms: 10_000,
        signal_type,
        direction: match signal_type {
            ToxicSignalType::LongBiasToxicFlow => ToxicSignalDirection::LongBias,
            ToxicSignalType::TrapRisk => ToxicSignalDirection::TrapRisk,
            _ => ToxicSignalDirection::ShortBias,
        },
        toxicity_score: 82,
        confidence: ToxicConfidence::High,
        primary_reason: "test fused toxic signal".to_string(),
        reason: vec!["analysis only".to_string()],
        supporting_evidence: vec![ToxicSupportingEvidence {
            source: "structural".to_string(),
            signal_id: "struct-1".to_string(),
            signal_type: "failed_breakout".to_string(),
            contribution_score: 80,
            summary: "test evidence".to_string(),
        }],
        invalidation_price: Some(101_000.0),
        suggested_stop_distance_usd: Some(500.0),
        chase_risk: ToxicChaseRisk::Medium,
        no_trade_reasons: Vec::new(),
        linked_active_trade_signal_ids: Vec::new(),
        linked_liquidation_signal_ids: Vec::new(),
        linked_wall_lifecycle_signal_ids: Vec::new(),
        linked_wall_interpretation_signal_ids: Vec::new(),
        linked_structural_signal_ids: Vec::new(),
        read_only: true,
    }
}

fn replay_recent(signal_ids: Vec<&str>) -> ToxicReplayRecentResponse {
    ToxicReplayRecentResponse {
        read_only: true,
        runtime_modified: false,
        analysis_only: true,
        execution_enabled: false,
        mode: "analysis_only".to_string(),
        selected_symbol: "BTC-PERP".to_string(),
        status: "replay_ready".to_string(),
        warnings: Vec::new(),
        no_trade_reasons: Vec::new(),
        signals: signal_ids
            .into_iter()
            .map(|signal_id| ToxicReplaySignalSummary {
                signal_id: signal_id.to_string(),
                signal_kind: "short_bias_toxic_flow".to_string(),
                confidence: 0.82,
                severity: "high".to_string(),
                created_at: 10_000,
                primary_reason: "test".to_string(),
                read_only: true,
            })
            .collect(),
    }
}

fn markout_recent(signals: Vec<ToxicMarkoutSignal>) -> ToxicMarkoutRecentResponse {
    ToxicMarkoutRecentResponse {
        read_only: true,
        runtime_modified: false,
        analysis_only: true,
        execution_enabled: false,
        mode: "analysis_only".to_string(),
        selected_symbol: "BTC-PERP".to_string(),
        status: "markout_ready".to_string(),
        warnings: Vec::new(),
        signals,
    }
}

fn markout_signal(
    signal_id: &str,
    signal_kind: &str,
    outcomes: [ToxicMarkoutOutcome; 4],
) -> ToxicMarkoutSignal {
    let windows = ["+1m", "+5m", "+15m", "+1h"]
        .into_iter()
        .zip(outcomes)
        .map(|(label, outcome)| ToxicMarkoutWindow {
            label: label.to_string(),
            horizon_ms: 60_000,
            outcome,
            markout_bps: None,
            price_at_signal: None,
            price_at_horizon: None,
            note: "test".to_string(),
        })
        .collect::<Vec<_>>();
    ToxicMarkoutSignal {
        signal_id: signal_id.to_string(),
        symbol: "BTC-PERP".to_string(),
        signal_kind: signal_kind.to_string(),
        direction: "SHORT_BIAS".to_string(),
        toxicity_score: 82,
        confidence: "HIGH".to_string(),
        created_at_ms: 10_000,
        overall_outcome: outcomes[0],
        aligned_windows: windows
            .iter()
            .filter(|window| window.outcome == ToxicMarkoutOutcome::Aligned)
            .count(),
        adverse_windows: windows
            .iter()
            .filter(|window| window.outcome == ToxicMarkoutOutcome::Adverse)
            .count(),
        neutral_windows: windows
            .iter()
            .filter(|window| window.outcome == ToxicMarkoutOutcome::Neutral)
            .count(),
        missing_windows: windows
            .iter()
            .filter(|window| window.outcome == ToxicMarkoutOutcome::NotEnoughData)
            .count(),
        windows,
        no_trade_reasons: Vec::new(),
        read_only: true,
    }
}

fn quality_summary(
    by_signal_type: Vec<ToxicQualityScorecardBucket>,
) -> ToxicQualityScorecardSummaryResponse {
    ToxicQualityScorecardSummaryResponse {
        read_only: true,
        runtime_modified: false,
        analysis_only: true,
        execution_enabled: false,
        mode: "analysis_only".to_string(),
        selected_symbol: "BTC-PERP".to_string(),
        status: "quality_ready".to_string(),
        warnings: Vec::new(),
        total_evaluations: by_signal_type
            .iter()
            .map(|bucket| bucket.total_evaluations)
            .sum(),
        aligned_ratio: 0.0,
        adverse_ratio: 0.0,
        neutral_ratio: 0.0,
        not_enough_data_ratio: 0.0,
        by_signal_type,
        by_window: Vec::new(),
        by_symbol: Vec::new(),
        downgrade_candidates: Vec::new(),
        no_trade_candidates: Vec::new(),
    }
}

fn quality_bucket(
    signal_kind: &str,
    total_evaluations: usize,
    aligned_ratio: f64,
    adverse_ratio: f64,
) -> ToxicQualityScorecardBucket {
    ToxicQualityScorecardBucket {
        key: signal_kind.to_string(),
        label: signal_kind.to_string(),
        total_evaluations,
        aligned_count: 0,
        adverse_count: 0,
        neutral_count: 0,
        not_enough_data_count: 0,
        aligned_ratio,
        adverse_ratio,
        neutral_ratio: 0.0,
        not_enough_data_ratio: 0.0,
        downgrade_candidate: false,
        no_trade_candidate: false,
        top_no_trade_reasons: Vec::new(),
        symbols: vec!["BTC-PERP".to_string()],
    }
}

fn recommendation_summary(
    recommendations: Vec<ToxicWeightRecommendationItem>,
) -> ToxicWeightRecommendationSummaryResponse {
    ToxicWeightRecommendationSummaryResponse {
        read_only: true,
        analysis_only: true,
        execution_enabled: false,
        manual_review_required: true,
        runtime_modified: false,
        runtime_weight_modified: false,
        config_modified: false,
        mode: "analysis_only".to_string(),
        selected_symbol: "BTC-PERP".to_string(),
        status: "recommendation_ready".to_string(),
        warnings: Vec::new(),
        total_recommendations: recommendations.len(),
        keep_count: 0,
        slight_upgrade_candidate_count: 0,
        slight_downgrade_candidate_count: 0,
        downgrade_candidate_count: 0,
        no_trade_only_candidate_count: 0,
        disable_candidate_count: 0,
        insufficient_data_count: 0,
        recommendations,
        by_signal_type: Vec::new(),
        by_symbol: Vec::new(),
        review_flags: Vec::new(),
    }
}

fn recommendation_item(
    signal_type: &str,
    recommendation: ToxicWeightRecommendationKind,
) -> ToxicWeightRecommendationItem {
    ToxicWeightRecommendationItem {
        symbol: "BTC-PERP".to_string(),
        signal_type: signal_type.to_string(),
        sample_count: 20,
        aligned_ratio: 0.62,
        adverse_ratio: 0.18,
        neutral_ratio: 0.20,
        best_window: Some("+1m".to_string()),
        worst_window: Some("+1h".to_string()),
        recommendation,
        current_weight_hint: "reference".to_string(),
        suggested_weight_hint: "reference".to_string(),
        confidence: "MEDIUM".to_string(),
        reason_codes: vec!["manual_review_required".to_string()],
        evidence: vec!["read-only scorecard evidence".to_string()],
        manual_review_required: true,
        runtime_weight_modified: false,
        config_modified: false,
    }
}

fn governance_summary(
    decisions: Vec<ToxicGovernanceDecision>,
) -> ToxicGovernanceLedgerSummaryResponse {
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
        mode: "analysis_only".to_string(),
        selected_symbol: "BTC-PERP".to_string(),
        status: "ledger_ready".to_string(),
        governance_status: "manual_review_required".to_string(),
        manual_review_decision_placeholder: "operator_decision_pending".to_string(),
        evidence_lineage: Vec::new(),
        warnings: Vec::new(),
        total_decisions: decisions.len(),
        accept_count: 0,
        reject_count: 0,
        watch_more_count: 0,
        needs_more_samples_count: 0,
        suppress_for_now_count: 0,
        escalate_review_count: 0,
        consensus_status: "manual_review_required".to_string(),
        recent_governance_notes: Vec::new(),
        decisions,
        by_symbol: Vec::new(),
        by_signal_type: Vec::new(),
    }
}

fn decision(signal_type: &str) -> ToxicGovernanceDecision {
    ToxicGovernanceDecision {
        id: "ledger-1".to_string(),
        symbol: "BTC-PERP".to_string(),
        signal_type: signal_type.to_string(),
        recommendation: ToxicWeightRecommendationKind::Keep,
        decision: ToxicGovernanceDecisionKind::WatchMore,
        reviewer: "operator".to_string(),
        reason: "manual review".to_string(),
        notes: "test".to_string(),
        confidence: 0.7,
        evidence_summary: Vec::new(),
        created_at_ms: 12_000,
        read_only: true,
        governance_ledger_only: true,
        runtime_weight_modified: false,
        config_modified: false,
        runtime_modified: false,
        auto_apply_enabled: false,
    }
}
