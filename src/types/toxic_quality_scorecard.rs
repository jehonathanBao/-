use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ToxicQualityScorecardBucket {
    pub key: String,
    pub label: String,
    pub total_evaluations: usize,
    pub aligned_count: usize,
    pub adverse_count: usize,
    pub neutral_count: usize,
    pub not_enough_data_count: usize,
    pub aligned_ratio: f64,
    pub adverse_ratio: f64,
    pub neutral_ratio: f64,
    pub not_enough_data_ratio: f64,
    pub downgrade_candidate: bool,
    pub no_trade_candidate: bool,
    pub top_no_trade_reasons: Vec<String>,
    pub symbols: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ToxicQualityScorecardCandidate {
    pub key: String,
    pub label: String,
    pub reason: String,
    pub total_evaluations: usize,
    pub aligned_ratio: f64,
    pub adverse_ratio: f64,
    pub neutral_ratio: f64,
    pub not_enough_data_ratio: f64,
    pub top_no_trade_reasons: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ToxicQualityScorecardSymbolSummary {
    pub symbol: String,
    pub total_evaluations: usize,
    pub aligned_ratio: f64,
    pub adverse_ratio: f64,
    pub neutral_ratio: f64,
    pub not_enough_data_ratio: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ToxicQualityScorecardSummaryResponse {
    pub read_only: bool,
    pub runtime_modified: bool,
    pub analysis_only: bool,
    pub execution_enabled: bool,
    pub mode: String,
    pub selected_symbol: String,
    pub status: String,
    pub warnings: Vec<String>,
    pub total_evaluations: usize,
    pub aligned_ratio: f64,
    pub adverse_ratio: f64,
    pub neutral_ratio: f64,
    pub not_enough_data_ratio: f64,
    pub by_signal_type: Vec<ToxicQualityScorecardBucket>,
    pub by_window: Vec<ToxicQualityScorecardBucket>,
    pub by_symbol: Vec<ToxicQualityScorecardSymbolSummary>,
    pub downgrade_candidates: Vec<ToxicQualityScorecardCandidate>,
    pub no_trade_candidates: Vec<ToxicQualityScorecardCandidate>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ToxicQualityScorecardStatusResponse {
    pub read_only: bool,
    pub runtime_modified: bool,
    pub analysis_only: bool,
    pub execution_enabled: bool,
    pub enabled: bool,
    pub mode: String,
    pub selected_symbol: String,
    pub status: String,
    pub total_evaluations: usize,
    pub signal_type_count: usize,
    pub window_count: usize,
    pub downgrade_candidate_count: usize,
    pub no_trade_candidate_count: usize,
    pub safety_boundary: Vec<String>,
}
