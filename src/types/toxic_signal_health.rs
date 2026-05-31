use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ToxicSignalHealthSummary {
    pub inbox_available: bool,
    pub groups_available: bool,
    pub detail_available: bool,
    pub daily_report_available: bool,
    pub alert_preview_available: bool,
    pub history_available: bool,
    pub total_signals: usize,
    pub signals_with_markout: usize,
    pub signals_missing_markout: usize,
    pub signals_with_quality: usize,
    pub signals_missing_quality: usize,
    pub signals_with_recommendation: usize,
    pub signals_missing_recommendation: usize,
    pub signals_with_governance: usize,
    pub signals_missing_governance: usize,
    pub not_enough_data_count: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ToxicSignalHealthIssue {
    pub kind: String,
    pub severity: String,
    pub count: usize,
    pub operator_note: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ToxicSignalHealthSummaryResponse {
    pub read_only: bool,
    pub runtime_modified: bool,
    pub analysis_only: bool,
    pub execution_enabled: bool,
    pub health_mode: String,
    pub repair_enabled: bool,
    pub backfill_enabled: bool,
    pub runtime_mutation_enabled: bool,
    pub selected_symbol: String,
    pub summary: ToxicSignalHealthSummary,
    pub health_bucket: String,
    pub issues: Vec<ToxicSignalHealthIssue>,
    pub operator_notes: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ToxicSignalHealthStatusResponse {
    pub read_only: bool,
    pub runtime_modified: bool,
    pub analysis_only: bool,
    pub execution_enabled: bool,
    pub health_mode: String,
    pub repair_enabled: bool,
    pub backfill_enabled: bool,
    pub runtime_mutation_enabled: bool,
    pub enabled: bool,
    pub status: String,
    pub selected_symbol: String,
    pub health_bucket: String,
    pub total_signals: usize,
    pub issue_count: usize,
    pub safety_boundary: Vec<String>,
}
