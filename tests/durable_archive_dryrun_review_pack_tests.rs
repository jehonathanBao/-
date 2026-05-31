use btc_toxic_flow_monitor_rs::{
    toxicity::durable_archive_dryrun_service::{
        durable_archive_dryrun_review_pack_by_id, durable_archive_dryrun_review_pack_latest,
        durable_archive_dryrun_write,
    },
    types::toxic_signal_history::{
        ToxicSignalHistoryAlertItem, ToxicSignalHistoryAlertRecentResponse,
        ToxicSignalHistoryRecentResponse, ToxicSignalHistoryReportItem,
        ToxicSignalHistoryReportRecentResponse, ToxicSignalHistorySignalItem,
    },
};

#[test]
fn durable_archive_dryrun_review_pack_aligns_json_and_markdown_with_contract() {
    let dry_run = durable_archive_dryrun_write(
        "BTCUSDT",
        &sample_history_recent(),
        &sample_alert_recent(),
        &sample_report_recent(),
    );
    let pack = durable_archive_dryrun_review_pack_latest(&dry_run);

    assert!(pack.found);
    assert!(pack.read_only);
    assert!(pack.analysis_only);
    assert!(pack.manual_review_required);
    assert!(!pack.execution_enabled);
    assert!(!pack.archive_write_enabled);
    assert!(!pack.notification_sent);
    assert_eq!(pack.summary.records_prepared, dry_run.records_prepared);
    assert_eq!(
        pack.summary.validation_error_count,
        dry_run.validation.errors.len()
    );
    assert_eq!(
        pack.summary.validation_warning_count,
        dry_run.validation.warnings.len()
    );
    assert_eq!(pack.validation.valid, dry_run.validation.valid);
    assert_eq!(
        pack.field_contract.source_snapshot_fields,
        dry_run.field_contract.source_snapshot_fields
    );
    assert!(pack
        .markdown
        .contains("# Durable Archive Dry-run Review Pack"));
    assert!(pack.markdown.contains("## Validation Errors"));
    assert!(pack.markdown.contains("## Field Contract"));
    assert!(pack.markdown.contains("## Safety Boundary"));
    assert!(pack.markdown.contains("archiveWriteEnabled=false"));
    assert!(pack.markdown.contains("executionTriggered=false"));
    assert!(pack.markdown.contains("No wallet/signing"));
}

#[test]
fn durable_archive_dryrun_review_pack_by_id_degrades_cleanly_when_id_is_missing() {
    let dry_run = durable_archive_dryrun_write(
        "BTCUSDT",
        &sample_history_recent(),
        &sample_alert_recent(),
        &sample_report_recent(),
    );
    let pack = durable_archive_dryrun_review_pack_by_id("BTCUSDT", &dry_run, "missing-pack-id");

    assert!(!pack.found);
    assert!(pack.read_only);
    assert!(pack.manual_review_required);
    assert_eq!(
        pack.validation.errors,
        vec!["review_pack_not_found".to_string()]
    );
    assert!(pack.markdown.contains("Found: false"));
}

fn sample_history_recent() -> ToxicSignalHistoryRecentResponse {
    ToxicSignalHistoryRecentResponse {
        read_only: true,
        runtime_modified: false,
        analysis_only: true,
        execution_enabled: false,
        retention_mode: "in_memory_bounded".to_string(),
        durable_storage_enabled: false,
        database_write_enabled: false,
        selected_symbol: "BTCUSDT".to_string(),
        items: vec![ToxicSignalHistorySignalItem {
            signal_id: "signal-btc-1".to_string(),
            symbol: "BTCUSDT".to_string(),
            signal_kind: "short_bias_toxic_flow".to_string(),
            direction_bias: "short".to_string(),
            severity: "high".to_string(),
            confidence: 0.82,
            created_at_ms: 1_000,
            markout_one_minute: "aligned".to_string(),
            markout_five_minute: "neutral".to_string(),
            markout_fifteen_minute: "not_enough_data".to_string(),
            markout_one_hour: "not_enough_data".to_string(),
            quality_bucket: "good".to_string(),
            recommendation_action: "keep".to_string(),
            no_trade_only: false,
            source: "signal_inbox".to_string(),
            history_recorded_at_ms: 1_234,
            operator_action: "watch_signal_only".to_string(),
        }],
        group_items: Vec::new(),
        operator_notes: Vec::new(),
    }
}

fn sample_alert_recent() -> ToxicSignalHistoryAlertRecentResponse {
    ToxicSignalHistoryAlertRecentResponse {
        read_only: true,
        runtime_modified: false,
        analysis_only: true,
        execution_enabled: false,
        retention_mode: "in_memory_bounded".to_string(),
        durable_storage_enabled: false,
        database_write_enabled: false,
        selected_symbol: "BTCUSDT".to_string(),
        items: vec![ToxicSignalHistoryAlertItem {
            signal_id: "signal-btc-1".to_string(),
            symbol: "BTCUSDT".to_string(),
            signal_kind: "short_bias_toxic_flow".to_string(),
            preview_status: "notify_candidate".to_string(),
            would_notify_if_enabled: true,
            no_trade_only: false,
            markout_readiness: "aligned_present".to_string(),
            source: "signal_alert_preview".to_string(),
            history_recorded_at_ms: 1_234,
            notification_sent: false,
            execution_triggered: false,
        }],
    }
}

fn sample_report_recent() -> ToxicSignalHistoryReportRecentResponse {
    ToxicSignalHistoryReportRecentResponse {
        read_only: true,
        runtime_modified: false,
        analysis_only: true,
        execution_enabled: false,
        retention_mode: "in_memory_bounded".to_string(),
        durable_storage_enabled: false,
        database_write_enabled: false,
        selected_symbol: "BTCUSDT".to_string(),
        items: vec![ToxicSignalHistoryReportItem {
            report_type: "daily".to_string(),
            date: "2026-05-30".to_string(),
            symbol: "BTCUSDT".to_string(),
            total_signals: 1,
            grouped_signals: 1,
            high_severity_signals: 1,
            no_trade_only_candidates: 0,
            downgrade_candidates: 0,
            not_enough_data_signals: 1,
            source: "signal_report".to_string(),
            history_recorded_at_ms: 1_234,
        }],
    }
}
