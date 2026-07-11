use super::types::{
    AltContractContext, AltContractDirection, AltContractLiquidityMicrostructure,
    AltContractWindowStats,
};

#[derive(Debug, Clone, Default)]
pub struct LiquidityMicrostructureInput {
    pub aggressive_buy_notional_usd: f64,
    pub aggressive_sell_notional_usd: f64,
    pub price_move_pct: f64,
    pub bid_depth_1pct_usd: Option<f64>,
    pub ask_depth_1pct_usd: Option<f64>,
    pub previous_bid_depth_1pct_usd: Option<f64>,
    pub previous_ask_depth_1pct_usd: Option<f64>,
    pub spread_bps: Option<f64>,
    pub previous_spread_bps: Option<f64>,
    pub large_order_add_usd: Option<f64>,
    pub large_order_cancel_usd: Option<f64>,
    pub large_order_executed_usd: Option<f64>,
    pub replenishment_ratio: Option<f64>,
}

pub fn score_signal_microstructure(
    stats: &AltContractWindowStats,
    _context: &AltContractContext,
) -> AltContractLiquidityMicrostructure {
    let buy_ratio = ((1.0 + stats.dominance.clamp(0.0, 1.0)) / 2.0).clamp(0.0, 1.0);
    let (buy_notional, sell_notional) = match stats.direction {
        AltContractDirection::Buy => (
            stats.total_notional_usd * buy_ratio,
            stats.total_notional_usd * (1.0 - buy_ratio),
        ),
        AltContractDirection::Sell => (
            stats.total_notional_usd * (1.0 - buy_ratio),
            stats.total_notional_usd * buy_ratio,
        ),
        _ => (
            stats.total_notional_usd * 0.5,
            stats.total_notional_usd * 0.5,
        ),
    };
    score_liquidity_microstructure(&LiquidityMicrostructureInput {
        aggressive_buy_notional_usd: buy_notional,
        aggressive_sell_notional_usd: sell_notional,
        price_move_pct: stats.price_move_pct.unwrap_or_default(),
        ..LiquidityMicrostructureInput::default()
    })
}

pub fn score_liquidity_microstructure(
    input: &LiquidityMicrostructureInput,
) -> AltContractLiquidityMicrostructure {
    let total_flow = input.aggressive_buy_notional_usd + input.aggressive_sell_notional_usd;
    let net_flow = input.aggressive_buy_notional_usd - input.aggressive_sell_notional_usd;
    let direction = if net_flow > 0.0 {
        "buy"
    } else if net_flow < 0.0 {
        "sell"
    } else {
        "neutral"
    };
    let directional_strength = if total_flow > 0.0 {
        (net_flow.abs() / total_flow).clamp(0.0, 1.0)
    } else {
        0.0
    };
    let depth_imbalance = depth_imbalance(input);
    let depth_drop = depth_drop(input);
    let spread_widen = spread_widen_ratio(input);
    let orderbook_evidence_available = has_orderbook_evidence(input);
    let spoofing_penalty = if orderbook_evidence_available {
        spoofing_score(input)
    } else {
        0.0
    };
    let absorption_strength = absorption_score(input, total_flow, directional_strength);
    let order_flow_pressure = order_flow_pressure_score(input, directional_strength);
    let imbalance_score = (depth_imbalance.abs() * 100.0).clamp(0.0, 100.0);
    let spread_behavior = spread_score(input, spread_widen, depth_drop);
    let lms_score = (0.25 * order_flow_pressure
        + 0.25 * absorption_strength
        + 0.20 * imbalance_score
        + 0.15 * spread_behavior
        + 0.15 * spoofing_penalty)
        .clamp(0.0, 100.0);
    let behavior = classify_behavior(
        input,
        direction,
        order_flow_pressure,
        absorption_strength,
        spoofing_penalty,
        imbalance_score,
        spread_behavior,
    );
    let market_control = market_control(direction, &behavior, lms_score);
    let explanation_tags = explanation_tags(
        &behavior,
        direction,
        depth_imbalance,
        spread_widen,
        spoofing_penalty,
        absorption_strength,
        order_flow_pressure,
    );

    AltContractLiquidityMicrostructure {
        evidence_mode: if orderbook_evidence_available {
            "orderbook".to_string()
        } else {
            "flow_only".to_string()
        },
        orderbook_evidence_available,
        lms_score: round2(lms_score),
        behavior,
        market_control,
        liquidity_pressure: pressure_label(lms_score),
        imbalance: round4(depth_imbalance),
        spread_state: spread_state(input, spread_widen),
        spoofing_state: if spoofing_penalty >= 70.0 {
            "detected".to_string()
        } else if spoofing_penalty >= 40.0 {
            "watch".to_string()
        } else {
            "none".to_string()
        },
        order_flow_pressure: round2(order_flow_pressure),
        absorption_strength: round2(absorption_strength),
        imbalance_score: round2(imbalance_score),
        spread_behavior: round2(spread_behavior),
        spoofing_penalty: round2(spoofing_penalty),
        explanation_tags,
        interpretation: interpretation(lms_score),
        read_only: true,
        direct_discord_gate: false,
    }
}

