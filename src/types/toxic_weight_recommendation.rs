use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ToxicWeightRecommendationKind {
    Keep,
    SlightUpgradeCandidate,
    SlightDowngradeCandidate,
    DowngradeCandidate,
    NoTradeOnlyCandidate,
    DisableCandidate,
    InsufficientData,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ToxicWeightRecommendationItem {
    pub symbol: String,
    pub signal_type: String,
    pub sample_count: usize,
    pub aligned_ratio: f64,
    pub adverse_ratio: f64,
    pub neutral_ratio: f64,
    pub best_window: Option<String>,
    pub worst_window: Option<String>,
    pub recommendation: ToxicWeightRecommendationKind,
    pub current_weight_hint: String,
    pub suggested_weight_hint: String,
    pub confidence: String,
    pub reason_codes: Vec<String>,
    pub evidence: Vec<String>,
    pub manual_review_required: bool,
    pub runtime_weight_modified: bool,
    pub config_modified: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ToxicWeightRecommendationSignalTypeSummary {
    pub signal_type: String,
    pub sample_count: usize,
    pub aligned_ratio: f64,
    pub adverse_ratio: f64,
    pub neutral_ratio: f64,
    pub best_window: Option<String>,
    pub worst_window: Option<String>,
    pub recommendation: ToxicWeightRecommendationKind,
    pub confidence: String,
    pub reason_codes: Vec<String>,
    pub manual_review_required: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ToxicWeightRecommendationSymbolSummary {
    pub symbol: String,
    pub total_recommendations: usize,
    pub keep_count: usize,
    pub slight_upgrade_candidate_count: usize,
    pub slight_downgrade_candidate_count: usize,
    pub downgrade_candidate_count: usize,
    pub no_trade_only_candidate_count: usize,
    pub disable_candidate_count: usize,
    pub insufficient_data_count: usize,
    pub manual_review_required_count: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ToxicWeightRecommendationReviewFlagSummary {
    pub review_flag: String,
    pub count: usize,
    pub severity: String,
    pub manual_review_required: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ToxicWeightRecommendationSummaryResponse {
    pub read_only: bool,
    pub analysis_only: bool,
    pub execution_enabled: bool,
    pub manual_review_required: bool,
    pub runtime_modified: bool,
    pub runtime_weight_modified: bool,
    pub config_modified: bool,
    pub mode: String,
    pub selected_symbol: String,
    pub status: String,
    pub warnings: Vec<String>,
    pub total_recommendations: usize,
    pub keep_count: usize,
    pub slight_upgrade_candidate_count: usize,
    pub slight_downgrade_candidate_count: usize,
    pub downgrade_candidate_count: usize,
    pub no_trade_only_candidate_count: usize,
    pub disable_candidate_count: usize,
    pub insufficient_data_count: usize,
    pub recommendations: Vec<ToxicWeightRecommendationItem>,
    pub by_signal_type: Vec<ToxicWeightRecommendationSignalTypeSummary>,
    pub by_symbol: Vec<ToxicWeightRecommendationSymbolSummary>,
    pub review_flags: Vec<ToxicWeightRecommendationReviewFlagSummary>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ToxicWeightRecommendationStatusResponse {
    pub read_only: bool,
    pub analysis_only: bool,
    pub execution_enabled: bool,
    pub manual_review_required: bool,
    pub runtime_modified: bool,
    pub runtime_weight_modified: bool,
    pub config_modified: bool,
    pub enabled: bool,
    pub mode: String,
    pub selected_symbol: String,
    pub status: String,
    pub total_recommendations: usize,
    pub manual_review_required_count: usize,
    pub safety_boundary: Vec<String>,
}
