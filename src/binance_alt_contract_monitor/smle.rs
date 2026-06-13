use super::types::{
    AltContractContext, AltContractMarketRegime, AltContractMasterCapitalStrength,
    AltContractSmartMoneyLifecycle, AltContractWindowConfirmation, AltContractWindowStats,
};

pub fn classify_smart_money_lifecycle(
    stats: &AltContractWindowStats,
    context: &AltContractContext,
    mcss: &AltContractMasterCapitalStrength,
    regime: &AltContractMarketRegime,
    window_confirmations: &[AltContractWindowConfirmation],
    previous: Option<&AltContractSmartMoneyLifecycle>,
) -> AltContractSmartMoneyLifecycle {
    let manipulation_event = regime.regime.eq_ignore_ascii_case("manipulation");
    let metrics = LifecycleMetrics::from(stats, context, mcss, regime, window_confirmations);
    let mut state = state_from_metrics(&metrics);
    let mut tags = lifecycle_tags(&metrics);
    let freeze_lifecycle = manipulation_event && !is_markdown(&metrics);

    if freeze_lifecycle {
        tags.push("manipulation_disturbance".to_string());
        if let Some(previous_state) = previous {
            state = previous_state.lifecycle_state.clone();
        }
    } else if manipulation_event {
        tags.push("manipulation_disturbance".to_string());
    }

    let transition_signal = transition_signal(previous, &state, freeze_lifecycle);
    let state_duration_min = state_duration_min(previous, &state, stats.window_sec);
    let state_path = state_path(previous, &state);
    let lifecycle_score = lifecycle_score(&metrics, manipulation_event);
    let confidence = state_confidence(
        &metrics,
        lifecycle_score,
        previous,
        &state,
        manipulation_event,
    );
    let explanation = lifecycle_explanation(&state, manipulation_event);

    AltContractSmartMoneyLifecycle {
        lifecycle_state: state,
        state_confidence: round2(confidence),
        state_duration_min: round2(state_duration_min),
        transition_signal,
        flow_consistency_score: round2(metrics.flow_consistency_score),
        lifecycle_score: round2(lifecycle_score),
        state_path,
        explanation_tags: tags,
        current_explanation: explanation,
    }
}

struct LifecycleMetrics {
    oi_trend: String,
    price_trend: String,
    regime: String,
    mcss: f64,
    build_proxy: f64,
    directional_strength: f64,
    dynamic_multiple: f64,
    liquidation_ratio: f64,
    flow_consistency_score: f64,
    price_efficiency: f64,
    volatility_shock: bool,
}

impl LifecycleMetrics {
    fn from(
        stats: &AltContractWindowStats,
        context: &AltContractContext,
        mcss: &AltContractMasterCapitalStrength,
        regime: &AltContractMarketRegime,
        window_confirmations: &[AltContractWindowConfirmation],
    ) -> Self {
        let confirmed_windows = window_confirmations
            .iter()
            .filter(|window| window.confirmed)
            .count() as f64;
        let flow_consistency_score = ((confirmed_windows * 28.0)
            + f64::from(context.persistence_windows) * 18.0
            + stats.dominance * 35.0)
            .clamp(0.0, 100.0);
        let liquidation_ratio = if stats.total_notional_usd > 0.0 {
            context.liquidation_notional_usd.unwrap_or(0.0) / stats.total_notional_usd
        } else {
            0.0
        };
        let price_efficiency = regime.efficiency_ratio;
        let dynamic_multiple = stats.dynamic_multiple.unwrap_or(0.0);
        let price_move = stats.price_move_pct.unwrap_or(0.0).abs();

        Self {
            oi_trend: regime.oi_trend.clone(),
            price_trend: regime.price_trend.clone(),
            regime: regime.regime.clone(),
            mcss: mcss.mcss,
            build_proxy: (mcss.oi_score + mcss.price_score + mcss.anomaly_score).clamp(0.0, 65.0),
            directional_strength: stats.dominance,
            dynamic_multiple,
            liquidation_ratio,
            flow_consistency_score,
            price_efficiency,
            volatility_shock: price_move >= 0.80 || dynamic_multiple >= 6.0,
        }
    }
}

fn state_from_metrics(metrics: &LifecycleMetrics) -> String {
    if is_markdown(metrics) {
        "Markdown".to_string()
    } else if is_markup(metrics) {
        "Markup".to_string()
    } else if metrics.regime.eq_ignore_ascii_case("distribution") || is_distribution(metrics) {
        "Distribution".to_string()
    } else if is_re_accumulation(metrics) {
        "ReAccumulation".to_string()
    } else {
        "Accumulation".to_string()
    }
}

fn is_markup(metrics: &LifecycleMetrics) -> bool {
    metrics.oi_trend == "up"
        && matches!(metrics.price_trend.as_str(), "slow_up" | "spike_up")
        && metrics.mcss >= 75.0
        && metrics.directional_strength >= 0.60
        && metrics.flow_consistency_score >= 55.0
}

fn is_distribution(metrics: &LifecycleMetrics) -> bool {
    matches!(metrics.oi_trend.as_str(), "flat" | "down")
        && matches!(metrics.price_trend.as_str(), "slow_up" | "spike_up")
        && metrics.mcss >= 55.0
        && metrics.price_efficiency <= 0.35
}

fn is_markdown(metrics: &LifecycleMetrics) -> bool {
    metrics.oi_trend == "down"
        && matches!(metrics.price_trend.as_str(), "down" | "spike_down")
        && (metrics.liquidation_ratio >= 0.20 || metrics.mcss >= 70.0 || metrics.volatility_shock)
}

