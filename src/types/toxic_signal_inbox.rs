use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ToxicSignalInboxOperatorAction {
    WatchSignalOnly,
    ReviewEvidence,
    ReviewMarkout,
    ReviewQuality,
    NoTradeWarning,
    NeedsMoreData,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ToxicSignalInboxFusionSummary {
    pub available: bool,
    pub summary: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ToxicSignalInboxReplaySummary {
    pub available: bool,
    pub evidence_count: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ToxicSignalInboxMarkoutSummary {
    pub available: bool,
    pub one_minute: String,
    pub five_minute: String,
    pub fifteen_minute: String,
    pub one_hour: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ToxicSignalInboxQualitySummary {
    pub available: bool,
    pub quality_bucket: String,
    pub aligned_ratio: f64,
    pub adverse_ratio: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ToxicSignalInboxRecommendationSummary {
    pub available: bool,
    pub action: String,
    pub no_trade_only: bool,
    pub manual_review_required: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ToxicSignalInboxGovernanceSummary {
    pub ledger_available: bool,
    pub latest_decision: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ToxicSignalInboxItem {
    pub signal_id: String,
    pub symbol: String,
    pub signal_kind: String,
    pub direction_bias: String,
    pub severity: String,
    pub confidence: f64,
    pub created_at_ms: u64,
    pub fusion: ToxicSignalInboxFusionSummary,
    pub replay: ToxicSignalInboxReplaySummary,
    pub markout: ToxicSignalInboxMarkoutSummary,
    pub quality: ToxicSignalInboxQualitySummary,
    pub recommendation: ToxicSignalInboxRecommendationSummary,
    pub governance: ToxicSignalInboxGovernanceSummary,
    pub operator_action: ToxicSignalInboxOperatorAction,
    pub read_only: bool,
    pub runtime_modified: bool,
    pub analysis_only: bool,
    pub execution_enabled: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ToxicSignalInboxRecentResponse {
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
    pub warnings: Vec<String>,
    pub items: Vec<ToxicSignalInboxItem>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ToxicSignalInboxStatusResponse {
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
    pub item_count: usize,
    pub last_signal_at_ms: Option<u64>,
    pub safety_boundary: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ToxicSignalInboxDetailResponse {
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
    pub item: Option<ToxicSignalInboxItem>,
}
