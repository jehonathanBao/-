use serde::{Deserialize, Serialize};

use super::{toxic::ToxicDirection, toxic_flow::ToxicConfidence};

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum LiquidationClusterSide {
    ShortAbove,
    LongBelow,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct EstimatedLiquidationCluster {
    pub side: LiquidationClusterSide,
    pub price: f64,
    pub distance_bps: f64,
    pub cluster_notional_usd: f64,
    pub cluster_density: f64,
    pub touched_snapshots: usize,
    pub first_seen_ts: i64,
    pub last_seen_ts: i64,
    pub reason_codes: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct LiquidationMetrics {
    pub enabled: bool,
    pub lookback_ms: i64,
    pub cluster_band_bps: f64,
    pub proximity_threshold_bps: f64,
    pub current_mid: Option<f64>,
    pub nearest_short_liq_cluster_above: Option<EstimatedLiquidationCluster>,
    pub nearest_long_liq_cluster_below: Option<EstimatedLiquidationCluster>,
    pub dominant_direction: ToxicDirection,
    pub nearest_cluster_side: Option<LiquidationClusterSide>,
    pub distance_bps: Option<f64>,
    pub cluster_notional_usd: Option<f64>,
    pub cluster_density: Option<f64>,
    pub liq_hunt_pressure: f64,
    pub liq_cluster_nearby: bool,
    pub possible_liq_hunt_setup: bool,
    pub reason_codes: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct LiquidationState {
    pub symbol: String,
    pub updated_at: i64,
    pub metrics: LiquidationMetrics,
    pub recent_clusters: Vec<EstimatedLiquidationCluster>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum LiquidationToxicSignalType {
    LiquidationClusterNearby,
    UpsideLiquidationMagnet,
    DownsideLiquidationMagnet,
    LongSqueezeRisk,
    ShortSqueezeRisk,
    LiquidationCascadeCandidate,
    LiquidationDeltaConfluence,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum LiquidationToxicDirection {
    Upside,
    Downside,
    Neutral,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct LiquidationToxicSignal {
    pub signal_id: String,
    pub symbol: String,
    pub ts_ms: u64,
    pub signal_type: LiquidationToxicSignalType,
    pub direction: LiquidationToxicDirection,
    pub current_price: f64,
    pub cluster_price: f64,
    pub distance_usd: f64,
    pub distance_bps: f64,
    pub estimated_liquidation_notional: f64,
    pub cluster_density_score: u8,
    pub magnet_score: u8,
    pub cascade_score: u8,
    pub linked_active_trade_signal_ids: Vec<String>,
    pub toxicity_score: u8,
    pub confidence: ToxicConfidence,
    pub reason: Vec<String>,
    pub read_only: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct LiquidationToxicityRecentResponse {
    pub read_only: bool,
    pub runtime_modified: bool,
    pub mode: String,
    pub selected_symbol: String,
    pub warnings: Vec<String>,
    pub no_trade_reasons: Vec<String>,
    pub signals: Vec<LiquidationToxicSignal>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct LiquidationToxicityStatusResponse {
    pub read_only: bool,
    pub runtime_modified: bool,
    pub enabled: bool,
    pub mode: String,
    pub signal_count: usize,
    pub last_signal_at_ms: Option<u64>,
    pub safety_boundary: Vec<String>,
}

pub fn empty_liquidation_state(now_ts: i64) -> LiquidationState {
    LiquidationState {
        symbol: "BTC-PERP".to_string(),
        updated_at: now_ts,
        metrics: LiquidationMetrics {
            enabled: false,
            lookback_ms: 0,
            cluster_band_bps: 0.0,
            proximity_threshold_bps: 0.0,
            current_mid: None,
            nearest_short_liq_cluster_above: None,
            nearest_long_liq_cluster_below: None,
            dominant_direction: ToxicDirection::Neutral,
            nearest_cluster_side: None,
            distance_bps: None,
            cluster_notional_usd: None,
            cluster_density: None,
            liq_hunt_pressure: 0.0,
            liq_cluster_nearby: false,
            possible_liq_hunt_setup: false,
            reason_codes: vec!["liquidation_disabled".to_string()],
        },
        recent_clusters: Vec::new(),
    }
}
