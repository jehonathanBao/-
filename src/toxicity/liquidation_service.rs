use std::sync::Arc;

use parking_lot::RwLock;

use crate::{
    config::{thresholds::LiquidationClusterParams, AppConfig},
    market_data::flow_window_service::FlowWindowService,
    normalizers::trade::now_ms,
    toxicity::{
        liquidation_cluster_engine::LiquidationClusterEngine, sweep_service::SweepService,
        vpin_service::VpinService,
    },
    types::liquidation::{empty_liquidation_state, LiquidationState},
};

#[derive(Clone)]
pub struct LiquidationService {
    flow_service: FlowWindowService,
    sweep_service: SweepService,
    vpin_service: VpinService,
    engine: LiquidationClusterEngine,
    lookback_ms: i64,
    compute_interval_ms: u64,
    latest_state: Arc<RwLock<LiquidationState>>,
    task: Arc<RwLock<Option<tokio::task::JoinHandle<()>>>>,
}

impl LiquidationService {
    pub fn new(
        flow_service: FlowWindowService,
        sweep_service: SweepService,
        vpin_service: VpinService,
        config: &AppConfig,
    ) -> Self {
        let params = LiquidationClusterParams {
            enabled: config.liquidation_enabled,
            lookback_ms: config.liquidation_lookback_ms,
            cluster_band_bps: config.liquidation_cluster_band_bps,
            min_cluster_distance_bps: config.liquidation_min_cluster_distance_bps,
            max_cluster_distance_bps: config.liquidation_max_cluster_distance_bps,
            proximity_threshold_bps: config.liquidation_proximity_threshold_bps,
            min_touches: config.liquidation_min_cluster_touches,
            pressure_threshold: config.liquidation_pressure_threshold,
        };

        Self {
            flow_service,
            sweep_service,
            vpin_service,
            engine: LiquidationClusterEngine::new(params.clone()),
            lookback_ms: params.lookback_ms,
            compute_interval_ms: config.toxic_compute_interval_ms,
            latest_state: Arc::new(RwLock::new(empty_liquidation_state(now_ms()))),
            task: Arc::new(RwLock::new(None)),
        }
    }

    pub fn start(&self) {
        if self.task.read().is_some() {
            return;
        }

        let service = self.clone();
        let handle = tokio::spawn(async move {
            let mut interval = tokio::time::interval(std::time::Duration::from_millis(
                service.compute_interval_ms,
            ));
            loop {
                interval.tick().await;
                service.compute_once(now_ms());
            }
        });
        *self.task.write() = Some(handle);
    }

    pub fn stop(&self) {
        if let Some(handle) = self.task.write().take() {
            handle.abort();
        }
    }

    pub fn get_state(&self) -> LiquidationState {
        self.latest_state.read().clone()
    }

    pub fn compute_once_for_tests(&self, now_ts: i64) -> LiquidationState {
        self.compute_once(now_ts)
    }

    fn compute_once(&self, now_ts: i64) -> LiquidationState {
        let snapshots = self
            .flow_service
            .get_price_snapshots_since(now_ts - self.lookback_ms);
        let state = self.engine.compute(
            now_ts,
            &self.flow_service.get_latest_flow_state(),
            &self.sweep_service.get_state(),
            &self.vpin_service.get_state(),
            &snapshots,
        );
        *self.latest_state.write() = state.clone();
        state
    }
}
