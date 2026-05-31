use serde::{Deserialize, Serialize};

use super::whale_flow_signal::WhaleFlowThresholds;

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct WhaleFlowCalibrationSampleStatus {
    pub total_candidates: usize,
    pub linked_markout_samples: usize,
    pub resolved_markout_evidence_count: usize,
    pub unresolved_markout_count: usize,
    pub not_enough_data_rate: f64,
    pub min_samples_required: usize,
    pub min_resolved_evidence_required: usize,
    pub max_not_enough_data_rate_for_tuning: f64,
    pub enough_data: bool,
    pub blocked_reason: Option<String>,
    pub blocked_reasons: Vec<String>,
    pub retention_mode: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct WhaleFlowCalibrationEvidenceSource {
    pub mode: String,
    pub uses_current_snapshot_only: bool,
    pub current_snapshot_fallback_used: bool,
    pub history_signals_available: usize,
    pub whale_candidates_evaluated: usize,
    pub resolved_markout_evidence_count: usize,
    pub unresolved_markout_count: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct WhaleFlowCalibrationOutcomeLinkage {
    pub linked_signal_id_matches: usize,
    pub fallback_matches: usize,
    pub no_outcome_linkage_count: usize,
    pub fallback_used: bool,
    pub operator_warnings: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct WhaleFlowCalibrationThresholdPerformanceItem {
    pub threshold: f64,
    pub candidate_count: usize,
    pub aligned_rate: f64,
    pub adverse_rate: f64,
    pub neutral_rate: f64,
    pub not_enough_data_rate: f64,
    pub verdict: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct WhaleFlowCalibrationThresholdPerformanceSummary {
    pub one_second_btc: WhaleFlowCalibrationThresholdPerformanceItem,
    pub five_second_btc: WhaleFlowCalibrationThresholdPerformanceItem,
    pub fifteen_second_btc: WhaleFlowCalibrationThresholdPerformanceItem,
    pub sixty_second_btc: WhaleFlowCalibrationThresholdPerformanceItem,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct WhaleFlowCalibrationClassificationQualityItem {
    pub classification: String,
    pub sample_count: usize,
    pub aligned_rate: f64,
    pub adverse_rate: f64,
    pub neutral_rate: f64,
    pub not_enough_data_rate: f64,
    pub quality_bucket: String,
    pub manual_tuning_note: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct WhaleFlowCalibrationVenueConfluenceItem {
    pub venue_count: usize,
    pub sample_count: usize,
    pub aligned_rate: f64,
    pub adverse_rate: f64,
    pub neutral_rate: f64,
    pub not_enough_data_rate: f64,
    pub verdict: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct WhaleFlowCalibrationBaselineSourceItem {
    pub baseline_source: String,
    pub sample_count: usize,
    pub aligned_rate: f64,
    pub adverse_rate: f64,
    pub neutral_rate: f64,
    pub not_enough_data_rate: f64,
    pub quality_bucket: String,
    pub manual_tuning_note: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct WhaleFlowCalibrationManualTuningNote {
    pub target: String,
    pub current_value: f64,
    pub suggested_action: String,
    pub reason: String,
    pub auto_applied: bool,
    pub config_modified: bool,
    pub manual_review_required: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct WhaleFlowCalibrationReportResponse {
    pub read_only: bool,
    pub analysis_only: bool,
    pub execution_enabled: bool,
    pub runtime_modified: bool,
    pub manual_review_required: bool,
    pub threshold_modified: bool,
    pub config_modified: bool,
    pub runtime_threshold_modified: bool,
    pub auto_apply_enabled: bool,
    pub selected_symbol: String,
    pub status: String,
    pub evidence_source: WhaleFlowCalibrationEvidenceSource,
    pub outcome_linkage: WhaleFlowCalibrationOutcomeLinkage,
    pub sample_status: WhaleFlowCalibrationSampleStatus,
    pub current_thresholds: WhaleFlowThresholds,
    pub threshold_performance: WhaleFlowCalibrationThresholdPerformanceSummary,
    pub by_classification: Vec<WhaleFlowCalibrationClassificationQualityItem>,
    pub venue_confluence: Vec<WhaleFlowCalibrationVenueConfluenceItem>,
    pub baseline_source_quality: Vec<WhaleFlowCalibrationBaselineSourceItem>,
    pub manual_tuning_notes: Vec<WhaleFlowCalibrationManualTuningNote>,
    pub warnings: Vec<String>,
    pub no_candidate_reasons: Vec<String>,
    pub markdown: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct WhaleFlowCalibrationStatusResponse {
    pub read_only: bool,
    pub analysis_only: bool,
    pub execution_enabled: bool,
    pub runtime_modified: bool,
    pub manual_review_required: bool,
    pub threshold_modified: bool,
    pub config_modified: bool,
    pub runtime_threshold_modified: bool,
    pub auto_apply_enabled: bool,
    pub enabled: bool,
    pub selected_symbol: String,
    pub status: String,
    pub total_candidates: usize,
    pub linked_markout_samples: usize,
    pub resolved_markout_evidence_count: usize,
    pub min_samples_required: usize,
    pub min_resolved_evidence_required: usize,
    pub enough_data: bool,
    pub current_thresholds: WhaleFlowThresholds,
    pub safety_boundary: Vec<String>,
}
