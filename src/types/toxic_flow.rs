#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum ToxicConfidence {
    Low,
    Medium,
    High,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum ToxicSide {
    Buy,
    Sell,
    Neutral,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ActiveTradeToxicSignalType {
    LargeAggressiveBuy,
    LargeAggressiveSell,
    BuySweep,
    SellSweep,
    CvdSpike,
    TradeImbalance,
    OneHourDeltaBuyDominant,
    OneHourDeltaSellDominant,
    AbsorptionCandidate,
    AdverseMarkout,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ActiveTradeToxicityFeatures {
    pub trade_count: usize,
    pub buy_volume: f64,
    pub sell_volume: f64,
    pub net_aggressive_volume: f64,
    pub imbalance_ratio: f64,
    pub large_trade_count: usize,
    pub burst_score: f64,
    pub volume_spike_score: f64,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ActiveTradeToxicityReport {
    pub read_only: bool,
    pub runtime_modified: bool,
    pub symbol: String,
    pub window_ms: u64,
    pub generated_at_ms: u64,
    pub status: String,
    pub score: f64,
    pub side_bias: String,
    pub features: ActiveTradeToxicityFeatures,
    pub warnings: Vec<String>,
    pub no_trade_reasons: Vec<String>,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ActiveTradeToxicSignal {
    pub signal_id: String,
    pub symbol: String,
    pub ts_ms: u64,
    pub signal_type: ActiveTradeToxicSignalType,
    pub side: ToxicSide,
    pub timeframe: Option<String>,
    pub candle_open_ms: Option<u64>,
    pub candle_close_ms: Option<u64>,
    pub window_ms: u64,
    pub delta: Option<f64>,
    pub abs_delta: Option<f64>,
    pub threshold: Option<f64>,
    pub aggressive_volume: f64,
    pub notional_usd: f64,
    pub trade_count: u64,
    pub cvd_delta: f64,
    pub buy_volume: f64,
    pub sell_volume: f64,
    pub imbalance_ratio: f64,
    pub open: Option<f64>,
    pub high: Option<f64>,
    pub low: Option<f64>,
    pub close: Option<f64>,
    pub price_impact_bps: Option<f64>,
    pub price_change_bps: Option<f64>,
    pub upper_wick_ratio: Option<f64>,
    pub lower_wick_ratio: Option<f64>,
    pub markout_5s: Option<f64>,
    pub markout_15s: Option<f64>,
    pub markout_60s: Option<f64>,
    pub toxicity_score: u8,
    pub confidence: ToxicConfidence,
    pub reason: Vec<String>,
    pub read_only: bool,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ActiveTradeToxicityRecentResponse {
    pub read_only: bool,
    pub runtime_modified: bool,
    pub mode: String,
    pub selected_symbol: String,
    pub status: String,
    pub score: f64,
    pub side_bias: String,
    pub warnings: Vec<String>,
    pub no_trade_reasons: Vec<String>,
    pub signals: Vec<ActiveTradeToxicSignal>,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ActiveTradeToxicityStatusResponse {
    pub read_only: bool,
    pub runtime_modified: bool,
    pub enabled: bool,
    pub mode: String,
    pub signal_count: usize,
    pub last_signal_at_ms: Option<u64>,
    pub safety_boundary: Vec<String>,
}
