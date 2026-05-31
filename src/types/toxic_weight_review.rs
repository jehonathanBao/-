use serde::{Deserialize, Serialize};

use crate::types::toxic_weight_recommendation::ToxicWeightRecommendationKind;

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ToxicWeightReviewItem {
    pub symbol: String,
    pub signal_type: String,
    pub sample_count: usize,
    pub aligned_ratio: f64,
    pub adverse_ratio: f64,
    pub neutral_ratio: f64,
    pub best_window: Option<String>,
    pub worst_window: Option<String>,
    pub recommended_action: ToxicWeightRecommendationKind,
    pub confidence: String,
    pub evidence_summary: Vec<String>,
    pub reason_codes: Vec<String>,
    pub governance_notes: Vec<String>,
    pub manual_review_required: bool,
    pub export_only: bool,
    pub runtime_weight_modified: bool,
    pub config_modified: bool,
    pub runtime_modified: bool,
    pub auto_apply_enabled: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ToxicWeightReviewSymbolSummary {
    pub symbol: String,
    pub total_items: usize,
    pub manual_review_required_count: usize,
    pub keep_count: usize,
    pub upgrade_candidate_count: usize,
    pub downgrade_candidate_count: usize,
    pub no_trade_only_count: usize,
    pub disable_candidate_count: usize,
    pub insufficient_data_count: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ToxicWeightReviewSummaryResponse {
    pub read_only: bool,
    pub analysis_only: bool,
    pub execution_enabled: bool,
    pub manual_review_required: bool,
    pub export_only: bool,
    pub runtime_modified: bool,
    pub runtime_weight_modified: bool,
    pub config_modified: bool,
    pub auto_apply_enabled: bool,
    pub mode: String,
    pub selected_symbol: String,
    pub status: String,
    pub warnings: Vec<String>,
    pub total_items: usize,
    pub manual_review_required_count: usize,
    pub keep_count: usize,
    pub upgrade_candidate_count: usize,
    pub downgrade_candidate_count: usize,
    pub no_trade_only_count: usize,
    pub disable_candidate_count: usize,
    pub insufficient_data_count: usize,
    pub governance_notes: Vec<String>,
    pub review_items: Vec<ToxicWeightReviewItem>,
    pub by_symbol: Vec<ToxicWeightReviewSymbolSummary>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ToxicWeightReviewStatusResponse {
    pub read_only: bool,
    pub analysis_only: bool,
    pub execution_enabled: bool,
    pub manual_review_required: bool,
    pub export_only: bool,
    pub runtime_modified: bool,
    pub runtime_weight_modified: bool,
    pub config_modified: bool,
    pub auto_apply_enabled: bool,
    pub enabled: bool,
    pub mode: String,
    pub selected_symbol: String,
    pub status: String,
    pub total_items: usize,
    pub manual_review_required_count: usize,
    pub safety_boundary: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ToxicWeightReviewExportResponse {
    pub read_only: bool,
    pub analysis_only: bool,
    pub execution_enabled: bool,
    pub manual_review_required: bool,
    pub export_only: bool,
    pub runtime_modified: bool,
    pub runtime_weight_modified: bool,
    pub config_modified: bool,
    pub auto_apply_enabled: bool,
    pub mode: String,
    pub selected_symbol: String,
    pub status: String,
    pub total_items: usize,
    pub recommendation_summary: ToxicWeightReviewSymbolSummary,
    pub manual_review_checklist: Vec<String>,
    pub governance_notes: Vec<String>,
    pub governance_notes_markdown: String,
    pub do_not_apply_conditions: Vec<String>,
    pub rollback_notes: Vec<String>,
    pub evidence_sources: Vec<String>,
    pub review_items: Vec<ToxicWeightReviewItem>,
    pub markdown_report: String,
}
