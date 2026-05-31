use serde::{Deserialize, Serialize};

use super::toxic_flow::ToxicConfidence;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum OrderbookWallSide {
    Bid,
    Ask,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum OrderbookWallEventType {
    SupportWallAppeared,
    ResistanceWallAppeared,
    WallUpdated,
    WallMovedUp,
    WallMovedDown,
    WallTouched,
    WallPartiallyFilled,
    WallConsumed,
    WallRemoved,
    FakeWallCandidate,
    AbsorptionCandidate,
    LiquidityInducementCandidate,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum OrderbookWallCandidateType {
    FakeSupportWall,
    FakeResistanceWall,
    SupportAbsorption,
    ResistanceAbsorption,
    LiquidityPullCandidate,
    LiquidityInducementCandidate,
    WallDeltaConfluence,
    WallLiquidationConfluence,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum OrderbookWallInterpretationType {
    SpoofAskWall,
    SpoofBidWall,
    PersistentAskWall,
    PersistentBidWall,
    AskAbsorption,
    BidAbsorption,
    LiquidityPullAbove,
    LiquidityPullBelow,
    SupportWallFailure,
    ResistanceWallFailure,
    LiquidityInducementAbove,
    LiquidityInducementBelow,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TrackedOrderbookWall {
    pub wall_id: String,
    pub symbol: String,
    pub side: OrderbookWallSide,
    pub price: f64,
    pub notional: f64,
    pub quantity: f64,
    pub distance_bps: f64,
    pub first_seen_ms: u64,
    pub last_seen_ms: u64,
    pub updates: usize,
    pub touches: usize,
    pub status: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct OrderbookWallLifecycleEvent {
    pub event_id: String,
    pub wall_id: String,
    pub symbol: String,
    pub event_type: OrderbookWallEventType,
    pub side: OrderbookWallSide,
    pub price: f64,
    pub notional: f64,
    pub distance_bps: f64,
    pub observed_at_ms: u64,
    pub reason: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct OrderbookWallToxicityCandidate {
    pub candidate_id: String,
    pub symbol: String,
    pub candidate_type: OrderbookWallCandidateType,
    pub side: OrderbookWallSide,
    pub price: f64,
    pub score: f64,
    pub confidence: ToxicConfidence,
    pub reasons: Vec<String>,
    pub confluence: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct OrderbookWallLifecycleState {
    pub read_only: bool,
    pub runtime_modified: bool,
    pub analysis_mode: String,
    pub symbol: String,
    pub generated_at_ms: u64,
    pub status: String,
    pub tracked_walls: Vec<TrackedOrderbookWall>,
    pub recent_events: Vec<OrderbookWallLifecycleEvent>,
    pub warnings: Vec<String>,
    pub no_trade_reasons: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct OrderbookWallLifecycleReport {
    pub read_only: bool,
    pub runtime_modified: bool,
    pub analysis_mode: String,
    pub symbol: String,
    pub generated_at_ms: u64,
    pub status: String,
    pub tracked_walls: Vec<TrackedOrderbookWall>,
    pub recent_events: Vec<OrderbookWallLifecycleEvent>,
    pub toxicity_candidates: Vec<OrderbookWallToxicityCandidate>,
    pub warnings: Vec<String>,
    pub no_trade_reasons: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct OrderbookWallLifecycleStatusResponse {
    pub read_only: bool,
    pub runtime_modified: bool,
    pub analysis_mode: String,
    pub enabled: bool,
    pub selected_symbol: String,
    pub status: String,
    pub tracked_wall_count: usize,
    pub recent_event_count: usize,
    pub candidate_count: usize,
    pub last_event_at_ms: Option<u64>,
    pub safety_boundary: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct OrderbookWallInterpretationSignal {
    pub signal_id: String,
    pub symbol: String,
    pub ts_ms: u64,
    pub wall_id: String,
    pub signal_type: OrderbookWallInterpretationType,
    pub side: OrderbookWallSide,
    pub wall_price: f64,
    pub wall_notional_usd: f64,
    pub distance_to_mid_bps: f64,
    pub persistence_ms: u64,
    pub touch_count: u32,
    pub consumed_ratio: f64,
    pub cancel_ratio: f64,
    pub moved_count: u32,
    pub aggressive_volume_against_wall: Option<f64>,
    pub post_touch_markout_bps: Option<f64>,
    pub spoof_score: u8,
    pub absorption_score: u8,
    pub inducement_score: u8,
    pub toxicity_score: u8,
    pub confidence: ToxicConfidence,
    pub reason: Vec<String>,
    pub read_only: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct OrderbookWallInterpretationReport {
    pub read_only: bool,
    pub runtime_modified: bool,
    pub analysis_mode: String,
    pub selected_symbol: String,
    pub generated_at_ms: u64,
    pub status: String,
    pub signals: Vec<OrderbookWallInterpretationSignal>,
    pub warnings: Vec<String>,
    pub no_trade_reasons: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct OrderbookWallInterpretationStatusResponse {
    pub read_only: bool,
    pub runtime_modified: bool,
    pub mode: String,
    pub enabled: bool,
    pub signal_count: usize,
    pub last_signal_at_ms: Option<u64>,
    pub safety_boundary: Vec<String>,
}
