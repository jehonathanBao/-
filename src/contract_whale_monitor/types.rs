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
    Debug,
    Clone,
    Copy,
    Default,
    PartialEq,
    Eq,
    PartialOrd,
    Ord,
    serde::Serialize,
    serde::Deserialize,
)]
#[serde(rename_all = "snake_case")]
pub enum ContractWhaleMarketType {
    Spot,
    #[default]
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

#[derive(
    Debug,
    Clone,
    Copy,
    Default,
    PartialEq,
    Eq,
    PartialOrd,
    Ord,
    serde::Serialize,
    serde::Deserialize,
)]
#[serde(rename_all = "snake_case")]
pub enum ContractWhaleSourceRole {
    Primary,
    Confirmation,
    #[default]
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
    #[serde(default = "default_threshold_profile")]
    pub threshold_profile: String,
    #[serde(default)]
    pub threshold_profile_reason: String,
    #[serde(default)]
    pub configured_contract_sources: Vec<String>,
    #[serde(default)]
    pub eligible_contract_sources: Vec<String>,
    #[serde(default)]
    pub active_contract_sources: Vec<String>,
}

#[derive(Debug, Clone, Default, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ContractWhaleSpotConfirmationContext {
    #[serde(default)]
    pub status: String,
    #[serde(default)]
    pub confirmation_type: String,
    #[serde(default)]
    pub direction: String,
    #[serde(default)]
    pub score: u8,
    #[serde(default)]
    pub latest_signal_id: Option<String>,
    #[serde(default)]
    pub latest_signal_at: Option<i64>,
    #[serde(default)]
    pub signal_type: Option<String>,
    #[serde(default)]
    pub severity: Option<String>,
    #[serde(default)]
    pub total_volume_btc: Option<f64>,
    #[serde(default)]
    pub net_volume_btc: Option<f64>,
    #[serde(default)]
    pub dominance: Option<f64>,
    #[serde(default)]
    pub coinbase_premium_pct: Option<f64>,
    #[serde(default)]
    pub final_result: Option<String>,
}

#[derive(Debug, Clone, Default, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ContractWhaleDiscordDryRunStats {
    #[serde(default)]
    pub signals_1h: usize,
    #[serde(default)]
    pub high_1h: usize,
    #[serde(default)]
    pub critical_1h: usize,
    #[serde(default)]
    pub s_1h: usize,
    #[serde(default)]
    pub would_send_1h: usize,
    #[serde(default)]
    pub skipped_low_score_1h: usize,
    #[serde(default)]
    pub skipped_cooldown_1h: usize,
    #[serde(default)]
    pub skipped_data_quality_1h: usize,
    #[serde(default)]
    pub skipped_warmup_1h: usize,
    #[serde(default)]
    pub skipped_display_only_1h: usize,
}

