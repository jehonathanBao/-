use super::{
    feature_store::{FeatureStore, FeatureVector, FlowStats},
    types::MarketFlowTick,
};

pub struct FeatureBuilder;

impl FeatureBuilder {
    pub fn build(tick: MarketFlowTick, stats: FlowStats) -> FeatureVector {
        FeatureVector::from_tick_and_stats(tick, stats)
    }

    pub fn build_from_store<S>(store: &S, tick: MarketFlowTick) -> FeatureVector
    where
        S: FeatureStore,
    {
        let stats = store.rolling_stats(&tick.symbol);
        Self::build(tick, stats)
    }
}
