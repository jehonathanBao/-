use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum LiqHuntSignalLevel {
    None,
    Watch,
    Likely,
    Active,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum LiqHuntDirection {
    ShortSqueeze,
    LongSqueeze,
    None,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct LiqHuntResult {
    pub symbol: String,
    pub ts: i64,
    pub level: LiqHuntSignalLevel,
    pub direction: LiqHuntDirection,
    pub score: f64,
    pub toxic_volume_btc: Option<f64>,
    pub toxic_severity: Option<String>,
    pub toxic_direction: Option<String>,
    pub vpin: Option<f64>,
    pub vpin_spike: bool,
    pub vpin_high: bool,
    pub vpin_extreme: bool,
    pub sweep_detected: bool,
    pub local_liquidity_drain: bool,
    pub spread_widened: bool,
    pub liq_cluster_nearby: bool,
    pub possible_liq_hunt_setup: bool,
    pub nearest_cluster_side: Option<String>,
    pub nearest_cluster_distance_bps: Option<f64>,
    pub nearest_cluster_notional_usd: Option<f64>,
    pub price_move_toward_cluster_bps: Option<f64>,
    pub price_distance_closing: bool,
    pub reason_codes: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct LiqHuntState {
    pub symbol: String,
    pub updated_at: i64,
    pub result: LiqHuntResult,
    pub recent_results: Vec<LiqHuntResult>,
}

impl LiqHuntSignalLevel {
    pub fn rank(self) -> u8 {
        match self {
            LiqHuntSignalLevel::None => 0,
            LiqHuntSignalLevel::Watch => 1,
            LiqHuntSignalLevel::Likely => 2,
            LiqHuntSignalLevel::Active => 3,
        }
    }
}

pub fn empty_liq_hunt_state(now_ts: i64) -> LiqHuntState {
    LiqHuntState {
        symbol: "BTC-PERP".to_string(),
        updated_at: now_ts,
        result: empty_liq_hunt_result(now_ts),
        recent_results: Vec::new(),
    }
}

pub fn empty_liq_hunt_result(now_ts: i64) -> LiqHuntResult {
    LiqHuntResult {
        symbol: "BTC-PERP".to_string(),
        ts: now_ts,
        level: LiqHuntSignalLevel::None,
        direction: LiqHuntDirection::None,
        score: 0.0,
        toxic_volume_btc: None,
        toxic_severity: None,
        toxic_direction: None,
        vpin: None,
        vpin_spike: false,
        vpin_high: false,
        vpin_extreme: false,
        sweep_detected: false,
        local_liquidity_drain: false,
        spread_widened: false,
        liq_cluster_nearby: false,
        possible_liq_hunt_setup: false,
        nearest_cluster_side: None,
        nearest_cluster_distance_bps: None,
        nearest_cluster_notional_usd: None,
        price_move_toward_cluster_bps: None,
        price_distance_closing: false,
        reason_codes: Vec::new(),
    }
}
