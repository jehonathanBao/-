use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DurableArchiveWriteStatusResponse {
    pub read_only: bool,
    pub analysis_only: bool,
    pub manual_review_required: bool,
    pub archive_write_enabled: bool,
    pub durable_storage_enabled: bool,
    pub database_write_enabled: bool,
    pub jsonl_write_enabled: bool,
    pub sqlite_write_enabled: bool,
    pub file_archive_write_enabled: bool,
    pub execution_enabled: bool,
    pub runtime_modified: bool,
    pub notification_sent: bool,
    pub execution_triggered: bool,
    pub dry_run_contract_preserved: bool,
    pub review_pack_contract_preserved: bool,
    pub write_status: String,
    pub rejection_reason: String,
    pub records_written: u64,
    pub bytes_written: u64,
    pub safety_boundary: Vec<String>,
    pub operator_notes: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DurableArchiveWriteRequest {
    pub requested_by: Option<String>,
    pub dry_run_id: Option<String>,
    pub requested_records: Option<u64>,
    pub write_intent: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DurableArchiveWriteRejectedResponse {
    pub ok: bool,
    pub write_accepted: bool,
    pub write_rejected: bool,
    pub rejection_reason: String,
    pub records_written: u64,
    pub bytes_written: u64,
    pub read_only: bool,
    pub analysis_only: bool,
    pub manual_review_required: bool,
    pub archive_write_enabled: bool,
    pub durable_storage_enabled: bool,
    pub database_write_enabled: bool,
    pub jsonl_write_enabled: bool,
    pub sqlite_write_enabled: bool,
    pub file_archive_write_enabled: bool,
    pub execution_enabled: bool,
    pub runtime_modified: bool,
    pub notification_sent: bool,
    pub execution_triggered: bool,
    pub dry_run_contract_preserved: bool,
    pub review_pack_contract_preserved: bool,
    pub write_status: String,
    pub request_contract: DurableArchiveWriteRequest,
    pub safety_boundary: Vec<String>,
    pub operator_notes: Vec<String>,
}
