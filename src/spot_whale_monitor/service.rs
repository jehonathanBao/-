use std::{
    collections::{BTreeMap, BTreeSet, VecDeque},
    sync::Arc,
};

use parking_lot::RwLock;
use tokio::task::JoinHandle;

use crate::{
    normalizers::trade::now_ms,
    storage::{
        spot_whale_repo::{SpotWhaleRepo, SpotWhaleSignalQuery},
        SqliteStore,
    },
};

use super::{
    collector_binance, collector_bitfinex, collector_coinbase,
    config::{spot_whale_runtime_config, SpotWhaleRuntimeConfig},
    detector::detect_spot_whale_signal_with_config,
    discord_notifier::{notify_spot_whale_discord, SpotWhaleDiscordSettings},
    types::{
        SpotExchange, SpotExchangeContribution, SpotExchangeStatus, SpotTrade, SpotTradeSide,
        SpotWhaleLatestResponse, SpotWhaleSeverity, SpotWhaleSignal, SpotWhaleSummary,
        SpotWhaleTrend60s, SpotWhaleWindowStats,
    },
    LOG_PREFIX, LOG_TARGET,
};

const WINDOWS_SEC: [u64; 3] = [5, 15, 60];
const MAX_TRADES: usize = 120_000;
const MAX_SIGNALS: usize = 500;
const TRADE_RETENTION_MS: i64 = 3_600_000;

#[derive(Clone)]
pub struct SpotWhaleService {
    enabled: bool,
    dry_run: bool,
    booted_at_ms: i64,
    store: Option<SqliteStore>,
    state: Arc<RwLock<SpotWhaleState>>,
    tasks: Arc<RwLock<Vec<JoinHandle<()>>>>,
}

#[derive(Debug)]
struct SpotWhaleState {
    trades: VecDeque<SpotTrade>,
    signals: VecDeque<SpotWhaleSignal>,
    seen_signal_ids: BTreeSet<String>,
    exchanges: BTreeMap<String, SpotExchangeStatus>,
    last_detector_scan_at: BTreeMap<String, i64>,
    last_discord_sent_at: Option<i64>,
}

impl SpotWhaleService {
    pub fn new(
        enabled: bool,
        dry_run: bool,
        booted_at_ms: i64,
        store: Option<SqliteStore>,
    ) -> Self {
        let runtime_config = spot_whale_runtime_config();
        let mut exchanges = BTreeMap::new();
        exchanges.insert(
            "binance".to_string(),
            if runtime_config.exchanges.binance_enabled && enabled {
                SpotExchangeStatus::disconnected()
            } else {
                SpotExchangeStatus::disabled()
            },
        );
        exchanges.insert(
            "coinbase".to_string(),
            if runtime_config.exchanges.coinbase_enabled && enabled {
                SpotExchangeStatus::disconnected()
            } else {
                SpotExchangeStatus::disabled()
            },
        );
        exchanges.insert(
            "bitfinex".to_string(),
            if runtime_config.exchanges.bitfinex_enabled && enabled {
                SpotExchangeStatus::disconnected()
            } else {
                SpotExchangeStatus::disabled()
            },
        );
        let restored = load_persisted_signals(store.as_ref(), MAX_SIGNALS);
        Self {
            enabled,
            dry_run,
            booted_at_ms,
            store,
            state: Arc::new(RwLock::new(SpotWhaleState {
                trades: VecDeque::new(),
                signals: restored.signals,
                seen_signal_ids: restored.seen_signal_ids,
                exchanges,
                last_detector_scan_at: BTreeMap::new(),
                last_discord_sent_at: restored.last_discord_sent_at,
            })),
            tasks: Arc::new(RwLock::new(Vec::new())),
        }
    }

