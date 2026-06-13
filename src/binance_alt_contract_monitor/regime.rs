use super::{
    detector::MarketImpulseContext,
    types::{
        AltContractContext, AltContractDirection, AltContractMarketRegime,
        AltContractMasterCapitalStrength, AltContractWindowConfirmation, AltContractWindowStats,
    },
};

pub fn classify_market_regime(
    stats: &AltContractWindowStats,
    context: &AltContractContext,
    mcss: &AltContractMasterCapitalStrength,
    window_confirmations: &[AltContractWindowConfirmation],
    market_context: &MarketImpulseContext,
) -> AltContractMarketRegime {
    let oi_trend = oi_trend(context);
    let price_trend = price_trend(stats.price_move_pct);
    let efficiency_ratio = efficiency_ratio(stats);
    let oi_lag_index = oi_lag_index(context, stats.price_move_pct);
    let liquidation_ratio = liquidation_ratio(stats.total_notional_usd, context);
    let multi_window_confirmed = multi_window_confirmed(window_confirmations, context);
    let dynamic_multiple = stats.dynamic_multiple.unwrap_or(0.0);
    let oi_mismatch = is_oi_mismatch(&oi_trend, stats.direction);
    let price_reversal = is_price_reversal(stats.direction, stats.price_move_pct);
    let price_trap = price_reversal && dynamic_multiple >= 6.0;
    let price_absorption = is_price_absorption(stats);
    let mut tags = common_tags(
        &oi_trend,
        &price_trend,
        dynamic_multiple,
        liquidation_ratio,
        price_absorption,
    );

    let manipulation_trigger = liquidation_ratio > 0.30 || dynamic_multiple >= 6.0 || price_trap;
    let manipulation_setup = mcss.mcss >= 70.0
        && manipulation_trigger
        && (oi_mismatch || liquidation_ratio > 0.30 || price_trap)
        && (!multi_window_confirmed || liquidation_ratio > 0.30 || price_trap);
    if manipulation_setup {
        let sub_type = manipulation_sub_type(stats.direction, liquidation_ratio, price_trap);
        tags.push(
            match sub_type.as_str() {
                "Manipulation_UP" => "fake_breakout",
                "Manipulation_DOWN" => "stop_hunt",
                "Liquidity_Trap" => "liquidity_trap",
                _ => "stop_hunt",
            }
            .to_string(),
        );
        let confidence = (62.0
            + (mcss.mcss - 70.0).max(0.0) * 0.35
            + if dynamic_multiple >= 6.0 { 8.0 } else { 0.0 }
            + if liquidation_ratio > 0.30 { 12.0 } else { 0.0 }
            + if price_trap { 10.0 } else { 0.0 })
        .clamp(0.0, 96.0);
        return regime(RegimeDraft {
            regime: "Manipulation",
            sub_type: Some(sub_type),
            confidence,
            mc_score: mcss.mcss,
            oi_trend,
            price_trend,
            efficiency_ratio,
            oi_lag_index,
            explanation_tags: tags,
        });
    }

    let accumulation_setup = (55.0..=85.0).contains(&mcss.mcss)
        && oi_trend == "up"
        && matches!(price_trend.as_str(), "flat" | "slow_up")
        && liquidation_ratio < 0.20
        && !market_context.market_wide_move
        && multi_window_confirmed
        && stats.dominance <= 0.72;
    if accumulation_setup {
        tags.push("smart_money_accumulating".to_string());
        if price_absorption {
            tags.push("price_absorption".to_string());
        }
        let confidence = (58.0
            + (mcss.mcss - 55.0) * 0.45
            + if price_absorption { 8.0 } else { 0.0 }
            + if stats.dominance <= 0.62 { 6.0 } else { 0.0 })
        .clamp(0.0, 92.0);
        return regime(RegimeDraft {
            regime: "Accumulation",
            confidence,
            sub_type: None,
            mc_score: mcss.mcss,
            oi_trend,
            price_trend,
            efficiency_ratio,
            oi_lag_index,
            explanation_tags: tags,
        });
    }

    let distribution_setup = (55.0..=85.0).contains(&mcss.mcss)
        && matches!(oi_trend.as_str(), "flat" | "down")
        && matches!(price_trend.as_str(), "slow_up" | "spike_up")
        && stats.total_notional_usd >= 500_000.0
        && (dynamic_multiple >= 2.0 || stats.dominance >= 0.60)
        && efficiency_ratio <= 0.35;
    if distribution_setup {
        tags.push("distribution_pressure".to_string());
        if stats.direction == AltContractDirection::Sell {
            tags.push("price_breakout_failed".to_string());
        }
        let confidence = (56.0
            + (mcss.mcss - 55.0) * 0.4
            + if oi_trend == "down" { 8.0 } else { 0.0 }
            + if stats.direction == AltContractDirection::Sell {
                6.0
            } else {
                0.0
            })
        .clamp(0.0, 90.0);
        return regime(RegimeDraft {
            regime: "Distribution",
            sub_type: None,
            confidence,
            mc_score: mcss.mcss,
            oi_trend,
            price_trend,
            efficiency_ratio,
            oi_lag_index,
            explanation_tags: tags,
        });
    }

    if same_direction_price(stats.direction, stats.price_move_pct) && mcss.mcss >= 55.0 {
        tags.push("trend_following".to_string());
    }

    regime(RegimeDraft {
        regime: "Unclear",
        sub_type: None,
        confidence: (mcss.mcss * 0.45).clamp(0.0, 55.0),
        mc_score: mcss.mcss,
        oi_trend,
        price_trend,
        efficiency_ratio,
        oi_lag_index,
        explanation_tags: tags,
    })
}

