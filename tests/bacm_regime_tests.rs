use btc_toxic_flow_monitor_rs::binance_alt_contract_monitor::{
    detector::MarketImpulseContext,
    regime::classify_market_regime,
    types::{
        AltContractContext, AltContractDirection, AltContractExchangeContribution,
        AltContractMasterCapitalStrength, AltContractSymbolTier, AltContractWindowConfirmation,
        AltContractWindowStats,
    },
};

#[test]
fn accumulation_requires_oi_up_flat_price_and_multi_window_confirmation() {
    let stats = stats("WIF", AltContractDirection::Buy, 800_000.0, 0.58, 0.03, 3.0);
    let context = AltContractContext {
        oi_change_1m_base: Some(100_000.0),
        oi_change_pct: Some(1.2),
        persistence_windows: 2,
        ..AltContractContext::default()
    };
    let regime = classify_market_regime(
        &stats,
        &context,
        &mcss(70.0),
        &confirmed_windows(),
        &MarketImpulseContext::default(),
    );

    assert_eq!(regime.regime, "Accumulation");
    assert!(regime.confidence >= 65.0, "{regime:?}");
    assert_eq!(regime.oi_trend, "up");
    assert_eq!(regime.price_trend, "flat");
    assert!(regime
        .explanation_tags
        .iter()
        .any(|tag| tag == "smart_money_accumulating"));
}

#[test]
fn distribution_detects_price_up_with_oi_down_and_low_efficiency() {
    let stats = stats(
        "XRP",
        AltContractDirection::Sell,
        2_000_000.0,
        0.66,
        0.30,
        3.0,
    );
    let context = AltContractContext {
        oi_change_1m_base: Some(-70_000.0),
        oi_change_pct: Some(-0.8),
        ..AltContractContext::default()
    };
    let regime = classify_market_regime(
        &stats,
        &context,
        &mcss(72.0),
        &[],
        &MarketImpulseContext::default(),
    );

    assert_eq!(regime.regime, "Distribution");
    assert_eq!(regime.oi_trend, "down");
    assert_eq!(regime.price_trend, "slow_up");
    assert!(regime
        .explanation_tags
        .iter()
        .any(|tag| tag == "distribution_pressure"));
}

#[test]
fn fake_breakout_with_dynamic_spike_and_oi_mismatch_is_manipulation() {
    let stats = stats(
        "ALT",
        AltContractDirection::Buy,
        1_500_000.0,
        0.82,
        -0.22,
        7.2,
    );
    let context = AltContractContext {
        oi_change_1m_base: Some(-10_000.0),
        oi_change_pct: Some(-0.2),
        ..AltContractContext::default()
    };
    let regime = classify_market_regime(
        &stats,
        &context,
        &mcss(80.0),
        &[],
        &MarketImpulseContext::default(),
    );

    assert_eq!(regime.regime, "Manipulation");
    assert_eq!(regime.sub_type.as_deref(), Some("Liquidity_Trap"));
    assert!(regime.confidence >= 80.0, "{regime:?}");
    assert!(regime
        .explanation_tags
        .iter()
        .any(|tag| tag == "liquidity_trap"));
}

#[test]
fn liquidation_zone_with_oi_down_is_manipulation_down() {
    let stats = stats(
        "PEPE",
        AltContractDirection::Sell,
        1_000_000.0,
        0.88,
        -0.90,
        8.0,
    );
    let context = AltContractContext {
        oi_change_1m_base: Some(-120_000.0),
        oi_change_pct: Some(-1.7),
        liquidation_notional_usd: Some(450_000.0),
        liquidation_suspected: true,
        force_order_snapshot: true,
        ..AltContractContext::default()
    };
    let regime = classify_market_regime(
        &stats,
        &context,
        &mcss(82.0),
        &[],
        &MarketImpulseContext::default(),
    );

    assert_eq!(regime.regime, "Manipulation");
    assert_eq!(regime.sub_type.as_deref(), Some("Manipulation_DOWN"));
    assert!(regime.explanation_tags.iter().any(|tag| tag == "stop_hunt"));
}

fn confirmed_windows() -> Vec<AltContractWindowConfirmation> {
    vec![
        AltContractWindowConfirmation {
            window_sec: 15,
            notional_usd: 500_000.0,
            dynamic_multiple: Some(3.0),
            directional_strength: 0.58,
            confirmed: true,
        },
        AltContractWindowConfirmation {
            window_sec: 60,
            notional_usd: 800_000.0,
            dynamic_multiple: Some(3.0),
            directional_strength: 0.58,
            confirmed: true,
        },
    ]
}

fn mcss(value: f64) -> AltContractMasterCapitalStrength {
    AltContractMasterCapitalStrength {
        mcss: value,
        tier: "Alt".to_string(),
        interpretation: "test".to_string(),
        ..AltContractMasterCapitalStrength::default()
    }
}

fn stats(
    symbol: &str,
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
        symbol: symbol.to_string(),
        product_id: format!("{symbol}USDT"),
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
