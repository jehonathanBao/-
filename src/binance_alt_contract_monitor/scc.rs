use super::types::{
    AltContractConfidenceLevel, AltContractContext, AltContractDirection,
    AltContractLiquidityMicrostructure, AltContractMarketControlGraph,
    AltContractMasterCapitalStrength, AltContractSeverity, AltContractSignalConfidence,
    AltContractSignalConfidenceBreakdown, AltContractSignalType, AltContractSmartMoneyLifecycle,
    AltContractSmartMoneyPrediction, AltContractWindowStats,
};

pub fn calibrate_signal_confidence(
    stats: &AltContractWindowStats,
    context: &AltContractContext,
    signal_type: AltContractSignalType,
    abnormal_score: u8,
    build_score: u8,
    severity: AltContractSeverity,
    mcss: &AltContractMasterCapitalStrength,
    lifecycle: &AltContractSmartMoneyLifecycle,
    prediction: &AltContractSmartMoneyPrediction,
    microstructure: &AltContractLiquidityMicrostructure,
    graph: &AltContractMarketControlGraph,
    market_wide_move: bool,
) -> AltContractSignalConfidence {
    let bacm_signal_strength = bacm_strength(abnormal_score, build_score, severity);
    let mcss_strength = mcss.mcss.clamp(0.0, 100.0);
    let smle_stability = average_nonzero(&[
        lifecycle.state_confidence,
        lifecycle.flow_consistency_score,
        lifecycle.lifecycle_score,
    ]);
    let smp_prediction_alignment = prediction_alignment(stats.direction, prediction);
    let lme_microstructure_support = microstructure_support(stats.direction, microstructure);
    let mcg_control_coherence = control_coherence(stats.direction, graph);
    let smaf_risk_penalty = risk_penalty(
        stats,
        context,
        prediction,
        microstructure,
        graph,
        market_wide_move,
    );

    let confidence_score = (0.20 * bacm_signal_strength
        + 0.20 * mcss_strength
        + 0.20 * smle_stability
        + 0.15 * smp_prediction_alignment
        + 0.15 * lme_microstructure_support
        + 0.10 * mcg_control_coherence
        - 0.10 * smaf_risk_penalty)
        .clamp(0.0, 100.0);
    let confidence_level = confidence_level(confidence_score);
    let reliability_factors = reliability_factors(
        stats,
        bacm_signal_strength,
        mcss_strength,
        smle_stability,
        smp_prediction_alignment,
        lme_microstructure_support,
        mcg_control_coherence,
    );
    let risk_factors = risk_factors(
        stats,
        context,
        smp_prediction_alignment,
        microstructure,
        graph,
        market_wide_move,
    );

    AltContractSignalConfidence {
        symbol: stats.symbol.clone(),
        signal_type: format!("{signal_type:?}"),
        confidence_score: round2(confidence_score),
        confidence_level,
        reliability_factors,
        risk_factors,
        breakdown: AltContractSignalConfidenceBreakdown {
            bacm_signal_strength: round2(bacm_signal_strength),
            mcss_strength: round2(mcss_strength),
            smle_stability: round2(smle_stability),
            smp_prediction_alignment: round2(smp_prediction_alignment),
            lme_microstructure_support: round2(lme_microstructure_support),
            mcg_control_coherence: round2(mcg_control_coherence),
            smaf_risk_penalty: round2(smaf_risk_penalty),
        },
        interpretation: interpretation(confidence_score),
        read_only: true,
        direct_discord_gate: false,
    }
}

fn bacm_strength(abnormal_score: u8, build_score: u8, severity: AltContractSeverity) -> f64 {
    let base = (f64::from(abnormal_score) + f64::from(build_score)) / 2.0;
    let severity_bonus = match severity {
        AltContractSeverity::S => 8.0,
        AltContractSeverity::Critical => 5.0,
        AltContractSeverity::High => 2.0,
        AltContractSeverity::Medium | AltContractSeverity::Calm => 0.0,
    };
    (base + severity_bonus).clamp(0.0, 100.0)
}

