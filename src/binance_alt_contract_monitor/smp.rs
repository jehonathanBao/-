use super::types::{
    AltContractContext, AltContractMarketRegime, AltContractMasterCapitalStrength,
    AltContractSmartMoneyLifecycle, AltContractSmartMoneyPrediction, AltContractWindowStats,
};

pub fn predict_smart_money_next_stage(
    stats: &AltContractWindowStats,
    context: &AltContractContext,
    mcss: &AltContractMasterCapitalStrength,
    lifecycle: &AltContractSmartMoneyLifecycle,
    regime: &AltContractMarketRegime,
) -> AltContractSmartMoneyPrediction {
    let current_state = normalized_state(&lifecycle.lifecycle_state);
    let mut forecast = base_forecast(&current_state);
    let metrics = PredictionMetrics::from(stats, context, mcss, lifecycle, regime);
    let mut factors = Vec::new();

    apply_state_adjustments(&mut forecast, &metrics, &mut factors);

    if metrics.manipulation_noise {
        forecast.probability -= 8.0;
        forecast.confidence -= 12.0;
        forecast.score -= 10.0;
        factors.push("manipulation_noise_filtered".to_string());
    }

    let direction = direction_bias(&forecast.next_state, &metrics);
    let probability = forecast.probability.clamp(35.0, 95.0);
    let confidence = forecast.confidence.clamp(25.0, 96.0);
    let prediction_score = forecast.score.clamp(0.0, 100.0);

    AltContractSmartMoneyPrediction {
        current_state,
        next_state: forecast.next_state,
        probability: round2(probability),
        time_horizon_min: forecast.time_horizon_min,
        direction_bias: direction.label,
        direction_probability: round2(direction.probability),
        confidence: round2(confidence),
        prediction_score: round2(prediction_score),
        trigger_factors: factors,
        explanation: prediction_explanation(forecast.explanation, metrics.manipulation_noise),
    }
}

struct PredictionMetrics {
    oi_trend: String,
    price_trend: String,
    mcss: f64,
    mcss_acceleration: f64,
    efficiency_ratio: f64,
    liquidation_ratio: f64,
    funding_rate: f64,
    flow_consistency: f64,
    lifecycle_confidence: f64,
    manipulation_noise: bool,
}

impl PredictionMetrics {
    fn from(
        stats: &AltContractWindowStats,
        context: &AltContractContext,
        mcss: &AltContractMasterCapitalStrength,
        lifecycle: &AltContractSmartMoneyLifecycle,
        regime: &AltContractMarketRegime,
    ) -> Self {
        let liquidation_ratio = if stats.total_notional_usd > 0.0 {
            context.liquidation_notional_usd.unwrap_or(0.0) / stats.total_notional_usd
        } else {
            0.0
        };
        let mcss_acceleration = (mcss.mcss - regime.mc_score).max(0.0);
        Self {
            oi_trend: regime.oi_trend.clone(),
            price_trend: regime.price_trend.clone(),
            mcss: mcss.mcss,
            mcss_acceleration,
            efficiency_ratio: regime.efficiency_ratio,
            liquidation_ratio,
            funding_rate: context.funding_rate.unwrap_or(0.0),
            flow_consistency: lifecycle.flow_consistency_score,
            lifecycle_confidence: lifecycle.state_confidence,
            manipulation_noise: regime.regime.eq_ignore_ascii_case("manipulation")
                || lifecycle
                    .explanation_tags
                    .iter()
                    .any(|tag| tag == "manipulation_disturbance"),
        }
    }
}

struct ForecastDraft {
    next_state: String,
    probability: f64,
    time_horizon_min: u32,
    confidence: f64,
    score: f64,
    explanation: &'static str,
}

struct DirectionDraft {
    label: String,
    probability: f64,
}

