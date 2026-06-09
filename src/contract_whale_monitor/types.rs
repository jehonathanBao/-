use std::collections::BTreeMap;

#[derive(
    Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, serde::Serialize, serde::Deserialize,
)]
#[serde(rename_all = "lowercase")]
pub enum ContractExchange {
    Binance,
    Okx,
    Bitfinex,
    Coinbase,
}

impl ContractExchange {
    pub fn as_key(self) -> &'static str {
        match self {
            Self::Binance => "binance",
            Self::Okx => "okx",
            Self::Bitfinex => "bitfinex",
            Self::Coinbase => "coinbase",
        }
    }
}

#[derive(
    Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, serde::Serialize, serde::Deserialize,
)]
#[serde(rename_all = "snake_case")]
pub enum ContractWhaleMarketType {
    Spot,
    Perp,
    Level2,
    Funding,
    Oi,
    Liquidation,
}

impl ContractWhaleMarketType {
    pub fn as_key(self) -> &'static str {
        match self {
            Self::Spot => "spot",
            Self::Perp => "perp",
            Self::Level2 => "level2",
            Self::Funding => "funding",
            Self::Oi => "oi",
            Self::Liquidation => "liquidation",
        }
    }

    pub fn as_env_key(self) -> &'static str {
        match self {
            Self::Spot => "SPOT",
            Self::Perp => "PERP",
            Self::Level2 => "LEVEL2",
            Self::Funding => "FUNDING",
            Self::Oi => "OI",
            Self::Liquidation => "LIQUIDATION",
        }
    }
}

impl Default for ContractWhaleMarketType {
    fn default() -> Self {
        Self::Perp
    }
}

#[derive(
    Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, serde::Serialize, serde::Deserialize,
)]
#[serde(rename_all = "snake_case")]
pub enum ContractWhaleSourceRole {
    Primary,
    Confirmation,
    Optional,
    Disabled,
}

impl ContractWhaleSourceRole {
    pub fn as_key(self) -> &'static str {
        match self {
            Self::Primary => "primary",
            Self::Confirmation => "confirmation",
            Self::Optional => "optional",
            Self::Disabled => "disabled",
        }
    }
}

