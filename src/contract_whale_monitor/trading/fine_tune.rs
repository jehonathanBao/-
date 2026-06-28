use crate::contract_whale_monitor::types::{
    ContractWhalePriceResponseType, ContractWhaleSignal, ContractWhaleSignalType,
};

use super::scoring::confidence_from_score;

pub fn adjusted_trade_score(
    signal: &ContractWhaleSignal,
    base_score: u8,
    regime_context: &str,
    liquidity_behavior: &str,
) -> u8 {
    let mut adjusted = f64::from(base_score);

    if is_chop_like_regime(regime_context) {
        adjusted *= chop_regime_multiplier(signal.signal_type);
    }
    if has_weak_follow_through(signal) {
        adjusted *= 0.78;
    }
    if fake_breakout_risk_high(signal, liquidity_behavior) {
        adjusted *= 0.50;
    }

    let stabilized = adjusted * 0.60 + f64::from(signal.score) * 0.40;
    stabilized.round().clamp(0.0, 100.0) as u8
}

pub fn fine_tune_reject_reason(
    signal: &ContractWhaleSignal,
    adjusted_score: u8,
    regime_context: &str,
    liquidity_behavior: &str,
) -> Option<&'static str> {
    if is_chop_like_regime(regime_context)
        && has_weak_follow_through(signal)
        && !signal.multi_exchange_confirmed
        && signal.event_lifecycle.update_count < 2
    {
        return Some("chop_regime_low_follow_through");
    }

    if high_volume_without_follow_through(signal) {
        return Some("high_volume_low_follow_through");
    }

    if fake_breakout_risk_high(signal, liquidity_behavior) && adjusted_score < 72 {
        return Some("fake_breakout_risk_high");
    }

    None
}

pub fn recalibrated_confidence(
    signal: &ContractWhaleSignal,
    adjusted_score: u8,
    liquidity_behavior: &str,
) -> u8 {
    let mut confidence = confidence_from_score(signal, adjusted_score);

    if !has_multi_window_confirmation(signal) {
        confidence = confidence.min(84);
    }
    if !has_price_follow_through(signal) {
        confidence = confidence.min(64);
    }
    if signal.event_lifecycle.update_count < 3 {
        confidence = confidence.min(79);
    }
    if fake_breakout_risk_high(signal, liquidity_behavior) {
        confidence = confidence.min(62);
    }

    confidence
}

pub fn setup_cluster_key(signal_type: ContractWhaleSignalType) -> &'static str {
    match signal_type {
        ContractWhaleSignalType::AggressiveBuy | ContractWhaleSignalType::AggressiveSell => {
            "trend_follow"
        }
        ContractWhaleSignalType::DownsideAbsorption
        | ContractWhaleSignalType::UpsideSuppression => "liquidity_reaction",
    }
}

pub fn should_prune_similar_setup(
    kept_signal: &ContractWhaleSignal,
    kept_direction: &str,
    kept_score: u8,
    candidate_signal: &ContractWhaleSignal,
    candidate_direction: &str,
    candidate_score: u8,
) -> bool {
    kept_direction == candidate_direction
        && setup_cluster_key(kept_signal.signal_type)
            == setup_cluster_key(candidate_signal.signal_type)
        && score_gap(kept_score, candidate_score) < 5
}

fn score_gap(left: u8, right: u8) -> u8 {
    left.abs_diff(right)
}

fn is_chop_like_regime(regime_context: &str) -> bool {
    let regime = regime_context.to_ascii_lowercase();
    regime.contains("ranging") || regime.contains("range") || regime.contains("chop")
}

fn chop_regime_multiplier(signal_type: ContractWhaleSignalType) -> f64 {
    match signal_type {
        ContractWhaleSignalType::AggressiveBuy | ContractWhaleSignalType::AggressiveSell => 0.72,
        ContractWhaleSignalType::DownsideAbsorption
        | ContractWhaleSignalType::UpsideSuppression => 0.92,
    }
}

fn high_volume_without_follow_through(signal: &ContractWhaleSignal) -> bool {
    signal.total_notional_usd >= 60_000_000.0 && has_weak_follow_through(signal)
}

fn has_weak_follow_through(signal: &ContractWhaleSignal) -> bool {
    signal.price_move_pct.unwrap_or_default().abs() < 0.08
}

fn has_price_follow_through(signal: &ContractWhaleSignal) -> bool {
    signal.price_move_pct.unwrap_or_default().abs() >= 0.10
        && !matches!(
            signal.price_response_type,
            ContractWhalePriceResponseType::NoClearResponse
        )
}

fn has_multi_window_confirmation(signal: &ContractWhaleSignal) -> bool {
    !signal.merged_from.is_empty()
}

fn fake_breakout_risk_high(signal: &ContractWhaleSignal, liquidity_behavior: &str) -> bool {
    liquidity_behavior == "fake_breakout"
        || (matches!(
            signal.signal_type,
            ContractWhaleSignalType::UpsideSuppression | ContractWhaleSignalType::AggressiveBuy
        ) && matches!(
            signal.price_response_type,
            ContractWhalePriceResponseType::NoClearResponse
                | ContractWhalePriceResponseType::UpsideResistance
        ))
}
