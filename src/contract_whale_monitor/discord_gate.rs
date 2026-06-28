use crate::signal_semantics::SignalSemanticTier;

use super::{
    discord::is_btc_contract_symbol,
    types::{ContractWhaleSeverity, ContractWhaleSignal},
};

pub fn classify_contract_whale_signal_semantic(signal: &ContractWhaleSignal) -> SignalSemanticTier {
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
) -> (bool, String) {
    if !semantic_tier_for_contract_whale_severity(severity).allows_discord() {
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
        ContractWhaleSeverity::High if is_btc_contract_symbol(symbol) => {
            (true, "btc_high_gate".to_string())
        }
        ContractWhaleSeverity::High => (false, "high_without_discord_confirmation".to_string()),
        ContractWhaleSeverity::Medium | ContractWhaleSeverity::Calm => {
            (false, observe_reason(severity).to_string())
        }
    }
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
