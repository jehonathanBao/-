use std::collections::BTreeMap;

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct VenueFlowBreakdown {
    pub aggressive_buy_btc: f64,
    pub aggressive_sell_btc: f64,
    pub aggressive_buy_usd: f64,
    pub aggressive_sell_usd: f64,
    pub net_aggressive_btc: f64,
    pub abs_aggressive_btc: f64,
    pub trade_count: u64,
    pub buy_trade_count: u64,
    pub sell_trade_count: u64,
    pub last_trade_ts: Option<i64>,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DataQuality {
    pub has_trades: bool,
    pub has_books: bool,
    pub active_venues: Vec<String>,
    pub stale_venues: Vec<String>,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct FlowWindow {
    pub symbol: String,
    pub window_ms: u64,
    pub now_ts: i64,
    pub aggressive_buy_btc: f64,
    pub aggressive_sell_btc: f64,
    pub aggressive_buy_usd: f64,
    pub aggressive_sell_usd: f64,
    pub net_aggressive_btc: f64,
    pub abs_aggressive_btc: f64,
    pub trade_count: u64,
    pub buy_trade_count: u64,
    pub sell_trade_count: u64,
    pub avg_trade_size_btc: f64,
    pub max_trade_size_btc: f64,
    pub venue_breakdown: BTreeMap<String, VenueFlowBreakdown>,
    pub mid_start: Option<f64>,
    pub mid_end: Option<f64>,
    pub price_move_bps: Option<f64>,
    pub spread_bps_median: Option<f64>,
    pub imbalance_10bps_median: Option<f64>,
    pub data_quality: DataQuality,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct FlowState {
    pub symbol: String,
    pub updated_at: i64,
    pub windows: BTreeMap<String, FlowWindow>,
}

pub fn empty_venue_breakdown() -> BTreeMap<String, VenueFlowBreakdown> {
    ["binance", "bybit", "okx"]
        .into_iter()
        .map(|venue| (venue.to_string(), empty_breakdown()))
        .collect()
}

pub fn empty_breakdown() -> VenueFlowBreakdown {
    VenueFlowBreakdown {
        aggressive_buy_btc: 0.0,
        aggressive_sell_btc: 0.0,
        aggressive_buy_usd: 0.0,
        aggressive_sell_usd: 0.0,
        net_aggressive_btc: 0.0,
        abs_aggressive_btc: 0.0,
        trade_count: 0,
        buy_trade_count: 0,
        sell_trade_count: 0,
        last_trade_ts: None,
    }
}
