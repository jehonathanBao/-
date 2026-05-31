use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ToxicMarkoutOutcome {
    Aligned,
    Adverse,
    Neutral,
    NotEnoughData,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ToxicMarkoutWindow {
    pub label: String,
    pub horizon_ms: u64,
    pub outcome: ToxicMarkoutOutcome,
    pub markout_bps: Option<f64>,
    pub price_at_signal: Option<f64>,
    pub price_at_horizon: Option<f64>,
    pub note: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ToxicMarkoutSignal {
    pub signal_id: String,
    pub symbol: String,
    pub signal_kind: String,
    pub direction: String,
    pub toxicity_score: u8,
    pub confidence: String,
    pub created_at_ms: u64,
    pub overall_outcome: ToxicMarkoutOutcome,
    pub aligned_windows: usize,
    pub adverse_windows: usize,
    pub neutral_windows: usize,
    pub missing_windows: usize,
    pub windows: Vec<ToxicMarkoutWindow>,
    pub no_trade_reasons: Vec<String>,
    pub read_only: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ToxicMarkoutRecentResponse {
    pub read_only: bool,
    pub runtime_modified: bool,
    pub analysis_only: bool,
    pub execution_enabled: bool,
    pub mode: String,
    pub selected_symbol: String,
    pub status: String,
    pub warnings: Vec<String>,
    pub signals: Vec<ToxicMarkoutSignal>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ToxicMarkoutStatusResponse {
    pub read_only: bool,
    pub runtime_modified: bool,
    pub analysis_only: bool,
    pub execution_enabled: bool,
    pub enabled: bool,
    pub mode: String,
    pub signal_count: usize,
    pub last_signal_at_ms: Option<u64>,
    pub safety_boundary: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ToxicMarkoutDetailResponse {
    pub read_only: bool,
    pub runtime_modified: bool,
    pub analysis_only: bool,
    pub execution_enabled: bool,
    pub mode: String,
    pub symbol: String,
    pub available: bool,
    pub reason: Option<String>,
    pub signal: Option<ToxicMarkoutSignal>,
}
