use btc_toxic_flow_monitor_rs::{
    toxicity::durable_archive_write_service::{
        durable_archive_write_reject, durable_archive_write_status,
    },
    types::durable_archive_write::DurableArchiveWriteRequest,
};

#[test]
fn durable_archive_write_status_is_disabled_by_default() {
    let status = durable_archive_write_status();

    assert!(status.read_only);
    assert!(status.analysis_only);
    assert!(status.manual_review_required);
    assert!(!status.archive_write_enabled);
    assert!(!status.durable_storage_enabled);
    assert!(!status.database_write_enabled);
    assert!(!status.jsonl_write_enabled);
    assert!(!status.sqlite_write_enabled);
    assert!(!status.file_archive_write_enabled);
    assert!(!status.execution_enabled);
    assert!(!status.runtime_modified);
    assert!(!status.notification_sent);
    assert!(!status.execution_triggered);
    assert!(status.dry_run_contract_preserved);
    assert!(status.review_pack_contract_preserved);
    assert_eq!(status.write_status, "disabled_by_default");
    assert_eq!(status.rejection_reason, "archive_write_disabled_by_default");
    assert_eq!(status.records_written, 0);
    assert_eq!(status.bytes_written, 0);
}

#[test]
fn durable_archive_write_rejects_intent_without_side_effect_counters() {
    let response = durable_archive_write_reject(Some(DurableArchiveWriteRequest {
        requested_by: Some("operator_review".to_string()),
        dry_run_id: Some("dryrun-btcusdt-1".to_string()),
        requested_records: Some(10),
        write_intent: Some("operator_attempted_archive_write".to_string()),
    }));

    assert!(!response.ok);
    assert!(!response.write_accepted);
    assert!(response.write_rejected);
    assert_eq!(
        response.rejection_reason,
        "archive_write_disabled_by_default"
    );
    assert_eq!(response.records_written, 0);
    assert_eq!(response.bytes_written, 0);
    assert!(response.read_only);
    assert!(response.analysis_only);
    assert!(response.manual_review_required);
    assert!(!response.archive_write_enabled);
    assert!(!response.database_write_enabled);
    assert!(!response.jsonl_write_enabled);
    assert!(!response.sqlite_write_enabled);
    assert!(!response.file_archive_write_enabled);
    assert!(!response.execution_enabled);
    assert!(!response.runtime_modified);
    assert!(!response.notification_sent);
    assert!(!response.execution_triggered);
    assert!(response.dry_run_contract_preserved);
    assert!(response.review_pack_contract_preserved);
    assert_eq!(response.request_contract.requested_records, Some(10));
}

#[test]
fn durable_archive_write_gate_does_not_create_archive_files() {
    let base = std::env::temp_dir().join("btc-toxic-flow-s16-write-gate-test");
    let db_path = base.with_extension("db");
    let jsonl_path = base.with_extension("jsonl");
    let sqlite_path = base.with_extension("sqlite");
    let archive_path = base.with_extension("archive");

    for path in [&db_path, &jsonl_path, &sqlite_path, &archive_path] {
        let _ = std::fs::remove_file(path);
    }

    let response = durable_archive_write_reject(Some(DurableArchiveWriteRequest {
        requested_by: Some("operator_review".to_string()),
        dry_run_id: Some("dryrun-no-file-write".to_string()),
        requested_records: Some(1),
        write_intent: Some(base.display().to_string()),
    }));

    assert!(response.write_rejected);
    assert_eq!(response.records_written, 0);
    assert_eq!(response.bytes_written, 0);
    assert!(!db_path.exists());
    assert!(!jsonl_path.exists());
    assert!(!sqlite_path.exists());
    assert!(!archive_path.exists());
}
