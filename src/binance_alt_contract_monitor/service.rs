use std::{
    collections::{BTreeMap, BTreeSet, VecDeque},
    fs::{self, OpenOptions},
    io::Write,
    path::PathBuf,
    sync::Arc,
    time::Duration,
};

use parking_lot::{Mutex, RwLock};
use tokio::{sync::mpsc, task::JoinHandle, time::MissedTickBehavior};

use crate::{
    normalizers::trade::now_ms,
    storage::{binance_alt_contract_repo::BinanceAltContractRepo, SqliteStore},
};

use super::{
    aggregator::{rolling_window_stats, trend_for_symbol},
    amios::run_market_intelligence_os,
    atca::run_trading_cognition_agent,
    collector,
    config::binance_alt_contract_runtime_config,
    context::empty_context,
    detector::{
        detect_alt_contract_signal_with_context, window_confirmation_for, MarketImpulseContext,
    },
    flow_state::PerSymbolFlowBook,
    impact::{impact_displayable, is_legacy_impact_score},
    scheduler::{AltCandidatePriority, FairScoringScheduler},
    smaf::{audit_smart_money_system, SmafAuditInput},
    smll::audit_self_learning_loop_with_outcomes,
    symbol_universe::{meta_from_product_id, tier_for_quote_volume},
    types::{
        AltContractAllMarketContextStatus, AltContractContext, AltContractDryRunStats,
        AltContractEventRecord, AltContractExchange, AltContractExchangeStatus,
        AltContractLatestResponse, AltContractOutcomeSummary, AltContractSeverity,
        AltContractSignal, AltContractSignalOutcome, AltContractSummary, AltContractSymbolMeta,
        AltContractSymbolTier, AltContractSymbolUniverseSummary, AltContractTrade,
        AltContractWindowStats, AltLiquidationEvent, AltLiquidationWindow, BacmRuntimeDiagnostics,
        LiquidationSide,
    },
    LOG_PREFIX, LOG_TARGET,
};

// Per-symbol trades are retained only for the short-lived detector windows.
// Cross-symbol ranking and breadth use PerSymbolFlowBook buckets instead.
const MAX_TRADES: usize = 20_000;
const MAX_SIGNALS: usize = 1_000;
const TRADE_RETENTION_MS: i64 = 3_600_000;
const DUPLICATE_WINDOW_MS: i64 = 10_000;
const OI_RETENTION_MS: i64 = 10 * 60_000;
const LIQUIDATION_CONTEXT_TTL_MS: i64 = 60_000;
const SUMMARY_MONITORED_SYMBOL_LIMIT: usize = 12;
const PERSISTENCE_MAX_RETRIES: u32 = 3;

#[derive(Clone)]
pub struct BinanceAltContractService {
    enabled: bool,
    dry_run: bool,
    booted_at_ms: i64,
    persistence_path: PathBuf,
    persistence_lock: Arc<Mutex<()>>,
    store: Option<SqliteStore>,
    persistence_tx: Arc<RwLock<Option<mpsc::Sender<PersistenceCommand>>>>,
    state: Arc<RwLock<BinanceAltContractState>>,
    tasks: Arc<RwLock<Vec<JoinHandle<()>>>>,
}

#[derive(Debug)]
struct BinanceAltContractState {
    trades_by_product: BTreeMap<String, VecDeque<AltContractTrade>>,
    flow_book: PerSymbolFlowBook,
    signals: VecDeque<AltContractSignal>,
    seen_signal_ids: BTreeSet<String>,
    exchanges: BTreeMap<String, AltContractExchangeStatus>,
    contexts: BTreeMap<String, AltContractContext>,
    symbol_metas: BTreeMap<String, AltContractSymbolMeta>,
    active_symbol_last_trade_at: BTreeMap<String, i64>,
    oi_snapshots: BTreeMap<String, VecDeque<(i64, f64)>>,
    liquidation_events: BTreeMap<String, VecDeque<AltLiquidationEvent>>,
    liquidation_event_seen_at: BTreeMap<String, i64>,
    liquidation_seen_at: BTreeMap<String, i64>,
    candidate_seen_at: BTreeMap<String, i64>,
    hot_oi_seen_at: BTreeMap<String, i64>,
    last_detector_scan_at: BTreeMap<String, i64>,
    scheduler: FairScoringScheduler,
    light_candidates_total: u64,
    full_score_attempts_total: u64,
    full_score_skipped_budget_total: u64,
    persistence_queue_depth: usize,
    oldest_persistence_enqueued_at: Option<i64>,
    persistence_dropped_total: u64,
    shard_connected: BTreeMap<usize, bool>,
    symbol_shards: BTreeMap<String, usize>,
    total_shards: usize,
    universe_last_refreshed_at: Option<i64>,
    events: BTreeMap<String, AltContractEventState>,
    outcomes: BTreeMap<String, AltContractSignalOutcome>,
    last_oi_poll_at: Option<i64>,
    last_force_order_at: Option<i64>,
    last_mark_price_at: Option<i64>,
    last_ticker_at: Option<i64>,
    mark_price_stream_connected: bool,
    ticker_stream_connected: bool,
    force_order_stream_connected: bool,
    error_events: VecDeque<i64>,
}

#[derive(Debug, Clone)]
enum PersistenceCommand {
    InsertSignal(AltContractSignal),
    UpdateOutcome(AltContractSignalOutcome),
    UpsertEvent(AltContractEventRecord),
}

#[derive(Debug, Clone)]
struct AltContractEventState {
    id: String,
    start_ts: i64,
    updated_at: i64,
    tier: AltContractSymbolTier,
    signal_type: String,
    direction: String,
    liquidation_driven: bool,
    status: String,
    close_reason: Option<String>,
    latest_signal_id: String,
    peak_signal_id: String,
    peak_abnormal_score: u8,
    peak_build_score: u8,
    signal_count: u32,
}

#[derive(Debug, Clone, Default)]
pub struct BinanceAltContractQuery {
    pub symbol: Option<String>,
    pub severity: Option<String>,
    pub signal_type: Option<String>,
    pub direction: Option<String>,
    pub would_send: Option<bool>,
    pub liquidation: Option<bool>,
    pub tier: Option<String>,
    pub min_build_score: Option<u8>,
    pub cursor_ts: Option<i64>,
    pub limit: Option<usize>,
}

#[derive(Debug, Clone, Default)]
pub struct AltContractOutcomeFilter {
    pub symbol: Option<String>,
    pub tier: Option<String>,
    pub window_sec: Option<u64>,
    pub signal_type: Option<String>,
    pub severity: Option<String>,
    pub ais_min: Option<f64>,
    pub ais_max: Option<f64>,
    pub regime: Option<String>,
    pub oi_context: Option<String>,
    pub time_of_day_utc: Option<u8>,
}

impl BinanceAltContractService {
    pub fn new(enabled: bool, dry_run: bool, booted_at_ms: i64) -> Self {
        Self::with_store(enabled, dry_run, booted_at_ms, None)
    }

    pub fn with_store(
        enabled: bool,
        dry_run: bool,
        booted_at_ms: i64,
        store: Option<SqliteStore>,
    ) -> Self {
        let runtime_config = binance_alt_contract_runtime_config();
        let mut exchanges = BTreeMap::new();
        exchanges.insert(
            "binance".to_string(),
            if enabled && runtime_config.exchange.binance_enabled {
                AltContractExchangeStatus::disconnected()
            } else {
                AltContractExchangeStatus::disabled()
            },
        );
        let restored = if enabled {
            store.as_ref().map_or_else(
                || {
                    load_persisted_signals(
                        &runtime_config.persistence_path,
                        MAX_SIGNALS,
                        now_ms(),
                        retention_days_to_ms(runtime_config.storage.signals_retention_days),
                    )
                },
                |store| load_persisted_sqlite_signals(store, MAX_SIGNALS),
            )
        } else {
            RestoredAltContractSignals::default()
        };
        let restored_outcomes = store
            .as_ref()
            .and_then(|store| store.load_alt_contract_outcomes(MAX_SIGNALS).ok())
            .unwrap_or_default()
            .into_iter()
            .map(|outcome| (outcome.signal_id.clone(), outcome))
            .collect();
        let restored_events = store
            .as_ref()
            .and_then(|store| store.load_alt_contract_events(MAX_SIGNALS).ok())
            .unwrap_or_default()
            .into_iter()
            .map(|event| {
                let key = format!(
                    "{}:{}:{}",
                    event.product_id, event.direction, event.signal_type
                );
                (key, event_state_from_record(event))
            })
            .collect();
        Self {
            enabled,
            dry_run,
            booted_at_ms,
            persistence_path: runtime_config.persistence_path,
            persistence_lock: Arc::new(Mutex::new(())),
            store,
            persistence_tx: Arc::new(RwLock::new(None)),
            state: Arc::new(RwLock::new(BinanceAltContractState {
                trades_by_product: BTreeMap::new(),
                flow_book: PerSymbolFlowBook::new(
                    runtime_config.flow_state.bucket_retention_seconds,
                ),
                signals: restored.signals,
                seen_signal_ids: restored.seen_signal_ids,
                exchanges,
                contexts: BTreeMap::new(),
                symbol_metas: BTreeMap::new(),
                active_symbol_last_trade_at: BTreeMap::new(),
                oi_snapshots: BTreeMap::new(),
                liquidation_events: BTreeMap::new(),
                liquidation_event_seen_at: BTreeMap::new(),
                liquidation_seen_at: BTreeMap::new(),
                candidate_seen_at: BTreeMap::new(),
                hot_oi_seen_at: BTreeMap::new(),
                last_detector_scan_at: BTreeMap::new(),
                scheduler: FairScoringScheduler::default(),
                light_candidates_total: 0,
                full_score_attempts_total: 0,
                full_score_skipped_budget_total: 0,
                persistence_queue_depth: 0,
                oldest_persistence_enqueued_at: None,
                persistence_dropped_total: 0,
                shard_connected: BTreeMap::new(),
                symbol_shards: BTreeMap::new(),
                total_shards: 0,
                universe_last_refreshed_at: None,
                events: restored_events,
                outcomes: restored_outcomes,
                last_oi_poll_at: None,
                last_force_order_at: None,
                last_mark_price_at: None,
                last_ticker_at: None,
                mark_price_stream_connected: false,
                ticker_stream_connected: false,
                force_order_stream_connected: false,
                error_events: VecDeque::new(),
            })),
            tasks: Arc::new(RwLock::new(Vec::new())),
        }
    }

    fn runtime_enabled(&self, config: &super::config::BinanceAltContractRuntimeConfig) -> bool {
        self.enabled || config.enabled
    }

    pub fn start(&self) {
        let config = binance_alt_contract_runtime_config();
        let enabled = self.runtime_enabled(&config);
        if !enabled || self.tasks.read().iter().any(|task| !task.is_finished()) {
            return;
        }
        tracing::info!(
            target: LOG_TARGET,
            enabled,
            dry_run = self.dry_run,
            "{} runtime started",
            LOG_PREFIX
        );
        self.start_persistence_worker();
        if config.exchange.binance_enabled {
            let service = self.clone();
            self.tasks.write().push(tokio::spawn(async move {
                collector::run(service).await;
            }));
            let service = self.clone();
            self.tasks.write().push(tokio::spawn(async move {
                collector::run_context_polling(service).await;
            }));
            let service = self.clone();
            self.tasks.write().push(tokio::spawn(async move {
                collector::run_all_market_context_stream(service).await;
            }));
            let service = self.clone();
            self.tasks.write().push(tokio::spawn(async move {
                collector::run_force_order_stream(service).await;
            }));
        }
        let service = self.clone();
        self.tasks.write().push(tokio::spawn(async move {
            service.run_cache_cleanup_loop().await;
        }));
    }

    pub fn stop(&self) {
        self.persistence_tx.write().take();
        let tasks = std::mem::take(&mut *self.tasks.write());
        for task in tasks {
            task.abort();
        }
        self.set_exchange_status(AltContractExchange::Binance, "disconnected", false, None);
    }

    pub fn ingest_live_trade(&self, trade: AltContractTrade) {
        let _ = self.ingest_trade(trade);
    }

    pub fn ingest_trade(&self, trade: AltContractTrade) -> Vec<AltContractSignal> {
        let config = binance_alt_contract_runtime_config();
        if !self.runtime_enabled(&config) || !self.product_enabled(&trade.product_id, &config) {
            return Vec::new();
        }
        self.mark_trade(trade.exchange, trade.ts);
        {
            let mut state = self.state.write();
            let symbol_trades = state
                .trades_by_product
                .entry(trade.product_id.clone())
                .or_default();
            symbol_trades.push_back(trade.clone());
            prune_trades(symbol_trades, trade.ts);
            state.flow_book.ingest(trade.clone());
            state
                .active_symbol_last_trade_at
                .insert(trade.product_id.clone(), trade.ts);
        }
        self.update_post_signal_validation(&trade);
        let meta = self.meta_for_product(&trade.product_id);
        let context = self.context_for_product(&trade.product_id);
        let force_scan = context.force_order_snapshot
            || trade.notional_usd
                >= config
                    .thresholds_for_tier(meta.tier)
                    .high_notional_usd
                    .max(1.0)
                    * 0.25;
        if !self.should_run_detector(&trade.product_id, trade.ts, &config, force_scan) {
            if force_scan {
                self.mark_candidate_product(&trade.product_id, trade.ts);
            }
            return Vec::new();
        }
        let (_, window_stats) = self.window_stats_for_product(&trade.product_id, trade.ts, &config);
        let Some(priority) = candidate_priority(&window_stats, &context, &config, trade.ts) else {
            return Vec::new();
        };
        self.mark_candidate_product(&trade.product_id, trade.ts);
        let selected = {
            let mut state = self.state.write();
            let selected = if config.scheduler.enabled {
                state.scheduler.upsert(priority);
                state.scheduler.select(trade.ts, &config.scheduler)
            } else {
                vec![priority]
            };
            state.full_score_attempts_total = state
                .full_score_attempts_total
                .saturating_add(u64::try_from(selected.len()).unwrap_or(u64::MAX));
            if selected.is_empty() {
                state.full_score_skipped_budget_total =
                    state.full_score_skipped_budget_total.saturating_add(1);
            }
            selected
        };
        selected
            .into_iter()
            .flat_map(|candidate| self.score_candidate(&candidate.product_id, trade.ts, &config))
            .collect()
    }

