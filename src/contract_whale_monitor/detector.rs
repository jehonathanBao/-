use super::{
    classification::classify_contract_whale_signal_v2,
    config::{
        contract_whale_runtime_config, ContractWhaleNotionalThresholds, ContractWhaleRuntimeConfig,
        ThresholdProfileResolution,
    },
    log_events,
    scoring::{discord_gate, score_contract_whale_breakdown_with_profile},
    types::{
        ContractWhaleActiveSourceEntry, ContractWhaleActiveSources, ContractWhaleDirection,
        ContractWhaleForcedFlowAttribution, ContractWhaleLiquidationForce,
        ContractWhaleLiquidationZone, ContractWhaleMarketDriver,
        ContractWhaleMarketDriverComponent, ContractWhaleMarketType,
        ContractWhalePriceImpactAttribution, ContractWhalePriceResponseType, ContractWhaleSeverity,
        ContractWhaleSignal, ContractWhaleSignalType, ContractWhaleThresholds,
        ContractWhaleWindowStats,
    },
    LOG_PREFIX, LOG_TARGET,
};
use crate::normalization::market_impact::{
    normalize_market_impact_from_metrics, MarketImpactNormalization,
};

#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ContractWhaleDetectorRejectReason {
    SymbolDisabled,
    NoActiveContractSources,
    ZeroVolume,
    DataQualityTooLow,
    NeutralNetVolume,
    BelowVolumeThreshold,
    BelowNotionalThreshold,
    DynamicMultipleTooLow,
    PercentileTooLow,
    DominanceTooLow,
    Warmup,
    MultiExchangeNotConfirmed,
    SameDirectionPriceMoveTooLow,
    Unknown,
}

impl ContractWhaleDetectorRejectReason {
    pub fn as_key(self) -> &'static str {
        match self {
            Self::SymbolDisabled => "symbol_disabled",
            Self::NoActiveContractSources => "no_active_contract_sources",
            Self::ZeroVolume => "zero_volume",
            Self::DataQualityTooLow => "data_quality_too_low",
            Self::NeutralNetVolume => "neutral_net_volume",
            Self::BelowVolumeThreshold => "below_volume_threshold",
            Self::BelowNotionalThreshold => "below_notional_threshold",
            Self::DynamicMultipleTooLow => "dynamic_multiple_too_low",
            Self::PercentileTooLow => "percentile_too_low",
            Self::DominanceTooLow => "dominance_too_low",
            Self::Warmup => "warmup",
            Self::MultiExchangeNotConfirmed => "multi_exchange_not_confirmed",
            Self::SameDirectionPriceMoveTooLow => "same_direction_price_move_too_low",
            Self::Unknown => "unknown",
        }
    }
}

#[derive(Debug, Clone)]
pub struct ContractWhaleDetectionDecision {
    pub signal: Option<ContractWhaleSignal>,
    pub reject_reason: Option<ContractWhaleDetectorRejectReason>,
}

pub fn detect_contract_whale_signal(
    stats: &ContractWhaleWindowStats,
) -> Option<ContractWhaleSignal> {
    detect_contract_whale_signal_with_config(stats, &contract_whale_runtime_config())
}

pub fn detect_contract_whale_signal_with_config(
    stats: &ContractWhaleWindowStats,
    config: &ContractWhaleRuntimeConfig,
) -> Option<ContractWhaleSignal> {
    inspect_contract_whale_signal_with_config(stats, config).signal
}

