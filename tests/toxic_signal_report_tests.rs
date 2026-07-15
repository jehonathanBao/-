use btc_toxic_flow_monitor_rs::{
    toxicity::toxic_signal_report::{
        build_toxic_signal_daily_report, build_toxic_signal_report_status,
        build_toxic_signal_rolling_report,
    },
    types::{
        toxic_quality_scorecard::{
            ToxicQualityScorecardCandidate, ToxicQualityScorecardSummaryResponse,
        },
        toxic_signal_group::{
            ToxicSignalGroup, ToxicSignalGroupOperatorAction, ToxicSignalGroupRecentResponse,
        },
        toxic_signal_history::{ToxicSignalHistoryAlertItem, ToxicSignalHistorySignalItem},
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
fn toxic_signal_daily_report_builds_summary_without_mutating_sources() {
    let inbox = inbox_recent(vec![
        inbox_item(
            "signal-1",
            "BTCUSDT",
            "short_bias_toxic_flow",
            "high",
            0.82,
            ("aligned", "neutral", "not_enough_data", "not_enough_data"),
            false,
        ),
        inbox_item(
            "signal-2",
            "BTCUSDT",
            "short_bias_toxic_flow",
            "medium",
            0.62,
            ("adverse", "aligned", "neutral", "not_enough_data"),
            true,
        ),
        inbox_item(
            "signal-3",
            "ETHUSDT",
            "trap_risk",
            "high",
            0.71,
            ("neutral", "neutral", "neutral", "neutral"),
            false,
        ),
    ]);
    let groups = group_recent();
    let quality = quality_summary();
    let recommendation = recommendation_summary();
    let inbox_before = serde_json::to_value(&inbox).expect("serialize inbox before");
    let groups_before = serde_json::to_value(&groups).expect("serialize groups before");

    let report = build_toxic_signal_daily_report(
        "BTCUSDT",
        "2026-05-30",
        &inbox,
        &groups,
        &quality,
        &recommendation,
    );

    assert!(report.read_only);
    assert!(!report.runtime_modified);
    assert!(report.analysis_only);
    assert!(!report.execution_enabled);
    assert_eq!(report.filter.symbol, "BTCUSDT");
    assert!(report.filter.view_only);
    assert!(!report.filter.persistent_watchlist_enabled);
    assert!(!report.filter.runtime_monitor_modified);
    assert_eq!(report.summary.total_signals, 3);
    assert_eq!(report.summary.grouped_signals, 2);
    assert_eq!(report.summary.high_severity_signals, 2);
    assert_eq!(report.summary.no_trade_only_candidates, 1);
    assert_eq!(report.summary.downgrade_candidates, 2);
    assert_eq!(report.summary.not_enough_data_signals, 2);
    assert_eq!(report.markout_summary.aligned, 2);
    assert_eq!(report.markout_summary.adverse, 1);
    assert_eq!(report.markout_summary.neutral, 6);
    assert_eq!(report.markout_summary.not_enough_data, 3);
    assert_eq!(report.by_symbol.len(), 2);
    assert_eq!(report.by_signal_kind.len(), 2);
    assert_eq!(report.top_groups.len(), 1);
    assert!(report.markdown.contains("# Toxic Signal Daily Report"));
    assert!(report.markdown.contains("No-trade-only candidates: 1"));
    assert!(report.markdown.contains("Downgrade candidates: 2"));
    assert!(report.markdown.contains("Not enough data signals: 2"));
    assert!(report
        .markdown
        .contains("Signal-only report. No trading action is available."));
    assert!(report.markdown.contains("No order placement"));
    assert!(report.markdown.contains("No wallet/signing"));
    assert!(report.markdown.contains("No live trading"));

    assert_eq!(
        serde_json::to_value(&inbox).expect("serialize inbox after"),
        inbox_before
    );
    assert_eq!(
        serde_json::to_value(&groups).expect("serialize groups after"),
        groups_before
    );
}

#[test]
fn toxic_signal_daily_report_handles_empty_inputs_and_not_enough_data_honestly() {
    let inbox = inbox_recent(Vec::new());
    let groups = ToxicSignalGroupRecentResponse {
        read_only: true,
        runtime_modified: false,
        analysis_only: true,
        execution_enabled: false,
        manual_review_required: true,
        runtime_weight_modified: false,
        config_modified: false,
        mode: "analysis_only".to_string(),
        selected_symbol: "BTCUSDT".to_string(),
        status: "empty_signal_groups".to_string(),
        cooldown_window_ms: 300_000,
        warnings: Vec::new(),
        groups: Vec::new(),
    };
    let quality = ToxicQualityScorecardSummaryResponse {
        read_only: true,
        runtime_modified: false,
        analysis_only: true,
        execution_enabled: false,
        mode: "analysis_only".to_string(),
        selected_symbol: "BTCUSDT".to_string(),
        status: "empty_quality_scorecard".to_string(),
        warnings: Vec::new(),
        total_evaluations: 0,
        aligned_ratio: 0.0,
        adverse_ratio: 0.0,
        neutral_ratio: 0.0,
        not_enough_data_ratio: 0.0,
        by_signal_type: Vec::new(),
        by_window: Vec::new(),
        by_symbol: Vec::new(),
        downgrade_candidates: Vec::new(),
        no_trade_candidates: Vec::new(),
    };
    let recommendation = ToxicWeightRecommendationSummaryResponse {
        read_only: true,
        analysis_only: true,
        execution_enabled: false,
        manual_review_required: true,
        runtime_modified: false,
        runtime_weight_modified: false,
        config_modified: false,
        mode: "analysis_only".to_string(),
        selected_symbol: "BTCUSDT".to_string(),
        status: "empty_recommendations".to_string(),
        warnings: Vec::new(),
        total_recommendations: 0,
        keep_count: 0,
        slight_upgrade_candidate_count: 0,
        slight_downgrade_candidate_count: 0,
        downgrade_candidate_count: 0,
        no_trade_only_candidate_count: 0,
        disable_candidate_count: 0,
        insufficient_data_count: 0,
        recommendations: Vec::new(),
        by_signal_type: Vec::new(),
        by_symbol: Vec::new(),
        review_flags: Vec::new(),
    };

    let report = build_toxic_signal_daily_report(
        "BTCUSDT",
        "2026-05-30",
        &inbox,
        &groups,
        &quality,
        &recommendation,
    );
    let status = build_toxic_signal_report_status("BTCUSDT", "2026-05-30", &inbox, &groups);

    assert_eq!(report.summary.total_signals, 0);
    assert!(report.by_symbol.is_empty());
    assert!(report.by_signal_kind.is_empty());
    assert!(report.top_groups.is_empty());
    assert_eq!(report.markout_summary.aligned, 0);
    assert_eq!(report.markout_summary.not_enough_data, 0);
    assert_eq!(status.status, "empty_daily_report");
    assert_eq!(status.total_signals, 0);
    assert_eq!(status.group_count, 0);
}

#[test]
fn toxic_signal_rolling_report_uses_in_memory_history_without_mutation() {
    let signal_history = vec![
        history_signal(
            "signal-1",
            "BTCUSDT",
            "short_bias_toxic_flow",
            ("aligned", "neutral", "not_enough_data", "aligned"),
            "keep",
            false,
        ),
        history_signal(
            "signal-2",
            "ETHUSDT",
            "long_bias_toxic_flow",
            ("adverse", "neutral", "neutral", "neutral"),
            "downgrade_candidate",
            false,
        ),
        history_signal(
            "signal-3",
            "BTCUSDT",
            "short_bias_toxic_flow",
            ("aligned", "aligned", "neutral", "neutral"),
            "no_trade_only_candidate",
            true,
        ),
    ];
    let alert_history = vec![
        history_alert("signal-1", "BTCUSDT", "notify_candidate"),
        history_alert("signal-2", "ETHUSDT", "review_candidate"),
    ];
    let signals_before = serde_json::to_value(&signal_history).expect("signals before");
    let alerts_before = serde_json::to_value(&alert_history).expect("alerts before");

    let report = build_toxic_signal_rolling_report("ALL", "7d", &signal_history, &alert_history);

    assert!(report.read_only);
    assert!(!report.runtime_modified);
    assert!(report.analysis_only);
    assert!(!report.execution_enabled);
    assert_eq!(report.report_type, "rolling");
    assert_eq!(report.window, "7d");
    assert_eq!(report.retention_mode, "in_memory_bounded");
    assert!(!report.durable_storage_enabled);
    assert!(!report.database_write_enabled);
    assert_eq!(report.summary.total_signals, 3);
    assert_eq!(report.summary.aligned, 4);
    assert_eq!(report.summary.adverse, 1);
    assert_eq!(report.summary.neutral, 6);
    assert_eq!(report.summary.not_enough_data, 1);
    assert_eq!(report.summary.no_trade_only_candidates, 1);
    assert_eq!(report.summary.downgrade_candidates, 1);
    assert_eq!(report.summary.notify_candidates, 1);
    assert_eq!(report.summary.review_candidates, 1);
    assert_eq!(report.summary.top_symbols[0], "BTCUSDT (2)");
    assert_eq!(
        report.summary.top_signal_kinds[0],
        "short_bias_toxic_flow (2)"
    );
    assert!(report.markdown.contains("# Toxic Signal Rolling Digest"));
    assert!(report.markdown.contains("Window: 7d"));
    assert!(report.markdown.contains("Notify candidates: 1"));
    assert!(report.markdown.contains("Review candidates: 1"));
    assert!(report.markdown.contains("No notification sending"));
    assert!(report.markdown.contains("No order placement"));
    assert!(report.markdown.contains("No live trading"));

    assert_eq!(
        serde_json::to_value(&signal_history).expect("signals after"),
        signals_before
    );
    assert_eq!(
        serde_json::to_value(&alert_history).expect("alerts after"),
        alerts_before
    );
}

#[test]
fn toxic_signal_rolling_report_handles_empty_history_cleanly() {
    let report = build_toxic_signal_rolling_report("BTCUSDT", "7d", &[], &[]);

    assert_eq!(report.summary.total_signals, 0);
    assert_eq!(report.summary.notify_candidates, 0);
    assert!(report.summary.top_symbols.is_empty());
    assert!(report.summary.top_signal_kinds.is_empty());
    assert!(report.markdown.contains("- None"));
}

fn inbox_recent(items: Vec<ToxicSignalInboxItem>) -> ToxicSignalInboxRecentResponse {
    ToxicSignalInboxRecentResponse {
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
        warnings: Vec::new(),
        items,
    }
}

fn group_recent() -> ToxicSignalGroupRecentResponse {
    ToxicSignalGroupRecentResponse {
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
        warnings: Vec::new(),
        groups: vec![ToxicSignalGroup {
            group_id: "group-btc-1".to_string(),
            symbol: "BTCUSDT".to_string(),
            signal_kind: "short_bias_toxic_flow".to_string(),
            direction_bias: "short".to_string(),
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
    }
}

fn quality_summary() -> ToxicQualityScorecardSummaryResponse {
    ToxicQualityScorecardSummaryResponse {
        read_only: true,
        runtime_modified: false,
        analysis_only: true,
        execution_enabled: false,
        mode: "analysis_only".to_string(),
        selected_symbol: "BTCUSDT".to_string(),
        status: "quality_ready".to_string(),
        warnings: Vec::new(),
        total_evaluations: 9,
        aligned_ratio: 0.55,
        adverse_ratio: 0.22,
        neutral_ratio: 0.11,
        not_enough_data_ratio: 0.12,
        by_signal_type: Vec::new(),
        by_window: Vec::new(),
        by_symbol: Vec::new(),
        downgrade_candidates: vec![ToxicQualityScorecardCandidate {
            key: "short_bias_toxic_flow".to_string(),
            label: "short_bias_toxic_flow".to_string(),
            reason: "adverse pressure".to_string(),
            total_evaluations: 4,
            aligned_ratio: 0.25,
            adverse_ratio: 0.50,
            neutral_ratio: 0.10,
            not_enough_data_ratio: 0.15,
            top_no_trade_reasons: vec!["review_more".to_string()],
        }],
        no_trade_candidates: Vec::new(),
    }
}

fn recommendation_summary() -> ToxicWeightRecommendationSummaryResponse {
    ToxicWeightRecommendationSummaryResponse {
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
        total_recommendations: 2,
        keep_count: 1,
        slight_upgrade_candidate_count: 0,
        slight_downgrade_candidate_count: 0,
        downgrade_candidate_count: 1,
        no_trade_only_candidate_count: 1,
        disable_candidate_count: 0,
        insufficient_data_count: 0,
        recommendations: vec![ToxicWeightRecommendationItem {
            symbol: "BTCUSDT".to_string(),
            signal_type: "short_bias_toxic_flow".to_string(),
            sample_count: 4,
            aligned_ratio: 0.25,
            adverse_ratio: 0.50,
            neutral_ratio: 0.10,
            best_window: Some("+1m".to_string()),
            worst_window: Some("+1h".to_string()),
            recommendation: ToxicWeightRecommendationKind::DowngradeCandidate,
            current_weight_hint: "1.0".to_string(),
            suggested_weight_hint: "0.8".to_string(),
            confidence: "MEDIUM".to_string(),
            reason_codes: vec!["downgrade".to_string()],
            evidence: vec!["quality_scorecard".to_string()],
            manual_review_required: true,
            runtime_weight_modified: false,
            config_modified: false,
        }],
        by_signal_type: Vec::new(),
        by_symbol: Vec::new(),
        review_flags: Vec::new(),
    }
}

fn inbox_item(
    signal_id: &str,
    symbol: &str,
    signal_kind: &str,
    severity: &str,
    confidence: f64,
    markout: (&str, &str, &str, &str),
    no_trade_only: bool,
) -> ToxicSignalInboxItem {
    ToxicSignalInboxItem {
        signal_id: signal_id.to_string(),
        symbol: symbol.to_string(),
        signal_kind: signal_kind.to_string(),
        direction_bias: if signal_kind == "trap_risk" {
            "neutral".to_string()
        } else {
            "short".to_string()
        },
        severity: severity.to_string(),
        risk_score: 82,
        data_quality_score: Some(82.0),
        confidence,
        created_at_ms: 1_000,
        fusion: ToxicSignalInboxFusionSummary {
            available: true,
            summary: "summary".to_string(),
        },
        replay: ToxicSignalInboxReplaySummary {
            available: true,
            evidence_count: 2,
        },
        markout: ToxicSignalInboxMarkoutSummary {
            available: true,
            one_minute: markout.0.to_string(),
            five_minute: markout.1.to_string(),
            fifteen_minute: markout.2.to_string(),
            one_hour: markout.3.to_string(),
        },
        quality: ToxicSignalInboxQualitySummary {
            available: true,
            quality_bucket: "mixed".to_string(),
            aligned_ratio: 0.55,
            adverse_ratio: 0.22,
        },
        recommendation: ToxicSignalInboxRecommendationSummary {
            available: true,
            action: if no_trade_only {
                "no_trade_only_candidate".to_string()
            } else {
                "keep".to_string()
            },
            no_trade_only,
            manual_review_required: true,
        },
        governance: ToxicSignalInboxGovernanceSummary {
            ledger_available: false,
            latest_decision: "needs_more_data".to_string(),
        },
        operator_action: if no_trade_only {
            ToxicSignalInboxOperatorAction::NoTradeWarning
        } else {
            ToxicSignalInboxOperatorAction::ReviewEvidence
        },
        read_only: true,
        runtime_modified: false,
        analysis_only: true,
        execution_enabled: false,
    }
}

fn history_signal(
    signal_id: &str,
    symbol: &str,
    signal_kind: &str,
    markout: (&str, &str, &str, &str),
    recommendation_action: &str,
    no_trade_only: bool,
) -> ToxicSignalHistorySignalItem {
    ToxicSignalHistorySignalItem {
        signal_id: signal_id.to_string(),
        symbol: symbol.to_string(),
        signal_kind: signal_kind.to_string(),
        direction_bias: "short".to_string(),
        severity: "high".to_string(),
        confidence: 0.82,
        created_at_ms: 1_000,
        markout_one_minute: markout.0.to_string(),
        markout_five_minute: markout.1.to_string(),
        markout_fifteen_minute: markout.2.to_string(),
        markout_one_hour: markout.3.to_string(),
        quality_bucket: "good".to_string(),
        recommendation_action: recommendation_action.to_string(),
        no_trade_only,
        source: "signal_history".to_string(),
        history_recorded_at_ms: 2_000,
        operator_action: "watch_signal_only".to_string(),
    }
}

fn history_alert(
    signal_id: &str,
    symbol: &str,
    preview_status: &str,
) -> ToxicSignalHistoryAlertItem {
    ToxicSignalHistoryAlertItem {
        signal_id: signal_id.to_string(),
        symbol: symbol.to_string(),
        signal_kind: "short_bias_toxic_flow".to_string(),
        preview_status: preview_status.to_string(),
        would_notify_if_enabled: preview_status == "notify_candidate",
        no_trade_only: false,
        markout_readiness: "aligned_present".to_string(),
        source: "signal_alert_preview".to_string(),
        history_recorded_at_ms: 2_000,
        notification_sent: false,
        execution_triggered: false,
    }
}
