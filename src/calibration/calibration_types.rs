use serde::{Deserialize, Serialize};

use crate::types::toxic::ToxicEvent;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum OutcomeLabel {
    Hit,
    FalsePositive,
    Neutral,
    Unknown,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct EventOutcome {
    pub event: ToxicEvent,
    pub current_mid: Option<f64>,
    pub forward_1s_bps: Option<f64>,
    pub forward_5s_bps: Option<f64>,
    pub forward_15s_bps: Option<f64>,
    pub forward_60s_bps: Option<f64>,
    pub primary_horizon_ms: Option<u64>,
    pub primary_move_bps: Option<f64>,
    pub label: OutcomeLabel,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CalibrationRunSummary {
    pub group: String,
    pub label: String,
    pub toxic_threshold_btc: f64,
    pub min_toxic_ratio: f64,
    pub vpin_bucket_size_btc: f64,
    pub vpin_lookback_buckets: usize,
    pub vpin_spike_zscore: f64,
    pub liq_hunt_likely_score: f64,
    pub liq_hunt_active_score: f64,
    pub event_count: usize,
    pub hit_count: usize,
    pub false_positive_count: usize,
    pub neutral_count: usize,
    pub unknown_count: usize,
    pub hit_rate: f64,
    pub false_positive_rate: f64,
    pub max_toxic_volume_btc: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ReasonCodeStat {
    pub reason_code: String,
    pub total_count: usize,
    pub hit_count: usize,
    pub false_positive_count: usize,
    pub neutral_count: usize,
    pub unknown_count: usize,
    pub hit_rate: f64,
    pub false_positive_rate: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CalibrationRecommendation {
    pub title: String,
    pub detail: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CalibrationReport {
    pub input_path: String,
    pub generated_at: i64,
    pub baseline: CalibrationRunSummary,
    pub event_outcomes: Vec<EventOutcome>,
    pub threshold_comparison: Vec<CalibrationRunSummary>,
    pub toxic_ratio_comparison: Vec<CalibrationRunSummary>,
    pub vpin_parameter_comparison: Vec<CalibrationRunSummary>,
    pub liq_hunt_score_comparison: Vec<CalibrationRunSummary>,
    pub reason_code_stats: Vec<ReasonCodeStat>,
    pub top_false_positives: Vec<EventOutcome>,
    pub top_hits: Vec<EventOutcome>,
    pub recommendations: Vec<CalibrationRecommendation>,
}

#[derive(Debug, Clone)]
pub struct CalibrationScenario {
    pub group: &'static str,
    pub label: String,
    pub toxic_threshold_btc: f64,
    pub min_toxic_ratio: f64,
    pub vpin_bucket_size_btc: f64,
    pub vpin_lookback_buckets: usize,
    pub vpin_spike_zscore: f64,
    pub liq_hunt_likely_score: f64,
    pub liq_hunt_active_score: f64,
}
