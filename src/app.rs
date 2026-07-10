use std::sync::{
    atomic::{AtomicBool, AtomicI64, AtomicU64, Ordering},
    Arc,
};

use axum::http::{header, HeaderMap};
use parking_lot::RwLock;

use crate::{
    alerts::{
        alert_service::{AlertService, DevTestSidecarAlertInput, DevTestSidecarAlertResult},
        alert_types::AlertState,
    },
    api::{
        contract_event_routes::FinalEventsV2Response,
        contract_whale_routes::{
            build_contract_whale_response_with_runtime_and_baselines, load_liquidation_contexts,
            load_market_context, load_quality_baselines, ContractWhaleResponseRuntime,
        },
        discord_notification_routes::{
            maybe_auto_push_discord, preferred_discord_alert_family, DiscordNotificationRequest,
        },
        toxic_signal_inbox_routes::build_recent,
        toxic_signal_ws_routes::{build_ws_snapshot, ToxicSignalWsItem},
    },
    binance_alt_contract_monitor::{
        config as bacm_config, service::BinanceAltContractService, LOG_PREFIX as BACM_LOG_PREFIX,
        LOG_TARGET as BACM_LOG_TARGET,
    },
    config::AppConfig,
    connectors::manager::ConnectorManager,
    contract_whale_monitor::{
        aggregator::aggregate_1s_buckets,
        collector_binance, collector_okx,
        config::contract_whale_runtime_config,
        discord_notifier::{
            evaluate_contract_whale_discord_gate, global_contract_whale_discord_cooldown_store,
            notify_contract_whale_discord, ContractWhaleDiscordSettings,
        },
        emission::{emission_key, fingerprint, should_emit},
        log_events as cwm_log_events,
        outcome_calibration::evaluate_contract_whale_signal_outcome,
        persistence::{
            flush_contract_flow_buckets_nonblocking,
            persist_contract_funding_snapshots_nonblocking,
            persist_contract_oi_snapshots_nonblocking, persist_contract_whale_signals_nonblocking,
            spawn_contract_whale_retention_task, ContractWhalePersistenceOutcome,
        },
        types::{
            ContractExchange, ContractFundingSnapshot, ContractOiSnapshot, ContractTrade,
            ContractTradeSide, ContractWhaleEmissionFingerprint, ContractWhaleMarketType,
        },
        LOG_PREFIX as CWM_LOG_PREFIX, LOG_TARGET as CWM_LOG_TARGET,
    },
    market_data::{event_bus::MarketDataBus, flow_window_service::FlowWindowService},
    runtime::main_force_events::best_main_force_event_observation,
    runtime::scan_log::{ScanLogItem, ScanLogStore},
    spot_whale_monitor::service::SpotWhaleService,
    storage::{
        contract_whale_repo::{
            ContractWhaleDiscordOutboxStatus, ContractWhaleRepo, ContractWhaleSignalQuery,
        },
        main_force_events_repo::MainForceEventsRepo,
        snapshot_service::StorageState,
        storage_health::{
            storage_health_guard_config, StorageHealthSnapshot, StorageHealthTracker,
        },
        SnapshotService, SqliteStore,
    },
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

#[derive(Debug, Clone, Default, serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ContractWhaleRuntimeDiagnostics {
    pub producer_loop: ContractWhaleProducerLoopDiagnostics,
    pub discord_queue: ContractWhaleDiscordQueueDiagnostics,
}

#[derive(Debug, Clone, Default, serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ContractWhaleProducerLoopDiagnostics {
    pub last_started_at: Option<i64>,
    pub last_completed_at: Option<i64>,
    pub last_duration_ms: Option<i64>,
    pub overlap_skipped: u64,
    pub missed_tick_policy: &'static str,
}

#[derive(Debug, Clone, Default, serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ContractWhaleDiscordQueueDiagnostics {
    pub pending: usize,
    pub retrying: usize,
    pub failed: usize,
    pub oldest_pending_age_sec: i64,
}

struct AppStateInner {
    config: AppConfig,
    booted_at_ms: i64,
    runtime_started: AtomicBool,
    runtime_control: Arc<RwLock<RuntimeControlTracker>>,
    discord_auto_push_task: Arc<RwLock<Option<tokio::task::JoinHandle<()>>>>,
    cwm_auto_push_task: Arc<RwLock<Option<tokio::task::JoinHandle<()>>>>,
    cwm_discord_outbox_task: Arc<RwLock<Option<tokio::task::JoinHandle<()>>>>,
    cwm_outcome_calibration_task: Arc<RwLock<Option<tokio::task::JoinHandle<()>>>>,
    cwm_market_context_task: Arc<RwLock<Option<tokio::task::JoinHandle<()>>>>,
    cwm_producer_running: AtomicBool,
    cwm_producer_last_started_at: AtomicI64,
    cwm_producer_last_completed_at: AtomicI64,
    cwm_producer_last_duration_ms: AtomicI64,
    cwm_producer_overlap_skipped: AtomicU64,
    cwm_emission_watermarks:
        Arc<RwLock<std::collections::BTreeMap<String, ContractWhaleEmissionFingerprint>>>,
    scan_log: ScanLogStore,
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
    storage_health: StorageHealthTracker,
    operator_api_token: Option<String>,
    contract_whale_store: Option<SqliteStore>,
    contract_whale_flow_flush_cursor_ms: Arc<RwLock<std::collections::BTreeMap<String, i64>>>,
    final_events_v2_cache:
        Arc<RwLock<std::collections::BTreeMap<String, CachedFinalEventsV2Entry>>>,
    signal_history_service: ToxicSignalHistoryService,
    whale_flow_candidate_history_service: WhaleFlowCandidateHistoryService,
    spot_whale_service: SpotWhaleService,
    binance_alt_contract_service: BinanceAltContractService,
}

