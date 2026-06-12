use std::{
    collections::{BTreeMap, BTreeSet, VecDeque},
    fs::{self, OpenOptions},
    io::Write,
    path::PathBuf,
    sync::Arc,
};

use parking_lot::RwLock;
use tokio::task::JoinHandle;

use crate::normalizers::trade::now_ms;

use super::{
    aggregator::{rolling_window_stats, trend_for_symbol},
    collector,
    config::binance_alt_contract_runtime_config,
    context::empty_context,
    detector::detect_alt_contract_signal,
    symbol_universe::meta_from_product_id,
    types::{
        AltContractContext, AltContractDryRunStats, AltContractExchange, AltContractExchangeStatus,
        AltContractLatestResponse, AltContractSeverity, AltContractSignal, AltContractSummary,
        AltContractSymbolUniverseSummary, AltContractTrade,
    },
    LOG_PREFIX, LOG_TARGET,
};

const MAX_TRADES: usize = 200_000;
const MAX_SIGNALS: usize = 1_000;
const TRADE_RETENTION_MS: i64 = 3_600_000;
const DUPLICATE_WINDOW_MS: i64 = 10_000;
const OI_RETENTION_MS: i64 = 10 * 60_000;
const LIQUIDATION_CONTEXT_TTL_MS: i64 = 60_000;

