use super::{
    config::ContractWhaleRuntimeConfig,
    types::{
        ContractWhaleActiveFlowDirection, ContractWhaleClassificationV2,
        ContractWhaleDynamicPriceThresholds, ContractWhaleOiContextTag,
        ContractWhalePriceResponseType, ContractWhaleSignalType,
        ContractWhaleStructureInterpretation, ContractWhaleWindowStats,
    },
};

pub fn classify_contract_whale_signal_v2(
    stats: &ContractWhaleWindowStats,
    signal_type: ContractWhaleSignalType,
    legacy_price_response_type: ContractWhalePriceResponseType,
    multi_exchange_confirmed: bool,
    config: &ContractWhaleRuntimeConfig,
) -> ContractWhaleClassificationV2 {
    if !config.classification.enabled {
        return ContractWhaleClassificationV2 {
            display_signal_type: legacy_display_label(signal_type).to_string(),
            structure_interpretation: legacy_structure(signal_type),
            flow_direction: flow_direction(stats, config),
            price_response_type_v2: legacy_price_response_type,
            oi_context: oi_context(stats, config, flow_direction(stats, config)),
            intent_confidence: 0,
            is_strong_main_force_intent: false,
            classification_version: "v2_disabled_legacy_compat".to_string(),
            classification_reasons: vec!["classification_v2_disabled".to_string()],
            dynamic_thresholds: dynamic_thresholds(config),
            price_efficiency: price_efficiency(stats),
        };
    }

    let flow = flow_direction(stats, config);
    let thresholds = dynamic_thresholds(config);
    let price_move = stats.price_move_pct.unwrap_or(0.0);
    let reversal = stats.price_reversal_ratio.unwrap_or(0.0);
    let efficiency = price_efficiency(stats);
    let same_direction_follow = match flow {
        ContractWhaleActiveFlowDirection::BuyDominant => price_move.max(0.0),
        ContractWhaleActiveFlowDirection::SellDominant => (-price_move).max(0.0),
        ContractWhaleActiveFlowDirection::Balanced | ContractWhaleActiveFlowDirection::Unknown => {
            0.0
        }
    };
    let strong_source_ok = !config
        .classification
        .require_multi_exchange_for_strong_intent
        || multi_exchange_confirmed;
    let absorption_source_ok =
        !config.classification.require_multi_exchange_for_absorption || multi_exchange_confirmed;

    let strong_intent = flow != ContractWhaleActiveFlowDirection::Balanced
        && flow != ContractWhaleActiveFlowDirection::Unknown
        && stats.dominance >= config.classification.strong_intent_dominance_min
        && same_direction_follow >= config.classification.follow_same_direction_min_pct
        && same_direction_follow >= thresholds.follow_pct
        && stats.data_quality >= config.classification.min_data_quality_for_strong_intent
        && strong_source_ok
        && matches!(
            signal_type,
            ContractWhaleSignalType::AggressiveBuy | ContractWhaleSignalType::AggressiveSell
        );

    let no_downside_follow = price_move > -thresholds.no_follow_pct || reversal >= 0.50;
    let no_upside_follow = price_move < thresholds.no_follow_pct || reversal >= 0.50;
    let absorption_quality_ok = stats.dominance >= config.classification.absorption_dominance_min
        && stats.total_notional_usd >= config.classification.absorption_min_notional_usd
        && stats.data_quality >= config.classification.min_data_quality_for_absorption
        && efficiency <= config.classification.low_price_efficiency_max
        && absorption_source_ok;
    let downside_absorption = flow == ContractWhaleActiveFlowDirection::SellDominant
        && absorption_quality_ok
        && no_downside_follow;
    let upside_suppression = flow == ContractWhaleActiveFlowDirection::BuyDominant
        && absorption_quality_ok
        && no_upside_follow;

    let structure_interpretation = if strong_intent {
        match flow {
            ContractWhaleActiveFlowDirection::BuyDominant => {
                ContractWhaleStructureInterpretation::MainForcePushUp
            }
            ContractWhaleActiveFlowDirection::SellDominant => {
                ContractWhaleStructureInterpretation::MainForceDumpDown
            }
            ContractWhaleActiveFlowDirection::Balanced
            | ContractWhaleActiveFlowDirection::Unknown => {
                ContractWhaleStructureInterpretation::UnclearDirectionalFlow
            }
        }
    } else if downside_absorption {
        ContractWhaleStructureInterpretation::DownsideAbsorption
    } else if upside_suppression {
        ContractWhaleStructureInterpretation::UpsideSuppression
    } else {
        match flow {
            ContractWhaleActiveFlowDirection::BuyDominant => {
                ContractWhaleStructureInterpretation::ActiveBuyPressure
            }
            ContractWhaleActiveFlowDirection::SellDominant => {
                ContractWhaleStructureInterpretation::ActiveSellPressure
            }
            ContractWhaleActiveFlowDirection::Balanced => {
                ContractWhaleStructureInterpretation::UnclearDirectionalFlow
            }
            ContractWhaleActiveFlowDirection::Unknown => {
                ContractWhaleStructureInterpretation::UnclearDirectionalFlow
            }
        }
    };

    let price_response_type_v2 = match structure_interpretation {
        ContractWhaleStructureInterpretation::MainForcePushUp => {
            ContractWhalePriceResponseType::TrendFollowUp
        }
        ContractWhaleStructureInterpretation::MainForceDumpDown => {
            ContractWhalePriceResponseType::TrendFollowDown
        }
        ContractWhaleStructureInterpretation::DownsideAbsorption => {
            ContractWhalePriceResponseType::DownsideAbsorption
        }
        ContractWhaleStructureInterpretation::UpsideSuppression => {
            ContractWhalePriceResponseType::UpsideResistance
        }
        ContractWhaleStructureInterpretation::ActiveBuyPressure
        | ContractWhaleStructureInterpretation::ActiveSellPressure
        | ContractWhaleStructureInterpretation::UnclearDirectionalFlow => {
            ContractWhalePriceResponseType::NoClearResponse
        }
    };

    let mut reasons = Vec::new();
    reasons.push(format!("flow_direction:{flow:?}"));
    reasons.push(format!("dominance:{:.2}", stats.dominance));
    reasons.push(format!("price_move_pct:{:.3}", price_move));
    reasons.push(format!("price_efficiency:{:.3}", efficiency));
    if multi_exchange_confirmed {
        reasons.push("multi_window_or_exchange_confirmed".to_string());
    }
    if strong_intent {
        reasons.push("price_follow_through".to_string());
        reasons.push("strong_main_force_intent_confirmed".to_string());
    } else if matches!(
        structure_interpretation,
        ContractWhaleStructureInterpretation::DownsideAbsorption
            | ContractWhaleStructureInterpretation::UpsideSuppression
    ) {
        reasons.push("low_impact_absorption_or_suppression".to_string());
    } else {
        reasons.push("active_pressure_without_full_intent_confirmation".to_string());
    }

    let oi_context = oi_context(stats, config, flow);
    reasons.push(format!("oi_context:{oi_context:?}"));

    ContractWhaleClassificationV2 {
        display_signal_type: display_label(structure_interpretation).to_string(),
        structure_interpretation,
        flow_direction: flow,
        price_response_type_v2,
        oi_context,
        intent_confidence: intent_confidence(
            stats,
            same_direction_follow,
            multi_exchange_confirmed,
            strong_intent,
            oi_context,
            config,
        ),
        is_strong_main_force_intent: strong_intent,
        classification_version: "contract_whale_v2_compat".to_string(),
        classification_reasons: reasons,
        dynamic_thresholds: thresholds,
        price_efficiency: round(efficiency, 4),
    }
}