pub fn inspect_contract_whale_signal_with_config(
    stats: &ContractWhaleWindowStats,
    config: &ContractWhaleRuntimeConfig,
) -> ContractWhaleDetectionDecision {
    if !config.symbol_enabled(&stats.symbol) {
        return rejected(ContractWhaleDetectorRejectReason::SymbolDisabled);
    }
    let resolution = config.threshold_profile_resolution_for_observed_sources(
        stats.exchanges.iter().map(|item| item.exchange.clone()),
    );
    if resolution.active_contract_sources.is_empty() {
        return rejected(ContractWhaleDetectorRejectReason::NoActiveContractSources);
    }
    let mut scoring_stats = stats.clone();
    scoring_stats.data_quality = effective_data_quality(&scoring_stats, config, &resolution);
    if scoring_stats.total_volume_btc <= 0.0 {
        return rejected(ContractWhaleDetectorRejectReason::ZeroVolume);
    }
    if scoring_stats.data_quality < 50 {
        return rejected(ContractWhaleDetectorRejectReason::DataQualityTooLow);
    }
    let price_response_type = classify_price_response(&scoring_stats);
    let Some(signal_type) = classify_signal_type(&scoring_stats, price_response_type) else {
        return rejected(ContractWhaleDetectorRejectReason::NeutralNetVolume);
    };
    let liquidation_suspected = liquidation_suspected(&scoring_stats, config);
    scoring_stats.liquidation_driven = liquidation_suspected;
    let severity = classify_severity(&scoring_stats, signal_type, config, &resolution);
    if severity == ContractWhaleSeverity::Calm {
        return rejected(reject_reason_for_calm(
            &scoring_stats,
            signal_type,
            config,
            &resolution,
        ));
    }
    let score_breakdown = score_contract_whale_breakdown_with_profile(
        &scoring_stats,
        signal_type,
        config,
        resolution.profile,
    );
    let score = score_breakdown.final_score.round().clamp(0.0, 100.0) as u8;
    let multi_exchange_confirmed =
        multi_exchange_confirmed_with_config(&scoring_stats, config, &resolution);
    let primary_source_override =
        primary_source_extreme_discord_candidate(&scoring_stats, signal_type, config, &resolution);
    let warmup_collect_only = runtime_warmup(&scoring_stats, config);
    let impact = market_impact_normalization(&scoring_stats);
    let (mut discord_eligible, mut discord_reason) = discord_gate(
        severity,
        score,
        multi_exchange_confirmed,
        scoring_stats.data_quality,
        primary_source_override,
        &scoring_stats.symbol,
        scoring_stats.total_volume_btc,
        Some(impact.impact_level.as_str()),
        config,
    );
    if warmup_collect_only {
        discord_eligible = false;
        discord_reason = "warmup_collect_only".to_string();
    }
    let direction = direction_for(signal_type);
    let liquidation_force =
        build_liquidation_force(&scoring_stats, liquidation_suspected, signal_type);
    let market_driver = build_market_driver(
        &scoring_stats,
        score,
        signal_type,
        price_response_type,
        &liquidation_force,
    );
    let mut classification_v2 = classify_contract_whale_signal_v2(
        &scoring_stats,
        signal_type,
        price_response_type,
        multi_exchange_confirmed,
        config,
    );
    if severity == ContractWhaleSeverity::Critical && scoring_stats.dynamic_multiple.is_none() {
        let thresholds = config.thresholds_for_symbol_window_with_profile(
            &scoring_stats.symbol,
            scoring_stats.window_sec,
            resolution.profile,
        );
        let notional_thresholds = config.notional_thresholds_usd_for_profile(resolution.profile);
        let primary_source_extreme = primary_source_extreme_flow(
            &scoring_stats,
            config,
            &resolution,
            thresholds,
            notional_thresholds,
        );
        if critical_absolute_fallback(
            &scoring_stats,
            signal_type,
            config,
            thresholds,
            notional_thresholds,
            primary_source_extreme,
        ) {
            classification_v2.evidence.evidence_degraded = true;
            classification_v2.evidence.evidence_reason =
                Some("critical_absolute_fallback".to_string());
            classification_v2
                .evidence
                .degradation_reasons
                .push("critical_absolute_fallback".to_string());
        }
    }

    let active_sources = active_source_snapshot(&scoring_stats, config, &resolution);
    let base_asset = contract_base_asset(&stats.symbol);
    let total_volume = round(stats.total_volume_btc, 3);
    let net_volume = round(stats.net_volume_btc, 3);
    let final_result = final_result_text(&classification_v2, liquidation_suspected);
    let signal = ContractWhaleSignal {
        id: format!(
            "contract-whale:{}:{}:{}:{}",
            stats.symbol,
            stats.window_sec,
            stats.ts,
            direction_label(direction)
        ),
        ts: stats.ts,
        symbol: stats.symbol.clone(),
        window_sec: stats.window_sec,
        signal_type,
        direction,
        severity,
        score,
        main_force_score: None,
        spot_score: None,
        contract_score: None,
        base_asset: base_asset.clone(),
        quantity_unit: base_asset,
        total_volume,
        net_volume,
        total_volume_btc: total_volume,
        net_volume_btc: net_volume,
        total_notional_usd: round(stats.total_notional_usd, 2),
        dominance: round(stats.dominance, 4),
        order_price_usd: signal_price_usd(&scoring_stats).map(|value| round(value, 2)),
        current_market_price_usd: None,
        price_deviation_pct: None,
        price_deviation_filtered: false,
        price_move_pct: scoring_stats.price_move_pct.map(|value| round(value, 4)),
        price_move_5s_pct: price_move_for_window(&scoring_stats, 5),
        price_move_15s_pct: price_move_for_window(&scoring_stats, 15),
        price_move_30s_pct: price_move_for_window(&scoring_stats, 30),
        price_response_type,
        classification_v2,
        main_exchange: scoring_stats.main_exchange.clone(),
        market_type: ContractWhaleMarketType::Perp,
        source_role: signal_source_role(&scoring_stats, config),
        exchanges: scoring_stats.exchanges.clone(),
        dominant_venue_net_contribution_share: scoring_stats
            .dominant_venue_net_contribution_share
            .map(|value| round(value, 4)),
        dynamic_multiple: scoring_stats.dynamic_multiple.map(|value| round(value, 3)),
        dynamic_baseline_btc: scoring_stats
            .dynamic_baseline_btc
            .map(|value| round(value, 3)),
        dynamic_threshold_level: scoring_stats.dynamic_threshold_level.clone(),
        percentile_level: scoring_stats.percentile_level.map(|value| round(value, 1)),
        impact_level: Some(impact.impact_level),
        signal_level: Some(impact.signal_level),
        signal_label: Some(impact.signal_label),
        normalized_strength: Some(impact.normalized_strength),
        impact_score: Some(round(impact.impact_score, 3)),
        impact_z_score: Some(round(impact.z_score, 3)),
        multi_exchange_confirmed,
        liquidation_suspected,
        liquidation_long_btc: round(scoring_stats.liquidation_context.long_liq_btc, 3),
        liquidation_short_btc: round(scoring_stats.liquidation_context.short_liq_btc, 3),
        liquidation_notional_usd: round(scoring_stats.liquidation_context.liq_notional_usd, 2),
        liquidation_ratio: scoring_stats
            .liquidation_context
            .liq_to_volume_ratio
            .map(|value| round(value, 4)),
        price_reversal_ratio: scoring_stats
            .price_reversal_ratio
            .map(|value| round(value, 4)),
        oi_change_1m_btc: scoring_stats
            .market_context
            .oi_change_1m_btc
            .map(|value| round(value, 3)),
        oi_change_5m_btc: scoring_stats
            .market_context
            .oi_change_5m_btc
            .map(|value| round(value, 3)),
        oi_change_pct: scoring_stats
            .market_context
            .oi_change_pct
            .map(|value| round(value, 4)),
        oi_bias: scoring_stats.market_context.oi_bias.clone(),
        funding_rate: scoring_stats
            .market_context
            .funding_rate
            .map(|value| round(value, 8)),
        funding_bias: scoring_stats.market_context.funding_bias.clone(),
        data_quality: scoring_stats.data_quality,
        score_breakdown,
        threshold_profile: resolution.profile_name.clone(),
        threshold_profile_reason: resolution.reason.clone(),
        configured_contract_sources: resolution.configured_keys(),
        eligible_contract_sources: resolution.eligible_keys(),
        active_contract_sources: resolution.active_keys(),
        active_sources,
        spot_confirmation: Default::default(),
        discord_eligible,
        discord_sent: false,
        discord_sent_at: None,
        discord_reason,
        discord_would_send: discord_eligible,
        final_result,
        cluster: Default::default(),
        persistence: Default::default(),
        whale_action: Default::default(),
        trajectory: Default::default(),
        liquidation_force,
        market_driver,
        event_lifecycle: Default::default(),
        event_quality: Default::default(),
        read_only: true,
        analysis_only: true,
        execution_enabled: false,
        merged_from: Vec::new(),
    };
    tracing::info!(
        target: LOG_TARGET,
        event = log_events::SIGNAL_GENERATED,
        symbol = signal.symbol.as_str(),
        window_sec = signal.window_sec,
        severity = ?signal.severity,
        score = signal.score,
        volume_score = signal.score_breakdown.volume_score,
        dynamic_baseline_btc = signal.dynamic_baseline_btc,
        dynamic_threshold_level = signal.dynamic_threshold_level.as_str(),
        dynamic_anomaly_score = signal.score_breakdown.dynamic_anomaly_score,
        directional_strength_score = signal.score_breakdown.directional_strength_score,
        price_response_score = signal.score_breakdown.price_response_score,
        penalty_score = signal.score_breakdown.penalty_score,
        discord_eligible = signal.discord_eligible,
        "{} signal generated",
        LOG_PREFIX
    );
    ContractWhaleDetectionDecision {
        signal: Some(signal),
        reject_reason: None,
    }
}

