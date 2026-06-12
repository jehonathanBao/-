use super::{
    config::BinanceAltContractRuntimeConfig,
    discord::{evaluate_alt_contract_discord_gate, AltContractDiscordCooldownStore},
    scoring::{score_alt_contract_signal, AltContractScoreResult},
    types::{
        AltContractContext, AltContractDirection, AltContractScoreBreakdown, AltContractSeverity,
        AltContractSignal, AltContractSignalType, AltContractSourceSnapshot, AltContractSymbolTier,
        AltContractWindowStats,
    },
    LOG_PREFIX, LOG_TARGET,
};

pub fn detect_alt_contract_signal(
    stats: &AltContractWindowStats,
    context: &AltContractContext,
    config: &BinanceAltContractRuntimeConfig,
) -> Option<AltContractSignal> {
    if stats.total_notional_usd <= 0.0 || stats.dominance < 0.45 {
        return None;
    }
    let score = score_alt_contract_signal(stats, context, config);
    let max_score = score.abnormal_score.max(score.build_score);
    if matches!(stats.tier, AltContractSymbolTier::D) && max_score < config.tier_d_min_signal_score
    {
        return None;
    }
    let mut severity = severity_for(stats, max_score, config);
    if tier_d_build_guard(stats, context, &score, config)
        && severity.rank()
            > config
                .tier_d_rules
                .max_severity_without_build_confirmation
                .rank()
    {
        severity = config.tier_d_rules.max_severity_without_build_confirmation;
    }
    if severity.rank() < AltContractSeverity::High.rank() {
        return None;
    }
    let signal_type = classify_signal_type(stats, context);
    let direction = direction_for(signal_type, stats.direction);
    let explain_tags = explain_tags(stats, context, signal_type);
    let abnormal_explanation = abnormal_explanation(&score.breakdown, stats, context);
    let build_explanation = build_explanation(&score.breakdown, stats, context);
    let liquidation_explanation = liquidation_explanation(context);
    let warmup = stats
        .startup_age_ms
        .is_some_and(|age| age < config.data_quality.warmup_ms);
    let mut signal = AltContractSignal {
        id: format!(
            "bacm:{}:{}:{}:{}",
            stats.product_id,
            stats.window_sec,
            stats.ts,
            signal_type_key(signal_type)
        ),
        ts: stats.ts,
        symbol: stats.symbol.clone(),
        product_id: stats.product_id.clone(),
        tier: stats.tier,
        window_sec: stats.window_sec,
        signal_type,
        direction,
        severity,
        abnormal_score: score.abnormal_score,
        build_score: score.build_score,
        direction_bias: score.direction_bias,
        data_quality: stats.data_quality,
        total_volume_base: round(stats.total_volume_base, 4),
        net_volume_base: round(stats.net_volume_base, 4),
        total_notional_usd: round(stats.total_notional_usd, 2),
        trigger_price_usd: stats.trigger_price_usd.map(|value| round(value, 8)),
        dominance: round(stats.dominance, 4),
        price_move_pct: stats.price_move_pct.map(|value| round(value, 4)),
        dynamic_multiple: stats.dynamic_multiple.map(|value| round(value, 3)),
        oi_change_1m_base: context.oi_change_1m_base.map(|value| round(value, 4)),
        oi_change_5m_base: context.oi_change_5m_base.map(|value| round(value, 4)),
        oi_change_pct: context.oi_change_pct.map(|value| round(value, 4)),
        funding_rate: context.funding_rate,
        liquidation_notional_usd: context
            .liquidation_notional_usd
            .map(|value| round(value, 2)),
        liquidation_suspected: context.liquidation_suspected,
        force_order_snapshot: context.force_order_snapshot,
        main_exchange: stats.main_exchange.clone(),
        exchanges: stats.exchanges.clone(),
        score_breakdown: score.breakdown,
        active_sources: active_sources_snapshot(config, stats),
        explain_tags,
        abnormal_explanation,
        build_explanation,
        liquidation_explanation,
        discord_eligible: false,
        discord_would_send: false,
        discord_sent: false,
        discord_sent_at: None,
        discord_reason: "not_evaluated".to_string(),
        final_result: final_result_text(signal_type),
        read_only: true,
        analysis_only: true,
        execution_enabled: false,
    };
    if tier_d_discord_guard(&signal, config) {
        signal.discord_eligible = false;
        signal.discord_would_send = false;
        signal.discord_sent = false;
        signal.discord_sent_at = None;
        signal.discord_reason = "tier_d_guard".to_string();
    } else {
        let gate = evaluate_alt_contract_discord_gate(&signal, &config.discord, warmup);
        signal.discord_eligible = gate.eligible;
        signal.discord_would_send = gate.would_send;
        signal.discord_sent = gate.sent;
        signal.discord_sent_at = gate.sent_at_ms;
        signal.discord_reason = gate.reason;
    }
    tracing::info!(
        target: LOG_TARGET,
        symbol = signal.symbol.as_str(),
        product_id = signal.product_id.as_str(),
        severity = ?signal.severity,
        abnormal_score = signal.abnormal_score,
        build_score = signal.build_score,
        "{} signal generated",
        LOG_PREFIX
    );
    Some(signal)
}

