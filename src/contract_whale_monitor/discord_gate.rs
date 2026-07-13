use crate::signal_semantics::SignalSemanticTier;

use super::{
    config::{contract_whale_runtime_config, ContractWhaleRuntimeConfig},
    discord::{is_btc_contract_symbol, meets_contract_whale_push_total_volume},
    types::{
        ContractWhalePriceResponseType, ContractWhaleSeverity, ContractWhaleSignal,
        ContractWhaleSignalType,
    },
};

pub fn btc_high_fallback_allowed(
    signal_type: ContractWhaleSignalType,
    price_response_type: ContractWhalePriceResponseType,
    score: u8,
) -> bool {
    score >= 70
        && matches!(
            (signal_type, price_response_type),
            (
                ContractWhaleSignalType::AggressiveBuy,
                ContractWhalePriceResponseType::TrendFollowUp
            ) | (
                ContractWhaleSignalType::AggressiveSell,
                ContractWhalePriceResponseType::TrendFollowDown
            )
        )
}

pub fn inferred_liquidation_display_only(
    liquidation_suspected: bool,
    liquidation_long_btc: f64,
    liquidation_short_btc: f64,
) -> bool {
    liquidation_suspected
        && liquidation_long_btc.max(0.0) + liquidation_short_btc.max(0.0) <= f64::EPSILON
}

pub fn classify_contract_whale_signal_semantic(signal: &ContractWhaleSignal) -> SignalSemanticTier {
    let config = contract_whale_runtime_config();
    if impact_level_discord_eligible(signal, &config) {
        return SignalSemanticTier::Alert;
    }
    semantic_tier_for_contract_whale_severity(signal.severity)
}

pub fn semantic_tier_for_contract_whale_severity(
    severity: ContractWhaleSeverity,
) -> SignalSemanticTier {
    match severity {
        ContractWhaleSeverity::Calm | ContractWhaleSeverity::Medium => SignalSemanticTier::Observe,
        ContractWhaleSeverity::High => SignalSemanticTier::Alert,
        ContractWhaleSeverity::Critical | ContractWhaleSeverity::S => SignalSemanticTier::Execution,
    }
}

pub fn discord_gate(
    severity: ContractWhaleSeverity,
    score: u8,
    multi_exchange_confirmed: bool,
    data_quality: u8,
    primary_source_override: bool,
    symbol: &str,
    total_volume_btc: f64,
    impact_level: Option<&str>,
    btc_high_fallback_allowed: bool,
    config: &ContractWhaleRuntimeConfig,
) -> (bool, String) {
    if !meets_contract_whale_push_total_volume(symbol, total_volume_btc) {
        return (false, "below_push_volume_threshold".to_string());
    }
    if !semantic_tier_for_contract_whale_severity(severity).allows_discord() {
        if config
            .discord
            .allows_impact_level(impact_level, data_quality)
        {
            return (true, "impact_level_gate".to_string());
        }
        return (false, observe_reason(severity).to_string());
    }
    if data_quality < 70 {
        return (false, "data_quality_display_only".to_string());
    }
    match severity {
        ContractWhaleSeverity::S | ContractWhaleSeverity::Critical => {
            (score >= 70, "critical_or_s_gate".to_string())
        }
        ContractWhaleSeverity::High if score >= 85 && multi_exchange_confirmed => {
            (true, "high_score_multi_exchange".to_string())
        }
        ContractWhaleSeverity::High if primary_source_override => {
            (true, "high_primary_source_extreme".to_string())
        }
        ContractWhaleSeverity::High
            if is_btc_contract_symbol(symbol) && btc_high_fallback_allowed =>
        {
            (true, "btc_high_gate".to_string())
        }
        ContractWhaleSeverity::High => (false, "high_without_discord_confirmation".to_string()),
        ContractWhaleSeverity::Medium | ContractWhaleSeverity::Calm => {
            (false, observe_reason(severity).to_string())
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn btc_high_fallback_requires_score_and_directional_price_follow_through() {
        assert!(btc_high_fallback_allowed(
            ContractWhaleSignalType::AggressiveBuy,
            ContractWhalePriceResponseType::TrendFollowUp,
            70,
        ));
        assert!(!btc_high_fallback_allowed(
            ContractWhaleSignalType::AggressiveBuy,
            ContractWhalePriceResponseType::TrendFollowUp,
            69,
        ));
        assert!(!btc_high_fallback_allowed(
            ContractWhaleSignalType::AggressiveBuy,
            ContractWhalePriceResponseType::NoClearResponse,
            90,
        ));
        assert!(!btc_high_fallback_allowed(
            ContractWhaleSignalType::DownsideAbsorption,
            ContractWhalePriceResponseType::DownsideAbsorption,
            90,
        ));
    }

    #[test]
    fn inferred_liquidation_is_display_only_but_live_liquidation_is_not() {
        assert!(inferred_liquidation_display_only(true, 0.0, 0.0));
        assert!(!inferred_liquidation_display_only(true, 1.0, 0.0));
        assert!(!inferred_liquidation_display_only(false, 0.0, 0.0));
    }
}

pub fn impact_level_discord_eligible(
    signal: &ContractWhaleSignal,
    config: &ContractWhaleRuntimeConfig,
) -> bool {
    if signal.discord_reason == "warmup_collect_only" {
        return false;
    }
    config
        .discord
        .allows_impact_level(signal.impact_level.as_deref(), signal.data_quality)
}

pub fn observe_reason(severity: ContractWhaleSeverity) -> &'static str {
    match severity {
        ContractWhaleSeverity::Medium => "medium_observe_only",
        ContractWhaleSeverity::Calm => "medium_or_low_display_only",
        ContractWhaleSeverity::High
        | ContractWhaleSeverity::Critical
        | ContractWhaleSeverity::S => "observe_only",
    }
}