fn flow_direction(
    stats: &ContractWhaleWindowStats,
    config: &ContractWhaleRuntimeConfig,
) -> ContractWhaleActiveFlowDirection {
    if stats.total_volume_btc <= f64::EPSILON {
        return ContractWhaleActiveFlowDirection::Unknown;
    }
    if stats.dominance < config.classification.flow_direction_dominance_min {
        return ContractWhaleActiveFlowDirection::Balanced;
    }
    if stats.net_volume_btc > 0.0 {
        ContractWhaleActiveFlowDirection::BuyDominant
    } else if stats.net_volume_btc < 0.0 {
        ContractWhaleActiveFlowDirection::SellDominant
    } else {
        ContractWhaleActiveFlowDirection::Balanced
    }
}

fn dynamic_thresholds(config: &ContractWhaleRuntimeConfig) -> ContractWhaleDynamicPriceThresholds {
    ContractWhaleDynamicPriceThresholds {
        no_follow_pct: config.classification.no_follow_pct,
        follow_pct: config.classification.follow_pct,
        strong_follow_pct: config.classification.strong_follow_pct,
        volatility_source: "fallback".to_string(),
    }
}

fn price_efficiency(stats: &ContractWhaleWindowStats) -> f64 {
    let price_move = stats.price_move_pct.unwrap_or(0.0).abs();
    if stats.total_volume_btc <= f64::EPSILON {
        return 0.0;
    }
    (price_move / stats.total_volume_btc) * 1_000.0
}