pub fn evaluate_discord_for_signal_with_store(
    signal: &AltContractSignal,
    config: &BinanceAltContractRuntimeConfig,
    warmup: bool,
    store: &AltContractDiscordCooldownStore,
    now: i64,
) -> super::discord::AltContractDiscordGate {
    super::discord::evaluate_alt_contract_discord_gate_with_store(
        signal,
        &config.discord,
        warmup,
        store,
        now,
    )
}

fn severity_for(
    stats: &AltContractWindowStats,
    max_score: u8,
    config: &BinanceAltContractRuntimeConfig,
) -> AltContractSeverity {
    let thresholds = config.thresholds_for_tier(stats.tier);
    let dynamic = stats.dynamic_multiple.unwrap_or(0.0);
    if (max_score >= 90
        || stats.total_notional_usd >= thresholds.s_notional_usd
        || dynamic >= config.dynamic.s_multiple)
        && stats.data_quality >= 70
    {
        AltContractSeverity::S
    } else if max_score >= 75
        || stats.total_notional_usd >= thresholds.critical_notional_usd
        || dynamic >= config.dynamic.critical_multiple
    {
        AltContractSeverity::Critical
    } else if max_score >= 60
        || stats.total_notional_usd >= thresholds.high_notional_usd
        || dynamic >= config.dynamic.high_multiple
    {
        AltContractSeverity::High
    } else if max_score >= 40 {
        AltContractSeverity::Medium
    } else {
        AltContractSeverity::Calm
    }
}

fn tier_d_build_guard(
    stats: &AltContractWindowStats,
    context: &AltContractContext,
    score: &AltContractScoreResult,
    config: &BinanceAltContractRuntimeConfig,
) -> bool {
    matches!(stats.tier, AltContractSymbolTier::D)
        && (score.abnormal_score < config.tier_d_rules.discord_min_abnormal_score
            || score.build_score < config.tier_d_rules.discord_min_build_score
            || (config.tier_d_rules.require_non_liquidation && context.liquidation_suspected))
}

fn tier_d_discord_guard(
    signal: &AltContractSignal,
    config: &BinanceAltContractRuntimeConfig,
) -> bool {
    matches!(signal.tier, AltContractSymbolTier::D)
        && (signal.abnormal_score < config.tier_d_rules.discord_min_abnormal_score
            || signal.build_score < config.tier_d_rules.discord_min_build_score
            || (config.tier_d_rules.require_non_liquidation && signal.liquidation_suspected))
}

