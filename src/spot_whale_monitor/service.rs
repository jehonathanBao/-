use std::{
    collections::{BTreeMap, BTreeSet, VecDeque},
    sync::Arc,
};

use base64::{engine::general_purpose::STANDARD as BASE64_STANDARD, Engine as _};
use parking_lot::RwLock;
use tokio::task::JoinHandle;

use crate::{
    normalizers::trade::{is_ingress_timestamp_acceptable, now_ms},
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
        is_permanent_spot_whale_signal, SpotExchange, SpotExchangeContribution, SpotExchangeStatus,
        SpotTrade, SpotTradeSide, SpotWhaleLatestResponse, SpotWhaleSeverity, SpotWhaleSignal,
        SpotWhaleSummary, SpotWhaleTrend60s, SpotWhaleWindowStats,
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
    seen_trade_ids: BTreeMap<String, i64>,
    signals: VecDeque<SpotWhaleSignal>,
    seen_signal_ids: BTreeSet<String>,
    exchanges: BTreeMap<String, SpotExchangeStatus>,
    symbol_exchanges: BTreeMap<String, BTreeMap<String, SpotExchangeStatus>>,
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
        let symbol_exchanges = runtime_config
            .enabled_symbols()
            .into_iter()
            .map(|symbol| (symbol, exchanges.clone()))
            .collect();
        Self {
            enabled,
            dry_run,
            booted_at_ms,
            store,
            state: Arc::new(RwLock::new(SpotWhaleState {
                trades: VecDeque::new(),
                seen_trade_ids: BTreeMap::new(),
                signals: restored.signals,
                seen_signal_ids: restored.seen_signal_ids,
                exchanges,
                symbol_exchanges,
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
        if self.store.is_some() {
            let service = self.clone();
            self.tasks.write().push(tokio::spawn(async move {
                let retention = spot_whale_runtime_config().retention;
                tokio::time::sleep(std::time::Duration::from_secs(
                    retention.initial_delay_seconds.max(1) as u64,
                ))
                .await;
                loop {
                    service.run_retention_once(now_ms());
                    tokio::time::sleep(std::time::Duration::from_secs(
                        retention.interval_seconds.max(1) as u64,
                    ))
                    .await;
                }
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
        if !is_ingress_timestamp_acceptable(trade.ts, now_ms()) {
            return Vec::new();
        }
        let trade_key = trade.trade_id.as_deref().map(|trade_id| {
            format!(
                "{}:{}:{}",
                trade.exchange.as_key(),
                trade.symbol.trim().to_ascii_uppercase(),
                trade_id
            )
        });
        {
            let mut state = self.state.write();
            if let Some(key) = trade_key.as_deref() {
                if state.seen_trade_ids.contains_key(key) {
                    return Vec::new();
                }
                state.seen_trade_ids.insert(key.to_string(), trade.ts);
            }
            let cutoff = now_ms().saturating_sub(TRADE_RETENTION_MS);
            state.seen_trade_ids.retain(|_, ts| *ts >= cutoff);
        }
        self.mark_trade(trade.exchange, &trade.symbol, trade.ts);
        {
            let mut state = self.state.write();
            state.trades.push_back(trade.clone());
            prune_trades(&mut state.trades, now_ms());
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
        let redacted = error.map(redact_error);
        let entry = state
            .exchanges
            .entry(exchange.as_key().to_string())
            .or_insert_with(SpotExchangeStatus::disconnected);
        entry.connected = false;
        entry.status = "reconnecting".to_string();
        entry.reconnect_count = entry.reconnect_count.saturating_add(1);
        entry.last_error = redacted.clone();
        for statuses in state.symbol_exchanges.values_mut() {
            let item = statuses
                .entry(exchange.as_key().to_string())
                .or_insert_with(SpotExchangeStatus::disconnected);
            item.connected = false;
            item.status = "reconnecting".to_string();
            item.reconnect_count = item.reconnect_count.saturating_add(1);
            item.last_error = redacted.clone();
        }
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
        let redacted = error.map(redact_error);
        entry.last_error = redacted.clone();
        for statuses in state.symbol_exchanges.values_mut() {
            let item = statuses
                .entry(exchange.as_key().to_string())
                .or_insert_with(SpotExchangeStatus::disconnected);
            item.last_error = redacted.clone();
            if !connected || matches!(status, "disconnected" | "reconnecting" | "degraded") {
                item.connected = false;
                item.status = status.to_string();
            } else if item.last_trade_at.is_none() {
                item.connected = false;
                item.status = "waiting_for_trade".to_string();
            }
        }
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
        let latest_age_sec = latest.map(|signal| now.saturating_sub(signal.ts).max(0) / 1_000);
        let latest_is_stale = latest_age_sec.is_some_and(|age| {
            age.saturating_mul(1_000) > runtime_config.data_quality.latest_signal_stale_ms
        });
        let current_latest = (!latest_is_stale).then_some(latest).flatten();
        let trend60s = trend_for_symbol(&state.trades, &symbol, now);
        let exchanges = summarized_exchange_statuses(
            summary_enabled,
            state
                .symbol_exchanges
                .get(&symbol)
                .unwrap_or(&state.exchanges),
            now,
            runtime_config.data_quality.heartbeat_stale_ms,
        );
        let health_status = health_status(summary_enabled, &exchanges);
        SpotWhaleSummary {
            status: current_latest
                .map(|signal| status_from_severity(signal.severity).to_string())
                .unwrap_or_else(|| "calm".to_string()),
            health_status: health_status.clone(),
            health_reason: health_reason(summary_enabled, &health_status).to_string(),
            direction: current_latest
                .map(|signal| format!("{:?}", signal.direction).to_ascii_lowercase())
                .unwrap_or_else(|| "neutral".to_string()),
            latest_direction: current_latest
                .map(|signal| format!("{:?}", signal.direction).to_ascii_lowercase())
                .unwrap_or_else(|| "neutral".to_string()),
            latest_severity: current_latest
                .map(|signal| signal.severity)
                .unwrap_or(SpotWhaleSeverity::Calm),
            latest_signal_at: latest.map(|signal| signal.ts),
            latest_age_sec,
            latest_is_stale,
            latest_stale_reason: latest_is_stale.then(|| "latest_signal_ttl_exceeded".to_string()),
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
        let query = SpotWhaleSignalQuery {
            symbol: Some(symbol.clone()),
            limit,
            ..SpotWhaleSignalQuery::default()
        };
        let total = self
            .query_persisted_signal_count(query.clone())
            .unwrap_or_else(|| self.in_memory_signal_count(&query));
        let items = self
            .query_persisted_signals(query.clone())
            .unwrap_or_else(|| self.in_memory_signals(&query));
        let has_more = items.len() < total;
        let next_cursor = next_spot_cursor(&items, has_more);
        SpotWhaleLatestResponse {
            summary: self.summary(&symbol),
            has_more,
            items,
            limit,
            offset: 0,
            total,
            next_cursor,
        }
    }

    pub fn history(&self, query: SpotWhaleQuery) -> SpotWhaleLatestResponse {
        let symbol = query.symbol.unwrap_or_else(|| "BTC".to_string());
        let symbol = normalize_symbol(&symbol);
        let limit = query.limit.unwrap_or(50).clamp(1, 200);
        let cursor_mode = query.cursor_ts.is_some();
        let offset = if cursor_mode {
            0
        } else {
            query.offset.unwrap_or(0)
        };
        let fetch_limit = if cursor_mode {
            limit.saturating_add(1)
        } else {
            limit
        };
        let persisted_query = SpotWhaleSignalQuery {
            symbol: Some(symbol.clone()),
            severity: query.severity.clone(),
            signal_type: query.signal_type.clone(),
            discord_sent: query.discord_sent,
            min_abs_net_volume_base: query.min_abs_net_volume_base,
            from_ts: query.from_ts,
            to_ts: query.to_ts,
            permanent_only: query.permanent_only,
            limit: fetch_limit,
            offset,
            cursor_ts: query.cursor_ts,
            cursor_signal_id: query.cursor_signal_id.clone(),
        };
        let total = self
            .query_persisted_signal_count(persisted_query.clone())
            .unwrap_or_else(|| self.in_memory_signal_count(&persisted_query));
        let mut items = self
            .query_persisted_signals(persisted_query.clone())
            .unwrap_or_else(|| self.in_memory_signals(&persisted_query));
        let has_more = if cursor_mode {
            items.len() > limit
        } else {
            offset.saturating_add(items.len()) < total
        };
        items.truncate(limit);
        let next_cursor = next_spot_cursor(&items, has_more);
        SpotWhaleLatestResponse {
            summary: self.summary(&symbol),
            has_more,
            items,
            limit,
            offset,
            total,
            next_cursor,
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
        let price_move_pct = same_venue_price_move_pct(&window_trades);
        let main_exchange = exchanges
            .iter()
            .max_by(|left, right| left.total_volume_base.total_cmp(&right.total_volume_base))
            .map(|item| item.exchange.clone());
        let dynamic_multiple =
            dynamic_multiple(&state.trades, symbol, window_sec, now, total_volume_base);
        let multi_exchange_confirmed = same_direction_exchange_count(
            &exchanges,
            net_volume_base,
            total_volume_base,
            total_notional_usd,
        ) >= 2;
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

    fn mark_trade(&self, exchange: SpotExchange, symbol: &str, ts: i64) {
        let mut state = self.state.write();
        let now = now_ms();
        mark_spot_trade_status(
            state
                .exchanges
                .entry(exchange.as_key().to_string())
                .or_insert_with(SpotExchangeStatus::disconnected),
            ts,
            now,
        );
        mark_spot_trade_status(
            state
                .symbol_exchanges
                .entry(normalize_symbol(symbol))
                .or_default()
                .entry(exchange.as_key().to_string())
                .or_insert_with(SpotExchangeStatus::disconnected),
            ts,
            now,
        );
    }

    pub fn run_retention_once(&self, now: i64) -> Option<usize> {
        let store = self.store.as_ref()?;
        let retention = spot_whale_runtime_config().retention;
        match store.prune_spot_whale_signals_retention(now) {
            Ok(deleted) => {
                tracing::info!(
                    target: LOG_TARGET,
                    table = "spot_whale_signals",
                    deleted,
                    ordinary_days = retention.signals_days.max(1),
                    important_days = retention.important_days.max(retention.signals_days.max(1)),
                    critical_days = retention.critical_days.max(retention.important_days.max(retention.signals_days.max(1))),
                    "{} retention completed",
                    LOG_PREFIX
                );
                Some(deleted)
            }
            Err(error) => {
                tracing::warn!(
                    target: LOG_TARGET,
                    table = "spot_whale_signals",
                    error = %error,
                    "{} retention failed",
                    LOG_PREFIX
                );
                None
            }
        }
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

    fn query_persisted_signal_count(&self, query: SpotWhaleSignalQuery) -> Option<usize> {
        let store = self.store.as_ref()?;
        match store.count_spot_whale_signals_with_query(&query) {
            Ok(count) => Some(count),
            Err(err) => {
                tracing::warn!(
                    target: LOG_TARGET,
                    "{} failed to count persisted signals: {err}",
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

    fn in_memory_signals(&self, query: &SpotWhaleSignalQuery) -> Vec<SpotWhaleSignal> {
        let mut signals = self
            .state
            .read()
            .signals
            .iter()
            .filter(|signal| signal_matches_query(signal, query))
            .cloned()
            .collect::<Vec<_>>();
        signals.sort_by(|left, right| right.ts.cmp(&left.ts).then_with(|| right.id.cmp(&left.id)));
        signals
            .into_iter()
            .skip(query.offset)
            .take(query.limit)
            .collect()
    }

    fn in_memory_signal_count(&self, query: &SpotWhaleSignalQuery) -> usize {
        self.state
            .read()
            .signals
            .iter()
            .rev()
            .filter(|signal| signal_matches_query(signal, query))
            .count()
    }
}

#[derive(Debug, Clone, Default)]
pub struct SpotWhaleQuery {
    pub symbol: Option<String>,
    pub severity: Option<String>,
    pub signal_type: Option<String>,
    pub discord_sent: Option<bool>,
    pub min_abs_net_volume_base: Option<f64>,
    pub offset: Option<usize>,
    pub cursor_ts: Option<i64>,
    pub cursor_signal_id: Option<String>,
    pub from_ts: Option<i64>,
    pub to_ts: Option<i64>,
    pub permanent_only: Option<bool>,
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
    total_volume_base: f64,
    total_notional_usd: f64,
) -> usize {
    let positive = net_volume_base > 0.0;
    exchanges
        .iter()
        .filter(|item| {
            item.total_volume_base > 0.0
                && item.dominance >= 0.55
                && item.total_volume_base / total_volume_base.max(f64::EPSILON) >= 0.05
                && item.total_notional_usd / total_notional_usd.max(f64::EPSILON) >= 0.02
        })
        .filter(|item| (item.net_volume_base > 0.0) == positive)
        .count()
}

fn same_venue_price_move_pct(trades: &[SpotTrade]) -> Option<f64> {
    let mut by_exchange: BTreeMap<String, Vec<&SpotTrade>> = BTreeMap::new();
    for trade in trades {
        by_exchange
            .entry(trade.exchange.as_key().to_string())
            .or_default()
            .push(trade);
    }
    let mut weighted_move = 0.0;
    let mut total_weight = 0.0;
    for venue_trades in by_exchange.values_mut() {
        venue_trades.sort_by_key(|trade| trade.ts);
        let (Some(first), Some(last)) = (venue_trades.first(), venue_trades.last()) else {
            continue;
        };
        if venue_trades.len() < 2 || last.ts <= first.ts || first.price <= 0.0 {
            continue;
        }
        let weight = venue_trades
            .iter()
            .map(|trade| trade.notional_usd.max(0.0))
            .sum::<f64>();
        if weight <= 0.0 {
            continue;
        }
        weighted_move += (last.price / first.price - 1.0) * 100.0 * weight;
        total_weight += weight;
    }
    (total_weight > 0.0).then_some(weighted_move / total_weight)
}

fn next_spot_cursor(items: &[SpotWhaleSignal], has_more: bool) -> Option<String> {
    has_more
        .then(|| items.last().map(encode_spot_cursor))
        .flatten()
}

pub fn encode_spot_history_cursor(signal: &SpotWhaleSignal) -> String {
    encode_spot_cursor(signal)
}

pub fn decode_spot_history_cursor(cursor: &str) -> Option<(i64, String)> {
    let decoded = BASE64_STANDARD.decode(cursor.trim()).ok()?;
    let decoded = String::from_utf8(decoded).ok()?;
    let (ts, signal_id) = decoded.split_once('|')?;
    let ts = ts.parse::<i64>().ok()?;
    (!signal_id.is_empty()).then(|| (ts, signal_id.to_string()))
}

fn encode_spot_cursor(signal: &SpotWhaleSignal) -> String {
    BASE64_STANDARD.encode(format!("{}|{}", signal.ts, signal.id))
}

fn mark_spot_trade_status(entry: &mut SpotExchangeStatus, ts: i64, now: i64) {
    entry.connected = true;
    entry.status = "connected".to_string();
    entry.last_trade_at = Some(ts);
    entry.latency_ms = Some(now.saturating_sub(ts).max(0));
    entry.last_error = None;
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

fn signal_matches_query(signal: &SpotWhaleSignal, query: &SpotWhaleSignalQuery) -> bool {
    if query
        .symbol
        .as_deref()
        .is_some_and(|symbol| signal.symbol != normalize_symbol(symbol))
    {
        return false;
    }
    if query
        .severity
        .as_deref()
        .is_some_and(|value| !value.eq_ignore_ascii_case(&format!("{:?}", signal.severity)))
    {
        return false;
    }
    if query.signal_type.as_deref().is_some_and(|value| {
        compact_filter_value(value) != compact_filter_value(&format!("{:?}", signal.signal_type))
    }) {
        return false;
    }
    if query
        .discord_sent
        .is_some_and(|value| signal.discord_sent != value)
    {
        return false;
    }
    if query
        .min_abs_net_volume_base
        .is_some_and(|threshold| signal.net_volume_base.abs() < threshold)
    {
        return false;
    }
    if query.from_ts.is_some_and(|from_ts| signal.ts < from_ts) {
        return false;
    }
    if query.to_ts.is_some_and(|to_ts| signal.ts >= to_ts) {
        return false;
    }
    if query.permanent_only.is_some_and(|value| {
        let is_permanent = signal.is_permanent
            || is_permanent_spot_whale_signal(&signal.symbol, signal.net_volume_base);
        is_permanent != value
    }) {
        return false;
    }
    match (query.cursor_ts, query.cursor_signal_id.as_deref()) {
        (Some(cursor_ts), Some(cursor_signal_id))
            if signal.ts > cursor_ts
                || (signal.ts == cursor_ts && signal.id.as_str() >= cursor_signal_id) =>
        {
            return false;
        }
        (Some(_), None) | (None, Some(_)) => return false,
        _ => {}
    }
    true
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

#[cfg(test)]
mod price_response_tests {
    use super::*;

    #[test]
    fn same_venue_price_response_does_not_turn_exchange_basis_into_a_move() {
        let trades = vec![
            spot_trade(1, SpotExchange::Binance, 100.0, 100.0),
            spot_trade(2, SpotExchange::Coinbase, 101.0, 101.0),
            spot_trade(3, SpotExchange::Binance, 100.0, 100.0),
            spot_trade(4, SpotExchange::Coinbase, 101.0, 101.0),
        ];

        assert_eq!(same_venue_price_move_pct(&trades), Some(0.0));
    }

    fn spot_trade(ts: i64, exchange: SpotExchange, price: f64, notional_usd: f64) -> SpotTrade {
        SpotTrade {
            ts,
            exchange,
            symbol: "BTC".to_string(),
            market: "spot".to_string(),
            price,
            qty_base: 1.0,
            notional_usd,
            side: SpotTradeSide::Buy,
            trade_id: None,
        }
    }
}
