use btc_toxic_flow_monitor_rs::{
    toxicity::toxic_signal_health::{
        build_toxic_signal_health_status, build_toxic_signal_health_summary,
    },
    types::{
        toxic_signal_alert_preview::{
            ToxicSignalAlertPreviewFilter, ToxicSignalAlertPreviewGate,
            ToxicSignalAlertPreviewItem, ToxicSignalAlertPreviewResponse,
            ToxicSignalAlertPreviewSummary,
        },
        toxic_signal_group::ToxicSignalGroupRecentResponse,
        toxic_signal_health::ToxicSignalHealthSummaryResponse,
        toxic_signal_history::{
            ToxicSignalHistoryRecentResponse, ToxicSignalHistoryStatusResponse,
        },
        toxic_signal_inbox::{
            ToxicSignalInboxFusionSummary, ToxicSignalInboxGovernanceSummary, ToxicSignalInboxItem,
            ToxicSignalInboxMarkoutSummary, ToxicSignalInboxOperatorAction,
            ToxicSignalInboxQualitySummary, ToxicSignalInboxRecentResponse,
            ToxicSignalInboxRecommendationSummary, ToxicSignalInboxReplaySummary,
        },
        toxic_signal_report::{
            ToxicSignalReportDailyResponse, ToxicSignalReportFilter,
            ToxicSignalReportMarkoutSummary, ToxicSignalReportSummary,
        },
    },
};