fn oi_context(
    stats: &ContractWhaleWindowStats,
    config: &ContractWhaleRuntimeConfig,
    flow: ContractWhaleActiveFlowDirection,
) -> ContractWhaleOiContextTag {
    let Some(oi_change_pct) = stats.market_context.oi_change_pct else {
        return ContractWhaleOiContextTag::OiUnavailable;
    };
    if !stats.market_context.oi_available && oi_change_pct.abs() <= f64::EPSILON {
        return ContractWhaleOiContextTag::OiUnavailable;
    }
    let threshold = config.classification.oi_context_change_pct;
    if oi_change_pct.abs() < threshold {
        return ContractWhaleOiContextTag::OiNotConfirmed;
    }
    match (flow, oi_change_pct.is_sign_positive()) {
        (ContractWhaleActiveFlowDirection::BuyDominant, true) => {
            ContractWhaleOiContextTag::NewLongBuild
        }
        (ContractWhaleActiveFlowDirection::SellDominant, true) => {
            ContractWhaleOiContextTag::NewShortBuild
        }
        (ContractWhaleActiveFlowDirection::BuyDominant, false) => {
            ContractWhaleOiContextTag::ShortCovering
        }
        (ContractWhaleActiveFlowDirection::SellDominant, false) => {
            ContractWhaleOiContextTag::LongUnwind
        }
        _ => ContractWhaleOiContextTag::OiNotConfirmed,
    }
}

fn intent_confidence(
    stats: &ContractWhaleWindowStats,
    same_direction_follow: f64,
    multi_exchange_confirmed: bool,
    strong_intent: bool,
    oi_context: ContractWhaleOiContextTag,
    config: &ContractWhaleRuntimeConfig,
) -> u8 {
    let mut score = 0.0;
    score += (stats.dominance / config.classification.strong_intent_dominance_min).min(1.0) * 22.0;
    score += (same_direction_follow / config.classification.follow_same_direction_min_pct).min(1.0)
        * 28.0;
    score += if multi_exchange_confirmed { 20.0 } else { 4.0 };
    score += (stats.data_quality as f64 / 100.0).clamp(0.0, 1.0) * 20.0;
    if !matches!(oi_context, ContractWhaleOiContextTag::OiUnavailable) {
        score += 5.0;
    }
    if strong_intent {
        score += 8.0;
    }
    score.round().clamp(0.0, 100.0) as u8
}

fn display_label(value: ContractWhaleStructureInterpretation) -> &'static str {
    match value {
        ContractWhaleStructureInterpretation::MainForcePushUp => "主力拉盘",
        ContractWhaleStructureInterpretation::MainForceDumpDown => "主力砸盘",
        ContractWhaleStructureInterpretation::ActiveBuyPressure => "主动买压",
        ContractWhaleStructureInterpretation::ActiveSellPressure => "主动卖压",
        ContractWhaleStructureInterpretation::DownsideAbsorption => "下方吸收",
        ContractWhaleStructureInterpretation::UpsideSuppression => "上方压制",
        ContractWhaleStructureInterpretation::UnclearDirectionalFlow => "不明确合约流",
    }
}

fn legacy_display_label(value: ContractWhaleSignalType) -> &'static str {
    match value {
        ContractWhaleSignalType::AggressiveBuy => "主力拉盘",
        ContractWhaleSignalType::AggressiveSell => "主力砸盘",
        ContractWhaleSignalType::DownsideAbsorption => "下方吸收",
        ContractWhaleSignalType::UpsideSuppression => "上方压制",
    }
}

fn legacy_structure(value: ContractWhaleSignalType) -> ContractWhaleStructureInterpretation {
    match value {
        ContractWhaleSignalType::AggressiveBuy => {
            ContractWhaleStructureInterpretation::MainForcePushUp
        }
        ContractWhaleSignalType::AggressiveSell => {
            ContractWhaleStructureInterpretation::MainForceDumpDown
        }
        ContractWhaleSignalType::DownsideAbsorption => {
            ContractWhaleStructureInterpretation::DownsideAbsorption
        }
        ContractWhaleSignalType::UpsideSuppression => {
            ContractWhaleStructureInterpretation::UpsideSuppression
        }
    }
}

fn round(value: f64, precision: i32) -> f64 {
    let factor = 10_f64.powi(precision);
    (value * factor).round() / factor
}
