use super::config::BinanceAltImpactConfig;
use super::types::{
    AltContractContext, AltContractDirection, AltContractImpactScore, AltContractMarketTier,
    AltContractSymbolTier, AltContractWindowStats,
};

pub const ALT_IMPACT_DISPLAY_THRESHOLD: f64 = 70.0;
pub const ALT_IMPACT_DISCORD_THRESHOLD: f64 = 85.0;
pub const ALT_IMPACT_S_THRESHOLD: f64 = 90.0;

pub fn score_alt_impact(
    stats: &AltContractWindowStats,
    context: &AltContractContext,
    market_tier: AltContractMarketTier,
) -> AltContractImpactScore {
    score_alt_impact_with_config(
        stats,
        context,
        market_tier,
        &BinanceAltImpactConfig::default(),
    )
}

pub fn score_alt_impact_with_config(
    stats: &AltContractWindowStats,
    context: &AltContractContext,
    market_tier: AltContractMarketTier,
    config: &BinanceAltImpactConfig,
) -> AltContractImpactScore {
    let (reference_volume_24h_usd, reference_source, reference_age_sec) =
        reference_volume_24h_with_config(stats, context, config);
    let market_impact_ratio = reference_volume_24h_usd
        .filter(|value| *value > 0.0)
        .map(|reference| stats.total_notional_usd / reference)
        .unwrap_or_default()
        .max(0.0);
    let market_impact_score = market_impact_score(market_impact_ratio, market_tier);
    let liquidity_impact = liquidity_impact_score(stats, market_tier, market_impact_ratio);
    let directional_score = directional_score(stats.dominance);
    let oi_confirmation = oi_confirmation_score(stats, context);
    let cap_impact = 0.0;
    let final_score =
        (market_impact_score + liquidity_impact + directional_score + oi_confirmation)
            .clamp(0.0, 100.0);

    AltContractImpactScore {
        market_impact_ratio: round4(market_impact_ratio),
        market_impact_score: round2(market_impact_score),
        liquidity_impact: round2(liquidity_impact),
        cap_impact,
        directional_strength: round4(stats.dominance),
        directional_score: round2(directional_score),
        oi_confirmation: round2(oi_confirmation),
        final_score: round2(final_score),
        display_threshold: ALT_IMPACT_DISPLAY_THRESHOLD,
        discord_threshold: ALT_IMPACT_DISCORD_THRESHOLD,
        s_threshold: ALT_IMPACT_S_THRESHOLD,
        reference_volume_24h_usd: reference_volume_24h_usd.map(round2),
        reference_age_sec,
        evidence_degraded: reference_volume_24h_usd.is_none() && config.require_reliable_reference,
        reference_source,
        interpretation: interpretation(final_score),
    }
}

pub fn impact_displayable(score: &AltContractImpactScore) -> bool {
    !score.evidence_degraded
        && score.final_score >= score.display_threshold.max(ALT_IMPACT_DISPLAY_THRESHOLD)
}

pub fn impact_discord_ready(score: &AltContractImpactScore) -> bool {
    !score.evidence_degraded
        && score.final_score >= score.discord_threshold.max(ALT_IMPACT_DISCORD_THRESHOLD)
}

pub fn impact_discord_level(score: &AltContractImpactScore) -> Option<&'static str> {
    if impact_s_ready(score) {
        Some("S")
    } else if impact_discord_ready(score) {
        Some("A")
    } else {
        None
    }
}

pub fn impact_s_ready(score: &AltContractImpactScore) -> bool {
    !score.evidence_degraded && score.final_score >= score.s_threshold.max(ALT_IMPACT_S_THRESHOLD)
}

pub fn is_legacy_impact_score(score: &AltContractImpactScore) -> bool {
    score.final_score <= 0.0
        && score.market_impact_ratio <= 0.0
        && score.market_impact_score <= 0.0
        && score.liquidity_impact <= 0.0
        && score.directional_score <= 0.0
        && score.oi_confirmation <= 0.0
}

fn reference_volume_24h_with_config(
    stats: &AltContractWindowStats,
    context: &AltContractContext,
    config: &BinanceAltImpactConfig,
) -> (Option<f64>, String, Option<u64>) {
    if let Some(value) = context
        .ticker_quote_volume_24h_usd
        .filter(|value| value.is_finite() && *value > 0.0)
    {
        let age_sec = context
            .ticker_updated_at
            .map(|updated_at| stats.ts.saturating_sub(updated_at).max(0) as u64 / 1_000);
        let fresh = age_sec.is_some_and(|age| age <= config.ticker_max_age_seconds);
        if fresh {
            return (Some(value), "ticker_quote_volume_24h".to_string(), age_sec);
        }
    }

    if let Some(value) = context
        .local_rolling_24h_notional_usd
        .filter(|value| value.is_finite() && *value > 0.0)
    {
        let age_sec = context
            .local_rolling_24h_updated_at
            .map(|updated_at| stats.ts.saturating_sub(updated_at).max(0) as u64 / 1_000);
        if age_sec.is_some_and(|age| age <= 24 * 60 * 60) {
            return (Some(value), "local_rolling_24h".to_string(), age_sec);
        }
    }

    if let Some(value) = context
        .historical_baseline_notional_usd
        .filter(|value| value.is_finite() && *value > 0.0)
    {
        let age_sec = context
            .historical_baseline_updated_at
            .map(|updated_at| stats.ts.saturating_sub(updated_at).max(0) as u64 / 1_000);
        if age_sec.is_some_and(|age| age <= 24 * 60 * 60) {
            return (Some(value), "historical_baseline".to_string(), age_sec);
        }
    }

    (None, "unavailable".to_string(), None)
}

