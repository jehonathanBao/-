use super::{
    config::BinanceAltContractRuntimeConfig,
    discord::{evaluate_alt_contract_discord_gate, AltContractDiscordCooldownStore},
    scoring::{
        funding_crowding_label, funding_crowding_penalty, score_alt_contract_signal,
        AltContractScoreResult,
    },
    types::{
        AltContractContext, AltContractDirection, AltContractScoreBreakdown, AltContractSeverity,
        AltContractSignal, AltContractSignalType, AltContractSourceSnapshot, AltContractSymbolTier,
        AltContractWindowConfirmation, AltContractWindowStats,
    },
    LOG_PREFIX, LOG_TARGET,
};

pub fn detect_alt_contract_signal(
    stats: &AltContractWindowStats,
    context: &AltContractContext,
    config: &BinanceAltContractRuntimeConfig,
) -> Option<AltContractSignal> {
    detect_alt_contract_signal_with_context(
        stats,
        context,
        config,
        vec![window_confirmation_for(stats, config)],
        MarketImpulseContext::default(),
    )
}

#[derive(Debug, Clone, Default)]
pub struct MarketImpulseContext {
    pub market_wide_move: bool,
    pub market_wide_direction: Option<String>,
    pub market_impulse_ratio: f64,
    pub relative_strength_rank: Option<u32>,
}

