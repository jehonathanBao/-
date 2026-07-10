use std::{
    collections::{BTreeMap, BTreeSet, VecDeque},
    fs::{self, OpenOptions},
    io::Write,
    path::PathBuf,
    sync::Arc,
    time::Duration,
};

use parking_lot::{Mutex, RwLock};
use tokio::{task::JoinHandle, time::MissedTickBehavior};

use crate::normalizers::trade::now_ms;

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
    impact::{impact_displayable, is_legacy_impact_score},
    smaf::{audit_smart_money_system, SmafAuditInput},
    smll::audit_self_learning_loop,
    symbol_universe::{meta_from_product_id, tier_for_quote_volume},
    types::{
        AltContractAllMarketContextStatus, AltContractContext, AltContractDryRunStats,
        AltContractExchange, AltContractExchangeStatus, AltContractLatestResponse,
        AltContractSeverity, AltContractSignal, AltContractSummary, AltContractSymbolMeta,
        AltContractSymbolTier, AltContractSymbolUniverseSummary, AltContractTrade,
        AltContractTradeSide, AltContractWindowStats, BacmRuntimeDiagnostics,
    },
    LOG_PREFIX, LOG_TARGET,
};

const MAX_TRADES: usize = 200_000;
const MAX_SIGNALS: usize = 1_000;
const TRADE_RETENTION_MS: i64 = 3_600_000;
const DUPLICATE_WINDOW_MS: i64 = 10_000;
const OI_RETENTION_MS: i64 = 10 * 60_000;
const LIQUIDATION_CONTEXT_TTL_MS: i64 = 60_000;
const SUMMARY_MONITORED_SYMBOL_LIMIT: usize = 12;

#[derive(Clone)]
pub struct BinanceAltContractService {
    enabled: bool,
    dry_run: bool,
    booted_at_ms: i64,
    persistence_path: PathBuf,
    persistence_lock: Arc<Mutex<()>>,
    state: Arc<RwLock<BinanceAltContractState>>,
    tasks: Arc<RwLock<Vec<JoinHandle<()>>>>,
}

