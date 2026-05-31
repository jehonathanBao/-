use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};

use crate::types::{
    liquidation::{LiquidationClusterSide, LiquidationMetrics},
    market::Venue,
    sweep::LiquidityThinnessResult,
};

pub type ToxicWindowMs = u64;

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum ToxicSeverity {
    Normal,
    Watch,
    Warning,
    Alert,
    Extreme,
}

impl ToxicSeverity {
    pub fn label(self) -> &'static str {
        match self {
            ToxicSeverity::Normal => "NORMAL",
            ToxicSeverity::Watch => "WATCH",
            ToxicSeverity::Warning => "WARNING",
            ToxicSeverity::Alert => "ALERT",
            ToxicSeverity::Extreme => "EXTREME",
        }
    }

    pub fn from_toxic_volume(toxic_volume_btc: f64, threshold_btc: f64) -> Self {
        let threshold = threshold_btc.max(1.0);
        if toxic_volume_btc >= threshold * 2.0 {
            ToxicSeverity::Extreme
        } else if toxic_volume_btc >= threshold {
            ToxicSeverity::Alert
        } else if toxic_volume_btc >= threshold * 0.6 {
            ToxicSeverity::Warning
        } else if toxic_volume_btc >= threshold * 0.3 {
            ToxicSeverity::Watch
        } else {
            ToxicSeverity::Normal
        }
    }

    pub fn is_at_least(self, other: ToxicSeverity) -> bool {
        self >= other
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum ToxicDirection {
    Buy,
    Sell,
    Neutral,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct VenueToxicBreakdown {
    pub aggressive_buy_btc: f64,
    pub aggressive_sell_btc: f64,
    pub net_aggressive_btc: f64,
    pub toxic_buy_btc: f64,
    pub toxic_sell_btc: f64,
    pub toxic_volume_btc: f64,
    pub trade_count: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ToxicVolumeResult {
    pub symbol: String,
    pub window_ms: ToxicWindowMs,
    pub ts: i64,
    pub direction: ToxicDirection,
    pub severity: ToxicSeverity,
    pub toxic_ratio: f64,
    pub toxic_volume_btc: f64,
    pub threshold_btc: f64,
    pub alert_triggered: bool,
    pub aggressive_buy_btc: f64,
    pub aggressive_sell_btc: f64,
    pub net_aggressive_btc: f64,
    pub abs_aggressive_btc: f64,
    pub markout_1s_bps: Option<f64>,
    pub markout_5s_bps: Option<f64>,
    pub markout_confirmed: bool,
    pub sweep_detected: bool,
    pub liquidity_thin: bool,
    pub liquidity: Option<LiquidityThinnessResult>,
    pub cross_venue_confirmed: bool,
    pub vpin_enabled: bool,
    pub vpin: Option<f64>,
    pub vpin_zscore: Option<f64>,
    pub vpin_spike: bool,
    pub vpin_high: bool,
    pub vpin_extreme: bool,
    pub liquidation_enabled: bool,
    pub nearest_cluster_side: Option<LiquidationClusterSide>,
    pub cluster_distance_bps: Option<f64>,
    pub cluster_notional_usd: Option<f64>,
    pub cluster_density: Option<f64>,
    pub liq_hunt_pressure: f64,
    pub liq_cluster_nearby: bool,
    pub possible_liq_hunt_setup: bool,
    pub leader_venue: Option<Venue>,
    pub venue_breakdown: BTreeMap<String, VenueToxicBreakdown>,
    pub reason_codes: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ToxicEvent {
    pub id: String,
    pub ts: i64,
    pub symbol: String,
    pub direction: ToxicDirection,
    pub severity: ToxicSeverity,
    pub toxic_volume_btc: f64,
    pub threshold_btc: f64,
    pub window_ms: ToxicWindowMs,
    pub leader_venue: Option<Venue>,
    pub aggressive_buy_btc: f64,
    pub aggressive_sell_btc: f64,
    pub net_aggressive_btc: f64,
    pub abs_aggressive_btc: f64,
    pub markout_1s_bps: Option<f64>,
    pub markout_5s_bps: Option<f64>,
    pub sweep_detected: bool,
    pub liquidity_thin: bool,
    pub liquidity: Option<LiquidityThinnessResult>,
    pub cross_venue_confirmed: bool,
    pub vpin_enabled: bool,
    pub vpin: Option<f64>,
    pub vpin_zscore: Option<f64>,
    pub vpin_spike: bool,
    pub vpin_high: bool,
    pub vpin_extreme: bool,
    pub liquidation_enabled: bool,
    pub nearest_cluster_side: Option<LiquidationClusterSide>,
    pub cluster_distance_bps: Option<f64>,
    pub cluster_notional_usd: Option<f64>,
    pub cluster_density: Option<f64>,
    pub liq_hunt_pressure: f64,
    pub liq_cluster_nearby: bool,
    pub possible_liq_hunt_setup: bool,
    pub reason_codes: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ToxicState {
    pub symbol: String,
    pub updated_at: i64,
    pub threshold_btc: f64,
    pub windows_ms: Vec<ToxicWindowMs>,
    pub results: BTreeMap<String, ToxicVolumeResult>,
    pub latest_event: Option<ToxicEvent>,
    pub recent_events: Vec<ToxicEvent>,
    pub quality: ToxicQuality,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ToxicQuality {
    pub has_flow: bool,
    pub has_markout: bool,
    pub has_sweep: bool,
    pub has_liquidation: bool,
    pub liquidation: Option<LiquidationMetrics>,
    pub active_venues: Vec<Venue>,
    pub stale_venues: Vec<Venue>,
}

pub fn empty_venue_toxic_breakdown() -> BTreeMap<String, VenueToxicBreakdown> {
    Venue::ALL
        .into_iter()
        .map(|venue| (venue.as_key().to_string(), VenueToxicBreakdown::default()))
        .collect()
}