fn classify_signal_type(
    stats: &AltContractWindowStats,
    context: &AltContractContext,
) -> AltContractSignalType {
    let price_move = stats
        .price_move_pct
        .or(context.price_move_1m_pct)
        .unwrap_or(0.0);
    if context.liquidation_suspected
        && context
            .liquidation_notional_usd
            .is_some_and(|notional| notional >= 5_000_000.0)
    {
        return AltContractSignalType::LiquidationCascade;
    }
    let oi_up = context
        .oi_change_1m_base
        .or(context.oi_change_5m_base)
        .is_some_and(|change| change > 0.0);
    match stats.direction {
        AltContractDirection::Buy if oi_up && price_move >= -0.05 => {
            AltContractSignalType::MainForceLongBuild
        }
        AltContractDirection::Sell if oi_up && price_move <= 0.05 => {
            AltContractSignalType::MainForceShortBuild
        }
        AltContractDirection::Buy if price_move < 0.05 && stats.dominance >= 0.60 => {
            AltContractSignalType::UpsideResistance
        }
        AltContractDirection::Sell if price_move > -0.05 && stats.dominance >= 0.60 => {
            AltContractSignalType::DownsideAbsorption
        }
        AltContractDirection::Buy => AltContractSignalType::AbnormalPump,
        AltContractDirection::Sell => AltContractSignalType::AbnormalDump,
        _ => AltContractSignalType::UnclearContractAnomaly,
    }
}

fn direction_for(
    signal_type: AltContractSignalType,
    stats_direction: AltContractDirection,
) -> AltContractDirection {
    match signal_type {
        AltContractSignalType::DownsideAbsorption => AltContractDirection::Absorption,
        AltContractSignalType::UpsideResistance => AltContractDirection::Suppression,
        AltContractSignalType::UnclearContractAnomaly => AltContractDirection::Neutral,
        _ => stats_direction,
    }
}

fn active_sources_snapshot(
    config: &BinanceAltContractRuntimeConfig,
    stats: &AltContractWindowStats,
) -> Vec<AltContractSourceSnapshot> {
    vec![AltContractSourceSnapshot {
        exchange: "binance".to_string(),
        market_type: "perp".to_string(),
        role: "primary".to_string(),
        enabled: config.exchange.binance_enabled,
        status: if stats.exchange_count > 0 {
            "active".to_string()
        } else if config.exchange.binance_enabled {
            "waiting_for_trades".to_string()
        } else {
            "disabled".to_string()
        },
    }]
}

fn explain_tags(
    stats: &AltContractWindowStats,
    context: &AltContractContext,
    signal_type: AltContractSignalType,
) -> Vec<String> {
    let mut tags = Vec::new();
    tags.push(signal_type_key(signal_type).to_string());
    if stats.total_notional_usd >= 50_000_000.0 {
        tags.push("large_notional_flow".to_string());
    }
    if stats.dominance >= 0.75 {
        tags.push("strong_directional_imbalance".to_string());
    }
    if stats.dynamic_multiple.is_some_and(|value| value >= 6.0) {
        tags.push("dynamic_volume_spike".to_string());
    }
    if context
        .oi_change_1m_base
        .or(context.oi_change_5m_base)
        .is_some()
    {
        tags.push("oi_context_available".to_string());
    }
    if context.funding_rate.is_some() {
        tags.push("funding_context_available".to_string());
    }
    if context.liquidation_suspected || context.force_order_snapshot {
        tags.push("liquidation_context".to_string());
    }
    if matches!(stats.tier, AltContractSymbolTier::D) {
        tags.push("tier_d_extra_guard".to_string());
    }
    tags
}

fn abnormal_explanation(
    breakdown: &AltContractScoreBreakdown,
    stats: &AltContractWindowStats,
    context: &AltContractContext,
) -> String {
    let dynamic = stats
        .dynamic_multiple
        .map(|value| format!("{value:.1}x"))
        .unwrap_or_else(|| "N/A".to_string());
    let liquidation = if context.liquidation_suspected {
        "，并带有清算上下文"
    } else {
        ""
    };
    format!(
        "异常分主要来自成交额强度 {:.1}、动态倍数 {:.1}、方向集中 {:.1} 和价格/清算冲击 {:.1}{liquidation}；当前名义额约 ${:.1}M，动态倍数 {}。",
        breakdown.volume_score,
        breakdown.dynamic_score,
        breakdown.directional_score,
        breakdown.price_score + breakdown.liquidation_score,
        stats.total_notional_usd / 1_000_000.0,
        dynamic
    )
}

