use serde::{Deserialize, Serialize};

pub const MAX_ACTIVE_TOKENS: usize = 10;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ContractTickSide {
    Buy,
    Sell,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum TokenFlowRegime {
    Accumulation,
    Building,
    Distribution,
    Neutral,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum CapitalPhase {
    Accumulation,
    Markup,
    Distribution,
    Breakdown,
    Neutral,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum FlowActorRegime {
    LiquidityProvider,
    MomentumChaser,
    SmartMoney,
    Mixed,
    Unknown,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum StabilityRegime {
    Trend,
    Chop,
    LiquidityExpansion,
    LiquidityStress,
    Manipulation,
    Neutral,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum AdvisoryDirection {
    Long,
    Short,
    NoTrade,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ExpectedHoldTime {
    Short,
    Mid,
    Long,
    None,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ContractTick {
    pub symbol: String,
    pub price: f64,
    pub size: f64,
    pub side: ContractTickSide,
    pub aggression: f64,
    pub orderbook_imbalance: f64,
    pub timestamp: u64,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TokenFlowSignal {
    pub symbol: String,
    pub regime: TokenFlowRegime,
    pub strength: f64,
    pub confidence: f64,
    pub ofi_windows: Vec<OfiWindowMetrics>,
    pub flow_persistence: f64,
    pub impact_response: ImpactResponse,
    pub liquidity_depletion: LiquidityDepletion,
    pub actor_decomposition: SmartMoneyDecomposition,
    pub signal_compression: SignalCompressionState,
    pub capital_structure: CapitalStructureView,
    pub position_reconstruction: SmartMoneyPositionReconstruction,
    pub evidence: Vec<String>,
    pub read_only: bool,
    pub detector: String,
    pub updated_at_ms: i64,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CapitalStructureView {
    pub phase: CapitalPhase,
    pub phase_label: String,
    pub phase_confidence: f64,
    pub behavior_windows: Vec<BehaviorWindowMetrics>,
    pub cost_basis: CostBasisEstimate,
    pub estimated_position: EstimatedPositionSize,
    pub horizon: TimeHorizonInference,
    pub distribution_risk: DistributionRisk,
    pub evidence: Vec<String>,
    pub read_only: bool,
}

impl Default for CapitalStructureView {
    fn default() -> Self {
        Self {
            phase: CapitalPhase::Neutral,
            phase_label: "neutral".to_string(),
            phase_confidence: 0.0,
            behavior_windows: vec![],
            cost_basis: CostBasisEstimate::default(),
            estimated_position: EstimatedPositionSize::default(),
            horizon: TimeHorizonInference::default(),
            distribution_risk: DistributionRisk::default(),
            evidence: vec!["no_capital_structure_evidence".to_string()],
            read_only: true,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct BehaviorWindowMetrics {
    pub window_sec: u64,
    pub cumulative_delta: f64,
    pub normalized_ofi: f64,
    pub vwap: f64,
    pub volume: f64,
    pub price_drift_pct: f64,
    pub volatility_pct: f64,
    pub absorption_score: f64,
    pub bid_replenishment_score: f64,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CostBasisEstimate {
    pub lower: f64,
    pub upper: f64,
    pub vwap_anchor: f64,
    pub confidence: f64,
}

impl Default for CostBasisEstimate {
    fn default() -> Self {
        Self {
            lower: 0.0,
            upper: 0.0,
            vwap_anchor: 0.0,
            confidence: 0.0,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct EstimatedPositionSize {
    pub lower_usd: f64,
    pub upper_usd: f64,
    pub confidence: f64,
}

impl Default for EstimatedPositionSize {
    fn default() -> Self {
        Self {
            lower_usd: 0.0,
            upper_usd: 0.0,
            confidence: 0.0,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TimeHorizonInference {
    pub min_minutes: f64,
    pub max_minutes: f64,
    pub detected_minutes: f64,
}

impl Default for TimeHorizonInference {
    fn default() -> Self {
        Self {
            min_minutes: 0.0,
            max_minutes: 0.0,
            detected_minutes: 0.0,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DistributionRisk {
    pub score: f64,
    pub level: String,
    pub reasons: Vec<String>,
}

impl Default for DistributionRisk {
    fn default() -> Self {
        Self {
            score: 0.0,
            level: "low".to_string(),
            reasons: vec!["no_distribution_evidence".to_string()],
        }
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SmartMoneyPositionReconstruction {
    pub accumulation_path: Vec<PositionPathSegment>,
    pub last_accumulation_node: Option<LastAccumulationNode>,
    pub distribution_path: Vec<PositionPathSegment>,
    pub latent_position: Vec<LatentPositionPoint>,
    pub confidence: f64,
    pub regime_label: String,
    pub evidence: Vec<String>,
    pub read_only: bool,
}

impl Default for SmartMoneyPositionReconstruction {
    fn default() -> Self {
        Self {
            accumulation_path: vec![],
            last_accumulation_node: None,
            distribution_path: vec![],
            latent_position: vec![],
            confidence: 0.0,
            regime_label: "neutral".to_string(),
            evidence: vec!["no_reconstruction_evidence".to_string()],
            read_only: true,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PositionPathSegment {
    pub phase: CapitalPhase,
    pub label: String,
    pub start_price: f64,
    pub end_price: f64,
    pub volume: f64,
    pub cumulative_delta: f64,
    pub impact: f64,
    pub duration_sec: u64,
    pub confidence: f64,
    pub characteristics: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct LastAccumulationNode {
    pub lower: f64,
    pub upper: f64,
    pub duration_sec: u64,
    pub volatility_pct: f64,
    pub absorption_efficiency: f64,
    pub confidence: f64,
    pub characteristics: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct LatentPositionPoint {
    pub timestamp: u64,
    pub price: f64,
    pub estimated_position: f64,
    pub impact_adjusted_position: f64,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct BehaviorProbabilities {
    pub continue_distribution: f64,
    pub range_consolidation: f64,
    pub rebound_markup: f64,
    pub secondary_accumulation: f64,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PhaseTimelineSegment {
    pub phase: CapitalPhase,
    pub label: String,
    pub start_ms: u64,
    pub end_ms: u64,
    pub duration_sec: u64,
    pub lower: f64,
    pub upper: f64,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CostDistributionBand {
    pub label: String,
    pub lower: f64,
    pub upper: f64,
    pub pct: f64,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SmartLevel {
    pub label: String,
    pub price: f64,
    pub role: String,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SmartMoneyReconstructionResponse {
    pub symbol: String,
    pub timeframe: String,
    pub current_phase: CapitalPhase,
    pub current_price: f64,
    pub change_24h_pct: Option<f64>,
    pub volume_24h_usd: Option<f64>,
    pub high_24h: Option<f64>,
    pub low_24h: Option<f64>,
    pub market_cap_usd: Option<f64>,
    pub cost_basis_low: f64,
    pub cost_basis_high: f64,
    pub vwap_anchor: f64,
    pub estimated_total_position_usdt_low: f64,
    pub estimated_total_position_usdt_high: f64,
    pub estimated_net_position_usdt: f64,
    pub floating_pnl_low_pct: f64,
    pub floating_pnl_high_pct: f64,
    pub accumulation_path: Vec<PositionPathSegment>,
    pub last_accumulation_node: Option<LastAccumulationNode>,
    pub distribution_path: Vec<PositionPathSegment>,
    pub distribution_completion_pct: f64,
    pub distribution_intensity_score: f64,
    pub short_term_behavior_probabilities: BehaviorProbabilities,
    pub phase_timeline: Vec<PhaseTimelineSegment>,
    pub cost_distribution: Vec<CostDistributionBand>,
    pub smart_levels: Vec<SmartLevel>,
    pub confidence: f64,
    pub read_only: bool,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TokenChartPoint {
    pub ts: u64,
    pub price: f64,
    pub volume: f64,
    pub net_position: f64,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TokenChartMarker {
    pub ts: u64,
    pub price: f64,
    pub label: String,
    pub kind: String,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SmartMoneyChartResponse {
    pub symbol: String,
    pub timeframe: String,
    pub points: Vec<TokenChartPoint>,
    pub phase_segments: Vec<PhaseTimelineSegment>,
    pub markers: Vec<TokenChartMarker>,
    pub read_only: bool,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SmartMoneyDecomposition {
    pub liquidity_provider_probability: f64,
    pub momentum_chaser_probability: f64,
    pub smart_money_probability: f64,
    pub dominant_actor: FlowActorRegime,
    pub lp_score: f64,
    pub momentum_score: f64,
    pub smart_money_score: f64,
    pub confidence: f64,
    pub explanation_tags: Vec<String>,
}

impl Default for SmartMoneyDecomposition {
    fn default() -> Self {
        Self {
            liquidity_provider_probability: 0.0,
            momentum_chaser_probability: 0.0,
            smart_money_probability: 0.0,
            dominant_actor: FlowActorRegime::Unknown,
            lp_score: 0.0,
            momentum_score: 0.0,
            smart_money_score: 0.0,
            confidence: 0.0,
            explanation_tags: vec!["no_actor_evidence".to_string()],
        }
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SignalCompressionState {
    pub smart_money_pressure: f64,
    pub momentum_flow_exhaustion: f64,
    pub liquidity_stress_manipulation: f64,
    pub position_validity_gate: PositionValidityGate,
    pub stability_kernel: TradingStabilityKernel,
    pub explanation_tags: Vec<String>,
    pub read_only: bool,
}

impl Default for SignalCompressionState {
    fn default() -> Self {
        Self {
            smart_money_pressure: 0.0,
            momentum_flow_exhaustion: 0.0,
            liquidity_stress_manipulation: 0.0,
            position_validity_gate: PositionValidityGate::default(),
            stability_kernel: TradingStabilityKernel::default(),
            explanation_tags: vec!["no_compressed_signal".to_string()],
            read_only: true,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PositionValidityGate {
    pub risk_score: f64,
    pub trade_permission: bool,
    pub position_size_multiplier: f64,
    pub reason: String,
    pub advisory_only: bool,
}

impl Default for PositionValidityGate {
    fn default() -> Self {
        Self {
            risk_score: 0.0,
            trade_permission: false,
            position_size_multiplier: 0.0,
            reason: "no_signal".to_string(),
            advisory_only: true,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TradingStabilityKernel {
    pub regime: StabilityRegime,
    pub regime_quality: f64,
    pub trade_signal: TradeSignalAdvisory,
    pub position_smoothing: PositionSmoothing,
    pub read_only: bool,
}

impl Default for TradingStabilityKernel {
    fn default() -> Self {
        Self {
            regime: StabilityRegime::Neutral,
            regime_quality: 0.0,
            trade_signal: TradeSignalAdvisory::default(),
            position_smoothing: PositionSmoothing::default(),
            read_only: true,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TradeSignalAdvisory {
    pub direction: AdvisoryDirection,
    pub confidence: f64,
    pub expected_hold_time: ExpectedHoldTime,
    pub invalidation_condition: String,
    pub reason: String,
    pub advisory_only: bool,
}

impl Default for TradeSignalAdvisory {
    fn default() -> Self {
        Self {
            direction: AdvisoryDirection::NoTrade,
            confidence: 0.0,
            expected_hold_time: ExpectedHoldTime::None,
            invalidation_condition: "no_signal".to_string(),
            reason: "no_trade".to_string(),
            advisory_only: true,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PositionSmoothing {
    pub suggested_size_multiplier: f64,
    pub volatility_adjustment: f64,
    pub drawdown_adjustment: f64,
    pub reason: String,
}

impl Default for PositionSmoothing {
    fn default() -> Self {
        Self {
            suggested_size_multiplier: 0.0,
            volatility_adjustment: 0.0,
            drawdown_adjustment: 1.0,
            reason: "no_signal".to_string(),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct OfiWindowMetrics {
    pub window_sec: u64,
    pub buy_pressure: f64,
    pub sell_pressure: f64,
    pub net_ofi: f64,
    pub normalized_ofi: f64,
    pub decay_weighted_ofi: f64,
    pub persistence: f64,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ImpactResponse {
    pub price_move_pct: f64,
    pub total_volume: f64,
    pub impact_per_volume: f64,
    pub absorption_score: f64,
    pub thin_liquidity_score: f64,
    pub classification: String,
}

impl Default for ImpactResponse {
    fn default() -> Self {
        Self {
            price_move_pct: 0.0,
            total_volume: 0.0,
            impact_per_volume: 0.0,
            absorption_score: 0.0,
            thin_liquidity_score: 0.0,
            classification: "unknown".to_string(),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct LiquidityDepletion {
    pub bid_depletion_rate: f64,
    pub ask_depletion_rate: f64,
    pub replenishment_rate: f64,
    pub depletion_pressure: f64,
}

impl Default for LiquidityDepletion {
    fn default() -> Self {
        Self {
            bid_depletion_rate: 0.0,
            ask_depletion_rate: 0.0,
            replenishment_rate: 0.0,
            depletion_pressure: 0.0,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TokenWatchItem {
    pub symbol: String,
    pub added_at_ms: i64,
    pub stream_status: String,
    pub last_signal: TokenFlowSignal,
    pub read_only: bool,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TokenWatchListResponse {
    pub items: Vec<TokenWatchItem>,
    pub max_active_tokens: usize,
    pub active_count: usize,
    pub read_only: bool,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct NewTokenWatchRequest {
    pub symbol: String,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct NewTokenWatchMutationResponse {
    pub ok: bool,
    pub item: Option<TokenWatchItem>,
    pub items: Vec<TokenWatchItem>,
    pub error: Option<String>,
    pub max_active_tokens: usize,
    pub read_only: bool,
}
