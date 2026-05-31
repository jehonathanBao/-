use btc_toxic_flow_monitor_rs::{
    toxicity::toxic_signal_detail::{
        build_toxic_signal_detail, build_toxic_signal_detail_status,
        build_toxic_signal_group_detail, ToxicSignalDetailContext,
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
            ToxicSignalType,
        },
        toxic_signal_group::{
            ToxicSignalGroup, ToxicSignalGroupOperatorAction, ToxicSignalGroupRecentResponse,
        },
        toxic_signal_inbox::{
            ToxicSignalInboxFusionSummary, ToxicSignalInboxGovernanceSummary, ToxicSignalInboxItem,
            ToxicSignalInboxMarkoutSummary, ToxicSignalInboxOperatorAction,
            ToxicSignalInboxQualitySummary, ToxicSignalInboxRecentResponse,
            ToxicSignalInboxRecommendationSummary, ToxicSignalInboxReplaySummary,
        },
        toxic_weight_recommendation::{
            ToxicWeightRecommendationItem, ToxicWeightRecommendationKind,
            ToxicWeightRecommendationSummaryResponse,
        },
    },
};

#[test]
fn toxic_signal_detail_returns_complete_payload_and_preserves_sources() {
    let fixture = detail_fixture(true);
    let inbox_before = serde_json::to_value(&fixture.inbox_recent).expect("serialize inbox");
    let group_before = serde_json::to_value(&fixture.group_recent).expect("serialize group");

    let detail = build_toxic_signal_detail("BTCUSDT", "signal-1", &fixture.context());

    assert!(detail.available);
    let payload = detail.detail.expect("detail payload");
    assert_eq!(payload.signal_id, "signal-1");
    assert_eq!(payload.source.group_id.as_deref(), Some("group-btc-1"));
    assert!(payload.source.group_available);
    assert_eq!(payload.timeline.len(), 7);
    assert_eq!(payload.timeline[0].stage, "grouping");
    assert!(payload.timeline[0].available);
    assert_eq!(payload.timeline[3].stage, "markout");
    assert!(payload.timeline[3].summary.contains("not_enough_data"));
    assert_eq!(
        payload.operator_narrative.why_no_execution,
        vec![
            "Signal only. No order placement.".to_string(),
            "Execution is disabled.".to_string(),
            "Manual review required.".to_string()
        ]
    );
    assert_eq!(
        payload.no_execution_reason,
        "Signal-only analysis. No trading action is available."
    );
    assert_eq!(
        serde_json::to_value(&fixture.inbox_recent).expect("serialize inbox after"),
        inbox_before
    );
    assert_eq!(
        serde_json::to_value(&fixture.group_recent).expect("serialize group after"),
        group_before
    );
}

#[test]
fn toxic_signal_detail_handles_missing_signal_and_missing_governance() {
    let fixture = detail_fixture(false);

    let missing = build_toxic_signal_detail("BTCUSDT", "missing", &fixture.context());
    assert!(!missing.available);
    assert_eq!(
        missing.reason.as_deref(),
        Some("signal_id_not_found_in_read_only_signal_detail")
    );

    let detail = build_toxic_signal_detail("BTCUSDT", "signal-1", &fixture.context());
    let payload = detail.detail.expect("detail payload");
    let governance_stage = payload
        .timeline
        .iter()
        .find(|stage| stage.stage == "governance")
        .expect("governance stage");
    assert!(!governance_stage.available);
    assert_eq!(
        governance_stage.summary,
        "No governance ledger entry available yet."
    );
    assert!(payload.evidence.governance.is_none());
}

#[test]
fn toxic_signal_group_detail_returns_representative_signal_and_members() {
    let fixture = detail_fixture(true);

    let detail = build_toxic_signal_group_detail("BTCUSDT", "group-btc-1", &fixture.context());
    assert!(detail.available);
    let payload = detail.detail.expect("group detail payload");
    assert_eq!(payload.group.representative_signal_id, "signal-1");
    assert_eq!(payload.members.len(), 2);
    assert_eq!(payload.representative_signal.signal_id, "signal-1");
    assert_eq!(payload.members[0].signal_id, "signal-1");
    assert_eq!(payload.members[1].signal_id, "signal-2");
}

