use std::sync::{
    atomic::{AtomicBool, Ordering},
    Arc,
};

use parking_lot::RwLock;

use crate::{
    alerts::{
        alert_service::{AlertService, DevTestSidecarAlertInput, DevTestSidecarAlertResult},
        alert_types::AlertState,
    },
    config::AppConfig,
    connectors::manager::ConnectorManager,
    market_data::{event_bus::MarketDataBus, flow_window_service::FlowWindowService},
    storage::{snapshot_service::StorageState, SnapshotService, SqliteStore},
    toxicity::{
        liq_hunt_service::LiqHuntService, liquidation_service::LiquidationService,
        markout_service::MarkoutService,
        orderbook_wall_lifecycle_service::OrderbookWallLifecycleService,
        sweep_service::SweepService, toxic_service::ToxicService,
        toxic_signal_history_service::ToxicSignalHistoryService, vpin_service::VpinService,
        whale_flow_candidate_history_service::WhaleFlowCandidateHistoryService,
    },
    types::{
        flow::FlowState,
        liq_hunt::LiqHuntState,
        liquidation::LiquidationState,
        market::{NormalizedTrade, Venue, VenueHealth},
        markout::MarkoutState,
        orderbook_wall::OrderbookWallLifecycleState,
        status::{
            RuntimeControlSummary, RuntimeStartResult, RuntimeStartState, RuntimeStopResult,
            RuntimeStopState, VenueHealthMap,
        },
        sweep::SweepState,
        toxic::{ToxicSeverity, ToxicState},
        vpin::VpinState,
    },
};

#[derive(Clone)]
pub struct AppState {
    inner: Arc<AppStateInner>,
}

struct AppStateInner {
    config: AppConfig,
    runtime_started: AtomicBool,
    runtime_control: Arc<RwLock<RuntimeControlTracker>>,
    market_data_bus: MarketDataBus,
    connector_manager: ConnectorManager,
    flow_service: FlowWindowService,
    markout_service: MarkoutService,
    sweep_service: SweepService,
    vpin_service: VpinService,
    liquidation_service: LiquidationService,
    liq_hunt_service: LiqHuntService,
    toxic_service: ToxicService,
    orderbook_wall_lifecycle_service: OrderbookWallLifecycleService,
    alert_service: AlertService,
    snapshot_service: SnapshotService,
    signal_history_service: ToxicSignalHistoryService,
    whale_flow_candidate_history_service: WhaleFlowCandidateHistoryService,
}

#[derive(Debug, Clone)]
struct RuntimeControlTracker {
    start_state: RuntimeStartState,
    last_start_at_ms: Option<i64>,
    last_start_error: Option<String>,
    start_attempt_count: u64,
    last_start_result: RuntimeStartResult,
    forced_start_failure: Option<String>,
    stop_state: RuntimeStopState,
    last_stop_at_ms: Option<i64>,
    last_stop_error: Option<String>,
    stop_attempt_count: u64,
    last_stop_result: RuntimeStopResult,
    forced_stop_failure: Option<String>,
}