fn market_impact_normalization(stats: &ContractWhaleWindowStats) -> MarketImpactNormalization {
    let impact_score = stats.dynamic_multiple.or_else(|| {
        let baseline = stats.dynamic_baseline_btc?;
        (baseline > f64::EPSILON).then(|| stats.total_volume_btc / baseline)
    });
    normalize_market_impact_from_metrics(
        stats.total_volume_btc,
        impact_score,
        impact_score,
        stats.percentile_level,
    )
}

fn rejected(reason: ContractWhaleDetectorRejectReason) -> ContractWhaleDetectionDecision {
    ContractWhaleDetectionDecision {
        signal: None,
        reject_reason: Some(reason),
    }
}

fn signal_price_usd(stats: &ContractWhaleWindowStats) -> Option<f64> {
    (stats.total_volume_btc > f64::EPSILON && stats.total_notional_usd > 0.0)
        .then(|| stats.total_notional_usd / stats.total_volume_btc)
        .filter(|price| price.is_finite() && *price > 0.0)
}

fn build_liquidation_force(
    stats: &ContractWhaleWindowStats,
    liquidation_suspected: bool,
    signal_type: ContractWhaleSignalType,
) -> ContractWhaleLiquidationForce {
    let long_liq = stats.liquidation_context.long_liq_btc.max(0.0);
    let short_liq = stats.liquidation_context.short_liq_btc.max(0.0);
    let total_liq = stats
        .liquidation_context
        .total_liq_btc
        .max(long_liq + short_liq);
    let liq_ratio = stats
        .liquidation_context
        .liq_to_volume_ratio
        .unwrap_or(0.0)
        .max(0.0);
    let price_move = stats.price_move_pct.unwrap_or(0.0);
    let abs_price_move = price_move.abs();
    let reversal = stats.price_reversal_ratio.unwrap_or(0.0).max(0.0);
    let dominance = stats.dominance.clamp(0.0, 1.0);
    let oi_falling = stats
        .market_context
        .oi_change_pct
        .is_some_and(|value| value <= -0.10);

    let long_liq_pressure = pressure_score(
        share(long_liq, total_liq) * 0.45
            + liq_ratio.min(1.0) * 0.35
            + (price_move < 0.0) as u8 as f64 * 0.10
            + oi_falling as u8 as f64 * 0.10,
    );
    let short_squeeze_pressure = pressure_score(
        share(short_liq, total_liq) * 0.45
            + liq_ratio.min(1.0) * 0.35
            + (price_move > 0.0) as u8 as f64 * 0.10
            + oi_falling as u8 as f64 * 0.10,
    );
    let shape_pressure = if liquidation_suspected { 0.35 } else { 0.0 }
        + (abs_price_move / 0.5).clamp(0.0, 1.0) * 0.30
        + reversal.min(1.0) * 0.20
        + dominance * 0.15;
    let cascade_intensity = pressure_score((liq_ratio.min(1.0) * 0.55) + shape_pressure * 0.45);
    let stop_hunt_probability = pressure_score(
        (reversal.min(1.0) * 0.45)
            + (abs_price_move / 0.35).clamp(0.0, 1.0) * 0.30
            + (stats.dynamic_multiple.unwrap_or(0.0) / 10.0).clamp(0.0, 1.0) * 0.25,
    );
    let forced_pct =
        (liq_ratio * 1.35 + (liquidation_suspected as u8 as f64) * 0.15).clamp(0.0, 0.80);
    let retail_pct = ((1.0 - forced_pct) * (1.0 - dominance) * 0.45).clamp(0.0, 0.35);
    let whale_pct = (1.0 - forced_pct - retail_pct).clamp(0.0, 1.0);
    let dominant_driver = if forced_pct >= whale_pct && forced_pct >= retail_pct {
        "liquidation_cascade"
    } else if retail_pct > whale_pct {
        "retail_follow_flow"
    } else {
        "whale_initiated_flow"
    }
    .to_string();

    let active_zone = if long_liq_pressure >= 60 && long_liq_pressure >= short_squeeze_pressure {
        "long_liquidation_zone"
    } else if short_squeeze_pressure >= 60 {
        "short_squeeze_zone"
    } else if stop_hunt_probability >= 65 {
        "stop_loss_sweep_zone"
    } else {
        "neutral"
    }
    .to_string();

    let mut zones = Vec::new();
    if long_liq > 0.0 || price_move < 0.0 {
        zones.push(liquidation_zone(
            "long_liquidation",
            stats,
            -1.0,
            long_liq_pressure,
            stats.liquidation_context.liq_notional_usd * share(long_liq, total_liq),
            "downside stop-loss and long liquidation cluster",
        ));
    }
    if short_liq > 0.0 || price_move > 0.0 {
        zones.push(liquidation_zone(
            "short_liquidation",
            stats,
            1.0,
            short_squeeze_pressure,
            stats.liquidation_context.liq_notional_usd * share(short_liq, total_liq),
            "upside stop-loss and short liquidation cluster",
        ));
    }

    let signed_move = if price_move.abs() > f64::EPSILON {
        price_move.signum()
    } else {
        match signal_type {
            ContractWhaleSignalType::AggressiveBuy => 1.0,
            ContractWhaleSignalType::AggressiveSell => -1.0,
            ContractWhaleSignalType::DownsideAbsorption
            | ContractWhaleSignalType::UpsideSuppression => 0.0,
        }
    };
    let whale_impact = signed_move * whale_pct * abs_price_move;
    let liquidation_cascade = signed_move * forced_pct * abs_price_move;
    let stop_loss_sweep = signed_move * (stop_hunt_probability as f64 / 100.0) * abs_price_move;
    let passive_absorption = match signal_type {
        ContractWhaleSignalType::DownsideAbsorption
        | ContractWhaleSignalType::UpsideSuppression => {
            -signed_move * (1.0 - dominance) * abs_price_move
        }
        _ => 0.0,
    };

    ContractWhaleLiquidationForce {
        active_zone,
        primary_driver: dominant_driver.clone(),
        long_liquidation_pressure: long_liq_pressure,
        short_squeeze_pressure,
        stop_hunt_probability,
        cascade_intensity,
        estimated_forced_size_usd: round(stats.liquidation_context.liq_notional_usd, 2),
        zones,
        flow_attribution: ContractWhaleForcedFlowAttribution {
            whale_pct: round(whale_pct, 4),
            retail_pct: round(retail_pct, 4),
            liquidation_pct: round(forced_pct, 4),
            dominant_driver,
        },
        price_impact: ContractWhalePriceImpactAttribution {
            whale_impact: round(whale_impact, 4),
            liquidation_cascade: round(liquidation_cascade, 4),
            stop_loss_sweep: round(stop_loss_sweep, 4),
            passive_absorption: round(passive_absorption, 4),
        },
    }
}

