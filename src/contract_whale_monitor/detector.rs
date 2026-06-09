use super::{
    config::{
        contract_whale_runtime_config, ContractWhaleRuntimeConfig, ThresholdProfileResolution,
    },
    log_events,
    scoring::{discord_gate, score_contract_whale_signal_with_profile},
    types::{
        ContractWhaleActiveSourceEntry, ContractWhaleActiveSources, ContractWhaleDirection,
        ContractWhaleMarketType, ContractWhaleSeverity, ContractWhaleSignal,
        ContractWhaleSignalType, ContractWhaleWindowStats,
    },
    LOG_PREFIX, LOG_TARGET,
};

pub fn detect_contract_whale_signal(
    stats: &ContractWhaleWindowStats,
) -> Option<ContractWhaleSignal> {
    detect_contract_whale_signal_with_config(stats, &contract_whale_runtime_config())
}

pub fn detect_contract_whale_signal_with_config(
    stats: &ContractWhaleWindowStats,
    config: &ContractWhaleRuntimeConfig,
) -> Option<ContractWhaleSignal> {
    if !config.symbol_enabled(&stats.symbol) {
        return None;
    }
    let resolution = config.threshold_profile_resolution_for_observed_sources(
        stats.exchanges.iter().map(|item| item.exchange.clone()),
    );
    if resolution.active_contract_sources.is_empty() {
        return None;
    }
    let mut scoring_stats = stats.clone();
    scoring_stats.data_quality = effective_data_quality(&scoring_stats, config);
    if scoring_stats.total_volume_btc <= 0.0 || scoring_stats.data_quality < 50 {
        return None;
    }
    let signal_type = classify_signal_type(&scoring_stats)?;
    let liquidation_suspected = liquidation_suspected(&scoring_stats, config);
    scoring_stats.liquidation_driven = liquidation_suspected;
    let severity = classify_severity(&scoring_stats, signal_type, config, &resolution);
    if severity == ContractWhaleSeverity::Calm {
        return None;
    }
    let score = score_contract_whale_signal_with_profile(
        &scoring_stats,
        signal_type,
        config,
        resolution.profile,
    );
    let multi_exchange_confirmed =
        multi_exchange_confirmed_with_config(&scoring_stats, config, &resolution);
    let warmup_collect_only = runtime_warmup(&scoring_stats, config);
    let (mut discord_eligible, mut discord_reason) = discord_gate(
        severity,
        score,
        multi_exchange_confirmed,
        scoring_stats.data_quality,
    );
    if warmup_collect_only {
        discord_eligible = false;
        discord_reason = "warmup_collect_only".to_string();
    }
    let direction = direction_for(signal_type);

    let active_sources = active_source_snapshot(&scoring_stats, config, &resolution);
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
        total_volume_btc: round(stats.total_volume_btc, 3),
        net_volume_btc: round(stats.net_volume_btc, 3),
        total_notional_usd: round(stats.total_notional_usd, 2),
        dominance: round(stats.dominance, 4),
        price_move_pct: scoring_stats.price_move_pct.map(|value| round(value, 4)),
        main_exchange: scoring_stats.main_exchange.clone(),
        market_type: ContractWhaleMarketType::Perp,
        source_role: signal_source_role(&scoring_stats, config),
        exchanges: scoring_stats.exchanges.clone(),
        dominant_venue_net_contribution_share: scoring_stats
            .dominant_venue_net_contribution_share
            .map(|value| round(value, 4)),
        dynamic_multiple: scoring_stats.dynamic_multiple.map(|value| round(value, 3)),
        percentile_level: scoring_stats.percentile_level.map(|value| round(value, 1)),
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
        threshold_profile: resolution.profile_name.clone(),
        threshold_profile_reason: resolution.reason.clone(),
        configured_contract_sources: resolution.configured_keys(),
        eligible_contract_sources: resolution.eligible_keys(),
        active_contract_sources: resolution.active_keys(),
        active_sources,
        discord_eligible,
        discord_sent: false,
        discord_sent_at: None,
        discord_reason,
        final_result: final_result_text(signal_type, liquidation_suspected),
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
        discord_eligible = signal.discord_eligible,
        "{} signal generated",
        LOG_PREFIX
    );
    Some(signal)
}