    pub fn insert_signal_for_tests(&self, signal: AltContractSignal) -> bool {
        self.insert_signal(signal)
    }

    pub fn prune_expired_cache_for_tests(&self, now: i64) {
        self.prune_expired_cache(now);
    }

    pub fn update_symbol_universe(&self, metas: Vec<AltContractSymbolMeta>) {
        let mut state = self.state.write();
        state.symbol_metas = metas
            .into_iter()
            .map(|meta| (meta.product_id.clone(), meta))
            .collect();
        state.universe_last_refreshed_at = Some(now_ms());
    }

    pub fn update_shard_status(&self, shard_id: usize, total_shards: usize, connected: bool) {
        let mut state = self.state.write();
        state.total_shards = total_shards;
        state.shard_connected.insert(shard_id, connected);
    }

    pub fn begin_shard_supervision(&self, total_shards: usize) {
        let mut state = self.state.write();
        state.total_shards = total_shards;
        state.shard_connected.clear();
        state.symbol_shards.clear();
    }

    pub fn update_shard_symbols(&self, shard_id: usize, symbols: &[String]) {
        let mut state = self.state.write();
        state
            .symbol_shards
            .retain(|_, assigned_shard| *assigned_shard != shard_id);
        for symbol in symbols {
            state
                .symbol_shards
                .insert(product_id_for_symbol(symbol), shard_id);
        }
    }

    pub fn runtime_diagnostics(&self) -> BacmRuntimeDiagnostics {
        let config = binance_alt_contract_runtime_config();
        let now = now_ms();
        let state = self.state.read();
        let universe_symbol_count = if state.symbol_metas.is_empty() {
            config.enabled_symbols().len()
        } else {
            state.symbol_metas.len()
        };
        let universe_refresh_age_sec = state.universe_last_refreshed_at.map(|refreshed_at| {
            u64::try_from(now.saturating_sub(refreshed_at).max(0) / 1_000).unwrap_or(u64::MAX)
        });
        let scheduler = state.scheduler.diagnostics(now);
        let universe_symbols = if state.symbol_metas.is_empty() {
            config.enabled_symbols()
        } else {
            state.symbol_metas.keys().cloned().collect::<Vec<_>>()
        };
        let missing_symbols = universe_symbols
            .iter()
            .filter(|product_id| {
                state
                    .symbol_shards
                    .get(*product_id)
                    .and_then(|shard_id| state.shard_connected.get(shard_id))
                    .copied()
                    != Some(true)
            })
            .cloned()
            .collect::<Vec<_>>();
        let symbol_coverage_ratio = if universe_symbols.is_empty() {
            1.0
        } else {
            (universe_symbols.len().saturating_sub(missing_symbols.len())) as f64
                / universe_symbols.len() as f64
        };
        BacmRuntimeDiagnostics {
            universe_symbol_count,
            active_symbol_count: state.active_symbol_last_trade_at.len(),
            connected_shards: state
                .shard_connected
                .values()
                .filter(|connected| **connected)
                .count(),
            total_shards: state.total_shards,
            symbol_coverage_ratio,
            missing_symbols,
            trade_buffer_total: state.trades_by_product.values().map(VecDeque::len).sum(),
            per_symbol_state_count: state.flow_book.symbol_count(),
            light_candidates_total: state.light_candidates_total,
            full_score_attempts_total: state.full_score_attempts_total,
            full_score_skipped_budget_total: state.full_score_skipped_budget_total,
            persistence_queue_depth: state.persistence_queue_depth,
            oldest_persistence_age_ms: state.oldest_persistence_enqueued_at.map(|enqueued_at| {
                u64::try_from(now.saturating_sub(enqueued_at).max(0)).unwrap_or(u64::MAX)
            }),
            persistence_dropped_total: state.persistence_dropped_total,
            universe_last_refreshed_at: state.universe_last_refreshed_at,
            universe_refresh_age_sec,
            scheduler_scored_by_tier: scheduler.scored_by_tier,
            scheduler_skipped_by_tier: scheduler.skipped_by_tier,
            scheduler_oldest_candidate_age_ms: scheduler.oldest_candidate_age_ms,
            scheduler_per_symbol_score_count: scheduler.per_symbol_score_count,
            scheduler_starved_candidate_count: scheduler.starved_candidate_count,
        }
    }

    pub fn monitored_product_ids(&self) -> Vec<String> {
        let config = binance_alt_contract_runtime_config();
        let state = self.state.read();
        let mut items = if state.symbol_metas.is_empty() {
            config.enabled_symbols()
        } else {
            state.symbol_metas.keys().cloned().collect::<Vec<_>>()
        };
        items.sort();
        items
    }

    pub fn product_enabled(
        &self,
        product_id: &str,
        config: &super::config::BinanceAltContractRuntimeConfig,
    ) -> bool {
        let product_id = product_id_for_symbol(product_id);
        let state = self.state.read();
        if state.symbol_metas.is_empty() {
            return config.symbol_enabled(&product_id);
        }
        state.symbol_metas.contains_key(&product_id)
    }

    pub fn update_open_interest(&self, product_id: &str, ts: i64, open_interest_base: f64) {
        if !open_interest_base.is_finite() || open_interest_base <= 0.0 {
            return;
        }
        let config = binance_alt_contract_runtime_config();
        let product_id = product_id_for_symbol(product_id);
        let mut state = self.state.write();
        let snapshots = state.oi_snapshots.entry(product_id.clone()).or_default();
        snapshots.push_back((ts, open_interest_base));
        while snapshots
            .front()
            .is_some_and(|(seen_at, _)| ts.saturating_sub(*seen_at) > OI_RETENTION_MS)
        {
            snapshots.pop_front();
        }
        let change_1m = oi_period_delta(
            snapshots,
            ts,
            60,
            config.oi.min_1m_history_seconds,
            config.oi.max_snapshot_gap_seconds,
        );
        let change_5m = oi_period_delta(
            snapshots,
            ts,
            300,
            config.oi.min_5m_history_seconds,
            config.oi.max_snapshot_gap_seconds,
        );
        let context = state.contexts.entry(product_id.clone()).or_default();
        context.oi_change_1m_base = change_1m.delta;
        context.oi_change_5m_base = change_5m.delta;
        context.oi_change_pct = change_1m.delta_pct;
        context.oi_change_1m = change_1m;
        context.oi_change_5m = change_5m;
        context.oi_updated_at = Some(ts);
        state.last_oi_poll_at = Some(ts);
    }

    pub fn context_snapshot(&self, product_id: &str) -> AltContractContext {
        self.context_for_product(product_id)
    }

    pub fn update_funding_context(&self, product_id: &str, funding_rate: Option<f64>) {
        let Some(funding_rate) = funding_rate.filter(|value| value.is_finite()) else {
            return;
        };
        let product_id = product_id_for_symbol(product_id);
        let mut state = self.state.write();
        let context = state.contexts.entry(product_id.clone()).or_default();
        context.funding_rate = Some(funding_rate);
        context.funding_bias = Some(if funding_rate > 0.0001 {
            "long".to_string()
        } else if funding_rate < -0.0001 {
            "short".to_string()
        } else {
            "neutral".to_string()
        });
    }

    pub fn update_mark_price_context(
        &self,
        product_id: &str,
        ts: i64,
        mark_price_usd: Option<f64>,
        funding_rate: Option<f64>,
    ) {
        let product_id = product_id_for_symbol(product_id);
        let mark_price = mark_price_usd.filter(|value| value.is_finite() && *value > 0.0);
        let mut state = self.state.write();
        let context = state.contexts.entry(product_id.clone()).or_default();
        if let Some(mark_price_usd) = mark_price {
            context.mark_price_usd = Some(mark_price_usd);
            context.mark_price_updated_at = Some(ts);
        }
        if let Some(funding_rate) = funding_rate.filter(|value| value.is_finite()) {
            context.funding_rate = Some(funding_rate);
            context.funding_bias = Some(if funding_rate > 0.0001 {
                "long".to_string()
            } else if funding_rate < -0.0001 {
                "short".to_string()
            } else {
                "neutral".to_string()
            });
        }
        state.last_mark_price_at = Some(ts);
        state.mark_price_stream_connected = true;
        drop(state);
        if let Some(mark_price) = mark_price {
            self.update_outcomes_with_price(&product_id, mark_price, ts);
        }
    }

    pub fn update_ticker_context(
        &self,
        product_id: &str,
        ts: i64,
        last_price_usd: Option<f64>,
        quote_volume_24h_usd: Option<f64>,
        price_change_24h_pct: Option<f64>,
    ) {
        let product_id = product_id_for_symbol(product_id);
        let mut state = self.state.write();
        let last_price_usd = last_price_usd.filter(|value| value.is_finite() && *value > 0.0);
        let quote_volume_24h_usd =
            quote_volume_24h_usd.filter(|value| value.is_finite() && *value >= 0.0);
        let price_change_24h_pct = price_change_24h_pct.filter(|value| value.is_finite());
        {
            let context = state.contexts.entry(product_id.clone()).or_default();
            if let Some(last_price_usd) = last_price_usd {
                context.last_price_usd = Some(last_price_usd);
            }
            if let Some(quote_volume_24h_usd) = quote_volume_24h_usd {
                context.ticker_quote_volume_24h_usd = Some(quote_volume_24h_usd);
            }
            if let Some(price_change_24h_pct) = price_change_24h_pct {
                context.ticker_price_change_24h_pct = Some(price_change_24h_pct);
            }
            context.ticker_updated_at = Some(ts);
        }
        if let Some(quote_volume_24h_usd) = quote_volume_24h_usd {
            if let Some(meta) = state.symbol_metas.get_mut(&product_id) {
                meta.quote_volume_24h_usd = quote_volume_24h_usd;
                meta.tier = tier_for_quote_volume(quote_volume_24h_usd);
            }
        }
        state.last_ticker_at = Some(ts);
        state.ticker_stream_connected = true;
    }

    pub fn update_liquidation_context(&self, product_id: &str, ts: i64, notional_usd: f64) {
        self.update_liquidation_event(AltLiquidationEvent {
            product_id: product_id.to_string(),
            ts,
            side: LiquidationSide::Unknown,
            notional_usd,
            price: None,
            quantity: None,
            source_event_id: None,
        });
    }

    pub fn update_liquidation_event(&self, mut event: AltLiquidationEvent) {
        if !event.notional_usd.is_finite() || event.notional_usd <= 0.0 {
            return;
        }
        event.product_id = product_id_for_symbol(&event.product_id);
        let config = binance_alt_contract_runtime_config();
        let retention_ms = i64::try_from(config.liquidation.retention_seconds)
            .unwrap_or(i64::MAX)
            .saturating_mul(1_000);
        let event_key = liquidation_event_key(&event);
        let mut state = self.state.write();
        if config.liquidation.deduplicate
            && state.liquidation_event_seen_at.contains_key(&event_key)
        {
            return;
        }
        let summary = {
            let events = state
                .liquidation_events
                .entry(event.product_id.clone())
                .or_default();
            events.push_back(event.clone());
            prune_liquidation_events(events, event.ts, retention_ms);
            liquidation_window(events, 60, event.ts)
        };
        state.liquidation_event_seen_at.insert(event_key, event.ts);
        prune_timestamp_map(&mut state.liquidation_event_seen_at, event.ts, retention_ms);
        let context = state.contexts.entry(event.product_id.clone()).or_default();
        context.liquidation_notional_usd = Some(summary.liquidation_total_usd);
        context.liquidation_count = summary.liquidation_count;
        context.dominant_liquidation_side = summary.dominant_liquidation_side;
        context.liquidation_suspected = true;
        context.force_order_snapshot = true;
        state.liquidation_seen_at.insert(event.product_id, event.ts);
        state.last_force_order_at = Some(event.ts);
        state.force_order_stream_connected = true;
    }

    pub fn liquidation_window_snapshot(
        &self,
        product_id: &str,
        window_sec: u64,
        now: i64,
    ) -> AltLiquidationWindow {
        let product_id = product_id_for_symbol(product_id);
        let state = self.state.read();
        state
            .liquidation_events
            .get(&product_id)
            .map(|events| liquidation_window(events, window_sec, now))
            .unwrap_or_else(|| AltLiquidationWindow {
                window_sec,
                ..Default::default()
            })
    }

