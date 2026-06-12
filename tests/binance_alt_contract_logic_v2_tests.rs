use btc_toxic_flow_monitor_rs::binance_alt_contract_monitor::{
    config::{
        reset_binance_alt_contract_runtime_config, set_binance_alt_contract_runtime_config,
        BinanceAltContractRuntimeConfig,
    },
    detector::{
        detect_alt_contract_signal_with_context, window_confirmation_for, MarketImpulseContext,
    },
    service::BinanceAltContractService,
    types::{
        AltContractContext, AltContractDirection, AltContractExchange,
        AltContractExchangeContribution, AltContractSignalType, AltContractSymbolTier,
        AltContractTrade, AltContractTradeSide, AltContractWindowStats,
    },
};

fn config() -> BinanceAltContractRuntimeConfig {
    let mut config = BinanceAltContractRuntimeConfig::default();
    config.enabled = true;
    config.dry_run = true;
    config.discord.dry_run = true;
    config.data_quality.warmup_ms = 1;
    config
}

#[test]
fn true_long_build_requires_evidence_chain() {
    let config = config();
    let stats = stats(
        "SOL",
        AltContractDirection::Buy,
        70_000_000.0,
        0.82,
        1.2,
        7.0,
        AltContractSymbolTier::B,
    );
    let context = AltContractContext {
        oi_change_1m_base: Some(120_000.0),
        oi_change_pct: Some(1.7),
        oi_updated_at: Some(stats.ts - 10_000),
        funding_rate: Some(0.0002),
        persistence_windows: 3,
        ..AltContractContext::default()
    };
    let windows = vec![
        window_confirmation_for(&stats, &config),
        confirmed_window(15, 18_000_000.0, Some(6.2), 0.76),
    ];

    let signal = detect_alt_contract_signal_with_context(
        &stats,
        &context,
        &config,
        windows,
        MarketImpulseContext::default(),
    )
    .expect("long build signal");

    assert_eq!(
        signal.signal_type,
        AltContractSignalType::MainForceLongBuild
    );
    assert!(signal.main_force_confidence >= 75.0);
    assert!(signal.evidence_count >= 4);
    assert!(signal.evidence_tags.iter().any(|tag| tag == "oi_expanding"));
    assert!(signal
        .evidence_tags
        .iter()
        .any(|tag| tag == "multi_window_confirmed"));
    assert_eq!(signal.oi_quality, "fresh");
}

#[test]
fn short_squeeze_like_liquidation_is_not_labeled_main_force_long() {
    let config = config();
    let stats = stats(
        "WIF",
        AltContractDirection::Buy,
        65_000_000.0,
        0.88,
        3.4,
        8.0,
        AltContractSymbolTier::B,
    );
    let context = AltContractContext {
        oi_change_1m_base: Some(-180_000.0),
        oi_change_pct: Some(-2.3),
        oi_updated_at: Some(stats.ts - 8_000),
        liquidation_notional_usd: Some(12_000_000.0),
        liquidation_suspected: true,
        force_order_snapshot: true,
        persistence_windows: 2,
        ..AltContractContext::default()
    };

    let signal = detect_alt_contract_signal_with_context(
        &stats,
        &context,
        &config,
        vec![window_confirmation_for(&stats, &config)],
        MarketImpulseContext::default(),
    )
    .expect("liquidation signal");

    assert_ne!(
        signal.signal_type,
        AltContractSignalType::MainForceLongBuild
    );
    assert!(matches!(
        signal.signal_type,
        AltContractSignalType::LiquidationCascade | AltContractSignalType::AbnormalPump
    ));
    assert!(signal.main_force_confidence < 75.0);
    assert!(signal.final_result.contains("清算") || signal.final_result.contains("OI"));
}

#[test]
fn market_wide_move_downgrades_non_leading_coin_confidence() {
    let config = config();
    let stats = stats(
        "ALT",
        AltContractDirection::Buy,
        42_000_000.0,
        0.77,
        0.9,
        6.5,
        AltContractSymbolTier::B,
    );
    let context = AltContractContext {
        oi_change_1m_base: Some(90_000.0),
        oi_change_pct: Some(1.2),
        oi_updated_at: Some(stats.ts - 10_000),
        funding_rate: Some(0.0001),
        persistence_windows: 2,
        ..AltContractContext::default()
    };

    let clean = detect_alt_contract_signal_with_context(
        &stats,
        &context,
        &config,
        vec![
            window_confirmation_for(&stats, &config),
            confirmed_window(15, 12_000_000.0, Some(5.0), 0.7),
        ],
        MarketImpulseContext::default(),
    )
    .expect("clean signal");
    let contaminated = detect_alt_contract_signal_with_context(
        &stats,
        &context,
        &config,
        vec![
            window_confirmation_for(&stats, &config),
            confirmed_window(15, 12_000_000.0, Some(5.0), 0.7),
        ],
        MarketImpulseContext {
            market_wide_move: true,
            market_wide_direction: Some("buy".to_string()),
            market_impulse_ratio: 0.23,
            relative_strength_rank: Some(35),
        },
    )
    .expect("market-wide signal");

    assert!(contaminated.market_wide_move);
    assert!(contaminated.main_force_confidence < clean.main_force_confidence);
    assert_ne!(
        contaminated.signal_type,
        AltContractSignalType::MainForceLongBuild
    );
}