#[test]
fn toxic_signal_health_builds_diagnostic_summary_without_mutating_sources() {
    let inbox = inbox_recent(vec![
        inbox_item(
            "signal-1",
            true,
            true,
            true,
            false,
            ("aligned", "neutral", "aligned", "neutral"),
        ),
        inbox_item(
            "signal-2",
            false,
            false,
            false,
            false,
            ("not_enough_data", "not_enough_data", "neutral", "neutral"),
        ),
        inbox_item(
            "signal-3",
            true,
            true,
            true,
            false,
            ("aligned", "aligned", "neutral", "neutral"),
        ),
    ]);
    let groups = group_recent(1);
    let report = daily_report(3);
    let preview = alert_preview(3);
    let history_status = history_status(3);
    let history_recent = history_recent(3);
    let inbox_before = serde_json::to_value(&inbox).expect("serialize inbox");
    let groups_before = serde_json::to_value(&groups).expect("serialize groups");

    let summary = build_toxic_signal_health_summary(
        "BTCUSDT",
        &inbox,
        &groups,
        &report,
        &preview,
        &history_status,
        &history_recent,
    );
    let status = build_toxic_signal_health_status(&summary);

    assert!(summary.read_only);
    assert!(!summary.runtime_modified);
    assert!(summary.analysis_only);
    assert!(!summary.execution_enabled);
    assert_eq!(summary.health_mode, "diagnostic_only");
    assert!(!summary.repair_enabled);
    assert!(!summary.backfill_enabled);
    assert!(!summary.runtime_mutation_enabled);
    assert_eq!(summary.summary.total_signals, 3);
    assert_eq!(summary.summary.signals_with_markout, 2);
    assert_eq!(summary.summary.signals_missing_markout, 1);
    assert_eq!(summary.summary.signals_with_quality, 2);
    assert_eq!(summary.summary.signals_missing_quality, 1);
    assert_eq!(summary.summary.signals_with_recommendation, 2);
    assert_eq!(summary.summary.signals_missing_recommendation, 1);
    assert_eq!(summary.summary.signals_with_governance, 0);
    assert_eq!(summary.summary.signals_missing_governance, 3);
    assert_eq!(summary.summary.not_enough_data_count, 1);
    assert_eq!(summary.health_bucket, "good");
    assert!(summary
        .issues
        .iter()
        .any(|issue| issue.kind == "missing_markout" && issue.count == 1));
    assert!(summary
        .issues
        .iter()
        .any(|issue| issue.kind == "missing_quality" && issue.count == 1));
    assert!(summary
        .issues
        .iter()
        .any(|issue| issue.kind == "missing_recommendation" && issue.count == 1));
    assert!(summary
        .issues
        .iter()
        .any(|issue| issue.kind == "missing_governance" && issue.count == 3));
    assert!(summary
        .operator_notes
        .iter()
        .any(|note| note.contains("No repair, backfill")));

    assert_eq!(status.health_mode, "diagnostic_only");
    assert!(!status.repair_enabled);
    assert!(!status.backfill_enabled);
    assert!(!status.runtime_mutation_enabled);
    assert_eq!(status.status, "signal_health_ready");
    assert!(status
        .safety_boundary
        .iter()
        .any(|item| item == "No order placement"));

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
fn toxic_signal_health_empty_inbox_returns_unavailable_without_panicking() {
    let summary = build_summary_for(
        "BTCUSDT",
        inbox_recent(Vec::new()),
        group_recent(0),
        daily_report(0),
        alert_preview(0),
        history_status(0),
        history_recent(0),
    );

    assert_eq!(summary.health_bucket, "unavailable");
    assert_eq!(summary.summary.total_signals, 0);
    assert_eq!(summary.issues.len(), 1);
    assert_eq!(summary.issues[0].kind, "symbol_not_found");
}

#[test]
fn toxic_signal_health_missing_governance_is_an_issue_not_a_failure() {
    let summary = build_summary_for(
        "ALL",
        inbox_recent(vec![inbox_item(
            "signal-1",
            true,
            true,
            true,
            false,
            ("aligned", "aligned", "aligned", "aligned"),
        )]),
        group_recent(1),
        daily_report(1),
        alert_preview(1),
        history_status(1),
        history_recent(1),
    );

    assert_eq!(summary.health_bucket, "excellent");
    let issue = summary
        .issues
        .iter()
        .find(|issue| issue.kind == "missing_governance")
        .expect("missing governance issue");
    assert_eq!(issue.severity, "info");
    assert_eq!(issue.count, 1);
}

#[test]
fn toxic_signal_health_flags_high_not_enough_data_ratio_honestly() {
    let summary = build_summary_for(
        "ALL",
        inbox_recent(vec![
            inbox_item(
                "signal-1",
                true,
                false,
                false,
                false,
                ("not_enough_data", "neutral", "neutral", "neutral"),
            ),
            inbox_item(
                "signal-2",
                true,
                false,
                false,
                false,
                ("not_enough_data", "neutral", "neutral", "neutral"),
            ),
        ]),
        group_recent(1),
        daily_report(2),
        alert_preview(2),
        history_status(2),
        history_recent(2),
    );

    assert_eq!(summary.health_bucket, "thin_data");
    assert_eq!(summary.summary.not_enough_data_count, 2);
    assert!(summary
        .issues
        .iter()
        .any(|issue| issue.kind == "high_not_enough_data_ratio" && issue.count == 2));
}

fn build_summary_for(
    requested_symbol: &str,
    inbox: ToxicSignalInboxRecentResponse,
    groups: ToxicSignalGroupRecentResponse,
    report: ToxicSignalReportDailyResponse,
    preview: ToxicSignalAlertPreviewResponse,
    history_status: ToxicSignalHistoryStatusResponse,
    history_recent: ToxicSignalHistoryRecentResponse,
) -> ToxicSignalHealthSummaryResponse {
    build_toxic_signal_health_summary(
        requested_symbol,
        &inbox,
        &groups,
        &report,
        &preview,
        &history_status,
        &history_recent,
    )
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
        status: if items.is_empty() {
            "empty_signal_inbox".to_string()
        } else {
            "signal_inbox_ready".to_string()
        },
        warnings: Vec::new(),
        items,
    }
}

