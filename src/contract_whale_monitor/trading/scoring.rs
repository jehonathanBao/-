use crate::contract_whale_monitor::types::{ContractWhalePriceResponseType, ContractWhaleSignal};

pub fn score_signal(signal: &ContractWhaleSignal) -> u8 {
    let volume_strength = (f64::from(signal.main_force_score.unwrap_or(signal.score)) / 100.0
        * 25.0)
        .clamp(0.0, 25.0);
    let price_response = price_response_points(signal.price_response_type, signal.price_move_pct);
    let dominance = (signal.dominance.clamp(0.0, 1.0) * 20.0).clamp(0.0, 20.0);
    let persistence = ((signal.event_lifecycle.update_count.max(1) as f64) * 5.0).clamp(5.0, 15.0);
    let consistency = if signal.multi_exchange_confirmed && !signal.merged_from.is_empty() {
        15.0
    } else if signal.multi_exchange_confirmed || !signal.merged_from.is_empty() {
        10.0
    } else {
        4.0
    };
    (volume_strength + price_response + dominance + persistence + consistency)
        .round()
        .clamp(0.0, 100.0) as u8
}

pub fn confidence_from_score(signal: &ContractWhaleSignal, score: u8) -> u8 {
    let quality = (signal.event_quality.quality_score.clamp(0.0, 1.0) * 100.0).round();
    (((score as f64) * 0.7) + quality * 0.3)
        .round()
        .clamp(0.0, 100.0) as u8
}

pub fn confidence_label(confidence: u8) -> &'static str {
    match confidence {
        85..=u8::MAX => "HIGH",
        70..=84 => "MEDIUM",
        55..=69 => "WATCH",
        _ => "LOW",
    }
}

fn price_response_points(
    response_type: ContractWhalePriceResponseType,
    price_move_pct: Option<f64>,
) -> f64 {
    let response_base = match response_type {
        ContractWhalePriceResponseType::TrendFollowUp
        | ContractWhalePriceResponseType::TrendFollowDown => 21.0,
        ContractWhalePriceResponseType::DownsideAbsorption
        | ContractWhalePriceResponseType::UpsideResistance => 17.0,
        ContractWhalePriceResponseType::NoClearResponse => 4.0,
    };
    let move_bonus = price_move_pct
        .map(|value| value.abs().clamp(0.0, 0.60) / 0.60 * 4.0)
        .unwrap_or(0.0);
    (response_base + move_bonus).clamp(0.0, 25.0)
}
