use crate::types::durable_archive_write::{
    DurableArchiveWriteRejectedResponse, DurableArchiveWriteRequest,
    DurableArchiveWriteStatusResponse,
};

const REJECTION_REASON: &str = "archive_write_disabled_by_default";
const WRITE_STATUS: &str = "disabled_by_default";

pub fn build_durable_archive_write_status() -> DurableArchiveWriteStatusResponse {
    DurableArchiveWriteStatusResponse {
        read_only: true,
        analysis_only: true,
        manual_review_required: true,
        archive_write_enabled: false,
        durable_storage_enabled: false,
        database_write_enabled: false,
        jsonl_write_enabled: false,
        sqlite_write_enabled: false,
        file_archive_write_enabled: false,
        execution_enabled: false,
        runtime_modified: false,
        notification_sent: false,
        execution_triggered: false,
        dry_run_contract_preserved: true,
        review_pack_contract_preserved: true,
        write_status: WRITE_STATUS.to_string(),
        rejection_reason: REJECTION_REASON.to_string(),
        records_written: 0,
        bytes_written: 0,
        safety_boundary: durable_archive_write_safety_boundary(),
        operator_notes: durable_archive_write_operator_notes(),
    }
}

pub fn reject_durable_archive_write(
    request_contract: DurableArchiveWriteRequest,
) -> DurableArchiveWriteRejectedResponse {
    DurableArchiveWriteRejectedResponse {
        ok: false,
        write_accepted: false,
        write_rejected: true,
        rejection_reason: REJECTION_REASON.to_string(),
        records_written: 0,
        bytes_written: 0,
        read_only: true,
        analysis_only: true,
        manual_review_required: true,
        archive_write_enabled: false,
        durable_storage_enabled: false,
        database_write_enabled: false,
        jsonl_write_enabled: false,
        sqlite_write_enabled: false,
        file_archive_write_enabled: false,
        execution_enabled: false,
        runtime_modified: false,
        notification_sent: false,
        execution_triggered: false,
        dry_run_contract_preserved: true,
        review_pack_contract_preserved: true,
        write_status: WRITE_STATUS.to_string(),
        request_contract,
        safety_boundary: durable_archive_write_safety_boundary(),
        operator_notes: durable_archive_write_operator_notes(),
    }
}

pub fn empty_durable_archive_write_request() -> DurableArchiveWriteRequest {
    DurableArchiveWriteRequest {
        requested_by: None,
        dry_run_id: None,
        requested_records: None,
        write_intent: Some("write_disabled_by_default_probe".to_string()),
    }
}

fn durable_archive_write_safety_boundary() -> Vec<String> {
    [
        "archiveWriteEnabled=false",
        "durableStorageEnabled=false",
        "databaseWriteEnabled=false",
        "jsonlWriteEnabled=false",
        "sqliteWriteEnabled=false",
        "fileArchiveWriteEnabled=false",
        "runtimeModified=false",
        "executionEnabled=false",
        "notificationSent=false",
        "executionTriggered=false",
        "manualReviewRequired=true",
        "dryRunContractPreserved=true",
        "reviewPackContractPreserved=true",
        "recordsWritten=0",
        "bytesWritten=0",
        "writeRejected=true",
        "archive_write_disabled_by_default",
        "No order placement",
        "No wallet/signing",
        "No live trading",
    ]
    .into_iter()
    .map(str::to_string)
    .collect()
}

fn durable_archive_write_operator_notes() -> Vec<String> {
    vec![
        "This is a disabled-by-default write gate.".to_string(),
        "It does not enable durable archive writes.".to_string(),
        "It does not write DB, JSONL, SQLite, or files.".to_string(),
        "Unsafe write attempts are rejected by default.".to_string(),
    ]
}
