use super::{
    config::BinanceAltContractRuntimeConfig,
    types::{
        AltContractContext, AltContractDirection, AltContractScoreBreakdown, AltContractSymbolTier,
        AltContractWindowStats,
    },
};

#[derive(Debug, Clone)]
pub struct AltContractScoreResult {
    pub abnormal_score: u8,
    pub build_score: u8,
    pub direction_bias: i16,
    pub breakdown: AltContractScoreBreakdown,
}

pub fn score_alt_contract_signal(
    stats: &AltContractWindowStats,
    context: &AltContractContext,
    config: &BinanceAltContractRuntimeConfig,
) -> AltContractScoreResult {
    let thresholds = config.thresholds_for_tier(stats.tier);
    let volume_score =
        (stats.total_notional_usd / thresholds.s_notional_usd * 25.0).clamp(0.0, 25.0);
    let dynamic_score = stats
        .dynamic_multiple
        .map(|multiple| (multiple / config.dynamic.s_multiple * 20.0).clamp(0.0, 20.0))
        .unwrap_or_else(|| {
            if stats.total_notional_usd >= thresholds.critical_notional_usd {
                8.0
            } else {
                0.0
            }
        });
    let directional_score = ((stats.dominance - 0.45) / 0.35 * 15.0).clamp(0.0, 15.0);
    let oi_score = oi_score(stats, context);
    let price_score = price_score(stats);
    let liquidation_score = liquidation_score(context);
    let persistence_score = f64::from(context.persistence_windows.min(3)) / 3.0 * 5.0;
    let funding_score = funding_score(context);
    let funding_penalty = funding_crowding_penalty(stats, context);
    let data_quality_score = f64::from(stats.data_quality) / 100.0 * 5.0;
    let penalty_score = penalty_score(stats, context);

    let raw_abnormal = volume_score
        + dynamic_score
        + directional_score
        + price_score
        + liquidation_score
        + data_quality_score
        + penalty_score;
    let raw_build = volume_score * 0.65
        + dynamic_score * 0.65
        + directional_score
        + oi_score
        + price_score * 0.6
        + persistence_score
        + funding_score
        - funding_penalty
        + data_quality_score
        + build_penalty(context, penalty_score);

    let abnormal_score = raw_abnormal.round().clamp(0.0, 100.0) as u8;
    let build_score = raw_build.round().clamp(0.0, 100.0) as u8;
    AltContractScoreResult {
        abnormal_score,
        build_score,
        direction_bias: direction_bias(stats, context),
        breakdown: AltContractScoreBreakdown {
            volume_score: round(volume_score),
            dynamic_score: round(dynamic_score),
            directional_score: round(directional_score),
            oi_score: round(oi_score),
            price_score: round(price_score),
            liquidation_score: round(liquidation_score),
            persistence_score: round(persistence_score),
            funding_score: round(funding_score),
            data_quality_score: round(data_quality_score),
            penalty_score: round(penalty_score),
            abnormal_score: f64::from(abnormal_score),
            build_score: f64::from(build_score),
        },
    }
}

pub fn funding_crowding_label(
    stats: &AltContractWindowStats,
    context: &AltContractContext,
) -> String {
    let Some(rate) = context.funding_rate else {
        return "unknown".to_string();
    };
    let extreme_long = rate >= 0.001;
    let extreme_short = rate <= -0.001;
    match (stats.direction, extreme_long, extreme_short) {
        (AltContractDirection::Buy, true, _) => "long_overcrowded".to_string(),
        (AltContractDirection::Sell, _, true) => "short_overcrowded".to_string(),
        (AltContractDirection::Sell, true, _) => "anti_crowded_short_build".to_string(),
        (AltContractDirection::Buy, _, true) => "anti_crowded_long_build".to_string(),
        _ => "neutral".to_string(),
    }
}

pub fn funding_crowding_penalty(
    stats: &AltContractWindowStats,
    context: &AltContractContext,
) -> f64 {
    match funding_crowding_label(stats, context).as_str() {
        "long_overcrowded" | "short_overcrowded" => 8.0,
        _ => 0.0,
    }
}

fn oi_score(stats: &AltContractWindowStats, context: &AltContractContext) -> f64 {
    let oi_change = context.oi_change_1m_base.or(context.oi_change_5m_base);
    let Some(change) = oi_change else {
        return 0.0;
    };
    let direction_aligned = match stats.direction {
        AltContractDirection::Buy => change > 0.0,
        AltContractDirection::Sell => change > 0.0,
        _ => false,
    };
    let magnitude = context.oi_change_pct.unwrap_or(0.0).abs();
    let base = (magnitude / 1.5 * 15.0).clamp(0.0, 15.0);
    if direction_aligned {
        base
    } else if change < 0.0 {
        (base * 0.35).min(6.0)
    } else {
        0.0
    }
}

fn price_score(stats: &AltContractWindowStats) -> f64 {
    let price_move = stats
        .price_move_pct
        .or_else(|| stats.trigger_price_usd.map(|_| 0.0))
        .unwrap_or(0.0);
    let same_direction = match stats.direction {
        AltContractDirection::Buy => price_move.max(0.0),
        AltContractDirection::Sell => (-price_move).max(0.0),
        _ => price_move.abs(),
    };
    (same_direction / 0.8 * 10.0).clamp(0.0, 10.0)
}

fn liquidation_score(context: &AltContractContext) -> f64 {
    let notional = context.liquidation_notional_usd.unwrap_or(0.0);
    let score = (notional / 20_000_000.0 * 10.0).clamp(0.0, 10.0);
    if context.liquidation_suspected {
        score.max(6.0)
    } else {
        score
    }
}

fn funding_score(context: &AltContractContext) -> f64 {
    context
        .funding_rate
        .map(|rate| (rate.abs() / 0.001 * 5.0).clamp(0.0, 5.0))
        .unwrap_or(0.0)
}

fn penalty_score(stats: &AltContractWindowStats, context: &AltContractContext) -> f64 {
    let mut penalty = 0.0;
    if stats.data_quality < 70 {
        penalty -= 15.0;
    }
    if stats.startup_age_ms.is_some_and(|age| age < 60_000) {
        penalty -= 10.0;
    }
    if matches!(stats.tier, AltContractSymbolTier::D) {
        penalty -= 10.0;
    }
    if matches!(stats.tier, AltContractSymbolTier::E) {
        penalty -= 20.0;
    }
    if context.force_order_snapshot && context.liquidation_suspected {
        penalty -= 5.0;
    }
    penalty
}

fn build_penalty(context: &AltContractContext, base_penalty: f64) -> f64 {
    if context.liquidation_suspected {
        base_penalty - 18.0
    } else {
        base_penalty
    }
}

fn direction_bias(stats: &AltContractWindowStats, context: &AltContractContext) -> i16 {
    let direction = match stats.direction {
        AltContractDirection::Buy => 1.0,
        AltContractDirection::Sell => -1.0,
        AltContractDirection::Absorption => 0.25,
        AltContractDirection::Suppression => -0.25,
        AltContractDirection::Neutral => 0.0,
    };
    let price = stats.price_move_pct.unwrap_or(0.0).signum() * 0.2;
    let oi = context
        .oi_change_1m_base
        .or(context.oi_change_5m_base)
        .map(|value| if value >= 0.0 { 0.15 } else { -0.05 })
        .unwrap_or(0.0);
    ((direction * stats.dominance * 100.0) + price * 100.0 + oi * 100.0)
        .round()
        .clamp(-100.0, 100.0) as i16
}

fn round(value: f64) -> f64 {
    (value * 10.0).round() / 10.0
}