    pub fn mark_all_market_context_connected(&self) {
        let mut state = self.state.write();
        state.mark_price_stream_connected = true;
        state.ticker_stream_connected = true;
    }

    pub fn mark_all_market_context_disconnected(&self, error: Option<String>) {
        let mut state = self.state.write();
        let has_error = error.is_some();
        state.mark_price_stream_connected = false;
        state.ticker_stream_connected = false;
        if has_error {
            push_error_event(&mut state, now_ms());
        }
    }

    pub fn mark_force_order_stream_connected(&self) {
        self.state.write().force_order_stream_connected = true;
    }

    pub fn mark_force_order_stream_disconnected(&self, error: Option<String>) {
        let mut state = self.state.write();
        let has_error = error.is_some();
        state.force_order_stream_connected = false;
        if has_error {
            push_error_event(&mut state, now_ms());
        }
    }

    pub fn hot_oi_product_ids(&self) -> Vec<String> {
        let config = binance_alt_contract_runtime_config();
        let now = now_ms();
        let ttl_ms = i64::try_from(config.oi_scheduler.candidate_ttl_sec)
            .unwrap_or(600)
            .saturating_mul(1000);
        let mut state = self.state.write();
        prune_seen_map(&mut state.candidate_seen_at, now, ttl_ms);
        prune_seen_map(&mut state.hot_oi_seen_at, now, ttl_ms);
        let mut items = state.hot_oi_seen_at.keys().cloned().collect::<Vec<_>>();
        items.sort();
        items
    }

    pub fn record_error(&self) {
        let now = now_ms();
        let mut state = self.state.write();
        push_error_event(&mut state, now);
    }

    pub fn mark_connected(&self, exchange: AltContractExchange) {
        self.set_exchange_status(exchange, "connected", true, None);
    }

    pub fn mark_reconnecting(&self, exchange: AltContractExchange, error: Option<String>) {
        let mut state = self.state.write();
        let has_error = error.is_some();
        {
            let entry = state
                .exchanges
                .entry(exchange.as_key().to_string())
                .or_insert_with(AltContractExchangeStatus::disconnected);
            entry.connected = false;
            entry.status = "reconnecting".to_string();
            entry.reconnect_count = entry.reconnect_count.saturating_add(1);
            entry.last_error = error.map(redact_error);
        }
        if has_error {
            push_error_event(&mut state, now_ms());
        }
    }

    pub fn set_exchange_status(
        &self,
        exchange: AltContractExchange,
        status: &str,
        connected: bool,
        error: Option<String>,
    ) {
        let mut state = self.state.write();
        let has_error = error.is_some();
        {
            let entry = state
                .exchanges
                .entry(exchange.as_key().to_string())
                .or_insert_with(AltContractExchangeStatus::disconnected);
            entry.status = status.to_string();
            entry.connected = connected;
            entry.last_error = error.map(redact_error);
        }
        if has_error {
            push_error_event(&mut state, now_ms());
        }
    }

    pub fn summary(&self, symbol: Option<&str>) -> AltContractSummary {
        let config = binance_alt_contract_runtime_config();
        let enabled = self.runtime_enabled(&config);
        let now = now_ms();
        let state = self.state.read();
        let empty_signals = VecDeque::new();
        let signals = if enabled {
            &state.signals
        } else {
            &empty_signals
        };
        let product_filter = symbol.map(product_id_for_symbol);
        let latest = signals.iter().rev().find(|signal| {
            product_filter
                .as_ref()
                .map(|item| &signal.product_id == item)
                .unwrap_or(true)
        });
        let all_monitored_symbols = if !enabled {
            Vec::new()
        } else if state.symbol_metas.is_empty() {
            config.enabled_symbols()
        } else {
            state.symbol_metas.keys().cloned().collect::<Vec<_>>()
        };
        let trend_product = product_filter
            .clone()
            .or_else(|| latest.map(|signal| signal.product_id.clone()))
            .or_else(|| all_monitored_symbols.first().cloned())
            .unwrap_or_else(|| "SOLUSDT".to_string());
        let exchanges = summarized_exchange_statuses(
            enabled,
            &state.exchanges,
            now,
            config.data_quality.heartbeat_stale_ms,
        );
        let errors1h = state
            .error_events
            .iter()
            .filter(|seen_at| now.saturating_sub(**seen_at) <= 60 * 60_000)
            .count();
        let active_anomaly_count = signals
            .iter()
            .filter(|signal| now.saturating_sub(signal.ts) <= 15 * 60_000)
            .count();
        let recent_critical_or_s_count = signals
            .iter()
            .filter(|signal| now.saturating_sub(signal.ts) <= 60 * 60_000)
            .filter(|signal| signal.severity.rank() >= AltContractSeverity::Critical.rank())
            .count();
        let dry_run_would_send_count = signals
            .iter()
            .filter(|signal| now.saturating_sub(signal.ts) <= 60 * 60_000)
            .filter(|signal| signal.discord_would_send)
            .count();
        let health_status = health_status(
            enabled,
            &exchanges,
            state.total_shards,
            state
                .shard_connected
                .values()
                .filter(|connected| **connected)
                .count(),
        );
        let dry_run_stats = dry_run_stats(signals, now);
        let last_trade_at = exchanges
            .values()
            .filter_map(|status| status.last_trade_at)
            .max();
        let top_active_symbols = if enabled {
            top_active_symbols(&state.flow_book, now)
        } else {
            Vec::new()
        };
        let monitored_symbols = summary_monitored_symbols(
            &all_monitored_symbols,
            &top_active_symbols,
            product_filter.as_deref(),
            latest,
        );
        let candidate_ttl_ms = i64::try_from(config.oi_scheduler.candidate_ttl_sec)
            .unwrap_or(600)
            .saturating_mul(1000);
        let all_market_context = AltContractAllMarketContextStatus {
            mark_price_connected: state.mark_price_stream_connected,
            ticker_connected: state.ticker_stream_connected,
            force_order_connected: state.force_order_stream_connected,
            last_mark_price_at: state.last_mark_price_at,
            last_ticker_at: state.last_ticker_at,
            last_force_order_at: state.last_force_order_at,
            candidate_symbols: recent_seen_keys(&state.candidate_seen_at, now, candidate_ttl_ms),
            hot_oi_symbols: recent_seen_keys(&state.hot_oi_seen_at, now, candidate_ttl_ms),
        };
        let smaf_report = audit_smart_money_system(SmafAuditInput {
            enabled,
            now_ms: now,
            exchanges: &exchanges,
            signals,
            last_oi_poll_at: state.last_oi_poll_at,
            last_force_order_at: state.last_force_order_at,
            last_mark_price_at: state.last_mark_price_at,
            last_ticker_at: state.last_ticker_at,
            errors1h,
        });
        let mut smll_report = audit_self_learning_loop_with_outcomes(
            &config.self_learning.mode,
            now,
            signals,
            &state.outcomes,
        );
        smll_report.min_samples_for_update = config.self_learning.min_samples_for_update;
        if smll_report.learning_mode == "real_outcome"
            && smll_report.sample_size < smll_report.min_samples_for_update
        {
            smll_report.accuracy_available = false;
            smll_report.accuracy_rate = 0.0;
            smll_report.reason = "insufficient_samples_for_reporting".to_string();
            smll_report.status = "collecting_outcomes".to_string();
            smll_report.calibration_updates.clear();
        }
        let atca_report =
            run_trading_cognition_agent(now, &state.signals, &smaf_report, &smll_report);
        let amios_report =
            run_market_intelligence_os(now, signals, &smaf_report, &smll_report, &atca_report);
        AltContractSummary {
            status: latest
                .map(|signal| status_from_severity(signal.severity).to_string())
                .unwrap_or_else(|| "calm".to_string()),
            health_status: health_status.clone(),
            health_reason: health_reason(enabled, &health_status).to_string(),
            collector_status: collector_status(enabled, exchanges.get("binance")),
            last_trade_at,
            last_oi_poll_at: state.last_oi_poll_at,
            last_force_order_at: state.last_force_order_at,
            flow_buckets1m: state
                .trades_by_product
                .values()
                .flat_map(|trades| trades.iter())
                .filter(|trade| now.saturating_sub(trade.ts) <= 60_000)
                .count(),
            signals1h: dry_run_stats.signals1h,
            would_send1h: dry_run_stats.would_send1h,
            top_active_symbols,
            errors1h,
            latest_direction: latest
                .map(|signal| format!("{:?}", signal.direction).to_ascii_lowercase())
                .unwrap_or_else(|| "neutral".to_string()),
            latest_severity: latest
                .map(|signal| signal.severity)
                .unwrap_or(AltContractSeverity::Calm),
            latest_signal_at: latest.map(|signal| signal.ts),
            signal_count: signals
                .iter()
                .filter(|signal| {
                    product_filter
                        .as_ref()
                        .map(|item| &signal.product_id == item)
                        .unwrap_or(true)
                })
                .count(),
            monitored_symbols,
            display_min_notional_usd: config.display.min_notional_usd,
            display_thresholds_usd: config.display.thresholds_summary(),
            active_anomaly_count,
            recent_critical_or_s_count,
            dry_run_would_send_count,
            enabled,
            dry_run: self.dry_run,
            read_only: true,
            symbol: symbol.map(|value| value.to_ascii_uppercase()),
            trend60s: state
                .trades_by_product
                .get(&trend_product)
                .map(|trades| trend_for_symbol(trades, &trend_product, now))
                .unwrap_or_default(),
            exchanges,
            dry_run_stats,
            symbol_universe: symbol_universe_summary(&config, &state.symbol_metas, enabled),
            all_market_context,
            smaf_report,
            smll_report,
            atca_report,
            amios_report,
        }
    }

    pub fn latest(&self, symbol: Option<&str>, limit: usize) -> AltContractLatestResponse {
        let config = binance_alt_contract_runtime_config();
        if !self.runtime_enabled(&config) {
            return AltContractLatestResponse {
                summary: self.summary(symbol),
                items: Vec::new(),
                limit: limit.clamp(1, 200),
            };
        }
        let limit = limit.clamp(1, 200);
        let product_filter = symbol.map(product_id_for_symbol);
        let mut items = self
            .state
            .read()
            .signals
            .iter()
            .filter(|signal| {
                product_filter
                    .as_ref()
                    .map(|item| &signal.product_id == item)
                    .unwrap_or(true)
            })
            .filter(|signal| display_signal(signal, &config))
            .cloned()
            .collect::<Vec<_>>();
        sort_latest_signals(&mut items);
        items.truncate(limit);
        AltContractLatestResponse {
            summary: self.summary(symbol),
            items,
            limit,
        }
    }

    pub fn history(&self, query: BinanceAltContractQuery) -> AltContractLatestResponse {
        let config = binance_alt_contract_runtime_config();
        if !self.runtime_enabled(&config) {
            return AltContractLatestResponse {
                summary: self.summary(query.symbol.as_deref()),
                items: Vec::new(),
                limit: query.limit.unwrap_or(50).clamp(1, 200),
            };
        }
        let limit = query.limit.unwrap_or(50).clamp(1, 200);
        let product_filter = query.symbol.as_deref().map(product_id_for_symbol);
        let severity_filter = query.severity.as_deref().map(compact_filter_value);
        let type_filter = query.signal_type.as_deref().map(compact_filter_value);
        let direction_filter = query.direction.as_deref().map(compact_filter_value);
        let tier_filter = query.tier.as_deref().map(compact_filter_value);
        let mut items = self
            .state
            .read()
            .signals
            .iter()
            .filter(|signal| {
                product_filter
                    .as_ref()
                    .map(|item| &signal.product_id == item)
                    .unwrap_or(true)
            })
            .filter(|signal| display_signal(signal, &config))
            .filter(|signal| {
                severity_filter
                    .as_ref()
                    .map(|value| compact_filter_value(&format!("{:?}", signal.severity)) == *value)
                    .unwrap_or(true)
            })
            .filter(|signal| {
                type_filter
                    .as_ref()
                    .map(|value| {
                        compact_filter_value(&format!("{:?}", signal.signal_type)) == *value
                    })
                    .unwrap_or(true)
            })
            .cloned()
            .collect::<Vec<_>>();
        items.retain(|signal| {
            direction_filter
                .as_ref()
                .map(|value| compact_filter_value(&format!("{:?}", signal.direction)) == *value)
                .unwrap_or(true)
        });
        items.retain(|signal| {
            query
                .would_send
                .map(|value| signal.discord_would_send == value)
                .unwrap_or(true)
        });
        items.retain(|signal| {
            query
                .liquidation
                .map(|value| signal.liquidation_suspected == value)
                .unwrap_or(true)
        });
        items.retain(|signal| {
            tier_filter
                .as_ref()
                .map(|value| compact_filter_value(&format!("{:?}", signal.tier)) == *value)
                .unwrap_or(true)
        });
        items.retain(|signal| {
            query
                .min_build_score
                .map(|value| signal.build_score >= value)
                .unwrap_or(true)
        });
        items.retain(|signal| {
            query
                .cursor_ts
                .map(|cursor_ts| signal.ts < cursor_ts)
                .unwrap_or(true)
        });
        sort_latest_signals(&mut items);
        items.truncate(limit);
        AltContractLatestResponse {
            summary: self.summary(query.symbol.as_deref()),
            items,
            limit,
        }
    }