impl Default for ContractWhaleSourceRole {
    fn default() -> Self {
        Self::Optional
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum ContractTradeSide {
    Buy,
    Sell,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ContractTrade {
    pub ts: i64,
    pub exchange: ContractExchange,
    pub symbol: String,
    pub market: String,
    pub price: f64,
    pub qty_btc: f64,
    pub notional_usd: f64,
    pub side: ContractTradeSide,
    pub raw_trade_count: Option<u64>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum ContractLiquidationSide {
    Long,
    Short,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ContractLiquidationOrder {
    pub ts: i64,
    pub exchange: ContractExchange,
    pub symbol: String,
    pub price: f64,
    pub qty_btc: f64,
    pub notional_usd: f64,
    pub side: ContractLiquidationSide,
}

#[derive(Debug, Clone, Default, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ContractLiquidationBucket {
    pub ts_bucket: i64,
    pub exchange: String,
    pub symbol: String,
    pub long_liq_btc: f64,
    pub short_liq_btc: f64,
    pub liq_notional_usd: f64,
    pub order_count: u64,
    pub max_single_liq_btc: f64,
    pub vwap: Option<f64>,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ContractOiSnapshot {
    pub ts: i64,
    pub exchange: ContractExchange,
    pub symbol: String,
    pub oi_btc: f64,
    pub oi_notional_usd: Option<f64>,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ContractFundingSnapshot {
    pub ts: i64,
    pub exchange: ContractExchange,
    pub symbol: String,
    pub funding_rate: f64,
}

#[derive(Debug, Clone, Default, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ContractWhaleLiquidationContext {
    pub long_liq_btc: f64,
    pub short_liq_btc: f64,
    pub total_liq_btc: f64,
    pub liq_notional_usd: f64,
    pub liq_to_volume_ratio: Option<f64>,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ContractWhaleMarketContext {
    #[serde(default)]
    pub context_expected: bool,
    #[serde(default)]
    pub ct_val_available: bool,
    #[serde(default)]
    pub oi_available: bool,
    #[serde(default)]
    pub funding_available: bool,
    #[serde(default)]
    pub oi_change_1m_btc: Option<f64>,
    #[serde(default)]
    pub oi_change_5m_btc: Option<f64>,
    #[serde(default)]
    pub oi_change_pct: Option<f64>,
    #[serde(default)]
    pub oi_bias: Option<String>,
    #[serde(default)]
    pub funding_rate: Option<f64>,
    #[serde(default)]
    pub funding_bias: Option<String>,
}

impl Default for ContractWhaleMarketContext {
    fn default() -> Self {
        Self {
            context_expected: false,
            ct_val_available: true,
            oi_available: false,
            funding_available: false,
            oi_change_1m_btc: None,
            oi_change_5m_btc: None,
            oi_change_pct: None,
            oi_bias: None,
            funding_rate: None,
            funding_bias: None,
        }
    }
}

#[derive(Debug, Clone, Default, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ContractFlowBucket {
    pub ts_bucket: i64,
    pub exchange: String,
    pub symbol: String,
    #[serde(default)]
    pub market_type: ContractWhaleMarketType,
    #[serde(default)]
    pub source_role: ContractWhaleSourceRole,
    #[serde(default)]
    pub product_id: Option<String>,
    pub buy_volume_btc: f64,
    pub sell_volume_btc: f64,
    pub buy_notional_usd: f64,
    pub sell_notional_usd: f64,
    pub trade_count: u64,
    pub max_single_trade_btc: f64,
    pub vwap: Option<f64>,
}

#[derive(Debug, Clone, Default, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ContractWhaleActiveSourceEntry {
    pub exchange: String,
    #[serde(default)]
    pub market_type: ContractWhaleMarketType,
    #[serde(default)]
    pub source_role: ContractWhaleSourceRole,
    #[serde(default)]
    pub enabled: bool,
    #[serde(default)]
    pub status: String,
    #[serde(default)]
    pub product_id: Option<String>,
}

#[derive(Debug, Clone, Default, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ContractWhaleActiveSources {
    #[serde(default)]
    pub contract: Vec<ContractWhaleActiveSourceEntry>,
    #[serde(default)]
    pub spot: Vec<ContractWhaleActiveSourceEntry>,
}

#[derive(Debug, Clone, Default, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ExchangeFlowContribution {
    pub exchange: String,
    pub buy_volume_btc: f64,
    pub sell_volume_btc: f64,
    pub total_volume_btc: f64,
    #[serde(default)]
    pub buy_share: f64,
    #[serde(default)]
    pub sell_share: f64,
    pub buy_notional_usd: f64,
    pub sell_notional_usd: f64,
    pub total_notional_usd: f64,
    pub net_volume_btc: f64,
    #[serde(default)]
    pub dominance: f64,
    #[serde(default)]
    pub net_contribution_share: f64,
    pub trade_count: u64,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ContractWhaleWindowStats {
    pub symbol: String,
    pub window_sec: u64,
    pub ts: i64,
    pub buy_volume_btc: f64,
    pub sell_volume_btc: f64,
    pub total_volume_btc: f64,
    pub net_volume_btc: f64,
    pub dominance: f64,
    pub buy_notional_usd: f64,
    pub sell_notional_usd: f64,
    pub total_notional_usd: f64,
    pub price_move_pct: Option<f64>,
    pub exchange_count: usize,
    pub main_exchange: Option<String>,
    pub exchanges: Vec<ExchangeFlowContribution>,
    #[serde(default)]
    pub dominant_venue_net_contribution_share: Option<f64>,
    pub dynamic_multiple: Option<f64>,
    pub percentile_level: Option<f64>,
    pub multi_exchange_confirmed: bool,
    pub liquidation_context: ContractWhaleLiquidationContext,
    pub market_context: ContractWhaleMarketContext,
    pub price_reversal_ratio: Option<f64>,
    pub data_quality: u8,
    pub ws_latency_ms: Option<i64>,
    pub startup_age_ms: Option<i64>,
    pub liquidation_driven: bool,
    pub price_jump_anomaly: bool,
}

#[derive(
    Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, serde::Serialize, serde::Deserialize,
)]
#[serde(rename_all = "lowercase")]
pub enum ContractWhaleSeverity {
    Calm,
    Medium,
    High,
    Critical,
    S,
}

impl ContractWhaleSeverity {
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
pub enum ContractWhaleSignalType {
    AggressiveBuy,
    AggressiveSell,
    DownsideAbsorption,
    UpsideSuppression,
}

#[derive(
    Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, serde::Serialize, serde::Deserialize,
)]
#[serde(rename_all = "lowercase")]
pub enum ContractWhaleDirection {
    Buy,
    Sell,
    Absorption,
    Suppression,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ContractWhaleSignal {
    pub id: String,
    pub ts: i64,
    pub symbol: String,
    pub window_sec: u64,
    pub signal_type: ContractWhaleSignalType,
    pub direction: ContractWhaleDirection,
    pub severity: ContractWhaleSeverity,
    pub score: u8,
    pub total_volume_btc: f64,
    pub net_volume_btc: f64,
    pub total_notional_usd: f64,
    pub dominance: f64,
    pub price_move_pct: Option<f64>,
    pub main_exchange: Option<String>,
    #[serde(default)]
    pub market_type: ContractWhaleMarketType,
    #[serde(default)]
    pub source_role: ContractWhaleSourceRole,
    pub exchanges: Vec<ExchangeFlowContribution>,
    #[serde(default)]
    pub dominant_venue_net_contribution_share: Option<f64>,
    pub dynamic_multiple: Option<f64>,
    #[serde(default)]
    pub percentile_level: Option<f64>,
    #[serde(default)]
    pub multi_exchange_confirmed: bool,
    #[serde(default)]
    pub liquidation_suspected: bool,
    #[serde(default)]
    pub liquidation_long_btc: f64,
    #[serde(default)]
    pub liquidation_short_btc: f64,
    #[serde(default)]
    pub liquidation_notional_usd: f64,
    #[serde(default)]
    pub liquidation_ratio: Option<f64>,
    #[serde(default)]
    pub price_reversal_ratio: Option<f64>,
    #[serde(default)]
    pub oi_change_1m_btc: Option<f64>,
    #[serde(default)]
    pub oi_change_5m_btc: Option<f64>,
    #[serde(default)]
    pub oi_change_pct: Option<f64>,
    #[serde(default)]
    pub oi_bias: Option<String>,
    #[serde(default)]
    pub funding_rate: Option<f64>,
    #[serde(default)]
    pub funding_bias: Option<String>,
    pub data_quality: u8,
    #[serde(default = "default_threshold_profile")]
    pub threshold_profile: String,
    #[serde(default)]
    pub active_sources: ContractWhaleActiveSources,
    pub discord_eligible: bool,
    pub discord_sent: bool,
    #[serde(default)]
    pub discord_sent_at: Option<i64>,
    pub discord_reason: String,
    pub final_result: String,
    pub read_only: bool,
    pub analysis_only: bool,
    pub execution_enabled: bool,
    #[serde(default)]
    pub merged_from: Vec<String>,
}

fn default_threshold_profile() -> String {
    "three_exchange".to_string()
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ContractWhaleSummary {
    pub status: String,
    pub health_status: String,
    pub health_reason: String,
    #[serde(default = "default_contract_summary_market_type")]
    pub market_type: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub meta: Option<ContractWhaleResponseMeta>,
    #[serde(default)]
    pub threshold_profile: String,
    #[serde(default)]
    pub active_exchange_count: usize,
    #[serde(default)]
    pub enabled_exchanges: Vec<String>,
    #[serde(default)]
    pub disabled_exchanges: Vec<String>,
    #[serde(default)]
    pub active_contract_exchanges: Vec<String>,
    pub direction: String,
    pub latest_direction: String,
    pub latest_severity: ContractWhaleSeverity,
    pub latest_signal_at: Option<i64>,
    pub latest_pushed_at_ms: Option<i64>,
    pub last_discord_sent_at: Option<i64>,
    pub updated_at_ms: i64,
    pub signal_count: usize,
    pub read_only: bool,
    pub enabled: bool,
    pub dry_run: bool,
    #[serde(default)]
    pub contract_data_quality: u8,
    #[serde(default)]
    pub spot_data_quality: u8,
    #[serde(default)]
    pub overall_data_quality: u8,
    pub warmup: bool,
    pub warmup_until_ms: Option<i64>,
    pub warmup_remaining_ms: Option<i64>,
    pub trend_60s: ContractWhaleTrend60s,
    pub exchanges: BTreeMap<String, ContractWhaleExchangeStatus>,
    #[serde(default)]
    pub platforms: BTreeMap<String, ContractWhalePlatformCapability>,
}

fn default_contract_summary_market_type() -> String {
    "perp".to_string()
}

#[derive(Debug, Clone, Default, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ContractWhaleTrend60s {
    pub buy_volume_btc: f64,
    pub sell_volume_btc: f64,
    pub total_volume_btc: f64,
    pub net_volume_btc: f64,
    pub dominance: f64,
    pub buy_ratio: f64,
    pub sell_ratio: f64,
    pub updated_at_ms: Option<i64>,
}

#[derive(Debug, Clone, Default, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ContractWhaleExchangeStatus {
    pub connected: bool,
    pub status: String,
    pub last_trade_at: Option<i64>,
    pub latency_ms: Option<i64>,
    pub reconnect_count: u64,
    #[serde(default)]
    pub platform_enabled: bool,
    #[serde(default)]
    pub contract_enabled: bool,
    #[serde(default)]
    pub enabled_markets: Vec<String>,
    #[serde(default)]
    pub market_roles: BTreeMap<String, String>,
}

