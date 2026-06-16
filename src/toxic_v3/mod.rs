//! Read-only game-theoretic market-flow inference layer.
//!
//! This module never places orders, cancels orders, signs requests, reads
//! private exchange streams, transfers funds, or changes Discord/Telegram
//! gates. It only converts already-observed public market-flow features into
//! enrichment metadata that upstream monitors may display or persist.

pub mod adaptive;
pub mod btc_liquidation;
pub mod btc_liquidation_dashboard;
pub mod enrichment;
pub mod evaluation;
pub mod feature_builder;
pub mod feature_store;
pub mod flow_reality;
pub mod gex;
pub mod glce;
pub mod hazard;
pub mod intent;
pub mod lhcs;
pub mod mff;
pub mod pipeline;
pub mod runtime;
pub mod signal;
pub mod signal_store;
pub mod stealth;
pub mod trajectory;
pub mod types;

pub use adaptive::{
    AdaptiveAdjustment, AdaptiveController, AdaptiveEngine, AdaptiveParameters, FeedbackSignal,
};
pub use btc_liquidation::{BTCLiquidationEngine, BTCLiquidationState};
pub use btc_liquidation_dashboard::{
    build_btc_liquidation_dashboard, BTCLiquidationDashboard, CascadeTimelinePoint,
    DashboardDataSources, DashboardForceFieldState, DashboardGammaWall, DashboardLiquidityLevel,
    LiqLevel, MarketStressOverview, SqueezeDirectionPanel,
};
pub use enrichment::{enrich_signal, ToxicV3SignalInput};
pub use evaluation::{
    EvaluationEngine, SystemEvaluationSample, SystemEvaluationState, SystemEvaluationVerdict,
    SystemHistory,
};
pub use feature_builder::FeatureBuilder;
pub use feature_store::{FeatureStore, FeatureVector, FlowStats, InMemoryFeatureStore};
pub use gex::{DealerBias, GEXEngine, GammaExposureState, GammaWall, OptionStrike, OptionsSurface};
pub use glce::{BreakoutBias, GLCEEngine, GLCEState, PriceLevel};
pub use hazard::HazardEngine;
pub use intent::IntentEngine;
pub use lhcs::{
    CascadeDirection, CascadeState, CascadeStep, LHCSEngine, LHCSState, LiquidationHeatmap,
    PriceBin, PriceLevelTrigger, PriceZone,
};
pub use mff::{MarketForceField, MarketForceFieldEngine, MarketRegime};
pub use pipeline::{InferenceBus, ProductionFlowPipeline, RecordingProductionFlowPipeline};
pub use runtime::{inference_loop, ExecutionRouter, FlowInferenceEngine};
pub use signal::{DecisionEngine, SignalAggregator, SignalEvent, SignalType};
pub use signal_store::{InMemorySignalStore, SignalStore};
pub use stealth::StealthEngine;
pub use types::{
    Direction, HazardState, HazardStateKind, IntentState, IntentType, MarketFlowExchange,
    MarketFlowTick, SignalSource, StealthFeatures, StealthRegime, StealthState, ToxicV3Enrichment,
    TrajectoryState, TrajectoryStateKind,
};