    pub fn ranked(&self, symbol: Option<&str>, limit: usize) -> AltContractLatestResponse {
        self.signals_for_view(symbol, limit, sort_ranked_signals)
    }

    pub fn top_impact(&self, symbol: Option<&str>, limit: usize) -> AltContractLatestResponse {
        self.signals_for_view(symbol, limit, sort_top_impact_signals)
    }

    pub fn outcome_summary(&self, filter: AltContractOutcomeFilter) -> AltContractOutcomeSummary {
        let config = binance_alt_contract_runtime_config();
        let min_samples_for_reporting = config.outcomes.min_samples_for_reporting;
        if !config.outcomes.enabled {
            return AltContractOutcomeSummary {
                insufficient_samples: true,
                min_samples_for_reporting,
                ..AltContractOutcomeSummary::default()
            };
        }
        let outcomes = self
            .state
            .read()
            .outcomes
            .values()
            .cloned()
            .filter(|outcome| outcome_matches_filter(outcome, &filter))
            .collect::<Vec<_>>();
        let completed = outcomes
            .iter()
            .filter(|outcome| outcome.markout_1h_bps.is_some())
            .collect::<Vec<_>>();
        let sample_count = completed.len();
        let follow_through_rate = (sample_count > 0).then(|| {
            completed
                .iter()
                .filter(|outcome| outcome.follow_through_1h == Some(true))
                .count() as f64
                / sample_count as f64
                * 100.0
        });
        AltContractOutcomeSummary {
            sample_count,
            insufficient_samples: sample_count < min_samples_for_reporting,
            min_samples_for_reporting,
            follow_through_rate,
            median_markout_bps: median_optional(
                completed
                    .iter()
                    .filter_map(|outcome| outcome.markout_1h_bps),
            ),
            median_mfe_bps: median_optional(
                completed.iter().filter_map(|outcome| outcome.mfe_1h_bps),
            ),
            median_mae_bps: median_optional(
                completed.iter().filter_map(|outcome| outcome.mae_1h_bps),
            ),
        }
    }

    fn signals_for_view(
        &self,
        symbol: Option<&str>,
        limit: usize,
        sorter: fn(&mut [AltContractSignal]),
    ) -> AltContractLatestResponse {
        let config = binance_alt_contract_runtime_config();
        let limit = limit.clamp(1, 200);
        if !self.runtime_enabled(&config) {
            return AltContractLatestResponse {
                summary: self.summary(symbol),
                items: Vec::new(),
                limit,
            };
        }
        let product_filter = symbol.map(product_id_for_symbol);
        let mut items = self
            .state
            .read()
            .signals
            .iter()
            .filter(|signal| {
                product_filter
                    .as_ref()
                    .map(|item| &signal.product_id == item)
                    .unwrap_or(true)
            })
            .filter(|signal| display_signal(signal, &config))
            .cloned()
            .collect::<Vec<_>>();
        sorter(&mut items);
        items.truncate(limit);
        AltContractLatestResponse {
            summary: self.summary(symbol),
            items,
            limit,
        }
    }

    fn insert_signal(&self, mut signal: AltContractSignal) -> bool {
        let config = binance_alt_contract_runtime_config();
        let mut state = self.state.write();
        if state.seen_signal_ids.contains(&signal.id) || duplicate_recent(&state.signals, &signal) {
            return false;
        }
        merge_event_state(&mut state, &mut signal, &config);
        let outcome = outcome_from_signal(&signal);
        let event_record = signal
            .event_id
            .as_ref()
            .and_then(|event_id| state.events.values().find(|event| &event.id == event_id))
            .map(|event| event_record_from_state(event, &signal.product_id));
        state.seen_signal_ids.insert(signal.id.clone());
        state.signals.push_back(signal.clone());
        state.outcomes.insert(signal.id.clone(), outcome.clone());
        prune_non_protected_signal_limit(&mut state.signals, MAX_SIGNALS);
        state.seen_signal_ids = state
            .signals
            .iter()
            .map(|signal| signal.id.clone())
            .collect();
        drop(state);
        self.persist_signal(&signal);
        self.persist_outcome(&outcome);
        if let Some(event) = event_record.as_ref() {
            self.persist_event(event);
        }
        true
    }

    fn update_post_signal_validation(&self, trade: &AltContractTrade) {
        let mut state = self.state.write();
        let context = state
            .contexts
            .get(&trade.product_id)
            .cloned()
            .unwrap_or_default();
        for signal in state
            .signals
            .iter_mut()
            .filter(|signal| signal.product_id == trade.product_id)
            .filter(|signal| signal.post_signal_status == "pending")
        {
            let age_ms = trade.ts.saturating_sub(signal.ts);
            if age_ms < 5 * 60_000 || signal.signal_vwap <= 0.0 {
                continue;
            }
            let oi_contracting = context
                .oi_change_1m_base
                .or(context.oi_change_5m_base)
                .is_some_and(|change| change < 0.0);
            match signal.direction {
                super::types::AltContractDirection::Buy => {
                    if trade.price >= signal.signal_vwap && !oi_contracting {
                        signal.post_signal_status = "validated".to_string();
                        signal.validated_at = Some(trade.ts);
                        signal.retest_status = "defended".to_string();
                    } else if trade.price < signal.signal_vwap {
                        signal.post_signal_status = if matches!(
                            signal.signal_type,
                            super::types::AltContractSignalType::AbnormalPump
                                | super::types::AltContractSignalType::MainForceLongBuild
                        ) {
                            "trap".to_string()
                        } else {
                            "failed".to_string()
                        };
                        signal.failed_at = Some(trade.ts);
                        signal.retest_status = "lost".to_string();
                    }
                }
                super::types::AltContractDirection::Sell => {
                    if trade.price <= signal.signal_vwap && !oi_contracting {
                        signal.post_signal_status = "validated".to_string();
                        signal.validated_at = Some(trade.ts);
                        signal.retest_status = "defended".to_string();
                    } else if trade.price > signal.signal_vwap {
                        signal.post_signal_status = if matches!(
                            signal.signal_type,
                            super::types::AltContractSignalType::AbnormalDump
                                | super::types::AltContractSignalType::MainForceShortBuild
                        ) {
                            "trap".to_string()
                        } else {
                            "failed".to_string()
                        };
                        signal.failed_at = Some(trade.ts);
                        signal.retest_status = "lost".to_string();
                    }
                }
                _ => {}
            }
        }
        let outcome_updates = state
            .signals
            .iter()
            .filter(|signal| signal.product_id == trade.product_id)
            .map(|signal| (signal.id.clone(), signal.direction))
            .collect::<Vec<_>>();
        for (signal_id, direction) in outcome_updates {
            if let Some(outcome) = state.outcomes.get_mut(&signal_id) {
                update_outcome_with_price(outcome, direction, trade.price, trade.ts);
            }
        }
        let persisted = state
            .outcomes
            .values()
            .filter(|outcome| outcome.product_id == trade.product_id)
            .cloned()
            .collect::<Vec<_>>();
        drop(state);
        for outcome in persisted {
            self.persist_outcome(&outcome);
        }
    }

    fn update_outcomes_with_price(&self, product_id: &str, price: f64, ts: i64) {
        let mut state = self.state.write();
        let updates = state
            .signals
            .iter()
            .filter(|signal| signal.product_id == product_id)
            .map(|signal| (signal.id.clone(), signal.direction))
            .collect::<Vec<_>>();
        for (signal_id, direction) in updates {
            if let Some(outcome) = state.outcomes.get_mut(&signal_id) {
                update_outcome_with_price(outcome, direction, price, ts);
            }
        }
        let persisted = state
            .outcomes
            .values()
            .filter(|outcome| outcome.product_id == product_id)
            .cloned()
            .collect::<Vec<_>>();
        drop(state);
        for outcome in persisted {
            self.persist_outcome(&outcome);
        }
    }

    fn mark_trade(&self, exchange: AltContractExchange, ts: i64) {
        let mut state = self.state.write();
        let entry = state
            .exchanges
            .entry(exchange.as_key().to_string())
            .or_insert_with(AltContractExchangeStatus::disconnected);
        let now = now_ms();
        entry.connected = true;
        entry.status = "connected".to_string();
        entry.last_trade_at = Some(ts);
        entry.latency_ms = Some(now.saturating_sub(ts).max(0));
        entry.last_error = None;
    }

    fn mark_candidate_product(&self, product_id: &str, ts: i64) {
        let product_id = product_id_for_symbol(product_id);
        let mut state = self.state.write();
        state.light_candidates_total = state.light_candidates_total.saturating_add(1);
        state.candidate_seen_at.insert(product_id.clone(), ts);
        state.hot_oi_seen_at.insert(product_id, ts);
    }

    fn should_run_detector(
        &self,
        product_id: &str,
        ts: i64,
        config: &super::config::BinanceAltContractRuntimeConfig,
        force_scan: bool,
    ) -> bool {
        let product_id = product_id_for_symbol(product_id);
        let min_interval_ms = config.detector.scan_interval_ms.max(1);
        let mut state = self.state.write();
        let last_scan_at = state.last_detector_scan_at.get(&product_id).copied();
        if !force_scan && last_scan_at.is_some_and(|last| ts.saturating_sub(last) < min_interval_ms)
        {
            return false;
        }
        state.last_detector_scan_at.insert(product_id, ts);
        true
    }

    fn window_stats_for_product(
        &self,
        product_id: &str,
        now: i64,
        config: &super::config::BinanceAltContractRuntimeConfig,
    ) -> (AltContractContext, Vec<AltContractWindowStats>) {
        let meta = self.meta_for_product(product_id);
        let context = self.context_for_product(product_id);
        let state = self.state.read();
        let Some(symbol_trades) = state.trades_by_product.get(product_id) else {
            return (context, Vec::new());
        };
        let window_stats = config
            .windows_sec
            .iter()
            .filter_map(|window_sec| {
                rolling_window_stats(
                    symbol_trades,
                    &state.flow_book,
                    &meta,
                    *window_sec,
                    now,
                    &context,
                    self.booted_at_ms,
                    config,
                )
            })
            .collect();
        (context, window_stats)
    }

    fn score_candidate(
        &self,
        product_id: &str,
        now: i64,
        config: &super::config::BinanceAltContractRuntimeConfig,
    ) -> Vec<AltContractSignal> {
        let (context, window_stats) = self.window_stats_for_product(product_id, now, config);
        if !window_stats
            .iter()
            .any(|stats| light_scan_candidate(stats, &context, config))
        {
            return Vec::new();
        }
        let window_confirmations = window_stats
            .iter()
            .map(|stats| window_confirmation_for(stats, config))
            .collect::<Vec<_>>();
        let market_context = self.market_impulse_context(
            product_id,
            now,
            window_stats
                .iter()
                .find(|stats| stats.window_sec == 60)
                .or_else(|| window_stats.first()),
        );
        let mut candidates = window_stats
            .into_iter()
            .filter(|stats| light_scan_candidate(stats, &context, config))
            .filter_map(|stats| {
                detect_alt_contract_signal_with_context(
                    &stats,
                    &context,
                    config,
                    window_confirmations.clone(),
                    market_context.clone(),
                )
            })
            .collect::<Vec<_>>();
        candidates.sort_by(|left, right| {
            right
                .severity
                .rank()
                .cmp(&left.severity.rank())
                .then_with(|| {
                    right
                        .abnormal_score
                        .max(right.build_score)
                        .cmp(&left.abnormal_score.max(left.build_score))
                })
        });
        let Some(best) = candidates.into_iter().next() else {
            return Vec::new();
        };
        self.insert_signal(best.clone())
            .then_some(best)
            .into_iter()
            .collect()
    }

    fn market_impulse_context(
        &self,
        product_id: &str,
        now: i64,
        current_stats: Option<&AltContractWindowStats>,
    ) -> MarketImpulseContext {
        let Some(current_stats) = current_stats else {
            return MarketImpulseContext::default();
        };
        let direction = current_stats.direction;
        if !matches!(
            direction,
            super::types::AltContractDirection::Buy | super::types::AltContractDirection::Sell
        ) {
            return MarketImpulseContext::default();
        }
        let config = binance_alt_contract_runtime_config();
        let state = self.state.read();
        let mut by_symbol = BTreeMap::<String, (f64, f64, f64)>::new();
        for symbol in state.flow_book.symbols() {
            let Some(window) = state.flow_book.window(symbol, 180, now) else {
                continue;
            };
            let entry = by_symbol.entry(symbol.to_string()).or_default();
            entry.0 = window.buy_notional_usd;
            entry.1 = window.sell_notional_usd;
            entry.2 = window.total_notional_usd;
        }
        let same_direction = by_symbol
            .iter()
            .filter(|(_, (buy, sell, total))| {
                let net = buy - sell;
                let dominance = if *total > 0.0 {
                    net.abs() / *total
                } else {
                    0.0
                };
                dominance >= 0.60
                    && match direction {
                        super::types::AltContractDirection::Buy => net > 0.0,
                        super::types::AltContractDirection::Sell => net < 0.0,
                        _ => false,
                    }
            })
            .map(|(symbol, (_, _, total))| (symbol.clone(), *total))
            .collect::<Vec<_>>();
        let active_symbol_count = by_symbol.len().max(1);
        let ratio = same_direction.len() as f64 / active_symbol_count as f64;
        let mut ranked = same_direction;
        ranked.sort_by(|left, right| right.1.total_cmp(&left.1));
        let current_product = product_id_for_symbol(product_id);
        let rank = ranked
            .iter()
            .position(|(symbol, _)| symbol == &current_product)
            .map(|index| index as u32 + 1);
        MarketImpulseContext {
            market_wide_move: ranked.len() >= config.discord.market_wide_symbol_count
                || ratio >= config.discord.market_wide_ratio,
            market_wide_direction: Some(
                match direction {
                    super::types::AltContractDirection::Buy => "buy",
                    super::types::AltContractDirection::Sell => "sell",
                    _ => "neutral",
                }
                .to_string(),
            ),
            market_impulse_ratio: ratio,
            relative_strength_rank: rank,
        }
    }

