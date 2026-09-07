use super::{
    config::contract_whale_runtime_config,
    discord_gate::{classify_contract_whale_signal_semantic, impact_level_discord_eligible},
    types::{ContractWhaleSeverity, ContractWhaleSignal},
};

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
    if !meets_contract_whale_push_total_volume(
        contract_whale_gate_symbol(signal),
        effective_push_total_volume(signal),
    ) {
        return false;
    }
    if !classify_contract_whale_signal_semantic(signal).allows_discord() {
        return false;
    }
    matches!(
        signal.severity,
        ContractWhaleSeverity::Critical | ContractWhaleSeverity::S
    ) || (signal.severity == ContractWhaleSeverity::High && is_btc_contract_symbol(&signal.symbol))
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