#[test]
fn toxic_signal_group_detail_handles_missing_group() {
    let fixture = detail_fixture(true);

    let detail = build_toxic_signal_group_detail("BTCUSDT", "missing-group", &fixture.context());
    assert!(!detail.available);
    assert_eq!(
        detail.reason.as_deref(),
        Some("group_id_not_found_in_read_only_signal_detail")
    );
}

#[test]
fn toxic_signal_detail_status_reports_safety_contract() {
    let fixture = detail_fixture(true);
    let status = build_toxic_signal_detail_status("BTCUSDT", &fixture.context());

    assert!(status.read_only);
    assert!(!status.runtime_modified);
    assert!(status.analysis_only);
    assert!(!status.execution_enabled);
    assert_eq!(status.signal_count, 2);
    assert_eq!(status.group_count, 1);
}

struct DetailFixture {
    fusion_recent: ToxicSignalRecentResponse,
    replay_recent: ToxicReplayRecentResponse,
    markout_recent: ToxicMarkoutRecentResponse,
    quality_summary: ToxicQualityScorecardSummaryResponse,
    recommendation_summary: ToxicWeightRecommendationSummaryResponse,
    governance_summary: ToxicGovernanceLedgerSummaryResponse,
    inbox_recent: ToxicSignalInboxRecentResponse,
    group_recent: ToxicSignalGroupRecentResponse,
}

impl DetailFixture {
    fn context(&self) -> ToxicSignalDetailContext<'_> {
        ToxicSignalDetailContext {
            fusion_recent: &self.fusion_recent,
            replay_recent: &self.replay_recent,
            markout_recent: &self.markout_recent,
            quality_summary: &self.quality_summary,
            recommendation_summary: &self.recommendation_summary,
            governance_summary: &self.governance_summary,
            inbox_recent: &self.inbox_recent,
            group_recent: &self.group_recent,
        }
    }
}

