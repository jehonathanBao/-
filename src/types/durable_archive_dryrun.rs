use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DurableArchiveDryRunEvidenceRefs {
    pub signal_history_ref: String,
    pub replay_ref: Option<String>,
    pub markout_ref: Option<String>,
    pub governance_ref: Option<String>,
    pub alert_preview_ref: Option<String>,
    pub report_ref: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DurableArchiveDryRunSafetyFlags {
    pub archive_write_enabled: bool,
    pub durable_storage_enabled: bool,
    pub database_write_enabled: bool,
    pub jsonl_write_enabled: bool,
    pub sqlite_write_enabled: bool,
    pub runtime_modified: bool,
    pub execution_enabled: bool,
    pub notification_sent: bool,
    pub execution_triggered: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DurableArchiveDryRunRecord {
    pub archive_record_id: String,
    pub schema_version: u32,
    pub created_at_ms: u64,
    pub source_signal_id: String,
    pub source_signal_type: String,
    pub symbol: String,
    pub signal_ts_ms: u64,
    pub signal_layer: String,
    pub direction: String,
    pub toxicity_score: f64,
    pub confidence: f64,
    pub evidence_refs: DurableArchiveDryRunEvidenceRefs,
    pub replay_ref: Option<String>,
    pub markout_ref: Option<String>,
    pub governance_ref: Option<String>,
    pub safety_flags: DurableArchiveDryRunSafetyFlags,
    pub write_mode: String,
    pub archive_write_enabled: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DurableArchiveDryRunFieldContract {
    pub source_snapshot_fields: Vec<String>,
    pub derived_fields: Vec<String>,
    pub evidence_reference_fields: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DurableArchiveDryRunValidation {
    pub valid: bool,
    pub field_types_valid: bool,
    pub source_snapshot_fields_valid: bool,
    pub derived_fields_valid: bool,
    pub evidence_refs_valid: bool,
    pub persistence_attempted: bool,
    pub errors: Vec<String>,
    pub warnings: Vec<String>,
    pub unsafe_fields_detected: Vec<String>,
    pub duplicate_signal_ids: Vec<String>,
    pub missing_required_fields: Vec<String>,
    pub forbidden_fields: Vec<String>,
    pub unsafe_execution_field_detected: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DurableArchiveDryRunResponse {
    pub ok: bool,
    pub read_only: bool,
    pub runtime_modified: bool,
    pub analysis_only: bool,
    pub execution_enabled: bool,
    pub dry_run: bool,
    pub action: String,
    pub selected_symbol: String,
    pub schema_version: u32,
    pub records_prepared: usize,
    pub archive_write_enabled: bool,
    pub durable_storage_enabled: bool,
    pub database_write_enabled: bool,
    pub jsonl_write_enabled: bool,
    pub sqlite_write_enabled: bool,
    pub notification_sent: bool,
    pub execution_triggered: bool,
    pub write_mode: String,
    pub field_contract: DurableArchiveDryRunFieldContract,
    pub validation: DurableArchiveDryRunValidation,
    pub records: Vec<DurableArchiveDryRunRecord>,
    pub safety_boundary: Vec<String>,
    pub operator_notes: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DurableArchiveDryRunReviewPackSummary {
    pub records_prepared: usize,
    pub validation_error_count: usize,
    pub validation_warning_count: usize,
    pub unsafe_field_count: usize,
    pub duplicate_signal_id_count: usize,
    pub missing_required_field_count: usize,
    pub forbidden_field_count: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DurableArchiveDryRunReviewPackResponse {
    pub found: bool,
    pub read_only: bool,
    pub runtime_modified: bool,
    pub analysis_only: bool,
    pub execution_enabled: bool,
    pub manual_review_required: bool,
    pub archive_write_enabled: bool,
    pub durable_storage_enabled: bool,
    pub database_write_enabled: bool,
    pub jsonl_write_enabled: bool,
    pub sqlite_write_enabled: bool,
    pub notification_sent: bool,
    pub execution_triggered: bool,
    pub dry_run: bool,
    pub review_pack_type: String,
    pub dry_run_id: String,
    pub selected_symbol: String,
    pub source_action: String,
    pub summary: DurableArchiveDryRunReviewPackSummary,
    pub field_contract: DurableArchiveDryRunFieldContract,
    pub validation: DurableArchiveDryRunValidation,
    pub records: Vec<DurableArchiveDryRunRecord>,
    pub safety_boundary: Vec<String>,
    pub operator_notes: Vec<String>,
    pub markdown: String,
}
