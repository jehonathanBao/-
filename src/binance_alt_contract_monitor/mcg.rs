use super::types::{
    AltContractContext, AltContractControlEdge, AltContractControlNode, AltContractDirection,
    AltContractLiquidityMicrostructure, AltContractMarketControlGraph, AltContractMarketRegime,
    AltContractMasterCapitalStrength, AltContractSmartMoneyLifecycle, AltContractWindowStats,
};

pub fn build_market_control_graph(
    stats: &AltContractWindowStats,
    context: &AltContractContext,
    microstructure: &AltContractLiquidityMicrostructure,
    mcss: &AltContractMasterCapitalStrength,
    regime: &AltContractMarketRegime,
    lifecycle: &AltContractSmartMoneyLifecycle,
) -> AltContractMarketControlGraph {
    let dominant_side = dominant_side(stats, microstructure);
    let absorption_power = absorption_power(stats, context, microstructure);
    let imbalance_control = microstructure.imbalance_score.clamp(0.0, 100.0);
    let liquidity_shaping = liquidity_shaping(stats, microstructure, regime);
    let price_control_efficiency = price_control_efficiency(stats, context, microstructure);
    let spoofing_signals = microstructure.spoofing_penalty.clamp(0.0, 100.0);
    let control_strength = (0.30 * absorption_power
        + 0.25 * imbalance_control
        + 0.20 * liquidity_shaping
        + 0.15 * price_control_efficiency
        + 0.10 * spoofing_signals)
        .clamp(0.0, 100.0);
    let control_type = control_type(
        stats,
        context,
        microstructure,
        regime,
        lifecycle,
        absorption_power,
        liquidity_shaping,
        spoofing_signals,
    );
    let control_nodes = control_nodes(
        stats,
        &dominant_side,
        &control_type,
        control_strength,
        absorption_power,
        liquidity_shaping,
    );
    let control_edges = control_edges(
        &control_nodes,
        &control_type,
        absorption_power,
        liquidity_shaping,
        spoofing_signals,
        microstructure,
    );

    AltContractMarketControlGraph {
        symbol: stats.symbol.clone(),
        control_nodes,
        control_edges,
        dominant_side,
        control_strength: round2(control_strength),
        control_type: control_type.clone(),
        control_path: control_path(&control_type),
        interpretation: interpretation(
            &control_type,
            control_strength,
            mcss.mcss,
            microstructure.lms_score,
        ),
        read_only: true,
        direct_discord_gate: false,
    }
}

fn dominant_side(
    stats: &AltContractWindowStats,
    microstructure: &AltContractLiquidityMicrostructure,
) -> String {
    match microstructure.market_control.as_str() {
        "buyer_side_control" => "buy".to_string(),
        "seller_side_control" => "sell".to_string(),
        "two_sided_absorption" => "neutral".to_string(),
        "fake_liquidity_control" => {
            if stats.net_volume_base > 0.0 {
                "buy".to_string()
            } else if stats.net_volume_base < 0.0 {
                "sell".to_string()
            } else {
                "neutral".to_string()
            }
        }
        _ => match stats.direction {
            AltContractDirection::Buy => "buy".to_string(),
            AltContractDirection::Sell => "sell".to_string(),
            _ => "neutral".to_string(),
        },
    }
}

fn absorption_power(
    stats: &AltContractWindowStats,
    context: &AltContractContext,
    microstructure: &AltContractLiquidityMicrostructure,
) -> f64 {
    let oi_expanding = context.oi_change_pct.unwrap_or_default() > 0.8;
    let flat_price = stats.price_move_pct.unwrap_or_default().abs() <= 0.08;
    let absorption_bonus = if flat_price && oi_expanding {
        25.0
    } else {
        0.0
    };
    (microstructure.absorption_strength + absorption_bonus).clamp(0.0, 100.0)
}

fn liquidity_shaping(
    stats: &AltContractWindowStats,
    microstructure: &AltContractLiquidityMicrostructure,
    regime: &AltContractMarketRegime,
) -> f64 {
    let behavior_bonus = if matches!(
        microstructure.behavior.as_str(),
        "LiquiditySweepUp" | "LiquiditySweepDown" | "LiquidityPullUp" | "LiquidityPullDown"
    ) {
        20.0
    } else {
        0.0
    };
    let manipulation_bonus = if regime.regime.eq_ignore_ascii_case("Manipulation") {
        15.0
    } else {
        0.0
    };
    (microstructure
        .order_flow_pressure
        .max(microstructure.spread_behavior)
        .max(stats.dominance * 100.0)
        + behavior_bonus
        + manipulation_bonus)
        .clamp(0.0, 100.0)
}

fn price_control_efficiency(
    stats: &AltContractWindowStats,
    context: &AltContractContext,
    microstructure: &AltContractLiquidityMicrostructure,
) -> f64 {
    let price_move = stats.price_move_pct.unwrap_or_default().abs();
    let oi_support = context.oi_change_pct.unwrap_or_default().abs();
    if microstructure.absorption_strength >= 60.0 && price_move <= 0.08 {
        return (70.0 + oi_support * 10.0).clamp(0.0, 100.0);
    }
    if microstructure.order_flow_pressure >= 70.0 && price_move >= 0.2 {
        return (65.0 + stats.dominance * 25.0).clamp(0.0, 100.0);
    }
    (stats.dominance * 55.0 + oi_support * 8.0).clamp(0.0, 100.0)
}

