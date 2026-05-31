use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ToxicSignalAlertPreviewFilter {
    pub symbol: String,
    pub view_only: bool,
    pub persistent_watchlist_enabled: bool,
    pub runtime_monitor_modified: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ToxicSignalAlertPreviewGate {
    pub dedup_window_ms: i64,
    pub min_severity: String,
    pub require_cross_venue: bool,
    pub require_markout: bool,
    pub require_liquidity_drain: bool,
    pub telegram_enabled: bool,
    pub notification_sent: bool,
    pub execution_triggered: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ToxicSignalAlertPreviewSummary {
    pub total_signals: usize,
    pub notify_candidates: usize,
    pub review_candidates: usize,
    pub suppressed_signals: usize,
    pub no_trade_only_signals: usize,
    pub governance_hold_signals: usize,
    pub not_enough_data_signals: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ToxicSignalAlertPreviewBucket {
    pub key: String,
    pub label: String,
    pub total_signals: usize,
    pub notify_candidates: usize,
    pub review_candidates: usize,
    pub suppressed_signals: usize,
    pub no_trade_only_signals: usize,
    pub not_enough_data_signals: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ToxicSignalAlertPreviewItem {
    pub signal_id: String,
    pub symbol: String,
    pub signal_kind: String,
    pub direction_bias: String,
    pub severity: String,
    pub confidence: f64,
    pub preview_status: String,
    pub would_notify_if_enabled: bool,
    pub no_trade_only: bool,
    pub quality_bucket: String,
    pub latest_governance_decision: String,
    pub markout_readiness: String,
    pub suppression_reasons: Vec<String>,
    pub review_reasons: Vec<String>,
    pub preview_message: String,
    pub notification_sent: bool,
    pub execution_triggered: bool,
    pub read_only: bool,
    pub analysis_only: bool,
    pub execution_enabled: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ToxicSignalAlertPreviewResponse {
    pub read_only: bool,
    pub runtime_modified: bool,
    pub analysis_only: bool,
    pub execution_enabled: bool,
    pub manual_review_required: bool,
    pub notification_sent: bool,
    pub execution_triggered: bool,
    pub preview_only: bool,
    pub runtime_weight_modified: bool,
    pub config_modified: bool,
    pub mode: String,
    pub status: String,
    pub selected_symbol: String,
    pub filter: ToxicSignalAlertPreviewFilter,
    pub gate: ToxicSignalAlertPreviewGate,
    pub summary: ToxicSignalAlertPreviewSummary,
    pub by_symbol: Vec<ToxicSignalAlertPreviewBucket>,
    pub by_signal_kind: Vec<ToxicSignalAlertPreviewBucket>,
    pub items: Vec<ToxicSignalAlertPreviewItem>,
    pub operator_notes: Vec<String>,
    pub markdown: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ToxicSignalAlertPreviewStatusResponse {
    pub read_only: bool,
    pub runtime_modified: bool,
    pub analysis_only: bool,
    pub execution_enabled: bool,
    pub manual_review_required: bool,
    pub notification_sent: bool,
    pub execution_triggered: bool,
    pub preview_only: bool,
    pub runtime_weight_modified: bool,
    pub config_modified: bool,
    pub enabled: bool,
    pub mode: String,
    pub status: String,
    pub selected_symbol: String,
    pub filter: ToxicSignalAlertPreviewFilter,
    pub gate: ToxicSignalAlertPreviewGate,
    pub total_signals: usize,
    pub notify_candidate_count: usize,
    pub suppressed_count: usize,
    pub safety_boundary: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ToxicSignalAlertPreviewExplainResponse {
    pub read_only: bool,
    pub runtime_modified: bool,
    pub analysis_only: bool,
    pub execution_enabled: bool,
    pub notification_sent: bool,
    pub execution_triggered: bool,
    pub found: bool,
    pub signal_id: String,
    pub symbol: String,
    pub alert_decision: String,
    pub decision_reasons: Vec<String>,
    pub suppression_reasons: Vec<String>,
    pub missing_inputs: Vec<String>,
    pub operator_note: String,
    pub reason: Option<String>,
}