fn detail_fixture(with_governance: bool) -> DetailFixture {
    let signal = ToxicSignal {
        signal_id: "signal-1".to_string(),
        symbol: "BTCUSDT".to_string(),
        ts_ms: 1_000,
        signal_type: ToxicSignalType::ShortBiasToxicFlow,
        direction: ToxicSignalDirection::ShortBias,
        toxicity_score: 88,
        confidence: ToxicConfidence::High,
        primary_reason: "Buy-side delta failed near resistance.".to_string(),
        reason: vec!["fusion_reason".to_string()],
        supporting_evidence: Vec::new(),
        invalidation_price: Some(80_780.0),
        suggested_stop_distance_usd: Some(260.0),
        chase_risk: ToxicChaseRisk::Medium,
        no_trade_reasons: vec!["manual_review_required".to_string()],
        linked_active_trade_signal_ids: Vec::new(),
        linked_liquidation_signal_ids: Vec::new(),
        linked_wall_lifecycle_signal_ids: Vec::new(),
        linked_wall_interpretation_signal_ids: Vec::new(),
        linked_structural_signal_ids: Vec::new(),
        read_only: true,
    };
    let second_signal = ToxicSignal {
        signal_id: "signal-2".to_string(),
        symbol: "BTCUSDT".to_string(),
        ts_ms: 1_100,
        signal_type: ToxicSignalType::ShortBiasToxicFlow,
        direction: ToxicSignalDirection::ShortBias,
        toxicity_score: 82,
        confidence: ToxicConfidence::Medium,
        primary_reason: "Follow-on grouped signal.".to_string(),
        reason: vec!["secondary_reason".to_string()],
        supporting_evidence: Vec::new(),
        invalidation_price: Some(80_800.0),
        suggested_stop_distance_usd: Some(220.0),
        chase_risk: ToxicChaseRisk::Medium,
        no_trade_reasons: vec!["manual_review_required".to_string()],
        linked_active_trade_signal_ids: Vec::new(),
        linked_liquidation_signal_ids: Vec::new(),
        linked_wall_lifecycle_signal_ids: Vec::new(),
        linked_wall_interpretation_signal_ids: Vec::new(),
        linked_structural_signal_ids: Vec::new(),
        read_only: true,
    };

    let inbox_recent = ToxicSignalInboxRecentResponse {
        read_only: true,
        runtime_modified: false,
        analysis_only: true,
        execution_enabled: false,
        manual_review_required: true,
        runtime_weight_modified: false,
        config_modified: false,
        mode: "analysis_only".to_string(),
        selected_symbol: "BTCUSDT".to_string(),
        status: "signal_inbox_ready".to_string(),
        warnings: vec!["grouped_for_display_only".to_string()],
        items: vec![
            inbox_item(
                "signal-1",
                0.82,
                1_000,
                ToxicSignalInboxOperatorAction::ReviewEvidence,
            ),
            inbox_item(
                "signal-2",
                0.62,
                1_100,
                ToxicSignalInboxOperatorAction::ReviewMarkout,
            ),
        ],
    };

    let group_recent = ToxicSignalGroupRecentResponse {
        read_only: true,
        runtime_modified: false,
        analysis_only: true,
        execution_enabled: false,
        manual_review_required: true,
        runtime_weight_modified: false,
        config_modified: false,
        mode: "analysis_only".to_string(),
        selected_symbol: "BTCUSDT".to_string(),
        status: "signal_groups_ready".to_string(),
        cooldown_window_ms: 300_000,
        warnings: vec!["grouped_for_display_only".to_string()],
        groups: vec![ToxicSignalGroup {
            group_id: "group-btc-1".to_string(),
            symbol: "BTCUSDT".to_string(),
            signal_kind: "short_bias_toxic_flow".to_string(),
            direction_bias: "short_bias".to_string(),
            count: 2,
            first_seen_at_ms: 1_000,
            last_seen_at_ms: 1_100,
            cooldown_window_ms: 300_000,
            max_severity: "high".to_string(),
            avg_confidence: 0.72,
            representative_signal_id: "signal-1".to_string(),
            member_signal_ids: vec!["signal-1".to_string(), "signal-2".to_string()],
            operator_action: ToxicSignalGroupOperatorAction::ReviewGroupedSignal,
            suppression_hint: "Grouped for display only. Original signals are preserved."
                .to_string(),
            original_signals_preserved: true,
            representative_confidence: 0.82,
            read_only: true,
            runtime_modified: false,
            analysis_only: true,
            execution_enabled: false,
        }],
    };

    DetailFixture {
        fusion_recent: ToxicSignalRecentResponse {
            read_only: true,
            runtime_modified: false,
            analysis_only: true,
            execution_enabled: false,
            mode: "analysis_only".to_string(),
            selected_symbol: "BTCUSDT".to_string(),
            status: "fusion_ready".to_string(),
            warnings: Vec::new(),
            no_trade_reasons: Vec::new(),
            signals: vec![signal, second_signal],
        },
        replay_recent: ToxicReplayRecentResponse {
            read_only: true,
            runtime_modified: false,
            analysis_only: true,
            execution_enabled: false,
            mode: "analysis_only".to_string(),
            selected_symbol: "BTCUSDT".to_string(),
            status: "replay_ready".to_string(),
            warnings: Vec::new(),
            no_trade_reasons: Vec::new(),
            signals: vec![ToxicReplaySignalSummary {
                signal_id: "signal-1".to_string(),
                signal_kind: "short_bias_toxic_flow".to_string(),
                confidence: 0.82,
                severity: "high".to_string(),
                created_at: 1_000,
                primary_reason: "Replay evidence available.".to_string(),
                read_only: true,
            }],
        },
        markout_recent: ToxicMarkoutRecentResponse {
            read_only: true,
            runtime_modified: false,
            analysis_only: true,
            execution_enabled: false,
            mode: "analysis_only".to_string(),
            selected_symbol: "BTCUSDT".to_string(),
            status: "markout_ready".to_string(),
            warnings: Vec::new(),
            signals: vec![ToxicMarkoutSignal {
                signal_id: "signal-1".to_string(),
                symbol: "BTCUSDT".to_string(),
                signal_kind: "short_bias_toxic_flow".to_string(),
                direction: "SHORT_BIAS".to_string(),
                toxicity_score: 88,
                confidence: "HIGH".to_string(),
                created_at_ms: 1_000,
                overall_outcome: ToxicMarkoutOutcome::Neutral,
                aligned_windows: 1,
                adverse_windows: 0,
                neutral_windows: 1,
                missing_windows: 2,
                windows: vec![
                    window("+1m", ToxicMarkoutOutcome::Aligned),
                    window("+5m", ToxicMarkoutOutcome::Neutral),
                    window("+15m", ToxicMarkoutOutcome::NotEnoughData),
                    window("+1h", ToxicMarkoutOutcome::NotEnoughData),
                ],
                no_trade_reasons: vec!["manual_review_required".to_string()],
                read_only: true,
            }],
        },
        quality_summary: ToxicQualityScorecardSummaryResponse {
            read_only: true,
            runtime_modified: false,
            analysis_only: true,
            execution_enabled: false,
            mode: "analysis_only".to_string(),
            selected_symbol: "BTCUSDT".to_string(),
            status: "quality_ready".to_string(),
            warnings: Vec::new(),
            total_evaluations: 12,
            aligned_ratio: 0.62,
            adverse_ratio: 0.18,
            neutral_ratio: 0.10,
            not_enough_data_ratio: 0.10,
            by_signal_type: vec![ToxicQualityScorecardBucket {
                key: "short_bias_toxic_flow".to_string(),
                label: "good".to_string(),
                total_evaluations: 12,
                aligned_count: 7,
                adverse_count: 2,
                neutral_count: 1,
                not_enough_data_count: 2,
                aligned_ratio: 0.62,
                adverse_ratio: 0.18,
                neutral_ratio: 0.10,
                not_enough_data_ratio: 0.10,
                downgrade_candidate: false,
                no_trade_candidate: false,
                top_no_trade_reasons: vec!["manual_review_required".to_string()],
                symbols: vec!["BTCUSDT".to_string()],
            }],
            by_window: Vec::new(),
            by_symbol: Vec::new(),
            downgrade_candidates: Vec::new(),
            no_trade_candidates: Vec::new(),
        },
        recommendation_summary: ToxicWeightRecommendationSummaryResponse {
            read_only: true,
            analysis_only: true,
            execution_enabled: false,
            manual_review_required: true,
            runtime_modified: false,
            runtime_weight_modified: false,
            config_modified: false,
            mode: "analysis_only".to_string(),
            selected_symbol: "BTCUSDT".to_string(),
            status: "recommendation_ready".to_string(),
            warnings: Vec::new(),
            total_recommendations: 1,
            keep_count: 1,
            slight_upgrade_candidate_count: 0,
            slight_downgrade_candidate_count: 0,
            downgrade_candidate_count: 0,
            no_trade_only_candidate_count: 0,
            disable_candidate_count: 0,
            insufficient_data_count: 0,
            recommendations: vec![ToxicWeightRecommendationItem {
                symbol: "BTCUSDT".to_string(),
                signal_type: "short_bias_toxic_flow".to_string(),
                sample_count: 12,
                aligned_ratio: 0.62,
                adverse_ratio: 0.18,
                neutral_ratio: 0.10,
                best_window: Some("+1m".to_string()),
                worst_window: Some("+15m".to_string()),
                recommendation: ToxicWeightRecommendationKind::Keep,
                current_weight_hint: "1.0".to_string(),
                suggested_weight_hint: "1.0".to_string(),
                confidence: "HIGH".to_string(),
                reason_codes: vec!["keep".to_string()],
                evidence: vec!["quality_good".to_string()],
                manual_review_required: true,
                runtime_weight_modified: false,
                config_modified: false,
            }],
            by_signal_type: Vec::new(),
            by_symbol: Vec::new(),
            review_flags: Vec::new(),
        },
        governance_summary: ToxicGovernanceLedgerSummaryResponse {
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
            selected_symbol: "BTCUSDT".to_string(),
            status: "governance_ready".to_string(),
            governance_status: "manual_review_pending".to_string(),
            manual_review_decision_placeholder: "manual_review_required".to_string(),
            evidence_lineage: vec!["t9".to_string(), "t10".to_string()],
            warnings: Vec::new(),
            total_decisions: usize::from(with_governance),
            accept_count: 0,
            reject_count: 0,
            watch_more_count: usize::from(with_governance),
            needs_more_samples_count: 0,
            suppress_for_now_count: 0,
            escalate_review_count: 0,
            consensus_status: "needs_more_data".to_string(),
            recent_governance_notes: Vec::new(),
            decisions: if with_governance {
                vec![ToxicGovernanceDecision {
                    id: "gov-1".to_string(),
                    symbol: "BTCUSDT".to_string(),
                    signal_type: "short_bias_toxic_flow".to_string(),
                    recommendation: ToxicWeightRecommendationKind::Keep,
                    decision: ToxicGovernanceDecisionKind::WatchMore,
                    reviewer: "ops".to_string(),
                    reason: "Need more samples".to_string(),
                    notes: "Hold for manual review".to_string(),
                    confidence: 0.60,
                    evidence_summary: vec!["markout_not_enough_data".to_string()],
                    created_at_ms: 1_200,
                    read_only: true,
                    governance_ledger_only: true,
                    runtime_weight_modified: false,
                    config_modified: false,
                    runtime_modified: false,
                    auto_apply_enabled: false,
                }]
            } else {
                Vec::new()
            },
            by_symbol: Vec::new(),
            by_signal_type: Vec::new(),
        },
        inbox_recent,
        group_recent,
    }
}

