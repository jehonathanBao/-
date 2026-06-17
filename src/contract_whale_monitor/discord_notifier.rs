use std::{
    collections::{BTreeMap, BTreeSet},
    sync::{Arc, OnceLock},
    time::Duration,
};

use parking_lot::RwLock;
use serde_json::Value;
use url::Url;

use crate::{
    contract_whale_monitor::{
        discord::{is_btc_contract_symbol, should_push_contract_whale_discord},
        log_events,
        types::{
            ContractWhaleDirection, ContractWhaleSeverity, ContractWhaleSignal,
            ContractWhaleSignalType,
        },
        LOG_PREFIX, LOG_TARGET,
    },
    normalizers::trade::now_ms,
    storage::{contract_whale_repo::ContractWhaleRepo, SqliteStore},
};

const DEFAULT_TIMEOUT_MS: u64 = 5_000;
const DEFAULT_MAX_ATTEMPTS: usize = 2;
const DEFAULT_COOLDOWN_SEC: i64 = 30;

static GLOBAL_COOLDOWN_STORE: OnceLock<ContractWhaleDiscordCooldownStore> = OnceLock::new();

#[derive(Debug, Clone)]
pub struct ContractWhaleDiscordSettings {
    pub enabled: bool,
    pub dry_run: bool,
    pub webhook_url: Option<String>,
    pub timeout_ms: u64,
    pub max_attempts: usize,
    pub cooldown_sec: i64,
}

impl ContractWhaleDiscordSettings {
    pub fn from_env(dry_run: bool) -> Self {
        Self {
            enabled: env_bool("CONTRACT_WHALE_DISCORD_ENABLED", true),
            dry_run,
            webhook_url: std::env::var("CONTRACT_WHALE_DISCORD_WEBHOOK_URL")
                .ok()
                .filter(|value| !value.trim().is_empty())
                .or_else(|| {
                    std::env::var("DISCORD_WEBHOOK_URL")
                        .ok()
                        .filter(|value| !value.trim().is_empty())
                }),
            timeout_ms: env_u64("CONTRACT_WHALE_DISCORD_TIMEOUT_MS", DEFAULT_TIMEOUT_MS),
            max_attempts: env_usize("CONTRACT_WHALE_DISCORD_MAX_ATTEMPTS", DEFAULT_MAX_ATTEMPTS)
                .clamp(1, 3),
            cooldown_sec: env_i64("CONTRACT_WHALE_DISCORD_COOLDOWN_SEC", DEFAULT_COOLDOWN_SEC)
                .clamp(30, 3600),
        }
    }

