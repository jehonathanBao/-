use std::sync::{
    atomic::{AtomicBool, AtomicI64, AtomicU64, Ordering},
    Arc,
};

use axum::http::{header, HeaderMap};
use parking_lot::RwLock;

pub use crate::api::contract_event_projection_runtime::ProjectionRuntimeStats;
pub use crate::api::contract_retention_runtime::ContractRetentionRuntimeStats;

use crate::{
    alerts::{
        alert_service::{AlertService, DevTestSidecarAlertInput, DevTestSidecarAlertResult},
        alert_types::AlertState,
    },
    api::{
        contract_event_projection_runtime::{
            ContractEventProjectionRuntime, ContractWhaleProjectionRuntime,
        },
        contract_retention_runtime::ContractRetentionRuntime,
        contract_whale_routes::{
            build_contract_whale_response_with_runtime_and_baselines, load_liquidation_contexts,
            load_market_context, load_quality_baselines, ContractWhaleResponseRuntime,
        },
        discord_notification_routes::{
            build_tof_anomaly_alert_request, maybe_auto_push_discord,
            preferred_discord_alert_family, DiscordNotificationRequest,
        },
        toxic_signal_inbox_routes::{
            build_recent, latest_cwm_signal_for_state, observed_tof_snapshot_for_state,
        },
        toxic_signal_ws_routes::{build_ws_snapshot_with_authoritative_state, ToxicSignalWsItem},
    },
    binance_alt_contract_monitor::{
        config as bacm_config, service::BinanceAltContractService, LOG_PREFIX as BACM_LOG_PREFIX,
        LOG_TARGET as BACM_LOG_TARGET,
    },
    config::AppConfig,
    connectors::manager::ConnectorManager,
    contract_whale_monitor::{
        aggregator::{aggregate_1s_buckets, aggregate_liquidation_1s_buckets},
        collector_binance, collector_okx,
        config::contract_whale_runtime_config,
        discord_gate::impact_grade_v3_discord_eligible,
        discord_notifier::{
            evaluate_contract_whale_discord_gate, evaluate_contract_whale_discord_v3_gate,
            global_contract_whale_discord_cooldown_store, notify_contract_whale_discord,
            notify_contract_whale_discord_v3, ContractWhaleDiscordGateDecision,
            ContractWhaleDiscordSettings,
        },
        emission::{emission_key, fingerprint, should_emit},
        hourly_delta_alert::{HourlyDeltaAlertRuntime, HourlyDeltaRuntimeDiagnostics},
        impact_forecast::{
            build_forecast, evaluate_horizon_outcomes, evaluate_trade_plan_state,
            ContractWhaleV4DecisionState, CONTRACT_WHALE_IMPACT_FORECAST_VERSION,
        },
        impact_grade::{
            apply_impact_assessment_to_signal, apply_unavailable_impact_assessment_to_signal,
        },
        impact_v4_2::{
            build_hybrid_forecast, evaluate_v42_outcomes,
            CONTRACT_WHALE_IMPACT_FORECAST_V4_2_VERSION,
        },
        impact_v4_2_gate::{evaluate_signal_gate, GateDecision},
        log_events as cwm_log_events,
        outcome_calibration::evaluate_contract_whale_signal_outcome,
        persistence::{
            backfill_contract_whale_impact_grades_nonblocking,
            flush_contract_flow_buckets_nonblocking,
            materialize_contract_whale_impact_grades_nonblocking,
            persist_contract_funding_snapshots_nonblocking,
            persist_contract_oi_snapshots_nonblocking,
            persist_contract_reference_prices_nonblocking,
            persist_contract_whale_signals_nonblocking, spawn_contract_whale_retention_task,
            ContractWhalePersistenceOutcome,
        },
        types::{
            ContractExchange, ContractFundingSnapshot, ContractLiquidationOrder,
            ContractOiSnapshot, ContractReferencePriceSnapshot, ContractTrade, ContractTradeSide,
            ContractWhaleEmissionFingerprint, ContractWhaleMarketType,
        },
        LOG_PREFIX as CWM_LOG_PREFIX, LOG_TARGET as CWM_LOG_TARGET,
    },
    market_data::{event_bus::MarketDataBus, flow_window_service::FlowWindowService},
    market_regime_engine::MarketRegimeService,
    regime_thresholds::RegimeThresholdManager,
    runtime::main_force_events::best_main_force_event_observation,
    runtime::scan_log::{ScanLogItem, ScanLogStore},
    spot_whale_monitor::service::SpotWhaleService,
    storage::{
        contract_event_grade_repo::ContractEventGradeRepo,
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
    pub oi_resolver: ContractWhaleOiResolverDiagnostics,
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

#[derive(Debug, Clone, serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ContractWhaleOiResolverDiagnostics {
    pub query_mode: String,
    pub consistent_source_count: usize,
    pub coverage_changed: bool,
}

impl Default for ContractWhaleOiResolverDiagnostics {
    fn default() -> Self {
        Self {
            query_mode: "batch_per_exchange".to_string(),
            consistent_source_count: 0,
            coverage_changed: false,
        }
    }
}

struct AppStateInner {
    config: AppConfig,
    booted_at_ms: i64,
    runtime_started: AtomicBool,
    lifecycle_lock: tokio::sync::Mutex<()>,
    runtime_control: Arc<RwLock<RuntimeControlTracker>>,
    discord_auto_push_task: Arc<RwLock<Option<tokio::task::JoinHandle<()>>>>,
    cwm_auto_push_task: Arc<RwLock<Option<tokio::task::JoinHandle<()>>>>,
    cwm_discord_outbox_task: Arc<RwLock<Option<tokio::task::JoinHandle<()>>>>,
    cwm_hourly_delta_runtime: Arc<RwLock<Option<HourlyDeltaAlertRuntime>>>,
    cwm_hourly_delta_tasks: Arc<RwLock<Vec<tokio::task::JoinHandle<()>>>>,
    cwm_outcome_calibration_task: Arc<RwLock<Option<tokio::task::JoinHandle<()>>>>,
    cwm_market_context_task: Arc<RwLock<Option<tokio::task::JoinHandle<()>>>>,
    cwm_liquidation_collector_tasks: Arc<RwLock<Vec<tokio::task::JoinHandle<()>>>>,
    cwm_producer_running: AtomicBool,
    cwm_producer_last_started_at: AtomicI64,
    cwm_producer_last_completed_at: AtomicI64,
    cwm_producer_last_duration_ms: AtomicI64,
    cwm_producer_overlap_skipped: AtomicU64,
    cwm_impact_backfill_started: AtomicBool,
    cwm_impact_maintenance_task: Arc<RwLock<Option<tokio::task::JoinHandle<()>>>>,
    cwm_oi_resolver_diagnostics: Arc<RwLock<ContractWhaleOiResolverDiagnostics>>,
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
    market_regime_service: MarketRegimeService,
    orderbook_wall_lifecycle_service: OrderbookWallLifecycleService,
    contract_passive_execution:
        crate::contract_whale_monitor::passive_execution::PassiveExecutionService,
    sustained_process_tracker:
        parking_lot::Mutex<crate::contract_whale_monitor::sustained_flow::SustainedProcessTracker>,
    alert_service: AlertService,
    snapshot_service: SnapshotService,
    storage_health: StorageHealthTracker,
    operator_api_token: Option<String>,
    contract_whale_store: Option<SqliteStore>,
    contract_whale_flow_flush_cursor_ms: Arc<RwLock<std::collections::BTreeMap<String, i64>>>,
    contract_sustained_scan_ms: Arc<RwLock<std::collections::BTreeMap<String, i64>>>,
    contract_event_projection_runtime: ContractEventProjectionRuntime,
    contract_whale_projection_runtime: ContractWhaleProjectionRuntime,
    contract_retention_runtime: ContractRetentionRuntime,
    signal_history_service: ToxicSignalHistoryService,
    whale_flow_candidate_history_service: WhaleFlowCandidateHistoryService,
    spot_whale_service: SpotWhaleService,
    binance_alt_contract_service: BinanceAltContractService,
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
        // A larger bounded buffer gives synchronous persistence and short
        // upstream reconnects enough room without allowing unbounded memory
        // growth. Consumer lag is exposed by the runtime health endpoints.
        let bus = MarketDataBus::new(16_384);
        let regime_manager = Arc::new(RegimeThresholdManager::from_runtime_config());
        let market_regime_service = MarketRegimeService::new(
            regime_manager.clone(),
            regime_manager.thresholds().refresh_interval_ms,
        );
        let flow_service = FlowWindowService::new(bus.clone(), &config);
        let markout_service = MarkoutService::new(bus.clone(), flow_service.clone(), &config);
        let sweep_service =
            SweepService::new_with_regime(flow_service.clone(), &config, regime_manager.clone());
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
        let vpin_service = VpinService::new_with_regime(
            bus.clone(),
            &config,
            shared_store.clone(),
            regime_manager.clone(),
        );
        let liquidation_service = LiquidationService::new(
            flow_service.clone(),
            sweep_service.clone(),
            vpin_service.clone(),
            &config,
        );
        let toxic_service = ToxicService::new_with_regime(
            flow_service.clone(),
            markout_service.clone(),
            sweep_service.clone(),
            vpin_service.clone(),
            liquidation_service.clone(),
            &config,
            regime_manager.clone(),
        );
        let liq_hunt_service = LiqHuntService::new_with_regime(
            flow_service.clone(),
            toxic_service.clone(),
            vpin_service.clone(),
            sweep_service.clone(),
            liquidation_service.clone(),
            &config,
            regime_manager,
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
        let binance_alt_contract_service = BinanceAltContractService::with_store(
            bacm_runtime_config.enabled,
            bacm_runtime_config.dry_run,
            booted_at_ms,
            contract_whale_store.clone(),
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
            cwm_retention,
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
                lifecycle_lock: tokio::sync::Mutex::new(()),
                runtime_control: Arc::new(RwLock::new(RuntimeControlTracker::new())),
                discord_auto_push_task: Arc::new(RwLock::new(None)),
                cwm_auto_push_task: Arc::new(RwLock::new(None)),
                cwm_discord_outbox_task: Arc::new(RwLock::new(None)),
                cwm_hourly_delta_runtime: Arc::new(RwLock::new(None)),
                cwm_hourly_delta_tasks: Arc::new(RwLock::new(Vec::new())),
                cwm_outcome_calibration_task: Arc::new(RwLock::new(None)),
                cwm_market_context_task: Arc::new(RwLock::new(None)),
                cwm_liquidation_collector_tasks: Arc::new(RwLock::new(Vec::new())),
                cwm_producer_running: AtomicBool::new(false),
                cwm_producer_last_started_at: AtomicI64::new(0),
                cwm_producer_last_completed_at: AtomicI64::new(0),
                cwm_producer_last_duration_ms: AtomicI64::new(0),
                cwm_producer_overlap_skipped: AtomicU64::new(0),
                cwm_impact_backfill_started: AtomicBool::new(false),
                cwm_impact_maintenance_task: Arc::new(RwLock::new(None)),
                cwm_oi_resolver_diagnostics: Arc::new(RwLock::new(
                    ContractWhaleOiResolverDiagnostics::default(),
                )),
                cwm_emission_watermarks: Arc::new(RwLock::new(cwm_emission_watermarks)),
                scan_log,
                contract_passive_execution:
                    crate::contract_whale_monitor::passive_execution::PassiveExecutionService::new(
                        bus.clone(),
                    ),
                sustained_process_tracker: parking_lot::Mutex::new(Default::default()),
                market_data_bus: bus,
                connector_manager,
                flow_service,
                markout_service,
                sweep_service,
                vpin_service,
                liquidation_service,
                liq_hunt_service,
                toxic_service,
                market_regime_service,
                orderbook_wall_lifecycle_service,
                alert_service,
                snapshot_service,
                storage_health,
                operator_api_token,
                contract_whale_store,
                contract_whale_flow_flush_cursor_ms: Arc::new(RwLock::new(
                    std::collections::BTreeMap::new(),
                )),
                contract_sustained_scan_ms: Arc::new(
                    RwLock::new(std::collections::BTreeMap::new()),
                ),
                contract_event_projection_runtime: ContractEventProjectionRuntime::new(),
                contract_whale_projection_runtime: ContractWhaleProjectionRuntime::new(),
                contract_retention_runtime: ContractRetentionRuntime::new(),
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
        let _lifecycle_guard = self.inner.lifecycle_lock.lock().await;
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
        self.inner.contract_passive_execution.start();
        self.inner.alert_service.start();
        self.inner.snapshot_service.start();
        self.inner.spot_whale_service.start();
        self.inner.binance_alt_contract_service.start();
        self.start_market_regime_loop();
        self.start_discord_auto_push_loop();
        self.start_contract_whale_market_context_loop();
        self.start_contract_whale_liquidation_collectors();
        self.start_contract_whale_impact_backfill_once();
        self.start_contract_whale_auto_push_loop();
        self.start_contract_whale_discord_outbox_loop();
        self.start_hourly_delta_alert_runtime();
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

    fn start_contract_whale_impact_backfill_once(&self) {
        let runtime_config = contract_whale_runtime_config();
        if !runtime_config.impact_grade_v3.enabled
            || self
                .inner
                .cwm_impact_backfill_started
                .swap(true, Ordering::SeqCst)
        {
            return;
        }
        let Some(store) = self.contract_whale_store() else {
            return;
        };
        let retention_days = runtime_config.retention.signals_days.max(1);
        let handle = tokio::spawn(async move {
            let mut interval = tokio::time::interval(std::time::Duration::from_secs(15 * 60));
            interval.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Skip);
            loop {
                interval.tick().await;
                let now_ms = crate::normalizers::trade::now_ms();
                let from_ts = now_ms.saturating_sub(retention_days * 24 * 60 * 60 * 1_000);
                match backfill_contract_whale_impact_grades_nonblocking(
                    Some(store.clone()),
                    from_ts,
                    now_ms,
                    now_ms,
                )
                .await
                {
                    Ok((signal_count, assessment_count)) => tracing::info!(
                        target: CWM_LOG_TARGET,
                        event = "cwm.impact_grade.backfill",
                        signal_count,
                        assessment_count,
                        "{} V3.2 historical impact-grade backfill completed",
                        CWM_LOG_PREFIX
                    ),
                    Err(error) => tracing::warn!(
                        target: CWM_LOG_TARGET,
                        event = cwm_log_events::ERROR,
                        error = %error,
                        "{} V3.2 historical impact-grade backfill failed",
                        CWM_LOG_PREFIX
                    ),
                }
            }
        });
        *self.inner.cwm_impact_maintenance_task.write() = Some(handle);
    }

    fn stop_contract_whale_impact_maintenance_loop(&self) {
        if let Some(handle) = self.inner.cwm_impact_maintenance_task.write().take() {
            handle.abort();
        }
    }

    pub async fn stop(&self) {
        let _ = self.ensure_monitoring_stopped().await;
    }

    pub async fn ensure_monitoring_stopped(&self) -> StopMonitoringOutcome {
        let _lifecycle_guard = self.inner.lifecycle_lock.lock().await;
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
        self.stop_contract_whale_liquidation_collectors();
        self.inner.binance_alt_contract_service.stop();
        self.inner.spot_whale_service.stop();
        self.inner.snapshot_service.stop();
        self.stop_contract_whale_auto_push_loop();
        self.stop_contract_whale_discord_outbox_loop();
        self.stop_hourly_delta_alert_runtime();
        self.stop_contract_whale_outcome_calibration_loop();
        self.stop_contract_whale_market_context_loop();
        self.stop_contract_whale_impact_maintenance_loop();
        self.stop_discord_auto_push_loop();
        self.stop_market_regime_loop();
        self.inner.alert_service.stop();
        self.inner.orderbook_wall_lifecycle_service.stop();
        self.inner.contract_passive_execution.stop();
        *self.inner.sustained_process_tracker.lock() = Default::default();
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

    fn start_market_regime_loop(&self) {
        let state = self.clone();
        self.inner
            .market_regime_service
            .start_with_provider(move || {
                crate::api::market_regime_routes::build_latest_market_features(&state, None)
            });
    }

    fn stop_market_regime_loop(&self) {
        self.inner.market_regime_service.stop();
    }

    pub fn regime_manager(&self) -> Arc<RegimeThresholdManager> {
        self.inner.market_regime_service.manager()
    }

    fn start_discord_auto_push_loop(&self) {
        if self.inner.discord_auto_push_task.read().is_some() {
            return;
        }
        let state = self.clone();
        let handle = tokio::spawn(async move {
            let mut interval = tokio::time::interval(discord_auto_push_interval());
            interval.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Skip);
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
            if contract_whale_producer_skip_missed_ticks() {
                interval.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Skip);
            }
            loop {
                interval.tick().await;
                let prevent_overlap = contract_whale_producer_prevent_overlap();
                if prevent_overlap
                    && state
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
                if prevent_overlap {
                    state
                        .inner
                        .cwm_producer_running
                        .store(false, Ordering::SeqCst);
                }
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

    /// Start the Binance force-order streams and persist one-second
    /// liquidation buckets.  This closes the former "defined_not_started"
    /// gap where liquidation evidence was always empty in V3 assessments.
    fn start_contract_whale_liquidation_collectors(&self) {
        if !self.config().contract_whale_monitor.enabled
            || self.contract_whale_store().is_none()
            || !contract_whale_runtime_config().impact_grade_v3.enabled
            || !self.inner.cwm_liquidation_collector_tasks.read().is_empty()
        {
            return;
        }
        let Some(store) = self.contract_whale_store() else {
            return;
        };
        let (tx, mut rx) = tokio::sync::mpsc::channel::<ContractLiquidationOrder>(2048);
        let mut tasks = Vec::new();
        for symbol in ["BTC", "ETH"] {
            let sender = tx.clone();
            tasks.push(tokio::spawn(async move {
                collector_binance::run_binance_force_order_collector_for_symbol(symbol, sender)
                    .await;
            }));
        }
        drop(tx);
        tasks.push(tokio::spawn(async move {
            let mut pending = Vec::with_capacity(128);
            while let Some(first) = rx.recv().await {
                pending.push(first);
                while let Ok(Some(next)) = tokio::time::timeout(
                    std::time::Duration::from_millis(250),
                    rx.recv(),
                )
                .await
                {
                    pending.push(next);
                    if pending.len() >= 512 { break; }
                }
                let buckets = aggregate_liquidation_1s_buckets(&pending);
                if let Err(error) = store.upsert_contract_liquidation_buckets(&buckets) {
                    tracing::warn!(target: CWM_LOG_TARGET, error = %error, "{} liquidation bucket persistence failed", CWM_LOG_PREFIX);
                }
                pending.clear();
            }
        }));
        *self.inner.cwm_liquidation_collector_tasks.write() = tasks;
        tracing::info!(target: CWM_LOG_TARGET, event = cwm_log_events::RUNTIME_STARTED, "{} Binance BTC/ETH forceOrder collectors started", CWM_LOG_PREFIX);
    }

    fn stop_contract_whale_liquidation_collectors(&self) {
        let mut tasks = self.inner.cwm_liquidation_collector_tasks.write();
        for task in tasks.drain(..) {
            task.abort();
        }
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
            let mut interval = tokio::time::interval(contract_whale_discord_outbox_poll_interval());
            if contract_whale_discord_outbox_skip_missed_ticks() {
                interval.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Skip);
            }
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

    fn start_hourly_delta_alert_runtime(&self) {
        let config = contract_whale_runtime_config().hourly_delta_alert;
        if !config.enabled {
            return;
        }
        if self.inner.cwm_hourly_delta_runtime.read().is_some() {
            return;
        }
        let runtime = HourlyDeltaAlertRuntime::new(
            config,
            self.config().contract_whale_monitor.dry_run,
            self.contract_whale_store(),
        );
        let handles = runtime.clone().spawn();
        *self.inner.cwm_hourly_delta_runtime.write() = Some(runtime);
        *self.inner.cwm_hourly_delta_tasks.write() = handles;
        tracing::info!(
            target: CWM_LOG_TARGET,
            event = cwm_log_events::HOURLY_DELTA_CLOSED,
            "{} hourly_delta_alert runtime started",
            CWM_LOG_PREFIX
        );
    }

    fn stop_hourly_delta_alert_runtime(&self) {
        if let Some(runtime) = self.inner.cwm_hourly_delta_runtime.write().take() {
            runtime.stop();
        }
        let handles = std::mem::take(&mut *self.inner.cwm_hourly_delta_tasks.write());
        for handle in handles {
            handle.abort();
        }
    }

    pub fn hourly_delta_runtime_diagnostics(&self) -> Option<HourlyDeltaRuntimeDiagnostics> {
        self.inner
            .cwm_hourly_delta_runtime
            .read()
            .as_ref()
            .map(|runtime| runtime.diagnostics())
    }

    async fn process_contract_whale_discord_outbox_once(&self) {
        let Some(store) = self.contract_whale_store() else {
            return;
        };
        let now = crate::normalizers::trade::now_ms();
        let claimed_store = store.clone();
        let claimed = match tokio::task::spawn_blocking(move || {
            claimed_store.claim_contract_whale_discord_outbox(
                contract_whale_discord_outbox_batch_size(),
                now,
            )
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
        let grade_config = contract_whale_runtime_config().impact_grade_v3;
        for mut item in claimed {
            if item.signal.ts <= self.booted_at_ms() {
                let finish_store = store.clone();
                let signal_id = item.signal_id.clone();
                let _ = tokio::task::spawn_blocking(move || {
                    finish_store.finish_contract_whale_discord_outbox(
                        &signal_id,
                        ContractWhaleDiscordOutboxStatus::Skipped,
                        None,
                        None,
                        Some("cached_before_boot_display_only"),
                    )
                })
                .await;
                continue;
            }
            let grade_repo = ContractEventGradeRepo::new(store.clone());
            let grade_version = grade_config.grade_version.clone();
            let sent_grade_version = grade_version.clone();
            let assessment_repo = grade_repo.clone();
            let signal_for_lookup = item.signal.clone();
            let assessment = tokio::task::spawn_blocking(move || {
                assessment_repo.get_assessment_for_signal(&signal_for_lookup, &grade_version)
            })
            .await
            .ok()
            .and_then(Result::ok)
            .flatten();
            let v3_delivery_enabled = grade_config.enabled && !grade_config.shadow_mode;
            let sent_episode_id = assessment
                .as_ref()
                .map(|assessment| assessment.episode_id.clone());
            let duplicate_grade = assessment.as_ref().is_some_and(|assessment| {
                impact_grade_v3_discord_eligible(assessment)
                    && grade_repo
                        .episode_alert_already_sent(
                            &assessment.episode_id,
                            &assessment.grade_version,
                        )
                        .unwrap_or(false)
            });
            if duplicate_grade {
                let finish_store = store.clone();
                let signal_id = item.signal_id.clone();
                let _ = tokio::task::spawn_blocking(move || {
                    finish_store.finish_contract_whale_discord_outbox(
                        &signal_id,
                        ContractWhaleDiscordOutboxStatus::Skipped,
                        None,
                        None,
                        Some("confirmed_impact_episode_already_sent"),
                    )
                })
                .await;
                continue;
            }
            if let Some(assessment) = assessment.as_ref() {
                apply_impact_assessment_to_signal(&mut item.signal, assessment);
            }
            let outcome = if v3_delivery_enabled {
                if assessment
                    .as_ref()
                    .is_some_and(impact_grade_v3_discord_eligible)
                {
                    notify_contract_whale_discord_v3(
                        &settings,
                        &item.signal,
                        assessment.as_ref().expect("assessment checked above"),
                        Some(store.clone()),
                        global_contract_whale_discord_cooldown_store(),
                    )
                    .await
                } else {
                    let finish_store = store.clone();
                    let signal_id = item.signal_id.clone();
                    let _ = tokio::task::spawn_blocking(move || {
                        finish_store.finish_contract_whale_discord_outbox(
                            &signal_id,
                            ContractWhaleDiscordOutboxStatus::Skipped,
                            None,
                            None,
                            Some("v3_grade_not_confirmed"),
                        )
                    })
                    .await;
                    continue;
                }
            } else {
                let base = evaluate_contract_whale_discord_gate(
                    &settings,
                    &item.signal,
                    global_contract_whale_discord_cooldown_store(),
                    now,
                );
                let decision = self.merge_v42_gate_decision(&item.signal, base, Some(&store), now);
                if !decision.allowed {
                    let finish_store = store.clone();
                    let signal_id = item.signal_id.clone();
                    let reason = format!("v42_gate_{}", decision.reason);
                    let _ = tokio::task::spawn_blocking(move || {
                        finish_store.finish_contract_whale_discord_outbox(
                            &signal_id,
                            ContractWhaleDiscordOutboxStatus::Skipped,
                            None,
                            None,
                            Some(&reason),
                        )
                    })
                    .await;
                    continue;
                }
                notify_contract_whale_discord(&settings, &item.signal, Some(store.clone())).await
            };
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
            if outcome.sent && sent_episode_id.is_some() {
                let marker_store = store.clone();
                let sent_episode_id = sent_episode_id.expect("episode id checked above");
                let _ = tokio::task::spawn_blocking(move || {
                    ContractEventGradeRepo::new(marker_store).mark_episode_alert_sent(
                        &sent_episode_id,
                        &sent_grade_version,
                        outcome
                            .sent_at_ms
                            .unwrap_or_else(crate::normalizers::trade::now_ms),
                    )
                })
                .await;
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
            let mut interval = tokio::time::interval(std::time::Duration::from_secs(30));
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
        let impact_grade_config = contract_whale_runtime_config().impact_grade_v3;
        let result =
            tokio::task::spawn_blocking(move || -> anyhow::Result<(usize, usize, usize, usize, usize)> {
                let mut signals =
                    evaluation_store.query_contract_whale_signals(&ContractWhaleSignalQuery {
                        from_ts: Some(now.saturating_sub(24 * 60 * 60 * 1_000)),
                        to_ts: Some(now.saturating_sub(30_000)),
                        limit: 500,
                        ..ContractWhaleSignalQuery::default()
                    })?;
                if impact_grade_config.enabled {
                    let grade_repo = ContractEventGradeRepo::new(evaluation_store.clone());
                    for signal in &mut signals {
                        match grade_repo
                            .get_assessment_for_signal(signal, &impact_grade_config.grade_version)?
                        {
                            Some(assessment) => {
                                apply_impact_assessment_to_signal(signal, &assessment)
                            }
                            None => apply_unavailable_impact_assessment_to_signal(
                                signal,
                                &impact_grade_config.grade_version,
                                "v3_assessment_unavailable",
                            ),
                        }
                    }
                }
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
                let legacy_written =
                    evaluation_store.upsert_contract_whale_signal_outcomes(&outcomes)?;

                // V4 snapshots are built from event history only.  The historical
                // set is loaded before evaluating each new event and the pure
                // builder applies `event_ts < signal.ts` again as a second guard.
                let mut v4_signals =
                    evaluation_store.query_contract_whale_signals(&ContractWhaleSignalQuery {
                        // The live loop owns immediate forecasts and maturing
                        // outcomes. Long-range replay is handled by the
                        // resumable chronological backfill command.
                        from_ts: Some(now.saturating_sub(2 * 24 * 60 * 60 * 1_000)),
                        to_ts: Some(now),
                        limit: 5_000,
                        ..ContractWhaleSignalQuery::default()
                    })?;
                // V4.1 is deliberately fail-closed to Binance evidence. This
                // also prevents legacy non-Binance rows left in the database
                // from being reclassified as current production forecasts.
                v4_signals.retain(|signal| {
                    let perp_binance_only = signal
                        .active_contract_sources
                        .iter()
                        .all(|source| source.eq_ignore_ascii_case("binance"))
                        && signal
                            .active_contract_sources
                            .iter()
                            .any(|source| source.eq_ignore_ascii_case("binance"));
                    let contribution_binance_only = signal.exchanges.iter().all(|contribution| {
                        contribution.exchange.eq_ignore_ascii_case("binance")
                    }) && signal
                        .exchanges
                        .iter()
                        .any(|contribution| contribution.exchange.eq_ignore_ascii_case("binance"));
                    perp_binance_only || contribution_binance_only
                });
                v4_signals.sort_by(|left, right| {
                    left.ts.cmp(&right.ts).then_with(|| left.id.cmp(&right.id))
                });
                let mut historical_by_symbol = std::collections::BTreeMap::new();
                for signal in &v4_signals {
                    if !historical_by_symbol.contains_key(&signal.symbol) {
                        historical_by_symbol.insert(
                            signal.symbol.clone(),
                            evaluation_store.list_contract_whale_horizon_outcomes_before(
                                &signal.symbol,
                                now,
                                20_000,
                            )?,
                        );
                    }
                }
                let mut v4_outcomes = Vec::new();
                let event_ids = v4_signals.iter().map(crate::contract_whale_monitor::impact_forecast::event_id).collect::<Vec<_>>();
                let mut frozen_base = std::collections::BTreeMap::new();
                let mut frozen_hybrid = std::collections::BTreeMap::new();
                for chunk in event_ids.chunks(500) {
                    let ids = chunk.iter().map(String::as_str).collect::<Vec<_>>();
                    frozen_base.extend(evaluation_store.load_contract_whale_impact_forecasts(&ids, CONTRACT_WHALE_IMPACT_FORECAST_VERSION)?);
                    frozen_hybrid.extend(evaluation_store.load_contract_whale_impact_forecasts(&ids, CONTRACT_WHALE_IMPACT_FORECAST_V4_2_VERSION)?);
                }
                let mut v4_forecasts = Vec::new();
                let mut v42_outcomes = Vec::new();
                let mut v42_forecasts = Vec::new();
                let mut v4_decision_states = Vec::new();
                let mut v42_decision_states = Vec::new();
                for signal in contract_calibration_anchors(v4_signals, &frozen_base, &frozen_hybrid) {
                    let event_id = crate::contract_whale_monitor::impact_forecast::event_id(&signal);
                    let history = historical_by_symbol.get(&signal.symbol).map(Vec::as_slice).unwrap_or(&[]);
                    if frozen_base.contains_key(&event_id) && frozen_hybrid.contains_key(&event_id)
                        && !contract_calibration_due(&event_id, signal.ts, history, now) {
                        continue;
                    }
                    let from_ts = signal.ts.saturating_sub(4 * 60 * 60 * 1_000);
                    let to_ts = now.min(signal.ts.saturating_add(86_400_000));
                    let buckets = evaluation_store.list_contract_flow_buckets_between(
                        &signal.symbol,
                        from_ts,
                        to_ts,
                    )?;
                    let reference_prices = evaluation_store
                        .list_contract_reference_prices_between(&signal.symbol, from_ts, to_ts)?;
                    let oi_snapshots = evaluation_store
                        .list_contract_oi_snapshots_between(&signal.symbol, from_ts, to_ts)?;
                    let funding_snapshots = evaluation_store
                        .list_contract_funding_snapshots_between(&signal.symbol, from_ts, to_ts)?;
                    let liquidation_buckets = evaluation_store
                        .list_contract_liquidation_buckets_between(&signal.symbol, signal.ts, to_ts)?;
                    if let Some(history) = historical_by_symbol.get_mut(&signal.symbol) {
                        let v4_forecast = frozen_base.get(&event_id).cloned().unwrap_or_else(|| build_forecast(
                            &signal,
                            history,
                            &reference_prices,
                            now,
                        ));
                        v4_forecasts.push(v4_forecast.clone());
                        v42_forecasts.push(frozen_hybrid.get(&event_id).cloned().unwrap_or_else(|| build_hybrid_forecast(
                            &signal,
                            history,
                            &reference_prices,
                            now,
                        )));
                        let outcomes = evaluate_horizon_outcomes(
                            &signal,
                            crate::contract_whale_monitor::impact_forecast::ContractWhaleOutcomeInputs {
                                flow_buckets: &buckets,
                                reference_prices: &reference_prices,
                                oi_snapshots: &oi_snapshots,
                                funding_snapshots: &funding_snapshots,
                                liquidation_buckets: &liquidation_buckets,
                            },
                            now,
                        );
                        let (decision_state, decision_reason) = evaluate_trade_plan_state(
                            &v4_forecast,
                            &outcomes,
                            now,
                        );
                        v4_decision_states.push(ContractWhaleV4DecisionState {
                            event_id: v4_forecast.event_id.clone(),
                            forecast_version: CONTRACT_WHALE_IMPACT_FORECAST_VERSION.to_string(),
                            state: decision_state,
                            reason: decision_reason,
                            updated_at_ms: now,
                            decided_at_ms: Some(now),
                        });
                        let v42_forecast = v42_forecasts.last().expect("v4.2 forecast present");
                        let v42_values = evaluate_v42_outcomes(
                            &signal,
                            crate::contract_whale_monitor::impact_forecast::ContractWhaleOutcomeInputs {
                                flow_buckets: &buckets,
                                reference_prices: &reference_prices,
                                oi_snapshots: &oi_snapshots,
                                funding_snapshots: &funding_snapshots,
                                liquidation_buckets: &liquidation_buckets,
                            },
                            now,
                        );
                        let (v42_state, v42_reason) = evaluate_trade_plan_state(v42_forecast, &v42_values, now);
                        v42_decision_states.push(ContractWhaleV4DecisionState {
                            event_id: v42_forecast.event_id.clone(),
                            forecast_version: CONTRACT_WHALE_IMPACT_FORECAST_V4_2_VERSION.to_string(),
                            state: v42_state,
                            reason: v42_reason,
                            updated_at_ms: now,
                            decided_at_ms: Some(now),
                        });
                        v4_outcomes.extend(outcomes);
                        v42_outcomes.extend(v42_values);
                    }
                }
                let v4_outcome_written =
                    evaluation_store.upsert_contract_whale_horizon_outcomes(&v4_outcomes)?;
                let v4_forecast_written =
                    evaluation_store.upsert_contract_whale_impact_forecasts(&v4_forecasts)?;
                evaluation_store.upsert_contract_whale_v4_decision_states(&v4_decision_states)?;
                let v42_outcome_written =
                    evaluation_store.upsert_contract_whale_horizon_outcomes(&v42_outcomes)?;
                let v42_forecast_written =
                    evaluation_store.upsert_contract_whale_impact_forecasts(&v42_forecasts)?;
                evaluation_store.upsert_contract_whale_v4_decision_states(&v42_decision_states)?;
                Ok((legacy_written, v4_outcome_written, v4_forecast_written, v42_outcome_written, v42_forecast_written))
            })
            .await;
        match result {
            Ok(Ok((
                legacy_written,
                v4_outcome_written,
                v4_forecast_written,
                v42_outcome_written,
                v42_forecast_written,
            ))) if legacy_written
                + v4_outcome_written
                + v4_forecast_written
                + v42_outcome_written
                + v42_forecast_written
                > 0 =>
            {
                tracing::debug!(
                    target: CWM_LOG_TARGET,
                    outcomes = legacy_written,
                    v4_outcomes = v4_outcome_written,
                    v4_forecasts = v4_forecast_written,
                    v42_outcomes = v42_outcome_written,
                    v42_forecasts = v42_forecast_written,
                    "{} contract whale outcomes and forecasts updated",
                    CWM_LOG_PREFIX
                )
            }
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
        let runtime_config = Arc::new(contract_whale_runtime_config());
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
        let mut reference_prices = Vec::<ContractReferencePriceSnapshot>::new();
        let fallback_ts = crate::normalizers::trade::now_ms();

        let symbol_results = futures_util::future::join_all(symbols.into_iter().map(|symbol| {
            let runtime_config = runtime_config.clone();
            async move {
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
            let binance_reference_prices = async {
                collector_binance::fetch_binance_reference_prices_for_symbol(
                    client,
                    &symbol,
                    fallback_ts,
                )
                .await
            };
            let okx_oi = async {
                if runtime_config
                    .exchanges
                    .okx
                    .market_enabled(ContractWhaleMarketType::Oi)
                {
                    let fallback_ct_val =
                        runtime_config.okx_instruments.fallback_ct_val_base(&symbol);
                    match collector_okx::resolve_okx_contract_value_with_cache(
                        client,
                        &symbol,
                        runtime_config.okx_instruments.metadata_enabled,
                        runtime_config.okx_instruments.refresh_minutes,
                        fallback_ct_val,
                    )
                    .await
                    {
                        Ok(resolution) if resolution.ct_val_base.is_some() => {
                            let mut snapshot =
                                collector_okx::fetch_okx_open_interest_snapshot_for_symbol(
                                    client,
                                    &symbol,
                                    resolution.ct_val_base.unwrap_or_default(),
                                )
                                .await?;
                            if let Some(snapshot) = snapshot.as_mut() {
                                snapshot.ct_val_available = resolution.ct_val_available;
                                snapshot.evidence_degraded_reason = resolution.reason.clone();
                            }
                            Ok(snapshot)
                        }
                        Ok(resolution) => {
                            tracing::warn!(
                                target: CWM_LOG_TARGET,
                                event = cwm_log_events::ERROR,
                                symbol = symbol.as_str(),
                                exchange = "okx",
                                context = "ct_val",
                                reason = ?resolution.reason,
                                "{} OKX instrument metadata unavailable and no configured fallback; OI snapshot skipped",
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
            let (binance_oi, binance_funding, binance_reference_prices, okx_oi, okx_funding) =
                tokio::join!(
                    binance_oi,
                    binance_funding,
                    binance_reference_prices,
                    okx_oi,
                    okx_funding
                );

                (
                    symbol,
                    (
                        binance_oi,
                        binance_funding,
                        binance_reference_prices,
                        okx_oi,
                        okx_funding,
                    ),
                )
            }
        }))
        .await;

        for (symbol, (binance_oi, binance_funding, binance_reference, okx_oi, okx_funding)) in
            symbol_results
        {
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
            match binance_reference {
                Ok(mut rows) => reference_prices.append(&mut rows),
                Err(error) => tracing::warn!(
                    target: CWM_LOG_TARGET,
                    event = cwm_log_events::ERROR,
                    symbol = symbol.as_str(),
                    exchange = "binance",
                    context = "reference_price",
                    error = %error,
                    "{} Binance reference-price fetch failed",
                    CWM_LOG_PREFIX
                ),
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
            persist_contract_funding_snapshots_nonblocking(store.clone(), funding_snapshots).await;
        let reference_outcome =
            persist_contract_reference_prices_nonblocking(store, reference_prices).await;

        if oi_outcome.written > 0 || funding_outcome.written > 0 || reference_outcome.written > 0 {
            tracing::info!(
                target: CWM_LOG_TARGET,
                event = "contract_market_context_poll",
                oi_written = oi_outcome.written,
                funding_written = funding_outcome.written,
                reference_price_written = reference_outcome.written,
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
            let flow_buckets = self.contract_evidence_buckets(&symbol, flow_state.updated_at);
            let response = build_contract_whale_response_with_runtime_and_baselines(
                &flow_state,
                &symbol,
                10,
                None,
                config.enabled,
                config.dry_run,
                ContractWhaleResponseRuntime {
                    flow_buckets: &flow_buckets,
                    venue_health: Some(&venue_health),
                    baselines: &baselines,
                    liquidations: &liquidations,
                    market_context: &market_context,
                    booted_at_ms: Some(self.booted_at_ms()),
                },
            );
            let settings = ContractWhaleDiscordSettings::from_env(config.dry_run);
            let mut candidates = response.items;
            candidates.extend(self.sustained_contract_candidates(&symbol).await);
            crate::api::contract_whale_routes::enrich_production_evidence(self, &mut candidates);
            for signal in &mut candidates {
                crate::contract_whale_monitor::sustained_flow::set_episode_identity(signal);
                self.inner.sustained_process_tracker.lock().attach(signal);
            }
            let mut impact_grade_materialization_failed = false;
            if runtime_config.impact_grade_v3.enabled {
                if let Err(error) = materialize_contract_whale_impact_grades_nonblocking(
                    store.clone(),
                    candidates.clone(),
                    crate::normalizers::trade::now_ms(),
                )
                .await
                {
                    impact_grade_materialization_failed = true;
                    tracing::warn!(
                        target: CWM_LOG_TARGET,
                        event = cwm_log_events::ERROR,
                        error = %error,
                        "{} impact grade materialization failed",
                        CWM_LOG_PREFIX
                    );
                }
            }
            let v3_delivery_enabled = runtime_config.impact_grade_v3.enabled
                && !runtime_config.impact_grade_v3.shadow_mode;
            let cooldown_store = global_contract_whale_discord_cooldown_store();
            let now = crate::normalizers::trade::now_ms();
            let grade_repo = store.clone().map(ContractEventGradeRepo::new);
            for signal in &mut candidates {
                let assessment = grade_repo.as_ref().and_then(|repo| {
                    repo.get_assessment_for_signal(
                        signal,
                        &runtime_config.impact_grade_v3.grade_version,
                    )
                    .ok()
                    .flatten()
                });
                if let Some(assessment) = assessment.as_ref() {
                    apply_impact_assessment_to_signal(signal, assessment);
                } else if runtime_config.impact_grade_v3.enabled {
                    apply_unavailable_impact_assessment_to_signal(
                        signal,
                        &runtime_config.impact_grade_v3.grade_version,
                        if impact_grade_materialization_failed {
                            "v3_assessment_failed"
                        } else {
                            "v3_assessment_unavailable"
                        },
                    );
                }
                if v3_delivery_enabled {
                    let decision = assessment.as_ref().map_or_else(
                        || ContractWhaleDiscordGateDecision {
                            allowed: false,
                            reason: if impact_grade_materialization_failed {
                                "v3_assessment_failed".to_string()
                            } else {
                                "v3_assessment_unavailable".to_string()
                            },
                        },
                        |assessment| {
                            evaluate_contract_whale_discord_v3_gate(
                                &settings,
                                signal,
                                assessment,
                                cooldown_store,
                                now,
                            )
                        },
                    );
                    signal.discord_eligible = decision.allowed;
                    signal.discord_would_send = decision.allowed;
                    signal.discord_reason = decision.reason;
                }
            }
            // Emission watermarks must observe the canonical grade. Filtering
            // before materialization would allow a legacy detector grade to
            // suppress an event whose V3 hard evidence just changed.
            let signals = self.filter_contract_whale_emissions(candidates);
            if contract_whale_discord_outbox_enabled() {
                let queued = signals
                    .iter()
                    .filter(|signal| {
                        let v3_assessment = grade_repo.as_ref().and_then(|repo| {
                            repo.get_assessment_for_signal(
                                signal,
                                &runtime_config.impact_grade_v3.grade_version,
                            )
                            .ok()
                            .flatten()
                        });
                        let decision = if v3_delivery_enabled {
                            v3_assessment.as_ref().map_or_else(
                                || ContractWhaleDiscordGateDecision {
                                    allowed: false,
                                    reason: "v3_assessment_unavailable".to_string(),
                                },
                                |assessment| {
                                    evaluate_contract_whale_discord_v3_gate(
                                        &settings,
                                        signal,
                                        assessment,
                                        cooldown_store,
                                        now,
                                    )
                                },
                            )
                        } else {
                            let base = evaluate_contract_whale_discord_gate(
                                &settings,
                                signal,
                                cooldown_store,
                                now,
                            );
                            self.merge_v42_gate_decision(signal, base, store.as_ref(), now)
                        };
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
                    let signal_count = signals.len();
                    let queued_count = queued.len();
                    let transaction_result = tokio::task::spawn_blocking(move || {
                        store.upsert_contract_whale_signals_with_outbox(&signals, &queued, now)
                    })
                    .await;
                    match transaction_result {
                        Ok(Ok((written, inserted))) => tracing::info!(
                            target: CWM_LOG_TARGET,
                            event = cwm_log_events::DISCORD_ELIGIBLE,
                            signal_count,
                            written,
                            queued = inserted,
                            eligible = queued_count,
                            "{} signal and discord outbox transaction committed",
                            CWM_LOG_PREFIX
                        ),
                        Ok(Err(error)) => tracing::warn!(
                            target: CWM_LOG_TARGET,
                            event = cwm_log_events::ERROR,
                            error = %error,
                            "{} signal and discord outbox transaction failed",
                            CWM_LOG_PREFIX
                        ),
                        Err(error) => tracing::warn!(
                            target: CWM_LOG_TARGET,
                            event = cwm_log_events::ERROR,
                            error = %error,
                            "{} signal and discord outbox transaction task failed",
                            CWM_LOG_PREFIX
                        ),
                    }
                } else {
                    let _ = persist_contract_whale_signals_nonblocking(None, signals).await;
                }
            } else {
                let _ = persist_contract_whale_signals_nonblocking(store.clone(), signals.clone())
                    .await;
                for signal in signals {
                    let assessment = grade_repo.as_ref().and_then(|repo| {
                        repo.get_assessment_for_signal(
                            &signal,
                            &runtime_config.impact_grade_v3.grade_version,
                        )
                        .ok()
                        .flatten()
                    });
                    let outcome = if v3_delivery_enabled {
                        if assessment
                            .as_ref()
                            .is_some_and(impact_grade_v3_discord_eligible)
                        {
                            notify_contract_whale_discord_v3(
                                &settings,
                                &signal,
                                assessment.as_ref().expect("assessment checked above"),
                                store.clone(),
                                global_contract_whale_discord_cooldown_store(),
                            )
                            .await
                        } else {
                            self.record_scan_log(
                                "debug",
                                cwm_log_events::DISCORD_SKIPPED,
                                format!(
                                    "{} discord skipped for {}: v3_grade_not_confirmed",
                                    CWM_LOG_PREFIX, signal.symbol
                                ),
                                Some(signal.symbol.clone()),
                                Some(signal.id.clone()),
                            );
                            continue;
                        }
                    } else {
                        let base = evaluate_contract_whale_discord_gate(
                            &settings,
                            &signal,
                            global_contract_whale_discord_cooldown_store(),
                            now,
                        );
                        let decision =
                            self.merge_v42_gate_decision(&signal, base, store.as_ref(), now);
                        if !decision.allowed {
                            self.record_scan_log(
                                "debug",
                                cwm_log_events::DISCORD_SKIPPED,
                                format!(
                                    "{} discord skipped for {}: {}",
                                    CWM_LOG_PREFIX, signal.symbol, decision.reason
                                ),
                                Some(signal.symbol.clone()),
                                Some(signal.id.clone()),
                            );
                            continue;
                        }
                        notify_contract_whale_discord(&settings, &signal, store.clone()).await
                    };
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

    fn merge_v42_gate_decision(
        &self,
        signal: &crate::contract_whale_monitor::types::ContractWhaleSignal,
        base: ContractWhaleDiscordGateDecision,
        store: Option<&SqliteStore>,
        now: i64,
    ) -> ContractWhaleDiscordGateDecision {
        let config = contract_whale_runtime_config();
        if !config.impact_v4_2.enabled {
            return base;
        }
        let Some(store) = store else {
            return ContractWhaleDiscordGateDecision {
                allowed: false,
                reason: "v42_gate_store_unavailable".to_string(),
            };
        };
        if !base.allowed {
            return base;
        }
        let gate: GateDecision = evaluate_signal_gate(store, signal, &config.impact_v4_2, now)
            .unwrap_or_else(|error| GateDecision {
                allowed: false,
                state: "FORCED_CLOSED".to_string(),
                reason: format!("v42_gate_evaluation_failed:{error}"),
                ..Default::default()
            });
        if gate.allowed {
            base
        } else {
            ContractWhaleDiscordGateDecision {
                allowed: false,
                reason: format!("v42_gate_{}", gate.reason),
            }
        }
    }

    pub(crate) fn contract_passive_evidence(
        &self,
        symbol: &str,
        at: i64,
        window: u64,
    ) -> crate::contract_whale_monitor::passive_execution::PassiveExecutionEvidence {
        self.inner
            .contract_passive_execution
            .evidence(symbol, at, window)
    }

    async fn sustained_contract_candidates(
        &self,
        symbol: &str,
    ) -> Vec<crate::contract_whale_monitor::types::ContractWhaleSignal> {
        let now = crate::normalizers::trade::now_ms();
        {
            let mut scans = self.inner.contract_sustained_scan_ms.write();
            if scans
                .get(symbol)
                .is_some_and(|last| now.saturating_sub(*last) < 60_000)
            {
                return Vec::new();
            }
            scans.insert(symbol.to_string(), now);
        }
        let Some(store) = self.contract_whale_store() else {
            return Vec::new();
        };
        let symbol = symbol.to_string();
        let booted = self.booted_at_ms();
        let config = contract_whale_runtime_config();
        match tokio::task::spawn_blocking(move || -> anyhow::Result<Vec<crate::contract_whale_monitor::types::ContractWhaleSignal>> {
            load_sustained_contract_candidates(&store, &symbol, now, booted, &config)
        }).await {
            Ok(Ok(signals)) => signals,
            _ => { tracing::warn!(target: CWM_LOG_TARGET, "sustained public-flow evidence unavailable"); Vec::new() }
        }
    }

    pub(crate) fn contract_evidence_buckets(
        &self,
        symbol: &str,
        at: i64,
    ) -> Vec<crate::contract_whale_monitor::types::ContractFlowBucket> {
        let canonical = contract_flow_base_asset(symbol);
        let trades = self
            .inner
            .flow_service
            .get_trades_since(at.saturating_sub(300_000));
        let trades = trades
            .iter()
            .filter(|trade| trade.ts <= at)
            .filter_map(|trade| normalized_trade_to_contract_trade(trade, &canonical))
            .collect::<Vec<_>>();
        aggregate_1s_buckets(&trades)
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
            let cwm_signal = latest_cwm_signal_for_state(self, &symbol);
            let tof_snapshot = observed_tof_snapshot_for_state(self, &symbol);
            let snapshot = build_ws_snapshot_with_authoritative_state(
                &recent,
                cwm_signal.as_ref(),
                tof_snapshot.as_ref(),
                self.runtime_started(),
            );
            self.observe_main_force_events(&symbol, &snapshot.signals)
                .await;

            if let Some(tof_snapshot) = tof_snapshot.as_ref() {
                if let Some(request) = build_tof_anomaly_alert_request(
                    tof_snapshot,
                    crate::normalizers::trade::now_ms(),
                ) {
                    let _ = maybe_auto_push_discord(
                        self,
                        request,
                        tof_snapshot.observed_at_ms.max(0) as u64,
                    )
                    .await;
                }
            }

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
            oi_resolver: self.inner.cwm_oi_resolver_diagnostics.read().clone(),
        }
    }

    pub fn record_contract_whale_oi_resolver_diagnostics(
        &self,
        diagnostics: ContractWhaleOiResolverDiagnostics,
    ) {
        *self.inner.cwm_oi_resolver_diagnostics.write() = diagnostics;
    }

    pub(crate) fn contract_event_projection_runtime(&self) -> ContractEventProjectionRuntime {
        self.inner.contract_event_projection_runtime.clone()
    }

    pub fn set_contract_event_projection_delay_for_tests(&self, delay: std::time::Duration) {
        self.inner
            .contract_event_projection_runtime
            .set_forced_delay(delay);
    }

    pub fn set_contract_event_projection_wait_budget_for_tests(
        &self,
        wait_budget: std::time::Duration,
    ) {
        self.inner
            .contract_event_projection_runtime
            .set_wait_budget(wait_budget);
    }

    pub async fn expire_contract_event_projection_cache_for_tests(&self, age: std::time::Duration) {
        self.inner
            .contract_event_projection_runtime
            .expire_cache_by(age)
            .await;
    }

    pub fn contract_event_projection_stats_for_tests(&self) -> ProjectionRuntimeStats {
        self.inner.contract_event_projection_runtime.stats()
    }

    pub(crate) fn contract_whale_projection_runtime(&self) -> ContractWhaleProjectionRuntime {
        self.inner.contract_whale_projection_runtime.clone()
    }

    pub fn set_contract_whale_projection_delay_for_tests(&self, delay: std::time::Duration) {
        self.inner
            .contract_whale_projection_runtime
            .set_forced_delay(delay);
    }

    pub fn set_contract_whale_projection_wait_budget_for_tests(
        &self,
        wait_budget: std::time::Duration,
    ) {
        self.inner
            .contract_whale_projection_runtime
            .set_wait_budget(wait_budget);
    }

    pub fn contract_whale_projection_stats_for_tests(&self) -> ProjectionRuntimeStats {
        self.inner.contract_whale_projection_runtime.stats()
    }

    pub(crate) fn contract_retention_runtime(&self) -> ContractRetentionRuntime {
        self.inner.contract_retention_runtime.clone()
    }

    pub fn set_contract_retention_delay_for_tests(&self, delay: std::time::Duration) {
        self.inner
            .contract_retention_runtime
            .set_forced_delay(delay);
    }

    pub fn contract_retention_stats_for_tests(&self) -> ContractRetentionRuntimeStats {
        self.inner.contract_retention_runtime.stats()
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

    pub fn price_snapshot_at_or_before_for_symbol(
        &self,
        ts: i64,
        symbol: &str,
    ) -> Option<crate::market_data::price_index::PriceSnapshot> {
        self.inner
            .flow_service
            .get_price_snapshot_at_or_before_for_symbol(ts, symbol)
    }

    pub fn price_snapshots_since(
        &self,
        ts: i64,
    ) -> Vec<crate::market_data::price_index::PriceSnapshot> {
        self.inner.flow_service.get_price_snapshots_since(ts)
    }

    pub fn price_snapshots_since_for_symbol(
        &self,
        ts: i64,
        symbol: &str,
    ) -> Vec<crate::market_data::price_index::PriceSnapshot> {
        self.inner
            .flow_service
            .get_price_snapshots_since_for_symbol(ts, symbol)
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
    let configured = contract_whale_runtime_config().producer.interval_ms;
    let ms = std::env::var("CONTRACT_WHALE_AUTO_PUSH_INTERVAL_MS")
        .ok()
        .and_then(|value| value.parse::<u64>().ok())
        .filter(|value| (1_000..=60_000).contains(value))
        .unwrap_or(configured.clamp(1_000, 60_000));
    std::time::Duration::from_millis(ms)
}

fn contract_whale_discord_outbox_enabled() -> bool {
    env_bool_setting(
        "CONTRACT_WHALE_DISCORD_OUTBOX_ENABLED",
        contract_whale_runtime_config().discord_outbox.enabled,
    )
}

fn contract_whale_discord_outbox_poll_interval() -> std::time::Duration {
    let configured = contract_whale_runtime_config()
        .discord_outbox
        .poll_interval_ms;
    let ms = std::env::var("CONTRACT_WHALE_DISCORD_OUTBOX_POLL_INTERVAL_MS")
        .ok()
        .and_then(|value| value.parse::<u64>().ok())
        .filter(|value| (100..=60_000).contains(value))
        .unwrap_or(configured.clamp(100, 60_000));
    std::time::Duration::from_millis(ms)
}

fn contract_whale_discord_outbox_batch_size() -> usize {
    let configured = contract_whale_runtime_config().discord_outbox.batch_size;
    std::env::var("CONTRACT_WHALE_DISCORD_OUTBOX_BATCH_SIZE")
        .ok()
        .and_then(|value| value.parse::<usize>().ok())
        .filter(|value| (1..=100).contains(value))
        .unwrap_or(configured.clamp(1, 100))
}

fn contract_whale_discord_outbox_skip_missed_ticks() -> bool {
    env_bool_setting("CONTRACT_WHALE_DISCORD_OUTBOX_SKIP_MISSED_TICKS", true)
}

fn contract_whale_producer_prevent_overlap() -> bool {
    env_bool_setting(
        "CONTRACT_WHALE_PRODUCER_PREVENT_OVERLAP",
        contract_whale_runtime_config().producer.prevent_overlap,
    )
}

fn contract_whale_producer_skip_missed_ticks() -> bool {
    env_bool_setting(
        "CONTRACT_WHALE_PRODUCER_SKIP_MISSED_TICKS",
        contract_whale_runtime_config().producer.skip_missed_ticks,
    )
}

fn env_bool_setting(name: &str, default: bool) -> bool {
    std::env::var(name)
        .ok()
        .and_then(|value| match value.trim().to_ascii_lowercase().as_str() {
            "1" | "true" | "yes" | "on" => Some(true),
            "0" | "false" | "no" | "off" => Some(false),
            _ => None,
        })
        .unwrap_or(default)
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
    let outbox_config = contract_whale_runtime_config().discord_outbox;
    let base_retry_ms = outbox_config
        .base_retry_seconds
        .max(1)
        .saturating_mul(1_000);
    let max_retry_ms = outbox_config
        .max_retry_seconds
        .max(outbox_config.base_retry_seconds)
        .saturating_mul(1_000);
    let exponential_ms = base_retry_ms
        .saturating_mul(1_i64 << attempts.min(8))
        .min(max_retry_ms);
    let jitter_range = (exponential_ms / 5).max(1);
    let jitter_ms = signal_id.bytes().fold(0_u64, |hash, byte| {
        hash.wrapping_mul(31).wrapping_add(byte as u64)
    }) % (jitter_range as u64 * 2 + 1);
    exponential_ms
        .saturating_add(jitter_ms as i64 - jitter_range)
        .clamp(1_000, max_retry_ms)
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

fn load_sustained_contract_candidates(
    store: &impl ContractWhaleRepo,
    symbol: &str,
    now: i64,
    booted: i64,
    config: &crate::contract_whale_monitor::config::ContractWhaleRuntimeConfig,
) -> anyhow::Result<Vec<crate::contract_whale_monitor::types::ContractWhaleSignal>> {
    let cutoff = crate::contract_whale_monitor::sustained_flow::closed_window_end(now) - 1;
    let context = load_market_context(
        store,
        &crate::types::flow::FlowState {
            symbol: symbol.into(),
            updated_at: cutoff,
            windows: Default::default(),
        },
        symbol,
    );
    let buckets = store.list_contract_flow_buckets_between(
        symbol,
        cutoff.saturating_sub(3 * 3_600_000),
        cutoff,
    )?;
    Ok(crate::contract_whale_monitor::sustained_flow::candidates(
        &buckets, symbol, now, booted, &context, config,
    ))
}

/// One immutable event-time/hypothesis anchor per frozen episode. If the retained
/// query no longer includes that anchor, skip it rather than substitute a later signal.
fn contract_calibration_anchors(
    mut signals: Vec<crate::contract_whale_monitor::types::ContractWhaleSignal>,
    frozen_base: &std::collections::BTreeMap<
        String,
        crate::contract_whale_monitor::impact_forecast::ContractWhaleMultiHorizonImpactForecast,
    >,
    frozen_hybrid: &std::collections::BTreeMap<
        String,
        crate::contract_whale_monitor::impact_forecast::ContractWhaleMultiHorizonImpactForecast,
    >,
) -> Vec<crate::contract_whale_monitor::types::ContractWhaleSignal> {
    use crate::contract_whale_monitor::{
        behavior_assessment::build_detection_behavior, impact_forecast::event_id,
    };
    signals.sort_by(|a, b| a.ts.cmp(&b.ts).then_with(|| a.id.cmp(&b.id)));
    let mut seen = std::collections::BTreeSet::new();
    signals
        .into_iter()
        .filter(|signal| {
            let id = event_id(signal);
            if seen.contains(&id) {
                return false;
            }
            let behavior = build_detection_behavior(signal, None, signal.ts);
            let hypothesis =
                serde_json::to_value(behavior.hypothesis).expect("serializable hypothesis");
            let direction =
                serde_json::to_value(behavior.direction_bias).expect("serializable direction");
            for frozen in [frozen_base.get(&id), frozen_hybrid.get(&id)]
                .into_iter()
                .flatten()
            {
                if frozen.event_ts != signal.ts
                    || hypothesis.as_str() != Some(frozen.behavior.as_str())
                    || direction.as_str() != Some(frozen.direction.as_str())
                {
                    return false;
                }
            }
            seen.insert(id)
        })
        .collect()
}

fn contract_calibration_due(
    event_id: &str,
    event_ts: i64,
    outcomes: &[crate::contract_whale_monitor::impact_forecast::ContractWhaleHorizonOutcome],
    now: i64,
) -> bool {
    use crate::contract_whale_monitor::impact_forecast::HORIZONS_SEC;
    HORIZONS_SEC.iter().any(|seconds| {
        let mature_at = event_ts.saturating_add(*seconds as i64 * 1000);
        if now < mature_at {
            return false;
        }
        [
            CONTRACT_WHALE_IMPACT_FORECAST_VERSION,
            CONTRACT_WHALE_IMPACT_FORECAST_V4_2_VERSION,
        ]
        .iter()
        .any(|version| {
            !outcomes.iter().any(|row| {
                row.event_id == event_id
                    && row.horizon_sec == *seconds
                    && row.outcome_version == *version
                    && (row.state == "complete"
                        || row.evaluated_at >= mature_at.saturating_add(600_000)
                        || now.saturating_sub(row.evaluated_at) < 60_000)
            })
        })
    })
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
        server_evidence_verified: signal.alert_eligible
            && signal.monitoring_started
            && signal.data_quality.is_some()
            && signal.read_only
            && !signal.runtime_modified
            && signal.analysis_only
            && !signal.execution_enabled,
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
        confidence: Some((signal.confidence * 100.0).clamp(0.0, 100.0)),
        data_quality: signal.data_quality,
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
        tof_score: signal.tof_score,
        candidate_type: Some(signal.candidate_type.clone()),
        explain_tags: Some(signal.explain_tags.clone()),
        direction_confidence: Some(signal.direction_confidence),
        perp_tof_metrics: Some(signal.perp_tof_metrics.clone()),
        perp_score: signal.perp_score,
        perp_candidate_type: Some(signal.perp_candidate_type.clone()),
        final_candidate_type: Some(signal.final_candidate_type.clone()),
        metrics_direction: serde_json::to_value(signal.metrics_direction)
            .ok()
            .and_then(|value| value.as_str().map(str::to_string)),
        advanced_tof_metrics: Some(signal.advanced_tof_metrics.clone()),
        advanced_score: signal.advanced_score,
        advanced_candidate_type: Some(signal.advanced_candidate_type.clone()),
        main_force_score: signal.main_force_score,
        extreme_impact_score: signal.extreme_impact_score,
        structure_bias: signal.structure_bias,
        market_structure_confidence: signal.market_structure_confidence,
        market_structure_data_quality: signal.market_structure_data_quality,
        market_structure_severity: signal.market_structure_severity.clone(),
        regime_type: signal.regime_type.clone(),
        spot_score: signal.spot_score,
        contract_score: signal.contract_score,
        cross_confirm_score: signal.cross_confirm_score,
        main_force_confirmed: signal.main_force_confirmed,
        signal_agreement: signal.signal_agreement,
        source_coverage: signal.source_coverage,
        oi_score: signal.oi_score,
        liquidation_score: signal.liquidation_score,
        test: None,
    };
    request.alert_family = Some(preferred_discord_alert_family(&request).to_string());
    if request.alert_family.as_deref() == Some("market_structure")
        && !signal.cwm_contribution.available
    {
        request.server_evidence_verified = false;
    }
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

    #[tokio::test]
    async fn production_evidence_is_event_aligned_and_persistable() {
        use crate::contract_whale_monitor::types::ContractWhaleSignal;
        use crate::spot_whale_monitor::types::{SpotExchange, SpotTrade, SpotTradeSide};
        let mut config = test_config(temp_sqlite_path("producer-evidence"));
        config.spot_whale_monitor.enabled = true;
        let state = AppState::new(config);
        let now = crate::normalizers::trade::now_ms();
        for second in 0..90 {
            state
                .flow_service_for_tests()
                .add_trade_for_tests(NormalizedTrade {
                    venue: Venue::Binance,
                    symbol: "BTC-PERP".into(),
                    ts: now - second * 1000,
                    price: 60000.0 + second as f64,
                    size_btc: 2.0,
                    size_usd: (60000.0 + second as f64) * 2.0,
                    aggressor_side: AggressorSide::Buy,
                    trade_id: Some(format!("evidence-{second}")),
                });
        }
        // Below the standalone whale threshold: ordinary spot trades still provide context.
        for second in 1..=3 {
            state.spot_whale_service().ingest_trade(SpotTrade {
                ts: now - second * 1000,
                exchange: SpotExchange::Binance,
                symbol: "BTC".into(),
                market: "spot".into(),
                price: 60000.0,
                qty_base: 0.01,
                notional_usd: 600.0,
                side: SpotTradeSide::Buy,
                trade_id: Some(format!("small-spot-{second}")),
            });
        }
        let buckets = state.contract_evidence_buckets("BTC", now);
        let micro = crate::contract_whale_monitor::aggregator::micro_volatility_from_buckets(
            &buckets,
            "BTC",
            now,
            &crate::contract_whale_monitor::config::contract_whale_runtime_config(),
        );
        assert!(micro.sample_count >= 60);
        assert_eq!(micro.source, "flow_1s_vwap");
        let mut signal: ContractWhaleSignal = serde_json::from_value(serde_json::json!({
            "id":"producer-test","ts":now,"symbol":"BTC","windowSec":15,"signalType":"aggressive_buy",
            "direction":"buy","severity":"high","score":80,"totalVolumeBtc":100,"netVolumeBtc":80,
            "totalNotionalUsd":6000000,"dominance":0.8,"mainExchange":"binance","exchanges":[],"dataQuality":90,
            "discordEligible":false,"discordSent":false,"discordReason":"test","finalResult":"test",
            "readOnly":true,"analysisOnly":true,"executionEnabled":false
        })).expect("fixture");
        crate::api::contract_whale_routes::enrich_production_evidence(
            &state,
            std::slice::from_mut(&mut signal),
        );
        assert_eq!(signal.spot_confirmation.status, "confirmed");
        assert_eq!(signal.spot_confirmation.latest_signal_at, Some(now - 1000));
        let store = state.contract_whale_store().expect("test database");
        store
            .upsert_contract_whale_signal(&signal)
            .expect("persist event evidence");
        let restored = store
            .list_contract_whale_signals("BTC", None, 10)
            .expect("reload event evidence")
            .into_iter()
            .find(|row| row.id == signal.id)
            .expect("persisted signal");
        assert_eq!(
            restored.spot_confirmation.confirmation_type,
            "confirms_contract_direction"
        );
        let mut cached = signal.clone();
        cached.id = "cached-pre-boot-test".into();
        cached.ts = state.booted_at_ms() - 1;
        store
            .enqueue_contract_whale_discord_outbox(std::slice::from_ref(&cached), now)
            .expect("queue cached fixture");
        state.process_contract_whale_discord_outbox_once().await;
        let reason: String = store
            .with_connection(|conn| {
                Ok(conn.query_row(
                    "SELECT last_error FROM contract_whale_discord_outbox WHERE signal_id = ?1",
                    [&cached.id],
                    |row| row.get(0),
                )?)
            })
            .expect("read skipped outbox");
        assert_eq!(reason, "cached_before_boot_display_only");
        signal.ts = now - 4000;
        crate::api::contract_whale_routes::enrich_production_evidence(
            &state,
            std::slice::from_mut(&mut signal),
        );
        assert_eq!(
            signal.spot_confirmation.status, "no_spot_sample",
            "future spot is not event evidence"
        );
    }

    #[test]
    fn calibration_uses_one_frozen_anchor_and_never_a_later_observation() {
        use crate::contract_whale_monitor::impact_forecast::{build_forecast, event_id};
        let mut anchor: crate::contract_whale_monitor::types::ContractWhaleSignal =
            serde_json::from_value(serde_json::json!({
                "id":"anchor","ts":600000,"symbol":"BTC","windowSec":60,"signalType":"aggressive_buy",
                "direction":"buy","severity":"high","score":80,"totalVolumeBtc":100,"netVolumeBtc":80,
                "totalNotionalUsd":6000000,"dominance":0.8,"mainExchange":"binance","exchanges":[],"dataQuality":90,
                "discordEligible":false,"discordSent":false,"discordReason":"test","finalResult":"test",
                "readOnly":true,"analysisOnly":true,"executionEnabled":false
            })).unwrap();
        anchor.event_lifecycle.event_id = "same-episode".into();
        let mut later = anchor.clone();
        later.id = "later".into();
        later.ts += 60000;
        let empty = Default::default();
        let selected = super::contract_calibration_anchors(
            vec![later.clone(), anchor.clone()],
            &empty,
            &empty,
        );
        assert_eq!(selected.len(), 1);
        assert_eq!(selected[0].id, anchor.id);
        let frozen = build_forecast(&selected[0], &[], &[], later.ts);
        let base = [(event_id(&anchor), frozen.clone())].into_iter().collect();
        let selected =
            super::contract_calibration_anchors(vec![later.clone(), anchor.clone()], &base, &empty);
        assert_eq!(selected.len(), 1);
        assert_eq!(selected[0].ts, frozen.event_ts);
        // Both outcome versions receive exactly this anchored signal from the production loop.
        let outcomes = crate::contract_whale_monitor::impact_forecast::evaluate_horizon_outcomes(
            &selected[0],
            crate::contract_whale_monitor::impact_forecast::ContractWhaleOutcomeInputs {
                flow_buckets: &[],
                reference_prices: &[],
                oi_snapshots: &[],
                funding_snapshots: &[],
                liquidation_buckets: &[],
            },
            anchor.ts + 86400000,
        );
        assert!(!outcomes.is_empty());
        assert!(outcomes.iter().all(|row| row.event_ts == frozen.event_ts
            && row.signal_id == anchor.id
            && row.direction == frozen.direction));
        assert!(super::contract_calibration_anchors(vec![later.clone()], &base, &empty).is_empty());
        let mismatched = [(event_id(&later), build_forecast(&later, &[], &[], later.ts))]
            .into_iter()
            .collect();
        assert!(
            super::contract_calibration_anchors(vec![anchor, later], &base, &mismatched).is_empty()
        );
    }

    #[test]
    fn sustained_producer_loads_oi_and_funding_at_closed_event_cutoff() {
        use crate::contract_whale_monitor::{
            config::ContractWhaleRuntimeConfig,
            types::{
                ContractExchange, ContractFlowBucket, ContractFundingSnapshot, ContractOiSnapshot,
            },
        };
        let state = AppState::new(test_config(temp_sqlite_path("sustained-causal-context")));
        let store = state.contract_whale_store().unwrap();
        let at = 10_845_000;
        let cutoff = 10_799_999;
        let rows = (0..10800)
            .map(|second| ContractFlowBucket {
                ts_bucket: second * 1000,
                symbol: "BTC".into(),
                exchange: "binance".into(),
                buy_volume_btc: if second >= 7200 { 0.02 } else { 0.0101 },
                sell_volume_btc: 0.01,
                buy_notional_usd: if second >= 7200 { 1200.0 } else { 606.0 },
                sell_notional_usd: 600.0,
                trade_count: 10,
                vwap: Some(60000.0),
                ..Default::default()
            })
            .collect::<Vec<_>>();
        store.upsert_contract_flow_buckets(&rows).unwrap();
        let oi = |ts, value| ContractOiSnapshot {
            ts,
            symbol: "BTC".into(),
            exchange: ContractExchange::Binance,
            oi_btc: value,
            oi_notional_usd: None,
            ct_val_available: true,
            evidence_degraded_reason: None,
        };
        let funding = |ts, rate| ContractFundingSnapshot {
            ts,
            symbol: "BTC".into(),
            exchange: ContractExchange::Binance,
            funding_rate: rate,
        };
        store
            .upsert_contract_oi_snapshots(&[oi(cutoff + 30000, 999999.0)])
            .unwrap();
        store
            .upsert_contract_funding_snapshots(&[funding(cutoff + 30000, 0.5)])
            .unwrap();
        let config = ContractWhaleRuntimeConfig::default();
        let missing =
            super::load_sustained_contract_candidates(&store, "BTC", at, 0, &config).unwrap();
        assert_eq!(missing.len(), 1);
        assert_eq!(missing[0].ts, cutoff);
        assert_eq!(missing[0].oi_change_pct, None);
        assert_eq!(missing[0].funding_rate, None);
        store
            .upsert_contract_oi_snapshots(&[
                oi(cutoff - 300000, 100000.0),
                oi(cutoff - 1000, 101000.0),
            ])
            .unwrap();
        store
            .upsert_contract_funding_snapshots(&[funding(cutoff - 1000, 0.001)])
            .unwrap();
        let causal =
            super::load_sustained_contract_candidates(&store, "BTC", at, 0, &config).unwrap();
        assert_eq!(causal.len(), 1);
        assert_eq!(causal[0].oi_change_pct, Some(1.0));
        assert_eq!(causal[0].funding_rate, Some(0.001));
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
