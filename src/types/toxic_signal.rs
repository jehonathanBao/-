use serde::{Deserialize, Serialize};

use super::toxic_flow::ToxicConfidence;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ToxicSignalType {
    ShortBiasToxicFlow,
    LongBiasToxicFlow,
    TrapRisk,
    BullTrapRisk,
    BearTrapRisk,
    SqueezeRiskUpside,
    SqueezeRiskDownside,
    AbsorptionReversalCandidate,
    LiquiditySweepReversalCandidate,
    NoTradeChopRisk,
    SpoofingCandidate,
    LayeringCandidate,
    IcebergCandidate,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum ToxicSignalDirection {
    ShortBias,
    LongBias,
    TrapRisk,
    Neutral,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum ToxicChaseRisk {
    Low,
    Medium,
    High,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ToxicSupportingEvidence {
    pub source: String,
    pub signal_id: String,
    pub signal_type: String,
    pub contribution_score: u8,
    pub summary: String,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ScoreBreakdown {
    pub toxicity_score: f64,
    pub confidence: f64,
    pub data_quality: f64,
    pub markout_evidence: f64,
    pub liquidity_impact: f64,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SignalEvidence {
    pub venue: String,
    pub symbol: String,
    pub window_ms: i64,
    pub observed_start_ms: i64,
    pub observed_end_ms: i64,
    pub add_qty: f64,
    pub cancel_qty: f64,
    pub fill_qty: f64,
    pub cancel_to_trade_ratio: Option<f64>,
    pub depth_before: Option<f64>,
    pub depth_after: Option<f64>,
    pub depth_impact: Option<f64>,
    pub price_impact_bps: Option<f64>,
    pub markout_1s_bps: Option<f64>,
    pub markout_5s_bps: Option<f64>,
    pub markout_30s_bps: Option<f64>,
    pub raw_evidence_links: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ToxicSignal {
    pub signal_id: String,
    pub symbol: String,
    pub ts_ms: u64,
    pub signal_type: ToxicSignalType,
    pub direction: ToxicSignalDirection,
    pub toxicity_score: u8,
    pub confidence: ToxicConfidence,
    pub primary_reason: String,
    pub reason: Vec<String>,
    pub supporting_evidence: Vec<ToxicSupportingEvidence>,
    pub invalidation_price: Option<f64>,
    pub suggested_stop_distance_usd: Option<f64>,
    pub chase_risk: ToxicChaseRisk,
    pub no_trade_reasons: Vec<String>,
    pub linked_active_trade_signal_ids: Vec<String>,
    pub linked_liquidation_signal_ids: Vec<String>,
    pub linked_wall_lifecycle_signal_ids: Vec<String>,
    pub linked_wall_interpretation_signal_ids: Vec<String>,
    pub linked_structural_signal_ids: Vec<String>,
    pub read_only: bool,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub detector_version: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub score_breakdown: Option<ScoreBreakdown>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub evidence: Option<SignalEvidence>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub data_quality: Option<f64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub dedupe_key: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub resolution_status: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ToxicSignalRecentResponse {
    pub read_only: bool,
    pub runtime_modified: bool,
    pub analysis_only: bool,
    pub execution_enabled: bool,
    pub mode: String,
    pub selected_symbol: String,
    pub status: String,
    pub warnings: Vec<String>,
    pub no_trade_reasons: Vec<String>,
    pub signals: Vec<ToxicSignal>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ToxicSignalStatusResponse {
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