fn base_forecast(current_state: &str) -> ForecastDraft {
    match current_state {
        "Accumulation" => ForecastDraft {
            next_state: "Markup".to_string(),
            probability: 75.0,
            time_horizon_min: 60,
            confidence: 64.0,
            score: 68.0,
            explanation: "吸筹后默认观察是否进入拉升阶段。",
        },
        "Markup" => ForecastDraft {
            next_state: "Distribution".to_string(),
            probability: 65.0,
            time_horizon_min: 45,
            confidence: 62.0,
            score: 64.0,
            explanation: "拉升后默认观察是否进入派发阶段。",
        },
        "Distribution" => ForecastDraft {
            next_state: "Markdown".to_string(),
            probability: 70.0,
            time_horizon_min: 30,
            confidence: 66.0,
            score: 70.0,
            explanation: "派发后默认观察是否进入下跌/砸盘阶段。",
        },
        "Markdown" => ForecastDraft {
            next_state: "ReAccumulation".to_string(),
            probability: 80.0,
            time_horizon_min: 90,
            confidence: 60.0,
            score: 66.0,
            explanation: "砸盘后默认观察是否进入再吸筹阶段。",
        },
        "ReAccumulation" => ForecastDraft {
            next_state: "Accumulation".to_string(),
            probability: 70.0,
            time_horizon_min: 120,
            confidence: 58.0,
            score: 62.0,
            explanation: "再吸筹后默认观察是否重新形成吸筹周期。",
        },
        _ => ForecastDraft {
            next_state: "Sideways".to_string(),
            probability: 50.0,
            time_horizon_min: 60,
            confidence: 35.0,
            score: 40.0,
            explanation: "生命周期不足，先按震荡观察。",
        },
    }
}

fn apply_state_adjustments(
    forecast: &mut ForecastDraft,
    metrics: &PredictionMetrics,
    factors: &mut Vec<String>,
) {
    if metrics.oi_trend == "up" && metrics.mcss >= 70.0 {
        forecast.probability += 7.0;
        forecast.score += 8.0;
        factors.push("oi_mcss_expansion".to_string());
    }
    if matches!(metrics.price_trend.as_str(), "slow_up" | "spike_up") && metrics.oi_trend == "down"
    {
        forecast.probability += 10.0;
        forecast.score += 8.0;
        factors.push("oi_momentum_divergence".to_string());
    }
    if metrics.efficiency_ratio <= 0.35 {
        forecast.probability += 6.0;
        forecast.score += 5.0;
        factors.push("efficiency_decay".to_string());
    }
    if metrics.liquidation_ratio >= 0.30 {
        forecast.probability += 8.0;
        forecast.confidence -= 6.0;
        forecast.score += 4.0;
        factors.push("liquidity_stress".to_string());
    }
    if metrics.funding_rate.abs() >= 0.0005 {
        forecast.probability += 4.0;
        factors.push("funding_extreme".to_string());
    }
    if metrics.flow_consistency >= 65.0 {
        forecast.confidence += 8.0;
        forecast.score += 6.0;
        factors.push("market_structure_consistent".to_string());
    }
    if metrics.lifecycle_confidence >= 75.0 {
        forecast.confidence += 6.0;
        factors.push("lifecycle_confidence_high".to_string());
    }
    if metrics.mcss_acceleration >= 5.0 {
        forecast.probability += 4.0;
        forecast.score += 4.0;
        factors.push("mcss_acceleration".to_string());
    }
}

fn direction_bias(next_state: &str, metrics: &PredictionMetrics) -> DirectionDraft {
    match next_state {
        "Markup" => DirectionDraft {
            label: "Bullish".to_string(),
            probability: (0.58 + metrics.mcss / 400.0).clamp(0.50, 0.86),
        },
        "Distribution" => DirectionDraft {
            label: "BearishRisk".to_string(),
            probability: (0.54 + metrics.efficiency_ratio.min(0.5) * 0.2).clamp(0.50, 0.78),
        },
        "Markdown" => DirectionDraft {
            label: "Bearish".to_string(),
            probability: (0.62 + metrics.liquidation_ratio.min(0.5) * 0.35).clamp(0.55, 0.88),
        },
        "ReAccumulation" => DirectionDraft {
            label: "ReboundWatch".to_string(),
            probability: (0.52 + (1.0 - metrics.efficiency_ratio.min(1.0)) * 0.16)
                .clamp(0.50, 0.76),
        },
        _ => DirectionDraft {
            label: "Sideways".to_string(),
            probability: 0.50,
        },
    }
}

fn prediction_explanation(base: &str, manipulation_noise: bool) -> String {
    if manipulation_noise {
        format!("{base} 当前包含操控/诱导噪音，预测层已降权处理，不把插针直接当作趋势。")
    } else {
        base.to_string()
    }
}

fn normalized_state(value: &str) -> String {
    match value.to_ascii_lowercase().as_str() {
        "markup" => "Markup".to_string(),
        "distribution" => "Distribution".to_string(),
        "markdown" => "Markdown".to_string(),
        "reaccumulation" | "re_accumulation" | "re-accumulation" => "ReAccumulation".to_string(),
        "sideways" => "Sideways".to_string(),
        _ => "Accumulation".to_string(),
    }
}

fn round2(value: f64) -> f64 {
    (value * 100.0).round() / 100.0
}
