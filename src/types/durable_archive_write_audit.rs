use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DurableArchiveWriteAuditStatusResponse {
    pub read_only: bool,
    pub analysis_only: bool,
    pub manual_review_required: bool,
    pub runtime_modified: bool,
    pub execution_enabled: bool,
    pub audit_mode: String,
    pub attempt_log_persistence_enabled: bool,
    pub attempt_log_file_write_enabled: bool,
    pub archive_write_enabled: bool,
    pub durable_storage_enabled: bool,
    pub database_write_enabled: bool,
    pub jsonl_write_enabled: bool,
    pub sqlite_write_enabled: bool,
    pub file_archive_write_enabled: bool,
    pub notification_sent: bool,
    pub execution_triggered: bool,
    pub recent_attempt_count: usize,
    pub latest_attempt_available: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DurableArchiveWriteAuditAttemptPreview {
    pub attempt_id: String,
    pub created_at_ms: u64,
    pub endpoint: String,
    pub write_accepted: bool,
    pub write_rejected: bool,
    pub rejection_reason: String,
    pub records_requested: u64,
    pub records_written: u64,
    pub bytes_written: u64,
    pub archive_write_enabled: bool,
    pub durable_storage_enabled: bool,
    pub database_write_enabled: bool,
    pub jsonl_write_enabled: bool,
    pub sqlite_write_enabled: bool,
    pub file_archive_write_enabled: bool,
    pub runtime_modified: bool,
    pub execution_enabled: bool,
    pub notification_sent: bool,
    pub execution_triggered: bool,
    pub safety_summary: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DurableArchiveWriteAuditRecentResponse {
    pub read_only: bool,
    pub analysis_only: bool,
    pub manual_review_required: bool,
    pub runtime_modified: bool,
    pub execution_enabled: bool,
    pub audit_mode: String,
    pub attempt_log_persistence_enabled: bool,
    pub attempt_log_file_write_enabled: bool,
    pub archive_write_enabled: bool,
    pub durable_storage_enabled: bool,
    pub database_write_enabled: bool,
    pub jsonl_write_enabled: bool,
    pub sqlite_write_enabled: bool,
    pub file_archive_write_enabled: bool,
    pub notification_sent: bool,
    pub execution_triggered: bool,
    pub attempts: Vec<DurableArchiveWriteAuditAttemptPreview>,
    pub latest_attempt_available: bool,
    pub operator_note: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DurableArchiveWriteAuditLatestResponse {
    pub read_only: bool,
    pub analysis_only: bool,
    pub manual_review_required: bool,
    pub runtime_modified: bool,
    pub execution_enabled: bool,
    pub audit_mode: String,
    pub attempt_log_persistence_enabled: bool,
    pub attempt_log_file_write_enabled: bool,
    pub archive_write_enabled: bool,
    pub durable_storage_enabled: bool,
    pub database_write_enabled: bool,
    pub jsonl_write_enabled: bool,
    pub sqlite_write_enabled: bool,
    pub file_archive_write_enabled: bool,
    pub notification_sent: bool,
    pub execution_triggered: bool,
    pub latest_attempt_available: bool,
    pub attempt: Option<DurableArchiveWriteAuditAttemptPreview>,
    pub operator_note: String,
}
