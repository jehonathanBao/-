use btc_toxic_flow_monitor_rs::binance_alt_contract_monitor::{
    mcss::score_master_capital_strength,
    types::{
        AltContractContext, AltContractDirection, AltContractExchangeContribution,
        AltContractMarketTier, AltContractSymbolTier, AltContractWindowStats,
    },
};

#[test]
fn btc_confirmed_large_flow_scores_above_eighty() {
    let stats = stats(
        "BTC",
        AltContractDirection::Buy,
        1_000_000.0,
        0.72,
        0.35,
        6.0,
        AltContractSymbolTier::A,
    );
    let context = confirmed_oi_context(1.4, None);
    let score = score_master_capital_strength(&stats, &context, AltContractMarketTier::UltraCore);

    assert!(score.mcss >= 80.0, "mcss={score:?}");
    assert_eq!(score.tier, "Ultra Core");
    assert_eq!(score.liquidity_weight, 0.6);
    assert_eq!(score.oi_score, 25.0);
    assert_eq!(score.price_score, 20.0);
}

#[test]
fn alt_two_hundred_k_with_oi_and_anomaly_scores_as_strong_money() {
    let stats = stats(
        "WIF",
        AltContractDirection::Buy,
        200_000.0,
        0.76,
        0.42,
        6.2,
        AltContractSymbolTier::C,
    );
    let context = confirmed_oi_context(1.2, None);
    let score = score_master_capital_strength(&stats, &context, AltContractMarketTier::Alt);

    assert!(score.mcss >= 75.0, "mcss={score:?}");
    assert_eq!(score.tier, "Alt");
    assert_eq!(score.liquidity_weight, 1.5);
    assert!(score.notional_score > 0.0);
    assert!(!score.interpretation.is_empty());
}

#[test]
fn wick_like_reverse_price_and_oi_drop_caps_mcss() {
    let confirmed = score_master_capital_strength(
        &stats(
            "XRP",
            AltContractDirection::Buy,
            2_000_000.0,
            0.82,
            0.55,
            7.0,
            AltContractSymbolTier::B,
        ),
        &confirmed_oi_context(1.1, None),
        AltContractMarketTier::Mainstream,
    );
    let wick = score_master_capital_strength(
        &stats(
            "XRP",
            AltContractDirection::Buy,
            2_000_000.0,
            0.82,
            -0.40,
            7.0,
            AltContractSymbolTier::B,
        ),
        &AltContractContext {
            oi_change_1m_base: Some(-50_000.0),
            oi_change_pct: Some(-1.0),
            ..AltContractContext::default()
        },
        AltContractMarketTier::Mainstream,
    );

    assert!(confirmed.mcss > wick.mcss);
    assert!(wick.mcss <= 69.0, "mcss={wick:?}");
    assert_eq!(wick.oi_score, -10.0);
    assert_eq!(wick.price_score, -15.0);
}

#[test]
fn liquidation_ratio_over_forty_percent_suppresses_mcss() {
    let stats = stats(
        "PEPE",
        AltContractDirection::Sell,
        1_000_000.0,
        0.88,
        -0.80,
        8.0,
        AltContractSymbolTier::D,
    );
    let context = confirmed_oi_context(
        1.8,
        Some(AltContractContext {
            liquidation_notional_usd: Some(500_000.0),
            liquidation_suspected: true,
            force_order_snapshot: true,
            ..AltContractContext::default()
        }),
    );
    let score = score_master_capital_strength(&stats, &context, AltContractMarketTier::Alt);

    assert_eq!(score.tier, "Micro Alt");
    assert_eq!(score.liquidity_weight, 1.8);
    assert_eq!(score.liquidation_penalty, 25.0);
    assert!(score.mcss <= 69.0, "mcss={score:?}");
}

fn confirmed_oi_context(
    oi_change_pct: f64,
    overrides: Option<AltContractContext>,
) -> AltContractContext {
    let mut context = overrides.unwrap_or_default();
    context.oi_change_1m_base = Some(100_000.0);
    context.oi_change_pct = Some(oi_change_pct);
    context
}

fn stats(
    symbol: &str,
    direction: AltContractDirection,
    notional: f64,
    dominance: f64,
    price_move_pct: f64,
    dynamic_multiple: f64,
    tier: AltContractSymbolTier,
) -> AltContractWindowStats {
    let signed_net = if direction == AltContractDirection::Buy {
        10_000.0 * dominance
    } else {
        -10_000.0 * dominance
    };
    AltContractWindowStats {
        symbol: symbol.to_string(),
        product_id: format!("{symbol}USDT"),
        tier,
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
