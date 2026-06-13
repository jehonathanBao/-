use super::types::{
    AltContractContext, AltContractDirection, AltContractMarketTier,
    AltContractMasterCapitalStrength, AltContractSymbolTier, AltContractWindowStats,
};

pub fn score_master_capital_strength(
    stats: &AltContractWindowStats,
    context: &AltContractContext,
    market_tier: AltContractMarketTier,
) -> AltContractMasterCapitalStrength {
    let liquidity_weight = liquidity_weight(market_tier, stats.tier);
    let tier = mcss_tier_label(market_tier, stats.tier).to_string();
    let liquidity_score = 8.0 * liquidity_weight;
    let notional_score = notional_strength_score(stats.total_notional_usd, liquidity_weight);
    let direction_score = direction_strength_score(stats.dominance);
    let oi_score = oi_confirmation_score(stats.direction, context);
    let price_score = price_response_score(stats.direction, stats.price_move_pct);
    let anomaly_score = dynamic_anomaly_score(stats.dynamic_multiple);
    let liquidation_penalty = liquidation_penalty(stats.total_notional_usd, context);

    let mut mcss =
        liquidity_score + notional_score + direction_score + oi_score + price_score + anomaly_score
            - liquidation_penalty;

    if oi_score <= 0.0 || price_score < 0.0 {
        mcss = mcss.min(69.0);
    }
    if liquidation_penalty >= 25.0 {
        mcss = mcss.min(69.0);
    } else if liquidation_penalty >= 15.0 {
        mcss = mcss.min(79.0);
    }

    let mcss = round2(mcss.clamp(0.0, 100.0));
    AltContractMasterCapitalStrength {
        mcss,
        tier,
        liquidity_weight: round2(liquidity_weight),
        notional_score: round2(notional_score),
        direction_score: round2(direction_score),
        oi_score: round2(oi_score),
        price_score: round2(price_score),
        anomaly_score: round2(anomaly_score),
        liquidation_penalty: round2(liquidation_penalty),
        interpretation: interpretation(mcss).to_string(),
    }
}

fn liquidity_weight(market_tier: AltContractMarketTier, symbol_tier: AltContractSymbolTier) -> f64 {
    match (market_tier, symbol_tier) {
        (AltContractMarketTier::UltraCore, _) => 0.6,
        (AltContractMarketTier::Mainstream, _) => 1.0,
        (AltContractMarketTier::Alt, AltContractSymbolTier::D | AltContractSymbolTier::E) => 1.8,
        (AltContractMarketTier::Alt, _) => 1.5,
    }
}

fn mcss_tier_label(
    market_tier: AltContractMarketTier,
    symbol_tier: AltContractSymbolTier,
) -> &'static str {
    match (market_tier, symbol_tier) {
        (AltContractMarketTier::UltraCore, _) => "Ultra Core",
        (AltContractMarketTier::Mainstream, _) => "Mainstream",
        (AltContractMarketTier::Alt, AltContractSymbolTier::D | AltContractSymbolTier::E) => {
            "Micro Alt"
        }
        (AltContractMarketTier::Alt, _) => "Alt",
    }
}

fn notional_strength_score(notional_usd: f64, liquidity_weight: f64) -> f64 {
    if notional_usd <= 0.0 {
        return 0.0;
    }
    ((notional_usd.log10_1p() - 4.0).max(0.0) * 8.0 * liquidity_weight).clamp(0.0, 22.0)
}

fn direction_strength_score(dominance: f64) -> f64 {
    if dominance >= 0.70 {
        25.0
    } else if dominance >= 0.60 {
        15.0
    } else if dominance >= 0.50 {
        5.0
    } else {
        0.0
    }
}

fn oi_confirmation_score(direction: AltContractDirection, context: &AltContractContext) -> f64 {
    let Some(change) = context.oi_change_1m_base.or(context.oi_change_5m_base) else {
        return 0.0;
    };
    if change < 0.0 {
        return -10.0;
    }
    match direction {
        AltContractDirection::Buy | AltContractDirection::Sell => 25.0,
        AltContractDirection::Absorption | AltContractDirection::Suppression => 10.0,
        AltContractDirection::Neutral => 0.0,
    }
}

fn price_response_score(direction: AltContractDirection, price_move_pct: Option<f64>) -> f64 {
    let price_move = price_move_pct.unwrap_or(0.0);
    if price_move.abs() < 0.05 {
        return 5.0;
    }
    let same_direction = match direction {
        AltContractDirection::Buy => price_move > 0.0,
        AltContractDirection::Sell => price_move < 0.0,
        AltContractDirection::Absorption => price_move >= -0.05,
        AltContractDirection::Suppression => price_move <= 0.05,
        AltContractDirection::Neutral => false,
    };
    if same_direction {
        20.0
    } else {
        -15.0
    }
}

fn dynamic_anomaly_score(dynamic_multiple: Option<f64>) -> f64 {
    let multiple = dynamic_multiple.unwrap_or(0.0);
    if multiple >= 6.0 {
        20.0
    } else if multiple >= 4.0 {
        15.0
    } else if multiple >= 2.0 {
        10.0
    } else {
        0.0
    }
}

fn liquidation_penalty(total_notional_usd: f64, context: &AltContractContext) -> f64 {
    let liquidation_notional = context.liquidation_notional_usd.unwrap_or(0.0);
    if total_notional_usd <= 0.0 {
        return if context.liquidation_suspected {
            15.0
        } else {
            0.0
        };
    }
    let ratio = liquidation_notional / total_notional_usd;
    if ratio >= 0.40 {
        25.0
    } else if ratio >= 0.20 || context.liquidation_suspected {
        15.0
    } else {
        0.0
    }
}

fn interpretation(mcss: f64) -> &'static str {
    if mcss < 30.0 {
        "无意义流动"
    } else if mcss < 50.0 {
        "弱资金流入"
    } else if mcss < 70.0 {
        "短线资金活跃"
    } else if mcss < 85.0 {
        "主力资金介入"
    } else if mcss < 95.0 {
        "疑似机构级别流入"
    } else {
        "智能资金/趋势起点"
    }
}

fn round2(value: f64) -> f64 {
    (value * 100.0).round() / 100.0
}

trait Log10OnePlus {
    fn log10_1p(self) -> f64;
}

impl Log10OnePlus for f64 {
    fn log10_1p(self) -> f64 {
        (1.0 + self).log10()
    }
}
