use super::{
    config::BinanceAltContractRuntimeConfig,
    context::context_for_window,
    discord::{evaluate_alt_contract_discord_gate, AltContractDiscordCooldownStore},
    impact::{impact_s_ready, score_alt_impact},
    lme::score_signal_microstructure,
    mcg::build_market_control_graph,
    mcss::score_master_capital_strength,
    regime::classify_market_regime,
    scc::calibrate_signal_confidence,
    scoring::{
        funding_crowding_label, funding_crowding_penalty, score_alt_contract_signal,
        AltContractScoreResult,
    },
    semantic::{apply_semantic_boundary, evaluate_exposure_gate},
    smle::classify_smart_money_lifecycle,
    smp::predict_smart_money_next_stage,
    types::{
        AltContractContext, AltContractDirection, AltContractGradeCondition,
        AltContractImpactScore, AltContractScoreBreakdown, AltContractSeverity, AltContractSignal,
        AltContractSignalType, AltContractSourceSnapshot, AltContractStructureConfidence,
        AltContractSymbolTier, AltContractWindowConfirmation, AltContractWindowStats,
        AltSignalAssessment,
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
    let selected_context = context_for_window(context, stats.window_sec);
    let context = &selected_context;
    let score = score_alt_contract_signal(stats, context, config);
    let market_tier = config.classify_market_tier(&stats.product_id);
    let alt_impact_score = score_alt_impact(stats, context, market_tier);
    let liquidity_microstructure = score_signal_microstructure(stats, context);
    let evidence = evidence_for(
        stats,
        context,
        config,
        &window_confirmations,
        &market_context,
    );
    let main_force_confidence =
        main_force_confidence(stats, context, &score, &evidence, &market_context);
    let s_grade = s_grade_evaluation(stats, context, &score, config, &alt_impact_score);
    let max_score = score.abnormal_score.max(score.build_score);
    if matches!(stats.tier, AltContractSymbolTier::D) && max_score < config.tier_d_min_signal_score
    {
        return None;
    }
    let mut severity = severity_for(stats, max_score, config, s_grade.eligible);
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
    let display_threshold_usd = config.display.threshold_for_market_tier(market_tier);
    let master_capital_strength = score_master_capital_strength(stats, context, market_tier);
    let market_regime = classify_market_regime(
        stats,
        context,
        &master_capital_strength,
        &window_confirmations,
        &market_context,
    );
    let smart_money_lifecycle = classify_smart_money_lifecycle(
        stats,
        context,
        &master_capital_strength,
        &market_regime,
        &window_confirmations,
        None,
    );
    let market_control_graph = build_market_control_graph(
        stats,
        context,
        &liquidity_microstructure,
        &master_capital_strength,
        &market_regime,
        &smart_money_lifecycle,
    );
    let smart_money_prediction = predict_smart_money_next_stage(
        stats,
        context,
        &master_capital_strength,
        &smart_money_lifecycle,
        &market_regime,
    );
    let signal_confidence = calibrate_signal_confidence(
        stats,
        context,
        signal_type,
        score.abnormal_score,
        score.build_score,
        severity,
        &master_capital_strength,
        &smart_money_lifecycle,
        &smart_money_prediction,
        &liquidity_microstructure,
        &market_control_graph,
        market_context.market_wide_move,
    );
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
        market_tier,
        display_threshold_usd: round(display_threshold_usd, 2),
        window_sec: stats.window_sec,
        signal_type,
        direction,
        severity,
        assessment: assess_signal(stats, context, severity, main_force_confidence),
        abnormal_score: score.abnormal_score,
        build_score: score.build_score,
        master_capital_strength,
        alt_impact_score,
        liquidity_microstructure,
        market_control_graph,
        market_regime,
        smart_money_lifecycle,
        smart_money_prediction,
        signal_confidence,
        s_grade_eligible: s_grade.eligible,
        s_grade_conditions: s_grade.conditions,
        s_grade_notional_threshold_usd: round(s_grade.notional_threshold_usd, 2),
        s_grade_volume_threshold_base: round(s_grade.volume_threshold_base, 4),
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
        oi_change_1m_pct: context
            .oi_change_1m
            .delta_pct
            .or(context.oi_change_pct)
            .map(|value| round(value, 4)),
        oi_change_5m_pct: context
            .oi_change_5m
            .delta_pct
            .or(context.oi_change_pct)
            .map(|value| round(value, 4)),
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
        discord_alert_kind: "none".to_string(),
        discord_min_notional_usd: 0.0,
        semantic: Default::default(),
        final_result: final_result_text(signal_type, main_force_confidence, context),
        read_only: true,
        analysis_only: true,
        execution_enabled: false,
    };
    let exposure_decision = evaluate_exposure_gate(&signal, warmup);
    apply_semantic_boundary(&mut signal, exposure_decision);
    signal.assessment.exposure_tier = if signal.semantic.exposure_allowed {
        super::types::AltContractExposureTier::Alert
    } else if signal.severity.rank() >= AltContractSeverity::High.rank() {
        super::types::AltContractExposureTier::Highlight
    } else {
        super::types::AltContractExposureTier::Observe
    };
    if tier_e_discord_guard(&signal, config) {
        signal.discord_eligible = false;
        signal.discord_would_send = false;
        signal.discord_sent = false;
        signal.discord_sent_at = None;
        signal.discord_reason = "low_liquidity_tier_guard".to_string();
        signal.discord_alert_kind = "none".to_string();
        signal.discord_min_notional_usd = config
            .discord
            .tier_thresholds
            .get(&signal.tier)
            .map(|tier| tier.min_notional_usd)
            .unwrap_or_default();
        signal.semantic.exposure_allowed = false;
        signal.semantic.exposure_reason = "low_liquidity_tier_guard".to_string();
        signal.semantic.layer = super::types::AltContractSemanticLayer::Interpretation;
    } else if tier_d_discord_guard(&signal, config) {
        signal.discord_eligible = false;
        signal.discord_would_send = false;
        signal.discord_sent = false;
        signal.discord_sent_at = None;
        signal.discord_reason = "tier_d_guard".to_string();
        signal.discord_alert_kind = "none".to_string();
        signal.discord_min_notional_usd = config
            .discord
            .tier_thresholds
            .get(&signal.tier)
            .map(|tier| tier.min_notional_usd)
            .unwrap_or_default();
        signal.semantic.exposure_allowed = false;
        signal.semantic.exposure_reason = "tier_d_guard".to_string();
        signal.semantic.layer = super::types::AltContractSemanticLayer::Interpretation;
    } else if !signal.semantic.exposure_allowed {
        signal.discord_eligible = false;
        signal.discord_would_send = false;
        signal.discord_sent = false;
        signal.discord_sent_at = None;
        signal.discord_reason = signal.semantic.exposure_reason.clone();
        signal.discord_alert_kind = "none".to_string();
        signal.discord_min_notional_usd = 0.0;
    } else {
        let gate = evaluate_alt_contract_discord_gate(&signal, &config.discord, warmup);
        signal.discord_eligible = gate.eligible;
        signal.discord_would_send = gate.would_send;
        signal.discord_sent = gate.sent;
        signal.discord_sent_at = gate.sent_at_ms;
        signal.discord_reason = gate.reason;
        signal.discord_alert_kind = gate.alert_kind;
        signal.discord_min_notional_usd = round(gate.min_notional_usd, 2);
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

fn assess_signal(
    stats: &AltContractWindowStats,
    context: &AltContractContext,
    severity: AltContractSeverity,
    main_force_confidence: f64,
) -> AltSignalAssessment {
    let structure_confidence = if main_force_confidence >= 80.0
        && !context.liquidation_suspected
        && context
            .oi_change_1m_base
            .or(context.oi_change_5m_base)
            .is_some()
    {
        AltContractStructureConfidence::High
    } else if main_force_confidence >= 55.0 {
        AltContractStructureConfidence::Medium
    } else {
        AltContractStructureConfidence::Low
    };
    let mut evidence_degraded_reasons = Vec::new();
    if context
        .oi_change_1m_base
        .or(context.oi_change_5m_base)
        .is_none()
    {
        evidence_degraded_reasons.push("oi_missing".to_string());
    }
    if context.ticker_quote_volume_24h_usd.is_none() {
        evidence_degraded_reasons.push("ticker_missing".to_string());
    }
    if stats.dynamic_multiple.is_none() {
        evidence_degraded_reasons.push("dynamic_baseline_unavailable".to_string());
    }
    if stats.data_quality < 80 {
        evidence_degraded_reasons.push("data_quality_degraded".to_string());
    }
    AltSignalAssessment {
        anomaly_severity: severity,
        structure_confidence,
        exposure_tier: super::types::AltContractExposureTier::Observe,
        evidence_degraded_reasons,
    }
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
    s_grade_eligible: bool,
) -> AltContractSeverity {
    let thresholds = config.thresholds_for_tier(stats.tier);
    let dynamic = stats.dynamic_multiple.unwrap_or(0.0);
    if s_grade_eligible
        && (max_score >= 90
            || stats.total_notional_usd >= thresholds.s_notional_usd
            || dynamic >= config.dynamic.s_multiple)
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

#[derive(Debug, Clone)]
struct SGradeEvaluation {
    eligible: bool,
    notional_threshold_usd: f64,
    volume_threshold_base: f64,
    conditions: Vec<AltContractGradeCondition>,
}

fn s_grade_evaluation(
    stats: &AltContractWindowStats,
    context: &AltContractContext,
    score: &AltContractScoreResult,
    config: &BinanceAltContractRuntimeConfig,
    alt_impact_score: &AltContractImpactScore,
) -> SGradeEvaluation {
    let thresholds = config.thresholds_for_tier(stats.tier);
    let notional_threshold_usd = thresholds.s_notional_usd.max(5_000.0);
    let volume_threshold_base = stats
        .trigger_price_usd
        .filter(|price| *price > 0.0)
        .map(|price| notional_threshold_usd / price)
        .unwrap_or(f64::INFINITY);
    let dynamic = stats.dynamic_multiple.unwrap_or(0.0);
    let oi_change_pct = context.oi_change_pct.unwrap_or_default();
    let price_available = stats.trigger_price_usd.is_some_and(|price| price > 0.0);
    let mut conditions = vec![
        grade_condition(
            "notional_threshold",
            "成交额达到 S 门槛",
            stats.total_notional_usd >= notional_threshold_usd,
            format_usd_plain(stats.total_notional_usd),
            format_usd_plain(notional_threshold_usd),
        ),
        grade_condition(
            "volume_threshold",
            "成交量达到 Tier 门槛",
            price_available && stats.total_volume_base >= volume_threshold_base,
            format_base_plain(stats.total_volume_base, &stats.symbol),
            if volume_threshold_base.is_finite() {
                format_base_plain(volume_threshold_base, &stats.symbol)
            } else {
                "需要有效价格".to_string()
            },
        ),
        grade_condition(
            "directional_share",
            "单向占比 >= 60%",
            stats.dominance >= 0.60,
            format!("{:.1}%", stats.dominance * 100.0),
            ">= 60.0%".to_string(),
        ),
        grade_condition(
            "oi_expansion",
            "OI 增幅 > 1%",
            oi_change_pct > 1.0,
            format!("{oi_change_pct:.2}%"),
            "> 1.00%".to_string(),
        ),
        grade_condition(
            "dynamic_multiple",
            "异常倍数 >= 6x",
            dynamic >= 6.0,
            format!("{dynamic:.2}x"),
            ">= 6.00x".to_string(),
        ),
        grade_condition(
            "abnormal_score",
            "异常分 >= 40",
            score.abnormal_score >= 40,
            format!("{}/100", score.abnormal_score),
            ">= 40/100".to_string(),
        ),
        grade_condition(
            "alt_impact_score",
            "AIS 相对冲击 >= 90",
            impact_s_ready(alt_impact_score),
            format!("{:.1}/100", alt_impact_score.final_score),
            format!(">= {:.1}/100", alt_impact_score.s_threshold),
        ),
        grade_condition(
            "non_liquidation",
            "非清算主导",
            !context.liquidation_suspected,
            if context.liquidation_suspected {
                "疑似清算".to_string()
            } else {
                "正常".to_string()
            },
            "非清算".to_string(),
        ),
        grade_condition(
            "data_quality",
            "数据质量 >= 70",
            stats.data_quality >= 70,
            format!("{}/100", stats.data_quality),
            ">= 70/100".to_string(),
        ),
    ];

    let eligible = conditions.iter().all(|condition| condition.passed);
    if matches!(
        stats.tier,
        AltContractSymbolTier::C | AltContractSymbolTier::D | AltContractSymbolTier::E
    ) && stats.total_notional_usd < notional_threshold_usd
    {
        conditions.push(grade_condition(
            "low_liquidity_extra_guard",
            "低流动性 Tier 额外保护",
            false,
            format_usd_plain(stats.total_notional_usd),
            format!(
                "低流动性 S 门槛 {}",
                format_usd_plain(notional_threshold_usd)
            ),
        ));
        return SGradeEvaluation {
            eligible: false,
            notional_threshold_usd,
            volume_threshold_base,
            conditions,
        };
    }

    SGradeEvaluation {
        eligible,
        notional_threshold_usd,
        volume_threshold_base,
        conditions,
    }
}

fn grade_condition(
    key: &str,
    label: &str,
    passed: bool,
    actual: String,
    threshold: String,
) -> AltContractGradeCondition {
    AltContractGradeCondition {
        key: key.to_string(),
        label: label.to_string(),
        passed,
        actual,
        threshold,
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
    let relative_leader = !market_context.market_wide_move
        || market_context
            .relative_strength_rank
            .is_some_and(|rank| rank <= 10);
    if relative_leader {
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
        && market_context
            .relative_strength_rank
            .is_none_or(|rank| rank > 10)
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
        && (context.liquidation_count >= 2
            || context
                .liquidation_notional_usd
                .is_some_and(|notional| notional >= 20_000_000.0))
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
    if context.liquidation_suspected && signal_type != AltContractSignalType::LiquidationCascade {
        return "当前异动伴随清算上下文，优先按清算驱动的行情冲击解释，不直接确认主力建仓。"
            .to_string();
    }
    match signal_type {
        AltContractSignalType::MainForceLongBuild => {
            format!(
                "Binance 山寨永续主动买入、OI 上升与价格响应同向，当前更适合作为累积压力解释；结构置信度 {:.0}/100。",
                main_force_confidence
            )
        }
        AltContractSignalType::MainForceShortBuild => {
            format!(
                "Binance 山寨永续主动卖出、OI 上升与价格响应同向，当前更适合作为分发压力解释；结构置信度 {:.0}/100。",
                main_force_confidence
            )
        }
        AltContractSignalType::AbnormalPump => {
            if context.liquidation_suspected {
                return "山寨永续主动买入和成交额异常放大，伴随清算上下文，暂作为上行失衡观察，不直接确认主力建仓。"
                    .to_string();
            }
            if context
                .oi_change_1m_base
                .or(context.oi_change_5m_base)
                .is_none()
            {
                "山寨永续主动买入和成交额异常放大，但 OI 尚未确认，先标记为上行失衡观察。"
                    .to_string()
            } else {
                "山寨永续主动买入和成交额异常放大，但证据更适合作为上行失衡观察。".to_string()
            }
        }
        AltContractSignalType::AbnormalDump => {
            if context.liquidation_suspected {
                return "山寨永续主动卖出和成交额异常放大，伴随清算上下文，暂作为下行失衡观察，不直接确认主力建仓。"
                    .to_string();
            }
            if context
                .oi_change_1m_base
                .or(context.oi_change_5m_base)
                .is_none()
            {
                "山寨永续主动卖出和成交额异常放大，但 OI 尚未确认，先标记为下行失衡观察。"
                    .to_string()
            } else {
                "山寨永续主动卖出和成交额异常放大，但证据更适合作为下行失衡观察。".to_string()
            }
        }
        AltContractSignalType::DownsideAbsorption => {
            "主动卖出放大但价格跌不动，更适合作为下方吸收解释。".to_string()
        }
        AltContractSignalType::UpsideResistance => {
            "主动买入放大但价格涨不动，更适合作为上方压制解释。".to_string()
        }
        AltContractSignalType::LiquidationCascade => {
            "成交与强平快照同时放大，优先标记为清算事件，不直接确认主力建仓。".to_string()
        }
        AltContractSignalType::UnclearContractAnomaly => {
            "山寨永续成交异常但确认项不足，暂作为合约异动观察。".to_string()
        }
    }
}

fn round(value: f64, places: i32) -> f64 {
    let factor = 10_f64.powi(places);
    (value * factor).round() / factor
}

fn format_usd_plain(value: f64) -> String {
    if value.abs() >= 1_000_000.0 {
        format!("${:.1}M", value / 1_000_000.0)
    } else if value.abs() >= 1_000.0 {
        format!("${:.0}K", value / 1_000.0)
    } else {
        format!("${value:.0}")
    }
}

fn format_base_plain(value: f64, symbol: &str) -> String {
    if value.is_finite() {
        format!("{value:.2} {symbol}")
    } else {
        "N/A".to_string()
    }
}
