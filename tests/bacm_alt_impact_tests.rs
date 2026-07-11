use btc_toxic_flow_monitor_rs::binance_alt_contract_monitor::{
    impact::{impact_discord_ready, impact_displayable, impact_s_ready, score_alt_impact},
    types::{
        AltContractContext, AltContractDirection, AltContractExchangeContribution,
        AltContractMarketTier, AltContractSymbolTier, AltContractWindowStats,
    },
};

#[test]
fn low_notional_without_relative_impact_stays_hidden() {
    let stats = stats("TINY", 50_000.0, 0.52, 1.2);
    let context = AltContractContext {
        ticker_quote_volume_24h_usd: Some(1_000_000_000.0),
        ticker_updated_at: Some(stats.ts - 10_000),
        oi_change_pct: Some(0.1),
        ..AltContractContext::default()
    };

    let impact = score_alt_impact(&stats, &context, AltContractMarketTier::Alt);

    assert!(impact.final_score < impact.display_threshold);
    assert!(!impact_displayable(&impact));
    assert!(!impact_discord_ready(&impact));
}

#[test]
fn mid_notional_with_large_market_share_becomes_displayable() {
    let stats = stats("WIF", 200_000.0, 0.72, 6.5);
    let context = AltContractContext {
        ticker_quote_volume_24h_usd: Some(4_000_000.0),
        ticker_updated_at: Some(stats.ts - 10_000),
        oi_change_pct: Some(1.4),
        ..AltContractContext::default()
    };

    let impact = score_alt_impact(&stats, &context, AltContractMarketTier::Alt);

    assert!(impact.market_impact_ratio >= 0.03);
    assert!(impact.final_score >= impact.display_threshold);
    assert!(impact_displayable(&impact));
}

#[test]
fn large_nominal_flow_can_still_be_noise_when_market_impact_is_low() {
    let stats = stats("BTC", 500_000.0, 0.54, 1.3);
    let context = AltContractContext {
        ticker_quote_volume_24h_usd: Some(500_000_000_000.0),
        ticker_updated_at: Some(stats.ts - 10_000),
        oi_change_pct: Some(0.0),
        ..AltContractContext::default()
    };

    let impact = score_alt_impact(&stats, &context, AltContractMarketTier::UltraCore);

    assert!(impact.market_impact_ratio < 0.003);
    assert!(impact.final_score < impact.display_threshold);
    assert!(!impact_displayable(&impact));
}

#[test]
fn s_grade_requires_extreme_relative_impact_not_just_high_direction() {
    let stats = stats("ALT", 250_000.0, 0.88, 7.0);
    let context = AltContractContext {
        ticker_quote_volume_24h_usd: Some(5_000_000.0),
        ticker_updated_at: Some(stats.ts - 10_000),
        oi_change_1m_base: Some(25_000.0),
        oi_change_pct: Some(1.8),
        ..AltContractContext::default()
    };

    let impact = score_alt_impact(&stats, &context, AltContractMarketTier::Alt);

    assert!(impact_discord_ready(&impact));
    assert!(impact_s_ready(&impact));
}

#[test]
fn impact_without_a_reliable_reference_is_unavailable_and_not_displayable() {
    let stats = stats("NOREF", 9_000_000.0, 0.92, 12.0);
    let impact = score_alt_impact(
        &stats,
        &AltContractContext::default(),
        AltContractMarketTier::Alt,
    );

    assert_eq!(impact.reference_source, "unavailable");
    assert!(impact.reference_volume_24h_usd.is_none());
    assert!(impact.evidence_degraded);
    assert!(!impact_displayable(&impact));
    assert!(!impact_discord_ready(&impact));
}

#[test]
fn stale_ticker_does_not_become_a_reliable_impact_reference() {
    let stats = stats("STALE", 9_000_000.0, 0.92, 12.0);
    let context = AltContractContext {
        ticker_quote_volume_24h_usd: Some(5_000_000.0),
        ticker_updated_at: Some(stats.ts - 121_000),
        ..AltContractContext::default()
    };

    let impact = score_alt_impact(&stats, &context, AltContractMarketTier::Alt);

    assert_eq!(impact.reference_source, "unavailable");
    assert!(impact.evidence_degraded);
    assert!(!impact_displayable(&impact));
}

#[test]
fn local_rolling_reference_is_used_before_historical_baseline() {
    let stats = stats("LOCAL", 200_000.0, 0.72, 6.5);
    let context = AltContractContext {
        local_rolling_24h_notional_usd: Some(4_000_000.0),
        local_rolling_24h_updated_at: Some(stats.ts - 10_000),
        historical_baseline_notional_usd: Some(10_000_000.0),
        historical_baseline_updated_at: Some(stats.ts - 10_000),
        ..AltContractContext::default()
    };

    let impact = score_alt_impact(&stats, &context, AltContractMarketTier::Alt);

    assert_eq!(impact.reference_source, "local_rolling_24h");
    assert_eq!(impact.reference_volume_24h_usd, Some(4_000_000.0));
    assert!(!impact.evidence_degraded);
}

fn stats(
    symbol: &str,
    notional: f64,
    dominance: f64,
    dynamic_multiple: f64,
) -> AltContractWindowStats {
    let total_volume = 10_000.0;
    let net = total_volume * dominance;
    AltContractWindowStats {
        symbol: symbol.to_string(),
        product_id: format!("{symbol}USDT"),
        tier: AltContractSymbolTier::C,
        window_sec: 60,
        ts: 1_700_000_000_000,
        buy_volume_base: (total_volume + net) / 2.0,
        sell_volume_base: (total_volume - net) / 2.0,
        total_volume_base: total_volume,
        net_volume_base: net,
        total_notional_usd: notional,
        dominance,
        direction: AltContractDirection::Buy,
        trigger_price_usd: Some(notional / total_volume),
        price_move_pct: Some(0.8),
        price_threshold_pct: None,
        exchange_count: 1,
        main_exchange: Some("binance".to_string()),
        exchanges: vec![AltContractExchangeContribution {
            exchange: "binance".to_string(),
            total_volume_base: total_volume,
            total_notional_usd: notional,
            net_volume_base: net,
            dominance,
            trade_count: 100,
            ..AltContractExchangeContribution::default()
        }],
        dynamic_multiple: Some(dynamic_multiple),
        data_quality: 92,
        startup_age_ms: Some(120_000),
    }
}
