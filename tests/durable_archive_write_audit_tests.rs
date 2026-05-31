use btc_toxic_flow_monitor_rs::toxicity::durable_archive_write_audit_service::{
    durable_archive_write_audit_latest, durable_archive_write_audit_recent,
    durable_archive_write_audit_status,
};

#[test]
fn durable_archive_write_audit_status_is_preview_only_and_non_persistent() {
    let payload = durable_archive_write_audit_status();

    assert!(payload.read_only);
    assert!(payload.analysis_only);
    assert!(payload.manual_review_required);
    assert!(!payload.runtime_modified);
    assert!(!payload.execution_enabled);
    assert_eq!(payload.audit_mode, "preview_only");
    assert!(!payload.attempt_log_persistence_enabled);
    assert!(!payload.attempt_log_file_write_enabled);
    assert!(!payload.archive_write_enabled);
    assert!(!payload.durable_storage_enabled);
    assert!(!payload.database_write_enabled);
    assert!(!payload.jsonl_write_enabled);
    assert!(!payload.sqlite_write_enabled);
    assert!(!payload.file_archive_write_enabled);
    assert!(!payload.notification_sent);
    assert!(!payload.execution_triggered);
    assert_eq!(payload.recent_attempt_count, 0);
    assert!(!payload.latest_attempt_available);
}

#[test]
fn durable_archive_write_audit_recent_gracefully_returns_no_attempts() {
    let payload = durable_archive_write_audit_recent();

    assert_eq!(payload.audit_mode, "preview_only");
    assert!(!payload.attempt_log_persistence_enabled);
    assert!(!payload.attempt_log_file_write_enabled);
    assert!(payload.attempts.is_empty());
    assert!(!payload.latest_attempt_available);
    assert_eq!(
        payload.operator_note,
        "No rejected archive write attempts are currently available in preview memory."
    );
}

#[test]
fn durable_archive_write_audit_latest_gracefully_returns_no_attempt() {
    let payload = durable_archive_write_audit_latest();

    assert_eq!(payload.audit_mode, "preview_only");
    assert!(!payload.attempt_log_persistence_enabled);
    assert!(!payload.attempt_log_file_write_enabled);
    assert!(!payload.latest_attempt_available);
    assert!(payload.attempt.is_none());
    assert_eq!(
        payload.operator_note,
        "No rejected archive write attempts are currently available in preview memory."
    );
}