#[derive(Debug, Clone)]
struct CachedFinalEventsV2Entry {
    cached_at_ms: i64,
    response: FinalEventsV2Response,
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
        let booted_at_ms = crate::normalizers::trade::now_ms();
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
        let shared_store_for_state = shared_store.clone();
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
        let storage_health = StorageHealthTracker::new(
            config
                .sqlite_enabled
                .then_some(std::path::PathBuf::from(config.sqlite_path.clone())),
            storage_health_guard_config(),
        );
        let snapshot_service = SnapshotService::new(
            config.sqlite_enabled,
            config.sqlite_path.clone(),
            config.snapshot_persist_interval_ms,
            shared_store.or_else(|| toxic_service.store()),
            flow_service.clone(),
            toxic_service.clone(),
            connector_manager.clone(),
            storage_health.clone(),
        );
        let contract_whale_store = shared_store_for_state.or_else(|| toxic_service.store());
        let signal_history_service = ToxicSignalHistoryService::default();
        let whale_flow_candidate_history_service = WhaleFlowCandidateHistoryService::default();
        let spot_whale_service = SpotWhaleService::new(
            config.spot_whale_monitor.enabled,
            config.spot_whale_monitor.dry_run,
            booted_at_ms,
            contract_whale_store.clone(),
        );
        let bacm_runtime_config = bacm_config::binance_alt_contract_runtime_config();
        let binance_alt_contract_service = BinanceAltContractService::new(
            bacm_runtime_config.enabled,
            bacm_runtime_config.dry_run,
            booted_at_ms,
        );
        let scan_log = ScanLogStore::new_from_env();
        scan_log.push(
            "info",
            "server_boot",
            "Runtime initialized in monitoring-only real-data capable mode",
            Some(config.symbol.clone()),
            None,
        );
        tracing::info!(
            target: BACM_LOG_TARGET,
            enabled = bacm_runtime_config.enabled,
            dry_run = bacm_runtime_config.dry_run,
            "{} config loaded",
            BACM_LOG_PREFIX
        );
        scan_log.push(
            "info",
            "bacm.config.loaded",
            format!(
                "{} config loaded: enabled={}, dry_run={}",
                BACM_LOG_PREFIX, bacm_runtime_config.enabled, bacm_runtime_config.dry_run
            ),
            Some(config.symbol.clone()),
            None,
        );
        tracing::info!(
            target: BACM_LOG_TARGET,
            enabled_symbols = ?if bacm_runtime_config.enabled {
                bacm_runtime_config.enabled_symbols()
            } else {
                Vec::new()
            },
            system_mode = config.system_mode.mode.as_str(),
            altcoin_disabled_reason = ?config.system_mode.altcoin_disabled_reason(),
            "{} runtime {}",
            BACM_LOG_PREFIX,
            if bacm_runtime_config.enabled { "enabled" } else { "disabled" }
        );
        tracing::info!(
            target: CWM_LOG_TARGET,
            event = cwm_log_events::CONFIG_LOADED,
            enabled = config.contract_whale_monitor.enabled,
            dry_run = config.contract_whale_monitor.dry_run,
            "{} config loaded",
            CWM_LOG_PREFIX
        );
        scan_log.push(
            "info",
            cwm_log_events::CONFIG_LOADED,
            format!(
                "{} config loaded: enabled={}, dry_run={}",
                CWM_LOG_PREFIX,
                config.contract_whale_monitor.enabled,
                config.contract_whale_monitor.dry_run
            ),
            Some(config.symbol.clone()),
            None,
        );
        let cwm_runtime_event = if config.contract_whale_monitor.enabled {
            cwm_log_events::RUNTIME_STARTED
        } else {
            cwm_log_events::RUNTIME_DISABLED
        };
        let cwm_runtime_message = if config.contract_whale_monitor.enabled {
            "runtime enabled"
        } else {
            "runtime disabled"
        };
        tracing::info!(
            target: CWM_LOG_TARGET,
            event = cwm_runtime_event,
            dry_run = config.contract_whale_monitor.dry_run,
            "{} {}",
            CWM_LOG_PREFIX,
            cwm_runtime_message
        );
        scan_log.push(
            "info",
            cwm_runtime_event,
            format!("{} {}", CWM_LOG_PREFIX, cwm_runtime_message),
            Some(config.symbol.clone()),
            None,
        );
        let cwm_retention = contract_whale_runtime_config().retention;
        spawn_contract_whale_retention_task(
            contract_whale_store.clone(),
            cwm_retention.flow_1s_days,
            cwm_retention.signals_days,
            storage_health.clone(),
        );
        let cwm_emission_watermarks = contract_whale_store
            .as_ref()
            .and_then(
                |store| match store.load_contract_whale_emission_watermarks() {
                    Ok(watermarks) => Some(watermarks),
                    Err(error) => {
                        tracing::warn!(
                            target: CWM_LOG_TARGET,
                            event = cwm_log_events::ERROR,
                            error = %error,
                            "{} emission watermark restore failed",
                            CWM_LOG_PREFIX
                        );
                        None
                    }
                },
            )
            .unwrap_or_default();
        let _ = storage_health.refresh_now();
        let operator_api_token = std::env::var("OPERATOR_TOKEN")
            .or_else(|_| std::env::var("OPERATOR_API_TOKEN"))
            .ok()
            .filter(|value| !value.trim().is_empty());

