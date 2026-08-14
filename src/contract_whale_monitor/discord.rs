use super::{
    behavior::is_behavior_alert_eligible,
    config::contract_whale_runtime_config,
    discord_gate::{classify_contract_whale_signal_semantic, impact_level_discord_eligible},
    types::{ContractWhaleSeverity, ContractWhaleSignal},
};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ContractWhaleNotificationLane {
    Behavior,
    Impact,
    Observe,
}

pub fn notification_lane(signal: &ContractWhaleSignal) -> ContractWhaleNotificationLane {
    if is_behavior_alert_eligible(&signal.behavior_assessment) && signal.multi_exchange_confirmed {
        return ContractWhaleNotificationLane::Behavior;
    }
    if impact_notification_eligible(signal) {
        return ContractWhaleNotificationLane::Impact;
    }
    ContractWhaleNotificationLane::Observe
}

pub fn impact_notification_eligible(signal: &ContractWhaleSignal) -> bool {
    if is_s_grade(signal) && !is_historic_s_impact(signal) {
        return false;
    }
    let config = contract_whale_runtime_config();
    impact_level_discord_eligible(signal, &config)
        || signal.discord_reason == "high_primary_source_extreme"
        || signal.discord_reason == "btc_high_gate"
        || (signal.liquidation_suspected
            && signal.liquidation_long_btc.max(0.0) + signal.liquidation_short_btc.max(0.0) >= 50.0)
}

fn is_s_grade(signal: &ContractWhaleSignal) -> bool {
    signal.severity == ContractWhaleSeverity::S
        || signal.impact_level.as_deref() == Some("S")
        || signal.signal_level.as_deref() == Some("S")
}

/// S 级只保留可复核的极端冲击：大规模实时清算或跨市场特大换手。
/// 普通高分、单窗口放量和单一交易所异常不会越级成为 S 级事件。
pub fn is_historic_s_impact(signal: &ContractWhaleSignal) -> bool {
    let liquidation_btc =
        signal.liquidation_long_btc.max(0.0) + signal.liquidation_short_btc.max(0.0);
    let liquidation_sweep = signal.liquidation_suspected && liquidation_btc >= 2_500.0;
    let extraordinary_turnover = signal.total_volume_btc >= 20_000.0
        && signal.window_sec >= 60
        && signal.multi_exchange_confirmed
        && signal.dynamic_multiple.unwrap_or(0.0) >= 10.0
        && signal.percentile_level.unwrap_or(0.0) >= 99.5
        && signal.dominance >= 0.65;
    liquidation_sweep || extraordinary_turnover
}

/// Normalizes the user-facing impact lane after the raw percentile calculation.
/// The raw score remains available for diagnostics, but an S label is never
/// exposed without replayable hard evidence.
pub fn sanitize_contract_whale_impact(signal: &mut ContractWhaleSignal) {
    let raw_s =
        signal.impact_level.as_deref() == Some("S") || signal.signal_level.as_deref() == Some("S");
    if raw_s && !is_historic_s_impact(signal) {
        signal.impact_level = Some("A".to_string());
        signal.signal_level = Some("L3".to_string());
        signal.signal_label = Some("HIGH IMPACT EVENT".to_string());
    }
}

const BTC_MIN_PUSH_TOTAL_VOLUME_BTC: f64 = 500.0;
const ETH_MIN_PUSH_TOTAL_VOLUME_BTC: f64 = 30_000.0;
const BTC_MIN_DISPLAY_TOTAL_VOLUME_BTC: f64 = 500.0;

pub fn build_contract_whale_discord_preview(signal: &ContractWhaleSignal) -> serde_json::Value {
    serde_json::json!({
        "symbol": signal.symbol,
        "eventType": "contract_whale_flow",
        "legacySignalType": signal.classification_v2.legacy_signal_type,
        "displaySignalType": signal.classification_v2.display_signal_type,
        "structureInterpretation": signal.classification_v2.structure_interpretation,
        "classificationVersion": signal.classification_v2.classification_version,
        "semanticMismatch": signal.classification_v2.semantic_mismatch,
        "severity": signal.severity,
        "direction": signal.direction,
        "windowSec": signal.window_sec,
        "finalResult": signal.final_result,
        "score": signal.score,
        "dataQuality": signal.data_quality,
        "discordEligible": signal.discord_eligible,
        "discordSent": signal.discord_sent,
        "totalVolumeBtc": signal.total_volume_btc,
        "totalNotionalUsd": signal.total_notional_usd,
        "netVolumeBtc": signal.net_volume_btc,
        "dominance": signal.dominance,
        "priceMovePct": signal.price_move_pct,
        "oiChange1mBtc": signal.oi_change_1m_btc,
        "oiChange5mBtc": signal.oi_change_5m_btc,
        "oiChangePct": signal.oi_change_pct,
        "oiBias": signal.oi_bias,
        "fundingRate": signal.funding_rate,
        "fundingBias": signal.funding_bias,
        "mainExchange": signal.main_exchange,
        "readOnly": true,
        "analysisOnly": true,
        "executionEnabled": false
    })
}