struct RegimeDraft {
    regime: &'static str,
    sub_type: Option<String>,
    confidence: f64,
    mc_score: f64,
    oi_trend: String,
    price_trend: String,
    efficiency_ratio: f64,
    oi_lag_index: f64,
    explanation_tags: Vec<String>,
}

fn regime(draft: RegimeDraft) -> AltContractMarketRegime {
    AltContractMarketRegime {
        regime: draft.regime.to_string(),
        sub_type: draft.sub_type,
        confidence: round2(draft.confidence),
        mc_score: round2(draft.mc_score),
        oi_trend: draft.oi_trend,
        price_trend: draft.price_trend.clone(),
        trend_5m: draft.price_trend.clone(),
        trend_15m: draft.price_trend,
        trend_1h: "unknown".to_string(),
        efficiency_ratio: round4(draft.efficiency_ratio),
        oi_lag_index: round4(draft.oi_lag_index),
        explanation_tags: draft.explanation_tags,
    }
}

fn oi_trend(context: &AltContractContext) -> String {
    let pct = context.oi_change_pct.unwrap_or(0.0);
    let base = context
        .oi_change_1m_base
        .or(context.oi_change_5m_base)
        .unwrap_or(0.0);
    if pct >= 0.5 || base > 0.0 {
        "up".to_string()
    } else if pct <= -0.5 || base < 0.0 {
        "down".to_string()
    } else {
        "flat".to_string()
    }
}

fn price_trend(price_move_pct: Option<f64>) -> String {
    let move_pct = price_move_pct.unwrap_or(0.0);
    if move_pct >= 0.80 {
        "spike_up".to_string()
    } else if move_pct >= 0.05 {
        "slow_up".to_string()
    } else if move_pct <= -0.80 {
        "spike_down".to_string()
    } else if move_pct <= -0.05 {
        "down".to_string()
    } else {
        "flat".to_string()
    }
}

fn efficiency_ratio(stats: &AltContractWindowStats) -> f64 {
    let notional_m = (stats.total_notional_usd / 1_000_000.0).max(0.001);
    stats.price_move_pct.unwrap_or(0.0).abs() / notional_m
}

fn oi_lag_index(context: &AltContractContext, price_move_pct: Option<f64>) -> f64 {
    let price = price_move_pct.unwrap_or(0.0).abs().max(0.01);
    context.oi_change_pct.unwrap_or(0.0).abs() / price
}

fn liquidation_ratio(total_notional_usd: f64, context: &AltContractContext) -> f64 {
    if total_notional_usd <= 0.0 {
        return 0.0;
    }
    context.liquidation_notional_usd.unwrap_or(0.0) / total_notional_usd
}

fn multi_window_confirmed(
    window_confirmations: &[AltContractWindowConfirmation],
    context: &AltContractContext,
) -> bool {
    window_confirmations
        .iter()
        .filter(|window| window.confirmed)
        .count()
        >= 2
        || context.persistence_windows >= 2
}

fn is_oi_mismatch(oi_trend: &str, direction: AltContractDirection) -> bool {
    match direction {
        AltContractDirection::Buy | AltContractDirection::Sell => oi_trend != "up",
        AltContractDirection::Absorption | AltContractDirection::Suppression => oi_trend == "down",
        AltContractDirection::Neutral => false,
    }
}

fn is_price_reversal(direction: AltContractDirection, price_move_pct: Option<f64>) -> bool {
    let price = price_move_pct.unwrap_or(0.0);
    match direction {
        AltContractDirection::Buy => price <= -0.15,
        AltContractDirection::Sell => price >= 0.15,
        _ => false,
    }
}

fn is_price_absorption(stats: &AltContractWindowStats) -> bool {
    let price = stats.price_move_pct.unwrap_or(0.0);
    match stats.direction {
        AltContractDirection::Sell => price > -0.05,
        AltContractDirection::Buy => price < 0.05,
        _ => false,
    }
}

fn same_direction_price(direction: AltContractDirection, price_move_pct: Option<f64>) -> bool {
    let price = price_move_pct.unwrap_or(0.0);
    match direction {
        AltContractDirection::Buy => price > 0.05,
        AltContractDirection::Sell => price < -0.05,
        _ => false,
    }
}

fn manipulation_sub_type(
    direction: AltContractDirection,
    liquidation_ratio: f64,
    price_reversal: bool,
) -> String {
    if liquidation_ratio > 0.40 {
        return match direction {
            AltContractDirection::Buy => "Manipulation_UP".to_string(),
            AltContractDirection::Sell => "Manipulation_DOWN".to_string(),
            _ => "Stop_Hunt".to_string(),
        };
    }
    if price_reversal {
        return "Liquidity_Trap".to_string();
    }
    match direction {
        AltContractDirection::Buy => "Manipulation_UP".to_string(),
        AltContractDirection::Sell => "Manipulation_DOWN".to_string(),
        _ => "Stop_Hunt".to_string(),
    }
}

fn common_tags(
    oi_trend: &str,
    price_trend: &str,
    dynamic_multiple: f64,
    liquidation_ratio: f64,
    price_absorption: bool,
) -> Vec<String> {
    let mut tags = Vec::new();
    if oi_trend == "up" {
        tags.push("oi_expanding".to_string());
    } else if oi_trend == "down" {
        tags.push("oi_contracting".to_string());
    }
    if price_absorption {
        tags.push("price_absorption".to_string());
    }
    if matches!(price_trend, "spike_up" | "spike_down") && dynamic_multiple >= 6.0 {
        tags.push("price_breakout_failed".to_string());
    }
    if liquidation_ratio > 0.30 {
        tags.push("stop_hunt".to_string());
    }
    tags
}

fn round2(value: f64) -> f64 {
    (value * 100.0).round() / 100.0
}

fn round4(value: f64) -> f64 {
    (value * 10_000.0).round() / 10_000.0
}