    pub fn dry_run_for_tests() -> Self {
        Self {
            enabled: true,
            dry_run: true,
            webhook_url: None,
            timeout_ms: DEFAULT_TIMEOUT_MS,
            max_attempts: 1,
            cooldown_sec: DEFAULT_COOLDOWN_SEC,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ContractWhaleDiscordGateDecision {
    pub allowed: bool,
    pub reason: String,
}

#[derive(Debug, Clone, PartialEq)]
pub struct ContractWhaleDiscordOutcome {
    pub eligible: bool,
    pub sent: bool,
    pub dry_run: bool,
    pub reason: String,
    pub sent_at_ms: Option<i64>,
    pub payload: Option<Value>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ContractWhaleDiscordCooldownEntry {
    pub signal_id: String,
    pub symbol: String,
    pub direction: ContractWhaleDirection,
    pub signal_type: ContractWhaleSignalType,
    pub severity: ContractWhaleSeverity,
    pub sent_at_ms: i64,
    pub score: u8,
}

#[derive(Debug, Clone, Default)]
pub struct ContractWhaleDiscordCooldownStore {
    inner: Arc<RwLock<ContractWhaleDiscordCooldownState>>,
}

#[derive(Debug, Default)]
struct ContractWhaleDiscordCooldownState {
    by_key: BTreeMap<ContractWhaleDiscordCooldownKey, ContractWhaleDiscordCooldownEntry>,
    sent_signal_ids: BTreeSet<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
struct ContractWhaleDiscordCooldownKey {
    symbol: String,
    direction: ContractWhaleDirection,
    signal_type: ContractWhaleSignalType,
}

impl ContractWhaleDiscordCooldownStore {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn record_sent(&self, signal: &ContractWhaleSignal, sent_at_ms: i64) {
        let mut state = self.inner.write();
        state.sent_signal_ids.insert(signal.id.clone());
        state.by_key.insert(
            cooldown_key(signal),
            ContractWhaleDiscordCooldownEntry {
                signal_id: signal.id.clone(),
                symbol: signal.symbol.clone(),
                direction: signal.direction,
                signal_type: signal.signal_type,
                severity: signal.severity,
                sent_at_ms,
                score: signal.score,
            },
        );
    }

    fn skip_reason(
        &self,
        signal: &ContractWhaleSignal,
        cooldown_sec: i64,
        now_ms: i64,
    ) -> Option<&'static str> {
        let state = self.inner.read();
        if state.sent_signal_ids.contains(&signal.id) {
            return Some("duplicate");
        }
        let entry = state.by_key.get(&cooldown_key(signal))?;
        let cooldown_ms = cooldown_sec.saturating_mul(1000);
        let within_cooldown = now_ms.saturating_sub(entry.sent_at_ms) < cooldown_ms;
        if within_cooldown && signal.severity.rank() <= entry.severity.rank() {
            return Some("cooldown");
        }
        None
    }
}

pub fn global_contract_whale_discord_cooldown_store() -> &'static ContractWhaleDiscordCooldownStore
{
    GLOBAL_COOLDOWN_STORE.get_or_init(ContractWhaleDiscordCooldownStore::new)
}

pub fn evaluate_contract_whale_discord_gate(
    settings: &ContractWhaleDiscordSettings,
    signal: &ContractWhaleSignal,
    cooldown_store: &ContractWhaleDiscordCooldownStore,
    now_ms: i64,
) -> ContractWhaleDiscordGateDecision {
    let primary_source_override = signal.discord_reason == "high_primary_source_extreme";
    let btc_contract_override = matches!(
        signal.severity,
        ContractWhaleSeverity::Medium | ContractWhaleSeverity::High
    ) && is_btc_contract_symbol(&signal.symbol);
    let min_score = match signal.severity {
        ContractWhaleSeverity::Medium if btc_contract_override => 0,
        ContractWhaleSeverity::High if btc_contract_override => 0,
        ContractWhaleSeverity::High if primary_source_override => 0,
        ContractWhaleSeverity::High => 85,
        ContractWhaleSeverity::Critical | ContractWhaleSeverity::S => 70,
        ContractWhaleSeverity::Medium | ContractWhaleSeverity::Calm => 101,
    };
    if !settings.enabled {
        return gate(false, "disabled");
    }
    if signal.data_quality < 70 {
        return gate(false, "data_quality_low");
    }
    if signal.score < min_score {
        return gate(false, "low_score");
    }
    if !signal.discord_eligible || !should_push_contract_whale_discord(signal) {
        return gate(false, "low_score");
    }
    if let Some(reason) = cooldown_store.skip_reason(signal, settings.cooldown_sec, now_ms) {
        return gate(false, reason);
    }
    if settings.dry_run {
        return gate(true, "dry_run");
    }
    gate(true, "eligible")
}

pub fn build_contract_whale_discord_payload(signal: &ContractWhaleSignal) -> Value {
    let severity = severity_label(signal.severity);
    let direction = direction_label(signal.direction);
    let signal_type = signal_type_label(signal.signal_type);
    let exchange_breakdown = signal
        .exchanges
        .iter()
        .map(|item| {
            format!(
                "{}: {:.0} BTC",
                item.exchange,
                item.total_volume_btc.max(0.0)
            )
        })
        .collect::<Vec<_>>()
        .join("\n");

    let description = if signal.liquidation_suspected && !signal.final_result.contains("强平") {
        format!("疑似强平推动，主力确定性降低：{}", signal.final_result)
    } else {
        signal.final_result.clone()
    };

    serde_json::json!({
        "content": format!("{} 主力合约异动 {severity}: {signal_type}", signal.symbol),
        "embeds": [{
            "title": format!("{} Contract Whale Flow", signal.symbol),
            "description": description,
            "color": severity_color(signal.severity),
            "fields": [
                {"name": "Symbol", "value": signal.symbol.clone(), "inline": true},
                {"name": "Event Type", "value": "contract_whale_flow", "inline": true},
                {"name": "Detector Type", "value": signal_type, "inline": true},
                {"name": "Direction", "value": direction, "inline": true},
                {"name": "Window", "value": format!("{}s", signal.window_sec), "inline": true},
                {"name": "Risk Score", "value": format!("{}/100", signal.score), "inline": true},
                {"name": "Data Quality", "value": format!("{}/100", signal.data_quality), "inline": true},
                {"name": "Total Volume", "value": format!("{:.0} BTC", signal.total_volume_btc), "inline": true},
                {"name": "Notional", "value": format!("${:.0}M", signal.total_notional_usd / 1_000_000.0), "inline": true},
                {"name": "Price", "value": trigger_price_label(signal.total_volume_btc, signal.total_notional_usd), "inline": true},
                {"name": "Net Direction", "value": format!("{:.0} BTC", signal.net_volume_btc), "inline": true},
                {"name": "Dominance", "value": format!("{:.1}%", signal.dominance * 100.0), "inline": true},
                {"name": "Price Move", "value": signal.price_move_pct.map(|value| format!("{value:+.2}%")).unwrap_or_else(|| "n/a".to_string()), "inline": true},
                {"name": "Dynamic Multiple", "value": signal.dynamic_multiple.map(|value| format!("{value:.1}x")).unwrap_or_else(|| "n/a".to_string()), "inline": true},
                {"name": "Percentile", "value": signal.percentile_level.map(|value| format!("P{value:.1}")).unwrap_or_else(|| "n/a".to_string()), "inline": true},
                {"name": "Liquidation", "value": liquidation_label(signal), "inline": true},
                {"name": "OI Change", "value": oi_label(signal), "inline": true},
                {"name": "Funding", "value": funding_label(signal), "inline": true},
                {"name": "Main Exchange", "value": signal.main_exchange.clone().unwrap_or_else(|| "multi".to_string()), "inline": true},
                {"name": "Exchanges", "value": if exchange_breakdown.is_empty() { "n/a".to_string() } else { exchange_breakdown }, "inline": false},
                {"name": "Final Result", "value": signal.final_result.clone(), "inline": false}
            ],
            "footer": {
                "text": format!("Candidate only | Signal: {}", signal.id)
            }
        }]
    })
}

fn trigger_price_label(total_volume_btc: f64, total_notional_usd: f64) -> String {
    if total_volume_btc <= 0.0 || total_notional_usd <= 0.0 {
        return "n/a".to_string();
    }
    format_price(total_notional_usd / total_volume_btc)
}

fn format_price(price: f64) -> String {
    if !price.is_finite() || price <= 0.0 {
        return "n/a".to_string();
    }
    if price >= 1000.0 {
        format!("${price:.0}")
    } else if price >= 1.0 {
        format!("${price:.2}")
    } else {
        format!("${price:.4}")
    }
}

pub fn build_contract_whale_discord_log_preview(signal: &ContractWhaleSignal) -> String {
    payload_preview(signal)
}

fn oi_label(signal: &ContractWhaleSignal) -> String {
    let bias = match signal.oi_bias.as_deref() {
        Some("rising") => "OI 上升，偏新开仓",
        Some("falling") => "OI 下降，偏平仓/去杠杆",
        Some("flat") => "OI 横盘",
        _ => "OI n/a",
    };
    match (signal.oi_change_5m_btc, signal.oi_change_pct) {
        (Some(change), Some(pct)) => format!("{change:+.0} BTC / {pct:+.2}% - {bias}"),
        (Some(change), None) => format!("{change:+.0} BTC - {bias}"),
        _ => bias.to_string(),
    }
}

fn funding_label(signal: &ContractWhaleSignal) -> String {
    let bias = match signal.funding_bias.as_deref() {
        Some("long") => "偏多",
        Some("short") => "偏空",
        Some("neutral") => "中性",
        _ => "n/a",
    };
    signal
        .funding_rate
        .map(|rate| format!("{:+.4}% {bias}", rate * 100.0))
        .unwrap_or_else(|| bias.to_string())
}

fn liquidation_label(signal: &ContractWhaleSignal) -> String {
    if signal.liquidation_suspected {
        format!(
            "suspected {:.0} BTC / {:.1}%",
            signal.liquidation_long_btc + signal.liquidation_short_btc,
            signal.liquidation_ratio.unwrap_or(0.0) * 100.0
        )
    } else {
        "not suspected".to_string()
    }
}

pub async fn notify_contract_whale_discord(
    settings: &ContractWhaleDiscordSettings,
    signal: &ContractWhaleSignal,
    store: Option<SqliteStore>,
) -> ContractWhaleDiscordOutcome {
    notify_contract_whale_discord_with_cooldown(
        settings,
        signal,
        store,
        global_contract_whale_discord_cooldown_store(),
    )
    .await
}

pub async fn notify_contract_whale_discord_with_cooldown(
    settings: &ContractWhaleDiscordSettings,
    signal: &ContractWhaleSignal,
    store: Option<SqliteStore>,
    cooldown_store: &ContractWhaleDiscordCooldownStore,
) -> ContractWhaleDiscordOutcome {
    let payload = build_contract_whale_discord_payload(signal);
    let gate_decision =
        evaluate_contract_whale_discord_gate(settings, signal, cooldown_store, now_ms());
    if !gate_decision.allowed {
        tracing::info!(
            target: LOG_TARGET,
            event = log_events::DISCORD_SKIPPED,
            signal_id = signal.id.as_str(),
            severity = ?signal.severity,
            score = signal.score,
            reason = gate_decision.reason.as_str(),
            "{} discord skipped: gate rejected",
            LOG_PREFIX
        );
        return skipped(signal, &gate_decision.reason, payload);
    }

    tracing::info!(
        target: LOG_TARGET,
        event = log_events::DISCORD_ELIGIBLE,
        signal_id = signal.id.as_str(),
        severity = ?signal.severity,
        score = signal.score,
        "{} discord eligible",
        LOG_PREFIX
    );

    if settings.dry_run {
        update_discord_status(store, signal.id.clone(), false, None).await;
        tracing::info!(
            target: LOG_TARGET,
            event = log_events::DISCORD_WOULD_SEND,
            signal_id = signal.id.as_str(),
            symbol = signal.symbol.as_str(),
            severity = ?signal.severity,
            message = %payload_preview(signal),
            "{} discord would_send",
            LOG_PREFIX
        );
        return ContractWhaleDiscordOutcome {
            eligible: true,
            sent: false,
            dry_run: true,
            reason: "dry_run".to_string(),
            sent_at_ms: None,
            payload: Some(payload),
        };
    }

    let Some(webhook_url) = settings.webhook_url.as_deref() else {
        tracing::warn!(
            target: LOG_TARGET,
            event = log_events::DISCORD_SKIPPED,
            signal_id = signal.id.as_str(),
            "{} discord skipped: webhook missing",
            LOG_PREFIX
        );
        return skipped(signal, "webhook_missing", payload);
    };

    if let Err(reason) = validate_discord_webhook_url(webhook_url) {
        tracing::warn!(
            target: LOG_TARGET,
            event = log_events::DISCORD_SKIPPED,
            signal_id = signal.id.as_str(),
            reason = reason.as_str(),
            "{} discord skipped: invalid webhook",
            LOG_PREFIX
        );
        return skipped(signal, "webhook_invalid", payload);
    }

    let client = match reqwest::Client::builder()
        .timeout(Duration::from_millis(settings.timeout_ms))
        .build()
    {
        Ok(client) => client,
        Err(error) => {
            tracing::warn!(
                target: LOG_TARGET,
                event = log_events::ERROR,
                signal_id = signal.id.as_str(),
                error = %error,
                "{} discord client build failed",
                LOG_PREFIX
            );
            return skipped(signal, "client_build_failed", payload);
        }
    };

    for attempt in 1..=settings.max_attempts {
        match client.post(webhook_url).json(&payload).send().await {
            Ok(response) if response.status().is_success() => {
                let sent_at_ms = now_ms();
                cooldown_store.record_sent(signal, sent_at_ms);
                update_discord_status(store, signal.id.clone(), true, Some(sent_at_ms)).await;
                tracing::info!(
                    target: LOG_TARGET,
                    event = log_events::DISCORD_SENT,
                    signal_id = signal.id.as_str(),
                    status = response.status().as_u16(),
                    "{} discord sent",
                    LOG_PREFIX
                );
                return ContractWhaleDiscordOutcome {
                    eligible: true,
                    sent: true,
                    dry_run: false,
                    reason: "sent".to_string(),
                    sent_at_ms: Some(sent_at_ms),
                    payload: Some(payload),
                };
            }
            Ok(response) => {
                let status = response.status();
                let retryable =
                    status.as_u16() == 429 || status.as_u16() == 408 || status.is_server_error();
                tracing::warn!(
                    target: LOG_TARGET,
                    event = log_events::ERROR,
                    signal_id = signal.id.as_str(),
                    status = status.as_u16(),
                    attempt = attempt,
                    retryable = retryable,
                    "{} discord send failed",
                    LOG_PREFIX
                );
                if !retryable || attempt >= settings.max_attempts {
                    break;
                }
                tokio::time::sleep(Duration::from_millis(500 * attempt as u64)).await;
            }
            Err(error) => {
                tracing::warn!(
                    target: LOG_TARGET,
                    event = log_events::ERROR,
                    signal_id = signal.id.as_str(),
                    attempt = attempt,
                    error = %error,
                    "{} discord send error",
                    LOG_PREFIX
                );
                if attempt >= settings.max_attempts {
                    break;
                }
                tokio::time::sleep(Duration::from_millis(500 * attempt as u64)).await;
            }
        }
    }

    update_discord_status(store, signal.id.clone(), false, None).await;
    ContractWhaleDiscordOutcome {
        eligible: true,
        sent: false,
        dry_run: false,
        reason: "send_failed".to_string(),
        sent_at_ms: None,
        payload: Some(payload),
    }
}

fn cooldown_key(signal: &ContractWhaleSignal) -> ContractWhaleDiscordCooldownKey {
    ContractWhaleDiscordCooldownKey {
        symbol: signal.symbol.to_ascii_uppercase(),
        direction: signal.direction,
        signal_type: signal.signal_type,
    }
}

fn gate(allowed: bool, reason: &str) -> ContractWhaleDiscordGateDecision {
    ContractWhaleDiscordGateDecision {
        allowed,
        reason: reason.to_string(),
    }
}

pub fn validate_discord_webhook_url(webhook_url: &str) -> Result<(), String> {
    let parsed = Url::parse(webhook_url).map_err(|_| "parse_failed".to_string())?;
    if parsed.scheme() != "https" {
        return Err("non_https".to_string());
    }
    let host = parsed.host_str().unwrap_or_default();
    if !matches!(host, "discord.com" | "discordapp.com") {
        return Err("host_not_allowed".to_string());
    }
    if !parsed.path().starts_with("/api/webhooks/") {
        return Err("path_not_allowed".to_string());
    }
    Ok(())
}

async fn update_discord_status(
    store: Option<SqliteStore>,
    signal_id: String,
    sent: bool,
    sent_at_ms: Option<i64>,
) {
    let Some(store) = store else {
        return;
    };
    match tokio::task::spawn_blocking(move || {
        store.update_contract_whale_discord_status(&signal_id, sent, sent_at_ms)
    })
    .await
    {
        Ok(Ok(_)) => {}
        Ok(Err(error)) => {
            tracing::warn!(
                target: LOG_TARGET,
                event = log_events::ERROR,
                error = %error,
                "{} discord status update failed",
                LOG_PREFIX
            );
        }
        Err(error) => {
            tracing::warn!(
                target: LOG_TARGET,
                event = log_events::ERROR,
                error = %error,
                "{} discord status update task failed",
                LOG_PREFIX
            );
        }
    }
}

fn skipped(
    signal: &ContractWhaleSignal,
    reason: &str,
    payload: Value,
) -> ContractWhaleDiscordOutcome {
    ContractWhaleDiscordOutcome {
        eligible: false,
        sent: false,
        dry_run: false,
        reason: if reason.trim().is_empty() {
            signal.discord_reason.clone()
        } else {
            reason.to_string()
        },
        sent_at_ms: None,
        payload: Some(payload),
    }
}

fn payload_preview(signal: &ContractWhaleSignal) -> String {
    format!(
        "{} CWM {} {} score={}/100 dataQuality={}/100 result={}",
        signal.symbol,
        severity_label(signal.severity),
        signal_type_label(signal.signal_type),
        signal.score,
        signal.data_quality,
        signal.final_result
    )
}

fn severity_label(severity: ContractWhaleSeverity) -> &'static str {
    match severity {
        ContractWhaleSeverity::S => "S级",
        ContractWhaleSeverity::Critical => "Critical",
        ContractWhaleSeverity::High => "High",
        ContractWhaleSeverity::Medium => "Medium",
        ContractWhaleSeverity::Calm => "Calm",
    }
}

fn signal_type_label(signal_type: ContractWhaleSignalType) -> &'static str {
    match signal_type {
        ContractWhaleSignalType::AggressiveBuy => "主力拉盘 / Aggressive Buy",
        ContractWhaleSignalType::AggressiveSell => "主力砸盘 / Aggressive Sell",
        ContractWhaleSignalType::DownsideAbsorption => "空头打不动 / 下方吸收",
        ContractWhaleSignalType::UpsideSuppression => "多头打不动 / 上方压制",
    }
}

fn direction_label(direction: ContractWhaleDirection) -> &'static str {
    match direction {
        ContractWhaleDirection::Buy => "主动买入",
        ContractWhaleDirection::Sell => "主动卖出",
        ContractWhaleDirection::Absorption => "卖出被吸收",
        ContractWhaleDirection::Suppression => "买入被压制",
    }
}

fn severity_color(severity: ContractWhaleSeverity) -> u32 {
    match severity {
        ContractWhaleSeverity::S => 0xc026d3,
        ContractWhaleSeverity::Critical => 0xef4444,
        ContractWhaleSeverity::High => 0xf97316,
        ContractWhaleSeverity::Medium => 0xeab308,
        ContractWhaleSeverity::Calm => 0x64748b,
    }
}

fn env_bool(key: &str, default: bool) -> bool {
    std::env::var(key)
        .ok()
        .map(|value| value.eq_ignore_ascii_case("true"))
        .unwrap_or(default)
}

fn env_u64(key: &str, default: u64) -> u64 {
    std::env::var(key)
        .ok()
        .and_then(|value| value.parse::<u64>().ok())
        .unwrap_or(default)
}

fn env_usize(key: &str, default: usize) -> usize {
    std::env::var(key)
        .ok()
        .and_then(|value| value.parse::<usize>().ok())
        .unwrap_or(default)
}

fn env_i64(key: &str, default: i64) -> i64 {
    std::env::var(key)
        .ok()
        .and_then(|value| value.parse::<i64>().ok())
        .unwrap_or(default)
}
