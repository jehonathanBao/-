//! Read-only game-theoretic market-flow inference layer.
//!
//! This module never places orders, cancels orders, signs requests, reads
//! private exchange streams, transfers funds, or changes Discord/Telegram
//! gates. It only converts already-observed public market-flow features into
//! enrichment metadata that upstream monitors may display or persist.

pub mod enrichment;
pub mod feature_builder;
pub mod feature_store;
pub mod flow_reality;
pub mod hazard;
pub mod intent;
pub mod pipeline;
pub mod runtime;
pub mod signal;
pub mod signal_store;
pub mod stealth;
pub mod trajectory;
pub mod types;

pub use enrichment::{enrich_signal, ToxicV3SignalInput};
pub use feature_builder::FeatureBuilder;
pub use feature_store::{FeatureStore, FeatureVector, FlowStats, InMemoryFeatureStore};
pub use hazard::HazardEngine;
pub use intent::IntentEngine;
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