#[derive(Clone)]
pub struct BinanceAltContractService {
    enabled: bool,
    dry_run: bool,
    booted_at_ms: i64,
    persistence_path: PathBuf,
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
    oi_snapshots: BTreeMap<String, VecDeque<(i64, f64)>>,
    liquidation_seen_at: BTreeMap<String, i64>,
    last_oi_poll_at: Option<i64>,
    last_force_order_at: Option<i64>,
    error_events: VecDeque<i64>,
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
        let restored = load_persisted_signals(&runtime_config.persistence_path, MAX_SIGNALS);
        Self {
            enabled,
            dry_run,
            booted_at_ms,
            persistence_path: runtime_config.persistence_path,
            state: Arc::new(RwLock::new(BinanceAltContractState {
                trades: VecDeque::new(),
                signals: restored.signals,
                seen_signal_ids: restored.seen_signal_ids,
                exchanges,
                contexts: BTreeMap::new(),
                oi_snapshots: BTreeMap::new(),
                liquidation_seen_at: BTreeMap::new(),
                last_oi_poll_at: None,
                last_force_order_at: None,
                error_events: VecDeque::new(),
            })),
            tasks: Arc::new(RwLock::new(Vec::new())),
        }
    }

    pub fn start(&self) {
        if !self.enabled || self.tasks.read().iter().any(|task| !task.is_finished()) {
            return;
        }
        let config = binance_alt_contract_runtime_config();
        tracing::info!(
            target: LOG_TARGET,
            enabled = self.enabled,
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
                collector::run_force_order_stream(service).await;
            }));
        }
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
        if !self.enabled || !config.symbol_enabled(&trade.product_id) {
            return Vec::new();
        }
        self.mark_trade(trade.exchange, trade.ts);
        {
            let mut state = self.state.write();
            state.trades.push_back(trade.clone());
            prune_trades(&mut state.trades, trade.ts);
        }
        let meta = meta_from_product_id(&trade.product_id);
        let context = self.context_for_product(&trade.product_id);
        let mut candidates = config
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
            .filter_map(|stats| detect_alt_contract_signal(&stats, &context, &config))
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

    pub fn update_open_interest(&self, product_id: &str, ts: i64, open_interest_base: f64) {
        if !open_interest_base.is_finite() || open_interest_base <= 0.0 {
            return;
        }
        let product_id = product_id_for_symbol(product_id);
        let mut state = self.state.write();
        let snapshots = state
            .oi_snapshots
            .entry(product_id.clone())
            .or_insert_with(VecDeque::new);
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
        let now = now_ms();
        let state = self.state.read();
        let product_filter = symbol.map(product_id_for_symbol);
        let latest = state.signals.iter().rev().find(|signal| {
            product_filter
                .as_ref()
                .map(|item| &signal.product_id == item)
                .unwrap_or(true)
        });
        let monitored_symbols = config.enabled_symbols();
        let trend_product = product_filter
            .clone()
            .or_else(|| latest.map(|signal| signal.product_id.clone()))
            .or_else(|| monitored_symbols.first().cloned())
            .unwrap_or_else(|| "SOLUSDT".to_string());
        let exchanges = summarized_exchange_statuses(
            self.enabled,
            &state.exchanges,
            now,
            config.data_quality.heartbeat_stale_ms,
        );
        let active_anomaly_count = state
            .signals
            .iter()
            .filter(|signal| now.saturating_sub(signal.ts) <= 15 * 60_000)
            .count();
        let recent_critical_or_s_count = state
            .signals
            .iter()
            .filter(|signal| now.saturating_sub(signal.ts) <= 60 * 60_000)
            .filter(|signal| signal.severity.rank() >= AltContractSeverity::Critical.rank())
            .count();
        let dry_run_would_send_count = state
            .signals
            .iter()
            .filter(|signal| now.saturating_sub(signal.ts) <= 60 * 60_000)
            .filter(|signal| signal.discord_would_send)
            .count();
        let health_status = health_status(self.enabled, &exchanges);
        let dry_run_stats = dry_run_stats(&state.signals, now);
        let last_trade_at = exchanges
            .values()
            .filter_map(|status| status.last_trade_at)
            .max();
        AltContractSummary {
            status: latest
                .map(|signal| status_from_severity(signal.severity).to_string())
                .unwrap_or_else(|| "calm".to_string()),
            health_status: health_status.clone(),
            health_reason: health_reason(self.enabled, &health_status).to_string(),
            collector_status: collector_status(self.enabled, exchanges.get("binance")),
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
            top_active_symbols: top_active_symbols(&state.trades, now),
            errors1h: state
                .error_events
                .iter()
                .filter(|seen_at| now.saturating_sub(**seen_at) <= 60 * 60_000)
                .count(),
            latest_direction: latest
                .map(|signal| format!("{:?}", signal.direction).to_ascii_lowercase())
                .unwrap_or_else(|| "neutral".to_string()),
            latest_severity: latest
                .map(|signal| signal.severity)
                .unwrap_or(AltContractSeverity::Calm),
            latest_signal_at: latest.map(|signal| signal.ts),
            signal_count: state
                .signals
                .iter()
                .filter(|signal| {
                    product_filter
                        .as_ref()
                        .map(|item| &signal.product_id == item)
                        .unwrap_or(true)
                })
                .count(),
            monitored_symbols,
            active_anomaly_count,
            recent_critical_or_s_count,
            dry_run_would_send_count,
            enabled: self.enabled,
            dry_run: self.dry_run,
            read_only: true,
            symbol: symbol.map(|value| value.to_ascii_uppercase()),
            trend60s: trend_for_symbol(&state.trades, &trend_product, now),
            exchanges,
            dry_run_stats,
            symbol_universe: symbol_universe_summary(&config),
        }
    }

    pub fn latest(&self, symbol: Option<&str>, limit: usize) -> AltContractLatestResponse {
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

    fn insert_signal(&self, signal: AltContractSignal) -> bool {
        let mut state = self.state.write();
        if state.seen_signal_ids.contains(&signal.id) || duplicate_recent(&state.signals, &signal) {
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

    fn persist_signal(&self, signal: &AltContractSignal) {
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
}

#[derive(Debug, Default)]
struct RestoredAltContractSignals {
    signals: VecDeque<AltContractSignal>,
    seen_signal_ids: BTreeSet<String>,
}

fn load_persisted_signals(path: &PathBuf, limit: usize) -> RestoredAltContractSignals {
    let Ok(text) = fs::read_to_string(path) else {
        return RestoredAltContractSignals::default();
    };
    let mut signals = text
        .lines()
        .rev()
        .filter_map(|line| serde_json::from_str::<AltContractSignal>(line).ok())
        .take(limit)
        .collect::<Vec<_>>();
    signals.reverse();
    let seen_signal_ids = signals.iter().map(|signal| signal.id.clone()).collect();
    RestoredAltContractSignals {
        signals: VecDeque::from(signals),
        seen_signal_ids,
    }
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
            .filter(|signal| signal.discord_reason == "low_score")
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

fn symbol_universe_summary(
    config: &super::config::BinanceAltContractRuntimeConfig,
) -> AltContractSymbolUniverseSummary {
    AltContractSymbolUniverseSummary {
        mode: if config.symbol_universe.whitelist.is_empty() {
            "auto".to_string()
        } else {
            "whitelist_only".to_string()
        },
        limit: config.symbol_universe.symbol_limit,
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

fn redact_error(error: String) -> String {
    error.replace(
        "https://discord.com/api/webhooks/",
        "https://discord.com/api/webhooks/[redacted]/",
    )
}