        Self {
            inner: Arc::new(AppStateInner {
                config,
                booted_at_ms,
                runtime_started: AtomicBool::new(false),
                runtime_control: Arc::new(RwLock::new(RuntimeControlTracker::new())),
                discord_auto_push_task: Arc::new(RwLock::new(None)),
                cwm_auto_push_task: Arc::new(RwLock::new(None)),
                cwm_discord_outbox_task: Arc::new(RwLock::new(None)),
                cwm_outcome_calibration_task: Arc::new(RwLock::new(None)),
                cwm_market_context_task: Arc::new(RwLock::new(None)),
                cwm_producer_running: AtomicBool::new(false),
                cwm_producer_last_started_at: AtomicI64::new(0),
                cwm_producer_last_completed_at: AtomicI64::new(0),
                cwm_producer_last_duration_ms: AtomicI64::new(0),
                cwm_producer_overlap_skipped: AtomicU64::new(0),
                cwm_emission_watermarks: Arc::new(RwLock::new(cwm_emission_watermarks)),
                scan_log,
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
                storage_health,
                operator_api_token,
                contract_whale_store,
                contract_whale_flow_flush_cursor_ms: Arc::new(RwLock::new(
                    std::collections::BTreeMap::new(),
                )),
                final_events_v2_cache: Arc::new(RwLock::new(std::collections::BTreeMap::new())),
                signal_history_service,
                whale_flow_candidate_history_service,
                spot_whale_service,
                binance_alt_contract_service,
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
        self.record_scan_log(
            "info",
            "scanner_starting",
            "Market-data scanner starting; alert-only mode remains enforced",
            Some(self.config().symbol.clone()),
            None,
        );

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
            runtime_control.last_start_error = Some(error.clone());
            drop(runtime_control);
            self.record_scan_log(
                "error",
                "scanner_start_failed",
                format!("Market-data scanner start failed: {error}"),
                Some(self.config().symbol.clone()),
                None,
            );
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
        self.inner.spot_whale_service.start();
        self.inner.binance_alt_contract_service.start();
        self.start_discord_auto_push_loop();
        self.start_contract_whale_market_context_loop();
        self.start_contract_whale_auto_push_loop();
        self.start_contract_whale_discord_outbox_loop();
        self.start_contract_whale_outcome_calibration_loop();
        self.record_scan_log(
            "info",
            "data_source_connecting",
            "Connecting configured market-data venues",
            Some(self.config().symbol.clone()),
            None,
        );
        self.inner.connector_manager.start_all().await;
        self.record_scan_log(
            "info",
            "scanner_started",
            "Market-data scanner started; Dashboard and Discord gates are alert-only",
            Some(self.config().symbol.clone()),
            None,
        );
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
        self.inner.binance_alt_contract_service.stop();
        self.inner.spot_whale_service.stop();
        self.inner.snapshot_service.stop();
        self.stop_contract_whale_auto_push_loop();
        self.stop_contract_whale_discord_outbox_loop();
        self.stop_contract_whale_outcome_calibration_loop();
        self.stop_contract_whale_market_context_loop();
        self.stop_discord_auto_push_loop();
        self.inner.alert_service.stop();
        self.inner.orderbook_wall_lifecycle_service.stop();
        self.inner.liq_hunt_service.stop();
        self.inner.toxic_service.stop();
        self.inner.liquidation_service.stop();
        self.inner.vpin_service.stop();
        self.inner.sweep_service.stop();
        self.inner.markout_service.stop();
        self.inner.flow_service.stop();
        self.record_scan_log(
            "info",
            "scanner_stopped",
            "Market-data scanner stopped",
            Some(self.config().symbol.clone()),
            None,
        );
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

    pub fn booted_at_ms(&self) -> i64 {
        self.inner.booted_at_ms
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

    pub fn record_scan_log(
        &self,
        level: impl AsRef<str>,
        kind: impl AsRef<str>,
        message: impl AsRef<str>,
        symbol: Option<String>,
        candidate_id: Option<String>,
    ) -> ScanLogItem {
        self.inner
            .scan_log
            .push(level, kind, message, symbol, candidate_id)
    }

    pub fn recent_scan_logs(&self, limit: usize) -> Vec<ScanLogItem> {
        self.inner.scan_log.recent(limit)
    }

    pub fn subscribe_scan_logs(&self) -> tokio::sync::broadcast::Receiver<ScanLogItem> {
        self.inner.scan_log.subscribe()
    }

    fn start_discord_auto_push_loop(&self) {
        if self.inner.discord_auto_push_task.read().is_some() {
            return;
        }
        let state = self.clone();
        let handle = tokio::spawn(async move {
            let mut interval = tokio::time::interval(discord_auto_push_interval());
            loop {
                interval.tick().await;
                state.evaluate_discord_auto_push_once().await;
            }
        });
        *self.inner.discord_auto_push_task.write() = Some(handle);
    }

    fn stop_discord_auto_push_loop(&self) {
        if let Some(handle) = self.inner.discord_auto_push_task.write().take() {
            handle.abort();
        }
    }

    fn start_contract_whale_auto_push_loop(&self) {
        if !self.config().contract_whale_monitor.enabled {
            return;
        }
        if self.inner.cwm_auto_push_task.read().is_some() {
            return;
        }
        let state = self.clone();
        let handle = tokio::spawn(async move {
            let mut interval = tokio::time::interval(contract_whale_auto_push_interval());
            interval.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Skip);
            loop {
                interval.tick().await;
                if state
                    .inner
                    .cwm_producer_running
                    .compare_exchange(false, true, Ordering::SeqCst, Ordering::SeqCst)
                    .is_err()
                {
                    state
                        .inner
                        .cwm_producer_overlap_skipped
                        .fetch_add(1, Ordering::SeqCst);
                    tracing::warn!(
                        target: CWM_LOG_TARGET,
                        event = "cwm.producer.overlap_skipped",
                        "{} contract whale producer overlap skipped",
                        CWM_LOG_PREFIX
                    );
                    continue;
                }
                let started_at = crate::normalizers::trade::now_ms();
                state
                    .inner
                    .cwm_producer_last_started_at
                    .store(started_at, Ordering::SeqCst);
                tracing::debug!(
                    target: CWM_LOG_TARGET,
                    event = "cwm.producer.started",
                    "{} contract whale producer started",
                    CWM_LOG_PREFIX
                );
                state.evaluate_contract_whale_auto_push_once().await;
                state
                    .inner
                    .cwm_producer_running
                    .store(false, Ordering::SeqCst);
                let completed_at = crate::normalizers::trade::now_ms();
                state
                    .inner
                    .cwm_producer_last_completed_at
                    .store(completed_at, Ordering::SeqCst);
                state
                    .inner
                    .cwm_producer_last_duration_ms
                    .store(completed_at.saturating_sub(started_at), Ordering::SeqCst);
                tracing::debug!(
                    target: CWM_LOG_TARGET,
                    event = "cwm.producer.completed",
                    duration_ms = completed_at.saturating_sub(started_at),
                    "{} contract whale producer completed",
                    CWM_LOG_PREFIX
                );
            }
        });
        *self.inner.cwm_auto_push_task.write() = Some(handle);
    }

    fn stop_contract_whale_auto_push_loop(&self) {
        if let Some(handle) = self.inner.cwm_auto_push_task.write().take() {
            handle.abort();
        }
        self.inner
            .cwm_producer_running
            .store(false, Ordering::SeqCst);
    }

    fn start_contract_whale_discord_outbox_loop(&self) {
        if !self.config().contract_whale_monitor.enabled
            || !contract_whale_discord_outbox_enabled()
            || self.contract_whale_store().is_none()
            || self.inner.cwm_discord_outbox_task.read().is_some()
        {
            return;
        }
        let state = self.clone();
        let handle = tokio::spawn(async move {
            let mut interval = tokio::time::interval(std::time::Duration::from_secs(1));
            interval.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Skip);
            loop {
                interval.tick().await;
                state.process_contract_whale_discord_outbox_once().await;
            }
        });
        *self.inner.cwm_discord_outbox_task.write() = Some(handle);
    }

    fn stop_contract_whale_discord_outbox_loop(&self) {
        if let Some(handle) = self.inner.cwm_discord_outbox_task.write().take() {
            handle.abort();
        }
    }

    async fn process_contract_whale_discord_outbox_once(&self) {
        let Some(store) = self.contract_whale_store() else {
            return;
        };
        let now = crate::normalizers::trade::now_ms();
        let claimed_store = store.clone();
        let claimed = match tokio::task::spawn_blocking(move || {
            claimed_store.claim_contract_whale_discord_outbox(20, now)
        })
        .await
        {
            Ok(Ok(items)) => items,
            Ok(Err(error)) => {
                tracing::warn!(
                    target: CWM_LOG_TARGET,
                    event = cwm_log_events::ERROR,
                    error = %error,
                    "{} discord outbox claim failed",
                    CWM_LOG_PREFIX
                );
                return;
            }
            Err(error) => {
                tracing::warn!(
                    target: CWM_LOG_TARGET,
                    event = cwm_log_events::ERROR,
                    error = %error,
                    "{} discord outbox claim task failed",
                    CWM_LOG_PREFIX
                );
                return;
            }
        };
        let settings =
            ContractWhaleDiscordSettings::from_env(self.config().contract_whale_monitor.dry_run);
        for item in claimed {
            let outcome =
                notify_contract_whale_discord(&settings, &item.signal, Some(store.clone())).await;
            let (status, next_attempt_at, sent_at, last_error) = if outcome.sent {
                (
                    ContractWhaleDiscordOutboxStatus::Sent,
                    None,
                    outcome.sent_at_ms,
                    None,
                )
            } else if outcome.dry_run {
                (ContractWhaleDiscordOutboxStatus::DryRun, None, None, None)
            } else if is_contract_whale_discord_retryable(&outcome.reason)
                && item.attempts < settings.max_attempts
            {
                (
                    ContractWhaleDiscordOutboxStatus::Retry,
                    Some(crate::normalizers::trade::now_ms().saturating_add(
                        contract_whale_discord_retry_delay_ms(&item.signal_id, item.attempts),
                    )),
                    None,
                    Some(outcome.reason.as_str()),
                )
            } else {
                (
                    ContractWhaleDiscordOutboxStatus::Dead,
                    None,
                    None,
                    Some(outcome.reason.as_str()),
                )
            };
            let finish_store = store.clone();
            let signal_id = item.signal_id.clone();
            let last_error = last_error.map(str::to_string);
            if let Err(error) = tokio::task::spawn_blocking(move || {
                finish_store.finish_contract_whale_discord_outbox(
                    &signal_id,
                    status,
                    next_attempt_at,
                    sent_at,
                    last_error.as_deref(),
                )
            })
            .await
            .unwrap_or_else(|error| Err(anyhow::anyhow!(error)))
            {
                tracing::warn!(
                    target: CWM_LOG_TARGET,
                    event = cwm_log_events::ERROR,
                    signal_id = item.signal_id.as_str(),
                    error = %error,
                    "{} discord outbox finish failed",
                    CWM_LOG_PREFIX
                );
            }
        }
    }

    fn start_contract_whale_outcome_calibration_loop(&self) {
        if !self.config().contract_whale_monitor.enabled
            || !contract_whale_outcome_calibration_enabled()
            || self.contract_whale_store().is_none()
            || self.inner.cwm_outcome_calibration_task.read().is_some()
        {
            return;
        }
        let state = self.clone();
        let handle = tokio::spawn(async move {
            let mut interval = tokio::time::interval(std::time::Duration::from_secs(15));
            interval.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Skip);
            loop {
                interval.tick().await;
                state
                    .process_contract_whale_outcome_calibration_once()
                    .await;
            }
        });
        *self.inner.cwm_outcome_calibration_task.write() = Some(handle);
    }

    fn stop_contract_whale_outcome_calibration_loop(&self) {
        if let Some(handle) = self.inner.cwm_outcome_calibration_task.write().take() {
            handle.abort();
        }
    }

    async fn process_contract_whale_outcome_calibration_once(&self) {
        let Some(store) = self.contract_whale_store() else {
            return;
        };
        let now = crate::normalizers::trade::now_ms();
        let evaluation_store = store.clone();
        let result = tokio::task::spawn_blocking(move || {
            let signals =
                evaluation_store.query_contract_whale_signals(&ContractWhaleSignalQuery {
                    from_ts: Some(now.saturating_sub(24 * 60 * 60 * 1_000)),
                    to_ts: Some(now.saturating_sub(30_000)),
                    limit: 500,
                    ..ContractWhaleSignalQuery::default()
                })?;
            let mut outcomes = Vec::new();
            for signal in signals {
                let to_ts = now.min(signal.ts.saturating_add(300_000));
                let buckets = evaluation_store.list_contract_flow_buckets_between(
                    &signal.symbol,
                    signal.ts,
                    to_ts,
                )?;
                if let Some(outcome) =
                    evaluate_contract_whale_signal_outcome(&signal, &buckets, now)
                {
                    outcomes.push(outcome);
                }
            }
            evaluation_store.upsert_contract_whale_signal_outcomes(&outcomes)
        })
        .await;
        match result {
            Ok(Ok(written)) if written > 0 => tracing::debug!(
                target: CWM_LOG_TARGET,
                outcomes = written,
                "{} contract whale outcomes updated",
                CWM_LOG_PREFIX
            ),
            Ok(Ok(_)) => {}
            Ok(Err(error)) => tracing::warn!(
                target: CWM_LOG_TARGET,
                event = cwm_log_events::ERROR,
                error = %error,
                "{} contract whale outcome evaluation failed",
                CWM_LOG_PREFIX
            ),
            Err(error) => tracing::warn!(
                target: CWM_LOG_TARGET,
                event = cwm_log_events::ERROR,
                error = %error,
                "{} contract whale outcome evaluation task failed",
                CWM_LOG_PREFIX
            ),
        }
    }

    fn start_contract_whale_market_context_loop(&self) {
        if !self.config().contract_whale_monitor.enabled {
            return;
        }
        if self.inner.cwm_market_context_task.read().is_some() {
            return;
        }
        let state = self.clone();
        let handle = tokio::spawn(async move {
            let client = reqwest::Client::builder()
                .timeout(std::time::Duration::from_secs(10))
                .build()
                .unwrap_or_else(|_| reqwest::Client::new());
            state.poll_contract_whale_market_context_once(&client).await;
            let mut interval = tokio::time::interval(contract_whale_market_context_poll_interval());
            interval.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Skip);
            loop {
                interval.tick().await;
                state.poll_contract_whale_market_context_once(&client).await;
            }
        });
        *self.inner.cwm_market_context_task.write() = Some(handle);
    }

