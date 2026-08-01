use std::sync::Arc;

use parking_lot::RwLock;

use crate::{
    config::{thresholds::LiqHuntParams, AppConfig},
    market_data::flow_window_service::FlowWindowService,
    normalizers::trade::now_ms,
    regime_thresholds::RegimeThresholdManager,
    toxicity::{
        liq_hunt_detector::{LiqHuntDetector, LiqHuntDetectorInput},
        liquidation_service::LiquidationService,
        sweep_service::SweepService,
        toxic_service::ToxicService,
        vpin_service::VpinService,
    },
    types::liq_hunt::{empty_liq_hunt_state, LiqHuntResult, LiqHuntState},
};

#[derive(Clone)]
pub struct LiqHuntService {
    flow_service: FlowWindowService,
    toxic_service: ToxicService,
    vpin_service: VpinService,
    sweep_service: SweepService,
    liquidation_service: LiquidationService,
    base_params: LiqHuntParams,
    detector: LiqHuntDetector,
    regime_manager: Arc<RegimeThresholdManager>,
    compute_interval_ms: u64,
    recent_result_limit: usize,
    latest_state: Arc<RwLock<LiqHuntState>>,
    recent_results: Arc<RwLock<Vec<LiqHuntResult>>>,
    task: Arc<RwLock<Option<tokio::task::JoinHandle<()>>>>,
    symbol: String,
}

impl LiqHuntService {
    pub fn new(
        flow_service: FlowWindowService,
        toxic_service: ToxicService,
        vpin_service: VpinService,
        sweep_service: SweepService,
        liquidation_service: LiquidationService,
        config: &AppConfig,
    ) -> Self {
        Self::new_with_regime(
            flow_service,
            toxic_service,
            vpin_service,
            sweep_service,
            liquidation_service,
            config,
            Arc::new(RegimeThresholdManager::from_runtime_config()),
        )
    }

    pub fn new_with_regime(
        flow_service: FlowWindowService,
        toxic_service: ToxicService,
        vpin_service: VpinService,
        sweep_service: SweepService,
        liquidation_service: LiquidationService,
        config: &AppConfig,
        regime_manager: Arc<RegimeThresholdManager>,
    ) -> Self {
        let params = LiqHuntParams {
            cluster_large_notional_usd: config.liq_hunt_cluster_large_notional_usd,
            near_distance_bps: config.liq_hunt_near_distance_bps,
            active_score: config.liq_hunt_active_score,
            likely_score: config.liq_hunt_likely_score,
            watch_score: config.liq_hunt_watch_score,
            ..LiqHuntParams::default()
        };

        Self {
            flow_service,
            toxic_service,
            vpin_service,
            sweep_service,
            liquidation_service,
            base_params: params.clone(),
            detector: LiqHuntDetector::new(params.clone()),
            regime_manager,
            compute_interval_ms: config.toxic_compute_interval_ms,
            recent_result_limit: params.recent_result_limit,
            latest_state: Arc::new(RwLock::new(empty_liq_hunt_state(now_ms()))),
            recent_results: Arc::new(RwLock::new(Vec::new())),
            task: Arc::new(RwLock::new(None)),
            symbol: config.symbol.clone(),
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

    pub fn get_state(&self) -> LiqHuntState {
        self.latest_state.read().clone()
    }

    pub fn compute_once_for_tests(&self, now_ts: i64) -> LiqHuntState {
        self.compute_once(now_ts)
    }

    fn compute_once(&self, now_ts: i64) -> LiqHuntState {
        let params = self
            .regime_manager
            .adjusted_liq_hunt_params(&self.base_params);
        let result = self
            .detector
            .with_params(params)
            .detect(LiqHuntDetectorInput {
                now_ts,
                symbol: self.symbol.clone(),
                toxic_state: self.toxic_service.get_state(),
                vpin_state: Some(self.vpin_service.get_state()),
                sweep_state: self.sweep_service.get_state(),
                liquidation_state: self.liquidation_service.get_state(),
                flow_state: self.flow_service.latest_state_for_symbol(&self.symbol),
            });

        let mut recent_results = self.recent_results.write();
        recent_results.push(result.clone());
        if recent_results.len() > self.recent_result_limit {
            let overflow = recent_results.len() - self.recent_result_limit;
            recent_results.drain(0..overflow);
        }

        let state = LiqHuntState {
            symbol: self.symbol.clone(),
            updated_at: now_ts,
            result,
            recent_results: recent_results.clone(),
        };
        *self.latest_state.write() = state.clone();
        state
    }
}
