use serde::{Deserialize, Serialize};

use super::{
    liquidation::LiquidationToxicSignal,
    orderbook_wall::{OrderbookWallInterpretationSignal, OrderbookWallLifecycleEvent},
    structural_toxicity::StructuralToxicSignal,
    toxic_flow::ActiveTradeToxicSignal,
    toxic_signal::ToxicSignal,
};

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ToxicReplaySignalSummary {
    pub signal_id: String,
    pub signal_kind: String,
    pub confidence: f64,
    pub severity: String,
    pub created_at: u64,
    pub primary_reason: String,
    pub read_only: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ToxicReplayRecentResponse {
    pub read_only: bool,
    pub runtime_modified: bool,
    pub analysis_only: bool,
    pub execution_enabled: bool,
    pub mode: String,
    pub selected_symbol: String,
    pub status: String,
    pub warnings: Vec<String>,
    pub no_trade_reasons: Vec<String>,
    pub signals: Vec<ToxicReplaySignalSummary>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ToxicReplayStatusResponse {
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
pub struct ToxicReplayEvidenceBreakdown {
    pub active_trade: Vec<ActiveTradeToxicSignal>,
    pub liquidation: Vec<LiquidationToxicSignal>,
    pub orderbook: Vec<OrderbookWallLifecycleEvent>,
    pub wall_interpretation: Vec<OrderbookWallInterpretationSignal>,
    pub structural: Vec<StructuralToxicSignal>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ToxicReplayPriceContext {
    pub current_price_reference: Option<f64>,
    pub invalidation_price: Option<f64>,
    pub suggested_stop_distance_usd: Option<f64>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ToxicReplayBookContext {
    pub status: String,
    pub tracked_wall_count: usize,
    pub recent_event_count: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ToxicReplayFlowContext {
    pub status: String,
    pub signal_count: usize,
    pub side_bias: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ToxicReplayLiquidationContext {
    pub status: String,
    pub signal_count: usize,
    pub dominant_bias: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ToxicReplayStructureContext {
    pub status: String,
    pub signal_count: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ToxicReplayContext {
    pub price: ToxicReplayPriceContext,
    pub book: ToxicReplayBookContext,
    pub flow: ToxicReplayFlowContext,
    pub liquidation: ToxicReplayLiquidationContext,
    pub structure: ToxicReplayStructureContext,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ToxicReplayOperatorNarrative {
    pub why_signal_fired: Vec<String>,
    pub supporting_evidence: Vec<String>,
    pub conflicting_evidence: Vec<String>,
    pub why_not_entry_signal: Vec<String>,
    pub risk_warnings: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ToxicReplayReferenceLevels {
    pub invalidation_price: Option<f64>,
    pub suggested_stop_distance_usd: Option<f64>,
    pub wording: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ToxicReplayMarkoutPreview {
    pub available: bool,
    pub note: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ToxicReplayDetail {
    pub read_only: bool,
    pub runtime_modified: bool,
    pub analysis_only: bool,
    pub execution_enabled: bool,
    pub mode: String,
    pub symbol: String,
    pub signal_id: String,
    pub signal_kind: String,
    pub confidence: f64,
    pub severity: String,
    pub created_at: u64,
    pub source_signal: ToxicSignal,
    pub evidence_breakdown: ToxicReplayEvidenceBreakdown,
    pub context: ToxicReplayContext,
    pub operator_narrative: ToxicReplayOperatorNarrative,
    pub reference_levels: ToxicReplayReferenceLevels,
    pub markout_preview: ToxicReplayMarkoutPreview,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ToxicReplayDetailResponse {
    pub read_only: bool,
    pub runtime_modified: bool,
    pub analysis_only: bool,
    pub execution_enabled: bool,
    pub mode: String,
    pub symbol: String,
    pub available: bool,
    pub reason: Option<String>,
    pub replay: Option<ToxicReplayDetail>,
}