fn market_impact_score(ratio: f64, market_tier: AltContractMarketTier) -> f64 {
    let tier_weight = match market_tier {
        AltContractMarketTier::UltraCore => 0.90,
        AltContractMarketTier::Mainstream => 1.00,
        AltContractMarketTier::Alt => 1.20,
    };
    let raw = if ratio >= 0.03 {
        40.0
    } else if ratio >= 0.01 {
        24.0 + ((ratio - 0.01) / 0.02 * 16.0)
    } else if ratio >= 0.003 {
        10.0 + ((ratio - 0.003) / 0.007 * 14.0)
    } else {
        (ratio / 0.003 * 10.0).clamp(0.0, 10.0)
    };
    (raw * tier_weight).clamp(0.0, 40.0)
}

fn liquidity_impact_score(
    stats: &AltContractWindowStats,
    market_tier: AltContractMarketTier,
    market_impact_ratio: f64,
) -> f64 {
    let dynamic = stats.dynamic_multiple.unwrap_or_default().max(0.0);
    let dynamic_score = if dynamic >= 6.0 {
        24.0 + ((dynamic - 6.0) / 6.0 * 6.0).clamp(0.0, 6.0)
    } else if dynamic >= 4.0 {
        16.0 + ((dynamic - 4.0) / 2.0 * 8.0)
    } else if dynamic >= 2.0 {
        8.0 + ((dynamic - 2.0) / 2.0 * 8.0)
    } else if dynamic > 0.0 {
        (dynamic / 2.0 * 8.0).clamp(0.0, 8.0)
    } else if market_impact_ratio >= 0.03 {
        24.0
    } else if market_impact_ratio >= 0.01 {
        16.0 + ((market_impact_ratio - 0.01) / 0.02 * 8.0)
    } else if market_impact_ratio >= 0.003 {
        8.0 + ((market_impact_ratio - 0.003) / 0.007 * 8.0)
    } else {
        (market_impact_ratio / 0.003 * 8.0).clamp(0.0, 8.0)
    };
    let tier_bonus = match (market_tier, stats.tier) {
        (AltContractMarketTier::Alt, AltContractSymbolTier::D | AltContractSymbolTier::E) => 4.0,
        (AltContractMarketTier::Alt, _) => 2.0,
        _ => 0.0,
    };
    (dynamic_score + tier_bonus).clamp(0.0, 30.0)
}

fn directional_score(dominance: f64) -> f64 {
    if dominance >= 0.70 {
        20.0
    } else if dominance >= 0.60 {
        12.0 + ((dominance - 0.60) / 0.10 * 8.0)
    } else if dominance >= 0.50 {
        5.0 + ((dominance - 0.50) / 0.10 * 7.0)
    } else {
        (dominance / 0.50 * 5.0).clamp(0.0, 5.0)
    }
}

fn oi_confirmation_score(stats: &AltContractWindowStats, context: &AltContractContext) -> f64 {
    let oi_base = context.oi_change_1m_base.or(context.oi_change_5m_base);
    let pct = context.oi_change_pct.unwrap_or_default();
    let direction_supported = match stats.direction {
        AltContractDirection::Buy | AltContractDirection::Sell => {
            oi_base.is_some_and(|value| value > 0.0)
        }
        _ => false,
    };
    if direction_supported && pct > 1.0 {
        10.0
    } else if direction_supported {
        6.0
    } else if oi_base.is_some_and(|value| value < 0.0) {
        -4.0
    } else {
        0.0
    }
}

fn interpretation(score: f64) -> String {
    if score >= 90.0 {
        "极强相对成交冲击，可能影响该币市场结构".to_string()
    } else if score >= 85.0 {
        "强相对成交冲击，可进入 Discord gate 观察".to_string()
    } else if score >= 70.0 {
        "有效相对冲击，适合前端展示".to_string()
    } else if score >= 50.0 {
        "资金活跃但相对冲击不足".to_string()
    } else {
        "相对市场冲击偏弱".to_string()
    }
}

fn round2(value: f64) -> f64 {
    (value * 100.0).round() / 100.0
}

fn round4(value: f64) -> f64 {
    (value * 10_000.0).round() / 10_000.0
}
