use btc_toxic_flow_monitor_rs::binance_alt_contract_monitor::{
    mcg::build_market_control_graph,
    types::{
        AltContractContext, AltContractDirection, AltContractLiquidityMicrostructure,
        AltContractMarketRegime, AltContractMasterCapitalStrength, AltContractSmartMoneyLifecycle,
        AltContractSymbolTier, AltContractWindowStats,
    },
};

#[test]
fn mcg_detects_accumulation_control() {
    let stats = stats(AltContractDirection::Buy, 0.62, 0.03);
    let context = context(1.4);
    let microstructure = microstructure(
        "Absorption_Buy",
        "two_sided_absorption",
        82.0,
        48.0,
        58.0,
        20.0,
        0.0,
    );
    let graph = build_market_control_graph(
        &stats,
        &context,
        &microstructure,
        &mcss(74.0),
        &regime("Accumulation"),
        &lifecycle("Accumulation"),
    );

    assert_eq!(graph.control_type, "ControlAccumulation");
    assert_eq!(graph.dominant_side, "neutral");
    assert!(graph.control_strength >= 55.0);
    assert!(graph
        .control_edges
        .iter()
        .any(|edge| edge.relation == "absorption_relation"));
    assert!(graph.read_only);
    assert!(!graph.direct_discord_gate);
}

#[test]
fn mcg_detects_distribution_control() {
    let stats = stats(AltContractDirection::Buy, 0.55, 0.09);
    let context = context(0.1);
    let microstructure = microstructure(
        "Absorption_Sell",
        "seller_side_control",
        78.0,
        42.0,
        52.0,
        22.0,
        0.0,
    );
    let graph = build_market_control_graph(
        &stats,
        &context,
        &microstructure,
        &mcss(69.0),
        &regime("Distribution"),
        &lifecycle("Distribution"),
    );

    assert_eq!(graph.control_type, "ControlDistribution");
    assert_eq!(graph.dominant_side, "sell");
    assert!(graph
        .control_edges
        .iter()
        .any(|edge| edge.relation == "pressure_flow"));
}

#[test]
fn mcg_detects_manipulation_control() {
    let stats = stats(AltContractDirection::Buy, 0.81, 0.64);
    let context = context(-0.3);
    let microstructure = microstructure(
        "LiquiditySweepUp",
        "fake_liquidity_control",
        30.0,
        88.0,
        76.0,
        75.0,
        70.0,
    );
    let graph = build_market_control_graph(
        &stats,
        &context,
        &microstructure,
        &mcss(91.0),
        &regime("Manipulation"),
        &lifecycle("Markup"),
    );

    assert_eq!(graph.control_type, "ControlManipulation");
    assert_eq!(graph.dominant_side, "buy");
    assert!(graph.control_strength >= 60.0);
    assert!(graph
        .control_edges
        .iter()
        .any(|edge| edge.relation == "manipulation_relation"));
}

#[test]
fn mcg_keeps_noise_as_no_clear_control() {
    let stats = stats(AltContractDirection::Neutral, 0.18, 0.01);
    let context = context(0.0);
    let microstructure = microstructure(
        "OrdinaryFlow",
        "no_clear_control",
        5.0,
        12.0,
        10.0,
        6.0,
        0.0,
    );
    let graph = build_market_control_graph(
        &stats,
        &context,
        &microstructure,
        &mcss(22.0),
        &regime("Unclear"),
        &lifecycle("Unknown"),
    );

    assert_eq!(graph.control_type, "NoClearControl");
    assert!(graph.control_strength < 40.0);
}

fn stats(
    direction: AltContractDirection,
    dominance: f64,
    price_move_pct: f64,
) -> AltContractWindowStats {
    let net_volume_base = match direction {
        AltContractDirection::Buy => 620.0,
        AltContractDirection::Sell => -620.0,
        _ => 0.0,
    };
    AltContractWindowStats {
        symbol: "SOL".to_string(),
        product_id: "SOLUSDT".to_string(),
        tier: AltContractSymbolTier::B,
        window_sec: 60,
        ts: 1_700_000_000_000,
        buy_volume_base: 810.0,
        sell_volume_base: 190.0,
        total_volume_base: 1_000.0,
        net_volume_base,
        total_notional_usd: 175_000.0,
        dominance,
        direction,
        trigger_price_usd: Some(175.0),
        price_move_pct: Some(price_move_pct),
        exchange_count: 1,
        main_exchange: Some("binance".to_string()),
        exchanges: Vec::new(),
        dynamic_multiple: Some(6.5),
        data_quality: 85,
        startup_age_ms: Some(120_000),
    }
}

fn context(oi_change_pct: f64) -> AltContractContext {
    AltContractContext {
        oi_change_pct: Some(oi_change_pct),
        last_price_usd: Some(175.0),
        mark_price_usd: Some(175.0),
        ..AltContractContext::default()
    }
}

fn microstructure(
    behavior: &str,
    market_control: &str,
    absorption_strength: f64,
    order_flow_pressure: f64,
    imbalance_score: f64,
    spread_behavior: f64,
    spoofing_penalty: f64,
) -> AltContractLiquidityMicrostructure {
    AltContractLiquidityMicrostructure {
        lms_score: absorption_strength
            .max(order_flow_pressure)
            .max(imbalance_score)
            .max(spoofing_penalty),
        behavior: behavior.to_string(),
        market_control: market_control.to_string(),
        liquidity_pressure: "HIGH".to_string(),
        imbalance: 0.35,
        spread_state: "stable".to_string(),
        spoofing_state: if spoofing_penalty > 0.0 {
            "detected".to_string()
        } else {
            "none".to_string()
        },
        order_flow_pressure,
        absorption_strength,
        imbalance_score,
        spread_behavior,
        spoofing_penalty,
        explanation_tags: vec!["read_only_microstructure".to_string(), behavior.to_string()],
        interpretation: "test microstructure".to_string(),
        read_only: true,
        direct_discord_gate: false,
    }
}

fn mcss(score: f64) -> AltContractMasterCapitalStrength {
    AltContractMasterCapitalStrength {
        mcss: score,
        tier: "Alt".to_string(),
        interpretation: "test mcss".to_string(),
        ..AltContractMasterCapitalStrength::default()
    }
}

fn regime(name: &str) -> AltContractMarketRegime {
    AltContractMarketRegime {
        regime: name.to_string(),
        confidence: 75.0,
        ..AltContractMarketRegime::default()
    }
}

fn lifecycle(state: &str) -> AltContractSmartMoneyLifecycle {
    AltContractSmartMoneyLifecycle {
        lifecycle_state: state.to_string(),
        state_confidence: 75.0,
        ..AltContractSmartMoneyLifecycle::default()
    }
}
