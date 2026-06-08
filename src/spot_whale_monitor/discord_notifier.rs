use std::{
    collections::{BTreeMap, BTreeSet},
    sync::{Arc, OnceLock},
    time::Duration,
};

use parking_lot::RwLock;
use serde_json::Value;

use crate::normalizers::trade::now_ms;

use super::{
    detector::discord_gate,
    types::{SpotWhaleDirection, SpotWhaleSeverity, SpotWhaleSignal, SpotWhaleSignalType},
    LOG_PREFIX, LOG_TARGET,
};

const DEFAULT_TIMEOUT_MS: u64 = 5_000;
const DEFAULT_MAX_ATTEMPTS: usize = 2;
const DEFAULT_COOLDOWN_SEC: i64 = 180;

static GLOBAL_COOLDOWN_STORE: OnceLock<SpotWhaleDiscordCooldownStore> = OnceLock::new();

#[derive(Debug, Clone)]
pub struct SpotWhaleDiscordSettings {
    pub enabled: bool,
    pub dry_run: bool,
    pub webhook_url: Option<String>,
    pub timeout_ms: u64,
    pub max_attempts: usize,
    pub cooldown_sec: i64,
}

impl SpotWhaleDiscordSettings {
    pub fn from_env(dry_run: bool) -> Self {
        Self {
            enabled: env_bool("SPOT_WHALE_DISCORD_ENABLED", true),
            dry_run,
            webhook_url: std::env::var("SPOT_WHALE_DISCORD_WEBHOOK_URL")
                .ok()
                .filter(|value| !value.trim().is_empty())
                .or_else(|| {
                    std::env::var("DISCORD_WEBHOOK_URL")
                        .ok()
                        .filter(|value| !value.trim().is_empty())
                }),
            timeout_ms: env_u64("SPOT_WHALE_DISCORD_TIMEOUT_MS", DEFAULT_TIMEOUT_MS),
            max_attempts: env_usize("SPOT_WHALE_DISCORD_MAX_ATTEMPTS", DEFAULT_MAX_ATTEMPTS)
                .clamp(1, 3),
            cooldown_sec: env_i64("SPOT_WHALE_DISCORD_COOLDOWN_SEC", DEFAULT_COOLDOWN_SEC)
                .clamp(30, 3600),
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct SpotWhaleDiscordOutcome {
    pub eligible: bool,
    pub sent: bool,
    pub dry_run: bool,
    pub reason: String,
    pub sent_at_ms: Option<i64>,
    pub payload: Option<Value>,
}

#[derive(Debug, Clone, Default)]
pub struct SpotWhaleDiscordCooldownStore {
    inner: Arc<RwLock<SpotWhaleDiscordCooldownState>>,
}

#[derive(Debug, Default)]
struct SpotWhaleDiscordCooldownState {
    by_key: BTreeMap<SpotWhaleDiscordCooldownKey, SpotWhaleDiscordCooldownEntry>,
    sent_signal_ids: BTreeSet<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
struct SpotWhaleDiscordCooldownKey {
    symbol: String,
    direction: SpotWhaleDirection,
    signal_type: SpotWhaleSignalType,
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct SpotWhaleDiscordCooldownEntry {
    signal_id: String,
    severity: SpotWhaleSeverity,
    sent_at_ms: i64,
    score: u8,
}

impl SpotWhaleDiscordCooldownStore {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn record_sent(&self, signal: &SpotWhaleSignal, sent_at_ms: i64) {
        let mut state = self.inner.write();
        state.sent_signal_ids.insert(signal.id.clone());
        state.by_key.insert(
            cooldown_key(signal),
            SpotWhaleDiscordCooldownEntry {
                signal_id: signal.id.clone(),
                severity: signal.severity,
                sent_at_ms,
                score: signal.score,
            },
        );
    }

    fn skip_reason(
        &self,
        signal: &SpotWhaleSignal,
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

pub fn global_spot_whale_discord_cooldown_store() -> &'static SpotWhaleDiscordCooldownStore {
    GLOBAL_COOLDOWN_STORE.get_or_init(SpotWhaleDiscordCooldownStore::new)
}

pub async fn notify_spot_whale_discord(
    settings: &SpotWhaleDiscordSettings,
    signal: &SpotWhaleSignal,
) -> SpotWhaleDiscordOutcome {
    let now = now_ms();
    let cooldown_store = global_spot_whale_discord_cooldown_store();
    let (eligible_by_signal, gate_reason) = discord_gate(
        signal.severity,
        signal.score,
        signal.multi_exchange_confirmed,
        signal.data_quality,
    );
    if !settings.enabled {
        return outcome(false, false, settings.dry_run, "disabled", None, None);
    }
    if !eligible_by_signal || !signal.discord_eligible {
        return outcome(
            false,
            false,
            settings.dry_run,
            gate_reason.as_str(),
            None,
            None,
        );
    }
    if let Some(reason) = cooldown_store.skip_reason(signal, settings.cooldown_sec, now) {
        return outcome(false, false, settings.dry_run, reason, None, None);
    }
    let payload = build_spot_whale_discord_payload(signal);
    if settings.dry_run {
        tracing::info!(
            target: LOG_TARGET,
            symbol = signal.symbol.as_str(),
            "{} discord would_send",
            LOG_PREFIX
        );
        return outcome(true, false, true, "dry_run", None, Some(payload));
    }
    let Some(webhook_url) = settings.webhook_url.as_deref() else {
        return outcome(true, false, false, "webhook_missing", None, Some(payload));
    };
    if !is_valid_discord_webhook(webhook_url) {
        return outcome(true, false, false, "webhook_invalid", None, Some(payload));
    }
    let client = match reqwest::Client::builder()
        .timeout(Duration::from_millis(settings.timeout_ms))
        .build()
    {
        Ok(client) => client,
        Err(_) => {
            return outcome(
                true,
                false,
                false,
                "client_build_failed",
                None,
                Some(payload),
            )
        }
    };
    let mut last_error = "send_failed".to_string();
    for _ in 0..settings.max_attempts {
        match client.post(webhook_url).json(&payload).send().await {
            Ok(response) if response.status().is_success() => {
                cooldown_store.record_sent(signal, now);
                tracing::info!(
                    target: LOG_TARGET,
                    symbol = signal.symbol.as_str(),
                    "{} discord sent",
                    LOG_PREFIX
                );
                return outcome(true, true, false, "sent", Some(now), Some(payload));
            }
            Ok(response) => {
                last_error = format!("http_status_{}", response.status().as_u16());
                if response.status().as_u16() == 429 {
                    break;
                }
            }
            Err(error) => {
                last_error = error.to_string();
            }
        }
    }
    outcome(true, false, false, &last_error, None, Some(payload))
}

pub fn build_spot_whale_discord_payload(signal: &SpotWhaleSignal) -> Value {
    let exchange_breakdown = signal
        .exchanges
        .iter()
        .map(|item| {
            format!(
                "{}: {:.2} {}",
                item.exchange, item.total_volume_base, signal.symbol
            )
        })
        .collect::<Vec<_>>()
        .join("\n");
    serde_json::json!({
        "content": format!("{} 现货异动 {}: {}", signal.symbol, severity_label(signal.severity), signal_type_label(signal.signal_type)),
        "embeds": [{
            "title": format!("{} Spot Whale Flow", signal.symbol),
            "description": signal.final_result,
            "color": severity_color(signal.severity),
            "fields": [
                {"name": "Symbol", "value": signal.symbol, "inline": true},
                {"name": "Event Type", "value": "spot_whale_flow", "inline": true},
                {"name": "Detector Type", "value": signal_type_label(signal.signal_type), "inline": true},
                {"name": "Direction", "value": direction_label(signal.direction), "inline": true},
                {"name": "Window", "value": format!("{}s", signal.window_sec), "inline": true},
                {"name": "Risk Score", "value": format!("{}/100", signal.score), "inline": true},
                {"name": "Data Quality", "value": format!("{}/100", signal.data_quality), "inline": true},
                {"name": "Total Volume", "value": format!("{:.2} {}", signal.total_volume_base, signal.symbol), "inline": true},
                {"name": "Notional", "value": format!("${:.1}M", signal.total_notional_usd / 1_000_000.0), "inline": true},
                {"name": "Net Direction", "value": format!("{:+.2} {}", signal.net_volume_base, signal.symbol), "inline": true},
                {"name": "Dominance", "value": format!("{:.1}%", signal.dominance * 100.0), "inline": true},
                {"name": "Price Move", "value": signal.price_move_pct.map(|value| format!("{value:+.2}%")).unwrap_or_else(|| "n/a".to_string()), "inline": true},
                {"name": "Coinbase Premium", "value": signal.coinbase_premium_pct.map(|value| format!("{value:+.3}%")).unwrap_or_else(|| "n/a".to_string()), "inline": true},
                {"name": "Main Exchange", "value": signal.main_exchange.clone().unwrap_or_else(|| "multi".to_string()), "inline": true},
                {"name": "Exchanges", "value": if exchange_breakdown.is_empty() { "n/a".to_string() } else { exchange_breakdown }, "inline": false},
                {"name": "Final Result", "value": signal.final_result, "inline": false}
            ],
            "footer": {"text": format!("Candidate only | Signal: {}", signal.id)}
        }]
    })
}

fn outcome(
    eligible: bool,
    sent: bool,
    dry_run: bool,
    reason: &str,
    sent_at_ms: Option<i64>,
    payload: Option<Value>,
) -> SpotWhaleDiscordOutcome {
    SpotWhaleDiscordOutcome {
        eligible,
        sent,
        dry_run,
        reason: reason.to_string(),
        sent_at_ms,
        payload,
    }
}

fn cooldown_key(signal: &SpotWhaleSignal) -> SpotWhaleDiscordCooldownKey {
    SpotWhaleDiscordCooldownKey {
        symbol: signal.symbol.clone(),
        direction: signal.direction,
        signal_type: signal.signal_type,
    }
}

fn is_valid_discord_webhook(value: &str) -> bool {
    url::Url::parse(value)
        .ok()
        .filter(|url| url.scheme() == "https")
        .and_then(|url| {
            url.host_str()
                .map(|host| host == "discord.com" || host == "discordapp.com")
        })
        .unwrap_or(false)
}

fn severity_label(severity: SpotWhaleSeverity) -> &'static str {
    match severity {
        SpotWhaleSeverity::S => "S",
        SpotWhaleSeverity::Critical => "Critical",
        SpotWhaleSeverity::High => "High",
        SpotWhaleSeverity::Medium => "Medium",
        SpotWhaleSeverity::Calm => "Calm",
    }
}

fn signal_type_label(signal_type: SpotWhaleSignalType) -> &'static str {
    match signal_type {
        SpotWhaleSignalType::SpotAggressiveBuy => "现货主动买入爆发",
        SpotWhaleSignalType::SpotAggressiveSell => "现货主动卖出爆发",
        SpotWhaleSignalType::SpotDownsideAbsorption => "现货下方吸收",
        SpotWhaleSignalType::SpotUpsideSuppression => "现货上方压制",
        SpotWhaleSignalType::SpotExchangeDislocation => "现货交易所错位",
    }
}

fn direction_label(direction: SpotWhaleDirection) -> &'static str {
    match direction {
        SpotWhaleDirection::Buy => "主动买入",
        SpotWhaleDirection::Sell => "主动卖出",
        SpotWhaleDirection::Absorption => "下方吸收",
        SpotWhaleDirection::Suppression => "上方压制",
        SpotWhaleDirection::Dislocation => "跨所错位",
    }
}

fn severity_color(severity: SpotWhaleSeverity) -> u32 {
    match severity {
        SpotWhaleSeverity::S => 0xd946ef,
        SpotWhaleSeverity::Critical => 0xef4444,
        SpotWhaleSeverity::High => 0xf97316,
        SpotWhaleSeverity::Medium => 0xeab308,
        SpotWhaleSeverity::Calm => 0x64748b,
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