fn build_market_driver(
    stats: &ContractWhaleWindowStats,
    score: u8,
    signal_type: ContractWhaleSignalType,
    price_response_type: ContractWhalePriceResponseType,
    liquidation_force: &ContractWhaleLiquidationForce,
) -> ContractWhaleMarketDriver {
    let flow_liq_pct = liquidation_force
        .flow_attribution
        .liquidation_pct
        .clamp(0.0, 1.0);
    let price_move_abs = stats.price_move_pct.unwrap_or(0.0).abs();
    let dynamic = (stats.dynamic_multiple.unwrap_or(0.0) / 10.0).clamp(0.0, 1.0);
    let max_forced_pressure = liquidation_force
        .long_liquidation_pressure
        .max(liquidation_force.short_squeeze_pressure) as f64
        / 100.0;
    let oi_pressure = stats
        .market_context
        .oi_change_pct
        .map(|value| (value.abs() / 2.0).clamp(0.0, 1.0))
        .unwrap_or(0.0);
    let funding_pressure = stats
        .market_context
        .funding_rate
        .map(|value| (value.abs() / 0.0005).clamp(0.0, 1.0))
        .unwrap_or(0.0);
    let absorption = matches!(
        price_response_type,
        ContractWhalePriceResponseType::DownsideAbsorption
            | ContractWhalePriceResponseType::UpsideResistance
    );
    let trend_follow = matches!(
        price_response_type,
        ContractWhalePriceResponseType::TrendFollowUp
            | ContractWhalePriceResponseType::TrendFollowDown
    );

    let whale_raw = (score as f64 / 100.0) * 0.35
        + stats.dominance.clamp(0.0, 1.0) * 0.30
        + (1.0 - flow_liq_pct) * 0.20
        + (stats.multi_exchange_confirmed as u8 as f64) * 0.10
        + (matches!(
            signal_type,
            ContractWhaleSignalType::AggressiveBuy | ContractWhaleSignalType::AggressiveSell
        ) as u8 as f64)
            * 0.05;
    let liquidity_raw = (liquidation_force.stop_hunt_probability as f64 / 100.0) * 0.30
        + (liquidation_force.cascade_intensity as f64 / 100.0) * 0.20
        + (absorption as u8 as f64) * 0.25
        + (1.0 - stats.dominance.clamp(0.0, 1.0)) * 0.15
        + (price_move_abs / 0.35).clamp(0.0, 1.0) * 0.10;
    let derivatives_raw = flow_liq_pct * 0.45
        + max_forced_pressure * 0.25
        + oi_pressure * 0.20
        + funding_pressure * 0.05
        + (stats.liquidation_driven as u8 as f64) * 0.15;
    let reflexivity_raw = dynamic * 0.25
        + (price_move_abs / 0.5).clamp(0.0, 1.0) * 0.20
        + (trend_follow as u8 as f64) * 0.15
        + stats.dominance.clamp(0.0, 1.0) * 0.10;

    let total = (whale_raw + liquidity_raw + derivatives_raw + reflexivity_raw).max(0.0001);
    let whale_pct = whale_raw / total;
    let liquidity_pct = liquidity_raw / total;
    let derivatives_pct = derivatives_raw / total;
    let reflexivity_pct = reflexivity_raw / total;
    let drivers = [
        ("whale_intent", whale_pct, whale_raw),
        ("liquidity_forcing", liquidity_pct, liquidity_raw),
        ("derivatives_pressure", derivatives_pct, derivatives_raw),
        ("reflexivity_feedback", reflexivity_pct, reflexivity_raw),
    ];
    let (primary_driver, _, _) = drivers
        .iter()
        .max_by(|left, right| {
            left.1
                .partial_cmp(&right.1)
                .unwrap_or(std::cmp::Ordering::Equal)
        })
        .copied()
        .unwrap_or(("whale_intent", whale_pct, whale_raw));
    let market_state = market_driver_state(primary_driver, liquidation_force, stats);
    let components = drivers
        .into_iter()
        .map(|(key, pct, raw)| ContractWhaleMarketDriverComponent {
            key: key.to_string(),
            score: pressure_score(raw.clamp(0.0, 1.0)),
            weight_pct: round(pct, 4),
        })
        .collect();

    ContractWhaleMarketDriver {
        primary_driver: primary_driver.to_string(),
        market_state: market_state.to_string(),
        whale_intent_pct: round(whale_pct, 4),
        liquidity_forcing_pct: round(liquidity_pct, 4),
        derivatives_pressure_pct: round(derivatives_pct, 4),
        reflexivity_pct: round(reflexivity_pct, 4),
        components,
        interpretation: market_driver_interpretation(primary_driver, market_state),
    }
}

fn market_driver_state(
    primary_driver: &str,
    liquidation_force: &ContractWhaleLiquidationForce,
    stats: &ContractWhaleWindowStats,
) -> &'static str {
    match primary_driver {
        "liquidity_forcing" => "liquidity_squeeze_regime",
        "derivatives_pressure" => match liquidation_force.active_zone.as_str() {
            "long_liquidation_zone" => "liquidation_cascade_regime",
            "short_squeeze_zone" => "short_squeeze_regime",
            "stop_loss_sweep_zone" => "stop_hunt_regime",
            _ => "derivatives_pressure_regime",
        },
        "reflexivity_feedback" => "reflexive_trend_phase",
        _ if stats.net_volume_btc < 0.0 => "whale_led_distribution",
        _ => "whale_led_expansion",
    }
}

fn market_driver_interpretation(primary_driver: &str, market_state: &str) -> String {
    match primary_driver {
        "liquidity_forcing" => {
            "价格主要受流动性真空、止损扫单或吸收结构推动，不宜只按主动买卖解释。".to_string()
        }
        "derivatives_pressure" => {
            "价格主要受清算、OI/funding 或衍生品强制流推动，当前更像被迫交易而非纯主力主动流。"
                .to_string()
        }
        "reflexivity_feedback" => {
            "价格移动已进入反馈放大阶段，趋势、成交和参与者反应正在互相强化。".to_string()
        }
        _ if market_state == "whale_led_distribution" => {
            "价格主要由主动卖方资金推动，当前更接近鲸鱼主导的派发/砸盘。".to_string()
        }
        _ => "价格主要由主动鲸鱼资金推动，清算和反馈因素为辅助。".to_string(),
    }
}