fn has_orderbook_evidence(input: &LiquidityMicrostructureInput) -> bool {
    input.bid_depth_1pct_usd.is_some()
        || input.ask_depth_1pct_usd.is_some()
        || input.previous_bid_depth_1pct_usd.is_some()
        || input.previous_ask_depth_1pct_usd.is_some()
        || input.spread_bps.is_some()
        || input.previous_spread_bps.is_some()
        || input.large_order_add_usd.is_some()
        || input.large_order_cancel_usd.is_some()
        || input.large_order_executed_usd.is_some()
        || input.replenishment_ratio.is_some()
}

fn order_flow_pressure_score(
    input: &LiquidityMicrostructureInput,
    directional_strength: f64,
) -> f64 {
    let price_follow = (input.price_move_pct > 0.05
        && input.aggressive_buy_notional_usd > input.aggressive_sell_notional_usd)
        || (input.price_move_pct < -0.05
            && input.aggressive_sell_notional_usd > input.aggressive_buy_notional_usd);
    let base = directional_strength * 80.0;
    (base + if price_follow { 20.0 } else { 0.0 }).clamp(0.0, 100.0)
}

fn absorption_score(
    input: &LiquidityMicrostructureInput,
    total_flow: f64,
    directional_strength: f64,
) -> f64 {
    if total_flow <= 0.0 {
        return 0.0;
    }
    let price_stuck = input.price_move_pct.abs() <= 0.05;
    let replenish = input
        .replenishment_ratio
        .unwrap_or_default()
        .clamp(0.0, 1.0);
    let flow_intensity = directional_strength * 70.0;
    let replenish_score = replenish * 30.0;
    if price_stuck {
        (flow_intensity + replenish_score).clamp(0.0, 100.0)
    } else {
        (replenish_score * 0.5).clamp(0.0, 40.0)
    }
}

fn depth_imbalance(input: &LiquidityMicrostructureInput) -> f64 {
    match (input.bid_depth_1pct_usd, input.ask_depth_1pct_usd) {
        (Some(bid), Some(ask)) if bid > 0.0 || ask > 0.0 => {
            ((bid - ask) / (bid + ask).max(1.0)).clamp(-1.0, 1.0)
        }
        _ => 0.0,
    }
}

fn depth_drop(input: &LiquidityMicrostructureInput) -> f64 {
    let bid_drop = relative_drop(input.previous_bid_depth_1pct_usd, input.bid_depth_1pct_usd);
    let ask_drop = relative_drop(input.previous_ask_depth_1pct_usd, input.ask_depth_1pct_usd);
    bid_drop.max(ask_drop)
}

fn relative_drop(previous: Option<f64>, current: Option<f64>) -> f64 {
    match (previous, current) {
        (Some(prev), Some(now)) if prev > 0.0 && now >= 0.0 => {
            ((prev - now) / prev).clamp(0.0, 1.0)
        }
        _ => 0.0,
    }
}

fn spread_widen_ratio(input: &LiquidityMicrostructureInput) -> f64 {
    match (input.previous_spread_bps, input.spread_bps) {
        (Some(prev), Some(now)) if prev > 0.0 && now >= prev => {
            ((now - prev) / prev).clamp(0.0, 5.0)
        }
        _ => 0.0,
    }
}

fn spread_score(input: &LiquidityMicrostructureInput, spread_widen: f64, depth_drop: f64) -> f64 {
    let absolute = input
        .spread_bps
        .map(|spread| (spread / 20.0 * 60.0).clamp(0.0, 60.0))
        .unwrap_or_default();
    let widening = (spread_widen / 1.0 * 40.0).clamp(0.0, 40.0);
    (absolute + widening + depth_drop * 25.0).clamp(0.0, 100.0)
}

fn spoofing_score(input: &LiquidityMicrostructureInput) -> f64 {
    let add = input.large_order_add_usd.unwrap_or_default().max(0.0);
    let cancel = input.large_order_cancel_usd.unwrap_or_default().max(0.0);
    if add <= 0.0 {
        return 0.0;
    }
    let executed = input.large_order_executed_usd.unwrap_or_default().max(0.0);
    let cancel_ratio = (cancel / add).clamp(0.0, 1.0);
    let execution_ratio = (executed / add).clamp(0.0, 1.0);
    ((cancel_ratio * 100.0) - (execution_ratio * 50.0)).clamp(0.0, 100.0)
}