#[derive(Debug, Clone, Default, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ContractWhalePlatformCapability {
    pub platform_enabled: bool,
    pub status: String,
    pub markets: BTreeMap<String, ContractWhaleMarketCapability>,
}

#[derive(Debug, Clone, Default, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ContractWhaleMarketCapability {
    pub enabled: bool,
    pub status: String,
    pub role: String,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ContractWhaleLatestResponse {
    pub summary: ContractWhaleSummary,
    pub items: Vec<ContractWhaleSignal>,
    pub filter: BTreeMap<String, String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub meta: Option<ContractWhaleResponseMeta>,
}

#[derive(Debug, Clone, Default, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ContractWhaleResponseMeta {
    pub exchange: Option<String>,
    pub market_type: Option<String>,
    pub exchange_status: Option<String>,
    pub reason: Option<String>,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ContractWhalePercentileThreshold {
    pub computed_at: i64,
    pub symbol: String,
    pub exchange: String,
    pub window_sec: u64,
    pub p99_0_btc: f64,
    pub p99_5_btc: f64,
    pub p99_9_btc: f64,
    pub sample_count: usize,
}

#[derive(Debug, Clone, Copy)]
pub struct ContractWhaleThresholds {
    pub high_btc: f64,
    pub critical_btc: f64,
    pub s_btc: f64,
}