fn build_explanation(
    breakdown: &AltContractScoreBreakdown,
    stats: &AltContractWindowStats,
    context: &AltContractContext,
) -> String {
    let oi_text = context
        .oi_change_pct
        .map(|value| format!("OI 约 {value:+.2}%"))
        .or_else(|| {
            context
                .oi_change_1m_base
                .or(context.oi_change_5m_base)
                .map(|value| format!("OI 变化 {value:+.2}"))
        })
        .unwrap_or_else(|| "OI 暂无确认".to_string());
    let persistence = if context.persistence_windows >= 2 {
        "多窗口持续"
    } else {
        "持续性仍待确认"
    };
    let tier_guard = if matches!(stats.tier, AltContractSymbolTier::D) {
        "；Tier D 币种需要更高建仓确认"
    } else {
        ""
    };
    format!(
        "建仓分综合 OI {:.1}、价格响应 {:.1}、持续性 {:.1} 与 Funding {:.1}；{}，{}{tier_guard}。",
        breakdown.oi_score,
        breakdown.price_score,
        breakdown.persistence_score,
        breakdown.funding_score,
        oi_text,
        persistence
    )
}

fn liquidation_explanation(context: &AltContractContext) -> String {
    if context.liquidation_suspected {
        let notional = context
            .liquidation_notional_usd
            .map(|value| format!("约 ${:.1}M", value / 1_000_000.0))
            .unwrap_or_else(|| "有强平快照".to_string());
        format!("检测到清算上下文（{notional}），该信号优先按行情冲击解释，不直接等同于主力建仓。")
    } else {
        "未检测到明显清算驱动，主力/异常判断主要来自主动成交、OI、价格响应与持续性。".to_string()
    }
}

fn signal_type_key(signal_type: AltContractSignalType) -> &'static str {
    match signal_type {
        AltContractSignalType::MainForceLongBuild => "long_build",
        AltContractSignalType::MainForceShortBuild => "short_build",
        AltContractSignalType::AbnormalPump => "pump",
        AltContractSignalType::AbnormalDump => "dump",
        AltContractSignalType::DownsideAbsorption => "absorption",
        AltContractSignalType::UpsideResistance => "resistance",
        AltContractSignalType::LiquidationCascade => "liquidation",
        AltContractSignalType::UnclearContractAnomaly => "unclear",
    }
}

fn final_result_text(signal_type: AltContractSignalType) -> String {
    match signal_type {
        AltContractSignalType::MainForceLongBuild => {
            "Binance 山寨永续主动买入、OI 上升与价格响应同向，疑似合约主力建多。".to_string()
        }
        AltContractSignalType::MainForceShortBuild => {
            "Binance 山寨永续主动卖出、OI 上升与价格响应同向，疑似合约主力建空。".to_string()
        }
        AltContractSignalType::AbnormalPump => {
            "山寨永续主动买入和成交额异常放大，出现异常拉升候选。".to_string()
        }
        AltContractSignalType::AbnormalDump => {
            "山寨永续主动卖出和成交额异常放大，出现异常下跌候选。".to_string()
        }
        AltContractSignalType::DownsideAbsorption => {
            "主动卖出放大但价格跌不动，疑似下方吸收。".to_string()
        }
        AltContractSignalType::UpsideResistance => {
            "主动买入放大但价格涨不动，疑似上方压制。".to_string()
        }
        AltContractSignalType::LiquidationCascade => {
            "成交与强平快照同时放大，优先标记为清算瀑布，不直接确认主力建仓。".to_string()
        }
        AltContractSignalType::UnclearContractAnomaly => {
            "山寨永续成交异常但确认项不足，暂作为合约异动待确认。".to_string()
        }
    }
}

fn round(value: f64, places: i32) -> f64 {
    let factor = 10_f64.powi(places);
    (value * factor).round() / factor
}
