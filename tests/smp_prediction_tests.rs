use btc_toxic_flow_monitor_rs::binance_alt_contract_monitor::{
    smp::predict_smart_money_next_stage,
    types::{
        AltContractContext, AltContractDirection, AltContractExchangeContribution,
        AltContractMarketRegime, AltContractMasterCapitalStrength, AltContractSmartMoneyLifecycle,
        AltContractSymbolTier, AltContractWindowStats,
    },
};

#[test]
fn accumulation_predicts_markup_when_oi_mcss_and_price_are_stable() {
    let prediction = predict_smart_money_next_stage(
        &stats(AltContractDirection::Buy, 700_000.0, 0.58, 0.03, 3.0),
        &context(Some(1.2), Some(100_000.0), None, 0.0001),
        &mcss(76.0, 70.0),
        &lifecycle("Accumulation", 78.0),
        &regime("Accumulation", "up", "flat", 0.08, 70.0),
    );

    assert_eq!(prediction.current_state, "Accumulation");
    assert_eq!(prediction.next_state, "Markup");
    assert!(prediction.probability >= 70.0, "{prediction:?}");
    assert_eq!(prediction.direction_bias, "Bullish");
    assert!(prediction
        .trigger_factors
        .iter()
        .any(|factor| factor == "oi_mcss_expansion"));
}

#[test]
fn markup_predicts_distribution_when_price_rises_but_oi_and_efficiency_fade() {
    let prediction = predict_smart_money_next_stage(
        &stats(AltContractDirection::Buy, 2_000_000.0, 0.64, 0.85, 4.0),
        &context(Some(-0.3), Some(-50_000.0), None, 0.0008),
        &mcss(78.0, 72.0),
        &lifecycle("Markup", 80.0),
        &regime("Distribution", "down", "spike_up", 0.12, 72.0),
    );

    assert_eq!(prediction.current_state, "Markup");
    assert_eq!(prediction.next_state, "Distribution");
    assert!(prediction.probability >= 65.0, "{prediction:?}");
    assert!(prediction
        .trigger_factors
        .iter()
        .any(|factor| factor == "oi_momentum_divergence"));
    assert!(prediction
        .trigger_factors
        .iter()
        .any(|factor| factor == "efficiency_decay"));
}

#[test]
fn distribution_predicts_markdown_when_oi_falls_and_price_followthrough_is_weak() {
    let prediction = predict_smart_money_next_stage(
        &stats(AltContractDirection::Sell, 1_200_000.0, 0.62, 0.02, 3.2),
        &context(Some(-0.8), Some(-120_000.0), None, 0.0002),
        &mcss(70.0, 68.0),
        &lifecycle("Distribution", 74.0),
        &regime("Distribution", "down", "flat", 0.10, 68.0),
    );

    assert_eq!(prediction.next_state, "Markdown");
    assert!(prediction.probability >= 70.0, "{prediction:?}");
    assert_eq!(prediction.direction_bias, "Bearish");
}

#[test]
fn markdown_predicts_re_accumulation_after_liquidation_flush() {
    let prediction = predict_smart_money_next_stage(
        &stats(AltContractDirection::Sell, 1_000_000.0, 0.88, -1.2, 7.0),
        &context(Some(-2.0), Some(-200_000.0), Some(450_000.0), 0.0001),
        &mcss(66.0, 66.0),
        &lifecycle("Markdown", 62.0),
        &regime("Manipulation", "down", "spike_down", 0.25, 66.0),
    );

    assert_eq!(prediction.next_state, "ReAccumulation");
    assert!(prediction.probability >= 70.0, "{prediction:?}");
    assert_eq!(prediction.direction_bias, "ReboundWatch");
    assert!(prediction
        .trigger_factors
        .iter()
        .any(|factor| factor == "liquidity_stress"));
}