    fn context_for_product(&self, product_id: &str) -> AltContractContext {
        let now = now_ms();
        let config = binance_alt_contract_runtime_config();
        let product_id = product_id_for_symbol(product_id);
        let mut context = self
            .state
            .read()
            .contexts
            .get(&product_id)
            .cloned()
            .unwrap_or_else(empty_context);
        let liquidation_stale = self
            .state
            .read()
            .liquidation_seen_at
            .get(&product_id)
            .map(|seen_at| now.saturating_sub(*seen_at) > LIQUIDATION_CONTEXT_TTL_MS)
            .unwrap_or(true);
        if liquidation_stale {
            context.liquidation_notional_usd = None;
            context.liquidation_count = 0;
            context.dominant_liquidation_side = LiquidationSide::Unknown;
            context.liquidation_suspected = false;
            context.force_order_snapshot = false;
        }
        let oi_stale = context.oi_updated_at.is_some_and(|updated_at| {
            now.saturating_sub(updated_at)
                > i64::try_from(config.oi.max_snapshot_gap_seconds)
                    .unwrap_or(i64::MAX)
                    .saturating_mul(1_000)
        });
        if oi_stale {
            mark_oi_period_stale(&mut context.oi_change_1m);
            mark_oi_period_stale(&mut context.oi_change_5m);
            context.oi_change_1m_base = None;
            context.oi_change_5m_base = None;
            context.oi_change_pct = None;
        }
        context
    }

    fn meta_for_product(&self, product_id: &str) -> AltContractSymbolMeta {
        let product_id = product_id_for_symbol(product_id);
        self.state
            .read()
            .symbol_metas
            .get(&product_id)
            .cloned()
            .unwrap_or_else(|| meta_from_product_id(&product_id))
    }

    fn persist_signal(&self, signal: &AltContractSignal) {
        if let Some(store) = &self.store {
            if let Some(tx) = self.persistence_tx.read().as_ref() {
                match tx.try_send(PersistenceCommand::InsertSignal(signal.clone())) {
                    Ok(()) => {
                        let mut state = self.state.write();
                        state.persistence_queue_depth =
                            state.persistence_queue_depth.saturating_add(1);
                        state
                            .oldest_persistence_enqueued_at
                            .get_or_insert_with(now_ms);
                    }
                    Err(mpsc::error::TrySendError::Full(_)) => {
                        self.record_persistence_drop(signal.id.as_str(), "full");
                    }
                    Err(mpsc::error::TrySendError::Closed(_)) => {
                        self.record_persistence_drop(signal.id.as_str(), "closed");
                    }
                }
            } else if let Err(error) = store.upsert_alt_contract_signal(signal) {
                tracing::warn!(target: LOG_TARGET, signal_id = signal.id.as_str(), "{} failed to persist BACM signal before worker start: {error}", LOG_PREFIX);
            }
            return;
        }
        let config = binance_alt_contract_runtime_config();
        if !config.storage.jsonl_archive_enabled {
            return;
        }
        let path = self.persistence_path.clone();
        let persistence_lock = self.persistence_lock.clone();
        let signal_id = signal.id.clone();
        let payload = match serde_json::to_string(signal) {
            Ok(payload) => payload,
            Err(error) => {
                tracing::warn!(target: LOG_TARGET, signal_id, "{} failed to serialize JSONL archive: {error}", LOG_PREFIX);
                return;
            }
        };
        let write_archive = move || {
            let _guard = persistence_lock.lock();
            if let Some(parent) = path.parent() {
                if let Err(error) = fs::create_dir_all(parent) {
                    tracing::warn!(target: LOG_TARGET, "{} failed to create JSONL archive dir: {error}", LOG_PREFIX);
                    return;
                }
            }
            match OpenOptions::new().create(true).append(true).open(&path) {
                Ok(mut file) => {
                    if let Err(error) = writeln!(file, "{payload}") {
                        tracing::warn!(target: LOG_TARGET, signal_id, "{} failed to write JSONL archive: {error}", LOG_PREFIX);
                    }
                }
                Err(error) => {
                    tracing::warn!(target: LOG_TARGET, "{} failed to open JSONL archive: {error}", LOG_PREFIX);
                }
            }
        };
        if tokio::runtime::Handle::try_current().is_ok() {
            tokio::task::spawn_blocking(write_archive);
        } else {
            write_archive();
        }
    }

    fn persist_outcome(&self, outcome: &AltContractSignalOutcome) {
        let Some(store) = &self.store else {
            return;
        };
        if let Some(tx) = self.persistence_tx.read().as_ref() {
            match tx.try_send(PersistenceCommand::UpdateOutcome(outcome.clone())) {
                Ok(()) => {
                    let mut state = self.state.write();
                    state.persistence_queue_depth = state.persistence_queue_depth.saturating_add(1);
                    state
                        .oldest_persistence_enqueued_at
                        .get_or_insert_with(now_ms);
                }
                Err(mpsc::error::TrySendError::Full(_)) => {
                    self.record_persistence_drop(outcome.signal_id.as_str(), "full");
                }
                Err(mpsc::error::TrySendError::Closed(_)) => {
                    self.record_persistence_drop(outcome.signal_id.as_str(), "closed");
                }
            }
        } else if let Err(error) = store.upsert_alt_contract_outcome(outcome) {
            tracing::warn!(target: LOG_TARGET, signal_id = outcome.signal_id.as_str(), "{} failed to persist BACM outcome before worker start: {error}", LOG_PREFIX);
        }
    }

    fn persist_event(&self, event: &AltContractEventRecord) {
        let Some(store) = &self.store else {
            return;
        };
        if let Some(tx) = self.persistence_tx.read().as_ref() {
            match tx.try_send(PersistenceCommand::UpsertEvent(event.clone())) {
                Ok(()) => {
                    let mut state = self.state.write();
                    state.persistence_queue_depth = state.persistence_queue_depth.saturating_add(1);
                    state
                        .oldest_persistence_enqueued_at
                        .get_or_insert_with(now_ms);
                }
                Err(mpsc::error::TrySendError::Full(_)) => {
                    self.record_persistence_drop(event.event_id.as_str(), "full");
                }
                Err(mpsc::error::TrySendError::Closed(_)) => {
                    self.record_persistence_drop(event.event_id.as_str(), "closed");
                }
            }
        } else if let Err(error) = store.upsert_alt_contract_event(event) {
            tracing::warn!(target: LOG_TARGET, event_id = event.event_id.as_str(), "{} failed to persist BACM event before worker start: {error}", LOG_PREFIX);
        }
    }

    fn start_persistence_worker(&self) {
        let Some(store) = self.store.clone() else {
            return;
        };
        if self.persistence_tx.read().is_some() {
            return;
        }
        let runtime_config = binance_alt_contract_runtime_config();
        let queue_capacity = runtime_config.storage.queue_capacity.max(1);
        let batch_size = runtime_config.storage.batch_size.max(1);
        let flush_interval_ms = runtime_config.storage.flush_interval_ms.max(1);
        let (tx, mut rx) = mpsc::channel::<PersistenceCommand>(queue_capacity);
        *self.persistence_tx.write() = Some(tx);
        let state = self.state.clone();
        self.tasks.write().push(tokio::spawn(async move {
            while let Some(first_command) = rx.recv().await {
                let mut batch = vec![first_command];
                let flush_at = tokio::time::Instant::now()
                    + Duration::from_millis(flush_interval_ms);
                while batch.len() < batch_size {
                    tokio::select! {
                        next = rx.recv() => match next {
                            Some(command) => batch.push(command),
                            None => break,
                        },
                        _ = tokio::time::sleep_until(flush_at) => break,
                    }
                }
                let batch_len = batch.len();
                let store = store.clone();
                let mut attempt = 0_u32;
                loop {
                    let batch_for_write = batch.clone();
                    let store_for_write = store.clone();
                    let result = tokio::task::spawn_blocking(move || {
                        let signals = batch_for_write
                            .iter()
                            .filter_map(|command| match command {
                                PersistenceCommand::InsertSignal(signal) => Some(signal.clone()),
                                PersistenceCommand::UpdateOutcome(_) | PersistenceCommand::UpsertEvent(_) => None,
                            })
                            .collect::<Vec<_>>();
                        store_for_write.upsert_alt_contract_signals(&signals)?;
                        for event in batch_for_write.iter().filter_map(|command| match command {
                            PersistenceCommand::UpsertEvent(event) => Some(event),
                            _ => None,
                        }) {
                            store_for_write.upsert_alt_contract_event(event)?;
                        }
                        for outcome in batch_for_write.iter().filter_map(|command| match command {
                            PersistenceCommand::InsertSignal(_) | PersistenceCommand::UpsertEvent(_) => None,
                            PersistenceCommand::UpdateOutcome(outcome) => Some(outcome),
                        }) {
                            store_for_write.upsert_alt_contract_outcome(outcome)?;
                        }
                        Ok::<(), anyhow::Error>(())
                    })
                    .await;
                    match result {
                        Ok(Ok(())) => break,
                        Ok(Err(error)) if attempt < PERSISTENCE_MAX_RETRIES => {
                            attempt = attempt.saturating_add(1);
                            tracing::warn!(
                                target: LOG_TARGET,
                                attempt,
                                "{} BACM persistence batch failed; retrying: {error}",
                                LOG_PREFIX
                            );
                            tokio::time::sleep(Duration::from_millis(
                                25_u64.saturating_mul(1_u64 << attempt.min(5)),
                            ))
                            .await;
                        }
                        Ok(Err(error)) => {
                            tracing::warn!(target: LOG_TARGET, "{} failed to persist BACM signal asynchronously after retries: {error}", LOG_PREFIX);
                            break;
                        }
                        Err(error) if attempt < PERSISTENCE_MAX_RETRIES => {
                            attempt = attempt.saturating_add(1);
                            tracing::warn!(target: LOG_TARGET, attempt, "{} BACM persistence worker join failed; retrying: {error}", LOG_PREFIX);
                        }
                        Err(error) => {
                            tracing::warn!(target: LOG_TARGET, "{} BACM persistence worker failed after retries: {error}", LOG_PREFIX);
                            break;
                        }
                    }
                }
                let mut state = state.write();
                state.persistence_queue_depth = state.persistence_queue_depth.saturating_sub(batch_len);
                if state.persistence_queue_depth == 0 {
                    state.oldest_persistence_enqueued_at = None;
                }
            }
        }));
    }

    fn record_persistence_drop(&self, signal_id: &str, reason: &str) {
        let mut state = self.state.write();
        state.persistence_dropped_total = state.persistence_dropped_total.saturating_add(1);
        tracing::warn!(
            target: LOG_TARGET,
            signal_id,
            reason,
            dropped_total = state.persistence_dropped_total,
            "{} BACM persistence queue rejected signal",
            LOG_PREFIX
        );
    }

    async fn run_cache_cleanup_loop(self) {
        let interval_sec = binance_alt_contract_runtime_config()
            .storage
            .cleanup_interval_sec
            .max(60);
        let mut interval = tokio::time::interval(Duration::from_secs(interval_sec));
        interval.set_missed_tick_behavior(MissedTickBehavior::Skip);
        loop {
            interval.tick().await;
            self.prune_expired_cache(now_ms());
        }
    }

