use btc_toxic_flow_monitor_rs::{
    toxicity::durable_archive_dryrun_service::{
        durable_archive_dryrun_validate_payload, durable_archive_dryrun_write,
    },
    types::toxic_signal_history::{
        ToxicSignalHistoryAlertItem, ToxicSignalHistoryAlertRecentResponse,
        ToxicSignalHistoryRecentResponse, ToxicSignalHistoryReportItem,
        ToxicSignalHistoryReportRecentResponse, ToxicSignalHistorySignalItem,
    },
};
use serde_json::Value;
use std::sync::OnceLock;

#[test]
fn durable_archive_dryrun_payload_matches_schema_contract() {
    let history_recent = sample_history_recent();
    let alert_recent = sample_alert_recent();
    let report_recent = sample_report_recent();

    let payload =
        durable_archive_dryrun_write("BTCUSDT", &history_recent, &alert_recent, &report_recent);

    assert!(payload.ok);
    assert!(payload.read_only);
    assert!(!payload.runtime_modified);
    assert!(payload.analysis_only);
    assert!(!payload.execution_enabled);
    assert_eq!(payload.action, "dry_run_write");
    assert_eq!(payload.schema_version, 1);
    assert_eq!(payload.records_prepared, 1);
    assert!(!payload.archive_write_enabled);
    assert!(!payload.durable_storage_enabled);
    assert!(!payload.database_write_enabled);
    assert!(!payload.jsonl_write_enabled);
    assert!(!payload.sqlite_write_enabled);
    assert!(!payload.notification_sent);
    assert!(!payload.execution_triggered);
    assert_eq!(payload.write_mode, "dry_run_only");
    assert!(payload.dry_run);
    assert!(payload.validation.valid);
    assert!(payload.validation.field_types_valid);
    assert!(payload.validation.source_snapshot_fields_valid);
    assert!(payload.validation.derived_fields_valid);
    assert!(payload.validation.evidence_refs_valid);
    assert!(!payload.validation.persistence_attempted);
    assert!(payload.validation.errors.is_empty());
    assert!(payload
        .field_contract
        .source_snapshot_fields
        .contains(&"sourceSignalId".to_string()));
    assert!(payload
        .field_contract
        .derived_fields
        .contains(&"toxicityScore".to_string()));
    assert!(payload
        .field_contract
        .evidence_reference_fields
        .contains(&"evidenceRefs".to_string()));

    let record = &payload.records[0];
    assert_eq!(record.archive_record_id, "archive-dryrun-signal-btc-1");
    assert_eq!(record.source_signal_id, "signal-btc-1");
    assert_eq!(record.source_signal_type, "short_bias_toxic_flow");
    assert_eq!(record.symbol, "BTCUSDT");
    assert_eq!(record.signal_layer, "signal_inbox");
    assert_eq!(record.direction, "short");
    assert!(record.toxicity_score > 0.0);
    assert_eq!(
        record.evidence_refs.signal_history_ref,
        "signal_history:signal-btc-1"
    );
    assert_eq!(
        record.replay_ref.as_deref(),
        Some("replay:BTCUSDT:signal-btc-1")
    );
    assert_eq!(record.markout_ref.as_deref(), Some("markout:signal-btc-1"));
    assert_eq!(record.governance_ref.as_deref(), Some("governance:BTCUSDT"));
    assert_eq!(
        record.evidence_refs.alert_preview_ref.as_deref(),
        Some("alert_preview:signal-btc-1:notify_candidate")
    );
    assert_eq!(
        record.evidence_refs.report_ref.as_deref(),
        Some("signal_report:2026-05-30:BTCUSDT")
    );
}

#[test]
fn durable_archive_dryrun_validation_matrix_accepts_good_payload_only_as_dryrun() {
    let payload = durable_archive_dryrun_validate_payload("BTCUSDT", case_payload("good_payload"));

    assert!(payload.validation.valid);
    assert!(payload.validation.errors.is_empty());
    assert_eq!(payload.records_prepared, 1);
    assert!(!payload.archive_write_enabled);
    assert!(!payload.database_write_enabled);
    assert!(!payload.jsonl_write_enabled);
    assert!(!payload.sqlite_write_enabled);
    assert!(!payload.notification_sent);
    assert!(!payload.execution_triggered);
    assert!(!payload.validation.persistence_attempted);
}

