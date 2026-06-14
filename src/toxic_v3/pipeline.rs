use tokio::sync::mpsc;

use super::{
    feature_builder::FeatureBuilder,
    feature_store::{FeatureStore, FeatureVector},
    runtime::FlowInferenceEngine,
    signal::SignalEvent,
    signal_store::SignalStore,
    types::MarketFlowTick,
};

#[derive(Debug, Clone)]
pub struct InferenceBus {
    tx: mpsc::Sender<MarketFlowTick>,
}

impl InferenceBus {
    pub fn new(tx: mpsc::Sender<MarketFlowTick>) -> Self {
        Self { tx }
    }

    pub async fn publish(
        &self,
        tick: MarketFlowTick,
    ) -> Result<(), mpsc::error::SendError<MarketFlowTick>> {
        self.tx.send(tick).await
    }
}

#[derive(Debug, Clone)]
pub struct ProductionFlowPipeline<S> {
    engine: FlowInferenceEngine,
    feature_store: S,
}

impl<S> ProductionFlowPipeline<S>
where
    S: FeatureStore,
{
    pub fn new(engine: FlowInferenceEngine, feature_store: S) -> Self {
        Self {
            engine,
            feature_store,
        }
    }

    pub fn feature_vector(&self, tick: MarketFlowTick) -> FeatureVector {
        FeatureBuilder::build_from_store(&self.feature_store, tick)
    }

    pub async fn process_tick(
        &mut self,
        tick: MarketFlowTick,
    ) -> Result<SignalEvent, mpsc::error::SendError<SignalEvent>> {
        let vector = FeatureBuilder::build_from_store(&self.feature_store, tick.clone());
        self.feature_store.update(&tick);
        self.engine.process_feature_vector(&vector).await
    }

    pub fn rolling_feature_vector(&self, tick: MarketFlowTick) -> FeatureVector {
        self.feature_store.feature_vector(tick)
    }
}

#[derive(Debug, Clone)]
pub struct RecordingProductionFlowPipeline<F, S> {
    pipeline: ProductionFlowPipeline<F>,
    signal_store: S,
}

impl<F, S> RecordingProductionFlowPipeline<F, S>
where
    F: FeatureStore,
    S: SignalStore,
{
    pub fn new(pipeline: ProductionFlowPipeline<F>, signal_store: S) -> Self {
        Self {
            pipeline,
            signal_store,
        }
    }

    pub async fn process_tick(
        &mut self,
        tick: MarketFlowTick,
    ) -> Result<SignalEvent, mpsc::error::SendError<SignalEvent>> {
        let signal = self.pipeline.process_tick(tick).await?;
        self.signal_store.record(&signal);
        Ok(signal)
    }

    pub fn recent_signals(&self, limit: usize) -> Vec<SignalEvent> {
        self.signal_store.recent(limit)
    }
}