pub fn should_push_contract_whale_discord(signal: &ContractWhaleSignal) -> bool {
    let min_push_volume =
        contract_whale_min_push_total_volume_btc(contract_whale_gate_symbol(signal)).unwrap_or(0.0);
    if signal.severity == ContractWhaleSeverity::High
        && signal.score < 70
        && signal.total_volume_btc > min_push_volume + f64::EPSILON
        && signal.event_lifecycle.volume_accumulated <= signal.total_volume_btc + f64::EPSILON
    {
        return false;
    }
    if matches!(
        signal.severity,
        ContractWhaleSeverity::Critical | ContractWhaleSeverity::S
    ) && !is_historic_s_impact(signal)
        && !is_behavior_alert_eligible(&signal.behavior_assessment)
    {
        return false;
    }
    if !meets_contract_whale_push_total_volume(
        contract_whale_gate_symbol(signal),
        effective_push_total_volume(signal),
    ) {
        return false;
    }
    let lane = notification_lane(signal);
    if lane == ContractWhaleNotificationLane::Observe {
        return false;
    }
    if lane == ContractWhaleNotificationLane::Impact {
        return true;
    }
    if !classify_contract_whale_signal_semantic(signal).allows_discord() {
        return false;
    }
    matches!(
        signal.severity,
        ContractWhaleSeverity::Critical | ContractWhaleSeverity::S
    ) || (signal.severity == ContractWhaleSeverity::High
        && is_btc_contract_symbol(&signal.symbol)
        && (signal.score >= 70
            || signal.event_lifecycle.volume_accumulated > signal.total_volume_btc + f64::EPSILON))
        || (signal.severity == ContractWhaleSeverity::High
            && ((signal.score >= 85
                && signal
                    .exchanges
                    .iter()
                    .filter(|item| item.total_volume_btc > 0.0)
                    .count()
                    >= 2)
                || signal.discord_reason == "high_primary_source_extreme"))
        || impact_level_discord_eligible(signal, &contract_whale_runtime_config())
}

pub(crate) fn contract_whale_gate_symbol(signal: &ContractWhaleSignal) -> &str {
    if !signal.quantity_unit.trim().is_empty() {
        signal.quantity_unit.as_str()
    } else if !signal.base_asset.trim().is_empty() {
        signal.base_asset.as_str()
    } else {
        signal.symbol.as_str()
    }
}

pub(crate) fn effective_push_total_volume(signal: &ContractWhaleSignal) -> f64 {
    if is_btc_contract_symbol(contract_whale_gate_symbol(signal))
        && signal.event_lifecycle.volume_accumulated > f64::EPSILON
    {
        signal.event_lifecycle.volume_accumulated
    } else {
        signal.total_volume_btc
    }
}

pub fn contract_whale_min_push_total_volume_btc(symbol: &str) -> Option<f64> {
    if is_btc_contract_symbol(symbol) {
        Some(BTC_MIN_PUSH_TOTAL_VOLUME_BTC)
    } else if is_eth_contract_symbol(symbol) {
        Some(ETH_MIN_PUSH_TOTAL_VOLUME_BTC)
    } else {
        None
    }
}

pub fn contract_whale_min_display_total_volume_btc(symbol: &str) -> Option<f64> {
    is_btc_contract_symbol(symbol).then_some(BTC_MIN_DISPLAY_TOTAL_VOLUME_BTC)
}

pub fn meets_contract_whale_push_total_volume(symbol: &str, total_volume_btc: f64) -> bool {
    contract_whale_min_push_total_volume_btc(symbol)
        .is_none_or(|threshold| total_volume_btc >= threshold)
}

pub fn meets_contract_whale_display_total_volume(symbol: &str, total_volume_btc: f64) -> bool {
    contract_whale_min_display_total_volume_btc(symbol)
        .is_none_or(|threshold| total_volume_btc >= threshold)
}

pub fn is_btc_contract_symbol(symbol: &str) -> bool {
    matches!(
        normalized_contract_symbol(symbol).as_str(),
        "BTC" | "BTCUSDT" | "BTCPERP"
    )
}

pub fn is_eth_contract_symbol(symbol: &str) -> bool {
    matches!(
        normalized_contract_symbol(symbol).as_str(),
        "ETH" | "ETHUSDT" | "ETHPERP"
    )
}

fn normalized_contract_symbol(symbol: &str) -> String {
    let normalized = symbol
        .chars()
        .filter(|ch| ch.is_ascii_alphanumeric())
        .collect::<String>()
        .to_ascii_uppercase();
    normalized
}