fn inbox_item(
    signal_id: &str,
    confidence: f64,
    created_at_ms: u64,
    operator_action: ToxicSignalInboxOperatorAction,
) -> ToxicSignalInboxItem {
    ToxicSignalInboxItem {
        signal_id: signal_id.to_string(),
        symbol: "BTCUSDT".to_string(),
        signal_kind: "short_bias_toxic_flow".to_string(),
        direction_bias: "short_bias".to_string(),
        severity: if confidence > 0.7 {
            "high".to_string()
        } else {
            "medium".to_string()
        },
        confidence,
        created_at_ms,
        fusion: ToxicSignalInboxFusionSummary {
            available: true,
            summary: "Short-biased toxic flow detected.".to_string(),
        },
        replay: ToxicSignalInboxReplaySummary {
            available: true,
            evidence_count: 5,
        },
        markout: ToxicSignalInboxMarkoutSummary {
            available: true,
            one_minute: "aligned".to_string(),
            five_minute: "neutral".to_string(),
            fifteen_minute: "not_enough_data".to_string(),
            one_hour: "not_enough_data".to_string(),
        },
        quality: ToxicSignalInboxQualitySummary {
            available: true,
            quality_bucket: "good".to_string(),
            aligned_ratio: 0.62,
            adverse_ratio: 0.18,
        },
        recommendation: ToxicSignalInboxRecommendationSummary {
            available: true,
            action: "keep".to_string(),
            no_trade_only: false,
            manual_review_required: true,
        },
        governance: ToxicSignalInboxGovernanceSummary {
            ledger_available: true,
            latest_decision: "watch_more".to_string(),
        },
        operator_action,
        read_only: true,
        runtime_modified: false,
        analysis_only: true,
        execution_enabled: false,
    }
}

fn window(label: &str, outcome: ToxicMarkoutOutcome) -> ToxicMarkoutWindow {
    ToxicMarkoutWindow {
        label: label.to_string(),
        horizon_ms: 60_000,
        outcome,
        markout_bps: None,
        price_at_signal: None,
        price_at_horizon: None,
        note: "test".to_string(),
    }
}