impl RuntimeControlTracker {
    fn new() -> Self {
        Self {
            start_state: RuntimeStartState::Stopped,
            last_start_at_ms: None,
            last_start_error: None,
            start_attempt_count: 0,
            last_start_result: RuntimeStartResult::None,
            forced_start_failure: None,
            stop_state: RuntimeStopState::Stopped,
            last_stop_at_ms: None,
            last_stop_error: None,
            stop_attempt_count: 0,
            last_stop_result: RuntimeStopResult::None,
            forced_stop_failure: None,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct StartMonitoringOutcome {
    pub runtime_modified: bool,
    pub start_state: RuntimeStartState,
    pub result: RuntimeStartResult,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct StopMonitoringOutcome {
    pub runtime_modified: bool,
    pub stop_state: RuntimeStopState,
    pub result: RuntimeStopResult,
}

impl AppState {
    pub fn new(config: AppConfig) -> Self {
        let bus = MarketDataBus::new(4096);
        let flow_service = FlowWindowService::new(bus.clone(), &config);
        let markout_service = MarkoutService::new(bus.clone(), flow_service.clone(), &config);
        let sweep_service = SweepService::new(flow_service.clone(), &config);
        let shared_store = if config.sqlite_enabled {
            SqliteStore::open(&config.sqlite_path)
                .and_then(|store| {
                    store.migrate()?;
                    Ok(store)
                })
                .ok()
        } else {
            None
        };
        let vpin_service = VpinService::new(bus.clone(), &config, shared_store.clone());
        let liquidation_service = LiquidationService::new(
            flow_service.clone(),
            sweep_service.clone(),
            vpin_service.clone(),
            &config,
        );
        let toxic_service = ToxicService::new(
            flow_service.clone(),
            markout_service.clone(),
            sweep_service.clone(),
            vpin_service.clone(),
            liquidation_service.clone(),
            &config,
        );
        let liq_hunt_service = LiqHuntService::new(
            flow_service.clone(),
            toxic_service.clone(),
            vpin_service.clone(),
            sweep_service.clone(),
            liquidation_service.clone(),
            &config,
        );
        let orderbook_wall_lifecycle_service =
            OrderbookWallLifecycleService::new(bus.clone(), config.symbol.clone());
        let alert_service = AlertService::new(Arc::new(toxic_service.clone()), &config);
        let connector_manager = ConnectorManager::new(bus.clone(), &config);
        let snapshot_service = SnapshotService::new(
            config.sqlite_enabled,
            config.sqlite_path.clone(),
            config.snapshot_persist_interval_ms,
            shared_store.or_else(|| toxic_service.store()),
            flow_service.clone(),
            toxic_service.clone(),
            connector_manager.clone(),
        );
        let signal_history_service = ToxicSignalHistoryService::default();
        let whale_flow_candidate_history_service = WhaleFlowCandidateHistoryService::default();

        Self {
            inner: Arc::new(AppStateInner {
                config,
                runtime_started: AtomicBool::new(false),
                runtime_control: Arc::new(RwLock::new(RuntimeControlTracker::new())),
                market_data_bus: bus,
                connector_manager,
                flow_service,
                markout_service,
                sweep_service,
                vpin_service,
                liquidation_service,
                liq_hunt_service,
                toxic_service,
                orderbook_wall_lifecycle_service,
                alert_service,
                snapshot_service,
                signal_history_service,
                whale_flow_candidate_history_service,
            }),
        }
    }

    pub async fn start(&self) {
        let _ = self.ensure_monitoring_started().await;
    }

    pub async fn ensure_monitoring_started(&self) -> StartMonitoringOutcome {
        {
            let mut runtime_control = self.inner.runtime_control.write();
            runtime_control.start_attempt_count += 1;
            if self.inner.runtime_started.load(Ordering::SeqCst) {
                runtime_control.start_state = RuntimeStartState::Started;
                runtime_control.last_start_result = RuntimeStartResult::AlreadyStarted;
                runtime_control.last_start_error = None;
                return StartMonitoringOutcome {
                    runtime_modified: false,
                    start_state: RuntimeStartState::Started,
                    result: RuntimeStartResult::AlreadyStarted,
                };
            }

            runtime_control.start_state = RuntimeStartState::Starting;
            runtime_control.last_start_error = None;
        }

        let forced_start_failure = {
            self.inner
                .runtime_control
                .read()
                .forced_start_failure
                .clone()
        };
        if let Some(error) = forced_start_failure {
            let mut runtime_control = self.inner.runtime_control.write();
            self.inner.runtime_started.store(false, Ordering::SeqCst);
            runtime_control.start_state = RuntimeStartState::Failed;
            runtime_control.last_start_result = RuntimeStartResult::Failed;
            runtime_control.last_start_error = Some(error);
            return StartMonitoringOutcome {
                runtime_modified: false,
                start_state: RuntimeStartState::Failed,
                result: RuntimeStartResult::Failed,
            };
        }
        self.inner.runtime_started.store(true, Ordering::SeqCst);
        self.inner.flow_service.start();
        self.inner.markout_service.start();
        self.inner.sweep_service.start();
        self.inner.vpin_service.start();
        self.inner.liquidation_service.start();
        self.inner.toxic_service.start();
        self.inner.liq_hunt_service.start();
        self.inner.orderbook_wall_lifecycle_service.start();
        self.inner.alert_service.start();
        self.inner.snapshot_service.start();
        self.inner.connector_manager.start_all().await;
        let mut runtime_control = self.inner.runtime_control.write();
        runtime_control.start_state = RuntimeStartState::Started;
        runtime_control.last_start_at_ms = Some(crate::normalizers::trade::now_ms());
        runtime_control.last_start_error = None;
        runtime_control.last_start_result = RuntimeStartResult::Started;
        StartMonitoringOutcome {
            runtime_modified: true,
            start_state: RuntimeStartState::Started,
            result: RuntimeStartResult::Started,
        }
    }

    pub async fn stop(&self) {
        let _ = self.ensure_monitoring_stopped().await;
    }

    pub async fn ensure_monitoring_stopped(&self) -> StopMonitoringOutcome {
        {
            let mut runtime_control = self.inner.runtime_control.write();
            runtime_control.stop_attempt_count += 1;
            if !self.inner.runtime_started.load(Ordering::SeqCst) {
                runtime_control.stop_state = RuntimeStopState::Stopped;
                runtime_control.last_stop_result = RuntimeStopResult::AlreadyStopped;
                runtime_control.last_stop_error = None;
                return StopMonitoringOutcome {
                    runtime_modified: false,
                    stop_state: RuntimeStopState::Stopped,
                    result: RuntimeStopResult::AlreadyStopped,
                };
            }

            runtime_control.stop_state = RuntimeStopState::Stopping;
            runtime_control.last_stop_error = None;
        }

        let forced_stop_failure = {
            self.inner
                .runtime_control
                .read()
                .forced_stop_failure
                .clone()
        };
        if let Some(error) = forced_stop_failure {
            let mut runtime_control = self.inner.runtime_control.write();
            runtime_control.stop_state = RuntimeStopState::Failed;
            runtime_control.last_stop_result = RuntimeStopResult::Failed;
            runtime_control.last_stop_error = Some(error);
            return StopMonitoringOutcome {
                runtime_modified: false,
                stop_state: RuntimeStopState::Failed,
                result: RuntimeStopResult::Failed,
            };
        }

        self.inner.runtime_started.store(false, Ordering::SeqCst);
        self.inner.connector_manager.stop_all().await;
        self.inner.snapshot_service.stop();
        self.inner.alert_service.stop();
        self.inner.orderbook_wall_lifecycle_service.stop();
        self.inner.liq_hunt_service.stop();
        self.inner.toxic_service.stop();
        self.inner.liquidation_service.stop();
        self.inner.vpin_service.stop();
        self.inner.sweep_service.stop();
        self.inner.markout_service.stop();
        self.inner.flow_service.stop();
        let mut runtime_control = self.inner.runtime_control.write();
        runtime_control.start_state = RuntimeStartState::Stopped;
        runtime_control.stop_state = RuntimeStopState::Stopped;
        runtime_control.last_stop_at_ms = Some(crate::normalizers::trade::now_ms());
        runtime_control.last_stop_error = None;
        runtime_control.last_stop_result = RuntimeStopResult::Stopped;
        StopMonitoringOutcome {
            runtime_modified: true,
            stop_state: RuntimeStopState::Stopped,
            result: RuntimeStopResult::Stopped,
        }
    }

    pub fn config(&self) -> &AppConfig {
        &self.inner.config
    }

    pub fn runtime_started(&self) -> bool {
        self.inner.runtime_started.load(Ordering::SeqCst)
    }

    pub fn runtime_control_summary(&self) -> RuntimeControlSummary {
        let runtime_control = self.inner.runtime_control.read();
        RuntimeControlSummary {
            monitoring_started: self.runtime_started(),
            one_click_start_enabled: true,
            start_action_label: "One-click Start Monitoring",
            start_action_mode: "monitoring_only",
            start_state: runtime_control.start_state,
            last_start_at_ms: runtime_control.last_start_at_ms,
            last_start_error: runtime_control.last_start_error.clone(),
            start_attempt_count: runtime_control.start_attempt_count,
            last_start_result: runtime_control.last_start_result,
            stop_state: runtime_control.stop_state,
            last_stop_at_ms: runtime_control.last_stop_at_ms,
            last_stop_error: runtime_control.last_stop_error.clone(),
            stop_attempt_count: runtime_control.stop_attempt_count,
            last_stop_result: runtime_control.last_stop_result,
        }
    }

    pub fn venue_health(&self) -> VenueHealthMap {
        self.inner.connector_manager.get_venue_health()
    }

    pub fn flow_state(&self) -> FlowState {
        self.inner.flow_service.latest_state()
    }

    pub fn market_data_quality(&self) -> crate::market_data::quality::MarketDataQualityTracker {
        self.inner.market_data_bus.quality_tracker()
    }

    pub fn markout_state(&self) -> MarkoutState {
        self.inner.markout_service.get_state()
    }

    pub fn sweep_state(&self) -> SweepState {
        self.inner.sweep_service.get_state()
    }

    pub fn toxic_state(&self) -> ToxicState {
        self.inner.toxic_service.get_state()
    }

    pub fn liquidation_state(&self) -> LiquidationState {
        self.inner.liquidation_service.get_state()
    }

    pub fn vpin_state(&self) -> VpinState {
        self.inner.vpin_service.get_state()
    }

    pub fn liq_hunt_state(&self) -> LiqHuntState {
        self.inner.liq_hunt_service.get_state()
    }

    pub fn orderbook_wall_lifecycle_state(&self) -> OrderbookWallLifecycleState {
        self.inner.orderbook_wall_lifecycle_service.get_state()
    }

    pub fn alert_state(&self) -> AlertState {
        self.inner.alert_service.get_state()
    }

    pub fn emit_runtime_acceptance_test_sidecar_alert(
        &self,
        severity: ToxicSeverity,
        venue: Venue,
        symbol: String,
        dedupe_suffix: String,
    ) -> anyhow::Result<DevTestSidecarAlertResult> {
        self.inner.alert_service.emit_runtime_acceptance_test_alert(
            crate::normalizers::trade::now_ms(),
            &DevTestSidecarAlertInput {
                severity,
                venue,
                symbol,
                dedupe_suffix,
            },
        )
    }

    pub fn storage_state(&self) -> StorageState {
        self.inner.snapshot_service.get_state()
    }

    pub fn signal_history_service(&self) -> ToxicSignalHistoryService {
        self.inner.signal_history_service.clone()
    }

    pub fn whale_flow_candidate_history_service(&self) -> WhaleFlowCandidateHistoryService {
        self.inner.whale_flow_candidate_history_service.clone()
    }

    pub fn recent_toxic_events(
        &self,
        limit: usize,
    ) -> anyhow::Result<Vec<crate::types::toxic::ToxicEvent>> {
        self.inner.toxic_service.list_recent_events(limit)
    }

    pub fn latest_toxic_event(&self) -> anyhow::Result<Option<crate::types::toxic::ToxicEvent>> {
        self.inner.toxic_service.get_latest_event()
    }

    pub fn recent_vpin_buckets(
        &self,
        limit: usize,
    ) -> anyhow::Result<Vec<crate::types::vpin::VpinBucket>> {
        self.inner.vpin_service.recent_buckets(limit)
    }

    pub fn set_health_for_tests(&self, health: VenueHealth) {
        self.inner.connector_manager.set_health_for_tests(health);
    }

    pub fn ingest_trade_event_for_tests(&self, trade: NormalizedTrade) {
        self.inner
            .connector_manager
            .ingest_trade_event_for_tests(trade);
    }

    pub fn shared_flow_for_tests(&self) -> Arc<RwLock<FlowState>> {
        self.inner.flow_service.shared_state()
    }

    pub fn flow_service_for_tests(&self) -> FlowWindowService {
        self.inner.flow_service.clone()
    }

    pub fn price_snapshot_at_or_before(
        &self,
        ts: i64,
    ) -> Option<crate::market_data::price_index::PriceSnapshot> {
        self.inner.flow_service.get_price_snapshot_at_or_before(ts)
    }

    pub fn price_snapshots_since(
        &self,
        ts: i64,
    ) -> Vec<crate::market_data::price_index::PriceSnapshot> {
        self.inner.flow_service.get_price_snapshots_since(ts)
    }

    pub fn shared_markout_engine_for_tests(
        &self,
    ) -> Arc<RwLock<crate::toxicity::markout_engine::MarkoutEngine>> {
        self.inner.markout_service.shared_engine_for_tests()
    }

    pub fn sweep_service_for_tests(&self) -> SweepService {
        self.inner.sweep_service.clone()
    }

    pub fn vpin_service_for_tests(&self) -> VpinService {
        self.inner.vpin_service.clone()
    }

    pub fn liquidation_service_for_tests(&self) -> LiquidationService {
        self.inner.liquidation_service.clone()
    }

    pub fn liq_hunt_service_for_tests(&self) -> LiqHuntService {
        self.inner.liq_hunt_service.clone()
    }

    pub fn orderbook_wall_lifecycle_service_for_tests(&self) -> OrderbookWallLifecycleService {
        self.inner.orderbook_wall_lifecycle_service.clone()
    }

    pub fn toxic_service_for_tests(&self) -> ToxicService {
        self.inner.toxic_service.clone()
    }

    pub fn alert_service_for_tests(&self) -> AlertService {
        self.inner.alert_service.clone()
    }

    pub fn snapshot_service_for_tests(&self) -> SnapshotService {
        self.inner.snapshot_service.clone()
    }

    pub fn signal_history_service_for_tests(&self) -> ToxicSignalHistoryService {
        self.inner.signal_history_service.clone()
    }

    pub fn whale_flow_candidate_history_service_for_tests(
        &self,
    ) -> WhaleFlowCandidateHistoryService {
        self.inner.whale_flow_candidate_history_service.clone()
    }

    pub fn set_start_failure_for_tests(&self, error: Option<String>) {
        self.inner.runtime_control.write().forced_start_failure = error;
    }

    pub fn set_stop_failure_for_tests(&self, error: Option<String>) {
        self.inner.runtime_control.write().forced_stop_failure = error;
    }
}