fn average_nonzero(values: &[f64]) -> f64 {
    let mut total = 0.0;
    let mut count = 0.0;
    for value in values.iter().copied() {
        if value > 0.0 {
            total += value.clamp(0.0, 100.0);
            count += 1.0;
        }
    }
    if count > 0.0 {
        total / count
    } else {
        0.0
    }
}

fn prediction_alignment(
    direction: AltContractDirection,
    prediction: &AltContractSmartMoneyPrediction,
) -> f64 {
    let mut score = (0.55 * prediction.confidence
        + 0.30 * prediction.probability
        + 0.15 * prediction.prediction_score)
        .clamp(0.0, 100.0);
    match (direction, direction_bias_key(&prediction.direction_bias)) {
        (AltContractDirection::Buy, "bullish") | (AltContractDirection::Sell, "bearish") => {
            score += 8.0;
        }
        (AltContractDirection::Buy, "bearish") | (AltContractDirection::Sell, "bullish") => {
            score -= 24.0;
        }
        (AltContractDirection::Neutral, _) => score -= 5.0,
        _ => {}
    }
    score.clamp(0.0, 100.0)
}

fn microstructure_support(
    direction: AltContractDirection,
    microstructure: &AltContractLiquidityMicrostructure,
) -> f64 {
    let mut score = microstructure.lms_score.clamp(0.0, 100.0);
    match (direction, microstructure.market_control.as_str()) {
        (AltContractDirection::Buy, "buyer_side_control")
        | (AltContractDirection::Sell, "seller_side_control") => score += 8.0,
        (AltContractDirection::Buy, "seller_side_control")
        | (AltContractDirection::Sell, "buyer_side_control") => score -= 18.0,
        _ => {}
    }
    if microstructure
        .spoofing_state
        .eq_ignore_ascii_case("detected")
        || microstructure.market_control == "fake_liquidity_control"
    {
        score -= 20.0;
    }
    score.clamp(0.0, 100.0)
}

fn control_coherence(
    direction: AltContractDirection,
    graph: &AltContractMarketControlGraph,
) -> f64 {
    let mut score = graph.control_strength.clamp(0.0, 100.0);
    match (direction, graph.dominant_side.as_str()) {
        (AltContractDirection::Buy, "buy") | (AltContractDirection::Sell, "sell") => {
            score += 8.0;
        }
        (AltContractDirection::Buy, "sell") | (AltContractDirection::Sell, "buy") => {
            score -= 18.0;
        }
        _ => {}
    }
    if graph.control_type == "NoClearControl" {
        score -= 10.0;
    }
    score.clamp(0.0, 100.0)
}

fn risk_penalty(
    stats: &AltContractWindowStats,
    context: &AltContractContext,
    prediction: &AltContractSmartMoneyPrediction,
    microstructure: &AltContractLiquidityMicrostructure,
    graph: &AltContractMarketControlGraph,
    market_wide_move: bool,
) -> f64 {
    let mut penalty = 0.0;
    if stats.data_quality < 70 {
        penalty += f64::from(70 - stats.data_quality) * 1.2;
    }
    if context.liquidation_suspected || context.force_order_snapshot {
        penalty += 18.0;
    }
    if liquidation_ratio(stats, context) >= 0.40 {
        penalty += 28.0;
    } else if liquidation_ratio(stats, context) >= 0.20 {
        penalty += 16.0;
    }
    if context.oi_change_pct.unwrap_or_default() < -0.5 {
        penalty += 10.0;
    }
    if microstructure
        .spoofing_state
        .eq_ignore_ascii_case("detected")
        || microstructure.market_control == "fake_liquidity_control"
    {
        penalty += 24.0;
    }
    if graph.control_type == "ControlManipulation" {
        penalty += 12.0;
    }
    if market_wide_move {
        penalty += 8.0;
    }
    if direction_bias_key(&prediction.direction_bias) == "conflict" {
        penalty += 10.0;
    }
    penalty.clamp(0.0, 100.0)
}

fn liquidation_ratio(stats: &AltContractWindowStats, context: &AltContractContext) -> f64 {
    let liquidation = context
        .liquidation_notional_usd
        .unwrap_or_default()
        .max(0.0);
    if stats.total_notional_usd > 0.0 {
        (liquidation / stats.total_notional_usd).clamp(0.0, 1.0)
    } else {
        0.0
    }
}