fn classify_behavior(
    input: &LiquidityMicrostructureInput,
    direction: &str,
    order_flow_pressure: f64,
    absorption_strength: f64,
    spoofing_penalty: f64,
    imbalance_score: f64,
    spread_behavior: f64,
) -> String {
    if spoofing_penalty >= 70.0 {
        return "SpoofingDetected".to_string();
    }
    if absorption_strength >= 70.0 {
        return match direction {
            "buy" => "Absorption_Sell".to_string(),
            "sell" => "Absorption_Buy".to_string(),
            _ => "Absorption".to_string(),
        };
    }
    if order_flow_pressure >= 70.0 && input.price_move_pct > 0.05 {
        return "LiquiditySweepUp".to_string();
    }
    if order_flow_pressure >= 70.0 && input.price_move_pct < -0.05 {
        return "LiquiditySweepDown".to_string();
    }
    if spread_behavior >= 70.0 {
        return if direction == "buy" {
            "LiquidityPullUp".to_string()
        } else {
            "LiquidityPullDown".to_string()
        };
    }
    if imbalance_score >= 60.0 {
        return if depth_imbalance(input) > 0.0 {
            "BullishImbalance".to_string()
        } else {
            "BearishImbalance".to_string()
        };
    }
    "OrdinaryFlow".to_string()
}

fn market_control(direction: &str, behavior: &str, lms_score: f64) -> String {
    if behavior == "OrdinaryFlow" && lms_score < 40.0 {
        return "no_clear_control".to_string();
    }
    if behavior.starts_with("Spoofing") {
        return "fake_liquidity_control".to_string();
    }
    if behavior.contains("Absorption") {
        return "two_sided_absorption".to_string();
    }
    match direction {
        "buy" => "buyer_side_control".to_string(),
        "sell" => "seller_side_control".to_string(),
        _ => "no_clear_control".to_string(),
    }
}

fn explanation_tags(
    behavior: &str,
    direction: &str,
    depth_imbalance: f64,
    spread_widen: f64,
    spoofing_penalty: f64,
    absorption_strength: f64,
    order_flow_pressure: f64,
) -> Vec<String> {
    let mut tags = vec!["read_only_microstructure".to_string()];
    match behavior {
        "LiquiditySweepUp" | "LiquiditySweepDown" => tags.push("liquidity_sweep".to_string()),
        "Absorption_Buy" | "Absorption_Sell" | "Absorption" => tags.push("absorption".to_string()),
        "SpoofingDetected" => tags.push("spoofing_detected".to_string()),
        "LiquidityPullUp" | "LiquidityPullDown" => tags.push("liquidity_pull".to_string()),
        "BullishImbalance" => tags.push("bullish_imbalance".to_string()),
        "BearishImbalance" => tags.push("bearish_imbalance".to_string()),
        _ => {}
    }
    if direction == "buy" {
        tags.push("aggressive_buy_pressure".to_string());
    } else if direction == "sell" {
        tags.push("aggressive_sell_pressure".to_string());
    }
    if depth_imbalance.abs() >= 0.6 {
        tags.push("depth_skew".to_string());
    }
    if spread_widen >= 0.5 {
        tags.push("spread_widening".to_string());
    }
    if spoofing_penalty >= 40.0 {
        tags.push("fake_liquidity_watch".to_string());
    }
    if absorption_strength >= 60.0 {
        tags.push("price_absorption".to_string());
    }
    if order_flow_pressure >= 70.0 {
        tags.push("aggressive_taker_flow".to_string());
    }
    tags
}

fn pressure_label(score: f64) -> String {
    if score >= 80.0 {
        "HIGH".to_string()
    } else if score >= 60.0 {
        "ELEVATED".to_string()
    } else if score >= 40.0 {
        "NORMAL".to_string()
    } else {
        "LOW".to_string()
    }
}

fn spread_state(input: &LiquidityMicrostructureInput, spread_widen: f64) -> String {
    if spread_widen >= 0.5 {
        "widening".to_string()
    } else if input.spread_bps.is_some() {
        "stable".to_string()
    } else {
        "unknown".to_string()
    }
}

fn interpretation(score: f64) -> String {
    if score >= 80.0 {
        "盘口微观结构显示强主力控盘迹象".to_string()
    } else if score >= 60.0 {
        "盘口结构出现明显变化，可作为 BACM 增强证据".to_string()
    } else if score >= 40.0 {
        "盘口结构处于普通波动区间".to_string()
    } else {
        "盘口结构信号较弱或缺少 L2 上下文".to_string()
    }
}

fn round2(value: f64) -> f64 {
    (value * 100.0).round() / 100.0
}

fn round4(value: f64) -> f64 {
    (value * 10_000.0).round() / 10_000.0
}
