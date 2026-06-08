use std::{
    collections::{BTreeMap, BTreeSet, VecDeque},
    sync::Arc,
};

use parking_lot::RwLock;
use tokio::task::JoinHandle;

use crate::normalizers::trade::now_ms;

use super::{
    collector_binance, collector_coinbase,
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
const DUPLICATE_WINDOW_MS: i64 = 10_000;

#[derive(Clone)]
pub struct SpotWhaleService {
    enabled: bool,
    dry_run: bool,
    booted_at_ms: i64,
    state: Arc<RwLock<SpotWhaleState>>,
    tasks: Arc<RwLock<Vec<JoinHandle<()>>>>,
}

#[derive(Debug)]
struct SpotWhaleState {
    trades: VecDeque<SpotTrade>,
    signals: VecDeque<SpotWhaleSignal>,
    seen_signal_ids: BTreeSet<String>,
    exchanges: BTreeMap<String, SpotExchangeStatus>,
    last_discord_sent_at: Option<i64>,
}

impl SpotWhaleService {
    pub fn new(enabled: bool, dry_run: bool, booted_at_ms: i64) -> Self {
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
        Self {
            enabled,
            dry_run,
            booted_at_ms,
            state: Arc::new(RwLock::new(SpotWhaleState {
                trades: VecDeque::new(),
                signals: VecDeque::new(),
                seen_signal_ids: BTreeSet::new(),
                exchanges,
                last_discord_sent_at: None,
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
    }

    pub fn stop(&self) {
        let tasks = std::mem::take(&mut *self.tasks.write());
        for task in tasks {
            task.abort();
        }
        self.set_exchange_status(SpotExchange::Binance, "disconnected", false, None);
        self.set_exchange_status(SpotExchange::Coinbase, "disconnected", false, None);
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
        let latest = state
            .signals
            .iter()
            .rev()
            .find(|signal| signal.symbol == symbol);
        let trend60s = trend_for_symbol(&state.trades, &symbol, now_ms());
        let health_status = health_status(self.enabled, &state.exchanges);
        SpotWhaleSummary {
            status: latest
                .map(|signal| status_from_severity(signal.severity).to_string())
                .unwrap_or_else(|| "calm".to_string()),
            health_status: health_status.clone(),
            health_reason: health_reason(self.enabled, &health_status).to_string(),
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
                .count(),
            read_only: true,
            enabled: self.enabled && spot_whale_runtime_config().symbol_enabled(&symbol),
            dry_run: self.dry_run,
            symbol,
            trend60s,
            exchanges: state.exchanges.clone(),
        }
    }

    pub fn latest(&self, symbol: &str, limit: usize) -> SpotWhaleLatestResponse {
        let symbol = normalize_symbol(symbol);
        let limit = limit.clamp(1, 200);
        let items = self
            .state
            .read()
            .signals
            .iter()
            .rev()
            .filter(|signal| signal.symbol == symbol)
            .take(limit)
            .cloned()
            .collect::<Vec<_>>();
        SpotWhaleLatestResponse {
            summary: self.summary(&symbol),
            items,
            limit,
        }
    }

    pub fn history(&self, query: SpotWhaleQuery) -> SpotWhaleLatestResponse {
        let symbol = query.symbol.unwrap_or_else(|| "BTC".to_string());
        let limit = query.limit.unwrap_or(50).clamp(1, 200);
        let items = self
            .state
            .read()
            .signals
            .iter()
            .rev()
            .filter(|signal| signal.symbol == normalize_symbol(&symbol))
            .filter(|signal| {
                query
                    .severity
                    .as_deref()
                    .map(|value| value.eq_ignore_ascii_case(&format!("{:?}", signal.severity)))
                    .unwrap_or(true)
            })
            .filter(|signal| {
                query
                    .signal_type
                    .as_deref()
                    .map(|value| value.eq_ignore_ascii_case(&format!("{:?}", signal.signal_type)))
                    .unwrap_or(true)
            })
            .filter(|signal| {
                query
                    .discord_sent
                    .map(|value| signal.discord_sent == value)
                    .unwrap_or(true)
            })
            .take(limit)
            .cloned()
            .collect::<Vec<_>>();
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
        let mut state = self.state.write();
        if state.seen_signal_ids.contains(&signal.id) || duplicate_recent(&state.signals, &signal) {
            return false;
        }
        state.seen_signal_ids.insert(signal.id.clone());
        state.signals.push_back(signal);
        while state.signals.len() > MAX_SIGNALS {
            if let Some(old) = state.signals.pop_front() {
                state.seen_signal_ids.remove(&old.id);
            }
        }
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
            signal.discord_reason = reason;
        }
        if sent {
            state.last_discord_sent_at = sent_at_ms;
        }
    }
}

#[derive(Debug, Clone, Default)]
pub struct SpotWhaleQuery {
    pub symbol: Option<String>,
    pub severity: Option<String>,
    pub signal_type: Option<String>,
    pub discord_sent: Option<bool>,
    pub limit: Option<usize>,
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

fn duplicate_recent(signals: &VecDeque<SpotWhaleSignal>, signal: &SpotWhaleSignal) -> bool {
    signals.iter().rev().take(20).any(|existing| {
        existing.symbol == signal.symbol
            && existing.signal_type == signal.signal_type
            && existing.direction == signal.direction
            && signal.ts.saturating_sub(existing.ts) <= DUPLICATE_WINDOW_MS
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
            "healthy" => "binance_coinbase_recent",
            "degraded" => "single_spot_source_recent",
            "unhealthy" => "spot_sources_disconnected",
            _ => "spot_whale_status_unknown",
        }
    }
}

fn redact_error(error: String) -> String {
    error.replace(
        "https://discord.com/api/webhooks/",
        "https://discord.com/api/webhooks/[redacted]/",
    )
}
