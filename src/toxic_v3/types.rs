use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "snake_case")]
pub enum MarketFlowExchange {
    Binance,
    Okx,
    Bitfinex,
    Coinbase,
    #[default]
    Other,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "snake_case")]
pub enum SignalSource {
    FlowInference,
    ContractWhale,
    BinanceAltContract,
    TofLite,
    #[default]
    Unknown,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "snake_case")]
pub enum Direction {
    Buy,
    Sell,
    Absorption,
    Suppression,
    #[default]
    Neutral,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "snake_case")]
pub enum StealthRegime {
    NonStealth,
    PartialStealth,
    ActiveCamouflage,
    ExtremeStealth,
    #[default]
    Unknown,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "snake_case")]
pub enum HazardStateKind {
    Calm,
    Building,
    Elevated,
    Critical,
    #[default]
    Unknown,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "snake_case")]
pub enum IntentType {
    Accumulation,
    Distribution,
    LiquidityHunting,
    StopHunt,
    StealthBuildUp,
    PanicExit,
    #[default]
    Unknown,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "snake_case")]
pub enum TrajectoryStateKind {
    SinglePoint,
    Building,
    Persistent,
    Decaying,
    Reversal,
    #[default]
    Unknown,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct MarketFlowTick {
    pub ts: i64,
    pub exchange: MarketFlowExchange,
    pub symbol: String,
    pub buy_volume: f64,
    pub sell_volume: f64,
    pub net_flow: f64,
    pub flow_acceleration: f64,
    pub trade_count: u32,
    pub avg_trade_size: f64,
    pub large_trade_ratio: f64,
    pub realized_vol: f64,
    pub open_interest_delta: f64,
    pub funding_rate: f64,
    pub liquidation_pressure: f64,
    pub price_move_pct: f64,
    pub dynamic_multiple: f64,
    pub anomaly_persistence_sec: f64,
    pub cross_exchange_dispersion: f64,
}

impl Default for MarketFlowTick {
    fn default() -> Self {
        Self {
            ts: 0,
            exchange: MarketFlowExchange::Other,
            symbol: String::new(),
            buy_volume: 0.0,
            sell_volume: 0.0,
            net_flow: 0.0,
            flow_acceleration: 0.0,
            trade_count: 0,
            avg_trade_size: 0.0,
            large_trade_ratio: 0.0,
            realized_vol: 0.0,
            open_interest_delta: 0.0,
            funding_rate: 0.0,
            liquidation_pressure: 0.0,
            price_move_pct: 0.0,
            dynamic_multiple: 0.0,
            anomaly_persistence_sec: 0.0,
            cross_exchange_dispersion: 0.0,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct StealthFeatures {
    pub fragmentation_index: f64,
    pub execution_entropy: f64,
    pub cross_exchange_sync: f64,
    pub order_size_variance: f64,
    pub timing_jitter: f64,
    pub impact_dilution_ratio: f64,
    pub cross_exchange_dispersion: f64,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct StealthState {
    pub gamma: f64,
    pub stealth_score: f64,
    pub is_camouflaging: bool,
    pub regime: StealthRegime,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct HazardState {
    pub lambda_t: f64,
    pub detection_pressure: f64,
    pub regulatory_sensitivity: f64,
    pub anomaly_persistence: f64,
    pub flow_irregularity: f64,
    pub liquidation_risk: f64,
    pub state: HazardStateKind,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct IntentState {
    pub intent: IntentType,
    pub confidence: f64,
    pub expected_horizon_sec: f64,
    pub aggression_level: f64,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct TrajectoryState {
    pub score: f64,
    pub state: TrajectoryStateKind,
    pub persistence_sec: f64,
    pub acceleration: f64,
    pub decay_rate: f64,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ToxicV3Enrichment {
    pub symbol: String,
    pub ts: i64,
    pub source: SignalSource,
    pub stealth_score: f64,
    pub stealth_regime: StealthRegime,
    pub hazard_lambda: f64,
    pub hazard_state: HazardStateKind,
    pub intent: IntentType,
    pub intent_confidence: f64,
    pub trajectory_score: f64,
    pub trajectory_state: TrajectoryStateKind,
    pub glce_squeeze_probability: f64,
    pub glce_liquidation_risk: f64,
    pub glce_gamma_pressure: f64,
    pub glce_breakout_bias: String,
    pub lhcs_cascade_probability: f64,
    pub lhcs_direction_bias: String,
    pub lhcs_trigger_level_count: usize,
    pub lhcs_liquidity_void_count: usize,
    pub gex_total: f64,
    pub gex_dealer_position_bias: String,
    pub gex_squeeze_probability: f64,
    pub gex_price_pin_pressure_index: f64,
    pub gex_gamma_wall_count: usize,
    pub mff_total_stress: f64,
    pub mff_liquidity_field: f64,
    pub mff_gamma_field: f64,
    pub mff_liquidation_field: f64,
    pub mff_cascade_field: f64,
    pub mff_directional_bias: String,
    pub mff_instability_index: f64,
    pub mff_regime_state: String,
    pub btc_liquidation_active: bool,
    pub btc_long_liquidation_pressure: f64,
    pub btc_short_liquidation_pressure: f64,
    pub btc_net_liquidation_bias: f64,
    pub btc_squeeze_up_probability: f64,
    pub btc_squeeze_down_probability: f64,
    pub btc_liquidation_cluster_count: usize,
    pub btc_cascade_risk: f64,
    pub btc_gamma_pressure: f64,
    pub explanation_tags: Vec<String>,
    pub read_only: bool,
    pub analysis_only: bool,
    pub direct_discord_gate: bool,
}

impl Default for ToxicV3Enrichment {
    fn default() -> Self {
        Self {
            symbol: String::new(),
            ts: 0,
            source: SignalSource::Unknown,
            stealth_score: 0.0,
            stealth_regime: StealthRegime::Unknown,
            hazard_lambda: 0.0,
            hazard_state: HazardStateKind::Unknown,
            intent: IntentType::Unknown,
            intent_confidence: 0.0,
            trajectory_score: 0.0,
            trajectory_state: TrajectoryStateKind::Unknown,
            glce_squeeze_probability: 0.0,
            glce_liquidation_risk: 0.0,
            glce_gamma_pressure: 0.0,
            glce_breakout_bias: "neutral".to_string(),
            lhcs_cascade_probability: 0.0,
            lhcs_direction_bias: "neutral".to_string(),
            lhcs_trigger_level_count: 0,
            lhcs_liquidity_void_count: 0,
            gex_total: 0.0,
            gex_dealer_position_bias: "neutral".to_string(),
            gex_squeeze_probability: 0.0,
            gex_price_pin_pressure_index: 0.0,
            gex_gamma_wall_count: 0,
            mff_total_stress: 0.0,
            mff_liquidity_field: 0.0,
            mff_gamma_field: 0.0,
            mff_liquidation_field: 0.0,
            mff_cascade_field: 0.0,
            mff_directional_bias: "neutral".to_string(),
            mff_instability_index: 0.0,
            mff_regime_state: "unknown".to_string(),
            btc_liquidation_active: false,
            btc_long_liquidation_pressure: 0.0,
            btc_short_liquidation_pressure: 0.0,
            btc_net_liquidation_bias: 0.0,
            btc_squeeze_up_probability: 0.0,
            btc_squeeze_down_probability: 0.0,
            btc_liquidation_cluster_count: 0,
            btc_cascade_risk: 0.0,
            btc_gamma_pressure: 0.0,
            explanation_tags: Vec::new(),
            read_only: true,
            analysis_only: true,
            direct_discord_gate: false,
        }
    }
}

pub(crate) fn clamp01(value: f64) -> f64 {
    if !value.is_finite() {
        return 0.0;
    }
    value.clamp(0.0, 1.0)
}

pub(crate) fn clamp100(value: f64) -> f64 {
    if !value.is_finite() {
        return 0.0;
    }
    value.clamp(0.0, 100.0)
}