    fn prune_expired_cache(&self, now: i64) {
        let config = binance_alt_contract_runtime_config();
        let signal_retention_ms = retention_days_to_ms(config.storage.signals_retention_days);
        let context_retention_ms = signal_retention_ms.max(OI_RETENTION_MS);
        let mut state = self.state.write();
        let signals_before = state.signals.len();
        let events_before = state.events.len();
        let oi_symbols_before = state.oi_snapshots.len();

        state
            .signals
            .retain(|signal| signal_survives_retention(signal, now, signal_retention_ms));
        prune_non_protected_signal_limit(&mut state.signals, MAX_SIGNALS);
        state.seen_signal_ids = state
            .signals
            .iter()
            .map(|signal| signal.id.clone())
            .collect();

        state
            .events
            .retain(|_, event| !expired(event.updated_at, now, signal_retention_ms));
        prune_timestamp_map(
            &mut state.liquidation_seen_at,
            now,
            LIQUIDATION_CONTEXT_TTL_MS,
        );
        prune_timestamp_map(&mut state.candidate_seen_at, now, context_retention_ms);
        prune_timestamp_map(&mut state.hot_oi_seen_at, now, context_retention_ms);
        prune_timestamp_map(&mut state.last_detector_scan_at, now, context_retention_ms);
        prune_timestamp_map(
            &mut state.active_symbol_last_trade_at,
            now,
            TRADE_RETENTION_MS,
        );
        for trades in state.trades_by_product.values_mut() {
            prune_trades(trades, now);
        }
        state
            .trades_by_product
            .retain(|_, trades| !trades.is_empty());
        for snapshots in state.oi_snapshots.values_mut() {
            while snapshots
                .front()
                .is_some_and(|(seen_at, _)| expired(*seen_at, now, OI_RETENTION_MS))
            {
                snapshots.pop_front();
            }
        }
        state
            .oi_snapshots
            .retain(|_, snapshots| !snapshots.is_empty());
        while state
            .error_events
            .front()
            .is_some_and(|seen_at| expired(*seen_at, now, 60 * 60_000))
        {
            state.error_events.pop_front();
        }

        let retained_signals = state.signals.iter().cloned().collect::<Vec<_>>();
        let signals_pruned = signals_before.saturating_sub(retained_signals.len());
        let events_pruned = events_before.saturating_sub(state.events.len());
        let oi_symbols_pruned = oi_symbols_before.saturating_sub(state.oi_snapshots.len());
        drop(state);

        let persistence_result = if let Some(store) = &self.store {
            store
                .prune_alt_contract_signals(now.saturating_sub(signal_retention_ms))
                .map(|_| ())
                .map_err(|error| std::io::Error::other(error.to_string()))
        } else {
            self.rewrite_persisted_signals(&retained_signals)
        };
        if let Err(error) = persistence_result {
            tracing::warn!(
                target: LOG_TARGET,
                "{} failed to compact BACM signal cache: {error}",
                LOG_PREFIX
            );
            return;
        }
        tracing::info!(
            target: LOG_TARGET,
            signals_pruned,
            events_pruned,
            oi_symbols_pruned,
            retained_signals = retained_signals.len(),
            retention_days = config.storage.signals_retention_days,
            "{} cache cleanup complete",
            LOG_PREFIX
        );
    }

    fn rewrite_persisted_signals(&self, signals: &[AltContractSignal]) -> std::io::Result<()> {
        let _guard = self.persistence_lock.lock();
        if signals.is_empty() {
            match fs::remove_file(&self.persistence_path) {
                Ok(()) => {}
                Err(error) if error.kind() == std::io::ErrorKind::NotFound => {}
                Err(error) => return Err(error),
            }
            return Ok(());
        }
        if let Some(parent) = self.persistence_path.parent() {
            fs::create_dir_all(parent)?;
        }
        let tmp_path = self.persistence_path.with_extension("jsonl.tmp");
        {
            let mut file = OpenOptions::new()
                .create(true)
                .write(true)
                .truncate(true)
                .open(&tmp_path)?;
            for signal in signals {
                let payload = serde_json::to_string(signal)
                    .map_err(|error| std::io::Error::new(std::io::ErrorKind::Other, error))?;
                writeln!(file, "{payload}")?;
            }
            file.flush()?;
        }
        let _ = fs::remove_file(&self.persistence_path);
        fs::rename(tmp_path, &self.persistence_path)?;
        Ok(())
    }
}

#[derive(Debug, Default)]
struct RestoredAltContractSignals {
    signals: VecDeque<AltContractSignal>,
    seen_signal_ids: BTreeSet<String>,
}

fn load_persisted_signals(
    path: &PathBuf,
    limit: usize,
    now: i64,
    retention_ms: i64,
) -> RestoredAltContractSignals {
    let Ok(text) = fs::read_to_string(path) else {
        return RestoredAltContractSignals::default();
    };
    let mut signals = text
        .lines()
        .rev()
        .filter_map(|line| serde_json::from_str::<AltContractSignal>(line).ok())
        .scan(0usize, |retained_non_protected, signal| {
            if is_protected_signal(&signal) {
                return Some(Some(signal));
            }
            if expired(signal.ts, now, retention_ms) || *retained_non_protected >= limit {
                return Some(None);
            }
            *retained_non_protected += 1;
            Some(Some(signal))
        })
        .flatten()
        .collect::<Vec<_>>();
    signals.reverse();
    let seen_signal_ids = signals.iter().map(|signal| signal.id.clone()).collect();
    RestoredAltContractSignals {
        signals: VecDeque::from(signals),
        seen_signal_ids,
    }
}

fn load_persisted_sqlite_signals(store: &SqliteStore, limit: usize) -> RestoredAltContractSignals {
    let mut signals = store.load_alt_contract_signals(limit).unwrap_or_default();
    signals.reverse();
    let seen_signal_ids = signals.iter().map(|signal| signal.id.clone()).collect();
    RestoredAltContractSignals {
        signals: VecDeque::from(signals),
        seen_signal_ids,
    }
}

fn retention_days_to_ms(days: u64) -> i64 {
    i64::try_from(days.max(1))
        .unwrap_or(i64::MAX / 86_400_000)
        .saturating_mul(86_400_000)
}

fn is_protected_signal(signal: &AltContractSignal) -> bool {
    signal.severity == AltContractSeverity::S
}

fn signal_survives_retention(signal: &AltContractSignal, now: i64, retention_ms: i64) -> bool {
    is_protected_signal(signal) || !expired(signal.ts, now, retention_ms)
}

fn prune_non_protected_signal_limit(signals: &mut VecDeque<AltContractSignal>, limit: usize) {
    while signals
        .iter()
        .filter(|signal| !is_protected_signal(signal))
        .count()
        > limit
    {
        let Some(index) = signals
            .iter()
            .position(|signal| !is_protected_signal(signal))
        else {
            break;
        };
        signals.remove(index);
    }
}

fn expired(ts: i64, now: i64, retention_ms: i64) -> bool {
    now.saturating_sub(ts) > retention_ms
}

fn prune_timestamp_map(map: &mut BTreeMap<String, i64>, now: i64, retention_ms: i64) {
    map.retain(|_, seen_at| !expired(*seen_at, now, retention_ms));
}

fn summarized_exchange_statuses(
    enabled: bool,
    exchanges: &BTreeMap<String, AltContractExchangeStatus>,
    now: i64,
    stale_ms: i64,
) -> BTreeMap<String, AltContractExchangeStatus> {
    exchanges
        .iter()
        .map(|(exchange, status)| {
            (
                exchange.clone(),
                summarized_exchange_status(enabled, status, now, stale_ms),
            )
        })
        .collect()
}

fn summarized_exchange_status(
    enabled: bool,
    status: &AltContractExchangeStatus,
    now: i64,
    stale_ms: i64,
) -> AltContractExchangeStatus {
    let mut item = status.clone();
    if !enabled || item.status == "disabled" {
        return item;
    }
    match item.last_trade_at {
        Some(last_trade_at) => {
            let age_ms = now.saturating_sub(last_trade_at);
            item.latency_ms = Some(age_ms.max(0));
            if age_ms > stale_ms {
                item.connected = false;
                item.status = "stale".to_string();
            } else {
                item.connected = true;
                item.status = "connected".to_string();
            }
        }
        None if item.connected || item.status == "connected" => {
            item.connected = true;
            item.latency_ms = None;
        }
        None => {}
    }
    item
}

fn duplicate_recent(signals: &VecDeque<AltContractSignal>, signal: &AltContractSignal) -> bool {
    signals.iter().rev().take(20).any(|existing| {
        existing.product_id == signal.product_id
            && existing.signal_type == signal.signal_type
            && existing.direction == signal.direction
            && signal.ts.saturating_sub(existing.ts) <= DUPLICATE_WINDOW_MS
            && existing.severity.rank() >= signal.severity.rank()
            && materially_equivalent_signal(existing, signal)
    })
}

fn materially_equivalent_signal(existing: &AltContractSignal, signal: &AltContractSignal) -> bool {
    score_delta_under(
        existing.alt_impact_score.final_score,
        signal.alt_impact_score.final_score,
        5.0,
    ) && relative_delta_under(existing.total_notional_usd, signal.total_notional_usd, 0.20)
        && optional_relative_delta_under(existing.dynamic_multiple, signal.dynamic_multiple, 0.20)
        && oi_context_key(existing) == oi_context_key(signal)
        && price_response_key(existing) == price_response_key(signal)
        && signal.liquidation_notional_usd.unwrap_or_default()
            <= existing.liquidation_notional_usd.unwrap_or_default() * 1.20
}

fn score_delta_under(left: f64, right: f64, max_delta: f64) -> bool {
    (left - right).abs() < max_delta
}

fn relative_delta_under(left: f64, right: f64, max_ratio: f64) -> bool {
    let baseline = left.abs().max(1.0);
    ((right - left).abs() / baseline) < max_ratio
}

fn optional_relative_delta_under(left: Option<f64>, right: Option<f64>, max_ratio: f64) -> bool {
    match (left, right) {
        (Some(left), Some(right)) => relative_delta_under(left, right, max_ratio),
        (None, None) => true,
        _ => false,
    }
}

fn oi_context_key(signal: &AltContractSignal) -> i8 {
    match signal.oi_change_pct.unwrap_or_default().partial_cmp(&0.0) {
        Some(std::cmp::Ordering::Greater) => 1,
        Some(std::cmp::Ordering::Less) => -1,
        _ => 0,
    }
}

fn price_response_key(signal: &AltContractSignal) -> i8 {
    match signal.price_move_pct.unwrap_or_default().partial_cmp(&0.0) {
        Some(std::cmp::Ordering::Greater) => 1,
        Some(std::cmp::Ordering::Less) => -1,
        _ => 0,
    }
}

fn merge_event_state(
    state: &mut BinanceAltContractState,
    signal: &mut AltContractSignal,
    config: &super::config::BinanceAltContractRuntimeConfig,
) {
    let now = signal.ts;
    state.events.retain(|_, event| {
        now.saturating_sub(event.updated_at) <= lifecycle_window_ms(event, config)
    });
    let event_key = format!(
        "{}:{:?}:{:?}",
        signal.product_id, signal.direction, signal.signal_type
    );
    let event = state
        .events
        .entry(event_key)
        .or_insert_with(|| AltContractEventState {
            id: format!(
                "bacm-event:{}:{:?}:{:?}:{}",
                signal.product_id, signal.direction, signal.signal_type, signal.ts
            ),
            start_ts: signal.ts,
            updated_at: signal.ts,
            tier: signal.tier,
            signal_type: format!("{:?}", signal.signal_type),
            direction: format!("{:?}", signal.direction),
            liquidation_driven: signal.liquidation_suspected,
            status: "active".to_string(),
            close_reason: None,
            latest_signal_id: signal.id.clone(),
            peak_signal_id: signal.id.clone(),
            peak_abnormal_score: signal.abnormal_score,
            peak_build_score: signal.build_score,
            signal_count: 0,
        });
    event.start_ts = event.start_ts.min(signal.ts);
    event.updated_at = signal.ts;
    event.latest_signal_id = signal.id.clone();
    event.liquidation_driven |= signal.liquidation_suspected;
    if signal.abnormal_score >= event.peak_abnormal_score
        && signal.build_score >= event.peak_build_score
    {
        event.peak_signal_id = signal.id.clone();
    }
    event.status = "active".to_string();
    event.close_reason = None;
    event.peak_abnormal_score = event.peak_abnormal_score.max(signal.abnormal_score);
    event.peak_build_score = event.peak_build_score.max(signal.build_score);
    event.signal_count = event.signal_count.saturating_add(1);
    signal.event_id = Some(event.id.clone());
    signal.event_start_ts = Some(event.start_ts);
    signal.event_status = event.status.clone();
    signal.event_close_reason = event.close_reason.clone();
    signal.event_latest_signal_id = Some(event.latest_signal_id.clone());
    signal.event_peak_signal_id = Some(event.peak_signal_id.clone());
    signal.event_signal_count = event.signal_count;
    signal.event_peak_abnormal_score = event.peak_abnormal_score;
    signal.event_peak_build_score = event.peak_build_score;
}

fn event_state_from_record(record: AltContractEventRecord) -> AltContractEventState {
    AltContractEventState {
        id: record.event_id,
        start_ts: record.start_ts,
        updated_at: record.last_update_ts,
        tier: record.tier,
        signal_type: record.signal_type,
        direction: record.direction,
        liquidation_driven: record.liquidation_driven,
        status: record.status,
        close_reason: None,
        latest_signal_id: record.latest_signal_id.unwrap_or_default(),
        peak_signal_id: record.peak_signal_id.unwrap_or_default(),
        peak_abnormal_score: record.peak_abnormal_score,
        peak_build_score: record.peak_build_score,
        signal_count: record.signal_count,
    }
}

fn event_record_from_state(
    event: &AltContractEventState,
    product_id: &str,
) -> AltContractEventRecord {
    AltContractEventRecord {
        event_id: event.id.clone(),
        product_id: product_id.to_string(),
        signal_type: event.signal_type.clone(),
        direction: event.direction.clone(),
        tier: event.tier,
        liquidation_driven: event.liquidation_driven,
        start_ts: event.start_ts,
        last_update_ts: event.updated_at,
        status: event.status.clone(),
        latest_signal_id: Some(event.latest_signal_id.clone()),
        peak_signal_id: Some(event.peak_signal_id.clone()),
        signal_count: event.signal_count,
        peak_abnormal_score: event.peak_abnormal_score,
        peak_build_score: event.peak_build_score,
    }
}