#[derive(Debug, Clone, Default, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ContractWhaleMarketStructureLite {
    #[serde(default)]
    pub status: String,
    #[serde(default)]
    pub regime_type: String,
    #[serde(default)]
    pub main_force_score: u8,
    #[serde(default)]
    pub extreme_impact_score: u8,
    #[serde(default)]
    pub structure_bias: i16,
    #[serde(default)]
    pub confidence: u8,
    #[serde(default)]
    pub data_quality: u8,
    #[serde(default)]
    pub spot_score: u8,
    #[serde(default)]
    pub contract_score: u8,
    #[serde(default)]
    pub cross_confirm_score: u8,
    #[serde(default)]
    pub main_force_confirmed: bool,
    #[serde(default)]
    pub extreme_impact_confirmed: bool,
    #[serde(default)]
    pub reason: String,
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
    #[serde(default)]
    pub dynamic_baseline_btc: Option<f64>,
    #[serde(default)]
    pub dynamic_threshold_level: String,
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

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ContractWhalePriceResponseType {
    TrendFollowUp,
    TrendFollowDown,
    DownsideAbsorption,
    UpsideResistance,
    #[default]
    NoClearResponse,
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

#[derive(Debug, Clone, Default, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ContractWhaleScoreBreakdown {
    pub volume_score: f64,
    pub notional_score: f64,
    pub dynamic_anomaly_score: f64,
    pub directional_strength_score: f64,
    pub price_response_score: f64,
    pub multi_source_score: f64,
    pub data_quality_score: f64,
    pub dominant_venue_score: f64,
    pub oi_context_score: f64,
    pub penalty_score: f64,
    pub final_score: f64,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ContractWhaleSignalCluster {
    #[serde(default)]
    pub cluster_id: String,
    #[serde(default)]
    pub signal_count: usize,
    #[serde(default)]
    pub dominant_intent: String,
    #[serde(default)]
    pub started_at: i64,
    #[serde(default)]
    pub updated_at: i64,
    #[serde(default)]
    pub duration_ms: u64,
    #[serde(default)]
    pub intensity: f64,
    #[serde(default)]
    pub price_range_pct: Option<f64>,
}

impl Default for ContractWhaleSignalCluster {
    fn default() -> Self {
        Self {
            cluster_id: String::new(),
            signal_count: 1,
            dominant_intent: "single_signal".to_string(),
            started_at: 0,
            updated_at: 0,
            duration_ms: 0,
            intensity: 0.0,
            price_range_pct: None,
        }
    }
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ContractWhalePersistenceState {
    #[serde(default)]
    pub persistence_score: f64,
    #[serde(default)]
    pub signal_half_life_ms: u64,
    #[serde(default)]
    pub regime_stability: f64,
    #[serde(default)]
    pub redundant_with_previous: bool,
    #[serde(default)]
    pub redundant_reason: String,
}

impl Default for ContractWhalePersistenceState {
    fn default() -> Self {
        Self {
            persistence_score: 0.0,
            signal_half_life_ms: 60_000,
            regime_stability: 1.0,
            redundant_with_previous: false,
            redundant_reason: String::new(),
        }
    }
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ContractWhaleAction {
    #[serde(default)]
    pub ts: i64,
    #[serde(default)]
    pub symbol: String,
    #[serde(default)]
    pub action_type: String,
    #[serde(default)]
    pub volume: f64,
    #[serde(default)]
    pub price_impact: f64,
    #[serde(default)]
    pub exchange: String,
}

impl Default for ContractWhaleAction {
    fn default() -> Self {
        Self {
            ts: 0,
            symbol: String::new(),
            action_type: "unknown".to_string(),
            volume: 0.0,
            price_impact: 0.0,
            exchange: "unknown".to_string(),
        }
    }
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ContractWhaleStealthProfile {
    #[serde(default)]
    pub gamma: f64,
    #[serde(default)]
    pub fragmentation: f64,
    #[serde(default)]
    pub entropy: f64,
    #[serde(default)]
    pub cross_exchange_dispersion: f64,
}

impl Default for ContractWhaleStealthProfile {
    fn default() -> Self {
        Self {
            gamma: 0.0,
            fragmentation: 0.0,
            entropy: 0.0,
            cross_exchange_dispersion: 0.0,
        }
    }
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ContractWhaleTrajectory {
    #[serde(default)]
    pub trajectory_id: String,
    #[serde(default)]
    pub start_ts: i64,
    #[serde(default)]
    pub end_ts: i64,
    #[serde(default)]
    pub duration_ms: u64,
    #[serde(default)]
    pub actions: Vec<ContractWhaleAction>,
    #[serde(default)]
    pub intent: String,
    #[serde(default)]
    pub regime_path: Vec<String>,
    #[serde(default)]
    pub stealth_profile: ContractWhaleStealthProfile,
    #[serde(default)]
    pub aggressiveness_curve: Vec<f64>,
    #[serde(default)]
    pub conclusion: String,
}

impl Default for ContractWhaleTrajectory {
    fn default() -> Self {
        Self {
            trajectory_id: String::new(),
            start_ts: 0,
            end_ts: 0,
            duration_ms: 0,
            actions: Vec::new(),
            intent: "unknown".to_string(),
            regime_path: Vec::new(),
            stealth_profile: ContractWhaleStealthProfile::default(),
            aggressiveness_curve: Vec::new(),
            conclusion: String::new(),
        }
    }
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ContractWhaleLiquidationZone {
    #[serde(default)]
    pub side: String,
    #[serde(default)]
    pub low_price_usd: Option<f64>,
    #[serde(default)]
    pub high_price_usd: Option<f64>,
    #[serde(default)]
    pub estimated_size_usd: f64,
    #[serde(default)]
    pub intensity: u8,
    #[serde(default)]
    pub reason: String,
}

impl Default for ContractWhaleLiquidationZone {
    fn default() -> Self {
        Self {
            side: "neutral".to_string(),
            low_price_usd: None,
            high_price_usd: None,
            estimated_size_usd: 0.0,
            intensity: 0,
            reason: String::new(),
        }
    }
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ContractWhaleForcedFlowAttribution {
    #[serde(default)]
    pub whale_pct: f64,
    #[serde(default)]
    pub retail_pct: f64,
    #[serde(default)]
    pub liquidation_pct: f64,
    #[serde(default)]
    pub dominant_driver: String,
}

impl Default for ContractWhaleForcedFlowAttribution {
    fn default() -> Self {
        Self {
            whale_pct: 1.0,
            retail_pct: 0.0,
            liquidation_pct: 0.0,
            dominant_driver: "whale_initiated_flow".to_string(),
        }
    }
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ContractWhalePriceImpactAttribution {
    #[serde(default)]
    pub whale_impact: f64,
    #[serde(default)]
    pub liquidation_cascade: f64,
    #[serde(default)]
    pub stop_loss_sweep: f64,
    #[serde(default)]
    pub passive_absorption: f64,
}

impl Default for ContractWhalePriceImpactAttribution {
    fn default() -> Self {
        Self {
            whale_impact: 0.0,
            liquidation_cascade: 0.0,
            stop_loss_sweep: 0.0,
            passive_absorption: 0.0,
        }
    }
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ContractWhaleLiquidationForce {
    #[serde(default)]
    pub active_zone: String,
    #[serde(default)]
    pub primary_driver: String,
    #[serde(default)]
    pub long_liquidation_pressure: u8,
    #[serde(default)]
    pub short_squeeze_pressure: u8,
    #[serde(default)]
    pub stop_hunt_probability: u8,
    #[serde(default)]
    pub cascade_intensity: u8,
    #[serde(default)]
    pub estimated_forced_size_usd: f64,
    #[serde(default)]
    pub zones: Vec<ContractWhaleLiquidationZone>,
    #[serde(default)]
    pub flow_attribution: ContractWhaleForcedFlowAttribution,
    #[serde(default)]
    pub price_impact: ContractWhalePriceImpactAttribution,
}

impl Default for ContractWhaleLiquidationForce {
    fn default() -> Self {
        Self {
            active_zone: "neutral".to_string(),
            primary_driver: "whale_initiated_flow".to_string(),
            long_liquidation_pressure: 0,
            short_squeeze_pressure: 0,
            stop_hunt_probability: 0,
            cascade_intensity: 0,
            estimated_forced_size_usd: 0.0,
            zones: Vec::new(),
            flow_attribution: ContractWhaleForcedFlowAttribution::default(),
            price_impact: ContractWhalePriceImpactAttribution::default(),
        }
    }
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ContractWhaleMarketDriverComponent {
    #[serde(default)]
    pub key: String,
    #[serde(default)]
    pub score: u8,
    #[serde(default)]
    pub weight_pct: f64,
}

impl Default for ContractWhaleMarketDriverComponent {
    fn default() -> Self {
        Self {
            key: "whale_intent".to_string(),
            score: 0,
            weight_pct: 0.0,
        }
    }
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ContractWhaleMarketDriver {
    #[serde(default)]
    pub primary_driver: String,
    #[serde(default)]
    pub market_state: String,
    #[serde(default)]
    pub whale_intent_pct: f64,
    #[serde(default)]
    pub liquidity_forcing_pct: f64,
    #[serde(default)]
    pub derivatives_pressure_pct: f64,
    #[serde(default)]
    pub reflexivity_pct: f64,
    #[serde(default)]
    pub components: Vec<ContractWhaleMarketDriverComponent>,
    #[serde(default)]
    pub interpretation: String,
}

impl Default for ContractWhaleMarketDriver {
    fn default() -> Self {
        Self {
            primary_driver: "whale_intent".to_string(),
            market_state: "whale_led_expansion".to_string(),
            whale_intent_pct: 1.0,
            liquidity_forcing_pct: 0.0,
            derivatives_pressure_pct: 0.0,
            reflexivity_pct: 0.0,
            components: Vec::new(),
            interpretation: "市场主要由主动资金流驱动。".to_string(),
        }
    }
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
    #[serde(default)]
    pub main_force_score: Option<u8>,
    #[serde(default)]
    pub spot_score: Option<u8>,
    #[serde(default)]
    pub contract_score: Option<u8>,
    #[serde(default)]
    pub base_asset: String,
    #[serde(default)]
    pub quantity_unit: String,
    #[serde(default)]
    pub total_volume: f64,
    #[serde(default)]
    pub net_volume: f64,
    pub total_volume_btc: f64,
    pub net_volume_btc: f64,
    pub total_notional_usd: f64,
    pub dominance: f64,
    #[serde(default)]
    pub order_price_usd: Option<f64>,
    #[serde(default)]
    pub current_market_price_usd: Option<f64>,
    #[serde(default)]
    pub price_deviation_pct: Option<f64>,
    #[serde(default)]
    pub price_deviation_filtered: bool,
    pub price_move_pct: Option<f64>,
    #[serde(default)]
    pub price_move_5s_pct: Option<f64>,
    #[serde(default)]
    pub price_move_15s_pct: Option<f64>,
    #[serde(default)]
    pub price_move_30s_pct: Option<f64>,
    #[serde(default)]
    pub price_response_type: ContractWhalePriceResponseType,
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
    pub dynamic_baseline_btc: Option<f64>,
    #[serde(default)]
    pub dynamic_threshold_level: String,
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
    #[serde(default)]
    pub score_breakdown: ContractWhaleScoreBreakdown,
    #[serde(default = "default_threshold_profile")]
    pub threshold_profile: String,
    #[serde(default)]
    pub threshold_profile_reason: String,
    #[serde(default)]
    pub configured_contract_sources: Vec<String>,
    #[serde(default)]
    pub eligible_contract_sources: Vec<String>,
    #[serde(default)]
    pub active_contract_sources: Vec<String>,
    #[serde(default)]
    pub active_sources: ContractWhaleActiveSources,
    #[serde(default)]
    pub spot_confirmation: ContractWhaleSpotConfirmationContext,
    pub discord_eligible: bool,
    pub discord_sent: bool,
    #[serde(default)]
    pub discord_sent_at: Option<i64>,
    pub discord_reason: String,
    #[serde(default)]
    pub discord_would_send: bool,
    pub final_result: String,
    #[serde(default)]
    pub cluster: ContractWhaleSignalCluster,
    #[serde(default)]
    pub persistence: ContractWhalePersistenceState,
    #[serde(default)]
    pub whale_action: ContractWhaleAction,
    #[serde(default)]
    pub trajectory: ContractWhaleTrajectory,
    #[serde(default)]
    pub liquidation_force: ContractWhaleLiquidationForce,
    #[serde(default)]
    pub market_driver: ContractWhaleMarketDriver,
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
    #[serde(default)]
    pub symbol: String,
    #[serde(default)]
    pub base_asset: String,
    #[serde(default)]
    pub quantity_unit: String,
    #[serde(default = "default_contract_summary_market_type")]
    pub market_type: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub meta: Option<ContractWhaleResponseMeta>,
    #[serde(default)]
    pub threshold_profile: String,
    #[serde(default)]
    pub threshold_profile_reason: String,
    #[serde(default)]
    pub configured_contract_sources: Vec<String>,
    #[serde(default)]
    pub eligible_contract_sources: Vec<String>,
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
    #[serde(default)]
    pub discord_dry_run_stats: ContractWhaleDiscordDryRunStats,
    #[serde(default)]
    pub market_structure_lite: ContractWhaleMarketStructureLite,
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
    #[serde(default)]
    pub symbol: String,
    #[serde(default)]
    pub base_asset: String,
    #[serde(default)]
    pub quantity_unit: String,
    #[serde(default)]
    pub buy_volume: f64,
    #[serde(default)]
    pub sell_volume: f64,
    #[serde(default)]
    pub total_volume: f64,
    #[serde(default)]
    pub net_volume: f64,
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
    #[serde(default)]
    pub product: Option<String>,
    #[serde(default)]
    pub source: Option<String>,
    #[serde(default)]
    pub requires_auth: bool,
    #[serde(default)]
    pub market_data_only: bool,
    #[serde(default)]
    pub auth_configured: bool,
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