fn reliability_factors(
    stats: &AltContractWindowStats,
    bacm_signal_strength: f64,
    mcss_strength: f64,
    smle_stability: f64,
    smp_prediction_alignment: f64,
    lme_microstructure_support: f64,
    mcg_control_coherence: f64,
) -> Vec<String> {
    let mut factors = Vec::new();
    if bacm_signal_strength >= 80.0 {
        factors.push("bacm_signal_strong".to_string());
    }
    if mcss_strength >= 75.0 {
        factors.push("mcss_strong_money".to_string());
    }
    if smle_stability >= 75.0 {
        factors.push("smle_stable_lifecycle".to_string());
    }
    if smp_prediction_alignment >= 70.0 {
        factors.push("smp_aligned".to_string());
    }
    if lme_microstructure_support >= 70.0 {
        factors.push("lme_orderbook_support".to_string());
    }
    if mcg_control_coherence >= 70.0 {
        factors.push("mcg_control_coherent".to_string());
    }
    if stats.dynamic_multiple.unwrap_or_default() >= 6.0 {
        factors.push("dynamic_multiple_confirmed".to_string());
    }
    if stats.data_quality >= 70 {
        factors.push("data_quality_ok".to_string());
    }
    factors
}

fn risk_factors(
    stats: &AltContractWindowStats,
    context: &AltContractContext,
    prediction_alignment: f64,
    microstructure: &AltContractLiquidityMicrostructure,
    graph: &AltContractMarketControlGraph,
    market_wide_move: bool,
) -> Vec<String> {
    let mut factors = Vec::new();
    if stats.data_quality < 70 {
        factors.push("data_quality_low".to_string());
    }
    if context.liquidation_suspected
        || context.force_order_snapshot
        || liquidation_ratio(stats, context) >= 0.20
    {
        factors.push("liquidation_interference".to_string());
    }
    if context.oi_change_pct.unwrap_or_default() < -0.5 {
        factors.push("oi_contracting".to_string());
    }
    if prediction_alignment < 50.0 {
        factors.push("prediction_misaligned".to_string());
    }
    if microstructure
        .spoofing_state
        .eq_ignore_ascii_case("detected")
        || microstructure.market_control == "fake_liquidity_control"
    {
        factors.push("spoofing_or_fake_liquidity".to_string());
    }
    if graph.control_type == "ControlManipulation" {
        factors.push("control_manipulation_risk".to_string());
    }
    if market_wide_move {
        factors.push("market_wide_noise".to_string());
    }
    factors
}

fn confidence_level(score: f64) -> AltContractConfidenceLevel {
    if score >= 90.0 {
        AltContractConfidenceLevel::VeryHigh
    } else if score >= 75.0 {
        AltContractConfidenceLevel::High
    } else if score >= 60.0 {
        AltContractConfidenceLevel::Medium
    } else if score >= 40.0 {
        AltContractConfidenceLevel::Weak
    } else {
        AltContractConfidenceLevel::Noise
    }
}

fn direction_bias_key(value: &str) -> &'static str {
    let value = value.to_ascii_lowercase();
    if value.contains("bull") || value.contains("long") {
        "bullish"
    } else if value.contains("bear") || value.contains("short") || value.contains("risk") {
        "bearish"
    } else if value.contains("conflict") {
        "conflict"
    } else {
        "neutral"
    }
}

fn interpretation(score: f64) -> String {
    if score >= 90.0 {
        "多层确认高度一致，属于极高可信信号；SCC 仅解释可信度，不改变推送 gate。".to_string()
    } else if score >= 75.0 {
        "多层确认较强，属于高可信主力行为候选；SCC 不直接触发 Discord。".to_string()
    } else if score >= 60.0 {
        "信号具备中等可信度，适合观察后续确认。".to_string()
    } else if score >= 40.0 {
        "信号存在明显冲突或风险因素，可信度较弱。".to_string()
    } else {
        "信号缺少多层确认，更接近噪音或不可用样本。".to_string()
    }
}

fn round2(value: f64) -> f64 {
    (value * 100.0).round() / 100.0
}
