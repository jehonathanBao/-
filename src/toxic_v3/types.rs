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