impl ContractWhaleThresholds {
    pub fn for_window(window_sec: u64) -> Self {
        match window_sec {
            5 => Self {
                high_btc: 800.0,
                critical_btc: 1500.0,
                s_btc: 2500.0,
            },
            15 => Self {
                high_btc: 1500.0,
                critical_btc: 2800.0,
                s_btc: 4500.0,
            },
            60 => Self {
                high_btc: 3500.0,
                critical_btc: 6500.0,
                s_btc: 10000.0,
            },
            _ => Self {
                high_btc: f64::INFINITY,
                critical_btc: f64::INFINITY,
                s_btc: f64::INFINITY,
            },
        }
    }

    pub fn binance_bitfinex_for_window(window_sec: u64) -> Self {
        match window_sec {
            5 => Self {
                high_btc: 650.0,
                critical_btc: 1200.0,
                s_btc: 2000.0,
            },
            15 => Self {
                high_btc: 1200.0,
                critical_btc: 2200.0,
                s_btc: 3600.0,
            },
            60 => Self {
                high_btc: 2800.0,
                critical_btc: 5200.0,
                s_btc: 8000.0,
            },
            _ => Self {
                high_btc: f64::INFINITY,
                critical_btc: f64::INFINITY,
                s_btc: f64::INFINITY,
            },
        }
    }
}
