use btc_toxic_flow_monitor_rs::binance_alt_contract_monitor::{
    scc::calibrate_signal_confidence,
    types::{
        AltContractConfidenceLevel, AltContractContext, AltContractDirection,
        AltContractExchangeContribution, AltContractLiquidityMicrostructure,
        AltContractMarketControlGraph, AltContractMasterCapitalStrength, AltContractSeverity,
        AltContractSignalType, AltContractSmartMoneyLifecycle, AltContractSmartMoneyPrediction,
        AltContractSymbolTier, AltContractWindowStats,
    },
};

#[test]
fn strong_main_force_build_scores_high_confidence() {
    let confidence = calibrate_signal_confidence(
        &stats(AltContractDirection::Buy, 0.72, 0.34, 92),
        &context(1.4, None, false),
        AltContractSignalType::MainForceLongBuild,
        88,
        86,
        AltContractSeverity::Critical,
        &mcss(88.0),
        &lifecycle(84.0, 86.0, 82.0, "Markup"),
        &prediction("Bullish", 84.0, 82.0, 80.0),
        &microstructure(90.0, "buyer_side_control", "none"),
        &graph(84.0, "buy", "ControlAccumulation"),
        false,
    );

    assert!(confidence.confidence_score >= 85.0, "{confidence:?}");
    assert!(matches!(
        confidence.confidence_level,
        AltContractConfidenceLevel::High | AltContractConfidenceLevel::VeryHigh
    ));
    assert!(confidence
        .reliability_factors
        .contains(&"mcss_strong_money".to_string()));
    assert!(confidence.read_only);
    assert!(!confidence.direct_discord_gate);
}

#[test]
fn fake_breakout_is_downgraded_to_weak_confidence() {
    let confidence = calibrate_signal_confidence(
        &stats(AltContractDirection::Buy, 0.74, 0.78, 78),
        &context(-0.4, None, false),
        AltContractSignalType::AbnormalPump,
        88,
        70,
        AltContractSeverity::Critical,
        &mcss(55.0),
        &lifecycle(45.0, 42.0, 46.0, "Markup"),
        &prediction("BearishRisk", 45.0, 38.0, 40.0),
        &microstructure(48.0, "fake_liquidity_control", "detected"),
        &graph(45.0, "buy", "ControlManipulation"),
        false,
    );

    assert!(
        (40.0..=60.0).contains(&confidence.confidence_score),
        "{confidence:?}"
    );
    assert!(confidence
        .risk_factors
        .contains(&"spoofing_or_fake_liquidity".to_string()));
    assert!(confidence
        .risk_factors
        .contains(&"prediction_misaligned".to_string()));
}

#[test]
fn liquidation_impact_lowers_confidence() {
    let confidence = calibrate_signal_confidence(
        &stats(AltContractDirection::Sell, 0.8, -0.9, 74),
        &context(-1.8, Some(500_000.0), true),
        AltContractSignalType::LiquidationCascade,
        92,
        42,
        AltContractSeverity::Critical,
        &mcss(58.0),
        &lifecycle(48.0, 44.0, 46.0, "Markdown"),
        &prediction("Bearish", 55.0, 54.0, 50.0),
        &microstructure(52.0, "seller_side_control", "none"),
        &graph(50.0, "sell", "ControlManipulation"),
        true,
    );

    assert!(confidence.confidence_score < 60.0, "{confidence:?}");
    assert!(confidence
        .risk_factors
        .contains(&"liquidation_interference".to_string()));
    assert!(confidence
        .risk_factors
        .contains(&"market_wide_noise".to_string()));
}