fn inbox_item(
    signal_id: &str,
    markout_available: bool,
    quality_available: bool,
    recommendation_available: bool,
    governance_available: bool,
    markout_windows: (&str, &str, &str, &str),
) -> ToxicSignalInboxItem {
    ToxicSignalInboxItem {
        signal_id: signal_id.to_string(),
        symbol: "BTCUSDT".to_string(),
        signal_kind: "short_bias_toxic_flow".to_string(),
        direction_bias: "short_bias".to_string(),
        severity: "high".to_string(),
        confidence: 0.82,
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
            available: markout_available,
            one_minute: markout_windows.0.to_string(),
            five_minute: markout_windows.1.to_string(),
            fifteen_minute: markout_windows.2.to_string(),
            one_hour: markout_windows.3.to_string(),
        },
        quality: ToxicSignalInboxQualitySummary {
            available: quality_available,
            quality_bucket: if quality_available {
                "good".to_string()
            } else {
                "not_enough_data".to_string()
            },
            aligned_ratio: 0.65,
            adverse_ratio: 0.15,
        },
        recommendation: ToxicSignalInboxRecommendationSummary {
            available: recommendation_available,
            action: if recommendation_available {
                "keep".to_string()
            } else {
                "insufficient_data".to_string()
            },
            no_trade_only: false,
            manual_review_required: true,
        },
        governance: ToxicSignalInboxGovernanceSummary {
            ledger_available: governance_available,
            latest_decision: if governance_available {
                "watch_more".to_string()
            } else {
                "missing_ledger_evidence".to_string()
            },
        },
        operator_action: ToxicSignalInboxOperatorAction::ReviewEvidence,
        read_only: true,
        runtime_modified: false,
        analysis_only: true,
        execution_enabled: false,
    }
}

fn group_recent(count: usize) -> ToxicSignalGroupRecentResponse {
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
        status: if count == 0 {
            "empty_signal_groups".to_string()
        } else {
            "signal_groups_ready".to_string()
        },
        cooldown_window_ms: 300_000,
        warnings: Vec::new(),
        groups: (0..count)
            .map(|index| btc_toxic_flow_monitor_rs::types::toxic_signal_group::ToxicSignalGroup {
                group_id: format!("group-{index}"),
                symbol: "BTCUSDT".to_string(),
                signal_kind: "short_bias_toxic_flow".to_string(),
                direction_bias: "short_bias".to_string(),
                count: 1,
                first_seen_at_ms: 1_000,
                last_seen_at_ms: 1_000,
                cooldown_window_ms: 300_000,
                max_severity: "high".to_string(),
                avg_confidence: 0.82,
                representative_signal_id: format!("signal-{index}"),
                member_signal_ids: vec![format!("signal-{index}")],
                operator_action:
                    btc_toxic_flow_monitor_rs::types::toxic_signal_group::ToxicSignalGroupOperatorAction::WatchGroupOnly,
                suppression_hint: "Grouped for display only. Original signals are preserved."
                    .to_string(),
                original_signals_preserved: true,
                representative_confidence: 0.82,
                read_only: true,
                runtime_modified: false,
                analysis_only: true,
                execution_enabled: false,
            })
            .collect(),
    }
}

fn daily_report(total_signals: usize) -> ToxicSignalReportDailyResponse {
    ToxicSignalReportDailyResponse {
        read_only: true,
        runtime_modified: false,
        analysis_only: true,
        execution_enabled: false,
        manual_review_required: true,
        runtime_weight_modified: false,
        config_modified: false,
        report_type: "daily".to_string(),
        mode: "analysis_only".to_string(),
        date: "2026-05-30".to_string(),
        filter: ToxicSignalReportFilter {
            symbol: "BTCUSDT".to_string(),
            view_only: true,
            persistent_watchlist_enabled: false,
            runtime_monitor_modified: false,
        },
        summary: ToxicSignalReportSummary {
            total_signals,
            grouped_signals: total_signals,
            high_severity_signals: total_signals,
            no_trade_only_candidates: 0,
            downgrade_candidates: 0,
            not_enough_data_signals: 0,
        },
        markout_summary: ToxicSignalReportMarkoutSummary {
            aligned: total_signals,
            adverse: 0,
            neutral: 0,
            not_enough_data: 0,
        },
        by_symbol: Vec::new(),
        by_signal_kind: Vec::new(),
        top_groups: Vec::new(),
        operator_notes: vec!["Signal-only report. No trading action is available.".to_string()],
        markdown: "# report".to_string(),
    }
}

