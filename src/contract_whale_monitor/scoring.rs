use super::{
    config::{
        contract_whale_runtime_config, ContractWhaleRuntimeConfig, ContractWhaleThresholdProfile,
    },
    types::{
        ContractWhaleScoreBreakdown, ContractWhaleSeverity, ContractWhaleSignalType,
        ContractWhaleWindowStats,
    },
};

pub fn score_contract_whale_signal(
    stats: &ContractWhaleWindowStats,
    signal_type: ContractWhaleSignalType,
) -> u8 {
    score_contract_whale_signal_with_config(stats, signal_type, &contract_whale_runtime_config())
}

pub fn score_contract_whale_signal_with_config(
    stats: &ContractWhaleWindowStats,
    signal_type: ContractWhaleSignalType,
    config: &ContractWhaleRuntimeConfig,
) -> u8 {
    score_contract_whale_signal_with_profile(stats, signal_type, config, config.threshold_profile())
}

pub fn score_contract_whale_signal_with_profile(
    stats: &ContractWhaleWindowStats,
    signal_type: ContractWhaleSignalType,
    config: &ContractWhaleRuntimeConfig,
    profile: ContractWhaleThresholdProfile,
) -> u8 {
    score_contract_whale_breakdown_with_profile(stats, signal_type, config, profile)
        .final_score
        .round()
        .clamp(0.0, 100.0) as u8
}

pub fn score_contract_whale_breakdown_with_profile(
    stats: &ContractWhaleWindowStats,
    signal_type: ContractWhaleSignalType,
    config: &ContractWhaleRuntimeConfig,
    profile: ContractWhaleThresholdProfile,
) -> ContractWhaleScoreBreakdown {
    let thresholds =
        config.thresholds_for_symbol_window_with_profile(&stats.symbol, stats.window_sec, profile);
    let notional_thresholds = config.notional_thresholds_usd_for_profile(profile);
    let scoring = &config.scoring;
    let primary_source_extreme =
        primary_source_extreme_score_candidate(stats, config, thresholds, notional_thresholds);
    let volume_cap = scoring.volume_strength_weight * 0.70;
    let notional_cap = (scoring.volume_strength_weight - volume_cap).max(0.0);
    let volume_score = if thresholds.s_btc.is_finite() {
        (stats.total_volume_btc / thresholds.s_btc * volume_cap).clamp(0.0, volume_cap)
    } else {
        0.0
    };
    let notional_score = if notional_thresholds.s.is_finite() && notional_thresholds.s > 0.0 {
        (stats.total_notional_usd / notional_thresholds.s * notional_cap).clamp(0.0, notional_cap)
    } else {
        0.0
    };
    let dynamic_score = stats
        .dynamic_multiple
        .map(|multiple| {
            (multiple / 10.0 * scoring.dynamic_multiple_weight)
                .clamp(0.0, scoring.dynamic_multiple_weight)
        })
        .unwrap_or(0.0);
    let dominance_score = ((stats.dominance - 0.50) / 0.25 * scoring.dominance_weight)
        .clamp(0.0, scoring.dominance_weight);
    let price_score = price_impact_score(stats, signal_type, scoring.price_impact_weight);
    let exchange_score = match stats.exchange_count {
        0 => 0.0,
        1 => scoring.multi_exchange_weight * 0.4,
        2 => scoring.multi_exchange_weight * 0.8,
        _ => scoring.multi_exchange_weight,
    };
    let data_quality_score = (stats.data_quality as f64 / 100.0 * scoring.data_quality_weight)
        .clamp(0.0, scoring.data_quality_weight);
    let dominant_venue_score = dominant_venue_net_flow_adjustment(stats, thresholds.critical_btc);
    let oi_context_score = oi_context_adjustment(stats);
    let mut penalty_score = 0.0;

    if config.active_exchange_count() >= 2
        && stats.exchange_count == 1
        && stats.total_volume_btc >= thresholds.critical_btc
        && !primary_source_extreme
    {
        penalty_score += scoring.penalties.single_exchange_only;
    }
    if stats.liquidation_driven {
        penalty_score += scoring.penalties.liquidation_suspected;
    }
    if stats
        .ws_latency_ms
        .is_some_and(|latency| latency > config.data_quality.high_latency_ms)
    {
        penalty_score += scoring.penalties.websocket_latency_high;
    }
    if stats
        .startup_age_ms
        .is_some_and(|age| age < config.data_quality.warmup_ms)
    {
        penalty_score += scoring.penalties.warmup_period;
    }
    if stats.price_jump_anomaly {
        penalty_score += scoring.penalties.price_jump_anomaly;
    }

    let score = volume_score
        + notional_score
        + dynamic_score
        + dominance_score
        + price_score
        + exchange_score
        + data_quality_score
        + dominant_venue_score
        + oi_context_score
        - penalty_score;

    ContractWhaleScoreBreakdown {
        volume_score: round_score(volume_score),
        notional_score: round_score(notional_score),
        dynamic_anomaly_score: round_score(dynamic_score),
        directional_strength_score: round_score(dominance_score),
        price_response_score: round_score(price_score),
        multi_source_score: round_score(exchange_score),
        data_quality_score: round_score(data_quality_score),
        dominant_venue_score: round_score(dominant_venue_score),
        oi_context_score: round_score(oi_context_score),
        penalty_score: round_score(-penalty_score),
        final_score: round_score(score.clamp(0.0, 100.0)),
    }
}

