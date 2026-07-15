use btc_toxic_flow_monitor_rs::{
    toxicity::toxic_signal_history_service::ToxicSignalHistoryService,
    types::{
        toxic_signal_alert_preview::{
            ToxicSignalAlertPreviewBucket, ToxicSignalAlertPreviewFilter,
            ToxicSignalAlertPreviewGate, ToxicSignalAlertPreviewItem,
            ToxicSignalAlertPreviewResponse, ToxicSignalAlertPreviewSummary,
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
        toxic_signal_report::{
            ToxicSignalReportBucket, ToxicSignalReportDailyResponse, ToxicSignalReportFilter,
            ToxicSignalReportMarkoutSummary, ToxicSignalReportSummary, ToxicSignalReportTopGroup,
        },
    },
};

#[test]
fn toxic_signal_history_status_exposes_bounded_in_memory_contract() {
    let service = ToxicSignalHistoryService::new(1000, 300, 300, 30);

    let status = service.status();

    assert!(status.read_only);
    assert!(!status.runtime_modified);
    assert!(status.analysis_only);
    assert!(!status.execution_enabled);
    assert_eq!(status.retention_mode, "in_memory_bounded");
    assert!(!status.durable_storage_enabled);
    assert!(!status.database_write_enabled);
    assert_eq!(status.max_signals, 1000);
    assert_eq!(status.max_groups, 300);
    assert_eq!(status.max_alerts, 300);
    assert_eq!(status.max_reports, 30);
    assert_eq!(status.current_signals, 0);
    assert_eq!(status.current_groups, 0);
    assert_eq!(status.current_alerts, 0);
    assert_eq!(status.current_reports, 0);
}

#[test]
fn toxic_signal_history_records_snapshots_without_mutating_sources() {
    let service = ToxicSignalHistoryService::new(10, 10, 10, 10);
    let inbox = inbox_recent(vec![
        inbox_item(
            "signal-btc-1",
            "BTCUSDT",
            "short_bias_toxic_flow",
            "high",
            0.82,
            false,
        ),
        inbox_item("signal-eth-1", "ETHUSDT", "trap_risk", "medium", 0.61, true),
    ]);
    let groups = group_recent();
    let alerts = alert_preview();
    let report = daily_report();
    let inbox_before = serde_json::to_value(&inbox).expect("serialize inbox");
    let groups_before = serde_json::to_value(&groups).expect("serialize groups");
    let alerts_before = serde_json::to_value(&alerts).expect("serialize alerts");
    let report_before = serde_json::to_value(&report).expect("serialize report");

    service.record_snapshot(1_234, &inbox, &groups, &alerts, &report);

    let recent = service.recent("ALL");
    let btc_recent = service.recent("BTCUSDT");
    let lookup = service.signal_by_id("signal-btc-1");
    let alert_recent = service.recent_alerts("ALL");
    let report_recent = service.recent_reports("ALL");

    assert_eq!(recent.items.len(), 2);
    assert_eq!(recent.group_items.len(), 1);
    assert_eq!(btc_recent.items.len(), 1);
    assert_eq!(btc_recent.items[0].signal_id, "signal-btc-1");
    assert!(lookup.found);
    assert_eq!(
        lookup.signal.as_ref().map(|item| item.signal_kind.as_str()),
        Some("short_bias_toxic_flow")
    );
    assert_eq!(alert_recent.items.len(), 2);
    assert_eq!(report_recent.items.len(), 1);
    let btc_item = recent
        .items
        .iter()
        .find(|item| item.signal_id == "signal-btc-1")
        .expect("btc signal retained");
    assert_eq!(btc_item.markout_one_minute, "aligned");
    assert_eq!(btc_item.quality_bucket, "good");
    assert_eq!(btc_item.recommendation_action, "keep");
    assert!(!btc_item.no_trade_only);

    assert_eq!(
        serde_json::to_value(&inbox).expect("serialize inbox after"),
        inbox_before
    );
    assert_eq!(
        serde_json::to_value(&groups).expect("serialize groups after"),
        groups_before
    );
    assert_eq!(
        serde_json::to_value(&alerts).expect("serialize alerts after"),
        alerts_before
    );
    assert_eq!(
        serde_json::to_value(&report).expect("serialize report after"),
        report_before
    );
}

#[test]
fn toxic_signal_history_drops_oldest_signals_when_capacity_is_exceeded() {
    let service = ToxicSignalHistoryService::new(2, 2, 2, 2);

    for (index, signal_id) in ["signal-1", "signal-2", "signal-3"].iter().enumerate() {
        let inbox = inbox_recent(vec![inbox_item(
            signal_id,
            "BTCUSDT",
            "short_bias_toxic_flow",
            "medium",
            0.60 + index as f64 * 0.05,
            false,
        )]);
        service.record_snapshot(
            2_000 + index as u64,
            &inbox,
            &empty_groups(),
            &empty_alert_preview(),
            &report_for_symbol("BTCUSDT"),
        );
    }

    let recent = service.recent("ALL");

    assert_eq!(recent.items.len(), 2);
    assert_eq!(recent.items[0].signal_id, "signal-3");
    assert_eq!(recent.items[1].signal_id, "signal-2");
    assert!(!recent.items.iter().any(|item| item.signal_id == "signal-1"));
}

#[test]
fn toxic_signal_history_handles_not_found_and_empty_auxiliary_history_honestly() {
    let service = ToxicSignalHistoryService::new(5, 5, 5, 5);

    let lookup = service.signal_by_id("missing-signal");
    let recent = service.recent("BTCUSDT");
    let alerts = service.recent_alerts("BTCUSDT");
    let reports = service.recent_reports("BTCUSDT");

    assert!(!lookup.found);
    assert!(lookup.signal.is_none());
    assert!(recent.items.is_empty());
    assert!(recent.group_items.is_empty());
    assert!(alerts.items.is_empty());
    assert!(reports.items.is_empty());
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

fn inbox_item(
    signal_id: &str,
    symbol: &str,
    signal_kind: &str,
    severity: &str,
    confidence: f64,
    no_trade_only: bool,
) -> ToxicSignalInboxItem {
    ToxicSignalInboxItem {
        signal_id: signal_id.to_string(),
        symbol: symbol.to_string(),
        signal_kind: signal_kind.to_string(),
        direction_bias: "short".to_string(),
        severity: severity.to_string(),
        risk_score: 82,
        data_quality_score: Some(82.0),
        confidence,
        created_at_ms: 1_000,
        fusion: ToxicSignalInboxFusionSummary {
            available: true,
            summary: "signal-only".to_string(),
        },
        replay: ToxicSignalInboxReplaySummary {
            available: true,
            evidence_count: 2,
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
            aligned_ratio: 0.6,
            adverse_ratio: 0.1,
        },
        recommendation: ToxicSignalInboxRecommendationSummary {
            available: true,
            action: if no_trade_only {
                "no_trade_only".to_string()
            } else {
                "keep".to_string()
            },
            no_trade_only,
            manual_review_required: true,
        },
        governance: ToxicSignalInboxGovernanceSummary {
            ledger_available: true,
            latest_decision: "watch_more".to_string(),
        },
        operator_action: ToxicSignalInboxOperatorAction::WatchSignalOnly,
        read_only: true,
        runtime_modified: false,
        analysis_only: true,
        execution_enabled: false,
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
            first_seen_at_ms: 900,
            last_seen_at_ms: 1_000,
            cooldown_window_ms: 300_000,
            max_severity: "high".to_string(),
            avg_confidence: 0.72,
            representative_signal_id: "signal-btc-1".to_string(),
            member_signal_ids: vec!["signal-btc-1".to_string()],
            operator_action: ToxicSignalGroupOperatorAction::ReviewGroupedSignal,
            suppression_hint: "Grouped for display only.".to_string(),
            original_signals_preserved: true,
            representative_confidence: 0.82,
            read_only: true,
            runtime_modified: false,
            analysis_only: true,
            execution_enabled: false,
        }],
    }
}

