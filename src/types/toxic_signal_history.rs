use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ToxicSignalHistorySignalItem {
    pub signal_id: String,
    pub symbol: String,
    pub signal_kind: String,
    pub direction_bias: String,
    pub severity: String,
    pub confidence: f64,
    pub created_at_ms: u64,
    pub markout_one_minute: String,
    pub markout_five_minute: String,
    pub markout_fifteen_minute: String,
    pub markout_one_hour: String,
    pub quality_bucket: String,
    pub recommendation_action: String,
    pub no_trade_only: bool,
    pub source: String,
    pub history_recorded_at_ms: u64,
    pub operator_action: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ToxicSignalHistoryGroupItem {
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
    pub member_signal_ids: Vec<String>,
    pub source: String,
    pub history_recorded_at_ms: u64,
    pub operator_action: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ToxicSignalHistoryAlertItem {
    pub signal_id: String,
    pub symbol: String,
    pub signal_kind: String,
    pub preview_status: String,
    pub would_notify_if_enabled: bool,
    pub no_trade_only: bool,
    pub markout_readiness: String,
    pub source: String,
    pub history_recorded_at_ms: u64,
    pub notification_sent: bool,
    pub execution_triggered: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ToxicSignalHistoryReportItem {
    pub report_type: String,
    pub date: String,
    pub symbol: String,
    pub total_signals: usize,
    pub grouped_signals: usize,
    pub high_severity_signals: usize,
    pub no_trade_only_candidates: usize,
    pub downgrade_candidates: usize,
    pub not_enough_data_signals: usize,
    pub source: String,
    pub history_recorded_at_ms: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ToxicSignalHistoryStatusResponse {
    pub read_only: bool,
    pub runtime_modified: bool,
    pub analysis_only: bool,
    pub execution_enabled: bool,
    pub retention_mode: String,
    pub durable_storage_enabled: bool,
    pub database_write_enabled: bool,
    pub max_signals: usize,
    pub max_groups: usize,
    pub max_alerts: usize,
    pub max_reports: usize,
    pub current_signals: usize,
    pub current_groups: usize,
    pub current_alerts: usize,
    pub current_reports: usize,
    pub safety_boundary: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ToxicSignalHistoryRecentResponse {
    pub read_only: bool,
    pub runtime_modified: bool,
    pub analysis_only: bool,
    pub execution_enabled: bool,
    pub retention_mode: String,
    pub durable_storage_enabled: bool,
    pub database_write_enabled: bool,
    pub selected_symbol: String,
    pub items: Vec<ToxicSignalHistorySignalItem>,
    pub group_items: Vec<ToxicSignalHistoryGroupItem>,
    pub operator_notes: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ToxicSignalHistorySignalLookupResponse {
    pub read_only: bool,
    pub runtime_modified: bool,
    pub analysis_only: bool,
    pub execution_enabled: bool,
    pub found: bool,
    pub signal: Option<ToxicSignalHistorySignalItem>,
    pub source: String,
    pub retention_mode: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ToxicSignalHistoryAlertRecentResponse {
    pub read_only: bool,
    pub runtime_modified: bool,
    pub analysis_only: bool,
    pub execution_enabled: bool,
    pub retention_mode: String,
    pub durable_storage_enabled: bool,
    pub database_write_enabled: bool,
    pub selected_symbol: String,
    pub items: Vec<ToxicSignalHistoryAlertItem>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ToxicSignalHistoryReportRecentResponse {
    pub read_only: bool,
    pub runtime_modified: bool,
    pub analysis_only: bool,
    pub execution_enabled: bool,
    pub retention_mode: String,
    pub durable_storage_enabled: bool,
    pub database_write_enabled: bool,
    pub selected_symbol: String,
    pub items: Vec<ToxicSignalHistoryReportItem>,
}
