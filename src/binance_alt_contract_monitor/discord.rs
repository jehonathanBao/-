use std::{
    collections::{BTreeMap, BTreeSet},
    sync::{Arc, OnceLock},
};

use parking_lot::RwLock;

use crate::normalizers::trade::now_ms;

use super::{
    config::{BinanceAltDiscordConfig, BinanceAltDiscordTierConfig},
    impact::{impact_discord_ready, is_legacy_impact_score, ALT_IMPACT_DISCORD_THRESHOLD},
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
    pub alert_kind: String,
    pub min_notional_usd: f64,
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
    sent_events: Vec<i64>,
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
    build_score: u8,
    sent_at_ms: i64,
}

impl AltContractDiscordCooldownStore {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn record_would_send(&self, signal: &AltContractSignal, sent_at_ms: i64) {
        let mut state = self.inner.write();
        state.sent_signal_ids.insert(signal.id.clone());
        state.sent_events.push(sent_at_ms);
        prune_sent_events(&mut state.sent_events, sent_at_ms);
        state.by_key.insert(
            cooldown_key(signal),
            AltContractDiscordCooldownEntry {
                severity: signal.severity,
                build_score: signal.build_score,
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
        let build_score_upgrade = signal.build_score >= entry.build_score.saturating_add(10);
        if within_cooldown
            && signal.severity.rank() <= entry.severity.rank()
            && !build_score_upgrade
        {
            return Some("cooldown");
        }
        None
    }

    fn hourly_cap_reached(
        &self,
        signal: &AltContractSignal,
        global_hourly_cap: usize,
        now_ms: i64,
    ) -> bool {
        let mut state = self.inner.write();
        prune_sent_events(&mut state.sent_events, now_ms);
        if global_hourly_cap == 0 || state.sent_events.len() < global_hourly_cap {
            return false;
        }
        !matches!(signal.severity, AltContractSeverity::S) && signal.build_score < 90
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
        return gate(false, false, config.dry_run, "disabled", "none", 0.0, None);
    }
    if warmup {
        return gate(false, false, config.dry_run, "warmup", "none", 0.0, None);
    }
    if signal.data_quality < config.min_data_quality {
        return gate(
            false,
            false,
            config.dry_run,
            "data_quality_low",
            "none",
            0.0,
            None,
        );
    }
    let Some(tier_config) = config.tier_thresholds.get(&signal.tier).copied() else {
        return gate(
            false,
            false,
            config.dry_run,
            "tier_config_missing",
            "none",
            0.0,
            None,
        );
    };
    if !is_legacy_impact_score(&signal.alt_impact_score)
        && !impact_discord_ready(&signal.alt_impact_score)
    {
        return gate(
            false,
            false,
            config.dry_run,
            "impact_score_low",
            "none",
            ALT_IMPACT_DISCORD_THRESHOLD,
            None,
        );
    }
    let display_floor = config.min_display_notional_usd.max(500_000.0);
    if is_legacy_impact_score(&signal.alt_impact_score) && signal.total_notional_usd < display_floor
    {
        return gate(
            false,
            false,
            config.dry_run,
            "low_display_notional",
            "none",
            display_floor,
            None,
        );
    }
    if !tier_config.enabled {
        return gate(
            false,
            false,
            config.dry_run,
            "tier_guard",
            "none",
            tier_config.min_notional_usd,
            None,
        );
    }
    if signal.market_wide_move
        && signal
            .relative_strength_rank
            .is_none_or(|rank| rank > config.market_wide_top_n)
    {
        return gate(
            false,
            false,
            config.dry_run,
            "market_wide_not_top",
            "market_wide_summary",
            tier_config.min_notional_usd,
            None,
        );
    }
    let decision = discord_decision(signal, config, &tier_config);
    if !decision.allowed {
        return gate(
            false,
            false,
            config.dry_run,
            decision.reason,
            decision.alert_kind,
            decision.min_notional_usd,
            None,
        );
    }
    if let Some(reason) = cooldown_store.skip_reason(signal, config.cooldown_sec, now) {
        return gate(
            false,
            false,
            config.dry_run,
            reason,
            decision.alert_kind,
            decision.min_notional_usd,
            None,
        );
    }
    if cooldown_store.hourly_cap_reached(signal, config.global_hourly_cap, now) {
        return gate(
            false,
            false,
            config.dry_run,
            "global_hourly_cap",
            decision.alert_kind,
            decision.min_notional_usd,
            None,
        );
    }
    if config.dry_run {
        cooldown_store.record_would_send(signal, now);
        return gate(
            true,
            true,
            true,
            "dry_run_would_send",
            decision.alert_kind,
            decision.min_notional_usd,
            None,
        );
    }
    let webhook_url = std::env::var(&config.webhook_env)
        .ok()
        .or_else(|| std::env::var("DISCORD_WEBHOOK_URL").ok());
    if webhook_url
        .as_deref()
        .map(|value| value.trim().is_empty())
        .unwrap_or(true)
    {
        return gate(
            false,
            false,
            false,
            "webhook_missing",
            decision.alert_kind,
            decision.min_notional_usd,
            None,
        );
    }
    gate(
        true,
        false,
        false,
        "live_send_not_enabled_for_bacm",
        decision.alert_kind,
        decision.min_notional_usd,
        None,
    )
}

pub fn build_alt_contract_discord_payload(signal: &AltContractSignal) -> serde_json::Value {
    let (title, judgement) = discord_copy(signal);
    serde_json::json!({
        "content": format!("{} {}", alert_prefix(signal), title),
        "embeds": [{
            "title": format!("{} {}", signal.symbol, title),
            "description": judgement,
            "fields": [
                {"name": "Symbol", "value": signal.symbol, "inline": true},
                {"name": "Type", "value": signal_type_label(signal.signal_type), "inline": true},
                {"name": "Direction", "value": direction_label(signal.direction), "inline": true},
                {"name": "Window", "value": format!("{}s", signal.window_sec), "inline": true},
                {"name": "Abnormal Score", "value": format!("{}/100", signal.abnormal_score), "inline": true},
                {"name": "Build Score", "value": format!("{}/100", signal.build_score), "inline": true},
                {"name": "Main Force Confidence", "value": format!("{:.0}/100", signal.main_force_confidence), "inline": true},
                {"name": "Evidence", "value": format!("{} items", signal.evidence_count), "inline": true},
                {"name": "Post Signal", "value": signal.post_signal_status.clone(), "inline": true},
                {"name": "Notional", "value": format!("${:.1}M", signal.total_notional_usd / 1_000_000.0), "inline": true},
                {"name": "AIS", "value": format!("{:.0}/100", signal.alt_impact_score.final_score), "inline": true},
                {"name": "Discord Gate", "value": format!("{} / {}", signal.discord_alert_kind, signal.discord_reason), "inline": true},
                {"name": "Dynamic Multiple", "value": signal.dynamic_multiple.map(|value| format!("{value:.1}x")).unwrap_or_else(|| "n/a".to_string()), "inline": true},
                {"name": "OI Change", "value": signal.oi_change_pct.map(|value| format!("{value:+.2}%")).unwrap_or_else(|| "n/a".to_string()), "inline": true},
                {"name": "Liquidation Driven", "value": if signal.liquidation_suspected { "yes" } else { "no" }, "inline": true},
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
    alert_kind: &str,
    min_notional_usd: f64,
    sent_at_ms: Option<i64>,
) -> AltContractDiscordGate {
    AltContractDiscordGate {
        eligible,
        would_send,
        sent: false,
        dry_run,
        reason: reason.to_string(),
        alert_kind: alert_kind.to_string(),
        min_notional_usd,
        sent_at_ms,
    }
}

#[derive(Debug, Clone, Copy)]
struct DiscordDecision {
    allowed: bool,
    reason: &'static str,
    alert_kind: &'static str,
    min_notional_usd: f64,
}

fn discord_decision(
    signal: &AltContractSignal,
    config: &BinanceAltDiscordConfig,
    tier_config: &BinanceAltDiscordTierConfig,
) -> DiscordDecision {
    if matches!(
        signal.severity,
        AltContractSeverity::Medium | AltContractSeverity::Calm
    ) {
        return decision(
            false,
            "medium_or_low",
            "display_only",
            tier_config.min_notional_usd,
        );
    }
    if tier_config.require_non_liquidation && signal.liquidation_suspected {
        return decision(
            false,
            "tier_requires_non_liquidation",
            "none",
            tier_config.min_notional_usd,
        );
    }
    if signal.severity == AltContractSeverity::S && !tier_config.s_enabled {
        return decision(false, "tier_s_disabled", "none", tier_config.s_notional_usd);
    }
    if let Some(decision) = main_force_decision(signal, config, tier_config) {
        return decision;
    }
    if let Some(decision) = liquidation_decision(signal, config, tier_config) {
        return decision;
    }
    if let Some(decision) = extreme_impulse_decision(signal, config, tier_config) {
        return decision;
    }
    decision(false, "low_score", "none", tier_config.min_notional_usd)
}

fn main_force_decision(
    signal: &AltContractSignal,
    config: &BinanceAltDiscordConfig,
    tier_config: &BinanceAltDiscordTierConfig,
) -> Option<DiscordDecision> {
    let is_long = signal.signal_type == AltContractSignalType::MainForceLongBuild
        && signal.direction == AltContractDirection::Buy;
    let is_short = signal.signal_type == AltContractSignalType::MainForceShortBuild
        && signal.direction == AltContractDirection::Sell;
    if !is_long && !is_short {
        return None;
    }
    let min_notional = tier_config
        .min_notional_usd
        .max(config.min_display_notional_usd);
    let oi_up = oi_expanding(signal);
    let funding_ok = !matches!(
        signal.funding_crowding.as_str(),
        "long_overcrowded" | "short_overcrowded"
    );
    if signal.total_notional_usd < min_notional && !impact_discord_ready(&signal.alt_impact_score) {
        return Some(decision(
            false,
            "tier_notional_low",
            "main_force_build",
            min_notional,
        ));
    }
    if signal.build_score < config.push_build_score.max(tier_config.require_build_score)
        || signal.main_force_confidence < f64::from(config.push_main_force_confidence)
        || signal.evidence_count < config.push_min_evidence_count
        || signal.dominance < 0.60
        || !oi_up
        || signal.oi_quality != "fresh"
        || signal.liquidation_suspected
        || !funding_ok
    {
        return Some(decision(
            false,
            "main_force_evidence_low",
            "main_force_build",
            min_notional,
        ));
    }
    Some(decision(
        true,
        "main_force_build",
        "main_force_build",
        min_notional,
    ))
}

fn extreme_impulse_decision(
    signal: &AltContractSignal,
    config: &BinanceAltDiscordConfig,
    tier_config: &BinanceAltDiscordTierConfig,
) -> Option<DiscordDecision> {
    let min_notional = tier_config
        .critical_notional_usd
        .max(config.min_display_notional_usd);
    if signal.liquidation_suspected {
        return None;
    }
    let build_confirmed = signal.build_score >= config.push_build_score
        && signal.evidence_count >= config.push_min_evidence_count;
    let price_move = signal.price_move_pct.unwrap_or_default().abs();
    if signal.abnormal_score
        >= config
            .push_abnormal_score
            .max(tier_config.require_abnormal_score)
        && signal.dynamic_multiple.unwrap_or_default() >= 8.0
        && signal.dominance >= 0.65
        && price_move >= 0.05
        && !build_confirmed
    {
        if signal.total_notional_usd < min_notional
            && !impact_discord_ready(&signal.alt_impact_score)
        {
            return Some(decision(
                false,
                "tier_critical_notional_low",
                "extreme_impulse",
                min_notional,
            ));
        }
        return Some(decision(
            true,
            "extreme_impulse",
            "extreme_impulse",
            min_notional,
        ));
    }
    None
}

fn liquidation_decision(
    signal: &AltContractSignal,
    config: &BinanceAltDiscordConfig,
    tier_config: &BinanceAltDiscordTierConfig,
) -> Option<DiscordDecision> {
    if !signal.liquidation_suspected {
        return None;
    }
    let min_notional = tier_config
        .critical_notional_usd
        .max(config.min_display_notional_usd);
    if !config.allow_liquidation_alerts {
        return Some(decision(
            false,
            "liquidation_alerts_disabled",
            "liquidation_shock",
            min_notional,
        ));
    }
    if signal.abnormal_score >= config.push_liquidation_abnormal_score
        && (signal.total_notional_usd >= min_notional
            || impact_discord_ready(&signal.alt_impact_score))
        && oi_contracting(signal)
        && signal.price_move_pct.unwrap_or_default().abs() >= 0.05
    {
        return Some(decision(
            true,
            "liquidation_shock",
            "liquidation_shock",
            min_notional,
        ));
    }
    Some(decision(
        false,
        "liquidation_evidence_low",
        "liquidation_shock",
        min_notional,
    ))
}

fn decision(
    allowed: bool,
    reason: &'static str,
    alert_kind: &'static str,
    min_notional_usd: f64,
) -> DiscordDecision {
    DiscordDecision {
        allowed,
        reason,
        alert_kind,
        min_notional_usd,
    }
}

fn oi_expanding(signal: &AltContractSignal) -> bool {
    signal
        .oi_change_pct
        .or(signal.oi_change_1m_pct)
        .or(signal.oi_change_5m_pct)
        .is_some_and(|value| value > 0.0)
        || signal
            .oi_change_1m_base
            .or(signal.oi_change_5m_base)
            .is_some_and(|value| value > 0.0)
}

fn oi_contracting(signal: &AltContractSignal) -> bool {
    signal
        .oi_change_pct
        .or(signal.oi_change_1m_pct)
        .or(signal.oi_change_5m_pct)
        .is_some_and(|value| value < 0.0)
        || signal
            .oi_change_1m_base
            .or(signal.oi_change_5m_base)
            .is_some_and(|value| value < 0.0)
}

fn prune_sent_events(events: &mut Vec<i64>, now_ms: i64) {
    let cutoff = now_ms.saturating_sub(60 * 60_000);
    events.retain(|sent_at| *sent_at >= cutoff);
}

fn cooldown_key(signal: &AltContractSignal) -> AltContractDiscordCooldownKey {
    AltContractDiscordCooldownKey {
        symbol: signal.symbol.clone(),
        direction: signal.direction,
        signal_type: signal.signal_type,
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

fn alert_prefix(signal: &AltContractSignal) -> &'static str {
    match signal.discord_alert_kind.as_str() {
        "main_force_build" => "🚨",
        "liquidation_shock" | "extreme_impulse" => "⚠️",
        _ => "⚠️",
    }
}

fn discord_copy(signal: &AltContractSignal) -> (&'static str, String) {
    match signal.discord_alert_kind.as_str() {
        "main_force_build" => match signal.signal_type {
            AltContractSignalType::MainForceLongBuild => (
                "Binance 山寨合约疑似主力建多",
                "主动买入异常放大，OI 同向上升，非清算驱动，疑似新多资金进场。".to_string(),
            ),
            AltContractSignalType::MainForceShortBuild => (
                "Binance 山寨合约疑似主力建空",
                "主动卖出异常放大，OI 同向上升，非清算驱动，疑似新空资金进场。".to_string(),
            ),
            _ => (
                "Binance 山寨合约疑似主力建仓",
                "主动流、OI 与证据链满足主力建仓 gate。".to_string(),
            ),
        },
        "liquidation_shock" => (
            "Binance 山寨合约清算冲击",
            "该异常主要由强平 / OI 下降推动，暂不判定为主力建仓。".to_string(),
        ),
        "market_wide_summary" => (
            "Binance 山寨合约集体异动",
            "山寨市场整体共振，单币主力建仓判断需结合相对强度。".to_string(),
        ),
        "extreme_impulse" => (
            "Binance 山寨合约极端异常冲击",
            "合约主动流异常冲击明显，但建仓证据不足，暂不判定为主力建仓。".to_string(),
        ),
        _ => (
            "Binance 山寨合约异动",
            "Candidate only，只读提醒，不代表自动交易或定性结论。".to_string(),
        ),
    }
}
