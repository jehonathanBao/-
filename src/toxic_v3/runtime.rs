use tokio::sync::mpsc;

use super::{
    feature_store::FeatureVector,
    signal::{DecisionEngine, SignalAggregator, SignalEvent},
    types::{MarketFlowTick, SignalSource},
};

#[derive(Debug, Clone)]
pub struct ExecutionRouter {
    tx: mpsc::Sender<SignalEvent>,
}

impl ExecutionRouter {
    pub fn new(tx: mpsc::Sender<SignalEvent>) -> Self {
        Self { tx }
    }

    pub async fn dispatch(
        &self,
        signal: SignalEvent,
    ) -> Result<(), mpsc::error::SendError<SignalEvent>> {
        self.tx.send(signal).await
    }
}

#[derive(Debug, Clone)]
pub struct FlowInferenceEngine {
    pub source: SignalSource,
    pub data_quality: f64,
    pub decision: DecisionEngine,
    router: ExecutionRouter,
}

impl FlowInferenceEngine {
    pub fn new(
        source: SignalSource,
        data_quality: f64,
        decision: DecisionEngine,
        router: ExecutionRouter,
    ) -> Self {
        Self {
            source,
            data_quality,
            decision,
            router,
        }
    }

    pub fn evaluate(&self, tick: &MarketFlowTick) -> SignalEvent {
        SignalAggregator::evaluate_tick(tick, self.source, self.data_quality, &self.decision)
    }

    pub fn evaluate_feature_vector(&self, vector: &FeatureVector) -> SignalEvent {
        SignalAggregator::evaluate_vector(vector, self.source, self.data_quality, &self.decision)
    }

    pub async fn process_tick(
        &self,
        tick: &MarketFlowTick,
    ) -> Result<SignalEvent, mpsc::error::SendError<SignalEvent>> {
        let signal = self.evaluate(tick);
        self.router.dispatch(signal.clone()).await?;
        Ok(signal)
    }

    pub async fn process_feature_vector(
        &self,
        vector: &FeatureVector,
    ) -> Result<SignalEvent, mpsc::error::SendError<SignalEvent>> {
        let signal = self.evaluate_feature_vector(vector);
        self.router.dispatch(signal.clone()).await?;
        Ok(signal)
    }

    pub async fn run(
        self,
        mut rx: mpsc::Receiver<MarketFlowTick>,
    ) -> Result<(), mpsc::error::SendError<SignalEvent>> {
        while let Some(tick) = rx.recv().await {
            let signal = self.evaluate(&tick);
            self.router.dispatch(signal).await?;
        }
        Ok(())
    }
}

pub async fn inference_loop(
    rx: mpsc::Receiver<MarketFlowTick>,
    router: ExecutionRouter,
) -> Result<(), mpsc::error::SendError<SignalEvent>> {
    FlowInferenceEngine::new(
        SignalSource::FlowInference,
        100.0,
        DecisionEngine::default(),
        router,
    )
    .run(rx)
    .await
}