#[test]
fn downside_absorption_is_not_short_build() {
    let config = config();
    let stats = stats(
        "ARB",
        AltContractDirection::Sell,
        38_000_000.0,
        0.84,
        0.02,
        6.0,
        AltContractSymbolTier::B,
    );
    let context = AltContractContext {
        oi_change_1m_base: Some(40_000.0),
        oi_change_pct: Some(0.8),
        oi_updated_at: Some(stats.ts - 9_000),
        funding_rate: Some(0.0),
        persistence_windows: 2,
        ..AltContractContext::default()
    };

    let signal = detect_alt_contract_signal_with_context(
        &stats,
        &context,
        &config,
        vec![
            window_confirmation_for(&stats, &config),
            confirmed_window(15, 10_000_000.0, Some(4.5), 0.74),
        ],
        MarketImpulseContext::default(),
    )
    .expect("absorption signal");

    assert_eq!(
        signal.signal_type,
        AltContractSignalType::DownsideAbsorption
    );
    assert_ne!(
        signal.signal_type,
        AltContractSignalType::MainForceShortBuild
    );
}

#[test]
fn tier_e_spike_without_oi_confirmation_is_display_only_not_main_force() {
    let config = config();
    let stats = stats(
        "TINY",
        AltContractDirection::Buy,
        9_000_000.0,
        0.96,
        8.0,
        12.0,
        AltContractSymbolTier::E,
    );
    let context = AltContractContext {
        oi_updated_at: None,
        funding_rate: Some(0.0),
        persistence_windows: 1,
        ..AltContractContext::default()
    };

    let signal = detect_alt_contract_signal_with_context(
        &stats,
        &context,
        &config,
        vec![window_confirmation_for(&stats, &config)],
        MarketImpulseContext::default(),
    )
    .expect("tier e display signal");

    assert_ne!(
        signal.signal_type,
        AltContractSignalType::MainForceLongBuild
    );
    assert_eq!(signal.discord_reason, "low_liquidity_tier_guard");
    assert!(!signal.discord_would_send);
    assert_eq!(signal.oi_quality, "missing");
}

#[test]
fn post_signal_validation_marks_failed_when_long_loses_signal_vwap() {
    reset_binance_alt_contract_runtime_config();
    let mut config = config();
    config.symbol_universe.whitelist = vec!["SOLUSDT".to_string()];
    config.persistence_path = std::env::temp_dir().join(format!(
        "{}-bacm-v2-post-validation.jsonl",
        std::process::id()
    ));
    let _ = std::fs::remove_file(&config.persistence_path);
    set_binance_alt_contract_runtime_config(config.clone());
    let now = 1_700_000_000_000_i64;
    let service = BinanceAltContractService::new(true, true, now - 120_000);
    service.update_open_interest("SOLUSDT", now - 70_000, 1_000_000.0);
    service.update_open_interest("SOLUSDT", now, 1_250_000.0);

    let warmup = service.ingest_trade(AltContractTrade {
        ts: now - 10_000,
        exchange: AltContractExchange::Binance,
        symbol: "SOL".to_string(),
        product_id: "SOLUSDT".to_string(),
        price: 98.0,
        qty_base: 10.0,
        notional_usd: 980.0,
        side: AltContractTradeSide::Buy,
        trade_id: Some("pre".to_string()),
    });
    assert!(warmup.is_empty());

    let first = service.ingest_trade(AltContractTrade {
        ts: now,
        exchange: AltContractExchange::Binance,
        symbol: "SOL".to_string(),
        product_id: "SOLUSDT".to_string(),
        price: 100.0,
        qty_base: 1_000_000.0,
        notional_usd: 100_000_000.0,
        side: AltContractTradeSide::Buy,
        trade_id: Some("open".to_string()),
    });
    assert_eq!(first[0].post_signal_status, "pending");

    service.ingest_trade(AltContractTrade {
        ts: now + 5 * 60_000 + 1_000,
        exchange: AltContractExchange::Binance,
        symbol: "SOL".to_string(),
        product_id: "SOLUSDT".to_string(),
        price: 94.0,
        qty_base: 2_000.0,
        notional_usd: 188_000.0,
        side: AltContractTradeSide::Sell,
        trade_id: Some("retest".to_string()),
    });

    let latest = service.latest(Some("SOL"), 10);
    let updated = latest
        .items
        .iter()
        .find(|signal| signal.id == first[0].id)
        .expect("original signal");
    assert_eq!(updated.post_signal_status, "trap");
    assert_eq!(updated.retest_status, "lost");

    let _ = std::fs::remove_file(config.persistence_path);
    reset_binance_alt_contract_runtime_config();
}

fn confirmed_window(
    window_sec: u64,
    notional_usd: f64,
    dynamic_multiple: Option<f64>,
    directional_strength: f64,
) -> btc_toxic_flow_monitor_rs::binance_alt_contract_monitor::types::AltContractWindowConfirmation {
    btc_toxic_flow_monitor_rs::binance_alt_contract_monitor::types::AltContractWindowConfirmation {
        window_sec,
        notional_usd,
        dynamic_multiple,
        directional_strength,
        confirmed: true,
    }
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
