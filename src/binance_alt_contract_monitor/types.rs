use std::collections::BTreeMap;

#[derive(
    Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, serde::Serialize, serde::Deserialize,
)]
#[serde(rename_all = "lowercase")]
pub enum AltContractExchange {
    Binance,
}

impl AltContractExchange {
    pub fn as_key(self) -> &'static str {
        match self {
            Self::Binance => "binance",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum AltContractTradeSide {
    Buy,
    Sell,
}

#[derive(
    Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, serde::Serialize, serde::Deserialize,
)]
#[serde(rename_all = "lowercase")]
pub enum AltContractDirection {
    Buy,
    Sell,
    Absorption,
    Suppression,
    Neutral,
}

#[derive(
    Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, serde::Serialize, serde::Deserialize,
)]
#[serde(rename_all = "lowercase")]
pub enum AltContractSeverity {
    Calm,
    Medium,
    High,
    Critical,
    S,
}

impl AltContractSeverity {
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
pub enum AltContractSignalType {
    MainForceLongBuild,
    MainForceShortBuild,
    AbnormalPump,
    AbnormalDump,
    DownsideAbsorption,
    UpsideResistance,
    LiquidationCascade,
    UnclearContractAnomaly,
}

#[derive(
    Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, serde::Serialize, serde::Deserialize,
)]
#[serde(rename_all = "lowercase")]
pub enum AltContractSymbolTier {
    A,
    B,
    C,
    D,
    E,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AltContractTrade {
    pub ts: i64,
    pub exchange: AltContractExchange,
    pub symbol: String,
    pub product_id: String,
    pub price: f64,
    pub qty_base: f64,
    pub notional_usd: f64,
    pub side: AltContractTradeSide,
    pub trade_id: Option<String>,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AltContractExchangeStatus {
    pub connected: bool,
    pub status: String,
    pub last_trade_at: Option<i64>,
    pub latency_ms: Option<i64>,
    pub reconnect_count: u64,
    pub last_error: Option<String>,
}

impl AltContractExchangeStatus {
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

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AltContractSymbolMeta {
    pub symbol: String,
    pub product_id: String,
    pub tier: AltContractSymbolTier,
    pub quote_volume_24h_usd: f64,
}

#[derive(Debug, Clone, Default, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AltContractExchangeContribution {
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

#[derive(Debug, Clone, Default, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AltContractContext {
    pub oi_change_1m_base: Option<f64>,
    pub oi_change_5m_base: Option<f64>,
    pub oi_change_pct: Option<f64>,
    pub oi_updated_at: Option<i64>,
    pub funding_rate: Option<f64>,
    pub funding_bias: Option<String>,
    pub mark_price_usd: Option<f64>,
    pub mark_price_updated_at: Option<i64>,
    pub last_price_usd: Option<f64>,
    pub ticker_quote_volume_24h_usd: Option<f64>,
    pub ticker_price_change_24h_pct: Option<f64>,
    pub ticker_updated_at: Option<i64>,
    pub liquidation_notional_usd: Option<f64>,
    pub liquidation_suspected: bool,
    pub price_move_1m_pct: Option<f64>,
    pub force_order_snapshot: bool,
    pub persistence_windows: u8,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AltContractWindowStats {
    pub symbol: String,
    pub product_id: String,
    pub tier: AltContractSymbolTier,
    pub window_sec: u64,
    pub ts: i64,
    pub buy_volume_base: f64,
    pub sell_volume_base: f64,
    pub total_volume_base: f64,
    pub net_volume_base: f64,
    pub total_notional_usd: f64,
    pub dominance: f64,
    pub direction: AltContractDirection,
    pub trigger_price_usd: Option<f64>,
    pub price_move_pct: Option<f64>,
    pub exchange_count: usize,
    pub main_exchange: Option<String>,
    pub exchanges: Vec<AltContractExchangeContribution>,
    pub dynamic_multiple: Option<f64>,
    pub data_quality: u8,
    pub startup_age_ms: Option<i64>,
}

#[derive(Debug, Clone, Default, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AltContractScoreBreakdown {
    pub volume_score: f64,
    pub dynamic_score: f64,
    pub directional_score: f64,
    pub oi_score: f64,
    pub price_score: f64,
    pub liquidation_score: f64,
    pub persistence_score: f64,
    pub funding_score: f64,
    pub data_quality_score: f64,
    pub penalty_score: f64,
    pub abnormal_score: f64,
    pub build_score: f64,
}

#[derive(Debug, Clone, Default, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AltContractWindowConfirmation {
    pub window_sec: u64,
    pub notional_usd: f64,
    pub dynamic_multiple: Option<f64>,
    pub directional_strength: f64,
    pub confirmed: bool,
}

#[derive(Debug, Clone, Default, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AltContractGradeCondition {
    pub key: String,
    pub label: String,
    pub passed: bool,
    pub actual: String,
    pub threshold: String,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AltContractSourceSnapshot {
    pub exchange: String,
    pub market_type: String,
    pub role: String,
    pub enabled: bool,
    pub status: String,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AltContractSignal {
    pub id: String,
    pub ts: i64,
    pub symbol: String,
    pub product_id: String,
    pub tier: AltContractSymbolTier,
    pub window_sec: u64,
    pub signal_type: AltContractSignalType,
    pub direction: AltContractDirection,
    pub severity: AltContractSeverity,
    pub abnormal_score: u8,
    pub build_score: u8,
    #[serde(default)]
    pub s_grade_eligible: bool,
    #[serde(default)]
    pub s_grade_conditions: Vec<AltContractGradeCondition>,
    #[serde(default)]
    pub s_grade_notional_threshold_usd: f64,
    #[serde(default)]
    pub s_grade_volume_threshold_base: f64,
    #[serde(default)]
    pub main_force_confidence: f64,
    #[serde(default)]
    pub evidence_count: u8,
    #[serde(default)]
    pub evidence_tags: Vec<String>,
    #[serde(default)]
    pub window_confirmations: Vec<AltContractWindowConfirmation>,
    #[serde(default)]
    pub market_wide_move: bool,
    #[serde(default)]
    pub market_wide_direction: Option<String>,
    #[serde(default)]
    pub market_impulse_ratio: f64,
    #[serde(default)]
    pub relative_strength_rank: Option<u32>,
    #[serde(default = "default_post_signal_status")]
    pub post_signal_status: String,
    #[serde(default)]
    pub validated_at: Option<i64>,
    #[serde(default)]
    pub failed_at: Option<i64>,
    #[serde(default)]
    pub signal_vwap: f64,
    #[serde(default = "default_retest_status")]
    pub retest_status: String,
    #[serde(default)]
    pub oi_freshness_sec: Option<u64>,
    #[serde(default)]
    pub oi_change_1m_pct: Option<f64>,
    #[serde(default)]
    pub oi_change_5m_pct: Option<f64>,
    #[serde(default)]
    pub oi_change_15m_pct: Option<f64>,
    #[serde(default)]
    pub oi_notional_change_usd: Option<f64>,
    #[serde(default = "default_oi_quality")]
    pub oi_quality: String,
    #[serde(default = "default_funding_crowding")]
    pub funding_crowding: String,
    #[serde(default)]
    pub funding_penalty: f64,
    #[serde(default)]
    pub spread_bps: Option<f64>,
    #[serde(default)]
    pub depth_0_5pct_usd: Option<f64>,
    #[serde(default)]
    pub depth_1pct_usd: Option<f64>,
    #[serde(default)]
    pub flow_to_depth_ratio: Option<f64>,
    #[serde(default)]
    pub event_id: Option<String>,
    #[serde(default)]
    pub event_signal_count: u32,
    #[serde(default)]
    pub event_peak_abnormal_score: u8,
    #[serde(default)]
    pub event_peak_build_score: u8,
    pub direction_bias: i16,
    pub data_quality: u8,
    pub total_volume_base: f64,
    pub net_volume_base: f64,
    pub total_notional_usd: f64,
    pub trigger_price_usd: Option<f64>,
    pub dominance: f64,
    pub price_move_pct: Option<f64>,
    pub dynamic_multiple: Option<f64>,
    pub oi_change_1m_base: Option<f64>,
    pub oi_change_5m_base: Option<f64>,
    pub oi_change_pct: Option<f64>,
    pub funding_rate: Option<f64>,
    pub liquidation_notional_usd: Option<f64>,
    pub liquidation_suspected: bool,
    pub force_order_snapshot: bool,
    pub main_exchange: Option<String>,
    pub exchanges: Vec<AltContractExchangeContribution>,
    pub score_breakdown: AltContractScoreBreakdown,
    pub active_sources: Vec<AltContractSourceSnapshot>,
    pub explain_tags: Vec<String>,
    pub abnormal_explanation: String,
    pub build_explanation: String,
    pub liquidation_explanation: String,
    pub discord_eligible: bool,
    pub discord_would_send: bool,
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
pub struct AltContractTrend60s {
    pub buy_volume_base: f64,
    pub sell_volume_base: f64,
    pub total_volume_base: f64,
    pub net_volume_base: f64,
    pub total_notional_usd: f64,
    pub dominance: f64,
    pub buy_ratio: f64,
    pub sell_ratio: f64,
    pub updated_at_ms: Option<i64>,
}

#[derive(Debug, Clone, Default, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AltContractAllMarketContextStatus {
    pub mark_price_connected: bool,
    pub ticker_connected: bool,
    pub force_order_connected: bool,
    pub last_mark_price_at: Option<i64>,
    pub last_ticker_at: Option<i64>,
    pub last_force_order_at: Option<i64>,
    pub candidate_symbols: Vec<String>,
    pub hot_oi_symbols: Vec<String>,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AltContractSummary {
    pub status: String,
    pub health_status: String,
    pub health_reason: String,
    pub collector_status: String,
    pub last_trade_at: Option<i64>,
    pub last_oi_poll_at: Option<i64>,
    pub last_force_order_at: Option<i64>,
    pub flow_buckets1m: usize,
    pub signals1h: usize,
    pub would_send1h: usize,
    pub top_active_symbols: Vec<String>,
    pub errors1h: usize,
    pub latest_direction: String,
    pub latest_severity: AltContractSeverity,
    pub latest_signal_at: Option<i64>,
    pub signal_count: usize,
    pub monitored_symbols: Vec<String>,
    pub active_anomaly_count: usize,
    pub recent_critical_or_s_count: usize,
    pub dry_run_would_send_count: usize,
    pub enabled: bool,
    pub dry_run: bool,
    pub read_only: bool,
    pub symbol: Option<String>,
    pub trend60s: AltContractTrend60s,
    pub exchanges: BTreeMap<String, AltContractExchangeStatus>,
    pub dry_run_stats: AltContractDryRunStats,
    pub symbol_universe: AltContractSymbolUniverseSummary,
    pub all_market_context: AltContractAllMarketContextStatus,
}

#[derive(Debug, Clone, Default, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AltContractDryRunStats {
    pub signals1h: usize,
    pub high1h: usize,
    pub critical1h: usize,
    pub s1h: usize,
    pub would_send1h: usize,
    pub skipped_low_score1h: usize,
    pub skipped_cooldown1h: usize,
    pub skipped_data_quality1h: usize,
    pub liquidation_driven1h: usize,
    pub signals24h: usize,
    pub high24h: usize,
    pub critical24h: usize,
    pub s24h: usize,
    pub would_send24h: usize,
    pub skipped_low_score24h: usize,
    pub skipped_cooldown24h: usize,
    pub skipped_data_quality24h: usize,
    pub liquidation_driven24h: usize,
}

#[derive(Debug, Clone, Default, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AltContractSymbolUniverseSummary {
    pub mode: String,
    pub limit: usize,
    pub monitored_count: usize,
    pub tier_counts: BTreeMap<String, usize>,
    pub whitelist: Vec<String>,
    pub blacklist: Vec<String>,
    pub excluded_symbols: Vec<String>,
    pub min_24h_quote_volume_usd: f64,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AltContractLatestResponse {
    pub summary: AltContractSummary,
    pub items: Vec<AltContractSignal>,
    pub limit: usize,
}

#[derive(Debug, Clone, Copy)]
pub struct AltContractTierThresholds {
    pub high_notional_usd: f64,
    pub critical_notional_usd: f64,
    pub s_notional_usd: f64,
}

fn default_post_signal_status() -> String {
    "pending".to_string()
}

fn default_retest_status() -> String {
    "unknown".to_string()
}

fn default_oi_quality() -> String {
    "missing".to_string()
}

fn default_funding_crowding() -> String {
    "neutral".to_string()
}
