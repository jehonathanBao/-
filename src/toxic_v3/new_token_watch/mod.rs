//! Read-only new-token contract-flow watcher.
//!
//! This module manages an operator-selected list of up to 50 symbols and turns
//! public contract-flow observations into candidate behavior labels. It never
//! places orders, cancels orders, reads private streams, signs requests, or
//! mutates exchange state.

pub mod adapter;
pub mod engine;
pub mod intent;
pub mod l2;
pub mod manager;
pub mod market_truth;
pub mod runtime;
pub mod session;
pub mod shadow;
pub mod types;
pub mod walls;

pub use adapter::NewTokenSignalAdapter;
pub use engine::NewTokenFlowEngine;
pub use manager::{normalize_symbol, TokenWatchError, TokenWatchManager};
pub use market_truth::fetch_market_price_snapshot;
pub use types::{
    AdvisoryDirection, BehaviorProbabilities, BehaviorWindowMetrics, CapitalPhase,
    CapitalStructureView, CapitalTimeline, CapitalTimelinePhase, ContractTick, ContractTickSide,
    CostBasisEstimate, CostDistributionBand, DecisionOrderType, DecisionTiming, DistributionRisk,
    EstimatedPositionSize, ExecutionStrategyKernel, ExpectedHoldTime, FlowActorRegime,
    ForcedFlowAttribution, LastAccumulationNode, LatentPositionPoint, LiquidationZone,
    LiquidityForceState, LiquidityReactionMap, LiquidityVacuumZone, MarketDynamicsState,
    MarketEnergy, MarketPriceSnapshot, MarketStateVector, MarketStateVelocity,
    NewTokenWatchMutationResponse, NewTokenWatchRequest, PhaseTimelineSegment, PositionFlowCurve,
    PositionFlowPoint, PositionPathSegment, PositionSmoothing, PositionValidityGate,
    PriceImpactDecomposition, PriceSource, RegimeTransitionProbability, SignalCompressionState,
    SmartLevel, SmartMoneyChartResponse, SmartMoneyDecomposition, SmartMoneyPositionReconstruction,
    SmartMoneyReconstructionResponse, StabilityRegime, StopLossCascadeState, TimeHorizonInference,
    TokenChartMarker, TokenChartPoint, TokenFlowRegime, TokenFlowSignal, TokenWatchItem,
    TokenWatchListResponse, TradeSignalAdvisory, TradingDecisionEntry, TradingDecisionExit,
    TradingDecisionKernel, TradingInvalidation, TradingPositionSize, TradingStabilityKernel,
    MAX_ACTIVE_TOKENS,
};
