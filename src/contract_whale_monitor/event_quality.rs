use super::types::{ContractWhaleEventQuality, ContractWhaleSignal};

const QUALITY_PUBLISH_THRESHOLD: f64 = 0.6;

pub fn apply_contract_whale_event_quality_filter(
    signals: Vec<ContractWhaleSignal>,
) -> Vec<ContractWhaleSignal> {
    signals
        .into_iter()
        .filter_map(|mut signal| {
            signal.event_quality = score_contract_whale_event(&signal);
            if signal.event_quality.valid {
                Some(signal)
            } else {
                None
            }
        })
        .collect()
}

fn score_contract_whale_event(signal: &ContractWhaleSignal) -> ContractWhaleEventQuality {
    let false_event_flags = false_event_flags(signal);
    let quality_score = event_quality_score(signal);
    let merge_similarity_score = event_merge_similarity_score(signal);
    ContractWhaleEventQuality {
        quality_score,
        merge_similarity_score,
        valid: quality_score > QUALITY_PUBLISH_THRESHOLD && false_event_flags.is_empty(),
        false_event_flags,
    }
}

fn event_quality_score(signal: &ContractWhaleSignal) -> f64 {
    let volume_consistency: f64 =
        if signal.total_volume_btc >= 300.0 || signal.total_notional_usd >= 20_000_000.0 {
            0.30
        } else if signal.total_volume_btc >= 100.0 || signal.total_notional_usd >= 5_000_000.0 {
            0.20
        } else {
            0.05
        };
    let oi_consistency: f64 = if oi_abs(signal) > f64::EPSILON {
        0.20
    } else if signal.event_lifecycle.update_count > 1 {
        0.10
    } else {
        0.08
    };
    let price_move_abs = signal.price_move_pct.unwrap_or_default().abs();
    let price_impact: f64 = if price_move_abs >= 0.05 && signal.dominance >= 0.20 {
        0.25
    } else if price_move_abs > 0.0 || signal.dominance >= 0.35 {
        0.16
    } else {
        0.04
    };
    let event_duration_ms = event_duration_ms(signal);
    let duration_stability: f64 = if event_duration_ms >= 5_000
        || signal.event_lifecycle.update_count > 1
        || signal.window_sec >= 15
    {
        0.25
    } else {
        0.05
    };

    (volume_consistency + oi_consistency + price_impact + duration_stability).clamp(0.0, 1.0)
}

fn event_merge_similarity_score(signal: &ContractWhaleSignal) -> f64 {
    let symbol_match = 0.30;
    let type_match = 0.30;
    let time_overlap = if signal.event_lifecycle.update_count > 1 {
        0.20
    } else {
        0.0
    };
    let volume_pattern_similarity = (signal.dominance.clamp(0.0, 1.0) * 0.20).clamp(0.0, 0.20);
    (symbol_match + type_match + time_overlap + volume_pattern_similarity).clamp(0.0, 1.0)
}

fn false_event_flags(signal: &ContractWhaleSignal) -> Vec<String> {
    let mut flags = Vec::new();
    let event_duration_ms = event_duration_ms(signal);
    let oi_abs = oi_abs(signal);
    let price_move_abs = signal.price_move_pct.unwrap_or_default().abs();

    if event_duration_ms < 5_000
        && signal.window_sec <= 5
        && oi_abs <= f64::EPSILON
        && signal.total_volume_btc < 100.0
    {
        flags.push("MICRO_SPIKE".to_string());
    }

    if signal.window_sec <= 5
        && signal.event_lifecycle.update_count <= 1
        && signal.total_volume_btc >= 300.0
        && price_move_abs < 0.03
        && oi_abs <= f64::EPSILON
    {
        flags.push("SPOOF_NO_FOLLOW_THROUGH".to_string());
    }

    if price_move_abs >= 0.25 && signal.dominance < 0.15 {
        flags.push("LIQUIDITY_NOISE".to_string());
    }

    if !signal.symbol.eq_ignore_ascii_case("BTC")
        && price_move_abs >= 1.0
        && oi_abs <= f64::EPSILON
        && signal.funding_rate.unwrap_or_default().abs() < 0.0001
    {
        flags.push("FAKE_PUMP".to_string());
    }

    flags
}

fn event_duration_ms(signal: &ContractWhaleSignal) -> i64 {
    signal
        .event_lifecycle
        .last_update_time
        .saturating_sub(signal.event_lifecycle.start_time)
}

fn oi_abs(signal: &ContractWhaleSignal) -> f64 {
    let event_oi = signal.event_lifecycle.oi_accumulated.abs();
    if event_oi > f64::EPSILON {
        event_oi
    } else {
        signal
            .oi_change_1m_btc
            .or(signal.oi_change_5m_btc)
            .unwrap_or_default()
            .abs()
    }
}
