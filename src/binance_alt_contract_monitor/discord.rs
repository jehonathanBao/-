use std::{
    collections::{BTreeMap, BTreeSet},
    sync::{Arc, OnceLock},
};

use parking_lot::RwLock;

use crate::normalizers::trade::now_ms;

use super::{
    config::BinanceAltDiscordConfig,
    types::{AltContractDirection, AltContractSeverity, AltContractSignal, AltContractSignalType},
};

static GLOBAL_COOLDOWN_STORE: OnceLock<AltContractDiscordCooldownStore> = OnceLock::new();

#[derive(Debug, Clone, PartialEq)]
pub struct AltContractDiscordGate {
    pub eligible: bool,
    pub would_send: bool,
    pub sent: bool,
    pub dry_run: bool,
    pub reason: String,
    pub sent_at_ms: Option<i64>,
}

#[derive(Debug, Clone, Default)]
pub struct AltContractDiscordCooldownStore {
    inner: Arc<RwLock<AltContractDiscordCooldownState>>,
}

#[derive(Debug, Default)]
struct AltContractDiscordCooldownState {
    by_key: BTreeMap<AltContractDiscordCooldownKey, AltContractDiscordCooldownEntry>,
    sent_signal_ids: BTreeSet<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
struct AltContractDiscordCooldownKey {
    symbol: String,
    direction: AltContractDirection,
    signal_type: AltContractSignalType,
}

#[derive(Debug, Clone)]
struct AltContractDiscordCooldownEntry {
    severity: AltContractSeverity,
    sent_at_ms: i64,
}

impl AltContractDiscordCooldownStore {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn record_would_send(&self, signal: &AltContractSignal, sent_at_ms: i64) {
        let mut state = self.inner.write();
        state.sent_signal_ids.insert(signal.id.clone());
        state.by_key.insert(
            cooldown_key(signal),
            AltContractDiscordCooldownEntry {
                severity: signal.severity,
                sent_at_ms,
            },
        );
    }

    fn skip_reason(
        &self,
        signal: &AltContractSignal,
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

pub fn global_alt_contract_discord_cooldown_store() -> &'static AltContractDiscordCooldownStore {
    GLOBAL_COOLDOWN_STORE.get_or_init(AltContractDiscordCooldownStore::new)
}

pub fn evaluate_alt_contract_discord_gate(
    signal: &AltContractSignal,
    config: &BinanceAltDiscordConfig,
    warmup: bool,
) -> AltContractDiscordGate {
    evaluate_alt_contract_discord_gate_with_store(
        signal,
        config,
        warmup,
        global_alt_contract_discord_cooldown_store(),
        now_ms(),
    )
}

pub fn evaluate_alt_contract_discord_gate_with_store(
    signal: &AltContractSignal,
    config: &BinanceAltDiscordConfig,
    warmup: bool,
    cooldown_store: &AltContractDiscordCooldownStore,
    now: i64,
) -> AltContractDiscordGate {
    if !config.enabled {
        return gate(false, false, config.dry_run, "disabled", None);
    }
    if warmup {
        return gate(false, false, config.dry_run, "warmup", None);
    }
    if signal.data_quality < 70 {
        return gate(false, false, config.dry_run, "data_quality_low", None);
    }
    let score_ok = signal.build_score >= config.min_build_score
        || signal.abnormal_score >= config.min_abnormal_score;
    if !score_ok {
        return gate(false, false, config.dry_run, "low_score", None);
    }
    if let Some(reason) = cooldown_store.skip_reason(signal, config.cooldown_sec, now) {
        return gate(false, false, config.dry_run, reason, None);
    }
    if config.dry_run {
        cooldown_store.record_would_send(signal, now);
        return gate(true, true, true, "dry_run_would_send", None);
    }
    gate(true, false, false, "live_send_not_enabled_for_bacm", None)
}

pub fn build_alt_contract_discord_payload(signal: &AltContractSignal) -> serde_json::Value {
    serde_json::json!({
        "content": format!("{} 山寨合约异动 {}: {}", signal.symbol, severity_label(signal.severity), signal_type_label(signal.signal_type)),
        "embeds": [{
            "title": format!("{} Binance Alt Contract Anomaly", signal.symbol),
            "description": signal.final_result,
            "fields": [
                {"name": "Symbol", "value": signal.symbol, "inline": true},
                {"name": "Type", "value": signal_type_label(signal.signal_type), "inline": true},
                {"name": "Direction", "value": direction_label(signal.direction), "inline": true},
                {"name": "Window", "value": format!("{}s", signal.window_sec), "inline": true},
                {"name": "Abnormal Score", "value": format!("{}/100", signal.abnormal_score), "inline": true},
                {"name": "Build Score", "value": format!("{}/100", signal.build_score), "inline": true},
                {"name": "Notional", "value": format!("${:.1}M", signal.total_notional_usd / 1_000_000.0), "inline": true},
                {"name": "Dynamic Multiple", "value": signal.dynamic_multiple.map(|value| format!("{value:.1}x")).unwrap_or_else(|| "n/a".to_string()), "inline": true},
                {"name": "Data Quality", "value": format!("{}/100", signal.data_quality), "inline": true},
                {"name": "Final Result", "value": signal.final_result, "inline": false}
            ],
            "footer": {"text": format!("Candidate only | Dry-run by default | Signal: {}", signal.id)}
        }]
    })
}

fn gate(
    eligible: bool,
    would_send: bool,
    dry_run: bool,
    reason: &str,
    sent_at_ms: Option<i64>,
) -> AltContractDiscordGate {
    AltContractDiscordGate {
        eligible,
        would_send,
        sent: false,
        dry_run,
        reason: reason.to_string(),
        sent_at_ms,
    }
}

fn cooldown_key(signal: &AltContractSignal) -> AltContractDiscordCooldownKey {
    AltContractDiscordCooldownKey {
        symbol: signal.symbol.clone(),
        direction: signal.direction,
        signal_type: signal.signal_type,
    }
}

fn severity_label(severity: AltContractSeverity) -> &'static str {
    match severity {
        AltContractSeverity::S => "S",
        AltContractSeverity::Critical => "Critical",
        AltContractSeverity::High => "High",
        AltContractSeverity::Medium => "Medium",
        AltContractSeverity::Calm => "Calm",
    }
}

fn signal_type_label(signal_type: AltContractSignalType) -> &'static str {
    match signal_type {
        AltContractSignalType::MainForceLongBuild => "主力建多",
        AltContractSignalType::MainForceShortBuild => "主力建空",
        AltContractSignalType::AbnormalPump => "异常拉升",
        AltContractSignalType::AbnormalDump => "异常下跌",
        AltContractSignalType::DownsideAbsorption => "下方吸收",
        AltContractSignalType::UpsideResistance => "上方压制",
        AltContractSignalType::LiquidationCascade => "清算瀑布",
        AltContractSignalType::UnclearContractAnomaly => "合约异动待确认",
    }
}

fn direction_label(direction: AltContractDirection) -> &'static str {
    match direction {
        AltContractDirection::Buy => "主动买入",
        AltContractDirection::Sell => "主动卖出",
        AltContractDirection::Absorption => "下方吸收",
        AltContractDirection::Suppression => "上方压制",
        AltContractDirection::Neutral => "中性",
    }
}