    pub fn start(&self) {
        if !self.enabled || self.tasks.read().iter().any(|task| !task.is_finished()) {
            return;
        }
        let config = spot_whale_runtime_config();
        tracing::info!(
            target: LOG_TARGET,
            enabled = self.enabled,
            dry_run = self.dry_run,
            "{} runtime started",
            LOG_PREFIX
        );
        if config.exchanges.binance_enabled {
            let service = self.clone();
            self.tasks.write().push(tokio::spawn(async move {
                collector_binance::run(service).await;
            }));
        }
        if config.exchanges.coinbase_enabled {
            let service = self.clone();
            self.tasks.write().push(tokio::spawn(async move {
                collector_coinbase::run(service).await;
            }));
        }
        if config.exchanges.bitfinex_enabled {
            let service = self.clone();
            self.tasks.write().push(tokio::spawn(async move {
                collector_bitfinex::run(service).await;
            }));
        }
    }

    pub fn stop(&self) {
        let tasks = std::mem::take(&mut *self.tasks.write());
        for task in tasks {
            task.abort();
        }
        self.set_exchange_status(SpotExchange::Binance, "disconnected", false, None);
        self.set_exchange_status(SpotExchange::Coinbase, "disconnected", false, None);
        self.set_exchange_status(SpotExchange::Bitfinex, "disconnected", false, None);
    }

    pub fn ingest_live_trade(&self, trade: SpotTrade) {
        let signals = self.ingest_trade(trade);
        for signal in signals {
            self.spawn_discord_notification(signal);
        }
    }