    fn stop_contract_whale_market_context_loop(&self) {
        if let Some(handle) = self.inner.cwm_market_context_task.write().take() {
            handle.abort();
        }
    }

    async fn poll_contract_whale_market_context_once(&self, client: &reqwest::Client) {
        let runtime_config = contract_whale_runtime_config();
        let store = self.contract_whale_store();
        if store.is_none() {
            return;
        }

        let symbols = enabled_contract_whale_symbols();
        if symbols.is_empty() {
            return;
        }

        let mut oi_snapshots = Vec::<ContractOiSnapshot>::new();
        let mut funding_snapshots = Vec::<ContractFundingSnapshot>::new();
        let fallback_ts = crate::normalizers::trade::now_ms();

        for symbol in symbols {
            let binance_oi = async {
                if runtime_config
                    .exchanges
                    .binance
                    .market_enabled(ContractWhaleMarketType::Oi)
                {
                    collector_binance::fetch_binance_open_interest_snapshot_for_symbol(
                        client,
                        &symbol,
                        None,
                        fallback_ts,
                    )
                    .await
                } else {
                    Ok(None)
                }
            };
            let binance_funding = async {
                if runtime_config
                    .exchanges
                    .binance
                    .market_enabled(ContractWhaleMarketType::Funding)
                {
                    collector_binance::fetch_binance_funding_snapshot_for_symbol(
                        client,
                        &symbol,
                        fallback_ts,
                    )
                    .await
                } else {
                    Ok(None)
                }
            };
            let okx_oi = async {
                if runtime_config
                    .exchanges
                    .okx
                    .market_enabled(ContractWhaleMarketType::Oi)
                {
                    match collector_okx::fetch_okx_contract_value_base(client, &symbol).await {
                        Ok(Some(ct_val_base)) => {
                            collector_okx::fetch_okx_open_interest_snapshot_for_symbol(
                                client,
                                &symbol,
                                ct_val_base,
                            )
                            .await
                        }
                        Ok(None) => {
                            tracing::warn!(
                                target: CWM_LOG_TARGET,
                                event = cwm_log_events::ERROR,
                                symbol = symbol.as_str(),
                                exchange = "okx",
                                context = "ct_val",
                                "{} OKX instrument metadata missing ctVal; OI snapshot skipped",
                                CWM_LOG_PREFIX
                            );
                            Ok(None)
                        }
                        Err(error) => Err(error),
                    }
                } else {
                    Ok(None)
                }
            };
            let okx_funding = async {
                if runtime_config
                    .exchanges
                    .okx
                    .market_enabled(ContractWhaleMarketType::Funding)
                {
                    collector_okx::fetch_okx_funding_snapshot_for_symbol(client, &symbol).await
                } else {
                    Ok(None)
                }
            };
            let (binance_oi, binance_funding, okx_oi, okx_funding) =
                tokio::join!(binance_oi, binance_funding, okx_oi, okx_funding);

            if runtime_config
                .exchanges
                .binance
                .market_enabled(ContractWhaleMarketType::Oi)
            {
                match binance_oi {
                    Ok(Some(snapshot)) => oi_snapshots.push(snapshot),
                    Ok(None) => {}
                    Err(error) => {
                        tracing::warn!(
                            target: CWM_LOG_TARGET,
                            event = cwm_log_events::ERROR,
                            symbol = symbol.as_str(),
                            exchange = "binance",
                            context = "oi",
                            error = %error,
                            "{} binance oi snapshot fetch failed",
                            CWM_LOG_PREFIX
                        );
                    }
                }
            }
            if runtime_config
                .exchanges
                .binance
                .market_enabled(ContractWhaleMarketType::Funding)
            {
                match binance_funding {
                    Ok(Some(snapshot)) => funding_snapshots.push(snapshot),
                    Ok(None) => {}
                    Err(error) => {
                        tracing::warn!(
                            target: CWM_LOG_TARGET,
                            event = cwm_log_events::ERROR,
                            symbol = symbol.as_str(),
                            exchange = "binance",
                            context = "funding",
                            error = %error,
                            "{} binance funding snapshot fetch failed",
                            CWM_LOG_PREFIX
                        );
                    }
                }
            }
            if runtime_config
                .exchanges
                .okx
                .market_enabled(ContractWhaleMarketType::Oi)
            {
                match okx_oi {
                    Ok(Some(snapshot)) => oi_snapshots.push(snapshot),
                    Ok(None) => {}
                    Err(error) => {
                        tracing::warn!(
                            target: CWM_LOG_TARGET,
                            event = cwm_log_events::ERROR,
                            symbol = symbol.as_str(),
                            exchange = "okx",
                            context = "oi",
                            error = %error,
                            "{} okx oi snapshot fetch failed",
                            CWM_LOG_PREFIX
                        );
                    }
                }
            }
            if runtime_config
                .exchanges
                .okx
                .market_enabled(ContractWhaleMarketType::Funding)
            {
                match okx_funding {
                    Ok(Some(snapshot)) => funding_snapshots.push(snapshot),
                    Ok(None) => {}
                    Err(error) => {
                        tracing::warn!(
                            target: CWM_LOG_TARGET,
                            event = cwm_log_events::ERROR,
                            symbol = symbol.as_str(),
                            exchange = "okx",
                            context = "funding",
                            error = %error,
                            "{} okx funding snapshot fetch failed",
                            CWM_LOG_PREFIX
                        );
                    }
                }
            }
        }

        let oi_outcome =
            persist_contract_oi_snapshots_nonblocking(store.clone(), oi_snapshots).await;
        let funding_outcome =
            persist_contract_funding_snapshots_nonblocking(store, funding_snapshots).await;

        if oi_outcome.written > 0 || funding_outcome.written > 0 {
            tracing::info!(
                target: CWM_LOG_TARGET,
                event = "contract_market_context_poll",
                oi_written = oi_outcome.written,
                funding_written = funding_outcome.written,
                "{} contract market context poll persisted snapshots",
                CWM_LOG_PREFIX
            );
        }
    }