fn liquidation_zone(
    side: &str,
    stats: &ContractWhaleWindowStats,
    direction: f64,
    intensity: u8,
    estimated_size_usd: f64,
    reason: &str,
) -> ContractWhaleLiquidationZone {
    let anchor = signal_price_usd(stats);
    let band_pct = ((stats.price_move_pct.unwrap_or(0.0).abs() / 100.0).max(0.0015)).min(0.012);
    let low_price_usd = anchor.map(|price| {
        if direction < 0.0 {
            price * (1.0 - band_pct * 2.0)
        } else {
            price * (1.0 + band_pct)
        }
    });
    let high_price_usd = anchor.map(|price| {
        if direction < 0.0 {
            price * (1.0 - band_pct)
        } else {
            price * (1.0 + band_pct * 2.0)
        }
    });
    ContractWhaleLiquidationZone {
        side: side.to_string(),
        low_price_usd: low_price_usd.map(|value| round(value, 2)),
        high_price_usd: high_price_usd.map(|value| round(value, 2)),
        estimated_size_usd: round(estimated_size_usd.max(0.0), 2),
        intensity,
        reason: reason.to_string(),
    }
}

fn pressure_score(value: f64) -> u8 {
    (value * 100.0).round().clamp(0.0, 100.0) as u8
}

fn share(part: f64, total: f64) -> f64 {
    if total <= f64::EPSILON {
        0.0
    } else {
        (part / total).clamp(0.0, 1.0)
    }
}

fn classify_signal_type(
    stats: &ContractWhaleWindowStats,
    price_response_type: ContractWhalePriceResponseType,
) -> Option<ContractWhaleSignalType> {
    match price_response_type {
        ContractWhalePriceResponseType::DownsideAbsorption => {
            return Some(ContractWhaleSignalType::DownsideAbsorption);
        }
        ContractWhalePriceResponseType::UpsideResistance => {
            return Some(ContractWhaleSignalType::UpsideSuppression);
        }
        ContractWhalePriceResponseType::TrendFollowUp
        | ContractWhalePriceResponseType::TrendFollowDown
        | ContractWhalePriceResponseType::NoClearResponse => {}
    }
    if stats.net_volume_btc > 0.0 {
        Some(ContractWhaleSignalType::AggressiveBuy)
    } else if stats.net_volume_btc < 0.0 {
        Some(ContractWhaleSignalType::AggressiveSell)
    } else {
        None
    }
}

fn classify_price_response(stats: &ContractWhaleWindowStats) -> ContractWhalePriceResponseType {
    let Some(price_move_pct) = stats.price_move_pct else {
        return ContractWhalePriceResponseType::NoClearResponse;
    };
    let reversal = stats.price_reversal_ratio.unwrap_or(0.0);
    if stats.net_volume_btc > 0.0 {
        if stats.dominance >= 0.60 && price_move_pct >= 0.12 {
            ContractWhalePriceResponseType::TrendFollowUp
        } else if stats.dominance >= 0.60 && (price_move_pct < 0.05 || reversal >= 0.50) {
            ContractWhalePriceResponseType::UpsideResistance
        } else {
            ContractWhalePriceResponseType::NoClearResponse
        }
    } else if stats.net_volume_btc < 0.0 {
        if stats.dominance >= 0.60 && price_move_pct <= -0.12 {
            ContractWhalePriceResponseType::TrendFollowDown
        } else if stats.dominance >= 0.60 && (price_move_pct > -0.05 || reversal >= 0.50) {
            ContractWhalePriceResponseType::DownsideAbsorption
        } else {
            ContractWhalePriceResponseType::NoClearResponse
        }
    } else {
        ContractWhalePriceResponseType::NoClearResponse
    }
}

fn classify_severity(
    stats: &ContractWhaleWindowStats,
    signal_type: ContractWhaleSignalType,
    config: &ContractWhaleRuntimeConfig,
    resolution: &ThresholdProfileResolution,
) -> ContractWhaleSeverity {
    let thresholds = config.thresholds_for_symbol_window_with_profile(
        &stats.symbol,
        stats.window_sec,
        resolution.profile,
    );
    if !thresholds.high_btc.is_finite() {
        return ContractWhaleSeverity::Calm;
    }
    let dynamic_multiple = stats.dynamic_multiple.unwrap_or(0.0);
    let same_direction_price_move = same_direction_price_move(stats, signal_type);
    let notional_thresholds = config.notional_thresholds_usd_for_profile(resolution.profile);
    let muted_absorption = matches!(
        signal_type,
        ContractWhaleSignalType::DownsideAbsorption | ContractWhaleSignalType::UpsideSuppression
    ) && (stats.price_move_pct.unwrap_or(0.0).abs() <= 0.05
        || stats.price_reversal_ratio.unwrap_or(0.0) >= 0.50);
    let primary_source_confirmed = active_primary_same_direction(stats, config, resolution);
    let multi_exchange_confirmed = multi_exchange_confirmed_with_config(stats, config, resolution);
    let primary_source_extreme =
        primary_source_extreme_flow(stats, config, resolution, thresholds, notional_thresholds);
    let critical_absolute_fallback = critical_absolute_fallback(
        stats,
        signal_type,
        config,
        thresholds,
        notional_thresholds,
        primary_source_extreme,
    );
    let evidence_fail_closed = config.classification.evidence_fail_closed_enabled;
    let critical_evidence_ok =
        (dynamic_threshold_required(stats.dynamic_multiple, 7.0, evidence_fail_closed)
            && percentile_threshold_pass(stats.percentile_level, 99.5, evidence_fail_closed))
            || critical_absolute_fallback;

    if stats.total_volume_btc >= thresholds.s_btc
        && stats.total_notional_usd >= notional_thresholds.s
        && dynamic_threshold_required(stats.dynamic_multiple, 10.0, evidence_fail_closed)
        && percentile_threshold_pass(stats.percentile_level, 99.9, evidence_fail_closed)
        && stats.dominance >= 0.65
        && stats.data_quality >= config.data_quality.min_critical_quality
        && !runtime_warmup(stats, config)
        && multi_exchange_confirmed
        && same_direction_price_move >= 0.25
    {
        return ContractWhaleSeverity::S;
    }

    if stats.total_volume_btc >= thresholds.critical_btc
        && stats.total_notional_usd >= notional_thresholds.critical
        && critical_evidence_ok
        && stats.dominance >= 0.60
        && stats.data_quality >= config.data_quality.min_critical_quality
        && !runtime_warmup(stats, config)
        && (primary_source_confirmed && (!muted_absorption || multi_exchange_confirmed))
        && (same_direction_price_move >= 0.15 || muted_absorption)
    {
        return ContractWhaleSeverity::Critical;
    }

    let standard_high = stats.total_volume_btc >= thresholds.high_btc
        && stats.total_notional_usd >= notional_thresholds.high
        && dynamic_threshold_pass(stats.dynamic_multiple, 5.0, evidence_fail_closed)
        && percentile_threshold_pass(stats.percentile_level, 99.0, evidence_fail_closed)
        && stats.dominance >= 0.55
        && stats.data_quality >= 65
        && stats.exchange_count >= 1;
    let primary_extreme_high = stats.dynamic_multiple.is_none()
        && primary_source_extreme
        && stats.data_quality >= 65
        && (same_direction_price_move >= 0.10 || muted_absorption);
    if standard_high || primary_extreme_high {
        return ContractWhaleSeverity::High;
    }

    let medium_price_confirmed =
        stats.total_volume_btc >= thresholds.high_btc * 0.6 && same_direction_price_move >= 0.30;
    let medium_dynamic_confirmed = dynamic_multiple >= 5.0 && stats.dominance >= 0.55;
    if medium_price_confirmed || medium_dynamic_confirmed {
        return ContractWhaleSeverity::Medium;
    }
    ContractWhaleSeverity::Calm
}