fn control_type(
    stats: &AltContractWindowStats,
    context: &AltContractContext,
    microstructure: &AltContractLiquidityMicrostructure,
    regime: &AltContractMarketRegime,
    lifecycle: &AltContractSmartMoneyLifecycle,
    absorption_power: f64,
    liquidity_shaping: f64,
    spoofing_signals: f64,
) -> String {
    let oi = context.oi_change_pct.unwrap_or_default();
    let price_move = stats.price_move_pct.unwrap_or_default();
    let regime_key = regime.regime.to_ascii_lowercase();
    let lifecycle_key = lifecycle.lifecycle_state.to_ascii_lowercase();
    let manipulation = spoofing_signals >= 60.0
        || microstructure.behavior.starts_with("LiquiditySweep")
        || regime_key == "manipulation"
        || (liquidity_shaping >= 75.0 && oi <= 0.3);
    if manipulation {
        return "ControlManipulation".to_string();
    }
    if absorption_power >= 60.0 && oi > 0.8 && price_move.abs() <= 0.12 {
        return "ControlAccumulation".to_string();
    }
    if (microstructure.behavior == "Absorption_Sell" || regime_key == "distribution")
        && price_move > 0.0
        && oi <= 0.5
    {
        return "ControlDistribution".to_string();
    }
    if lifecycle_key == "accumulation" || lifecycle_key == "reaccumulation" {
        return "ControlAccumulation".to_string();
    }
    if lifecycle_key == "distribution" {
        return "ControlDistribution".to_string();
    }
    "NoClearControl".to_string()
}

fn control_nodes(
    stats: &AltContractWindowStats,
    dominant_side: &str,
    control_type: &str,
    control_strength: f64,
    absorption_power: f64,
    liquidity_shaping: f64,
) -> Vec<AltContractControlNode> {
    let price = stats.trigger_price_usd;
    vec![
        AltContractControlNode {
            id: format!("{}:symbol", stats.product_id),
            node_type: "Symbol".to_string(),
            label: stats.product_id.clone(),
            side: dominant_side.to_string(),
            strength: round2(control_strength),
            price,
        },
        AltContractControlNode {
            id: format!("{}:price-zone", stats.product_id),
            node_type: "PriceLevel".to_string(),
            label: price
                .map(|value| format!("control zone {:.6}", value))
                .unwrap_or_else(|| "control zone unknown".to_string()),
            side: dominant_side.to_string(),
            strength: round2((control_strength + liquidity_shaping) / 2.0),
            price,
        },
        AltContractControlNode {
            id: format!("{}:liquidity-zone", stats.product_id),
            node_type: "LiquidityZone".to_string(),
            label: liquidity_zone_label(control_type),
            side: dominant_side.to_string(),
            strength: round2(absorption_power.max(liquidity_shaping)),
            price,
        },
    ]
}

fn control_edges(
    nodes: &[AltContractControlNode],
    control_type: &str,
    absorption_power: f64,
    liquidity_shaping: f64,
    spoofing_signals: f64,
    microstructure: &AltContractLiquidityMicrostructure,
) -> Vec<AltContractControlEdge> {
    if nodes.len() < 3 {
        return Vec::new();
    }
    let relation = match control_type {
        "ControlAccumulation" => "absorption_relation",
        "ControlDistribution" => "pressure_flow",
        "ControlManipulation" => "manipulation_relation",
        _ => "control_relation",
    };
    vec![
        AltContractControlEdge {
            from: nodes[0].id.clone(),
            to: nodes[1].id.clone(),
            relation: relation.to_string(),
            strength: round2(liquidity_shaping.max(spoofing_signals)),
            evidence: vec![
                microstructure.behavior.clone(),
                microstructure.market_control.clone(),
            ],
        },
        AltContractControlEdge {
            from: nodes[1].id.clone(),
            to: nodes[2].id.clone(),
            relation: "liquidity_transfer".to_string(),
            strength: round2(absorption_power.max(microstructure.imbalance_score)),
            evidence: microstructure.explanation_tags.clone(),
        },
    ]
}

fn liquidity_zone_label(control_type: &str) -> String {
    match control_type {
        "ControlAccumulation" => "下方流动性吸收区".to_string(),
        "ControlDistribution" => "上方派发/压制区".to_string(),
        "ControlManipulation" => "诱导/扫单流动性区".to_string(),
        _ => "普通流动性区".to_string(),
    }
}

fn control_path(control_type: &str) -> Vec<String> {
    match control_type {
        "ControlAccumulation" => vec![
            "Bid absorption".to_string(),
            "Price containment".to_string(),
            "Potential markup preparation".to_string(),
        ],
        "ControlDistribution" => vec![
            "Ask absorption".to_string(),
            "Breakout suppression".to_string(),
            "Liquidity exit".to_string(),
        ],
        "ControlManipulation" => vec![
            "Liquidity shaping".to_string(),
            "Cognitive trap".to_string(),
            "Sweep or revert risk".to_string(),
        ],
        _ => vec!["No stable control path".to_string()],
    }
}

fn interpretation(control_type: &str, css: f64, mcss: f64, lms: f64) -> String {
    let strength = if css >= 80.0 {
        "强控盘"
    } else if css >= 60.0 {
        "明显控制"
    } else if css >= 40.0 {
        "弱控制"
    } else {
        "无明确控盘"
    };
    let label = match control_type {
        "ControlAccumulation" => "控制吸筹",
        "ControlDistribution" => "控制派发",
        "ControlManipulation" => "操控/诱导",
        _ => "控制关系未确认",
    };
    format!(
        "{label} · {strength} · MCSS {:.0}/100 · LMS {:.0}/100",
        mcss, lms
    )
}

fn round2(value: f64) -> f64 {
    (value * 100.0).round() / 100.0
}
