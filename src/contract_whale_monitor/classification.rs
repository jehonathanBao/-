use super::{
    config::ContractWhaleRuntimeConfig,
    types::{
        ContractWhaleActiveFlowDirection, ContractWhaleClassificationV2,
        ContractWhaleDynamicPriceThresholds, ContractWhaleEvidenceState,
        ContractWhaleEvidenceSummary, ContractWhaleOiContextTag, ContractWhaleOiWindowContext,
        ContractWhalePriceResponseType, ContractWhaleResolvedOiContext, ContractWhaleSignalType,
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
        let flow = flow_direction(stats, config);
        let oi = legacy_oi_context_resolution(stats, config, flow);
        return ContractWhaleClassificationV2 {
            legacy_signal_type: legacy_signal_type_key(signal_type).to_string(),
            display_signal_type: legacy_display_label(signal_type).to_string(),
            structure_interpretation: legacy_structure(signal_type),
            flow_direction: flow,
            price_response_type_v2: legacy_price_response_type,
            oi_context: oi.oi_context,
            oi_context_label: oi.oi_context_label,
            oi_delta: oi.oi_delta,
            oi_delta_pct: oi.oi_delta_pct,
            oi_available: oi.oi_available,
            oi_reason: oi.oi_reason,
            oi_consistent_sources: Vec::new(),
            oi_excluded_sources: Vec::new(),
            oi_source_coverage_changed: false,
            oi_cross_exchange_consensus: None,
            oi_evidence_degraded: stats.market_context.evidence_degraded,
            oi_evidence_reason: stats.market_context.evidence_reason.clone(),
            intent_confidence: 0,
            is_strong_main_force_intent: false,
            classification_version: "v2_disabled_legacy_compat".to_string(),
            semantic_mismatch: false,
            classification_reasons: vec!["classification_v2_disabled".to_string()],
            dynamic_thresholds: dynamic_thresholds(stats, config),
            price_efficiency: price_efficiency(stats, config),
            price_efficiency_version: price_efficiency_version(config).to_string(),
            evidence: evidence_summary(stats, multi_exchange_confirmed),
        };
    }

    let flow = flow_direction(stats, config);
    let thresholds = dynamic_thresholds(stats, config);
    let price_move = stats.price_move_pct.unwrap_or(0.0);
    let reversal = stats.price_reversal_ratio.unwrap_or(0.0);
    let efficiency = price_efficiency(stats, config);
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
        && efficiency
            <= if config.classification.normalized_price_efficiency_enabled {
                config
                    .classification
                    .low_price_efficiency_max_bps_per_million
            } else {
                config.classification.low_price_efficiency_max
            }
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

    let oi = legacy_oi_context_resolution(stats, config, flow);
    let oi_context = oi.oi_context;
    reasons.push(format!("oi_context:{oi_context:?}"));

    ContractWhaleClassificationV2 {
        legacy_signal_type: legacy_signal_type_key(signal_type).to_string(),
        display_signal_type: display_label(structure_interpretation).to_string(),
        structure_interpretation,
        flow_direction: flow,
        price_response_type_v2,
        oi_context,
        oi_context_label: oi.oi_context_label,
        oi_delta: oi.oi_delta,
        oi_delta_pct: oi.oi_delta_pct,
        oi_available: oi.oi_available,
        oi_reason: oi.oi_reason,
        oi_consistent_sources: Vec::new(),
        oi_excluded_sources: Vec::new(),
        oi_source_coverage_changed: false,
        oi_cross_exchange_consensus: None,
        oi_evidence_degraded: stats.market_context.evidence_degraded,
        oi_evidence_reason: stats.market_context.evidence_reason.clone(),
        intent_confidence: intent_confidence(
            stats,
            same_direction_follow,
            multi_exchange_confirmed,
            strong_intent,
            oi_context,
            config,
        ),
        is_strong_main_force_intent: strong_intent,
        classification_version: "contract_whale_v2_shadow".to_string(),
        semantic_mismatch: legacy_structure(signal_type) != structure_interpretation,
        classification_reasons: reasons,
        dynamic_thresholds: thresholds,
        price_efficiency: round(efficiency, 4),
        price_efficiency_version: price_efficiency_version(config).to_string(),
        evidence: evidence_summary(stats, multi_exchange_confirmed),
    }
}