#[test]
fn durable_archive_dryrun_validation_matrix_rejects_missing_and_forbidden_fields() {
    let missing =
        durable_archive_dryrun_validate_payload("BTCUSDT", case_payload("missing_required_fields"));
    assert!(!missing.validation.valid);
    assert_eq!(missing.records_prepared, 0);
    assert!(missing
        .validation
        .errors
        .contains(&"missing_required_field".to_string()));
    assert!(missing
        .validation
        .missing_required_fields
        .contains(&"signalId".to_string()));
    assert!(missing
        .validation
        .missing_required_fields
        .contains(&"signalKind".to_string()));
    assert!(missing
        .validation
        .missing_required_fields
        .contains(&"createdAtMs".to_string()));
    assert!(missing
        .validation
        .missing_required_fields
        .contains(&"schemaVersion".to_string()));
    assert!(missing
        .validation
        .missing_required_fields
        .contains(&"sourceModule".to_string()));

    let forbidden =
        durable_archive_dryrun_validate_payload("BTCUSDT", case_payload("forbidden_fields"));
    assert!(!forbidden.validation.valid);
    assert!(forbidden
        .validation
        .errors
        .contains(&"forbidden_field_present".to_string()));
    assert!(forbidden
        .validation
        .forbidden_fields
        .contains(&"privateKey".to_string()));
    assert!(forbidden
        .validation
        .forbidden_fields
        .contains(&"telegramBotToken".to_string()));
}

#[test]
fn durable_archive_dryrun_validation_matrix_flags_execution_and_notification_inputs() {
    let execution =
        durable_archive_dryrun_validate_payload("BTCUSDT", case_payload("execution_like_fields"));
    assert!(!execution.validation.valid);
    assert!(execution.validation.unsafe_execution_field_detected);
    assert!(execution
        .validation
        .errors
        .contains(&"unsafe_execution_field_detected".to_string()));
    assert!(!execution.execution_enabled);

    let notification = durable_archive_dryrun_validate_payload(
        "BTCUSDT",
        case_payload("unsafe_notification_fields"),
    );
    assert!(!notification.validation.valid);
    assert!(notification
        .validation
        .errors
        .contains(&"unsafe_notification_field_present".to_string()));
    assert!(!notification.notification_sent);
    assert!(!notification.execution_triggered);
}

#[test]
fn durable_archive_dryrun_validation_matrix_reports_duplicates_and_bad_refs() {
    let duplicate =
        durable_archive_dryrun_validate_payload("BTCUSDT", case_payload("duplicate_signal_id"));
    assert!(duplicate.validation.valid);
    assert!(duplicate
        .validation
        .warnings
        .contains(&"duplicate_signal_id".to_string()));
    assert!(duplicate
        .validation
        .duplicate_signal_ids
        .contains(&"signal_001".to_string()));

    let bad_schema =
        durable_archive_dryrun_validate_payload("BTCUSDT", case_payload("invalid_schema_version"));
    assert!(!bad_schema.validation.valid);
    assert!(bad_schema
        .validation
        .errors
        .contains(&"invalid_schema_version".to_string()));

    let bad_refs =
        durable_archive_dryrun_validate_payload("BTCUSDT", case_payload("invalid_evidence_refs"));
    assert!(!bad_refs.validation.valid);
    assert!(bad_refs
        .validation
        .errors
        .contains(&"invalid_evidence_ref".to_string()));
}

#[test]
fn durable_archive_dryrun_validation_matrix_handles_oversized_and_empty_payloads() {
    let oversized =
        durable_archive_dryrun_validate_payload("BTCUSDT", case_payload("oversized_payload"));
    assert!(oversized.validation.valid);
    assert!(oversized
        .validation
        .warnings
        .contains(&"payload_too_large".to_string()));

    let empty = durable_archive_dryrun_validate_payload("BTCUSDT", case_payload("empty_records"));
    assert!(!empty.validation.valid);
    assert_eq!(empty.records_prepared, 0);
    assert!(empty.validation.errors.contains(&"no_records".to_string()));
    assert!(!empty.archive_write_enabled);
}

#[test]
fn durable_archive_dryrun_keeps_safety_flags_fixed_and_does_not_persist() {
    let payload = durable_archive_dryrun_write(
        "BTCUSDT",
        &sample_history_recent(),
        &sample_alert_recent(),
        &sample_report_recent(),
    );
    let record = &payload.records[0];

    assert!(!record.safety_flags.archive_write_enabled);
    assert!(!record.safety_flags.durable_storage_enabled);
    assert!(!record.safety_flags.database_write_enabled);
    assert!(!record.safety_flags.jsonl_write_enabled);
    assert!(!record.safety_flags.sqlite_write_enabled);
    assert!(!record.safety_flags.runtime_modified);
    assert!(!record.safety_flags.execution_enabled);
    assert!(!record.safety_flags.notification_sent);
    assert!(!record.safety_flags.execution_triggered);
    assert!(payload
        .safety_boundary
        .contains(&"No order placement".to_string()));
    assert!(payload
        .safety_boundary
        .contains(&"No wallet/signing".to_string()));
    assert!(payload
        .safety_boundary
        .contains(&"No live trading".to_string()));
    assert!(!payload.validation.persistence_attempted);
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

fn validation_cases() -> Value {
    serde_json::from_str(include_str!(
        "fixtures/durable_archive_dryrun_validation_cases.json"
    ))
    .expect("validation fixture json")
}

fn case_payload(name: &str) -> &'static Value {
    static CASES: OnceLock<Value> = OnceLock::new();
    let cases = CASES.get_or_init(validation_cases);
    cases.get(name).expect("fixture case exists")
}