fn lifecycle_window_ms(
    event: &AltContractEventState,
    config: &super::config::BinanceAltContractRuntimeConfig,
) -> i64 {
    let seconds = if event.liquidation_driven {
        config.lifecycle.liquidation_merge_seconds
    } else {
        match event.tier {
            AltContractSymbolTier::A | AltContractSymbolTier::B => {
                config.lifecycle.tier_a_b_merge_seconds
            }
            AltContractSymbolTier::C => config.lifecycle.tier_c_merge_seconds,
            AltContractSymbolTier::D | AltContractSymbolTier::E => {
                config.lifecycle.tier_d_e_merge_seconds
            }
        }
    };
    i64::try_from(seconds.max(1))
        .unwrap_or(i64::MAX / 1_000)
        .saturating_mul(1_000)
}

fn prune_trades(trades: &mut VecDeque<AltContractTrade>, now: i64) {
    while trades.len() > MAX_TRADES
        || trades
            .front()
            .is_some_and(|trade| now.saturating_sub(trade.ts) > TRADE_RETENTION_MS)
    {
        trades.pop_front();
    }
}

fn push_error_event(state: &mut BinanceAltContractState, now: i64) {
    state.error_events.push_back(now);
    while state
        .error_events
        .front()
        .is_some_and(|seen_at| now.saturating_sub(*seen_at) > 60 * 60_000)
    {
        state.error_events.pop_front();
    }
}

fn prune_seen_map(items: &mut BTreeMap<String, i64>, now: i64, ttl_ms: i64) {
    items.retain(|_, seen_at| now.saturating_sub(*seen_at) <= ttl_ms);
}

fn recent_seen_keys(items: &BTreeMap<String, i64>, now: i64, ttl_ms: i64) -> Vec<String> {
    let mut keys = items
        .iter()
        .filter(|(_, seen_at)| now.saturating_sub(**seen_at) <= ttl_ms)
        .map(|(symbol, _)| symbol.clone())
        .collect::<Vec<_>>();
    keys.sort();
    keys
}

fn light_scan_candidate(
    stats: &AltContractWindowStats,
    context: &AltContractContext,
    config: &super::config::BinanceAltContractRuntimeConfig,
) -> bool {
    let thresholds = config.thresholds_for_tier(stats.tier);
    let high = thresholds.high_notional_usd.max(1.0);
    stats.total_notional_usd >= high * 0.6
        || (stats.window_sec >= 60 && stats.total_notional_usd >= high * 0.5)
        || stats.dynamic_multiple.is_some_and(|value| value >= 3.0)
        || stats.price_move_pct.is_some_and(|value| value.abs() >= 2.0)
        || context.force_order_snapshot
        || (stats.dominance >= 0.60 && stats.total_notional_usd >= high * 0.25)
}

fn candidate_priority(
    window_stats: &[AltContractWindowStats],
    context: &AltContractContext,
    config: &super::config::BinanceAltContractRuntimeConfig,
    ts: i64,
) -> Option<AltCandidatePriority> {
    window_stats
        .iter()
        .filter(|stats| light_scan_candidate(stats, context, config))
        .max_by(|left, right| {
            let left_ratio = left.total_notional_usd
                / config
                    .thresholds_for_tier(left.tier)
                    .high_notional_usd
                    .max(1.0);
            let right_ratio = right.total_notional_usd
                / config
                    .thresholds_for_tier(right.tier)
                    .high_notional_usd
                    .max(1.0);
            left_ratio.total_cmp(&right_ratio)
        })
        .map(|stats| AltCandidatePriority {
            product_id: stats.product_id.clone(),
            tier: stats.tier,
            window_sec: stats.window_sec,
            relative_notional: stats.total_notional_usd
                / config
                    .thresholds_for_tier(stats.tier)
                    .high_notional_usd
                    .max(1.0),
            dynamic_multiple: stats.dynamic_multiple.unwrap_or_default(),
            dominance: stats.dominance,
            abs_price_move_pct: stats.price_move_pct.unwrap_or_default().abs(),
            liquidation_present: context.liquidation_suspected || context.force_order_snapshot,
            candidate_created_at: ts,
            last_scored_at: None,
        })
}

fn oi_period_delta(
    snapshots: &VecDeque<(i64, f64)>,
    now: i64,
    period_sec: u64,
    min_history_sec: u64,
    max_gap_sec: u64,
) -> super::types::OiPeriodDelta {
    let period_ms = i64::try_from(period_sec)
        .unwrap_or(i64::MAX)
        .saturating_mul(1_000);
    let min_history_ms = i64::try_from(min_history_sec)
        .unwrap_or(i64::MAX)
        .saturating_mul(1_000);
    let max_gap_ms = i64::try_from(max_gap_sec)
        .unwrap_or(i64::MAX)
        .saturating_mul(1_000);
    let mut result = super::types::OiPeriodDelta {
        period_sec,
        ..Default::default()
    };
    let Some((after_ts, after_oi)) = snapshots.back().copied() else {
        result.reason = Some("missing_snapshot".to_string());
        return result;
    };
    result.after_ts = Some(after_ts);
    result.after_oi = Some(after_oi);
    if now.saturating_sub(after_ts) > max_gap_ms {
        result.stale = true;
        result.reason = Some("latest_snapshot_stale".to_string());
        return result;
    }
    if after_ts.saturating_sub(snapshots.front().map(|(ts, _)| *ts).unwrap_or(after_ts))
        < min_history_ms
    {
        result.reason = Some("insufficient_history".to_string());
        return result;
    }
    let target = after_ts.saturating_sub(period_ms);
    let Some((before_ts, before_oi)) = snapshots
        .iter()
        .copied()
        .min_by_key(|(snapshot_ts, _)| snapshot_ts.saturating_sub(target).abs())
    else {
        result.reason = Some("missing_reference_snapshot".to_string());
        return result;
    };
    result.before_ts = Some(before_ts);
    result.before_oi = Some(before_oi);
    if before_oi <= 0.0 || (before_ts.saturating_sub(target)).abs() > max_gap_ms {
        result.stale = true;
        result.reason = Some("reference_snapshot_stale_or_invalid".to_string());
        return result;
    }
    let delta = after_oi - before_oi;
    result.delta = Some(delta);
    result.delta_pct = Some(delta / before_oi * 100.0);
    result.available = true;
    result
}

fn mark_oi_period_stale(period: &mut super::types::OiPeriodDelta) {
    period.available = false;
    period.stale = true;
    period.reason = Some("latest_snapshot_stale".to_string());
}

fn liquidation_event_key(event: &AltLiquidationEvent) -> String {
    event.source_event_id.clone().unwrap_or_else(|| {
        format!(
            "{}:{}:{:?}:{:.8}:{:.8}",
            event.product_id,
            event.ts,
            event.side,
            event.price.unwrap_or_default(),
            event.quantity.unwrap_or_default()
        )
    })
}

fn prune_liquidation_events(
    events: &mut VecDeque<AltLiquidationEvent>,
    now: i64,
    retention_ms: i64,
) {
    while events
        .front()
        .is_some_and(|event| now.saturating_sub(event.ts) > retention_ms)
    {
        events.pop_front();
    }
}

fn liquidation_window(
    events: &VecDeque<AltLiquidationEvent>,
    window_sec: u64,
    now: i64,
) -> AltLiquidationWindow {
    let window_ms = i64::try_from(window_sec)
        .unwrap_or(i64::MAX)
        .saturating_mul(1_000);
    let mut summary = AltLiquidationWindow {
        window_sec,
        ..Default::default()
    };
    for event in events.iter().filter(|event| {
        now.saturating_sub(event.ts) >= 0 && now.saturating_sub(event.ts) <= window_ms
    }) {
        summary.liquidation_count += 1;
        summary.liquidation_total_usd += event.notional_usd;
        match event.side {
            LiquidationSide::LongLiquidation => summary.long_liquidation_usd += event.notional_usd,
            LiquidationSide::ShortLiquidation => {
                summary.short_liquidation_usd += event.notional_usd
            }
            LiquidationSide::Unknown => {}
        }
    }
    summary.dominant_liquidation_side =
        if summary.long_liquidation_usd > summary.short_liquidation_usd {
            LiquidationSide::LongLiquidation
        } else if summary.short_liquidation_usd > summary.long_liquidation_usd {
            LiquidationSide::ShortLiquidation
        } else {
            LiquidationSide::Unknown
        };
    summary
}

fn product_id_for_symbol(symbol: &str) -> String {
    let value = symbol.trim().to_ascii_uppercase();
    if value.ends_with("USDT") {
        value
    } else {
        format!("{value}USDT")
    }
}

fn compact_filter_value(value: &str) -> String {
    value
        .chars()
        .filter(|ch| *ch != '_')
        .flat_map(|ch| ch.to_lowercase())
        .collect()
}

fn status_from_severity(severity: AltContractSeverity) -> &'static str {
    match severity {
        AltContractSeverity::S | AltContractSeverity::Critical => "strong",
        AltContractSeverity::High => "active",
        AltContractSeverity::Medium => "watch",
        AltContractSeverity::Calm => "calm",
    }
}

fn health_status(
    enabled: bool,
    exchanges: &BTreeMap<String, AltContractExchangeStatus>,
    total_shards: usize,
    connected_shards: usize,
) -> String {
    if !enabled {
        return "disabled".to_string();
    }
    let connected = exchanges.values().filter(|status| status.connected).count();
    if connected == 0 || (total_shards > 0 && connected_shards == 0) {
        "unhealthy".to_string()
    } else if total_shards > 0 && connected_shards < total_shards {
        "degraded".to_string()
    } else if connected >= 1 {
        "healthy".to_string()
    } else {
        "unhealthy".to_string()
    }
}

fn health_reason(enabled: bool, health_status: &str) -> &'static str {
    if !enabled {
        "binance_alt_contract_monitor_disabled"
    } else {
        match health_status {
            "healthy" => "binance_alt_recent",
            "degraded" => "binance_alt_partial_shard_coverage",
            "unhealthy" => "binance_alt_stale_or_disconnected",
            _ => "binance_alt_status_unknown",
        }
    }
}

fn collector_status(enabled: bool, binance: Option<&AltContractExchangeStatus>) -> String {
    if !enabled {
        return "disabled".to_string();
    }
    match binance.map(|status| status.status.as_str()) {
        Some("connected") => "running".to_string(),
        Some("connecting") | Some("reconnecting") => "connecting".to_string(),
        Some("stale") => "waiting_data".to_string(),
        Some(value) => value.to_string(),
        None => "waiting_data".to_string(),
    }
}

fn dry_run_stats(signals: &VecDeque<AltContractSignal>, now: i64) -> AltContractDryRunStats {
    let one_hour = dry_run_window_stats(signals, now, 60 * 60_000);
    let one_day = dry_run_window_stats(signals, now, 24 * 60 * 60_000);
    AltContractDryRunStats {
        signals1h: one_hour.signals,
        high1h: one_hour.high,
        critical1h: one_hour.critical,
        s1h: one_hour.s,
        would_send1h: one_hour.would_send,
        skipped_low_score1h: one_hour.skipped_low_score,
        skipped_cooldown1h: one_hour.skipped_cooldown,
        skipped_data_quality1h: one_hour.skipped_data_quality,
        liquidation_driven1h: one_hour.liquidation_driven,
        signals24h: one_day.signals,
        high24h: one_day.high,
        critical24h: one_day.critical,
        s24h: one_day.s,
        would_send24h: one_day.would_send,
        skipped_low_score24h: one_day.skipped_low_score,
        skipped_cooldown24h: one_day.skipped_cooldown,
        skipped_data_quality24h: one_day.skipped_data_quality,
        liquidation_driven24h: one_day.liquidation_driven,
    }
}

#[derive(Debug, Default)]
struct DryRunWindowStats {
    signals: usize,
    high: usize,
    critical: usize,
    s: usize,
    would_send: usize,
    skipped_low_score: usize,
    skipped_cooldown: usize,
    skipped_data_quality: usize,
    liquidation_driven: usize,
}

fn dry_run_window_stats(
    signals: &VecDeque<AltContractSignal>,
    now: i64,
    window_ms: i64,
) -> DryRunWindowStats {
    let recent = signals
        .iter()
        .filter(|signal| now.saturating_sub(signal.ts) <= window_ms)
        .collect::<Vec<_>>();
    DryRunWindowStats {
        signals: recent.len(),
        high: recent
            .iter()
            .filter(|signal| signal.severity == AltContractSeverity::High)
            .count(),
        critical: recent
            .iter()
            .filter(|signal| signal.severity == AltContractSeverity::Critical)
            .count(),
        s: recent
            .iter()
            .filter(|signal| signal.severity == AltContractSeverity::S)
            .count(),
        would_send: recent
            .iter()
            .filter(|signal| signal.discord_would_send)
            .count(),
        skipped_low_score: recent
            .iter()
            .filter(|signal| {
                matches!(
                    signal.discord_reason.as_str(),
                    "low_score"
                        | "main_force_evidence_low"
                        | "liquidation_evidence_low"
                        | "tier_notional_low"
                        | "tier_critical_notional_low"
                        | "medium_or_low"
                )
            })
            .count(),
        skipped_cooldown: recent
            .iter()
            .filter(|signal| signal.discord_reason == "cooldown")
            .count(),
        skipped_data_quality: recent
            .iter()
            .filter(|signal| signal.discord_reason == "data_quality_low")
            .count(),
        liquidation_driven: recent
            .iter()
            .filter(|signal| signal.liquidation_suspected)
            .count(),
    }
}