#[derive(Debug)]
struct BinanceAltContractState {
    trades: VecDeque<AltContractTrade>,
    signals: VecDeque<AltContractSignal>,
    seen_signal_ids: BTreeSet<String>,
    exchanges: BTreeMap<String, AltContractExchangeStatus>,
    contexts: BTreeMap<String, AltContractContext>,
    symbol_metas: BTreeMap<String, AltContractSymbolMeta>,
    active_symbol_last_trade_at: BTreeMap<String, i64>,
    oi_snapshots: BTreeMap<String, VecDeque<(i64, f64)>>,
    liquidation_seen_at: BTreeMap<String, i64>,
    candidate_seen_at: BTreeMap<String, i64>,
    hot_oi_seen_at: BTreeMap<String, i64>,
    last_detector_scan_at: BTreeMap<String, i64>,
    scoring_budget: ScoringBudgetState,
    light_candidates_total: u64,
    full_score_attempts_total: u64,
    full_score_skipped_budget_total: u64,
    shard_connected: BTreeMap<usize, bool>,
    total_shards: usize,
    universe_last_refreshed_at: Option<i64>,
    events: BTreeMap<String, AltContractEventState>,
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
struct AltContractEventState {
    id: String,
    updated_at: i64,
    peak_abnormal_score: u8,
    peak_build_score: u8,
    signal_count: u32,
}

#[derive(Debug, Clone, Default)]
struct ScoringBudgetState {
    window_start_ms: i64,
    full_scores: u64,
    burst_scores: u64,
}

impl ScoringBudgetState {
    fn allow(
        &mut self,
        ts: i64,
        force_scan: bool,
        max_full_scores: u64,
        max_burst_scores: u64,
        window_ms: i64,
    ) -> bool {
        let window_ms = window_ms.max(1);
        if ts.saturating_sub(self.window_start_ms) >= window_ms {
            *self = ScoringBudgetState {
                window_start_ms: ts,
                full_scores: 0,
                burst_scores: 0,
            };
        }
        if self.full_scores >= max_full_scores.max(1) {
            return false;
        }
        if force_scan {
            if self.burst_scores >= max_burst_scores.max(1) {
                return false;
            }
            self.burst_scores = self.burst_scores.saturating_add(1);
        }
        self.full_scores = self.full_scores.saturating_add(1);
        true
    }
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
    pub limit: Option<usize>,
}

impl BinanceAltContractService {
    pub fn new(enabled: bool, dry_run: bool, booted_at_ms: i64) -> Self {
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
            load_persisted_signals(
                &runtime_config.persistence_path,
                MAX_SIGNALS,
                now_ms(),
                retention_days_to_ms(runtime_config.storage.signals_retention_days),
            )
        } else {
            RestoredAltContractSignals::default()
        };
        Self {
            enabled,
            dry_run,
            booted_at_ms,
            persistence_path: runtime_config.persistence_path,
            persistence_lock: Arc::new(Mutex::new(())),
            state: Arc::new(RwLock::new(BinanceAltContractState {
                trades: VecDeque::new(),
                signals: restored.signals,
                seen_signal_ids: restored.seen_signal_ids,
                exchanges,
                contexts: BTreeMap::new(),
                symbol_metas: BTreeMap::new(),
                active_symbol_last_trade_at: BTreeMap::new(),
                oi_snapshots: BTreeMap::new(),
                liquidation_seen_at: BTreeMap::new(),
                candidate_seen_at: BTreeMap::new(),
                hot_oi_seen_at: BTreeMap::new(),
                last_detector_scan_at: BTreeMap::new(),
                scoring_budget: ScoringBudgetState::default(),
                light_candidates_total: 0,
                full_score_attempts_total: 0,
                full_score_skipped_budget_total: 0,
                shard_connected: BTreeMap::new(),
                total_shards: 0,
                universe_last_refreshed_at: None,
                events: BTreeMap::new(),
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
            state.trades.push_back(trade.clone());
            state
                .active_symbol_last_trade_at
                .insert(trade.product_id.clone(), trade.ts);
            prune_trades(&mut state.trades, trade.ts);
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
        let window_stats = config
            .windows_sec
            .iter()
            .filter_map(|window_sec| {
                let state = self.state.read();
                rolling_window_stats(
                    &state.trades,
                    &meta,
                    *window_sec,
                    trade.ts,
                    &context,
                    self.booted_at_ms,
                    &config,
                )
            })
            .collect::<Vec<_>>();
        if window_stats
            .iter()
            .any(|stats| light_scan_candidate(stats, &context, &config))
        {
            self.mark_candidate_product(&trade.product_id, trade.ts);
        }
        let window_confirmations = window_stats
            .iter()
            .map(|stats| window_confirmation_for(stats, &config))
            .collect::<Vec<_>>();
        let market_context = self.market_impulse_context(
            &trade.product_id,
            trade.ts,
            window_stats
                .iter()
                .find(|stats| stats.window_sec == 60)
                .or_else(|| window_stats.first()),
        );
        let mut candidates = window_stats
            .into_iter()
            .filter(|stats| light_scan_candidate(stats, &context, &config))
            .filter_map(|stats| {
                detect_alt_contract_signal_with_context(
                    &stats,
                    &context,
                    &config,
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
        if self.insert_signal(best.clone()) {
            vec![best]
        } else {
            Vec::new()
        }
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
        BacmRuntimeDiagnostics {
            universe_symbol_count,
            active_symbol_count: state.active_symbol_last_trade_at.len(),
            connected_shards: state
                .shard_connected
                .values()
                .filter(|connected| **connected)
                .count(),
            total_shards: state.total_shards,
            trade_buffer_total: state.trades.len(),
            per_symbol_state_count: 0,
            light_candidates_total: state.light_candidates_total,
            full_score_attempts_total: state.full_score_attempts_total,
            full_score_skipped_budget_total: state.full_score_skipped_budget_total,
            persistence_queue_depth: 0,
            oldest_persistence_age_ms: None,
            universe_last_refreshed_at: state.universe_last_refreshed_at,
            universe_refresh_age_sec,
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
        let change_1m = oi_change_for_window(snapshots, ts, 60_000, open_interest_base);
        let change_5m = oi_change_for_window(snapshots, ts, 5 * 60_000, open_interest_base);
        let base = change_5m.or(change_1m).and_then(|change| {
            snapshots
                .front()
                .map(|(_, value)| *value)
                .filter(|value| *value > 0.0)
                .map(|previous| (change / previous) * 100.0)
        });
        let context = state.contexts.entry(product_id).or_default();
        context.oi_change_1m_base = change_1m;
        context.oi_change_5m_base = change_5m;
        context.oi_change_pct = base;
        context.oi_updated_at = Some(ts);
        state.last_oi_poll_at = Some(ts);
    }

    pub fn update_funding_context(&self, product_id: &str, funding_rate: Option<f64>) {
        let Some(funding_rate) = funding_rate.filter(|value| value.is_finite()) else {
            return;
        };
        let product_id = product_id_for_symbol(product_id);
        let mut state = self.state.write();
        let context = state.contexts.entry(product_id).or_default();
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
        let mut state = self.state.write();
        let context = state.contexts.entry(product_id).or_default();
        if let Some(mark_price_usd) =
            mark_price_usd.filter(|value| value.is_finite() && *value > 0.0)
        {
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
        if !notional_usd.is_finite() || notional_usd <= 0.0 {
            return;
        }
        let product_id = product_id_for_symbol(product_id);
        let mut state = self.state.write();
        let context = state.contexts.entry(product_id.clone()).or_default();
        context.liquidation_notional_usd =
            Some(context.liquidation_notional_usd.unwrap_or_default() + notional_usd);
        context.liquidation_suspected = true;
        context.force_order_snapshot = true;
        state.liquidation_seen_at.insert(product_id, ts);
        state.last_force_order_at = Some(ts);
        state.force_order_stream_connected = true;
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
        let empty_trades = VecDeque::new();
        let signals = if enabled {
            &state.signals
        } else {
            &empty_signals
        };
        let trades = if enabled {
            &state.trades
        } else {
            &empty_trades
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
        let health_status = health_status(enabled, &exchanges);
        let dry_run_stats = dry_run_stats(signals, now);
        let last_trade_at = exchanges
            .values()
            .filter_map(|status| status.last_trade_at)
            .max();
        let top_active_symbols = top_active_symbols(trades, now);
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
        let smll_report = audit_self_learning_loop(now, signals);
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
                .trades
                .iter()
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
            trend60s: trend_for_symbol(trades, &trend_product, now),
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
        sort_signals(&mut items);
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
        sort_signals(&mut items);
        items.truncate(limit);
        AltContractLatestResponse {
            summary: self.summary(query.symbol.as_deref()),
            items,
            limit,
        }
    }

    fn insert_signal(&self, mut signal: AltContractSignal) -> bool {
        let mut state = self.state.write();
        if state.seen_signal_ids.contains(&signal.id) || duplicate_recent(&state.signals, &signal) {
            return false;
        }
        merge_event_state(&mut state, &mut signal);
        state.seen_signal_ids.insert(signal.id.clone());
        state.signals.push_back(signal.clone());
        prune_non_protected_signal_limit(&mut state.signals, MAX_SIGNALS);
        state.seen_signal_ids = state
            .signals
            .iter()
            .map(|signal| signal.id.clone())
            .collect();
        drop(state);
        self.persist_signal(&signal);
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
        state.full_score_attempts_total = state.full_score_attempts_total.saturating_add(1);
        let max_full_scores = config
            .detector
            .max_global_full_scoring_per_sec
            .max(config.detector.max_full_scores_per_sec)
            .max(1);
        let max_burst_scores = config.detector.max_burst_full_scoring.max(1);
        if !state.scoring_budget.allow(
            ts,
            force_scan,
            max_full_scores,
            max_burst_scores,
            config.detector.burst_window_ms,
        ) {
            state.full_score_skipped_budget_total =
                state.full_score_skipped_budget_total.saturating_add(1);
            return false;
        }
        state.last_detector_scan_at.insert(product_id, ts);
        true
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
        let monitored_count = if state.symbol_metas.is_empty() {
            config.enabled_symbols().len()
        } else {
            state.symbol_metas.len()
        }
        .max(1);
        let start = now.saturating_sub(3 * 60_000);
        let mut by_symbol = BTreeMap::<String, (f64, f64, f64)>::new();
        for trade in state
            .trades
            .iter()
            .filter(|trade| trade.ts >= start && trade.ts <= now)
        {
            let entry = by_symbol.entry(trade.product_id.clone()).or_default();
            match trade.side {
                AltContractTradeSide::Buy => entry.0 += trade.notional_usd,
                AltContractTradeSide::Sell => entry.1 += trade.notional_usd,
            }
            entry.2 += trade.notional_usd;
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
        let ratio = same_direction.len() as f64 / monitored_count as f64;
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
            context.liquidation_suspected = false;
            context.force_order_snapshot = false;
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
        let _guard = self.persistence_lock.lock();
        if let Some(parent) = self.persistence_path.parent() {
            if let Err(error) = fs::create_dir_all(parent) {
                tracing::warn!(
                    target: LOG_TARGET,
                    "{} failed to create persistence dir: {error}",
                    LOG_PREFIX
                );
                return;
            }
        }
        let payload = match serde_json::to_string(signal) {
            Ok(payload) => payload,
            Err(error) => {
                tracing::warn!(
                    target: LOG_TARGET,
                    signal_id = signal.id.as_str(),
                    "{} failed to serialize signal: {error}",
                    LOG_PREFIX
                );
                return;
            }
        };
        match OpenOptions::new()
            .create(true)
            .append(true)
            .open(&self.persistence_path)
        {
            Ok(mut file) => {
                if let Err(error) = writeln!(file, "{payload}") {
                    tracing::warn!(
                        target: LOG_TARGET,
                        signal_id = signal.id.as_str(),
                        "{} failed to write signal: {error}",
                        LOG_PREFIX
                    );
                }
            }
            Err(error) => {
                tracing::warn!(
                    target: LOG_TARGET,
                    "{} failed to open persistence file: {error}",
                    LOG_PREFIX
                );
            }
        }
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

        if let Err(error) = self.rewrite_persisted_signals(&retained_signals) {
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
    })
}

fn merge_event_state(state: &mut BinanceAltContractState, signal: &mut AltContractSignal) {
    let now = signal.ts;
    state
        .events
        .retain(|_, event| now.saturating_sub(event.updated_at) <= 15 * 60_000);
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
            updated_at: signal.ts,
            peak_abnormal_score: signal.abnormal_score,
            peak_build_score: signal.build_score,
            signal_count: 0,
        });
    event.updated_at = signal.ts;
    event.peak_abnormal_score = event.peak_abnormal_score.max(signal.abnormal_score);
    event.peak_build_score = event.peak_build_score.max(signal.build_score);
    event.signal_count = event.signal_count.saturating_add(1);
    signal.event_id = Some(event.id.clone());
    signal.event_signal_count = event.signal_count;
    signal.event_peak_abnormal_score = event.peak_abnormal_score;
    signal.event_peak_build_score = event.peak_build_score;
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

fn oi_change_for_window(
    snapshots: &VecDeque<(i64, f64)>,
    now: i64,
    window_ms: i64,
    current: f64,
) -> Option<f64> {
    snapshots
        .iter()
        .rev()
        .find(|(ts, _)| now.saturating_sub(*ts) >= window_ms)
        .map(|(_, previous)| current - *previous)
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

fn health_status(enabled: bool, exchanges: &BTreeMap<String, AltContractExchangeStatus>) -> String {
    if !enabled {
        return "disabled".to_string();
    }
    let connected = exchanges.values().filter(|status| status.connected).count();
    if connected >= 1 {
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

fn top_active_symbols(trades: &VecDeque<AltContractTrade>, now: i64) -> Vec<String> {
    let mut by_symbol = BTreeMap::<String, f64>::new();
    for trade in trades
        .iter()
        .filter(|trade| now.saturating_sub(trade.ts) <= 15 * 60_000)
    {
        *by_symbol.entry(trade.product_id.clone()).or_default() += trade.notional_usd;
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

fn sort_signals(items: &mut [AltContractSignal]) {
    items.sort_by(|left, right| {
        right
            .severity
            .rank()
            .cmp(&left.severity.rank())
            .then_with(|| right.build_score.cmp(&left.build_score))
            .then_with(|| right.abnormal_score.cmp(&left.abnormal_score))
            .then_with(|| right.ts.cmp(&left.ts))
    });
}

fn display_signal(
    signal: &AltContractSignal,
    config: &super::config::BinanceAltContractRuntimeConfig,
) -> bool {
    if !is_legacy_impact_score(&signal.alt_impact_score) {
        return impact_displayable(&signal.alt_impact_score);
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
    use super::ScoringBudgetState;

    #[test]
    fn scoring_budget_limits_global_full_scores_per_window() {
        let mut budget = ScoringBudgetState::default();

        assert!(budget.allow(1_000, false, 1, 3, 1_000));
        assert!(
            !budget.allow(1_100, false, 1, 3, 1_000),
            "second score in the same budget window should be deferred"
        );
        assert!(
            budget.allow(2_100, false, 1, 3, 1_000),
            "new budget window should admit scoring again"
        );
    }

    #[test]
    fn scoring_budget_limits_force_scan_bursts() {
        let mut budget = ScoringBudgetState::default();

        assert!(budget.allow(1_000, true, 10, 1, 1_000));
        assert!(
            !budget.allow(1_100, true, 10, 1, 1_000),
            "extra force scan in the same window should be deferred by burst budget"
        );
        assert!(
            budget.allow(1_200, false, 10, 1, 1_000),
            "normal non-burst scoring can still use remaining global budget"
        );
    }
}