fn oi_context_adjustment(stats: &ContractWhaleWindowStats) -> f64 {
    match stats.market_context.oi_change_pct {
        Some(change_pct) if change_pct >= 0.20 => 4.0,
        Some(change_pct) if change_pct <= -0.20 => -6.0,
        _ => 0.0,
    }
}

fn dominant_venue_net_flow_adjustment(stats: &ContractWhaleWindowStats, critical_btc: f64) -> f64 {
    let Some(share) = stats.dominant_venue_net_contribution_share else {
        return 0.0;
    };
    if !share.is_finite() || stats.net_volume_btc.abs() < critical_btc * 0.5 {
        return 0.0;
    }
    ((share - 0.70) / 0.30 * 5.0).clamp(0.0, 5.0)
}

fn primary_source_extreme_score_candidate(
    stats: &ContractWhaleWindowStats,
    config: &ContractWhaleRuntimeConfig,
    thresholds: super::types::ContractWhaleThresholds,
    notional_thresholds: super::config::ContractWhaleNotionalThresholds,
) -> bool {
    stats.dynamic_multiple.is_none()
        && stats.exchange_count == 1
        && stats.main_exchange.as_deref().is_some_and(|exchange| {
            config
                .primary_contract_exchanges()
                .iter()
                .any(|item| item == exchange)
        })
        && stats.total_notional_usd >= notional_thresholds.high
        && stats.dominance >= 0.60
        && stats.net_volume_btc.abs() >= (thresholds.high_btc * 0.40).max(500.0)
}

pub fn dominant_venue_net_flow_score_for_display(stats: &ContractWhaleWindowStats) -> f64 {
    dominant_venue_net_flow_adjustment(
        stats,
        contract_whale_runtime_config()
            .thresholds_for_symbol_window(&stats.symbol, stats.window_sec)
            .critical_btc,
    )
}

pub fn discord_gate(
    severity: ContractWhaleSeverity,
    score: u8,
    multi_exchange_confirmed: bool,
    data_quality: u8,
    primary_source_override: bool,
    symbol: &str,
) -> (bool, String) {
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
        ContractWhaleSeverity::Medium if is_btc_contract_symbol(symbol) => {
            (true, "btc_all_contract_signals_gate".to_string())
        }
        ContractWhaleSeverity::Medium | ContractWhaleSeverity::Calm => {
            (false, "medium_or_low_display_only".to_string())
        }
    }
}

fn is_btc_contract_symbol(symbol: &str) -> bool {
    let normalized = symbol
        .chars()
        .filter(|ch| ch.is_ascii_alphanumeric())
        .collect::<String>()
        .to_ascii_uppercase();
    matches!(normalized.as_str(), "BTC" | "BTCUSDT" | "BTCPERP")
}

fn price_impact_score(
    stats: &ContractWhaleWindowStats,
    signal_type: ContractWhaleSignalType,
    weight: f64,
) -> f64 {
    let Some(price_move_pct) = stats.price_move_pct else {
        return 0.0;
    };
    match signal_type {
        ContractWhaleSignalType::AggressiveBuy | ContractWhaleSignalType::AggressiveSell => {
            (price_move_pct.abs() / 0.25 * weight).clamp(0.0, weight)
        }
        ContractWhaleSignalType::DownsideAbsorption
        | ContractWhaleSignalType::UpsideSuppression => {
            if price_move_pct.abs() <= 0.05 {
                weight * 0.8
            } else {
                weight * 0.4
            }
        }
    }
}

fn round_score(value: f64) -> f64 {
    (value * 10.0).round() / 10.0
}