#[test]
fn all_layers_aligned_scores_very_high_confidence() {
    let confidence = calibrate_signal_confidence(
        &stats(AltContractDirection::Buy, 0.84, 0.62, 96),
        &context(2.2, None, false),
        AltContractSignalType::MainForceLongBuild,
        96,
        94,
        AltContractSeverity::S,
        &mcss(96.0),
        &lifecycle(93.0, 94.0, 92.0, "Markup"),
        &prediction("Bullish", 94.0, 93.0, 92.0),
        &microstructure(94.0, "buyer_side_control", "none"),
        &graph(95.0, "buy", "ControlAccumulation"),
        false,
    );

    assert!(confidence.confidence_score >= 90.0, "{confidence:?}");
    assert_eq!(
        confidence.confidence_level,
        AltContractConfidenceLevel::VeryHigh
    );
    assert!(confidence
        .reliability_factors
        .contains(&"mcg_control_coherent".to_string()));
}

fn stats(
    direction: AltContractDirection,
    dominance: f64,
    price_move_pct: f64,
    data_quality: u8,
) -> AltContractWindowStats {
    let net_volume_base = match direction {
        AltContractDirection::Buy => 720.0,
        AltContractDirection::Sell => -720.0,
        _ => 0.0,
    };
    AltContractWindowStats {
        symbol: "SOL".to_string(),
        product_id: "SOLUSDT".to_string(),
        tier: AltContractSymbolTier::B,
        window_sec: 60,
        ts: 1_700_000_000_000,
        buy_volume_base: 860.0,
        sell_volume_base: 140.0,
        total_volume_base: 1_000.0,
        net_volume_base,
        total_notional_usd: 1_000_000.0,
        dominance,
        direction,
        trigger_price_usd: Some(100.0),
        price_move_pct: Some(price_move_pct),
        price_threshold_pct: None,
        exchange_count: 1,
        main_exchange: Some("binance".to_string()),
        exchanges: vec![AltContractExchangeContribution {
            exchange: "binance".to_string(),
            total_volume_base: 1_000.0,
            total_notional_usd: 1_000_000.0,
            net_volume_base,
            dominance,
            trade_count: 100,
            ..AltContractExchangeContribution::default()
        }],
        dynamic_multiple: Some(7.2),
        data_quality,
        startup_age_ms: Some(120_000),
    }
}

fn context(
    oi_change_pct: f64,
    liquidation_notional_usd: Option<f64>,
    liquidation_suspected: bool,
) -> AltContractContext {
    AltContractContext {
        oi_change_pct: Some(oi_change_pct),
        liquidation_notional_usd,
        liquidation_suspected,
        force_order_snapshot: liquidation_suspected,
        ..AltContractContext::default()
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

fn lifecycle(
    state_confidence: f64,
    flow_consistency_score: f64,
    lifecycle_score: f64,
    state: &str,
) -> AltContractSmartMoneyLifecycle {
    AltContractSmartMoneyLifecycle {
        lifecycle_state: state.to_string(),
        state_confidence,
        flow_consistency_score,
        lifecycle_score,
        ..AltContractSmartMoneyLifecycle::default()
    }
}

fn prediction(
    direction_bias: &str,
    confidence: f64,
    probability: f64,
    prediction_score: f64,
) -> AltContractSmartMoneyPrediction {
    AltContractSmartMoneyPrediction {
        direction_bias: direction_bias.to_string(),
        confidence,
        probability,
        prediction_score,
        ..AltContractSmartMoneyPrediction::default()
    }
}

fn microstructure(
    lms_score: f64,
    market_control: &str,
    spoofing_state: &str,
) -> AltContractLiquidityMicrostructure {
    AltContractLiquidityMicrostructure {
        lms_score,
        market_control: market_control.to_string(),
        spoofing_state: spoofing_state.to_string(),
        ..AltContractLiquidityMicrostructure::default()
    }
}

fn graph(
    control_strength: f64,
    dominant_side: &str,
    control_type: &str,
) -> AltContractMarketControlGraph {
    AltContractMarketControlGraph {
        control_strength,
        dominant_side: dominant_side.to_string(),
        control_type: control_type.to_string(),
        ..AltContractMarketControlGraph::default()
    }
}