fn reject_reason_for_calm(
    stats: &ContractWhaleWindowStats,
    signal_type: ContractWhaleSignalType,
    config: &ContractWhaleRuntimeConfig,
    resolution: &ThresholdProfileResolution,
) -> ContractWhaleDetectorRejectReason {
    let thresholds = config.thresholds_for_symbol_window_with_profile(
        &stats.symbol,
        stats.window_sec,
        resolution.profile,
    );
    let notional_thresholds = config.notional_thresholds_usd_for_profile(resolution.profile);
    let dynamic_multiple = stats.dynamic_multiple.unwrap_or(0.0);
    let percentile_level = stats.percentile_level.unwrap_or(0.0);
    let same_direction_price_move = same_direction_price_move(stats, signal_type);

    if runtime_warmup(stats, config) {
        return ContractWhaleDetectorRejectReason::Warmup;
    }
    let medium_volume_threshold = thresholds.high_btc * 0.6;
    let medium_volume_ok = stats.total_volume_btc >= medium_volume_threshold;
    let medium_dynamic_ok = dynamic_multiple >= 5.0 && stats.dominance >= 0.55;

    if !medium_volume_ok && !medium_dynamic_ok {
        return ContractWhaleDetectorRejectReason::BelowVolumeThreshold;
    }
    if stats.total_notional_usd < notional_thresholds.high {
        return ContractWhaleDetectorRejectReason::BelowNotionalThreshold;
    }
    if stats.dynamic_multiple.is_some() && dynamic_multiple < 5.0 {
        return ContractWhaleDetectorRejectReason::DynamicMultipleTooLow;
    }
    if stats.percentile_level.is_some() && percentile_level < 99.0 {
        return ContractWhaleDetectorRejectReason::PercentileTooLow;
    }
    if stats.dominance < 0.55 {
        return ContractWhaleDetectorRejectReason::DominanceTooLow;
    }
    if stats.data_quality < 65 {
        return ContractWhaleDetectorRejectReason::DataQualityTooLow;
    }
    if stats.exchange_count >= 2 && !multi_exchange_confirmed_with_config(stats, config, resolution)
    {
        return ContractWhaleDetectorRejectReason::MultiExchangeNotConfirmed;
    }
    if medium_volume_ok && same_direction_price_move < 0.30 {
        return ContractWhaleDetectorRejectReason::SameDirectionPriceMoveTooLow;
    }
    ContractWhaleDetectorRejectReason::Unknown
}

fn dynamic_threshold_pass(
    dynamic_multiple: Option<f64>,
    required_multiple: f64,
    fail_closed: bool,
) -> bool {
    match dynamic_multiple {
        Some(multiple) => multiple >= required_multiple,
        None => !fail_closed,
    }
}

fn dynamic_threshold_required(
    dynamic_multiple: Option<f64>,
    required_multiple: f64,
    fail_closed: bool,
) -> bool {
    match dynamic_multiple {
        Some(multiple) => multiple >= required_multiple,
        None => !fail_closed,
    }
}

fn critical_absolute_fallback(
    stats: &ContractWhaleWindowStats,
    signal_type: ContractWhaleSignalType,
    config: &ContractWhaleRuntimeConfig,
    thresholds: ContractWhaleThresholds,
    notional_thresholds: ContractWhaleNotionalThresholds,
    primary_source_extreme: bool,
) -> bool {
    if stats.dynamic_multiple.is_some() || !primary_source_extreme || runtime_warmup(stats, config)
    {
        return false;
    }
    let same_direction_price_move = same_direction_price_move(stats, signal_type);
    stats.total_volume_btc >= thresholds.critical_btc
        && stats.total_notional_usd >= notional_thresholds.critical
        && stats.dominance >= 0.70
        && same_direction_price_move >= 0.18
}

fn percentile_threshold_pass(
    percentile_level: Option<f64>,
    required_level: f64,
    fail_closed: bool,
) -> bool {
    match percentile_level {
        Some(level) => level >= required_level,
        None => !fail_closed,
    }
}

fn liquidation_suspected(
    stats: &ContractWhaleWindowStats,
    config: &ContractWhaleRuntimeConfig,
) -> bool {
    stats.liquidation_driven
        || stats
            .liquidation_context
            .liq_to_volume_ratio
            .is_some_and(|ratio| ratio >= 0.25 && stats.liquidation_context.total_liq_btc >= 50.0)
        || liquidation_shape_suspected(stats, config)
}

fn liquidation_shape_suspected(
    stats: &ContractWhaleWindowStats,
    config: &ContractWhaleRuntimeConfig,
) -> bool {
    let price_move = stats.price_move_pct.unwrap_or(0.0).abs();
    let reversal = stats.price_reversal_ratio.unwrap_or(0.0);
    let profile = config.threshold_profile_resolution_for_observed_sources(
        stats.exchanges.iter().map(|item| item.exchange.clone()),
    );
    let thresholds = config.thresholds_for_symbol_window_with_profile(
        &stats.symbol,
        stats.window_sec,
        profile.profile,
    );
    stats.total_volume_btc >= thresholds.critical_btc
        && stats.dominance >= 0.80
        && price_move >= 0.25
        && reversal >= 0.50
}

