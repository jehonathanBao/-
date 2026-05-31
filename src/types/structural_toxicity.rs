use serde::{Deserialize, Serialize};

use super::toxic_flow::ToxicConfidence;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum StructuralToxicSignalType {
    LiquiditySweepHigh,
    LiquiditySweepLow,
    FailedBreakout,
    FailedBreakdown,
    StopHuntUpside,
    StopHuntDownside,
    SupportTrap,
    ResistanceTrap,
    BullTrapCandidate,
    BearTrapCandidate,
    KeyLevelAbsorption,
    KeyLevelSpoofConfluence,
    LiquidationWallConfluence,
    DeltaStructureDivergence,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum StructuralToxicDirection {
    UpsideTrap,
    DownsideTrap,
    BullishReversalCandidate,
    BearishReversalCandidate,
    Neutral,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum StructuralLevelType {
    SessionHigh,
    SessionLow,
    RecentSwingHigh,
    RecentSwingLow,
    RoundNumber,
    PreviousHigh,
    PreviousLow,
    LiquidationClusterLevel,
    WallPriceLevel,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct StructuralToxicSignal {
    pub signal_id: String,
    pub symbol: String,
    pub ts_ms: u64,
    pub signal_type: StructuralToxicSignalType,
    pub direction: StructuralToxicDirection,
    pub level_type: StructuralLevelType,
    pub level_price: f64,
    pub current_price: f64,
    pub sweep_distance_usd: Option<f64>,
    pub sweep_distance_bps: Option<f64>,
    pub reclaim_or_reject: bool,
    pub time_outside_level_ms: Option<u64>,
    pub linked_active_trade_signal_ids: Vec<String>,
    pub linked_liquidation_signal_ids: Vec<String>,
    pub linked_wall_signal_ids: Vec<String>,
    pub linked_wall_interpretation_signal_ids: Vec<String>,
    pub toxicity_score: u8,
    pub confidence: ToxicConfidence,
    pub reason: Vec<String>,
    pub read_only: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct StructuralToxicityRecentResponse {
    pub read_only: bool,
    pub runtime_modified: bool,
    pub mode: String,
    pub selected_symbol: String,
    pub status: String,
    pub warnings: Vec<String>,
    pub no_trade_reasons: Vec<String>,
    pub signals: Vec<StructuralToxicSignal>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct StructuralToxicityStatusResponse {
    pub read_only: bool,
    pub runtime_modified: bool,
    pub enabled: bool,
    pub mode: String,
    pub signal_count: usize,
    pub last_signal_at_ms: Option<u64>,
    pub safety_boundary: Vec<String>,
}
