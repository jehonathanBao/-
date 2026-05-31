use serde::{Deserialize, Serialize};

use super::toxic_flow::{ToxicConfidence, ToxicSide};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum WhaleFlowCandidateType {
    AggressiveBuy,
    AggressiveSell,
    Absorption,
    LiquidationSweep,
    Trap,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct WhaleFlowVenueCoverage {
    pub configured_venues: usize,
    pub enabled_venues: usize,
    pub connected_venues: usize,
    pub active_trade_venues: usize,
    pub active_book_venues: usize,
    pub venues_with_recent_trades: Vec<String>,
    pub venues_with_recent_books: Vec<String>,
    pub venues_missing_trades: Vec<String>,
    pub venues_missing_books: Vec<String>,
    pub min_venue_confluence_required: usize,
    pub venue_confluence_satisfied: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct WhaleFlowBaselineQuality {
    pub relative_volume_multiple: Option<f64>,
    pub baseline_source: String,
    pub baseline_window_ms: Option<u64>,
    pub fallback_used: bool,
    pub insufficient_history: bool,
    pub operator_warning: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct WhaleFlowCandidateDiagnostics {
    pub data_quality: String,
    pub why_candidate: Vec<String>,
    pub missing_inputs: Vec<String>,
    pub degradation_reasons: Vec<String>,
    pub confidence_modifiers: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct WhaleFlowDataQualitySummary {
    pub status: String,
    pub venue_coverage_status: String,
    pub baseline_status: String,
    pub latest_trade_available: bool,
    pub latest_book_available: bool,
    pub operator_warning: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct WhaleFlowThresholds {
    pub one_second_btc: f64,
    pub five_second_btc: f64,
    pub fifteen_second_btc: f64,
    pub sixty_second_btc: f64,
    pub direction_ratio_min: f64,
    pub relative_volume_multiple_min: f64,
    pub min_venue_confirmations: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct WhaleFlowCandidate {
    pub candidate_id: String,
    pub symbol: String,
    pub ts_ms: u64,
    pub window: String,
    pub window_ms: u64,
    pub volume_btc: f64,
    pub gross_volume_btc: f64,
    pub direction: ToxicSide,
    pub direction_bias: f64,
    pub historical_volume_ratio: Option<f64>,
    pub historical_baseline_window_ms: Option<u64>,
    pub price_impact_bps: Option<f64>,
    pub depth_drop_ratio: Option<f64>,
    pub same_direction_venues: usize,
    pub candidate_type: WhaleFlowCandidateType,
    pub toxicity_score: u8,
    pub confidence: ToxicConfidence,
    pub primary_reason: String,
    pub reason: Vec<String>,
    pub linked_active_trade_signal_ids: Vec<String>,
    pub linked_liquidation_signal_ids: Vec<String>,
    pub linked_wall_candidate_ids: Vec<String>,
    pub linked_wall_interpretation_signal_ids: Vec<String>,
    pub linked_structural_signal_ids: Vec<String>,
    pub linked_fusion_signal_ids: Vec<String>,
    pub diagnostics: WhaleFlowCandidateDiagnostics,
    pub read_only: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct WhaleFlowRecentResponse {
    pub read_only: bool,
    pub runtime_modified: bool,
    pub analysis_only: bool,
    pub execution_enabled: bool,
    pub mode: String,
    pub selected_symbol: String,
    pub status: String,
    pub history_baseline_mode: String,
    pub lagged_events: u64,
    pub dropped_events: u64,
    pub flow_windows_populated: bool,
    pub connected_venues: usize,
    pub data_quality: WhaleFlowDataQualitySummary,
    pub venue_coverage: WhaleFlowVenueCoverage,
    pub baseline_quality: WhaleFlowBaselineQuality,
    pub thresholds: WhaleFlowThresholds,
    pub warnings: Vec<String>,
    pub no_trade_reasons: Vec<String>,
    pub no_candidate_reasons: Vec<String>,
    pub degradation_warnings: Vec<String>,
    pub candidates: Vec<WhaleFlowCandidate>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct WhaleFlowStatusResponse {
    pub read_only: bool,
    pub runtime_modified: bool,
    pub analysis_only: bool,
    pub execution_enabled: bool,
    pub enabled: bool,
    pub mode: String,
    pub selected_symbol: String,
    pub status: String,
    pub candidate_count: usize,
    pub last_candidate_at_ms: Option<u64>,
    pub lagged_events: u64,
    pub dropped_events: u64,
    pub flow_windows_populated: bool,
    pub connected_venues: usize,
    pub data_quality: WhaleFlowDataQualitySummary,
    pub venue_coverage: WhaleFlowVenueCoverage,
    pub baseline_quality: WhaleFlowBaselineQuality,
    pub thresholds: WhaleFlowThresholds,
    pub safety_boundary: Vec<String>,
}