#[test]
fn manipulation_noise_lowers_confidence_but_does_not_change_lifecycle_input() {
    let noisy = predict_smart_money_next_stage(
        &stats(AltContractDirection::Buy, 900_000.0, 0.84, -0.30, 8.0),
        &context(Some(-0.2), Some(-20_000.0), None, 0.0001),
        &mcss(80.0, 74.0),
        &lifecycle_with_tags("Markup", 78.0, vec!["manipulation_disturbance"]),
        &regime("Manipulation", "down", "spike_up", 0.42, 74.0),
    );

    assert_eq!(noisy.current_state, "Markup");
    assert_eq!(noisy.next_state, "Distribution");
    assert!(noisy
        .trigger_factors
        .iter()
        .any(|factor| factor == "manipulation_noise_filtered"));
    assert!(noisy.explanation.contains("不把插针直接当作趋势"));
}

fn mcss(value: f64, previous_regime_score: f64) -> AltContractMasterCapitalStrength {
    AltContractMasterCapitalStrength {
        mcss: value,
        tier: "Alt".to_string(),
        interpretation: "test".to_string(),
        notional_score: previous_regime_score,
        ..AltContractMasterCapitalStrength::default()
    }
}

fn lifecycle(state: &str, confidence: f64) -> AltContractSmartMoneyLifecycle {
    lifecycle_with_tags(state, confidence, vec!["flow_consistent"])
}

fn lifecycle_with_tags(
    state: &str,
    confidence: f64,
    tags: Vec<&str>,
) -> AltContractSmartMoneyLifecycle {
    AltContractSmartMoneyLifecycle {
        lifecycle_state: state.to_string(),
        state_confidence: confidence,
        flow_consistency_score: 82.0,
        explanation_tags: tags.into_iter().map(str::to_string).collect(),
        ..AltContractSmartMoneyLifecycle::default()
    }
}

fn regime(
    regime: &str,
    oi_trend: &str,
    price_trend: &str,
    efficiency_ratio: f64,
    mc_score: f64,
) -> AltContractMarketRegime {
    AltContractMarketRegime {
        regime: regime.to_string(),
        oi_trend: oi_trend.to_string(),
        price_trend: price_trend.to_string(),
        efficiency_ratio,
        mc_score,
        ..AltContractMarketRegime::default()
    }
}

fn context(
    oi_change_pct: Option<f64>,
    oi_change_base: Option<f64>,
    liquidation_notional_usd: Option<f64>,
    funding_rate: f64,
) -> AltContractContext {
    AltContractContext {
        oi_change_pct,
        oi_change_1m_base: oi_change_base,
        liquidation_notional_usd,
        liquidation_suspected: liquidation_notional_usd.is_some(),
        force_order_snapshot: liquidation_notional_usd.is_some(),
        funding_rate: Some(funding_rate),
        ..AltContractContext::default()
    }
}

fn stats(
    direction: AltContractDirection,
    notional: f64,
    dominance: f64,
    price_move_pct: f64,
    dynamic_multiple: f64,
) -> AltContractWindowStats {
    let signed_net = if direction == AltContractDirection::Buy {
        10_000.0 * dominance
    } else {
        -10_000.0 * dominance
    };
    AltContractWindowStats {
        symbol: "WIF".to_string(),
        product_id: "WIFUSDT".to_string(),
        tier: AltContractSymbolTier::C,
        window_sec: 60,
        ts: 1_700_000_000_000,
        buy_volume_base: if signed_net > 0.0 { 9_000.0 } else { 1_000.0 },
        sell_volume_base: if signed_net < 0.0 { 9_000.0 } else { 1_000.0 },
        total_volume_base: 10_000.0,
        net_volume_base: signed_net,
        total_notional_usd: notional,
        dominance,
        direction,
        trigger_price_usd: Some(notional / 10_000.0),
        price_move_pct: Some(price_move_pct),
        price_threshold_pct: None,
        exchange_count: 1,
        main_exchange: Some("binance".to_string()),
        exchanges: vec![AltContractExchangeContribution {
            exchange: "binance".to_string(),
            total_volume_base: 10_000.0,
            total_notional_usd: notional,
            net_volume_base: signed_net,
            dominance,
            trade_count: 100,
            ..AltContractExchangeContribution::default()
        }],
        dynamic_multiple: Some(dynamic_multiple),
        data_quality: 92,
        startup_age_ms: Some(120_000),
    }
}
