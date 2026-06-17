//! Read-only new-token contract-flow watcher.
//!
//! This module manages an operator-selected list of up to 10 symbols and turns
//! public contract-flow observations into candidate behavior labels. It never
//! places orders, cancels orders, reads private streams, signs requests, or
//! mutates exchange state.

pub mod adapter;
pub mod collector;
pub mod engine;
pub mod manager;
pub mod types;

pub use adapter::NewTokenSignalAdapter;
pub use collector::ContractFlowCollector;
pub use engine::NewTokenFlowEngine;
pub use manager::{normalize_symbol, TokenWatchError, TokenWatchManager};
pub use types::{
    AdvisoryDirection, BehaviorProbabilities, BehaviorWindowMetrics, CapitalPhase,
    CapitalStructureView, CapitalTimeline, CapitalTimelinePhase, ContractTick, ContractTickSide,
    CostBasisEstimate, CostDistributionBand, DecisionOrderType, DecisionTiming, DistributionRisk,
    EstimatedPositionSize, ExpectedHoldTime, FlowActorRegime, ForcedFlowAttribution,
    LastAccumulationNode, LatentPositionPoint, LiquidationZone, LiquidityForceState,
    LiquidityReactionMap, LiquidityVacuumZone, MarketDynamicsState, MarketEnergy,
    MarketStateVector, MarketStateVelocity, NewTokenWatchMutationResponse, NewTokenWatchRequest,
    PhaseTimelineSegment, PositionFlowCurve, PositionFlowPoint, PositionPathSegment,
    PositionSmoothing, PositionValidityGate, PriceImpactDecomposition, RegimeTransitionProbability,
    SignalCompressionState, SmartLevel, SmartMoneyChartResponse, SmartMoneyDecomposition,
    SmartMoneyPositionReconstruction, SmartMoneyReconstructionResponse, StabilityRegime,
    StopLossCascadeState, TimeHorizonInference, TokenChartMarker, TokenChartPoint, TokenFlowRegime,
    TokenFlowSignal, TokenWatchItem, TokenWatchListResponse, TradeSignalAdvisory,
    TradingDecisionEntry, TradingDecisionExit, TradingDecisionKernel, TradingInvalidation,
    TradingPositionSize, TradingStabilityKernel, MAX_ACTIVE_TOKENS,
};