    async fn evaluate_contract_whale_auto_push_once(&self) {
        let config = self.config().contract_whale_monitor;
        if !config.enabled {
            return;
        }
        let runtime_config = contract_whale_runtime_config();
        let symbols = runtime_config
            .symbols
            .iter()
            .filter(|(_, symbol_config)| symbol_config.enabled)
            .map(|(symbol, _)| symbol.clone())
            .collect::<Vec<_>>();
        for symbol in symbols {
            let _ = self
                .flush_live_contract_flow_buckets_for_symbol(&symbol)
                .await;
            let flow_state = self.flow_state_for_symbol(&symbol);
            let store = self.contract_whale_store();
            let baselines = store
                .as_ref()
                .map(|store| load_quality_baselines(store, &flow_state, &symbol))
                .unwrap_or_default();
            let liquidations = store
                .as_ref()
                .map(|store| load_liquidation_contexts(store, &flow_state, &symbol))
                .unwrap_or_default();
            let market_context = store
                .as_ref()
                .map(|store| load_market_context(store, &flow_state, &symbol))
                .unwrap_or_default();
            let venue_health = self.venue_health();
            let response = build_contract_whale_response_with_runtime_and_baselines(
                &flow_state,
                &symbol,
                10,
                None,
                config.enabled,
                config.dry_run,
                ContractWhaleResponseRuntime {
                    venue_health: Some(&venue_health),
                    baselines: &baselines,
                    liquidations: &liquidations,
                    market_context: &market_context,
                    booted_at_ms: Some(self.booted_at_ms()),
                },
            );
            let settings = ContractWhaleDiscordSettings::from_env(config.dry_run);
            let signals = self.filter_contract_whale_emissions(response.items);
            let _ =
                persist_contract_whale_signals_nonblocking(store.clone(), signals.clone()).await;
            if contract_whale_discord_outbox_enabled() {
                let cooldown_store = global_contract_whale_discord_cooldown_store();
                let now = crate::normalizers::trade::now_ms();
                let queued = signals
                    .iter()
                    .filter(|signal| {
                        let decision = evaluate_contract_whale_discord_gate(
                            &settings,
                            signal,
                            cooldown_store,
                            now,
                        );
                        self.record_scan_log(
                            if decision.allowed { "info" } else { "debug" },
                            if decision.allowed {
                                cwm_log_events::DISCORD_ELIGIBLE
                            } else {
                                cwm_log_events::DISCORD_SKIPPED
                            },
                            format!(
                                "{} discord {} for {}: {}",
                                CWM_LOG_PREFIX,
                                if decision.allowed {
                                    "queued"
                                } else {
                                    "skipped"
                                },
                                signal.symbol,
                                decision.reason
                            ),
                            Some(signal.symbol.clone()),
                            Some(signal.id.clone()),
                        );
                        decision.allowed
                    })
                    .cloned()
                    .collect::<Vec<_>>();
                if let Some(store) = store.clone() {
                    if !queued.is_empty() {
                        let queued_count = queued.len();
                        match tokio::task::spawn_blocking(move || {
                            store.enqueue_contract_whale_discord_outbox(&queued, now)
                        })
                        .await
                        {
                            Ok(Ok(inserted)) => tracing::info!(
                                target: CWM_LOG_TARGET,
                                event = cwm_log_events::DISCORD_ELIGIBLE,
                                queued = inserted,
                                eligible = queued_count,
                                "{} discord outbox queued eligible signals",
                                CWM_LOG_PREFIX
                            ),
                            Ok(Err(error)) => tracing::warn!(
                                target: CWM_LOG_TARGET,
                                event = cwm_log_events::ERROR,
                                error = %error,
                                "{} discord outbox enqueue failed",
                                CWM_LOG_PREFIX
                            ),
                            Err(error) => tracing::warn!(
                                target: CWM_LOG_TARGET,
                                event = cwm_log_events::ERROR,
                                error = %error,
                                "{} discord outbox enqueue task failed",
                                CWM_LOG_PREFIX
                            ),
                        }
                    }
                }
            } else {
                for signal in signals {
                    let outcome =
                        notify_contract_whale_discord(&settings, &signal, store.clone()).await;
                    self.record_scan_log(
                        if outcome.sent { "info" } else { "debug" },
                        if outcome.sent {
                            cwm_log_events::DISCORD_SENT
                        } else {
                            cwm_log_events::DISCORD_SKIPPED
                        },
                        format!(
                            "{} discord {} for {}: {}",
                            CWM_LOG_PREFIX,
                            if outcome.sent { "sent" } else { "skipped" },
                            signal.symbol,
                            outcome.reason
                        ),
                        Some(signal.symbol.clone()),
                        Some(signal.id.clone()),
                    );
                }
            }
        }
    }

    fn filter_contract_whale_emissions(
        &self,
        signals: Vec<crate::contract_whale_monitor::types::ContractWhaleSignal>,
    ) -> Vec<crate::contract_whale_monitor::types::ContractWhaleSignal> {
        let now = crate::normalizers::trade::now_ms();
        let emission_config = contract_whale_runtime_config().emission;
        let mut watermarks = self.inner.cwm_emission_watermarks.write();
        let mut emitted = Vec::with_capacity(signals.len());
        for signal in signals {
            let key = emission_key(&signal);
            if should_emit(&signal, watermarks.get(&key), now, &emission_config) {
                watermarks.insert(key, fingerprint(&signal, now));
                emitted.push(signal);
            } else {
                tracing::debug!(
                    target: CWM_LOG_TARGET,
                    event = "cwm.producer.emission_suppressed",
                    signal_id = signal.id.as_str(),
                    symbol = signal.symbol.as_str(),
                    window_sec = signal.window_sec,
                    "{} contract whale near-duplicate signal suppressed",
                    CWM_LOG_PREFIX
                );
            }
        }
        let watermark_snapshot = (!emitted.is_empty()).then(|| watermarks.clone());
        drop(watermarks);
        if let (Some(store), Some(watermarks)) = (self.contract_whale_store(), watermark_snapshot) {
            tokio::spawn(async move {
                match tokio::task::spawn_blocking(move || {
                    store.upsert_contract_whale_emission_watermarks(&watermarks)
                })
                .await
                {
                    Ok(Ok(_)) => {}
                    Ok(Err(error)) => tracing::warn!(
                        target: CWM_LOG_TARGET,
                        event = cwm_log_events::ERROR,
                        error = %error,
                        "{} emission watermark persist failed",
                        CWM_LOG_PREFIX
                    ),
                    Err(error) => tracing::warn!(
                        target: CWM_LOG_TARGET,
                        event = cwm_log_events::ERROR,
                        error = %error,
                        "{} emission watermark persist task failed",
                        CWM_LOG_PREFIX
                    ),
                }
            });
        }
        emitted
    }