fn is_re_accumulation(metrics: &LifecycleMetrics) -> bool {
    matches!(metrics.oi_trend.as_str(), "flat" | "up")
        && matches!(metrics.price_trend.as_str(), "flat" | "slow_up")
        && metrics.mcss >= 45.0
        && metrics.mcss <= 75.0
        && metrics.liquidation_ratio < 0.15
        && metrics.dynamic_multiple < 4.0
}

fn transition_signal(
    previous: Option<&AltContractSmartMoneyLifecycle>,
    state: &str,
    freeze_lifecycle: bool,
) -> Option<String> {
    if freeze_lifecycle {
        return None;
    }
    let previous_state = previous?.lifecycle_state.as_str();
    if previous_state == state {
        None
    } else {
        Some(format!("{previous_state}->{state}"))
    }
}

fn state_duration_min(
    previous: Option<&AltContractSmartMoneyLifecycle>,
    state: &str,
    window_sec: u64,
) -> f64 {
    let window_min = (window_sec as f64 / 60.0).max(0.1);
    if let Some(previous_state) = previous {
        if previous_state.lifecycle_state == state {
            return previous_state.state_duration_min + window_min;
        }
    }
    window_min
}

fn state_path(previous: Option<&AltContractSmartMoneyLifecycle>, state: &str) -> Vec<String> {
    let mut path = previous
        .map(|previous_state| previous_state.state_path.clone())
        .unwrap_or_default();
    if path.last().is_none_or(|last| last != state) {
        path.push(state.to_string());
    }
    if path.len() > 5 {
        path.remove(0);
    }
    path
}

fn lifecycle_score(metrics: &LifecycleMetrics, manipulation_event: bool) -> f64 {
    let trend_alignment = if same_direction_structure(metrics) {
        20.0
    } else {
        8.0
    };
    let oi_consistency = match metrics.oi_trend.as_str() {
        "up" => 20.0,
        "flat" => 12.0,
        "down" => 10.0,
        _ => 5.0,
    };
    let price_structure = match metrics.price_trend.as_str() {
        "slow_up" | "down" => 18.0,
        "flat" => 14.0,
        "spike_up" | "spike_down" => 10.0,
        _ => 6.0,
    };
    let mcss_strength = (metrics.mcss * 0.24).min(24.0);
    let volatility_regime = if metrics.volatility_shock { 6.0 } else { 14.0 };
    let manipulation_penalty = if manipulation_event {
        18.0
    } else if metrics.liquidation_ratio >= 0.40 {
        14.0
    } else {
        0.0
    };
    (trend_alignment + oi_consistency + price_structure + mcss_strength + volatility_regime
        - manipulation_penalty)
        .clamp(0.0, 100.0)
}

fn state_confidence(
    metrics: &LifecycleMetrics,
    lifecycle_score: f64,
    previous: Option<&AltContractSmartMoneyLifecycle>,
    state: &str,
    manipulation_event: bool,
) -> f64 {
    let continuity = previous
        .filter(|previous_state| previous_state.lifecycle_state == state)
        .map(|_| 8.0)
        .unwrap_or(0.0);
    let disturbance_penalty = if manipulation_event { 12.0 } else { 0.0 };
    (lifecycle_score * 0.65
        + metrics.flow_consistency_score * 0.20
        + metrics.build_proxy * 0.15
        + continuity
        - disturbance_penalty)
        .clamp(0.0, 96.0)
}

fn same_direction_structure(metrics: &LifecycleMetrics) -> bool {
    matches!(
        (metrics.oi_trend.as_str(), metrics.price_trend.as_str()),
        ("up", "slow_up")
            | ("up", "spike_up")
            | ("flat", "flat")
            | ("down", "down")
            | ("down", "spike_down")
    )
}

fn lifecycle_tags(metrics: &LifecycleMetrics) -> Vec<String> {
    let mut tags = Vec::new();
    if metrics.oi_trend == "up" {
        tags.push("oi_expansion".to_string());
    } else if metrics.oi_trend == "down" {
        tags.push("oi_contraction".to_string());
    }
    if metrics.flow_consistency_score >= 65.0 {
        tags.push("flow_consistent".to_string());
    }
    if metrics.mcss >= 75.0 {
        tags.push("mcss_confirmed".to_string());
    }
    if metrics.liquidation_ratio >= 0.20 {
        tags.push("liquidation_disturbance".to_string());
    }
    if metrics.price_efficiency <= 0.35 {
        tags.push("low_price_efficiency".to_string());
    }
    tags
}

fn lifecycle_explanation(state: &str, manipulation_event: bool) -> String {
    let base = match state {
        "Accumulation" => "当前接近吸筹阶段：OI 与资金强度温和抬升，价格结构未进入强趋势。",
        "Markup" => "当前接近拉升阶段：OI、价格突破和 MCSS 同向增强，趋势可能进入加速段。",
        "Distribution" => "当前接近派发阶段：价格仍偏强，但 OI 或效率开始减弱，需警惕高位出货。",
        "Markdown" => "当前接近砸盘阶段：价格走弱、OI 收缩或清算扰动增加，踩踏风险上升。",
        "ReAccumulation" => {
            "当前接近再吸筹阶段：下跌后价格企稳，OI 停止恶化，可能进入新建仓观察期。"
        }
        _ => "生命周期结构仍未确认。",
    };
    if manipulation_event {
        format!("{base} 当前信号包含操控/诱导扰动，作为插入事件处理，不直接改变生命周期主状态。")
    } else {
        base.to_string()
    }
}

fn round2(value: f64) -> f64 {
    (value * 100.0).round() / 100.0
}
