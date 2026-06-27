use crate::contract_whale_monitor::types::{
    ContractWhaleDirection, ContractWhalePriceResponseType, ContractWhaleSignal,
};

pub fn score_signal_strength(signal: &ContractWhaleSignal) -> u8 {
    let volume_strength = ((signal.total_volume_btc / 5_000.0) * 25.0).clamp(0.0, 25.0);
    let price_strength = match signal.price_response_type {
        ContractWhalePriceResponseType::TrendFollowUp
        | ContractWhalePriceResponseType::TrendFollowDown => 18.0
            + (signal.price_move_pct.unwrap_or_default().abs() / 0.60 * 7.0).clamp(0.0, 7.0),
        ContractWhalePriceResponseType::DownsideAbsorption
        | ContractWhalePriceResponseType::UpsideResistance => 14.0
            + (signal.price_move_pct.unwrap_or_default().abs() / 0.25 * 6.0).clamp(0.0, 6.0),
        ContractWhalePriceResponseType::NoClearResponse => {
            (signal.price_move_pct.unwrap_or_default().abs() / 0.20 * 10.0).clamp(0.0, 10.0)
        }
    }
    .clamp(0.0, 25.0);
    let dominance_strength = (signal.dominance.abs() * 20.0).clamp(0.0, 20.0);
    let persistence_strength = ((signal.event_lifecycle.update_count as f64).min(5.0) / 5.0
        * 15.0)
        .clamp(0.0, 15.0);
    let consistency_strength = consistency_points(signal).clamp(0.0, 15.0);

    (volume_strength
        + price_strength
        + dominance_strength
        + persistence_strength
        + consistency_strength)
        .round()
        .clamp(0.0, 100.0) as u8
}

pub fn strength_label(score: u8) -> &'static str {
    if score > 80 {
        "HIGH"
    } else if score >= 60 {
        "MEDIUM"
    } else {
        "LOW"
    }
}

pub fn direction_bias_label(direction: ContractWhaleDirection) -> &'static str {
    match direction {
        ContractWhaleDirection::Buy => "BUY",
        ContractWhaleDirection::Sell => "SELL",
        ContractWhaleDirection::Absorption => "ABSORPTION",
        ContractWhaleDirection::Suppression => "SUPPRESSION",
    }
}

fn consistency_points(signal: &ContractWhaleSignal) -> f64 {
    let mut score = 0.0;
    if signal.multi_exchange_confirmed {
        score += 7.0;
    }
    if !signal.merged_from.is_empty() {
        score += 4.0;
    }
    if signal.event_quality.quality_score >= 0.75 {
        score += 2.0;
    }
    if signal.main_force_score.unwrap_or(signal.score) >= 80 {
        score += 2.0;
    }
    score
}
