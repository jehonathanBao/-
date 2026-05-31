use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ToxicSignalGroupOperatorAction {
    ReviewGroupedSignal,
    WatchGroupOnly,
    NeedsMoreData,
    NoTradeWarningGroup,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ToxicSignalGroup {
    pub group_id: String,
    pub symbol: String,
    pub signal_kind: String,
    pub direction_bias: String,
    pub count: usize,
    pub first_seen_at_ms: u64,
    pub last_seen_at_ms: u64,
    pub cooldown_window_ms: u64,
    pub max_severity: String,
    pub avg_confidence: f64,
    pub representative_signal_id: String,
    pub member_signal_ids: Vec<String>,
    pub operator_action: ToxicSignalGroupOperatorAction,
    pub suppression_hint: String,
    pub original_signals_preserved: bool,
    #[serde(skip_serializing, skip_deserializing, default)]
    pub representative_confidence: f64,
    pub read_only: bool,
    pub runtime_modified: bool,
    pub analysis_only: bool,
    pub execution_enabled: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ToxicSignalGroupRecentResponse {
    pub read_only: bool,
    pub runtime_modified: bool,
    pub analysis_only: bool,
    pub execution_enabled: bool,
    pub manual_review_required: bool,
    pub runtime_weight_modified: bool,
    pub config_modified: bool,
    pub mode: String,
    pub selected_symbol: String,
    pub status: String,
    pub cooldown_window_ms: u64,
    pub warnings: Vec<String>,
    pub groups: Vec<ToxicSignalGroup>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ToxicSignalGroupStatusResponse {
    pub read_only: bool,
    pub runtime_modified: bool,
    pub analysis_only: bool,
    pub execution_enabled: bool,
    pub manual_review_required: bool,
    pub runtime_weight_modified: bool,
    pub config_modified: bool,
    pub enabled: bool,
    pub mode: String,
    pub selected_symbol: String,
    pub status: String,
    pub cooldown_window_ms: u64,
    pub group_count: usize,
    pub last_group_at_ms: Option<u64>,
    pub safety_boundary: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ToxicSignalGroupDetailResponse {
    pub read_only: bool,
    pub runtime_modified: bool,
    pub analysis_only: bool,
    pub execution_enabled: bool,
    pub manual_review_required: bool,
    pub runtime_weight_modified: bool,
    pub config_modified: bool,
    pub mode: String,
    pub selected_symbol: String,
    pub available: bool,
    pub reason: Option<String>,
    pub group: Option<ToxicSignalGroup>,
}