fn classify_signal_type(stats: &ContractWhaleWindowStats) -> Option<ContractWhaleSignalType> {
    let price_move_pct = stats.price_move_pct.unwrap_or(0.0);
    let reversal = stats.price_reversal_ratio.unwrap_or(0.0);
    if stats.net_volume_btc > 0.0 {
        if stats.dominance >= 0.60 && (price_move_pct < 0.05 || reversal >= 0.50) {
            Some(ContractWhaleSignalType::UpsideSuppression)
        } else {
            Some(ContractWhaleSignalType::AggressiveBuy)
        }
    } else if stats.net_volume_btc < 0.0 {
        if stats.dominance >= 0.60 && (price_move_pct > -0.05 || reversal >= 0.50) {
            Some(ContractWhaleSignalType::DownsideAbsorption)
        } else {
            Some(ContractWhaleSignalType::AggressiveSell)
        }
    } else {
        None
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

    if stats.total_volume_btc >= thresholds.s_btc
        && stats.total_notional_usd >= notional_thresholds.s
        && dynamic_threshold_required(stats.dynamic_multiple, 10.0)
        && percentile_threshold_pass(stats.percentile_level, 99.9)
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
        && dynamic_threshold_required(stats.dynamic_multiple, 7.0)
        && percentile_threshold_pass(stats.percentile_level, 99.5)
        && stats.dominance >= 0.60
        && stats.data_quality >= config.data_quality.min_critical_quality
        && !runtime_warmup(stats, config)
        && (primary_source_confirmed && (!muted_absorption || multi_exchange_confirmed))
        && (same_direction_price_move >= 0.15 || muted_absorption)
    {
        return ContractWhaleSeverity::Critical;
    }

    if stats.total_volume_btc >= thresholds.high_btc
        && stats.total_notional_usd >= notional_thresholds.high
        && dynamic_threshold_pass(stats.dynamic_multiple, 5.0)
        && percentile_threshold_pass(stats.percentile_level, 99.0)
        && stats.dominance >= 0.55
        && stats.data_quality >= 65
        && stats.exchange_count >= 1
    {
        return ContractWhaleSeverity::High;
    }

    if stats.total_volume_btc >= thresholds.high_btc * 0.5 || dynamic_multiple >= 4.0 {
        return ContractWhaleSeverity::Medium;
    }
    ContractWhaleSeverity::Calm
}

fn dynamic_threshold_pass(dynamic_multiple: Option<f64>, required_multiple: f64) -> bool {
    match dynamic_multiple {
        Some(multiple) => multiple >= required_multiple,
        None => true,
    }
}

fn dynamic_threshold_required(dynamic_multiple: Option<f64>, required_multiple: f64) -> bool {
    match dynamic_multiple {
        Some(multiple) => multiple >= required_multiple,
        None => false,
    }
}

fn percentile_threshold_pass(percentile_level: Option<f64>, required_level: f64) -> bool {
    match percentile_level {
        Some(level) => level >= required_level,
        None => true,
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
) -> u8 {
    let mut quality = stats.data_quality;
    if config.active_exchange_count() >= 2 && stats.exchange_count <= 1 {
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
        quality = quality.saturating_sub(config.data_quality.ct_val_missing_penalty);
        quality = quality.min(config.data_quality.min_discord_quality.saturating_sub(1));
    }
    if stats.price_jump_anomaly {
        quality =
            quality.saturating_sub(penalty_to_u8(config.scoring.penalties.price_jump_anomaly));
    }
    quality
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

fn final_result_text(signal_type: ContractWhaleSignalType, liquidation_suspected: bool) -> String {
    let base = match signal_type {
        ContractWhaleSignalType::AggressiveBuy => "多平台主动买入爆发，疑似主力合约拉盘",
        ContractWhaleSignalType::AggressiveSell => "多平台主动卖出爆发，疑似主力合约砸盘",
        ContractWhaleSignalType::DownsideAbsorption => {
            "主动卖出放大但价格未明显下跌，疑似下方承接吸收"
        }
        ContractWhaleSignalType::UpsideSuppression => {
            "主动买入放大但价格未明显上涨，疑似上方卖盘压制"
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

fn round(value: f64, places: i32) -> f64 {
    let factor = 10_f64.powi(places);
    (value * factor).round() / factor
}