fn effective_data_quality(
    stats: &ContractWhaleWindowStats,
    config: &ContractWhaleRuntimeConfig,
    resolution: &ThresholdProfileResolution,
) -> u8 {
    let mut quality = stats.data_quality;
    let thresholds = config.thresholds_for_symbol_window_with_profile(
        &stats.symbol,
        stats.window_sec,
        resolution.profile,
    );
    let notional_thresholds = config.notional_thresholds_usd_for_profile(resolution.profile);
    let primary_extreme_missing_dynamic = stats.dynamic_multiple.is_none()
        && primary_source_extreme_flow(stats, config, resolution, thresholds, notional_thresholds);
    if config.active_exchange_count() >= 2
        && stats.exchange_count <= 1
        && !primary_extreme_missing_dynamic
    {
        quality = quality.saturating_sub(config.data_quality.single_exchange_penalty);
    }
    if stats
        .ws_latency_ms
        .is_some_and(|latency| latency > config.data_quality.high_latency_ms)
    {
        quality = quality.saturating_sub(penalty_to_u8(
            config.scoring.penalties.websocket_latency_high,
        ));
    }
    if stats
        .startup_age_ms
        .is_some_and(|age| age < config.data_quality.warmup_ms)
    {
        quality = quality.saturating_sub(penalty_to_u8(config.scoring.penalties.warmup_period));
    }
    if stats.market_context.context_expected && !stats.market_context.ct_val_available {
        let ct_val_penalty = config
            .data_quality
            .ct_val_missing_penalty
            .max(config.okx_instruments.fallback_quality_penalty);
        quality = quality.saturating_sub(ct_val_penalty);
        quality = quality.min(config.data_quality.min_discord_quality.saturating_sub(1));
    }
    if stats.price_jump_anomaly {
        quality =
            quality.saturating_sub(penalty_to_u8(config.scoring.penalties.price_jump_anomaly));
    }
    quality
}

fn primary_source_extreme_discord_candidate(
    stats: &ContractWhaleWindowStats,
    signal_type: ContractWhaleSignalType,
    config: &ContractWhaleRuntimeConfig,
    resolution: &ThresholdProfileResolution,
) -> bool {
    let thresholds = config.thresholds_for_symbol_window_with_profile(
        &stats.symbol,
        stats.window_sec,
        resolution.profile,
    );
    let notional_thresholds = config.notional_thresholds_usd_for_profile(resolution.profile);
    stats.dynamic_multiple.is_none()
        && primary_source_extreme_flow(stats, config, resolution, thresholds, notional_thresholds)
        && stats.data_quality >= config.data_quality.min_discord_quality
        && stats.total_notional_usd >= notional_thresholds.critical
        && same_direction_price_move(stats, signal_type) >= 0.15
        && !stats.liquidation_driven
}

fn primary_source_extreme_flow(
    stats: &ContractWhaleWindowStats,
    config: &ContractWhaleRuntimeConfig,
    resolution: &ThresholdProfileResolution,
    thresholds: ContractWhaleThresholds,
    notional_thresholds: ContractWhaleNotionalThresholds,
) -> bool {
    stats.exchange_count == 1
        && active_primary_same_direction(stats, config, resolution)
        && stats.total_notional_usd >= notional_thresholds.high
        && stats.dominance >= 0.60
        && stats.net_volume_btc.abs() >= (thresholds.high_btc * 0.40).max(500.0)
}

fn runtime_warmup(stats: &ContractWhaleWindowStats, config: &ContractWhaleRuntimeConfig) -> bool {
    stats
        .startup_age_ms
        .is_some_and(|age| age < config.data_quality.warmup_ms)
}

fn penalty_to_u8(value: f64) -> u8 {
    value.round().clamp(0.0, 100.0) as u8
}

fn same_direction_price_move(
    stats: &ContractWhaleWindowStats,
    signal_type: ContractWhaleSignalType,
) -> f64 {
    let price_move_pct = stats.price_move_pct.unwrap_or(0.0);
    match signal_type {
        ContractWhaleSignalType::AggressiveBuy => price_move_pct.max(0.0),
        ContractWhaleSignalType::AggressiveSell => (-price_move_pct).max(0.0),
        ContractWhaleSignalType::DownsideAbsorption
        | ContractWhaleSignalType::UpsideSuppression => 0.0,
    }
}

fn same_direction_exchange_count(
    stats: &ContractWhaleWindowStats,
    config: &ContractWhaleRuntimeConfig,
    resolution: &ThresholdProfileResolution,
) -> usize {
    let net_positive = stats.net_volume_btc > 0.0;
    stats
        .exchanges
        .iter()
        .filter(|item| active_source_contains(resolution, &item.exchange))
        .filter(|item| config.exchange_enabled(&item.exchange))
        .filter(|item| item.total_volume_btc > 0.0)
        .filter(|item| item.dominance >= 0.55)
        .filter(|item| (item.net_volume_btc > 0.0) == net_positive)
        .count()
}

fn active_primary_same_direction(
    stats: &ContractWhaleWindowStats,
    config: &ContractWhaleRuntimeConfig,
    resolution: &ThresholdProfileResolution,
) -> bool {
    let net_positive = stats.net_volume_btc > 0.0;
    stats.exchanges.iter().any(|item| {
        active_source_contains(resolution, &item.exchange)
            && config.exchange_enabled(&item.exchange)
            && matches!(item.exchange.as_str(), "binance" | "okx")
            && item.total_volume_btc > 0.0
            && item.dominance >= 0.55
            && (item.net_volume_btc > 0.0) == net_positive
    })
}

fn multi_exchange_confirmed_with_config(
    stats: &ContractWhaleWindowStats,
    config: &ContractWhaleRuntimeConfig,
    resolution: &ThresholdProfileResolution,
) -> bool {
    if resolution.active_contract_sources.len() < 2 {
        return false;
    }
    let min_confirmed_exchanges = 2;
    stats.exchange_count >= min_confirmed_exchanges
        && same_direction_exchange_count(stats, config, resolution) >= min_confirmed_exchanges
        && active_primary_same_direction(stats, config, resolution)
        && bitfinex_confirmation_ok(stats, resolution)
}

fn bitfinex_confirmation_ok(
    stats: &ContractWhaleWindowStats,
    resolution: &ThresholdProfileResolution,
) -> bool {
    if resolution.profile_name != "binance_bitfinex" || resolution.active_contract_sources.len() < 2
    {
        return true;
    }
    let net_positive = stats.net_volume_btc > 0.0;
    stats.exchanges.iter().any(|item| {
        item.exchange == "bitfinex"
            && item.total_volume_btc >= 20.0
            && item.net_contribution_share >= 0.05
            && (item.net_volume_btc > 0.0) == net_positive
    })
}

fn active_source_contains(resolution: &ThresholdProfileResolution, exchange: &str) -> bool {
    resolution
        .active_contract_sources
        .iter()
        .any(|source| source.as_key().eq_ignore_ascii_case(exchange))
}

fn contract_base_asset(symbol: &str) -> String {
    symbol
        .trim()
        .split(['-', '_', '/', ':'])
        .next()
        .unwrap_or(symbol)
        .trim_end_matches("USDT")
        .trim_end_matches("USD")
        .to_ascii_uppercase()
}