    pub fn ingest_trade(&self, trade: SpotTrade) -> Vec<SpotWhaleSignal> {
        if !self.enabled || !spot_whale_runtime_config().symbol_enabled(&trade.symbol) {
            return Vec::new();
        }
        self.mark_trade(trade.exchange, trade.ts);
        {
            let mut state = self.state.write();
            state.trades.push_back(trade.clone());
            prune_trades(&mut state.trades, trade.ts);
        }
        let config = spot_whale_runtime_config();
        if !self.should_run_detector(&trade.symbol, trade.ts, config.performance.scan_interval_ms) {
            return Vec::new();
        }
        let mut candidates = WINDOWS_SEC
            .iter()
            .filter_map(|window_sec| {
                self.window_stats(&trade.symbol, *window_sec, trade.ts, &config)
            })
            .filter_map(|stats| detect_spot_whale_signal_with_config(&stats, &config))
            .collect::<Vec<_>>();
        candidates.sort_by(|left, right| {
            right
                .severity
                .rank()
                .cmp(&left.severity.rank())
                .then_with(|| right.score.cmp(&left.score))
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

    pub fn mark_connected(&self, exchange: SpotExchange) {
        self.set_exchange_status(exchange, "connected", true, None);
    }

    pub fn exchange_trade_stale(&self, exchange: SpotExchange, stale_ms: i64) -> bool {
        let stale_ms = stale_ms.max(1);
        self.state
            .read()
            .exchanges
            .get(exchange.as_key())
            .and_then(|status| status.last_trade_at)
            .is_some_and(|last_trade_at| now_ms().saturating_sub(last_trade_at) > stale_ms)
    }

    pub fn mark_reconnecting(&self, exchange: SpotExchange, error: Option<String>) {
        let mut state = self.state.write();
        let entry = state
            .exchanges
            .entry(exchange.as_key().to_string())
            .or_insert_with(SpotExchangeStatus::disconnected);
        entry.connected = false;
        entry.status = "reconnecting".to_string();
        entry.reconnect_count = entry.reconnect_count.saturating_add(1);
        entry.last_error = error.map(redact_error);
    }

    pub fn set_exchange_status(
        &self,
        exchange: SpotExchange,
        status: &str,
        connected: bool,
        error: Option<String>,
    ) {
        let mut state = self.state.write();
        let entry = state
            .exchanges
            .entry(exchange.as_key().to_string())
            .or_insert_with(SpotExchangeStatus::disconnected);
        entry.status = status.to_string();
        entry.connected = connected;
        entry.last_error = error.map(redact_error);
    }

    pub fn summary(&self, symbol: &str) -> SpotWhaleSummary {
        let symbol = normalize_symbol(symbol);
        let state = self.state.read();
        let now = now_ms();
        let runtime_config = spot_whale_runtime_config();
        let summary_enabled = self.enabled && runtime_config.symbol_enabled(&symbol);
        let latest = state
            .signals
            .iter()
            .rev()
            .find(|signal| signal.symbol == symbol);
        let trend60s = trend_for_symbol(&state.trades, &symbol, now);
        let exchanges = summarized_exchange_statuses(
            summary_enabled,
            &state.exchanges,
            now,
            runtime_config.data_quality.heartbeat_stale_ms,
        );
        let health_status = health_status(summary_enabled, &exchanges);
        SpotWhaleSummary {
            status: latest
                .map(|signal| status_from_severity(signal.severity).to_string())
                .unwrap_or_else(|| "calm".to_string()),
            health_status: health_status.clone(),
            health_reason: health_reason(summary_enabled, &health_status).to_string(),
            direction: latest
                .map(|signal| format!("{:?}", signal.direction).to_ascii_lowercase())
                .unwrap_or_else(|| "neutral".to_string()),
            latest_direction: latest
                .map(|signal| format!("{:?}", signal.direction).to_ascii_lowercase())
                .unwrap_or_else(|| "neutral".to_string()),
            latest_severity: latest
                .map(|signal| signal.severity)
                .unwrap_or(SpotWhaleSeverity::Calm),
            latest_signal_at: latest.map(|signal| signal.ts),
            last_discord_sent_at: state.last_discord_sent_at,
            updated_at_ms: Some(now_ms()),
            signal_count: state
                .signals
                .iter()
                .filter(|signal| signal.symbol == symbol)
                .count()
                .max(self.persisted_signal_count(&symbol)),
            read_only: true,
            enabled: summary_enabled,
            dry_run: self.dry_run,
            symbol,
            trend60s,
            exchanges,
        }
    }

    pub fn latest(&self, symbol: &str, limit: usize) -> SpotWhaleLatestResponse {
        let symbol = normalize_symbol(symbol);
        let limit = limit.clamp(1, 200);
        let items = self
            .query_persisted_signals(SpotWhaleSignalQuery {
                symbol: Some(symbol.clone()),
                limit,
                ..SpotWhaleSignalQuery::default()
            })
            .unwrap_or_else(|| {
                self.state
                    .read()
                    .signals
                    .iter()
                    .rev()
                    .filter(|signal| signal.symbol == symbol)
                    .take(limit)
                    .cloned()
                    .collect::<Vec<_>>()
            });
        SpotWhaleLatestResponse {
            summary: self.summary(&symbol),
            items,
            limit,
        }
    }

    pub fn history(&self, query: SpotWhaleQuery) -> SpotWhaleLatestResponse {
        let symbol = query.symbol.unwrap_or_else(|| "BTC".to_string());
        let symbol = normalize_symbol(&symbol);
        let limit = query.limit.unwrap_or(50).clamp(1, 200);
        let items = self
            .query_persisted_signals(SpotWhaleSignalQuery {
                symbol: Some(symbol.clone()),
                severity: query.severity.clone(),
                signal_type: query.signal_type.clone(),
                discord_sent: query.discord_sent,
                min_abs_net_volume_base: query.min_abs_net_volume_base,
                limit,
                ..SpotWhaleSignalQuery::default()
            })
            .unwrap_or_else(|| {
                self.state
                    .read()
                    .signals
                    .iter()
                    .rev()
                    .filter(|signal| signal.symbol == symbol)
                    .filter(|signal| {
                        query
                            .severity
                            .as_deref()
                            .map(|value| {
                                value.eq_ignore_ascii_case(&format!("{:?}", signal.severity))
                            })
                            .unwrap_or(true)
                    })
                    .filter(|signal| {
                        query
                            .signal_type
                            .as_deref()
                            .map(|value| {
                                compact_filter_value(value)
                                    == compact_filter_value(&format!("{:?}", signal.signal_type))
                            })
                            .unwrap_or(true)
                    })
                    .filter(|signal| {
                        query
                            .discord_sent
                            .map(|value| signal.discord_sent == value)
                            .unwrap_or(true)
                    })
                    .filter(|signal| {
                        query
                            .min_abs_net_volume_base
                            .map(|threshold| signal.net_volume_base.abs() >= threshold)
                            .unwrap_or(true)
                    })
                    .take(limit)
                    .cloned()
                    .collect::<Vec<_>>()
            });
        SpotWhaleLatestResponse {
            summary: self.summary(&symbol),
            items,
            limit,
        }
    }

    fn window_stats(
        &self,
        symbol: &str,
        window_sec: u64,
        now: i64,
        config: &SpotWhaleRuntimeConfig,
    ) -> Option<SpotWhaleWindowStats> {
        let state = self.state.read();
        let window_ms = i64::try_from(window_sec).ok()?.saturating_mul(1000);
        let start = now.saturating_sub(window_ms);
        let mut window_trades = state
            .trades
            .iter()
            .filter(|trade| trade.symbol == symbol && trade.ts >= start && trade.ts <= now)
            .cloned()
            .collect::<Vec<_>>();
        if window_trades.is_empty() {
            return None;
        }
        window_trades.sort_by_key(|trade| trade.ts);
        let exchanges = exchange_contributions(&window_trades);
        let buy_volume_base = exchanges
            .iter()
            .map(|item| item.buy_volume_base)
            .sum::<f64>();
        let sell_volume_base = exchanges
            .iter()
            .map(|item| item.sell_volume_base)
            .sum::<f64>();
        let total_volume_base = buy_volume_base + sell_volume_base;
        let net_volume_base = buy_volume_base - sell_volume_base;
        let dominance = if total_volume_base > 0.0 {
            net_volume_base.abs() / total_volume_base
        } else {
            0.0
        };
        let total_notional_usd = exchanges
            .iter()
            .map(|item| item.total_notional_usd)
            .sum::<f64>();
        let first_price = window_trades.first()?.price;
        let last_price = window_trades.last()?.price;
        let price_move_pct = if first_price > 0.0 {
            Some((last_price / first_price - 1.0) * 100.0)
        } else {
            None
        };
        let main_exchange = exchanges
            .iter()
            .max_by(|left, right| left.total_volume_base.total_cmp(&right.total_volume_base))
            .map(|item| item.exchange.clone());
        let dynamic_multiple =
            dynamic_multiple(&state.trades, symbol, window_sec, now, total_volume_base);
        let multi_exchange_confirmed =
            same_direction_exchange_count(&exchanges, net_volume_base) >= 2;
        let mut data_quality = 100_u8;
        if exchanges.len() <= 1 {
            data_quality = data_quality.saturating_sub(config.data_quality.single_exchange_penalty);
        }
        let startup_age_ms = Some(now.saturating_sub(self.booted_at_ms));
        if startup_age_ms.is_some_and(|age| age < config.data_quality.warmup_ms) {
            data_quality = data_quality.saturating_sub(20);
        }
        let coinbase_premium_pct = exchange_latest_price(&window_trades, "coinbase")
            .zip(exchange_latest_price(&window_trades, "binance"))
            .filter(|(_, binance)| *binance > 0.0)
            .map(|(coinbase, binance)| (coinbase / binance - 1.0) * 100.0);
        Some(SpotWhaleWindowStats {
            symbol: symbol.to_string(),
            window_sec,
            ts: now,
            buy_volume_base,
            sell_volume_base,
            total_volume_base,
            net_volume_base,
            total_notional_usd,
            dominance,
            price_move_pct,
            coinbase_premium_pct,
            exchange_count: exchanges.len(),
            main_exchange,
            exchanges,
            dynamic_multiple,
            multi_exchange_confirmed,
            data_quality,
            startup_age_ms,
        })
    }

    fn insert_signal(&self, signal: SpotWhaleSignal) -> bool {
        let duplicate_window_ms = spot_whale_runtime_config().performance.duplicate_window_ms;
        let mut state = self.state.write();
        if state.seen_signal_ids.contains(&signal.id)
            || duplicate_recent(&state.signals, &signal, duplicate_window_ms)
        {
            return false;
        }
        state.seen_signal_ids.insert(signal.id.clone());
        state.signals.push_back(signal.clone());
        while state.signals.len() > MAX_SIGNALS {
            if let Some(old) = state.signals.pop_front() {
                state.seen_signal_ids.remove(&old.id);
            }
        }
        drop(state);
        self.persist_signal(&signal);
        true
    }

    fn should_run_detector(&self, symbol: &str, now: i64, scan_interval_ms: i64) -> bool {
        let scan_interval_ms = scan_interval_ms.max(0);
        let mut state = self.state.write();
        let last_scan_at = state
            .last_detector_scan_at
            .get(symbol)
            .copied()
            .unwrap_or(i64::MIN);
        if now.saturating_sub(last_scan_at) < scan_interval_ms {
            return false;
        }
        state.last_detector_scan_at.insert(symbol.to_string(), now);
        true
    }

    fn mark_trade(&self, exchange: SpotExchange, ts: i64) {
        let mut state = self.state.write();
        let entry = state
            .exchanges
            .entry(exchange.as_key().to_string())
            .or_insert_with(SpotExchangeStatus::disconnected);
        let now = now_ms();
        entry.connected = true;
        entry.status = "connected".to_string();
        entry.last_trade_at = Some(ts);
        entry.latency_ms = Some(now.saturating_sub(ts).max(0));
        entry.last_error = None;
    }

    fn spawn_discord_notification(&self, signal: SpotWhaleSignal) {
        let service = self.clone();
        let settings = SpotWhaleDiscordSettings::from_env(self.dry_run);
        tokio::spawn(async move {
            let outcome = notify_spot_whale_discord(&settings, &signal).await;
            service.update_discord_outcome(
                &signal.id,
                outcome.sent,
                outcome.sent_at_ms,
                outcome.reason,
            );
        });
    }

    fn update_discord_outcome(
        &self,
        signal_id: &str,
        sent: bool,
        sent_at_ms: Option<i64>,
        reason: String,
    ) {
        let mut state = self.state.write();
        if let Some(signal) = state
            .signals
            .iter_mut()
            .find(|signal| signal.id == signal_id)
        {
            signal.discord_sent = sent;
            signal.discord_sent_at = sent_at_ms;
            signal.discord_reason = reason.clone();
        }
        if sent {
            state.last_discord_sent_at = sent_at_ms;
        }
        drop(state);
        self.persist_discord_outcome(signal_id, sent, sent_at_ms, &reason);
    }

    fn persist_signal(&self, signal: &SpotWhaleSignal) {
        if let Some(store) = &self.store {
            if let Err(err) = store.upsert_spot_whale_signal(signal) {
                tracing::warn!(
                    target: LOG_TARGET,
                    signal_id = signal.id.as_str(),
                    "{} failed to persist signal: {err}",
                    LOG_PREFIX
                );
            }
        }
    }

    fn persist_discord_outcome(
        &self,
        signal_id: &str,
        sent: bool,
        sent_at_ms: Option<i64>,
        reason: &str,
    ) {
        if let Some(store) = &self.store {
            if let Err(err) =
                store.update_spot_whale_discord_status(signal_id, sent, sent_at_ms, reason)
            {
                tracing::warn!(
                    target: LOG_TARGET,
                    signal_id,
                    "{} failed to persist discord outcome: {err}",
                    LOG_PREFIX
                );
            }
        }
    }

    fn query_persisted_signals(&self, query: SpotWhaleSignalQuery) -> Option<Vec<SpotWhaleSignal>> {
        let store = self.store.as_ref()?;
        match store.query_spot_whale_signals(&query) {
            Ok(signals) => Some(signals),
            Err(err) => {
                tracing::warn!(
                    target: LOG_TARGET,
                    "{} failed to load persisted signals: {err}",
                    LOG_PREFIX
                );
                None
            }
        }
    }

    fn persisted_signal_count(&self, symbol: &str) -> usize {
        self.store
            .as_ref()
            .and_then(|store| store.count_spot_whale_signals(symbol).ok())
            .unwrap_or(0)
    }
}

#[derive(Debug, Clone, Default)]
pub struct SpotWhaleQuery {
    pub symbol: Option<String>,
    pub severity: Option<String>,
    pub signal_type: Option<String>,
    pub discord_sent: Option<bool>,
    pub min_abs_net_volume_base: Option<f64>,
    pub limit: Option<usize>,
}

#[derive(Debug, Default)]
struct RestoredSpotWhaleSignals {
    signals: VecDeque<SpotWhaleSignal>,
    seen_signal_ids: BTreeSet<String>,
    last_discord_sent_at: Option<i64>,
}

fn load_persisted_signals(store: Option<&SqliteStore>, limit: usize) -> RestoredSpotWhaleSignals {
    let Some(store) = store else {
        return RestoredSpotWhaleSignals::default();
    };
    let mut signals = match store.query_spot_whale_signals(&SpotWhaleSignalQuery {
        limit,
        ..SpotWhaleSignalQuery::default()
    }) {
        Ok(signals) => signals,
        Err(err) => {
            tracing::warn!(
                target: LOG_TARGET,
                "{} failed to restore persisted signals: {err}",
                LOG_PREFIX
            );
            return RestoredSpotWhaleSignals::default();
        }
    };
    signals.reverse();
    let last_discord_sent_at = signals
        .iter()
        .filter_map(|signal| signal.discord_sent_at)
        .max();
    let seen_signal_ids = signals.iter().map(|signal| signal.id.clone()).collect();
    RestoredSpotWhaleSignals {
        signals: VecDeque::from(signals),
        seen_signal_ids,
        last_discord_sent_at,
    }
}

fn exchange_contributions(trades: &[SpotTrade]) -> Vec<SpotExchangeContribution> {
    let mut by_exchange: BTreeMap<String, SpotExchangeContribution> = BTreeMap::new();
    for trade in trades {
        let entry = by_exchange
            .entry(trade.exchange.as_key().to_string())
            .or_insert_with(|| SpotExchangeContribution {
                exchange: trade.exchange.as_key().to_string(),
                ..SpotExchangeContribution::default()
            });
        match trade.side {
            SpotTradeSide::Buy => {
                entry.buy_volume_base += trade.qty_base;
                entry.buy_notional_usd += trade.notional_usd;
            }
            SpotTradeSide::Sell => {
                entry.sell_volume_base += trade.qty_base;
                entry.sell_notional_usd += trade.notional_usd;
            }
        }
        entry.trade_count = entry.trade_count.saturating_add(1);
    }
    by_exchange
        .into_values()
        .map(|mut item| {
            item.total_volume_base = item.buy_volume_base + item.sell_volume_base;
            item.total_notional_usd = item.buy_notional_usd + item.sell_notional_usd;
            item.net_volume_base = item.buy_volume_base - item.sell_volume_base;
            item.dominance = if item.total_volume_base > 0.0 {
                item.net_volume_base.abs() / item.total_volume_base
            } else {
                0.0
            };
            item
        })
        .collect()
}

fn same_direction_exchange_count(
    exchanges: &[SpotExchangeContribution],
    net_volume_base: f64,
) -> usize {
    let positive = net_volume_base > 0.0;
    exchanges
        .iter()
        .filter(|item| item.total_volume_base > 0.0 && item.dominance >= 0.55)
        .filter(|item| (item.net_volume_base > 0.0) == positive)
        .count()
}

fn trend_for_symbol(trades: &VecDeque<SpotTrade>, symbol: &str, now: i64) -> SpotWhaleTrend60s {
    let start = now.saturating_sub(60_000);
    let mut trend = SpotWhaleTrend60s::default();
    for trade in trades
        .iter()
        .filter(|trade| trade.symbol == symbol && trade.ts >= start && trade.ts <= now)
    {
        match trade.side {
            SpotTradeSide::Buy => trend.buy_volume_base += trade.qty_base,
            SpotTradeSide::Sell => trend.sell_volume_base += trade.qty_base,
        }
        trend.updated_at_ms = Some(trade.ts);
    }
    trend.total_volume_base = trend.buy_volume_base + trend.sell_volume_base;
    trend.net_volume_base = trend.buy_volume_base - trend.sell_volume_base;
    if trend.total_volume_base > 0.0 {
        trend.dominance = trend.net_volume_base.abs() / trend.total_volume_base;
        trend.buy_ratio = trend.buy_volume_base / trend.total_volume_base;
        trend.sell_ratio = trend.sell_volume_base / trend.total_volume_base;
    }
    trend
}

fn dynamic_multiple(
    trades: &VecDeque<SpotTrade>,
    symbol: &str,
    window_sec: u64,
    now: i64,
    current_volume: f64,
) -> Option<f64> {
    let window_ms = i64::try_from(window_sec).ok()?.saturating_mul(1000);
    let lookback_start = now.saturating_sub(3_600_000);
    let current_start = now.saturating_sub(window_ms);
    let mut buckets: BTreeMap<i64, f64> = BTreeMap::new();
    for trade in trades.iter().filter(|trade| {
        trade.symbol == symbol && trade.ts >= lookback_start && trade.ts < current_start
    }) {
        let bucket = (trade.ts - lookback_start) / window_ms;
        *buckets.entry(bucket).or_insert(0.0) += trade.qty_base;
    }
    if buckets.len() < 10 {
        return None;
    }
    let average = buckets.values().sum::<f64>() / buckets.len() as f64;
    (average > 0.0).then_some(current_volume / average)
}

fn exchange_latest_price(trades: &[SpotTrade], exchange: &str) -> Option<f64> {
    trades
        .iter()
        .rev()
        .find(|trade| trade.exchange.as_key() == exchange)
        .map(|trade| trade.price)
}

fn duplicate_recent(
    signals: &VecDeque<SpotWhaleSignal>,
    signal: &SpotWhaleSignal,
    duplicate_window_ms: i64,
) -> bool {
    let duplicate_window_ms = duplicate_window_ms.max(0);
    signals.iter().rev().take(20).any(|existing| {
        existing.symbol == signal.symbol
            && existing.signal_type == signal.signal_type
            && existing.direction == signal.direction
            && signal.ts.saturating_sub(existing.ts) <= duplicate_window_ms
            && existing.severity.rank() >= signal.severity.rank()
    })
}

fn prune_trades(trades: &mut VecDeque<SpotTrade>, now: i64) {
    while trades.len() > MAX_TRADES
        || trades
            .front()
            .is_some_and(|trade| now.saturating_sub(trade.ts) > TRADE_RETENTION_MS)
    {
        trades.pop_front();
    }
}

fn normalize_symbol(symbol: &str) -> String {
    symbol.trim().trim_end_matches("-SPOT").to_ascii_uppercase()
}

fn compact_filter_value(value: &str) -> String {
    value
        .chars()
        .filter(|ch| *ch != '_')
        .flat_map(|ch| ch.to_lowercase())
        .collect()
}

fn status_from_severity(severity: SpotWhaleSeverity) -> &'static str {
    match severity {
        SpotWhaleSeverity::S | SpotWhaleSeverity::Critical => "strong",
        SpotWhaleSeverity::High => "active",
        SpotWhaleSeverity::Medium => "watch",
        SpotWhaleSeverity::Calm => "calm",
    }
}

fn health_status(enabled: bool, exchanges: &BTreeMap<String, SpotExchangeStatus>) -> String {
    if !enabled {
        return "disabled".to_string();
    }
    let connected = exchanges.values().filter(|status| status.connected).count();
    match connected {
        2.. => "healthy".to_string(),
        1 => "degraded".to_string(),
        _ => "unhealthy".to_string(),
    }
}

fn health_reason(enabled: bool, health_status: &str) -> &'static str {
    if !enabled {
        "spot_whale_monitor_disabled"
    } else {
        match health_status {
            "healthy" => "multiple_spot_sources_recent",
            "degraded" => "single_spot_source_recent",
            "unhealthy" => "spot_sources_stale_or_disconnected",
            _ => "spot_whale_status_unknown",
        }
    }
}

fn summarized_exchange_statuses(
    enabled: bool,
    exchanges: &BTreeMap<String, SpotExchangeStatus>,
    now: i64,
    stale_ms: i64,
) -> BTreeMap<String, SpotExchangeStatus> {
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
    status: &SpotExchangeStatus,
    now: i64,
    stale_ms: i64,
) -> SpotExchangeStatus {
    let mut item = status.clone();
    if !enabled || item.status == "disabled" {
        return item;
    }
    let stale_ms = stale_ms.max(1);
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

fn redact_error(error: String) -> String {
    error.replace(
        "https://discord.com/api/webhooks/",
        "https://discord.com/api/webhooks/[redacted]/",
    )
}
