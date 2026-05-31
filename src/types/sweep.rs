use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};

use crate::types::market::Venue;

pub type SweepWindowMs = u64;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum SweepDirection {
    Buy,
    Sell,
    None,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct VenueSweepBreakdown {
    pub swept_buy_btc: f64,
    pub swept_sell_btc: f64,
    pub net_swept_btc: f64,
    pub trade_count: u64,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct LiquidityThinnessResult {
    pub symbol: String,
    pub window_ms: SweepWindowMs,
    pub bid_depth_start_btc: Option<f64>,
    pub bid_depth_end_btc: Option<f64>,
    pub ask_depth_start_btc: Option<f64>,
    pub ask_depth_end_btc: Option<f64>,
    pub bid_depth_drop_ratio: Option<f64>,
    pub ask_depth_drop_ratio: Option<f64>,
    pub spread_start_bps: Option<f64>,
    pub spread_end_bps: Option<f64>,
    pub spread_widen_ratio: Option<f64>,
    pub bid_thin: bool,
    pub ask_thin: bool,
    pub spread_widened: bool,
    pub reason_codes: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SweepResult {
    pub symbol: String,
    pub window_ms: SweepWindowMs,
    pub direction: SweepDirection,
    pub sweep_detected: bool,
    pub swept_volume_btc: f64,
    pub swept_volume_usd: f64,
    pub aggressive_buy_btc: f64,
    pub aggressive_sell_btc: f64,
    pub net_aggressive_btc: f64,
    pub trade_count: u64,
    pub same_direction_trade_count: u64,
    pub price_start: Option<f64>,
    pub price_end: Option<f64>,
    pub price_impact_bps: Option<f64>,
    pub leader_venue: Option<Venue>,
    pub venue_breakdown: BTreeMap<String, VenueSweepBreakdown>,
    pub liquidity: Option<LiquidityThinnessResult>,
    pub reason_codes: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SweepQuality {
    pub has_trades: bool,
    pub has_books: bool,
    pub active_venues: Vec<Venue>,
    pub stale_venues: Vec<Venue>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SweepState {
    pub symbol: String,
    pub updated_at: i64,
    pub windows_ms: Vec<SweepWindowMs>,
    pub results: BTreeMap<String, SweepResult>,
    pub quality: SweepQuality,
}

pub fn empty_venue_sweep_breakdown() -> BTreeMap<String, VenueSweepBreakdown> {
    Venue::ALL
        .into_iter()
        .map(|venue| (venue.as_key().to_string(), VenueSweepBreakdown::default()))
        .collect()
}
