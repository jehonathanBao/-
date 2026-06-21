use serde::{Deserialize, Serialize};

pub const MAX_ACTIVE_TOKENS: usize = 50;

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

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum PriceSource {
    MarketSpot,
    MarketPerp,
    MarkPrice,
    Vwap,
    Reconstructed,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct MarketPriceSnapshot {
    pub price: f64,
    pub source: PriceSource,
    pub updated_at_ms: i64,
    pub change_24h_pct: Option<f64>,
    pub volume_24h_usd: Option<f64>,
    pub high_24h: Option<f64>,
    pub low_24h: Option<f64>,
    pub stale: bool,
    pub fallback_reason: Option<String>,
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
    pub density_peak: f64,
    pub confidence: f64,
}

impl Default for CostBasisEstimate {
    fn default() -> Self {
        Self {
            lower: 0.0,
            upper: 0.0,
            vwap_anchor: 0.0,
            density_peak: 0.0,
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
    pub market_price: f64,
    pub market_price_source: PriceSource,
    pub analysis_price: f64,
    pub analysis_price_source: PriceSource,
    pub price_fallback_reason: Option<String>,
    pub change_24h_pct: Option<f64>,
    pub volume_24h_usd: Option<f64>,
    pub high_24h: Option<f64>,
    pub low_24h: Option<f64>,
    pub market_cap_usd: Option<f64>,
    pub cost_basis_low: f64,
    pub cost_basis_high: f64,
    pub vwap_anchor: f64,
    pub density_peak: f64,
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
    pub behavior_windows: Vec<BehaviorWindowMetrics>,
    pub capital_timeline: CapitalTimeline,
    pub position_flow_curve: PositionFlowCurve,
    pub liquidity_reaction_map: LiquidityReactionMap,
    pub market_dynamics: MarketDynamicsState,
    pub liquidity_force: LiquidityForceState,
    pub trading_decision: TradingDecisionKernel,
    pub execution_strategy: ExecutionStrategyKernel,
    pub phase_timeline: Vec<PhaseTimelineSegment>,
    pub cost_distribution: Vec<CostDistributionBand>,
    pub smart_levels: Vec<SmartLevel>,
    pub confidence: f64,
    pub read_only: bool,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CapitalTimeline {
    pub phases: Vec<CapitalTimelinePhase>,
    pub dominant_phase: CapitalPhase,
    pub total_duration_sec: u64,
    pub narrative: String,
}

impl Default for CapitalTimeline {
    fn default() -> Self {
        Self {
            phases: vec![],
            dominant_phase: CapitalPhase::Neutral,
            total_duration_sec: 0,
            narrative: "awaiting_capital_timeline".to_string(),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CapitalTimelinePhase {
    pub phase: CapitalPhase,
    pub label: String,
    pub start_ms: u64,
    pub end_ms: u64,
    pub duration_sec: u64,
    pub net_flow_usd: f64,
    pub transition_reason: String,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PositionFlowCurve {
    pub points: Vec<PositionFlowPoint>,
    pub accumulation_slope_usd_per_min: f64,
    pub distribution_slope_usd_per_min: f64,
    pub latest_position_usd: f64,
}

impl Default for PositionFlowCurve {
    fn default() -> Self {
        Self {
            points: vec![],
            accumulation_slope_usd_per_min: 0.0,
            distribution_slope_usd_per_min: 0.0,
            latest_position_usd: 0.0,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PositionFlowPoint {
    pub ts: u64,
    pub position_usd: f64,
    pub speed_usd_per_min: f64,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct LiquidityReactionMap {
    pub impact_efficiency: f64,
    pub absorption_ratio: f64,
    pub liquidity_response: String,
    pub vacuum_zones: Vec<LiquidityVacuumZone>,
    pub evidence: Vec<String>,
}

impl Default for LiquidityReactionMap {
    fn default() -> Self {
        Self {
            impact_efficiency: 0.0,
            absorption_ratio: 0.0,
            liquidity_response: "unknown".to_string(),
            vacuum_zones: vec![],
            evidence: vec!["no_liquidity_reaction_evidence".to_string()],
        }
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct LiquidityVacuumZone {
    pub lower: f64,
    pub upper: f64,
    pub intensity: f64,
    pub reason: String,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct MarketDynamicsState {
    pub state_vector: MarketStateVector,
    pub state_velocity: MarketStateVelocity,
    pub transition_matrix: Vec<RegimeTransitionProbability>,
    pub market_energy: MarketEnergy,
    pub trajectory_summary: String,
    pub read_only: bool,
}

impl Default for MarketDynamicsState {
    fn default() -> Self {
        Self {
            state_vector: MarketStateVector::default(),
            state_velocity: MarketStateVelocity::default(),
            transition_matrix: vec![],
            market_energy: MarketEnergy::default(),
            trajectory_summary: "awaiting_market_dynamics".to_string(),
            read_only: true,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct MarketStateVector {
    pub smp: f64,
    pub mfe: f64,
    pub lsm: f64,
    pub regime: StabilityRegime,
    pub position_usd: f64,
    pub cost_basis: f64,
    pub liquidity: f64,
}

impl Default for MarketStateVector {
    fn default() -> Self {
        Self {
            smp: 0.0,
            mfe: 0.0,
            lsm: 0.0,
            regime: StabilityRegime::Neutral,
            position_usd: 0.0,
            cost_basis: 0.0,
            liquidity: 0.0,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct MarketStateVelocity {
    pub flow_acceleration: f64,
    pub liquidity_shift_rate: f64,
    pub regime_transition_speed: f64,
    pub position_velocity_usd_per_min: f64,
}

impl Default for MarketStateVelocity {
    fn default() -> Self {
        Self {
            flow_acceleration: 0.0,
            liquidity_shift_rate: 0.0,
            regime_transition_speed: 0.0,
            position_velocity_usd_per_min: 0.0,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RegimeTransitionProbability {
    pub from: CapitalPhase,
    pub to: CapitalPhase,
    pub probability: f64,
    pub reason: String,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct MarketEnergy {
    pub score: f64,
    pub level: String,
    pub flow_strength: f64,
    pub liquidity_availability: f64,
    pub regime_stability: f64,
}

impl Default for MarketEnergy {
    fn default() -> Self {
        Self {
            score: 0.0,
            level: "low".to_string(),
            flow_strength: 0.0,
            liquidity_availability: 0.0,
            regime_stability: 0.0,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct LiquidityForceState {
    pub liquidation_zones: Vec<LiquidationZone>,
    pub stop_loss_cascade: StopLossCascadeState,
    pub forced_flow_attribution: ForcedFlowAttribution,
    pub price_impact_decomposition: PriceImpactDecomposition,
    pub primary_driver: String,
    pub active_zone: String,
    pub read_only: bool,
}

impl Default for LiquidityForceState {
    fn default() -> Self {
        Self {
            liquidation_zones: vec![],
            stop_loss_cascade: StopLossCascadeState::default(),
            forced_flow_attribution: ForcedFlowAttribution::default(),
            price_impact_decomposition: PriceImpactDecomposition::default(),
            primary_driver: "unknown".to_string(),
            active_zone: "neutral_zone".to_string(),
            read_only: true,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct LiquidationZone {
    pub side: String,
    pub lower: f64,
    pub upper: f64,
    pub intensity: f64,
    pub leverage_density: f64,
    pub reason: String,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct StopLossCascadeState {
    pub stop_hunt_probability: f64,
    pub cascade_intensity: f64,
    pub sweep_direction: AdvisoryDirection,
    pub liquidity_sweep: String,
}

impl Default for StopLossCascadeState {
    fn default() -> Self {
        Self {
            stop_hunt_probability: 0.0,
            cascade_intensity: 0.0,
            sweep_direction: AdvisoryDirection::NoTrade,
            liquidity_sweep: "none".to_string(),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ForcedFlowAttribution {
    pub whale_pct: f64,
    pub retail_pct: f64,
    pub liquidation_pct: f64,
    pub dominant_driver: String,
}

impl Default for ForcedFlowAttribution {
    fn default() -> Self {
        Self {
            whale_pct: 0.0,
            retail_pct: 0.0,
            liquidation_pct: 0.0,
            dominant_driver: "unknown".to_string(),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PriceImpactDecomposition {
    pub whale_impact: f64,
    pub liquidation_cascade: f64,
    pub stop_loss_sweep: f64,
    pub passive_absorption: f64,
}

impl Default for PriceImpactDecomposition {
    fn default() -> Self {
        Self {
            whale_impact: 0.0,
            liquidation_cascade: 0.0,
            stop_loss_sweep: 0.0,
            passive_absorption: 0.0,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum DecisionOrderType {
    Market,
    Limit,
    None,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum DecisionTiming {
    Immediate,
    Wait,
    Invalid,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TradingDecisionKernel {
    pub direction: AdvisoryDirection,
    pub entry: TradingDecisionEntry,
    pub exit: TradingDecisionExit,
    pub position_size: TradingPositionSize,
    pub invalidation: TradingInvalidation,
    pub confidence: f64,
    pub advisory_only: bool,
    pub read_only: bool,
}

impl Default for TradingDecisionKernel {
    fn default() -> Self {
        Self {
            direction: AdvisoryDirection::NoTrade,
            entry: TradingDecisionEntry::default(),
            exit: TradingDecisionExit::default(),
            position_size: TradingPositionSize::default(),
            invalidation: TradingInvalidation::default(),
            confidence: 0.0,
            advisory_only: true,
            read_only: true,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ExecutionStrategyKernel {
    pub direction: AdvisoryDirection,
    pub entry: TradingDecisionEntry,
    pub exit: TradingDecisionExit,
    pub position_size: TradingPositionSize,
    pub stop: TradingInvalidation,
    pub confidence: f64,
    pub primary_driver: String,
    pub secondary_driver: String,
    pub reasoning: Vec<String>,
    pub advisory_only: bool,
    pub read_only: bool,
}

impl Default for ExecutionStrategyKernel {
    fn default() -> Self {
        Self {
            direction: AdvisoryDirection::NoTrade,
            entry: TradingDecisionEntry::default(),
            exit: TradingDecisionExit::default(),
            position_size: TradingPositionSize::default(),
            stop: TradingInvalidation::default(),
            confidence: 0.0,
            primary_driver: "none".to_string(),
            secondary_driver: "none".to_string(),
            reasoning: vec!["advisory_only_no_exchange_execution".to_string()],
            advisory_only: true,
            read_only: true,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TradingDecisionEntry {
    pub order_type: DecisionOrderType,
    pub zone_low: f64,
    pub zone_high: f64,
    pub timing: DecisionTiming,
    pub condition: String,
}

impl Default for TradingDecisionEntry {
    fn default() -> Self {
        Self {
            order_type: DecisionOrderType::None,
            zone_low: 0.0,
            zone_high: 0.0,
            timing: DecisionTiming::Invalid,
            condition: "no_entry".to_string(),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TradingDecisionExit {
    pub zone_low: f64,
    pub zone_high: f64,
    pub condition: String,
    pub timing: DecisionTiming,
}

impl Default for TradingDecisionExit {
    fn default() -> Self {
        Self {
            zone_low: 0.0,
            zone_high: 0.0,
            condition: "no_exit".to_string(),
            timing: DecisionTiming::Invalid,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TradingPositionSize {
    pub pct: f64,
    pub multiplier: f64,
    pub reason: String,
}

impl Default for TradingPositionSize {
    fn default() -> Self {
        Self {
            pct: 0.0,
            multiplier: 0.0,
            reason: "no_trade".to_string(),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TradingInvalidation {
    pub active: bool,
    pub price_level: f64,
    pub regime_condition: String,
    pub flow_condition: String,
    pub liquidity_condition: String,
}

impl Default for TradingInvalidation {
    fn default() -> Self {
        Self {
            active: true,
            price_level: 0.0,
            regime_condition: "no_regime_confirmation".to_string(),
            flow_condition: "no_flow_confirmation".to_string(),
            liquidity_condition: "no_liquidity_confirmation".to_string(),
        }
    }
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
    pub market_price: f64,
    pub market_price_source: PriceSource,
    pub analysis_price: f64,
    pub analysis_price_source: PriceSource,
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
    pub stable_signals: StableSignals,
    pub regime_state: RegimeState,
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
            stable_signals: StableSignals::default(),
            regime_state: RegimeState::default(),
            position_validity_gate: PositionValidityGate::default(),
            stability_kernel: TradingStabilityKernel::default(),
            explanation_tags: vec!["no_compressed_signal".to_string()],
            read_only: true,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct StableSignals {
    pub smp_stable: f64,
    pub mfe_stable: f64,
    pub lsm_stable: f64,
    pub stability_score: f64,
    pub persistence_windows: u32,
    pub flip_penalty: f64,
}

impl Default for StableSignals {
    fn default() -> Self {
        Self {
            smp_stable: 0.0,
            mfe_stable: 0.0,
            lsm_stable: 0.0,
            stability_score: 0.0,
            persistence_windows: 0,
            flip_penalty: 0.0,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RegimeState {
    pub current: StabilityRegime,
    pub confidence: f64,
    pub stability: f64,
    pub transition_risk: String,
}

impl Default for RegimeState {
    fn default() -> Self {
        Self {
            current: StabilityRegime::Neutral,
            confidence: 0.0,
            stability: 0.0,
            transition_risk: "low".to_string(),
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
