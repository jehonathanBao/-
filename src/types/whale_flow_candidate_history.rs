use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct WhaleFlowCandidateHistoryStatusResponse {
    pub read_only: bool,
    pub analysis_only: bool,
    pub execution_enabled: bool,
    pub runtime_modified: bool,
    pub selected_symbol: String,
    pub retention_mode: String,
    pub durable_storage_enabled: bool,
    pub database_write_enabled: bool,
    pub jsonl_write_enabled: bool,
    pub sqlite_write_enabled: bool,
    pub archive_write_enabled: bool,
    pub current_candidates: usize,
    pub max_candidates: usize,
    pub oldest_candidate_at_ms: Option<u64>,
    pub latest_candidate_at_ms: Option<u64>,
    pub deduplicated_count: u64,
    pub evicted_count: u64,
    pub recorded_count: u64,
    pub resolved_markout_evidence_count: usize,
    pub unresolved_candidate_count: usize,
    pub not_enough_data_count: usize,
    pub min_candidates_required: usize,
    pub min_resolved_evidence_required: usize,
    pub max_not_enough_data_rate_for_tuning: f64,
    pub calibration_ready: bool,
    pub calibration_blocked_reasons: Vec<String>,
    pub operator_notes: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct WhaleFlowCandidateHistoryItem {
    pub candidate_id: String,
    pub symbol: String,
    pub classification: String,
    pub window_ms: u64,
    pub volume_btc: f64,
    pub direction_bias: String,
    pub direction_ratio: f64,
    pub relative_volume_multiple: Option<f64>,
    pub venue_confluence_count: usize,
    pub baseline_source: String,
    pub data_quality: String,
    pub created_at_ms: u64,
    pub outcome_status: String,
    pub markout_status: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct WhaleFlowCandidateHistoryRecentResponse {
    pub read_only: bool,
    pub analysis_only: bool,
    pub execution_enabled: bool,
    pub runtime_modified: bool,
    pub selected_symbol: String,
    pub retention_mode: String,
    pub status: String,
    pub items: Vec<WhaleFlowCandidateHistoryItem>,
    pub operator_notes: Vec<String>,
}
