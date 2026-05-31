use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ToxicSignalReportFilter {
    pub symbol: String,
    pub view_only: bool,
    pub persistent_watchlist_enabled: bool,
    pub runtime_monitor_modified: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ToxicSignalReportSummary {
    pub total_signals: usize,
    pub grouped_signals: usize,
    pub high_severity_signals: usize,
    pub no_trade_only_candidates: usize,
    pub downgrade_candidates: usize,
    pub not_enough_data_signals: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ToxicSignalReportMarkoutSummary {
    pub aligned: usize,
    pub adverse: usize,
    pub neutral: usize,
    pub not_enough_data: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ToxicSignalReportBucket {
    pub key: String,
    pub label: String,
    pub signal_count: usize,
    pub high_severity_signals: usize,
    pub no_trade_only_candidates: usize,
    pub downgrade_candidates: usize,
    pub not_enough_data_signals: usize,
    pub avg_confidence: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ToxicSignalReportTopGroup {
    pub group_id: String,
    pub symbol: String,
    pub signal_kind: String,
    pub direction_bias: String,
    pub count: usize,
    pub first_seen_at_ms: u64,
    pub last_seen_at_ms: u64,
    pub max_severity: String,
    pub avg_confidence: f64,
    pub representative_signal_id: String,
    pub original_signals_preserved: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ToxicSignalReportDailyResponse {
    pub read_only: bool,
    pub runtime_modified: bool,
    pub analysis_only: bool,
    pub execution_enabled: bool,
    pub manual_review_required: bool,
    pub runtime_weight_modified: bool,
    pub config_modified: bool,
    pub report_type: String,
    pub mode: String,
    pub date: String,
    pub filter: ToxicSignalReportFilter,
    pub summary: ToxicSignalReportSummary,
    pub markout_summary: ToxicSignalReportMarkoutSummary,
    pub by_symbol: Vec<ToxicSignalReportBucket>,
    pub by_signal_kind: Vec<ToxicSignalReportBucket>,
    pub top_groups: Vec<ToxicSignalReportTopGroup>,
    pub operator_notes: Vec<String>,
    pub markdown: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ToxicSignalReportStatusResponse {
    pub read_only: bool,
    pub runtime_modified: bool,
    pub analysis_only: bool,
    pub execution_enabled: bool,
    pub manual_review_required: bool,
    pub runtime_weight_modified: bool,
    pub config_modified: bool,
    pub enabled: bool,
    pub report_type: String,
    pub mode: String,
    pub selected_symbol: String,
    pub status: String,
    pub date: String,
    pub filter: ToxicSignalReportFilter,
    pub total_signals: usize,
    pub group_count: usize,
    pub last_signal_at_ms: Option<u64>,
    pub safety_boundary: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ToxicSignalRollingDigestSummary {
    pub total_signals: usize,
    pub aligned: usize,
    pub adverse: usize,
    pub neutral: usize,
    pub not_enough_data: usize,
    pub top_symbols: Vec<String>,
    pub top_signal_kinds: Vec<String>,
    pub no_trade_only_candidates: usize,
    pub downgrade_candidates: usize,
    pub notify_candidates: usize,
    pub review_candidates: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ToxicSignalReportRollingResponse {
    pub read_only: bool,
    pub runtime_modified: bool,
    pub analysis_only: bool,
    pub execution_enabled: bool,
    pub manual_review_required: bool,
    pub runtime_weight_modified: bool,
    pub config_modified: bool,
    pub report_type: String,
    pub mode: String,
    pub window: String,
    pub retention_mode: String,
    pub durable_storage_enabled: bool,
    pub database_write_enabled: bool,
    pub filter: ToxicSignalReportFilter,
    pub summary: ToxicSignalRollingDigestSummary,
    pub operator_notes: Vec<String>,
    pub markdown: String,
}