    async fn flush_live_contract_flow_buckets_for_symbol(
        &self,
        symbol: &str,
    ) -> ContractWhalePersistenceOutcome {
        let now = crate::normalizers::trade::now_ms();
        let canonical_symbol = contract_flow_base_asset(symbol);
        let last_flushed_ts = self
            .inner
            .contract_whale_flow_flush_cursor_ms
            .read()
            .get(&canonical_symbol)
            .copied();
        let lookback_from = last_flushed_ts
            .map(|ts| ts.saturating_sub(contract_flow_flush_rewind_ms()))
            .unwrap_or_else(|| now.saturating_sub(contract_flow_initial_lookback_ms()));
        let trades = self.inner.flow_service.get_trades_since(lookback_from);
        let contract_trades = trades
            .iter()
            .filter_map(|trade| normalized_trade_to_contract_trade(trade, &canonical_symbol))
            .collect::<Vec<_>>();
        if contract_trades.is_empty() {
            tracing::debug!(
                target: CWM_LOG_TARGET,
                event = "contract_flow_live_flush",
                symbol = canonical_symbol.as_str(),
                status = "empty",
                reason = "no_pending_buckets",
                "{} live flow flush empty",
                CWM_LOG_PREFIX
            );
            return ContractWhalePersistenceOutcome {
                attempted: true,
                succeeded: true,
                written: 0,
            };
        }

        let buckets = aggregate_1s_buckets(&contract_trades);
        if buckets.is_empty() {
            tracing::debug!(
                target: CWM_LOG_TARGET,
                event = "contract_flow_live_flush",
                symbol = canonical_symbol.as_str(),
                status = "empty",
                reason = "no_aggregated_buckets",
                "{} live flow flush empty",
                CWM_LOG_PREFIX
            );
            return ContractWhalePersistenceOutcome {
                attempted: true,
                succeeded: true,
                written: 0,
            };
        }

        let bucket_count = buckets.len();
        let buy_volume_btc = buckets
            .iter()
            .map(|bucket| bucket.buy_volume_btc)
            .sum::<f64>();
        let sell_volume_btc = buckets
            .iter()
            .map(|bucket| bucket.sell_volume_btc)
            .sum::<f64>();
        let max_ts_bucket = buckets.iter().map(|bucket| bucket.ts_bucket).max();
        let started_at = std::time::Instant::now();
        let outcome =
            flush_contract_flow_buckets_nonblocking(self.contract_whale_store(), buckets).await;

        if outcome.succeeded {
            if let Some(max_ts_bucket) = max_ts_bucket {
                let mut cursors = self.inner.contract_whale_flow_flush_cursor_ms.write();
                let entry = cursors
                    .entry(canonical_symbol.clone())
                    .or_insert(max_ts_bucket);
                *entry = (*entry).max(max_ts_bucket);
            }
            tracing::info!(
                target: CWM_LOG_TARGET,
                event = "contract_flow_live_flush",
                symbol = canonical_symbol.as_str(),
                status = "ok",
                rows = outcome.written,
                bucket_count = bucket_count,
                duration_ms = started_at.elapsed().as_millis() as u64,
                "{} live flow flush ok",
                CWM_LOG_PREFIX
            );
            tracing::info!(
                target: CWM_LOG_TARGET,
                event = "contract_flow_bucket_breakdown",
                symbol = canonical_symbol.as_str(),
                rows = bucket_count,
                buy_volume_btc = buy_volume_btc,
                sell_volume_btc = sell_volume_btc,
                "{} live flow bucket breakdown",
                CWM_LOG_PREFIX
            );
        } else if outcome.attempted {
            tracing::warn!(
                target: CWM_LOG_TARGET,
                event = "contract_flow_live_flush",
                symbol = canonical_symbol.as_str(),
                status = "error",
                pending_rows = bucket_count,
                "{} live flow flush failed",
                CWM_LOG_PREFIX
            );
        } else {
            tracing::debug!(
                target: CWM_LOG_TARGET,
                event = "contract_flow_live_flush",
                symbol = canonical_symbol.as_str(),
                status = "skipped",
                reason = "sqlite_store_unavailable",
                "{} live flow flush skipped",
                CWM_LOG_PREFIX
            );
        }

        outcome
    }

    async fn evaluate_discord_auto_push_once(&self) {
        let symbols = market_structure_event_symbols(self);
        for symbol in symbols {
            let recent = build_recent(self, &symbol);
            let snapshot = build_ws_snapshot(&recent);
            self.observe_main_force_events(&symbol, &snapshot.signals)
                .await;

            if !symbol.eq_ignore_ascii_case(&self.config().symbol) || recent.items.is_empty() {
                continue;
            }

            for (item, signal) in recent.items.iter().zip(snapshot.signals.iter()) {
                let request = discord_request_from_signal(signal);
                let _ = maybe_auto_push_discord(self, request, item.created_at_ms).await;
            }
        }
    }

    async fn observe_main_force_events(&self, symbol: &str, signals: &[ToxicSignalWsItem]) {
        let observation = best_main_force_event_observation(signals, symbol);
        let Some(store) = self.contract_whale_store() else {
            return;
        };
        let symbol = symbol.to_ascii_uppercase();
        let now = crate::normalizers::trade::now_ms();
        let result = tokio::task::spawn_blocking(move || {
            store.observe_main_force_event(&symbol, observation.as_ref(), now)
        })
        .await;
        match result {
            Ok(Ok(Some(event))) => {
                self.record_scan_log(
                    "debug",
                    "main_force_event_observed",
                    format!(
                        "main force event tracked for {}: {} / {}",
                        event.symbol, event.regime_type, event.severity
                    ),
                    Some(event.symbol.clone()),
                    None,
                );
            }
            Ok(Ok(None)) => {}
            Ok(Err(error)) => {
                tracing::warn!(error = %error, "main force event observation failed");
            }
            Err(error) => {
                tracing::warn!(error = %error, "main force event observation task failed");
            }
        }
    }

    pub fn venue_health(&self) -> VenueHealthMap {
        self.inner.connector_manager.get_venue_health()
    }

    pub fn flow_state(&self) -> FlowState {
        self.inner.flow_service.latest_state()
    }

    pub fn flow_state_for_symbol(&self, symbol: &str) -> FlowState {
        self.inner.flow_service.latest_state_for_symbol(symbol)
    }

    pub fn spot_whale_service(&self) -> SpotWhaleService {
        self.inner.spot_whale_service.clone()
    }

