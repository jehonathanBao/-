use crate::types::durable_archive_write_audit::{
    DurableArchiveWriteAuditLatestResponse, DurableArchiveWriteAuditRecentResponse,
    DurableArchiveWriteAuditStatusResponse,
};

const AUDIT_MODE: &str = "preview_only";
const OPERATOR_NOTE: &str =
    "No rejected archive write attempts are currently available in preview memory.";

pub fn build_durable_archive_write_audit_status() -> DurableArchiveWriteAuditStatusResponse {
    DurableArchiveWriteAuditStatusResponse {
        read_only: true,
        analysis_only: true,
        manual_review_required: true,
        runtime_modified: false,
        execution_enabled: false,
        audit_mode: AUDIT_MODE.to_string(),
        attempt_log_persistence_enabled: false,
        attempt_log_file_write_enabled: false,
        archive_write_enabled: false,
        durable_storage_enabled: false,
        database_write_enabled: false,
        jsonl_write_enabled: false,
        sqlite_write_enabled: false,
        file_archive_write_enabled: false,
        notification_sent: false,
        execution_triggered: false,
        recent_attempt_count: 0,
        latest_attempt_available: false,
    }
}

pub fn build_durable_archive_write_audit_recent() -> DurableArchiveWriteAuditRecentResponse {
    DurableArchiveWriteAuditRecentResponse {
        read_only: true,
        analysis_only: true,
        manual_review_required: true,
        runtime_modified: false,
        execution_enabled: false,
        audit_mode: AUDIT_MODE.to_string(),
        attempt_log_persistence_enabled: false,
        attempt_log_file_write_enabled: false,
        archive_write_enabled: false,
        durable_storage_enabled: false,
        database_write_enabled: false,
        jsonl_write_enabled: false,
        sqlite_write_enabled: false,
        file_archive_write_enabled: false,
        notification_sent: false,
        execution_triggered: false,
        attempts: Vec::new(),
        latest_attempt_available: false,
        operator_note: OPERATOR_NOTE.to_string(),
    }
}

pub fn build_durable_archive_write_audit_latest() -> DurableArchiveWriteAuditLatestResponse {
    DurableArchiveWriteAuditLatestResponse {
        read_only: true,
        analysis_only: true,
        manual_review_required: true,
        runtime_modified: false,
        execution_enabled: false,
        audit_mode: AUDIT_MODE.to_string(),
        attempt_log_persistence_enabled: false,
        attempt_log_file_write_enabled: false,
        archive_write_enabled: false,
        durable_storage_enabled: false,
        database_write_enabled: false,
        jsonl_write_enabled: false,
        sqlite_write_enabled: false,
        file_archive_write_enabled: false,
        notification_sent: false,
        execution_triggered: false,
        latest_attempt_available: false,
        attempt: None,
        operator_note: OPERATOR_NOTE.to_string(),
    }
}