pub fn detect_alt_contract_signal_with_context(
    stats: &AltContractWindowStats,
    context: &AltContractContext,
    config: &BinanceAltContractRuntimeConfig,
    window_confirmations: Vec<AltContractWindowConfirmation>,
    market_context: MarketImpulseContext,
) -> Option<AltContractSignal> {
    if stats.total_notional_usd <= 0.0 || stats.dominance < 0.45 {
        return None;
    }
    let score = score_alt_contract_signal(stats, context, config);
    let evidence = evidence_for(
        stats,
        context,
        config,
        &window_confirmations,
        &market_context,
    );
    let main_force_confidence =
        main_force_confidence(stats, context, &score, &evidence, &market_context);
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
    if tier_e_build_guard(stats, context, &score, config)
        && severity.rank()
            > config
                .tier_e_rules
                .max_severity_without_build_confirmation
                .rank()
    {
        severity = config.tier_e_rules.max_severity_without_build_confirmation;
    }
    if severity.rank() < AltContractSeverity::High.rank() {
        return None;
    }
    let signal_type = classify_signal_type(stats, context, &evidence, main_force_confidence);
    let direction = direction_for(signal_type, stats.direction);
    let explain_tags = explain_tags(stats, context, signal_type);
    let abnormal_explanation = abnormal_explanation(&score.breakdown, stats, context);
    let build_explanation = build_explanation(&score.breakdown, stats, context, &evidence);
    let liquidation_explanation = liquidation_explanation(context);
    let funding_crowding = funding_crowding_label(stats, context);
    let funding_penalty = funding_crowding_penalty(stats, context);
    let signal_vwap = stats.trigger_price_usd.unwrap_or_default();
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
        main_force_confidence: round(main_force_confidence, 2),
        evidence_count: evidence.tags.len().min(u8::MAX as usize) as u8,
        evidence_tags: evidence.tags,
        window_confirmations,
        market_wide_move: market_context.market_wide_move,
        market_wide_direction: market_context.market_wide_direction,
        market_impulse_ratio: round(market_context.market_impulse_ratio, 4),
        relative_strength_rank: market_context.relative_strength_rank,
        post_signal_status: "pending".to_string(),
        validated_at: None,
        failed_at: None,
        signal_vwap: round(signal_vwap, 8),
        retest_status: "unknown".to_string(),
        oi_freshness_sec: context
            .oi_updated_at
            .map(|seen_at| stats.ts.saturating_sub(seen_at).max(0) as u64 / 1000),
        oi_change_1m_pct: context.oi_change_pct.map(|value| round(value, 4)),
        oi_change_5m_pct: context.oi_change_pct.map(|value| round(value, 4)),
        oi_change_15m_pct: None,
        oi_notional_change_usd: context
            .oi_change_1m_base
            .or(context.oi_change_5m_base)
            .zip(stats.trigger_price_usd)
            .map(|(change, price)| round(change * price, 2)),
        oi_quality: oi_quality(stats, context),
        funding_crowding,
        funding_penalty: round(funding_penalty, 2),
        spread_bps: None,
        depth_0_5pct_usd: None,
        depth_1pct_usd: None,
        flow_to_depth_ratio: None,
        event_id: None,
        event_signal_count: 0,
        event_peak_abnormal_score: score.abnormal_score,
        event_peak_build_score: score.build_score,
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
        final_result: final_result_text(signal_type, main_force_confidence, context),
        read_only: true,
        analysis_only: true,
        execution_enabled: false,
    };
    if tier_e_discord_guard(&signal, config) {
        signal.discord_eligible = false;
        signal.discord_would_send = false;
        signal.discord_sent = false;
        signal.discord_sent_at = None;
        signal.discord_reason = "low_liquidity_tier_guard".to_string();
    } else if tier_d_discord_guard(&signal, config) {
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

#[derive(Debug, Clone, Default)]
struct EvidenceResult {
    tags: Vec<String>,
}

pub fn window_confirmation_for(
    stats: &AltContractWindowStats,
    config: &BinanceAltContractRuntimeConfig,
) -> AltContractWindowConfirmation {
    let thresholds = config.thresholds_for_tier(stats.tier);
    let confirmed = stats.total_notional_usd >= thresholds.high_notional_usd
        || stats
            .dynamic_multiple
            .is_some_and(|value| value >= config.dynamic.high_multiple)
        || (stats.dominance >= 0.60
            && stats.total_notional_usd >= thresholds.high_notional_usd * 0.6);
    AltContractWindowConfirmation {
        window_sec: stats.window_sec,
        notional_usd: stats.total_notional_usd,
        dynamic_multiple: stats.dynamic_multiple,
        directional_strength: stats.dominance,
        confirmed,
    }
}

fn evidence_for(
    stats: &AltContractWindowStats,
    context: &AltContractContext,
    config: &BinanceAltContractRuntimeConfig,
    windows: &[AltContractWindowConfirmation],
    market_context: &MarketImpulseContext,
) -> EvidenceResult {
    let mut tags = Vec::new();
    if stats.direction == AltContractDirection::Buy && stats.dominance >= 0.60 {
        tags.push("aggressive_buy_dominant".to_string());
    }
    if stats.direction == AltContractDirection::Sell && stats.dominance >= 0.60 {
        tags.push("aggressive_sell_dominant".to_string());
    }
    if stats
        .dynamic_multiple
        .is_some_and(|value| value >= config.dynamic.critical_multiple)
    {
        tags.push("dynamic_multiple_critical".to_string());
    } else if stats
        .dynamic_multiple
        .is_some_and(|value| value >= config.dynamic.high_multiple)
    {
        tags.push("dynamic_multiple_high".to_string());
    }
    if context
        .oi_change_1m_base
        .or(context.oi_change_5m_base)
        .is_some_and(|value| value > 0.0)
    {
        tags.push("oi_expanding".to_string());
    }
    if context
        .oi_change_1m_base
        .or(context.oi_change_5m_base)
        .is_some_and(|value| value < 0.0)
    {
        tags.push("oi_contracting".to_string());
    }
    let price_move = stats
        .price_move_pct
        .or(context.price_move_1m_pct)
        .unwrap_or_default();
    if (stats.direction == AltContractDirection::Buy && price_move > 0.05)
        || (stats.direction == AltContractDirection::Sell && price_move < -0.05)
    {
        tags.push("price_follow_through".to_string());
    }
    if (stats.direction == AltContractDirection::Sell && price_move > -0.05)
        || (stats.direction == AltContractDirection::Buy && price_move < 0.05)
    {
        tags.push("price_absorption".to_string());
    }
    if !context.liquidation_suspected && !context.force_order_snapshot {
        tags.push("not_liquidation_driven".to_string());
    }
    if funding_crowding_penalty(stats, context) == 0.0 {
        tags.push("funding_not_overcrowded".to_string());
    }
    let confirmed_windows = windows.iter().filter(|window| window.confirmed).count();
    if confirmed_windows >= 2 || context.persistence_windows >= 2 {
        tags.push("multi_window_confirmed".to_string());
    }
    if !market_context.market_wide_move {
        match stats.direction {
            AltContractDirection::Buy => tags.push("market_relative_strength".to_string()),
            AltContractDirection::Sell => tags.push("market_relative_weakness".to_string()),
            _ => {}
        }
    } else if market_context
        .relative_strength_rank
        .is_some_and(|rank| rank <= 10)
    {
        match stats.direction {
            AltContractDirection::Buy => tags.push("market_relative_strength".to_string()),
            AltContractDirection::Sell => tags.push("market_relative_weakness".to_string()),
            _ => {}
        }
    }
    EvidenceResult { tags }
}

fn main_force_confidence(
    stats: &AltContractWindowStats,
    context: &AltContractContext,
    score: &AltContractScoreResult,
    evidence: &EvidenceResult,
    market_context: &MarketImpulseContext,
) -> f64 {
    let mut confidence =
        f64::from(score.build_score) * 0.55 + f64::from(evidence.tags.len() as u8) * 6.0;
    if evidence
        .tags
        .iter()
        .any(|tag| tag == "multi_window_confirmed")
    {
        confidence += 8.0;
    }
    if context.liquidation_suspected || context.force_order_snapshot {
        confidence -= 18.0;
    }
    if market_context.market_wide_move
        && !market_context
            .relative_strength_rank
            .is_some_and(|rank| rank <= 10)
    {
        confidence -= 15.0;
    }
    confidence -= funding_crowding_penalty(stats, context);
    if oi_quality(stats, context) != "fresh"
        && context
            .oi_change_1m_base
            .or(context.oi_change_5m_base)
            .is_none()
    {
        confidence -= 10.0;
    }
    confidence.clamp(0.0, 100.0)
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

fn tier_e_build_guard(
    stats: &AltContractWindowStats,
    context: &AltContractContext,
    score: &AltContractScoreResult,
    config: &BinanceAltContractRuntimeConfig,
) -> bool {
    let oi_confirmed = context
        .oi_change_1m_base
        .or(context.oi_change_5m_base)
        .is_some_and(|change| change > 0.0);
    matches!(stats.tier, AltContractSymbolTier::E)
        && (score.abnormal_score < config.tier_e_rules.discord_min_abnormal_score
            || score.build_score < config.tier_e_rules.discord_min_build_score
            || stats.dynamic_multiple.unwrap_or(0.0) < config.tier_e_rules.min_dynamic_multiple
            || (config.tier_e_rules.require_oi_confirmation && !oi_confirmed)
            || (config.tier_e_rules.require_non_liquidation && context.liquidation_suspected))
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

fn tier_e_discord_guard(
    signal: &AltContractSignal,
    config: &BinanceAltContractRuntimeConfig,
) -> bool {
    let oi_confirmed = signal
        .oi_change_1m_base
        .or(signal.oi_change_5m_base)
        .is_some_and(|change| change > 0.0);
    matches!(signal.tier, AltContractSymbolTier::E)
        && (signal.abnormal_score < config.tier_e_rules.discord_min_abnormal_score
            || signal.build_score < config.tier_e_rules.discord_min_build_score
            || signal.dynamic_multiple.unwrap_or(0.0) < config.tier_e_rules.min_dynamic_multiple
            || (config.tier_e_rules.require_oi_confirmation && !oi_confirmed)
            || (config.tier_e_rules.require_non_liquidation && signal.liquidation_suspected))
}

fn classify_signal_type(
    stats: &AltContractWindowStats,
    context: &AltContractContext,
    evidence: &EvidenceResult,
    main_force_confidence: f64,
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
    let strong_main_force_chain =
        evidence.tags.len() >= 4 && main_force_confidence >= 75.0 && !context.liquidation_suspected;
    let oi_up = context
        .oi_change_1m_base
        .or(context.oi_change_5m_base)
        .is_some_and(|change| change > 0.0);
    match stats.direction {
        AltContractDirection::Buy if price_move < 0.05 && stats.dominance >= 0.60 => {
            AltContractSignalType::UpsideResistance
        }
        AltContractDirection::Sell if price_move > -0.05 && stats.dominance >= 0.60 => {
            AltContractSignalType::DownsideAbsorption
        }
        AltContractDirection::Buy if strong_main_force_chain && oi_up && price_move >= -0.05 => {
            AltContractSignalType::MainForceLongBuild
        }
        AltContractDirection::Sell if strong_main_force_chain && oi_up && price_move <= 0.05 => {
            AltContractSignalType::MainForceShortBuild
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
    if matches!(stats.tier, AltContractSymbolTier::E) {
        tags.push("tier_e_low_liquidity_guard".to_string());
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
    evidence: &EvidenceResult,
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
    let tier_guard = if matches!(stats.tier, AltContractSymbolTier::E) {
        "；Tier E 低流动性币种默认只展示，需要极高建仓确认才进入推送 gate"
    } else if matches!(stats.tier, AltContractSymbolTier::D) {
        "；Tier D 币种需要更高建仓确认"
    } else {
        ""
    };
    let evidence_text = if evidence.tags.len() >= 4 {
        format!("证据链满足 {} 项", evidence.tags.len())
    } else {
        format!("证据链仅 {} 项，暂不直接确认主力建仓", evidence.tags.len())
    };
    format!(
        "建仓分综合 OI {:.1}、价格响应 {:.1}、持续性 {:.1} 与 Funding {:.1}；{}，{}；{}{tier_guard}。",
        breakdown.oi_score,
        breakdown.price_score,
        breakdown.persistence_score,
        breakdown.funding_score,
        oi_text,
        persistence,
        evidence_text
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

fn oi_quality(stats: &AltContractWindowStats, context: &AltContractContext) -> String {
    let Some(updated_at) = context.oi_updated_at else {
        return "missing".to_string();
    };
    let freshness_sec = stats.ts.saturating_sub(updated_at).max(0) / 1000;
    if freshness_sec <= 60 {
        "fresh".to_string()
    } else {
        "stale".to_string()
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

fn final_result_text(
    signal_type: AltContractSignalType,
    main_force_confidence: f64,
    context: &AltContractContext,
) -> String {
    match signal_type {
        AltContractSignalType::MainForceLongBuild => {
            format!("Binance 山寨永续主动买入、OI 上升与价格响应同向，疑似合约主力建多；主力置信度 {:.0}/100。", main_force_confidence)
        }
        AltContractSignalType::MainForceShortBuild => {
            format!("Binance 山寨永续主动卖出、OI 上升与价格响应同向，疑似合约主力建空；主力置信度 {:.0}/100。", main_force_confidence)
        }
        AltContractSignalType::AbnormalPump => {
            if context
                .oi_change_1m_base
                .or(context.oi_change_5m_base)
                .is_none()
            {
                "山寨永续主动买入和成交额异常放大，但 OI 尚未确认，先标记为合约异常冲击。"
                    .to_string()
            } else {
                "山寨永续主动买入和成交额异常放大，建仓证据不足，先标记为异常拉升候选。".to_string()
            }
        }
        AltContractSignalType::AbnormalDump => {
            if context
                .oi_change_1m_base
                .or(context.oi_change_5m_base)
                .is_none()
            {
                "山寨永续主动卖出和成交额异常放大，但 OI 尚未确认，先标记为合约异常冲击。"
                    .to_string()
            } else {
                "山寨永续主动卖出和成交额异常放大，建仓证据不足，先标记为异常下跌候选。".to_string()
            }
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