fn top_active_symbols(flow_book: &PerSymbolFlowBook, now: i64) -> Vec<String> {
    let mut by_symbol = BTreeMap::<String, f64>::new();
    for product_id in flow_book.symbols() {
        let notional_usd = flow_book
            .window(product_id, 15 * 60, now)
            .map(|window| window.total_notional_usd)
            .unwrap_or_default();
        if notional_usd > 0.0 {
            by_symbol.insert(product_id.to_string(), notional_usd);
        }
    }
    let mut items = by_symbol.into_iter().collect::<Vec<_>>();
    items.sort_by(|left, right| {
        right
            .1
            .partial_cmp(&left.1)
            .unwrap_or(std::cmp::Ordering::Equal)
    });
    items
        .into_iter()
        .take(5)
        .map(|(symbol, _)| symbol)
        .collect()
}

fn summary_monitored_symbols(
    all_symbols: &[String],
    top_active_symbols: &[String],
    product_filter: Option<&str>,
    latest: Option<&AltContractSignal>,
) -> Vec<String> {
    let mut items = Vec::new();
    push_summary_symbol(&mut items, product_filter);
    push_summary_symbol(&mut items, latest.map(|signal| signal.product_id.as_str()));
    for symbol in top_active_symbols {
        push_summary_symbol(&mut items, Some(symbol));
    }
    for symbol in all_symbols {
        if items.len() >= SUMMARY_MONITORED_SYMBOL_LIMIT {
            break;
        }
        push_summary_symbol(&mut items, Some(symbol));
    }
    items
}

fn push_summary_symbol(items: &mut Vec<String>, symbol: Option<&str>) {
    if items.len() >= SUMMARY_MONITORED_SYMBOL_LIMIT {
        return;
    }
    let Some(symbol) = symbol else {
        return;
    };
    if symbol.is_empty() || items.iter().any(|item| item == symbol) {
        return;
    }
    items.push(symbol.to_string());
}

fn symbol_universe_summary(
    config: &super::config::BinanceAltContractRuntimeConfig,
    metas: &BTreeMap<String, AltContractSymbolMeta>,
    enabled: bool,
) -> AltContractSymbolUniverseSummary {
    let mut tier_counts = BTreeMap::<String, usize>::new();
    if enabled {
        for meta in metas.values() {
            let tier = match meta.tier {
                AltContractSymbolTier::A => "A",
                AltContractSymbolTier::B => "B",
                AltContractSymbolTier::C => "C",
                AltContractSymbolTier::D => "D",
                AltContractSymbolTier::E => "E",
            };
            *tier_counts.entry(tier.to_string()).or_default() += 1;
        }
    }
    AltContractSymbolUniverseSummary {
        mode: if enabled {
            config.effective_universe_mode().as_str().to_string()
        } else {
            "disabled".to_string()
        },
        limit: config.symbol_universe.symbol_limit,
        monitored_count: if !enabled {
            0
        } else if metas.is_empty() {
            config.enabled_symbols().len()
        } else {
            metas.len()
        },
        tier_counts,
        whitelist: config.symbol_universe.whitelist.clone(),
        blacklist: config.symbol_universe.blacklist.clone(),
        excluded_symbols: config.symbol_universe.exclude_symbols.clone(),
        min_24h_quote_volume_usd: config.symbol_universe.min_24h_quote_volume_usd,
    }
}

fn sort_latest_signals(items: &mut [AltContractSignal]) {
    items.sort_by(|left, right| right.ts.cmp(&left.ts).then_with(|| right.id.cmp(&left.id)));
}

fn outcome_from_signal(signal: &AltContractSignal) -> AltContractSignalOutcome {
    AltContractSignalOutcome {
        signal_id: signal.id.clone(),
        product_id: signal.product_id.clone(),
        tier: signal.tier,
        signal_ts: signal.ts,
        window_sec: signal.window_sec,
        signal_type: format!("{:?}", signal.signal_type),
        anomaly_severity: signal.assessment.anomaly_severity,
        structure_confidence: signal.assessment.structure_confidence,
        exposure_tier: signal.assessment.exposure_tier,
        ais_score: signal.alt_impact_score.final_score,
        abnormal_score: signal.abnormal_score,
        build_score: signal.build_score,
        regime: signal.market_regime.regime.clone(),
        oi_context: oi_context_key(signal).to_string(),
        liquidation_context: if signal.liquidation_suspected {
            "liquidation_driven".to_string()
        } else {
            "not_liquidation_driven".to_string()
        },
        entry_price: signal.trigger_price_usd.filter(|price| *price > 0.0),
        outcome_version: "v1_read_only".to_string(),
        ..AltContractSignalOutcome::default()
    }
}

fn update_outcome_with_price(
    outcome: &mut AltContractSignalOutcome,
    direction: super::types::AltContractDirection,
    price: f64,
    ts: i64,
) {
    let Some(entry_price) = outcome.entry_price.filter(|entry| *entry > 0.0) else {
        return;
    };
    if !price.is_finite() || price <= 0.0 || ts < outcome.signal_ts {
        return;
    }
    let elapsed = ts.saturating_sub(outcome.signal_ts);
    let raw_bps = (price / entry_price - 1.0) * 10_000.0;
    let signed_bps = match direction {
        super::types::AltContractDirection::Sell => -raw_bps,
        super::types::AltContractDirection::Neutral => 0.0,
        _ => raw_bps,
    };
    if elapsed <= 60 * 60_000 {
        outcome.mfe_1h_bps = Some(outcome.mfe_1h_bps.unwrap_or(signed_bps).max(signed_bps));
        outcome.mae_1h_bps = Some(outcome.mae_1h_bps.unwrap_or(signed_bps).min(signed_bps));
    }
    update_outcome_checkpoint(
        &mut outcome.markout_5m_bps,
        &mut outcome.follow_through_5m,
        &mut outcome.evaluated_5m_at,
        elapsed,
        5 * 60_000,
        signed_bps,
        ts,
    );
    update_outcome_checkpoint(
        &mut outcome.markout_15m_bps,
        &mut outcome.follow_through_15m,
        &mut outcome.evaluated_15m_at,
        elapsed,
        15 * 60_000,
        signed_bps,
        ts,
    );
    update_outcome_checkpoint(
        &mut outcome.markout_1h_bps,
        &mut outcome.follow_through_1h,
        &mut outcome.evaluated_1h_at,
        elapsed,
        60 * 60_000,
        signed_bps,
        ts,
    );
}

fn update_outcome_checkpoint(
    markout: &mut Option<f64>,
    follow_through: &mut Option<bool>,
    evaluated_at: &mut Option<i64>,
    elapsed: i64,
    checkpoint_ms: i64,
    signed_bps: f64,
    ts: i64,
) {
    if elapsed >= checkpoint_ms && markout.is_none() {
        *markout = Some(signed_bps);
        *follow_through = Some(signed_bps > 0.0);
        *evaluated_at = Some(ts);
    }
}

fn median_optional(values: impl Iterator<Item = f64>) -> Option<f64> {
    let mut values = values.filter(|value| value.is_finite()).collect::<Vec<_>>();
    if values.is_empty() {
        return None;
    }
    values.sort_by(|left, right| left.total_cmp(right));
    let middle = values.len() / 2;
    Some(if values.len().is_multiple_of(2) {
        (values[middle - 1] + values[middle]) / 2.0
    } else {
        values[middle]
    })
}

fn outcome_matches_filter(
    outcome: &AltContractSignalOutcome,
    filter: &AltContractOutcomeFilter,
) -> bool {
    if filter
        .symbol
        .as_deref()
        .is_some_and(|symbol| outcome.product_id != product_id_for_symbol(symbol))
    {
        return false;
    }
    if filter.tier.as_deref().is_some_and(|tier| {
        compact_filter_value(&format!("{:?}", outcome.tier)) != compact_filter_value(tier)
    }) {
        return false;
    }
    if filter
        .window_sec
        .is_some_and(|window_sec| outcome.window_sec != window_sec)
    {
        return false;
    }
    if filter.signal_type.as_deref().is_some_and(|signal_type| {
        compact_filter_value(&outcome.signal_type) != compact_filter_value(signal_type)
    }) {
        return false;
    }
    if filter.severity.as_deref().is_some_and(|severity| {
        compact_filter_value(&format!("{:?}", outcome.anomaly_severity))
            != compact_filter_value(severity)
    }) {
        return false;
    }
    if filter
        .ais_min
        .is_some_and(|ais_min| outcome.ais_score < ais_min)
        || filter
            .ais_max
            .is_some_and(|ais_max| outcome.ais_score > ais_max)
    {
        return false;
    }
    if filter
        .regime
        .as_deref()
        .is_some_and(|regime| !outcome.regime.eq_ignore_ascii_case(regime))
    {
        return false;
    }
    if filter
        .oi_context
        .as_deref()
        .is_some_and(|oi_context| !outcome.oi_context.eq_ignore_ascii_case(oi_context))
    {
        return false;
    }
    if filter.time_of_day_utc.is_some_and(|expected_hour| {
        let actual_hour = ((outcome.signal_ts.div_euclid(3_600_000) % 24) + 24) % 24;
        actual_hour as u8 != expected_hour
    }) {
        return false;
    }
    true
}

fn sort_ranked_signals(items: &mut [AltContractSignal]) {
    items.sort_by(|left, right| {
        composite_signal_score(right)
            .total_cmp(&composite_signal_score(left))
            .then_with(|| right.ts.cmp(&left.ts))
    });
}

fn sort_top_impact_signals(items: &mut [AltContractSignal]) {
    items.sort_by(|left, right| {
        right
            .alt_impact_score
            .final_score
            .total_cmp(&left.alt_impact_score.final_score)
            .then_with(|| right.ts.cmp(&left.ts))
    });
}

fn composite_signal_score(signal: &AltContractSignal) -> f64 {
    f64::from(signal.abnormal_score) * 0.45
        + f64::from(signal.build_score) * 0.35
        + signal.alt_impact_score.final_score * 0.20
}

fn display_signal(
    signal: &AltContractSignal,
    config: &super::config::BinanceAltContractRuntimeConfig,
) -> bool {
    if !is_legacy_impact_score(&signal.alt_impact_score) {
        let absolute_floor = signal
            .display_threshold_usd
            .max(config.display_threshold_for_product(&signal.product_id));
        return impact_displayable(&signal.alt_impact_score)
            && (!config.impact.absolute_floor_required
                || signal.total_notional_usd >= absolute_floor);
    }
    let threshold =
        if signal.display_threshold_usd.is_finite() && signal.display_threshold_usd > 0.0 {
            signal.display_threshold_usd
        } else {
            config.display_threshold_for_product(&signal.product_id)
        };
    signal.total_notional_usd >= threshold
}

fn redact_error(error: String) -> String {
    error.replace(
        "https://discord.com/api/webhooks/",
        "https://discord.com/api/webhooks/[redacted]/",
    )
}

#[cfg(test)]
mod tests {
    use crate::binance_alt_contract_monitor::scheduler::{
        AltCandidatePriority, FairSchedulerConfig, FairScoringScheduler,
    };
    use crate::binance_alt_contract_monitor::types::AltContractSymbolTier;

    #[test]
    fn scoring_budget_limits_global_full_scores_per_window() {
        let mut scheduler = FairScoringScheduler::default();
        let config = FairSchedulerConfig {
            full_scores_per_second: 1,
            max_scores_per_symbol_per_second: 1,
            ..FairSchedulerConfig::default()
        };

        scheduler.upsert(test_candidate("BTCUSDT", 1_000));
        assert_eq!(scheduler.select(1_000, &config).len(), 1);
        scheduler.upsert(test_candidate("ETHUSDT", 1_100));
        assert!(scheduler.select(1_100, &config).is_empty());
        assert_eq!(scheduler.select(2_100, &config).len(), 1);
    }

    #[test]
    fn scoring_budget_limits_force_scan_bursts() {
        let mut scheduler = FairScoringScheduler::default();
        let config = FairSchedulerConfig {
            full_scores_per_second: 2,
            max_scores_per_symbol_per_second: 1,
            ..FairSchedulerConfig::default()
        };

        scheduler.upsert(test_candidate("BTCUSDT", 1_000));
        scheduler.upsert(test_candidate("ETHUSDT", 1_000));
        let selected = scheduler.select(1_000, &config);
        assert_eq!(selected.len(), 2);
        assert!(scheduler.select(1_100, &config).is_empty());
    }

    fn test_candidate(product_id: &str, candidate_created_at: i64) -> AltCandidatePriority {
        AltCandidatePriority {
            product_id: product_id.to_string(),
            tier: AltContractSymbolTier::C,
            window_sec: 60,
            relative_notional: 1.0,
            dynamic_multiple: 1.0,
            dominance: 0.8,
            abs_price_move_pct: 0.5,
            liquidation_present: false,
            candidate_created_at,
            last_scored_at: None,
        }
    }
}
