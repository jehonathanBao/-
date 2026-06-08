use super::types::{ContractWhaleSeverity, ContractWhaleSignal};

pub fn build_contract_whale_discord_preview(signal: &ContractWhaleSignal) -> serde_json::Value {
    serde_json::json!({
        "symbol": signal.symbol,
        "eventType": "contract_whale_flow",
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
    matches!(
        signal.severity,
        ContractWhaleSeverity::Critical | ContractWhaleSeverity::S
    ) || (signal.severity == ContractWhaleSeverity::High
        && signal.score >= 85
        && signal
            .exchanges
            .iter()
            .filter(|item| item.total_volume_btc > 0.0)
            .count()
            >= 2)
}