fn empty_groups() -> ToxicSignalGroupRecentResponse {
    ToxicSignalGroupRecentResponse {
        groups: Vec::new(),
        ..group_recent()
    }
}

fn alert_preview() -> ToxicSignalAlertPreviewResponse {
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
        status: "notification_preview_ready".to_string(),
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
            total_signals: 2,
            notify_candidates: 1,
            review_candidates: 0,
            suppressed_signals: 1,
            no_trade_only_signals: 1,
            governance_hold_signals: 0,
            not_enough_data_signals: 0,
        },
        by_symbol: vec![ToxicSignalAlertPreviewBucket {
            key: "BTCUSDT".to_string(),
            label: "BTCUSDT".to_string(),
            total_signals: 2,
            notify_candidates: 1,
            review_candidates: 0,
            suppressed_signals: 1,
            no_trade_only_signals: 1,
            not_enough_data_signals: 0,
        }],
        by_signal_kind: Vec::new(),
        items: vec![
            ToxicSignalAlertPreviewItem {
                signal_id: "signal-btc-1".to_string(),
                symbol: "BTCUSDT".to_string(),
                signal_kind: "short_bias_toxic_flow".to_string(),
                direction_bias: "short".to_string(),
                severity: "high".to_string(),
                confidence: 0.82,
                preview_status: "notify_candidate".to_string(),
                would_notify_if_enabled: true,
                no_trade_only: false,
                quality_bucket: "good".to_string(),
                latest_governance_decision: "watch_more".to_string(),
                markout_readiness: "ready".to_string(),
                suppression_reasons: Vec::new(),
                review_reasons: vec!["high severity".to_string()],
                preview_message: "notify".to_string(),
                notification_sent: false,
                execution_triggered: false,
                read_only: true,
                analysis_only: true,
                execution_enabled: false,
            },
            ToxicSignalAlertPreviewItem {
                signal_id: "signal-eth-1".to_string(),
                symbol: "ETHUSDT".to_string(),
                signal_kind: "trap_risk".to_string(),
                direction_bias: "short".to_string(),
                severity: "medium".to_string(),
                confidence: 0.61,
                preview_status: "suppressed_no_trade_only".to_string(),
                would_notify_if_enabled: false,
                no_trade_only: true,
                quality_bucket: "good".to_string(),
                latest_governance_decision: "watch_more".to_string(),
                markout_readiness: "ready".to_string(),
                suppression_reasons: vec!["no trade".to_string()],
                review_reasons: Vec::new(),
                preview_message: "suppress".to_string(),
                notification_sent: false,
                execution_triggered: false,
                read_only: true,
                analysis_only: true,
                execution_enabled: false,
            },
        ],
        operator_notes: Vec::new(),
        markdown: "# Preview".to_string(),
    }
}