fn direction_for(signal_type: ContractWhaleSignalType) -> ContractWhaleDirection {
    match signal_type {
        ContractWhaleSignalType::AggressiveBuy => ContractWhaleDirection::Buy,
        ContractWhaleSignalType::AggressiveSell => ContractWhaleDirection::Sell,
        ContractWhaleSignalType::DownsideAbsorption => ContractWhaleDirection::Absorption,
        ContractWhaleSignalType::UpsideSuppression => ContractWhaleDirection::Suppression,
    }
}

fn direction_label(direction: ContractWhaleDirection) -> &'static str {
    match direction {
        ContractWhaleDirection::Buy => "buy",
        ContractWhaleDirection::Sell => "sell",
        ContractWhaleDirection::Absorption => "absorption",
        ContractWhaleDirection::Suppression => "suppression",
    }
}

fn final_result_text(
    classification: &super::types::ContractWhaleClassificationV2,
    liquidation_suspected: bool,
) -> String {
    let base = match classification.structure_interpretation {
        super::types::ContractWhaleStructureInterpretation::MainForcePushUp => {
            "多平台主动买入爆发，疑似主力合约拉盘"
        }
        super::types::ContractWhaleStructureInterpretation::MainForceDumpDown => {
            "多平台主动卖出爆发，疑似主力合约砸盘"
        }
        super::types::ContractWhaleStructureInterpretation::ActiveBuyPressure => {
            "多平台主动买入爆发，主动买压待价格确认"
        }
        super::types::ContractWhaleStructureInterpretation::ActiveSellPressure => {
            "多平台主动卖出爆发，主动卖压待价格确认"
        }
        super::types::ContractWhaleStructureInterpretation::DownsideAbsorption => {
            "主动卖出放大但价格未明显下跌，疑似下方承接吸收"
        }
        super::types::ContractWhaleStructureInterpretation::UpsideSuppression => {
            "主动买入放大但价格未明显上涨，疑似上方卖盘压制"
        }
        super::types::ContractWhaleStructureInterpretation::UnclearDirectionalFlow => {
            "多平台合约成交异常，方向暂不明确"
        }
    };
    if liquidation_suspected {
        format!("疑似强平推动，主力确定性降低：{base}")
    } else {
        base.to_string()
    }
}

fn signal_source_role(
    stats: &ContractWhaleWindowStats,
    config: &ContractWhaleRuntimeConfig,
) -> super::types::ContractWhaleSourceRole {
    stats
        .main_exchange
        .as_deref()
        .and_then(|exchange| config.exchange_platform(exchange))
        .map(|platform| platform.market_role(ContractWhaleMarketType::Perp))
        .unwrap_or_default()
}

fn active_source_snapshot(
    stats: &ContractWhaleWindowStats,
    config: &ContractWhaleRuntimeConfig,
    resolution: &ThresholdProfileResolution,
) -> ContractWhaleActiveSources {
    let contract_markets = [
        ContractWhaleMarketType::Perp,
        ContractWhaleMarketType::Level2,
        ContractWhaleMarketType::Funding,
        ContractWhaleMarketType::Oi,
        ContractWhaleMarketType::Liquidation,
    ];
    let contract = config
        .platform_keys()
        .into_iter()
        .flat_map(|exchange| {
            contract_markets.into_iter().filter_map(move |market_type| {
                let platform = config.exchange_platform(&exchange)?;
                if !platform.market_enabled(market_type) {
                    return None;
                }
                let participated = market_type == ContractWhaleMarketType::Perp
                    && stats.exchanges.iter().any(|item| {
                        item.exchange.eq_ignore_ascii_case(&exchange) && item.total_volume_btc > 0.0
                    });
                Some(ContractWhaleActiveSourceEntry {
                    exchange: exchange.clone(),
                    market_type,
                    source_role: platform.market_role(market_type),
                    enabled: platform.market_enabled(market_type),
                    status: snapshot_market_status(platform, market_type, participated),
                    product_id: platform.source_for_market(market_type).product.clone(),
                })
            })
        })
        .collect();

    let spot = config
        .platform_keys()
        .into_iter()
        .filter_map(|exchange| {
            let platform = config.exchange_platform(&exchange)?;
            Some(ContractWhaleActiveSourceEntry {
                exchange,
                market_type: ContractWhaleMarketType::Spot,
                source_role: platform.market_role(ContractWhaleMarketType::Spot),
                enabled: platform.market_enabled(ContractWhaleMarketType::Spot),
                status: snapshot_market_status(platform, ContractWhaleMarketType::Spot, false),
                product_id: platform
                    .source_for_market(ContractWhaleMarketType::Spot)
                    .product
                    .clone(),
            })
        })
        .collect();

    ContractWhaleActiveSources {
        contract,
        spot,
        threshold_profile: resolution.profile_name.clone(),
        threshold_profile_reason: resolution.reason.clone(),
        configured_contract_sources: resolution.configured_keys(),
        eligible_contract_sources: resolution.eligible_keys(),
        active_contract_sources: resolution.active_keys(),
    }
}

fn snapshot_market_status(
    platform: &super::config::ContractWhalePlatformConfig,
    market_type: ContractWhaleMarketType,
    participated: bool,
) -> String {
    if !platform.enabled || !platform.any_market_enabled() {
        return "disabled".to_string();
    }
    if platform.market_enabled(market_type) {
        let source = platform.source_for_market(market_type);
        if market_type == ContractWhaleMarketType::Perp
            && source.requires_auth
            && !source.auth_configured()
        {
            return "auth_missing".to_string();
        }
        if market_type == ContractWhaleMarketType::Perp && source.requires_auth && !participated {
            return "ready".to_string();
        }
        return if participated {
            "active".to_string()
        } else if market_type == ContractWhaleMarketType::Spot
            && !platform.contract_markets_enabled()
        {
            "spot_only".to_string()
        } else {
            "configured".to_string()
        };
    }
    if market_type == ContractWhaleMarketType::Perp
        && platform.market_enabled(ContractWhaleMarketType::Spot)
        && !platform.contract_markets_enabled()
    {
        "spot_only".to_string()
    } else {
        "disabled".to_string()
    }
}

fn price_move_for_window(stats: &ContractWhaleWindowStats, window_sec: u64) -> Option<f64> {
    let price_move_pct = stats.price_move_pct?;
    let matches_window = stats.window_sec == window_sec
        || (window_sec == 30 && (30..60).contains(&stats.window_sec))
        || (window_sec == 30 && stats.window_sec >= 60);
    matches_window.then(|| round(price_move_pct, 4))
}

fn round(value: f64, places: i32) -> f64 {
    let factor = 10_f64.powi(places);
    (value * factor).round() / factor
}
