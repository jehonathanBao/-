use btc_toxic_flow_monitor_rs::binance_alt_contract_monitor::{
    smle::classify_smart_money_lifecycle,
    types::{
        AltContractContext, AltContractDirection, AltContractExchangeContribution,
        AltContractMarketRegime, AltContractMasterCapitalStrength, AltContractSmartMoneyLifecycle,
        AltContractSymbolTier, AltContractWindowConfirmation, AltContractWindowStats,
    },
};

#[test]
fn accumulation_transitions_to_markup_on_oi_breakout_and_high_mcss() {
    let previous = lifecycle("Accumulation", 21.0, vec!["Accumulation"]);
    let stats = stats(AltContractDirection::Buy, 1_500_000.0, 0.66, 0.82, 5.0, 60);
    let context = context(Some(2.1), Some(120_000.0), None, 3);
    let lifecycle = classify_smart_money_lifecycle(
        &stats,
        &context,
        &mcss(82.0),
        &regime("Accumulation", "up", "spike_up", 0.24),
        &confirmed_windows(),
        Some(&previous),
    );

    assert_eq!(lifecycle.lifecycle_state, "Markup");
    assert_eq!(
        lifecycle.transition_signal.as_deref(),
        Some("Accumulation->Markup")
    );
    assert!(lifecycle.state_confidence >= 65.0, "{lifecycle:?}");
    assert!(lifecycle.state_path.ends_with(&["Markup".to_string()]));
}

#[test]
fn markup_transitions_to_distribution_when_oi_flattens_and_efficiency_fades() {
    let previous = lifecycle("Markup", 42.0, vec!["Accumulation", "Markup"]);
    let stats = stats(AltContractDirection::Sell, 2_000_000.0, 0.62, 0.26, 3.0, 60);
    let context = context(Some(0.0), Some(0.0), None, 2);
    let lifecycle = classify_smart_money_lifecycle(
        &stats,
        &context,
        &mcss(72.0),
        &regime("Distribution", "flat", "slow_up", 0.13),
        &confirmed_windows(),
        Some(&previous),
    );

    assert_eq!(lifecycle.lifecycle_state, "Distribution");
    assert_eq!(
        lifecycle.transition_signal.as_deref(),
        Some("Markup->Distribution")
    );
    assert!(lifecycle
        .explanation_tags
        .iter()
        .any(|tag| tag == "low_price_efficiency"));
}

#[test]
fn distribution_transitions_to_markdown_on_price_breakdown_oi_down_and_liquidation() {
    let previous = lifecycle(
        "Distribution",
        18.0,
        vec!["Accumulation", "Markup", "Distribution"],
    );
    let stats = stats(
        AltContractDirection::Sell,
        1_200_000.0,
        0.72,
        -0.90,
        7.0,
        60,
    );
    let context = context(Some(-1.8), Some(-150_000.0), Some(360_000.0), 1);
    let lifecycle = classify_smart_money_lifecycle(
        &stats,
        &context,
        &mcss(78.0),
        &regime("Manipulation", "down", "spike_down", 0.75),
        &confirmed_windows(),
        Some(&previous),
    );

    assert_eq!(lifecycle.lifecycle_state, "Markdown");
    assert_eq!(
        lifecycle.transition_signal.as_deref(),
        Some("Distribution->Markdown")
    );
    assert!(lifecycle
        .explanation_tags
        .iter()
        .any(|tag| tag == "liquidation_disturbance"));
}

#[test]
fn markdown_transitions_to_re_accumulation_when_volatility_falls_and_oi_stabilizes() {
    let previous = lifecycle("Markdown", 35.0, vec!["Distribution", "Markdown"]);
    let stats = stats(AltContractDirection::Buy, 500_000.0, 0.52, 0.01, 2.0, 60);
    let context = context(Some(0.0), Some(0.0), None, 2);
    let lifecycle = classify_smart_money_lifecycle(
        &stats,
        &context,
        &mcss(62.0),
        &regime("Unclear", "flat", "flat", 0.02),
        &confirmed_windows(),
        Some(&previous),
    );

    assert_eq!(lifecycle.lifecycle_state, "ReAccumulation");
    assert_eq!(
        lifecycle.transition_signal.as_deref(),
        Some("Markdown->ReAccumulation")
    );
}

#[test]
fn manipulation_is_inserted_without_breaking_previous_lifecycle_state() {
    let previous = lifecycle("Markup", 12.0, vec!["Accumulation", "Markup"]);
    let stats = stats(AltContractDirection::Buy, 1_000_000.0, 0.84, -0.28, 8.0, 15);
    let context = context(Some(-0.4), Some(-30_000.0), None, 1);
    let lifecycle = classify_smart_money_lifecycle(
        &stats,
        &context,
        &mcss(80.0),
        &regime("Manipulation", "down", "spike_up", 0.40),
        &confirmed_windows(),
        Some(&previous),
    );

    assert_eq!(lifecycle.lifecycle_state, "Markup");
    assert_eq!(lifecycle.transition_signal, None);
    assert!(lifecycle
        .explanation_tags
        .iter()
        .any(|tag| tag == "manipulation_disturbance"));
    assert!(lifecycle.current_explanation.contains("插入事件"));
}

fn lifecycle(state: &str, duration_min: f64, path: Vec<&str>) -> AltContractSmartMoneyLifecycle {
    AltContractSmartMoneyLifecycle {
        lifecycle_state: state.to_string(),
        state_duration_min: duration_min,
        state_path: path.into_iter().map(str::to_string).collect(),
        ..AltContractSmartMoneyLifecycle::default()
    }
}

fn mcss(value: f64) -> AltContractMasterCapitalStrength {
    AltContractMasterCapitalStrength {
        mcss: value,
        tier: "Alt".to_string(),
        interpretation: "test".to_string(),
        ..AltContractMasterCapitalStrength::default()
    }
}

fn regime(
    regime: &str,
    oi_trend: &str,
    price_trend: &str,
    efficiency_ratio: f64,
) -> AltContractMarketRegime {
    AltContractMarketRegime {
        regime: regime.to_string(),
        oi_trend: oi_trend.to_string(),
        price_trend: price_trend.to_string(),
        efficiency_ratio,
        ..AltContractMarketRegime::default()
    }
}

fn context(
    oi_change_pct: Option<f64>,
    oi_change_base: Option<f64>,
    liquidation_notional_usd: Option<f64>,
    persistence_windows: u8,
) -> AltContractContext {
    AltContractContext {
        oi_change_pct,
        oi_change_1m_base: oi_change_base,
        liquidation_notional_usd,
        liquidation_suspected: liquidation_notional_usd.is_some(),
        force_order_snapshot: liquidation_notional_usd.is_some(),
        persistence_windows,
        ..AltContractContext::default()
    }
}

fn confirmed_windows() -> Vec<AltContractWindowConfirmation> {
    vec![
        AltContractWindowConfirmation {
            window_sec: 15,
            notional_usd: 500_000.0,
            dynamic_multiple: Some(3.0),
            directional_strength: 0.62,
            confirmed: true,
        },
        AltContractWindowConfirmation {
            window_sec: 60,
            notional_usd: 900_000.0,
            dynamic_multiple: Some(4.0),
            directional_strength: 0.64,
            confirmed: true,
        },
    ]
}

fn stats(
    direction: AltContractDirection,
    notional: f64,
    dominance: f64,
    price_move_pct: f64,
    dynamic_multiple: f64,
    window_sec: u64,
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
        window_sec,
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