fn empty_alert_preview() -> ToxicSignalAlertPreviewResponse {
    ToxicSignalAlertPreviewResponse {
        summary: ToxicSignalAlertPreviewSummary {
            total_signals: 0,
            notify_candidates: 0,
            review_candidates: 0,
            suppressed_signals: 0,
            no_trade_only_signals: 0,
            governance_hold_signals: 0,
            not_enough_data_signals: 0,
        },
        by_symbol: Vec::new(),
        items: Vec::new(),
        status: "empty_notification_preview".to_string(),
        ..alert_preview()
    }
}

fn daily_report() -> ToxicSignalReportDailyResponse {
    report_for_symbol("BTCUSDT")
}

fn report_for_symbol(symbol: &str) -> ToxicSignalReportDailyResponse {
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
            symbol: symbol.to_string(),
            view_only: true,
            persistent_watchlist_enabled: false,
            runtime_monitor_modified: false,
        },
        summary: ToxicSignalReportSummary {
            total_signals: 2,
            grouped_signals: 1,
            high_severity_signals: 1,
            no_trade_only_candidates: 1,
            downgrade_candidates: 0,
            not_enough_data_signals: 1,
        },
        markout_summary: ToxicSignalReportMarkoutSummary {
            aligned: 1,
            adverse: 0,
            neutral: 1,
            not_enough_data: 2,
        },
        by_symbol: vec![ToxicSignalReportBucket {
            key: symbol.to_string(),
            label: symbol.to_string(),
            signal_count: 2,
            high_severity_signals: 1,
            no_trade_only_candidates: 1,
            downgrade_candidates: 0,
            not_enough_data_signals: 1,
            avg_confidence: 0.71,
        }],
        by_signal_kind: Vec::new(),
        top_groups: vec![ToxicSignalReportTopGroup {
            group_id: "group-btc-1".to_string(),
            symbol: symbol.to_string(),
            signal_kind: "short_bias_toxic_flow".to_string(),
            direction_bias: "short".to_string(),
            count: 2,
            first_seen_at_ms: 900,
            last_seen_at_ms: 1_000,
            max_severity: "high".to_string(),
            avg_confidence: 0.72,
            representative_signal_id: "signal-btc-1".to_string(),
            original_signals_preserved: true,
        }],
        operator_notes: vec!["Signal-only report.".to_string()],
        markdown: "# Report".to_string(),
    }
}