    pub fn binance_alt_contract_service(&self) -> BinanceAltContractService {
        self.inner.binance_alt_contract_service.clone()
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

    pub fn storage_health_snapshot(&self) -> StorageHealthSnapshot {
        self.inner.storage_health.refresh_now()
    }

    pub fn operator_token_authorized(&self, headers: &HeaderMap) -> bool {
        let Some(expected) = self.inner.operator_api_token.as_deref() else {
            return false;
        };
        let header_token = headers
            .get("x-operator-api-token")
            .or_else(|| headers.get("x-operator-token"))
            .and_then(|value| value.to_str().ok());
        if header_token == Some(expected) {
            return true;
        }
        headers
            .get(header::AUTHORIZATION)
            .and_then(|value| value.to_str().ok())
            .and_then(|value| value.strip_prefix("Bearer "))
            == Some(expected)
    }

    pub fn operator_token_configured(&self) -> bool {
        self.inner.operator_api_token.is_some()
    }

    pub fn signal_history_service(&self) -> ToxicSignalHistoryService {
        self.inner.signal_history_service.clone()
    }

    pub fn whale_flow_candidate_history_service(&self) -> WhaleFlowCandidateHistoryService {
        self.inner.whale_flow_candidate_history_service.clone()
    }

    pub fn contract_whale_store(&self) -> Option<SqliteStore> {
        self.inner.contract_whale_store.clone()
    }

    pub fn contract_whale_runtime_diagnostics(&self) -> ContractWhaleRuntimeDiagnostics {
        let last_started_at = self
            .inner
            .cwm_producer_last_started_at
            .load(Ordering::SeqCst);
        let last_completed_at = self
            .inner
            .cwm_producer_last_completed_at
            .load(Ordering::SeqCst);
        let last_duration_ms = self
            .inner
            .cwm_producer_last_duration_ms
            .load(Ordering::SeqCst);
        let queue = self
            .contract_whale_store()
            .and_then(|store| {
                store
                    .contract_whale_discord_outbox_stats(crate::normalizers::trade::now_ms())
                    .ok()
            })
            .unwrap_or_default();
        ContractWhaleRuntimeDiagnostics {
            producer_loop: ContractWhaleProducerLoopDiagnostics {
                last_started_at: (last_started_at > 0).then_some(last_started_at),
                last_completed_at: (last_completed_at > 0).then_some(last_completed_at),
                last_duration_ms: (last_duration_ms > 0).then_some(last_duration_ms),
                overlap_skipped: self
                    .inner
                    .cwm_producer_overlap_skipped
                    .load(Ordering::SeqCst),
                missed_tick_policy: "skip",
            },
            discord_queue: ContractWhaleDiscordQueueDiagnostics {
                pending: queue.pending,
                retrying: queue.retrying,
                failed: queue.failed,
                oldest_pending_age_sec: queue.oldest_pending_age_sec,
            },
        }
    }

    pub fn cached_final_events_v2(&self, key: &str) -> Option<(i64, FinalEventsV2Response)> {
        self.inner
            .final_events_v2_cache
            .read()
            .get(key)
            .map(|entry| (entry.cached_at_ms, entry.response.clone()))
    }

    pub fn store_final_events_v2_cache(
        &self,
        key: String,
        cached_at_ms: i64,
        response: FinalEventsV2Response,
    ) {
        self.inner.final_events_v2_cache.write().insert(
            key,
            CachedFinalEventsV2Entry {
                cached_at_ms,
                response,
            },
        );
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

fn market_structure_event_symbols(state: &AppState) -> Vec<String> {
    let mut symbols = vec![state.config().symbol.trim().to_ascii_uppercase()];
    for (symbol, symbol_config) in &contract_whale_runtime_config().symbols {
        if symbol_config.enabled {
            let normalized = symbol.trim().to_ascii_uppercase();
            if !symbols.iter().any(|existing| existing == &normalized) {
                symbols.push(normalized);
            }
        }
    }
    symbols
}

fn discord_auto_push_interval() -> std::time::Duration {
    let ms = std::env::var("DISCORD_AUTO_PUSH_INTERVAL_MS")
        .ok()
        .and_then(|value| value.parse::<u64>().ok())
        .filter(|value| (500..=60_000).contains(value))
        .unwrap_or(1_000);
    std::time::Duration::from_millis(ms)
}

fn contract_whale_auto_push_interval() -> std::time::Duration {
    let ms = std::env::var("CONTRACT_WHALE_AUTO_PUSH_INTERVAL_MS")
        .ok()
        .and_then(|value| value.parse::<u64>().ok())
        .filter(|value| (1_000..=60_000).contains(value))
        .unwrap_or(2_000);
    std::time::Duration::from_millis(ms)
}

fn contract_whale_discord_outbox_enabled() -> bool {
    std::env::var("CONTRACT_WHALE_DISCORD_OUTBOX_ENABLED")
        .ok()
        .and_then(|value| match value.trim().to_ascii_lowercase().as_str() {
            "1" | "true" | "yes" | "on" => Some(true),
            "0" | "false" | "no" | "off" => Some(false),
            _ => None,
        })
        .unwrap_or(true)
}

fn contract_whale_outcome_calibration_enabled() -> bool {
    std::env::var("CONTRACT_WHALE_OUTCOME_CALIBRATION_ENABLED")
        .ok()
        .and_then(|value| match value.trim().to_ascii_lowercase().as_str() {
            "1" | "true" | "yes" | "on" => Some(true),
            "0" | "false" | "no" | "off" => Some(false),
            _ => None,
        })
        .unwrap_or(true)
}

fn is_contract_whale_discord_retryable(reason: &str) -> bool {
    matches!(reason, "send_failed")
}

fn contract_whale_discord_retry_delay_ms(signal_id: &str, attempts: usize) -> i64 {
    let exponential_ms = 1_000_i64.saturating_mul(1_i64 << attempts.min(5));
    let jitter_ms = signal_id.bytes().fold(0_u64, |hash, byte| {
        hash.wrapping_mul(31).wrapping_add(byte as u64)
    }) % 401;
    exponential_ms.saturating_add(jitter_ms as i64 - 200)
}

fn contract_whale_market_context_poll_interval() -> std::time::Duration {
    let ms = std::env::var("CONTRACT_WHALE_MARKET_CONTEXT_POLL_INTERVAL_MS")
        .ok()
        .and_then(|value| value.parse::<u64>().ok())
        .filter(|value| (5_000..=120_000).contains(value))
        .unwrap_or(15_000);
    std::time::Duration::from_millis(ms)
}

fn enabled_contract_whale_symbols() -> Vec<String> {
    contract_whale_runtime_config()
        .symbols
        .iter()
        .filter(|(_, symbol_config)| symbol_config.enabled)
        .map(|(symbol, _)| symbol.trim().to_ascii_uppercase())
        .collect()
}

fn contract_flow_initial_lookback_ms() -> i64 {
    120_000
}

fn contract_flow_flush_rewind_ms() -> i64 {
    5_000
}

fn normalized_trade_to_contract_trade(
    trade: &NormalizedTrade,
    requested_symbol: &str,
) -> Option<ContractTrade> {
    let canonical_symbol = contract_flow_base_asset(&trade.symbol);
    if !canonical_symbol.eq_ignore_ascii_case(requested_symbol) {
        return None;
    }
    let exchange = match trade.venue {
        Venue::Binance => ContractExchange::Binance,
        Venue::Okx => ContractExchange::Okx,
        Venue::Bitfinex => ContractExchange::Bitfinex,
        Venue::Bybit => return None,
    };
    if trade.ts <= 0
        || !trade.price.is_finite()
        || trade.price <= 0.0
        || !trade.size_btc.is_finite()
        || trade.size_btc <= 0.0
        || !trade.size_usd.is_finite()
        || trade.size_usd <= 0.0
    {
        return None;
    }
    Some(ContractTrade {
        ts: trade.ts,
        exchange,
        symbol: canonical_symbol,
        market: "perp".to_string(),
        price: trade.price,
        qty_btc: trade.size_btc,
        notional_usd: trade.size_usd,
        side: match trade.aggressor_side {
            crate::types::market::AggressorSide::Buy => ContractTradeSide::Buy,
            crate::types::market::AggressorSide::Sell => ContractTradeSide::Sell,
        },
        raw_trade_count: Some(1),
    })
}

fn contract_flow_base_asset(symbol: &str) -> String {
    let upper = symbol.trim().to_ascii_uppercase();
    let first = upper
        .split([':', '/', '_'])
        .next()
        .unwrap_or(upper.as_str());
    let base = first.split('-').next().unwrap_or(first);
    base.trim_end_matches("PERP")
        .trim_end_matches("USDT")
        .trim_end_matches("USD")
        .trim_end_matches("F0")
        .to_string()
}

fn discord_request_from_signal(signal: &ToxicSignalWsItem) -> DiscordNotificationRequest {
    let mut request = DiscordNotificationRequest {
        alert_family: None,
        signal_id: Some(signal.id.clone()),
        id: Some(signal.id.clone()),
        dedupe_key: Some(signal.id.clone()),
        exchange: Some("Runtime".to_string()),
        symbol: Some(signal.symbol.clone()),
        signal_type: Some(signal.detector.clone()),
        level: Some(signal.severity.clone()),
        side: Some(signal.direction_label.clone()),
        score: Some(signal.final_risk_score),
        confidence: Some(signal.toxic_short_score.confidence),
        data_quality: Some(signal.data_quality),
        reason: Some(signal.final_result.clone()),
        impact: None,
        impact_level: None,
        time: Some(signal.created_at.clone()),
        price_range: signal.trigger_price_usd.map(format_trigger_price_range),
        add_qty: None,
        cancel_qty: None,
        fill_qty: None,
        cancel_to_trade_ratio: None,
        depth_before: None,
        depth_after: None,
        depth_impact: None,
        price_impact_bps: None,
        markout_1s_bps: None,
        markout_5s_bps: None,
        markout_30s_bps: None,
        tof_metrics: Some(signal.tof_metrics.clone()),
        tof_score: Some(signal.tof_score),
        candidate_type: Some(signal.candidate_type.clone()),
        explain_tags: Some(signal.explain_tags.clone()),
        direction_confidence: Some(signal.direction_confidence),
        perp_tof_metrics: Some(signal.perp_tof_metrics.clone()),
        perp_score: Some(signal.perp_score),
        perp_candidate_type: Some(signal.perp_candidate_type.clone()),
        final_candidate_type: Some(signal.final_candidate_type.clone()),
        metrics_direction: serde_json::to_value(signal.metrics_direction)
            .ok()
            .and_then(|value| value.as_str().map(str::to_string)),
        advanced_tof_metrics: Some(signal.advanced_tof_metrics.clone()),
        advanced_score: Some(signal.advanced_score),
        advanced_candidate_type: Some(signal.advanced_candidate_type.clone()),
        main_force_score: Some(signal.main_force_score),
        extreme_impact_score: Some(signal.extreme_impact_score),
        structure_bias: Some(signal.structure_bias),
        market_structure_confidence: Some(signal.market_structure_confidence),
        market_structure_data_quality: Some(signal.market_structure_data_quality),
        market_structure_severity: Some(signal.market_structure_severity.clone()),
        regime_type: Some(signal.regime_type.clone()),
        spot_score: Some(signal.spot_score),
        contract_score: Some(signal.contract_score),
        cross_confirm_score: Some(signal.cross_confirm_score),
        main_force_confirmed: Some(signal.main_force_confirmed),
        signal_agreement: Some(signal.signal_agreement),
        source_coverage: Some(signal.source_coverage),
        oi_score: Some(signal.oi_score),
        liquidation_score: Some(signal.liquidation_score),
        test: None,
    };
    request.alert_family = Some(preferred_discord_alert_family(&request).to_string());
    request
}

fn format_trigger_price_range(price: f64) -> String {
    if price >= 1000.0 {
        format!("${price:.0}")
    } else if price >= 1.0 {
        format!("${price:.2}")
    } else {
        format!("${price:.4}")
    }
}

#[cfg(test)]
mod tests {
    use super::AppState;
    use crate::{
        config::{
            env::{ContractWhaleMonitorConfig, SpotWhaleMonitorConfig},
            system_mode::SystemModeConfig,
            venues::{VenueConfig, VenueConfigs},
            AppConfig,
        },
        storage::contract_whale_repo::ContractWhaleRepo,
        types::{
            market::{AggressorSide, NormalizedTrade, Venue},
            toxic::ToxicSeverity,
        },
    };
    use std::time::{SystemTime, UNIX_EPOCH};

    #[tokio::test]
    async fn live_contract_flow_flush_persists_canonical_btc_buckets() {
        let state = AppState::new(test_config(temp_sqlite_path("live-contract-flow-flush")));
        let now = crate::normalizers::trade::now_ms();
        let flow_service = state.flow_service_for_tests();
        flow_service.add_trade_for_tests(NormalizedTrade {
            venue: Venue::Binance,
            symbol: "BTC-PERP".to_string(),
            ts: now - 2_000,
            price: 60_000.0,
            size_btc: 0.42,
            size_usd: 25_200.0,
            aggressor_side: AggressorSide::Buy,
            trade_id: Some("binance-btc-1".to_string()),
        });
        flow_service.add_trade_for_tests(NormalizedTrade {
            venue: Venue::Bitfinex,
            symbol: "BTC-PERP".to_string(),
            ts: now - 1_000,
            price: 60_010.0,
            size_btc: 0.33,
            size_usd: 19_803.3,
            aggressor_side: AggressorSide::Sell,
            trade_id: Some("bitfinex-btc-1".to_string()),
        });

        let outcome = state
            .flush_live_contract_flow_buckets_for_symbol("BTC")
            .await;
        assert!(outcome.attempted);
        assert!(outcome.succeeded);

        let store = state.contract_whale_store().expect("sqlite store");
        let buckets = store
            .list_contract_flow_buckets_between("BTC", now - 60_000, now + 1_000)
            .expect("contract flow rows");
        assert!(!buckets.is_empty(), "expected persisted BTC flow buckets");
        assert!(buckets.iter().all(|bucket| bucket.symbol == "BTC"));

        let second_outcome = state
            .flush_live_contract_flow_buckets_for_symbol("BTC")
            .await;
        assert!(second_outcome.succeeded);
        let buckets_after_second_flush = store
            .list_contract_flow_buckets_between("BTC", now - 60_000, now + 1_000)
            .expect("contract flow rows after second flush");
        assert_eq!(
            buckets.len(),
            buckets_after_second_flush.len(),
            "repeated flush should upsert rather than duplicate rows"
        );
    }

    fn temp_sqlite_path(name: &str) -> String {
        let unique = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("time")
            .as_nanos();
        std::env::temp_dir()
            .join(format!(
                "btc-toxic-flow-{name}-{unique}-{}.sqlite",
                std::process::id()
            ))
            .to_string_lossy()
            .to_string()
    }

    fn test_config(sqlite_path: String) -> AppConfig {
        AppConfig {
            app_env: "test".to_string(),
            read_only: true,
            api_host: "127.0.0.1".parse().expect("valid ip"),
            api_port: 0,
            symbol: "BTC-PERP".to_string(),
            toxic_volume_alert_btc: 1000.0,
            windows_ms: vec![1000, 5000, 15000, 60000],
            markout_horizons_ms: vec![1000, 5000, 15000],
            sweep_windows_ms: vec![1000, 5000, 15000],
            venues: VenueConfigs {
                binance: VenueConfig {
                    venue: Venue::Binance,
                    enabled: false,
                },
                bybit: VenueConfig {
                    venue: Venue::Bybit,
                    enabled: false,
                },
                okx: VenueConfig {
                    venue: Venue::Okx,
                    enabled: false,
                },
            },
            flow_compute_interval_ms: 50,
            markout_resolve_interval_ms: 50,
            sweep_compute_interval_ms: 50,
            toxic_compute_interval_ms: 50,
            telegram_enabled: false,
            telegram_bot_token: String::new(),
            telegram_chat_id: String::new(),
            alert_dedup_window_ms: 30_000,
            alert_min_severity: ToxicSeverity::Alert,
            alert_require_cross_venue: true,
            alert_require_markout: true,
            alert_require_liquidity_drain: false,
            sqlite_enabled: true,
            sqlite_path,
            snapshot_persist_interval_ms: 1000,
            raw_snapshot_enabled: false,
            raw_snapshot_sample_rate_ms: 1000,
            replay_enabled: false,
            replay_report_dir: ".runtime/reports".to_string(),
            vpin_enabled: true,
            vpin_bucket_size_btc: 100.0,
            vpin_lookback_buckets: 50,
            vpin_min_buckets: 10,
            vpin_spike_zscore: 2.5,
            vpin_high_threshold: 0.70,
            vpin_extreme_threshold: 0.85,
            vpin_persist_buckets: true,
            liquidation_enabled: true,
            liquidation_lookback_ms: 120_000,
            liquidation_cluster_band_bps: 6.0,
            liquidation_min_cluster_distance_bps: 5.0,
            liquidation_max_cluster_distance_bps: 150.0,
            liquidation_proximity_threshold_bps: 25.0,
            liquidation_min_cluster_touches: 3,
            liquidation_pressure_threshold: 0.65,
            liq_hunt_cluster_large_notional_usd: 50_000_000.0,
            liq_hunt_near_distance_bps: 25.0,
            liq_hunt_active_score: 75.0,
            liq_hunt_likely_score: 50.0,
            liq_hunt_watch_score: 30.0,
            book_stale_ms: 5000,
            max_buffer_age_ms: 120000,
            system_mode: SystemModeConfig::default(),
            contract_whale_monitor: ContractWhaleMonitorConfig {
                enabled: true,
                dry_run: true,
            },
            spot_whale_monitor: SpotWhaleMonitorConfig {
                enabled: false,
                dry_run: true,
            },
        }
    }
}
