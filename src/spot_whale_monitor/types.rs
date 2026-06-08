use std::collections::BTreeMap;

#[derive(
    Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, serde::Serialize, serde::Deserialize,
)]
#[serde(rename_all = "lowercase")]
pub enum SpotExchange {
    Binance,
    Coinbase,
}

impl SpotExchange {
    pub fn as_key(self) -> &'static str {
        match self {
            Self::Binance => "binance",
            Self::Coinbase => "coinbase",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum SpotTradeSide {
    Buy,
    Sell,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SpotTrade {
    pub ts: i64,
    pub exchange: SpotExchange,
    pub symbol: String,
    pub market: String,
    pub price: f64,
    pub qty_base: f64,
    pub notional_usd: f64,
    pub side: SpotTradeSide,
    pub trade_id: Option<String>,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SpotExchangeStatus {
    pub connected: bool,
    pub status: String,
    pub last_trade_at: Option<i64>,
    pub latency_ms: Option<i64>,
    pub reconnect_count: u64,
    pub last_error: Option<String>,
}

impl SpotExchangeStatus {
    pub fn disabled() -> Self {
        Self {
            connected: false,
            status: "disabled".to_string(),
            last_trade_at: None,
            latency_ms: None,
            reconnect_count: 0,
            last_error: None,
        }
    }

    pub fn disconnected() -> Self {
        Self {
            connected: false,
            status: "disconnected".to_string(),
            last_trade_at: None,
            latency_ms: None,
            reconnect_count: 0,
            last_error: None,
        }
    }
}

#[derive(Debug, Clone, Default, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SpotExchangeContribution {
    pub exchange: String,
    pub buy_volume_base: f64,
    pub sell_volume_base: f64,
    pub total_volume_base: f64,
    pub buy_notional_usd: f64,
    pub sell_notional_usd: f64,
    pub total_notional_usd: f64,
    pub net_volume_base: f64,
    pub dominance: f64,
    pub trade_count: u64,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SpotWhaleWindowStats {
    pub symbol: String,
    pub window_sec: u64,
    pub ts: i64,
    pub buy_volume_base: f64,
    pub sell_volume_base: f64,
    pub total_volume_base: f64,
    pub net_volume_base: f64,
    pub total_notional_usd: f64,
    pub dominance: f64,
    pub price_move_pct: Option<f64>,
    pub coinbase_premium_pct: Option<f64>,
    pub exchange_count: usize,
    pub main_exchange: Option<String>,
    pub exchanges: Vec<SpotExchangeContribution>,
    pub dynamic_multiple: Option<f64>,
    pub multi_exchange_confirmed: bool,
    pub data_quality: u8,
    pub startup_age_ms: Option<i64>,
}

#[derive(
    Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, serde::Serialize, serde::Deserialize,
)]
#[serde(rename_all = "lowercase")]
pub enum SpotWhaleSeverity {
    Calm,
    Medium,
    High,
    Critical,
    S,
}

impl SpotWhaleSeverity {
    pub fn rank(self) -> u8 {
        match self {
            Self::Calm => 0,
            Self::Medium => 1,
            Self::High => 2,
            Self::Critical => 3,
            Self::S => 4,
        }
    }
}

#[derive(
    Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, serde::Serialize, serde::Deserialize,
)]
#[serde(rename_all = "snake_case")]
pub enum SpotWhaleSignalType {
    SpotAggressiveBuy,
    SpotAggressiveSell,
    SpotDownsideAbsorption,
    SpotUpsideSuppression,
    SpotExchangeDislocation,
}

#[derive(
    Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, serde::Serialize, serde::Deserialize,
)]
#[serde(rename_all = "lowercase")]
pub enum SpotWhaleDirection {
    Buy,
    Sell,
    Absorption,
    Suppression,
    Dislocation,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SpotWhaleSignal {
    pub id: String,
    pub ts: i64,
    pub symbol: String,
    pub window_sec: u64,
    pub signal_type: SpotWhaleSignalType,
    pub direction: SpotWhaleDirection,
    pub severity: SpotWhaleSeverity,
    pub score: u8,
    pub total_volume_base: f64,
    pub net_volume_base: f64,
    pub total_notional_usd: f64,
    pub dominance: f64,
    pub price_move_pct: Option<f64>,
    pub coinbase_premium_pct: Option<f64>,
    pub main_exchange: Option<String>,
    pub exchanges: Vec<SpotExchangeContribution>,
    pub dynamic_multiple: Option<f64>,
    pub multi_exchange_confirmed: bool,
    pub data_quality: u8,
    pub discord_eligible: bool,
    pub discord_sent: bool,
    pub discord_sent_at: Option<i64>,
    pub discord_reason: String,
    pub final_result: String,
    pub read_only: bool,
    pub analysis_only: bool,
    pub execution_enabled: bool,
}

#[derive(Debug, Clone, Default, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SpotWhaleTrend60s {
    pub buy_volume_base: f64,
    pub sell_volume_base: f64,
    pub total_volume_base: f64,
    pub net_volume_base: f64,
    pub dominance: f64,
    pub buy_ratio: f64,
    pub sell_ratio: f64,
    pub updated_at_ms: Option<i64>,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SpotWhaleSummary {
    pub status: String,
    pub health_status: String,
    pub health_reason: String,
    pub direction: String,
    pub latest_direction: String,
    pub latest_severity: SpotWhaleSeverity,
    pub latest_signal_at: Option<i64>,
    pub last_discord_sent_at: Option<i64>,
    pub updated_at_ms: Option<i64>,
    pub signal_count: usize,
    pub read_only: bool,
    pub enabled: bool,
    pub dry_run: bool,
    pub symbol: String,
    pub trend60s: SpotWhaleTrend60s,
    pub exchanges: BTreeMap<String, SpotExchangeStatus>,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SpotWhaleLatestResponse {
    pub summary: SpotWhaleSummary,
    pub items: Vec<SpotWhaleSignal>,
    pub limit: usize,
}

#[derive(Debug, Clone, Copy)]
pub struct SpotWhaleThresholds {
    pub high_base: f64,
    pub critical_base: f64,
    pub s_base: f64,
    pub high_notional_usd: f64,
    pub critical_notional_usd: f64,
    pub s_notional_usd: f64,
}