fn evidence_summary(
    stats: &ContractWhaleWindowStats,
    multi_exchange_confirmed: bool,
) -> ContractWhaleEvidenceSummary {
    let dynamic_multiple = stats
        .dynamic_multiple
        .filter(|value| value.is_finite())
        .map(ContractWhaleEvidenceState::Available)
        .unwrap_or(ContractWhaleEvidenceState::Missing);
    let percentile_level = stats
        .percentile_level
        .filter(|value| value.is_finite())
        .map(ContractWhaleEvidenceState::Available)
        .unwrap_or(ContractWhaleEvidenceState::InsufficientSamples);
    let oi = if stats.market_context.oi_available {
        stats
            .market_context
            .oi_change_pct
            .filter(|value| value.is_finite())
            .map(ContractWhaleEvidenceState::Available)
            .unwrap_or(ContractWhaleEvidenceState::Missing)
    } else {
        ContractWhaleEvidenceState::Missing
    };
    let funding = if stats.market_context.funding_available {
        stats
            .market_context
            .funding_rate
            .filter(|value| value.is_finite())
            .map(ContractWhaleEvidenceState::Available)
            .unwrap_or(ContractWhaleEvidenceState::Missing)
    } else {
        ContractWhaleEvidenceState::Missing
    };
    let multi_exchange_confirmation = if stats.exchange_count >= 2 {
        ContractWhaleEvidenceState::Available(multi_exchange_confirmed)
    } else {
        ContractWhaleEvidenceState::InsufficientSamples
    };
    let (liquidation_status, liquidation_reason) = if stats.liquidation_context.total_liq_btc > 0.0
    {
        ("live".to_string(), None)
    } else if stats.liquidation_driven {
        (
            "inferred".to_string(),
            Some("price_volume_shape_only".to_string()),
        )
    } else {
        (
            "unavailable".to_string(),
            Some("no_live_liquidation_samples".to_string()),
        )
    };
    let mut degradation_reasons = Vec::new();
    if matches!(dynamic_multiple, ContractWhaleEvidenceState::Missing) {
        degradation_reasons.push("dynamic_multiple_missing".to_string());
    }
    if matches!(
        percentile_level,
        ContractWhaleEvidenceState::InsufficientSamples
    ) {
        degradation_reasons.push("percentile_insufficient_samples".to_string());
    }
    if stats.micro_volatility.source == "fallback" {
        degradation_reasons.push(
            if stats.micro_volatility.stale {
                "micro_volatility_stale"
            } else {
                "micro_volatility_insufficient_samples"
            }
            .to_string(),
        );
    }
    if stats.market_context.evidence_degraded {
        degradation_reasons.push(
            stats
                .market_context
                .evidence_reason
                .clone()
                .unwrap_or_else(|| "market_context_evidence_degraded".to_string()),
        );
    }
    ContractWhaleEvidenceSummary {
        dynamic_multiple,
        percentile_level,
        oi,
        funding,
        multi_exchange_confirmation,
        liquidation_status,
        liquidation_reason,
        evidence_degraded: !degradation_reasons.is_empty(),
        evidence_reason: degradation_reasons.first().cloned(),
        degradation_reasons,
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

fn dynamic_thresholds(
    stats: &ContractWhaleWindowStats,
    config: &ContractWhaleRuntimeConfig,
) -> ContractWhaleDynamicPriceThresholds {
    let micro = &stats.micro_volatility;
    let usable_micro_volatility = (config.classification.micro_volatility_enabled
        && !micro.stale
        && micro.sample_count >= config.classification.micro_volatility_min_samples)
        .then(|| micro.value_pct.filter(|value| value.is_finite()))
        .flatten();
    let (no_follow_pct, follow_pct, strong_follow_pct, volatility_source) =
        if let Some(volatility) = usable_micro_volatility {
            (
                config
                    .classification
                    .no_follow_pct
                    .max(volatility * config.classification.micro_volatility_no_follow_multiplier),
                config
                    .classification
                    .follow_pct
                    .max(volatility * config.classification.micro_volatility_follow_multiplier),
                config.classification.strong_follow_pct.max(
                    volatility
                        * config
                            .classification
                            .micro_volatility_strong_follow_multiplier,
                ),
                micro.source.clone(),
            )
        } else {
            (
                config.classification.no_follow_pct,
                config.classification.follow_pct,
                config.classification.strong_follow_pct,
                "fallback".to_string(),
            )
        };
    ContractWhaleDynamicPriceThresholds {
        no_follow_pct,
        follow_pct,
        strong_follow_pct,
        volatility_source,
        micro_volatility_pct: usable_micro_volatility,
        volatility_sample_count: micro.sample_count,
        volatility_stale: micro.stale,
    }
}

fn price_efficiency(stats: &ContractWhaleWindowStats, config: &ContractWhaleRuntimeConfig) -> f64 {
    let price_move = stats.price_move_pct.unwrap_or(0.0).abs();
    if !config.classification.normalized_price_efficiency_enabled {
        return if stats.total_volume_btc <= f64::EPSILON {
            0.0
        } else {
            (price_move / stats.total_volume_btc) * 1_000.0
        };
    }
    if stats.total_notional_usd <= f64::EPSILON {
        return 0.0;
    }
    (price_move * 100.0) / (stats.total_notional_usd / 1_000_000.0)
}

fn price_efficiency_version(config: &ContractWhaleRuntimeConfig) -> &'static str {
    if config.classification.normalized_price_efficiency_enabled {
        "notional_bps_per_usd_million_v1"
    } else {
        "legacy_btc_volume_v0"
    }
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

fn legacy_oi_context_resolution(
    stats: &ContractWhaleWindowStats,
    config: &ContractWhaleRuntimeConfig,
    flow: ContractWhaleActiveFlowDirection,
) -> ContractWhaleResolvedOiContext {
    let oi_context = oi_context(stats, config, flow);
    ContractWhaleResolvedOiContext {
        oi_context,
        oi_context_label: oi_context.label().to_string(),
        oi_delta: stats
            .market_context
            .oi_change_5m_btc
            .or(stats.market_context.oi_change_1m_btc),
        oi_delta_pct: stats.market_context.oi_change_pct,
        oi_available: stats.market_context.oi_available
            && stats.market_context.oi_change_pct.is_some(),
        oi_reason: Some(
            match oi_context {
                ContractWhaleOiContextTag::NewLongBuild => {
                    "market_context_oi_increased_with_buy_pressure"
                }
                ContractWhaleOiContextTag::NewShortBuild => {
                    "market_context_oi_increased_with_sell_pressure"
                }
                ContractWhaleOiContextTag::ShortCovering => {
                    "market_context_oi_decreased_with_buy_pressure"
                }
                ContractWhaleOiContextTag::LongUnwind => {
                    "market_context_oi_decreased_with_sell_pressure"
                }
                ContractWhaleOiContextTag::OiNotConfirmed => "market_context_oi_not_confirmed",
                ContractWhaleOiContextTag::OiUnavailable => "market_context_oi_unavailable",
            }
            .to_string(),
        ),
    }
}

pub fn resolve_contract_whale_oi_context_from_window(
    structure_interpretation: ContractWhaleStructureInterpretation,
    flow: ContractWhaleActiveFlowDirection,
    price_response_type: ContractWhalePriceResponseType,
    price_move_pct: Option<f64>,
    oi_window: Option<&ContractWhaleOiWindowContext>,
    config: &ContractWhaleRuntimeConfig,
) -> ContractWhaleResolvedOiContext {
    if !config.classification.oi_context_enabled {
        return ContractWhaleResolvedOiContext {
            oi_context: ContractWhaleOiContextTag::OiUnavailable,
            oi_context_label: ContractWhaleOiContextTag::OiUnavailable.label().to_string(),
            oi_delta: None,
            oi_delta_pct: None,
            oi_available: false,
            oi_reason: Some("oi_context_disabled".to_string()),
        };
    }
    let Some(oi_window) = oi_window else {
        return ContractWhaleResolvedOiContext {
            oi_context: ContractWhaleOiContextTag::OiUnavailable,
            oi_context_label: ContractWhaleOiContextTag::OiUnavailable.label().to_string(),
            oi_delta: None,
            oi_delta_pct: None,
            oi_available: false,
            oi_reason: Some("no_oi_snapshot_in_window".to_string()),
        };
    };
    if !oi_window.available {
        return ContractWhaleResolvedOiContext {
            oi_context: ContractWhaleOiContextTag::OiUnavailable,
            oi_context_label: ContractWhaleOiContextTag::OiUnavailable.label().to_string(),
            oi_delta: oi_window.oi_delta,
            oi_delta_pct: oi_window.oi_delta_pct,
            oi_available: false,
            oi_reason: Some(
                oi_window
                    .reason
                    .clone()
                    .unwrap_or_else(|| "no_oi_snapshot_in_window".to_string()),
            ),
        };
    }

    if config.classification.oi_consensus_guard_enabled && oi_window.source_coverage_changed {
        return ContractWhaleResolvedOiContext {
            oi_context: ContractWhaleOiContextTag::OiNotConfirmed,
            oi_context_label: ContractWhaleOiContextTag::OiNotConfirmed
                .label()
                .to_string(),
            oi_delta: oi_window.oi_delta,
            oi_delta_pct: oi_window.oi_delta_pct,
            oi_available: true,
            oi_reason: Some("oi_source_coverage_changed".to_string()),
        };
    }

    if config.classification.oi_consensus_guard_enabled
        && matches!(oi_window.cross_exchange_consensus, Some(false))
    {
        return ContractWhaleResolvedOiContext {
            oi_context: ContractWhaleOiContextTag::OiNotConfirmed,
            oi_context_label: ContractWhaleOiContextTag::OiNotConfirmed
                .label()
                .to_string(),
            oi_delta: oi_window.oi_delta,
            oi_delta_pct: oi_window.oi_delta_pct,
            oi_available: true,
            oi_reason: Some("oi_cross_exchange_conflict".to_string()),
        };
    }

    let oi_delta_pct = oi_window.oi_delta_pct;
    let Some(delta_pct) = oi_delta_pct else {
        return ContractWhaleResolvedOiContext {
            oi_context: ContractWhaleOiContextTag::OiUnavailable,
            oi_context_label: ContractWhaleOiContextTag::OiUnavailable.label().to_string(),
            oi_delta: oi_window.oi_delta,
            oi_delta_pct: None,
            oi_available: false,
            oi_reason: Some(
                oi_window
                    .reason
                    .clone()
                    .unwrap_or_else(|| "no_oi_snapshot_in_window".to_string()),
            ),
        };
    };

    if matches!(
        structure_interpretation,
        ContractWhaleStructureInterpretation::DownsideAbsorption
            | ContractWhaleStructureInterpretation::UpsideSuppression
    ) {
        let (label, reason) = if delta_pct >= config.classification.oi_delta_min_pct {
            (
                "OI 增加，新仓对抗".to_string(),
                "oi_increased_during_absorption_or_suppression".to_string(),
            )
        } else if delta_pct <= -config.classification.oi_delta_min_pct {
            (
                "OI 下降，平仓/止损参与".to_string(),
                "oi_decreased_during_absorption_or_suppression".to_string(),
            )
        } else {
            (
                ContractWhaleOiContextTag::OiNotConfirmed
                    .label()
                    .to_string(),
                "oi_delta_below_threshold".to_string(),
            )
        };
        return ContractWhaleResolvedOiContext {
            oi_context: ContractWhaleOiContextTag::OiNotConfirmed,
            oi_context_label: label,
            oi_delta: oi_window.oi_delta,
            oi_delta_pct,
            oi_available: true,
            oi_reason: Some(reason),
        };
    }

    if delta_pct.abs() <= config.classification.oi_flat_max_abs_pct
        || delta_pct.abs() < config.classification.oi_delta_min_pct
    {
        return ContractWhaleResolvedOiContext {
            oi_context: ContractWhaleOiContextTag::OiNotConfirmed,
            oi_context_label: ContractWhaleOiContextTag::OiNotConfirmed
                .label()
                .to_string(),
            oi_delta: oi_window.oi_delta,
            oi_delta_pct,
            oi_available: true,
            oi_reason: Some("oi_delta_below_threshold".to_string()),
        };
    }

    match flow {
        ContractWhaleActiveFlowDirection::Balanced | ContractWhaleActiveFlowDirection::Unknown => {
            return ContractWhaleResolvedOiContext {
                oi_context: ContractWhaleOiContextTag::OiNotConfirmed,
                oi_context_label: ContractWhaleOiContextTag::OiNotConfirmed
                    .label()
                    .to_string(),
                oi_delta: oi_window.oi_delta,
                oi_delta_pct,
                oi_available: true,
                oi_reason: Some("direction_not_clear_for_oi_context".to_string()),
            };
        }
        ContractWhaleActiveFlowDirection::BuyDominant
        | ContractWhaleActiveFlowDirection::SellDominant => {}
    }

    let price_confirmed = match flow {
        ContractWhaleActiveFlowDirection::BuyDominant => {
            matches!(
                price_response_type,
                ContractWhalePriceResponseType::TrendFollowUp
            ) || price_move_pct.unwrap_or_default() > 0.0
        }
        ContractWhaleActiveFlowDirection::SellDominant => {
            matches!(
                price_response_type,
                ContractWhalePriceResponseType::TrendFollowDown
            ) || price_move_pct.unwrap_or_default() < 0.0
        }
        ContractWhaleActiveFlowDirection::Balanced | ContractWhaleActiveFlowDirection::Unknown => {
            false
        }
    };

    if !price_confirmed {
        return ContractWhaleResolvedOiContext {
            oi_context: ContractWhaleOiContextTag::OiNotConfirmed,
            oi_context_label: ContractWhaleOiContextTag::OiNotConfirmed
                .label()
                .to_string(),
            oi_delta: oi_window.oi_delta,
            oi_delta_pct,
            oi_available: true,
            oi_reason: Some("price_not_confirmed_for_oi_context".to_string()),
        };
    }

    let (oi_context, reason) = match (flow, delta_pct.is_sign_positive()) {
        (ContractWhaleActiveFlowDirection::BuyDominant, true) => (
            ContractWhaleOiContextTag::NewLongBuild,
            "oi_increased_with_buy_pressure",
        ),
        (ContractWhaleActiveFlowDirection::BuyDominant, false) => (
            ContractWhaleOiContextTag::ShortCovering,
            "oi_decreased_with_buy_pressure",
        ),
        (ContractWhaleActiveFlowDirection::SellDominant, true) => (
            ContractWhaleOiContextTag::NewShortBuild,
            "oi_increased_with_sell_pressure",
        ),
        (ContractWhaleActiveFlowDirection::SellDominant, false) => (
            ContractWhaleOiContextTag::LongUnwind,
            "oi_decreased_with_sell_pressure",
        ),
        (ContractWhaleActiveFlowDirection::Balanced, _)
        | (ContractWhaleActiveFlowDirection::Unknown, _) => (
            ContractWhaleOiContextTag::OiNotConfirmed,
            "direction_not_clear_for_oi_context",
        ),
    };

    ContractWhaleResolvedOiContext {
        oi_context,
        oi_context_label: oi_context.label().to_string(),
        oi_delta: oi_window.oi_delta,
        oi_delta_pct,
        oi_available: true,
        oi_reason: Some(reason.to_string()),
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

fn legacy_signal_type_key(value: ContractWhaleSignalType) -> &'static str {
    match value {
        ContractWhaleSignalType::AggressiveBuy => "aggressive_buy",
        ContractWhaleSignalType::AggressiveSell => "aggressive_sell",
        ContractWhaleSignalType::DownsideAbsorption => "downside_absorption",
        ContractWhaleSignalType::UpsideSuppression => "upside_suppression",
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