fn alert_preview(total_signals: usize) -> ToxicSignalAlertPreviewResponse {
    let items = (0..total_signals)
        .map(|index| ToxicSignalAlertPreviewItem {
            signal_id: format!("signal-{index}"),
            symbol: "BTCUSDT".to_string(),
            signal_kind: "short_bias_toxic_flow".to_string(),
            direction_bias: "short_bias".to_string(),
            severity: "high".to_string(),
            confidence: 0.82,
            preview_status: "notify_candidate".to_string(),
            would_notify_if_enabled: true,
            no_trade_only: false,
            quality_bucket: "good".to_string(),
            latest_governance_decision: "watch_more".to_string(),
            markout_readiness: "aligned_present".to_string(),
            suppression_reasons: Vec::new(),
            review_reasons: vec!["reason".to_string()],
            preview_message: "preview".to_string(),
            notification_sent: false,
            execution_triggered: false,
            read_only: true,
            analysis_only: true,
            execution_enabled: false,
        })
        .collect();

    ToxicSignalAlertPreviewResponse {
        read_only: true,
        runtime_modified: false,
        analysis_only: true,
        execution_enabled: false,
        manual_review_required: true,
        notification_sent: false,
        execution_triggered: false,
        preview_only: true,
        runtime_weight_modified: false,
        config_modified: false,
        mode: "notification_preview_only".to_string(),
        status: if total_signals == 0 {
            "empty_notification_preview".to_string()
        } else {
            "notification_preview_ready".to_string()
        },
        selected_symbol: "BTCUSDT".to_string(),
        filter: ToxicSignalAlertPreviewFilter {
            symbol: "BTCUSDT".to_string(),
            view_only: true,
            persistent_watchlist_enabled: false,
            runtime_monitor_modified: false,
        },
        gate: ToxicSignalAlertPreviewGate {
            dedup_window_ms: 30_000,
            min_severity: "alert".to_string(),
            require_cross_venue: true,
            require_markout: true,
            require_liquidity_drain: false,
            telegram_enabled: false,
            notification_sent: false,
            execution_triggered: false,
        },
        summary: ToxicSignalAlertPreviewSummary {
            total_signals,
            notify_candidates: total_signals,
            review_candidates: 0,
            suppressed_signals: 0,
            no_trade_only_signals: 0,
            governance_hold_signals: 0,
            not_enough_data_signals: 0,
        },
        by_symbol: Vec::new(),
        by_signal_kind: Vec::new(),
        items,
        operator_notes: vec!["Signal health only. No runtime action is performed.".to_string()],
        markdown: "# preview".to_string(),
    }
}

fn history_status(current_signals: usize) -> ToxicSignalHistoryStatusResponse {
    ToxicSignalHistoryStatusResponse {
        read_only: true,
        runtime_modified: false,
        analysis_only: true,
        execution_enabled: false,
        retention_mode: "in_memory_bounded".to_string(),
        durable_storage_enabled: false,
        database_write_enabled: false,
        max_signals: 1000,
        max_groups: 300,
        max_alerts: 300,
        max_reports: 30,
        current_signals,
        current_groups: 0,
        current_alerts: 0,
        current_reports: 0,
        safety_boundary: vec!["readOnly=true".to_string()],
    }
}

fn history_recent(item_count: usize) -> ToxicSignalHistoryRecentResponse {
    ToxicSignalHistoryRecentResponse {
        read_only: true,
        runtime_modified: false,
        analysis_only: true,
        execution_enabled: false,
        retention_mode: "in_memory_bounded".to_string(),
        durable_storage_enabled: false,
        database_write_enabled: false,
        selected_symbol: "BTCUSDT".to_string(),
        items: (0..item_count)
            .map(|index| btc_toxic_flow_monitor_rs::types::toxic_signal_history::ToxicSignalHistorySignalItem {
                signal_id: format!("signal-{index}"),
                symbol: "BTCUSDT".to_string(),
                signal_kind: "short_bias_toxic_flow".to_string(),
                direction_bias: "short_bias".to_string(),
                severity: "high".to_string(),
                confidence: 0.82,
                created_at_ms: 1_000,
                source: "signal_inbox".to_string(),
                history_recorded_at_ms: 1_100,
                operator_action: "watch_signal_only".to_string(),
                markout_one_minute: "aligned".to_string(),
                markout_five_minute: "neutral".to_string(),
                markout_fifteen_minute: "adverse".to_string(),
                markout_one_hour: "not_enough_data".to_string(),
                quality_bucket: "good".to_string(),
                recommendation_action: "keep".to_string(),
                no_trade_only: false,
            })
            .collect(),
        group_items: Vec::new(),
        operator_notes: vec!["history".to_string()],
    }
}
