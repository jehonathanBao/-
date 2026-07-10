use btc_toxic_flow_monitor_rs::binance_alt_contract_monitor::{
    config::{
        reset_binance_alt_contract_runtime_config, set_binance_alt_contract_runtime_config,
        BinanceAltContractRuntimeConfig, BinanceAltDataQualityConfig, BinanceAltDiscordConfig,
    },
    detector::detect_alt_contract_signal,
    service::BinanceAltContractService,
    types::{
        AltContractContext, AltContractDirection, AltContractExchange,
        AltContractExchangeContribution, AltContractSeverity, AltContractSignalType,
        AltContractSymbolTier, AltContractTrade, AltContractTradeSide, AltContractWindowStats,
    },
};

fn test_config() -> BinanceAltContractRuntimeConfig {
    BinanceAltContractRuntimeConfig {
        enabled: true,
        dry_run: true,
        discord: BinanceAltDiscordConfig {
            dry_run: true,
            ..BinanceAltDiscordConfig::default()
        },
        data_quality: BinanceAltDataQualityConfig {
            warmup_ms: 1,
            ..BinanceAltDataQualityConfig::default()
        },
        ..BinanceAltContractRuntimeConfig::default()
    }
}

#[test]
fn detects_main_force_long_build_when_flow_oi_and_price_align() {
    reset_binance_alt_contract_runtime_config();
    let config = test_config();
    let stats = stats(
        "SOL",
        AltContractDirection::Buy,
        140_000_000.0,
        0.86,
        1.05,
        10.0,
        AltContractSymbolTier::B,
    );
    let context = AltContractContext {
        oi_change_1m_base: Some(240_000.0),
        oi_change_pct: Some(1.8),
        oi_updated_at: Some(stats.ts - 10_000),
        funding_rate: Some(0.0),
        persistence_windows: 3,
        ticker_quote_volume_24h_usd: Some(4_000_000_000.0),
        ..AltContractContext::default()
    };

    let signal = detect_alt_contract_signal(&stats, &context, &config).expect("signal");

    assert_eq!(
        signal.signal_type,
        AltContractSignalType::MainForceLongBuild
    );
    assert_eq!(signal.direction, AltContractDirection::Buy);
    assert!(signal.main_force_confidence >= 75.0);
    assert!(signal.evidence_count >= 4);
    assert_eq!(signal.severity, AltContractSeverity::S);
    assert!(
        !signal.discord_eligible,
        "a single-window observation must remain display-only; discord_reason={} alert_kind={} build={} abnormal={} confidence={} evidence={} oi_quality={}",
        signal.discord_reason,
        signal.discord_alert_kind,
        signal.build_score,
        signal.abnormal_score,
        signal.main_force_confidence,
        signal.evidence_count,
        signal.oi_quality
    );
    assert!(!signal.discord_would_send);
    assert_eq!(signal.discord_reason, "semantic_interpretation_only");
    assert!(!signal.explain_tags.is_empty());
    assert!(signal.abnormal_explanation.contains("异常分"));
    assert!(signal.build_explanation.contains("建仓分"));
    assert!(signal
        .liquidation_explanation
        .contains("未检测到明显清算驱动"));
    assert!(signal.read_only);
    assert!(!signal.execution_enabled);
}

#[test]
fn detects_main_force_short_build_when_sell_flow_and_oi_align() {
    reset_binance_alt_contract_runtime_config();
    let config = test_config();
    let stats = stats(
        "DOGE",
        AltContractDirection::Sell,
        90_000_000.0,
        0.82,
        -1.1,
        9.5,
        AltContractSymbolTier::B,
    );
    let context = AltContractContext {
        oi_change_1m_base: Some(950_000_000.0),
        oi_change_pct: Some(2.1),
        funding_rate: Some(-0.0011),
        persistence_windows: 3,
        ..AltContractContext::default()
    };

    let signal = detect_alt_contract_signal(&stats, &context, &config).expect("signal");

    assert_eq!(
        signal.signal_type,
        AltContractSignalType::MainForceShortBuild
    );
    assert_eq!(signal.direction, AltContractDirection::Sell);
    assert!(signal.main_force_confidence >= 75.0);
    assert!(signal.evidence_count >= 4);
    assert!(signal.direction_bias < 0);
}

#[test]
fn liquidation_cascade_is_not_labeled_as_main_force_build() {
    reset_binance_alt_contract_runtime_config();
    let config = test_config();
    let stats = stats(
        "XRP",
        AltContractDirection::Sell,
        120_000_000.0,
        0.90,
        -2.2,
        11.0,
        AltContractSymbolTier::A,
    );
    let context = AltContractContext {
        oi_change_1m_base: Some(-18_000_000.0),
        oi_change_pct: Some(-2.4),
        liquidation_notional_usd: Some(35_000_000.0),
        liquidation_suspected: true,
        force_order_snapshot: true,
        persistence_windows: 2,
        ..AltContractContext::default()
    };

    let signal = detect_alt_contract_signal(&stats, &context, &config).expect("signal");

    assert_eq!(
        signal.signal_type,
        AltContractSignalType::LiquidationCascade
    );
    assert!(signal.abnormal_score > signal.build_score);
    assert!(signal.final_result.contains("清算"));
}

#[test]
fn tier_d_false_positive_is_filtered_before_alerting() {
    reset_binance_alt_contract_runtime_config();
    let config = test_config();
    let stats = stats(
        "TINY",
        AltContractDirection::Buy,
        3_000_000.0,
        0.92,
        0.02,
        5.0,
        AltContractSymbolTier::D,
    );
    let context = AltContractContext::default();

    let signal = detect_alt_contract_signal(&stats, &context, &config);

    assert!(signal.is_none());
}

#[test]
fn tier_d_signal_uses_guard_before_discord_dry_run() {
    reset_binance_alt_contract_runtime_config();
    let mut config = test_config();
    config.tier_d_min_signal_score = 40;
    let stats = stats(
        "TINY",
        AltContractDirection::Buy,
        20_000_000.0,
        0.96,
        1.4,
        12.0,
        AltContractSymbolTier::D,
    );
    let context = AltContractContext {
        liquidation_suspected: true,
        force_order_snapshot: true,
        liquidation_notional_usd: Some(8_000_000.0),
        ..AltContractContext::default()
    };

    let signal = detect_alt_contract_signal(&stats, &context, &config).expect("tier d signal");

    assert_eq!(signal.discord_reason, "tier_d_guard");
    assert!(!signal.discord_would_send);
    assert!(signal.severity.rank() <= AltContractSeverity::High.rank());
    assert!(signal
        .explain_tags
        .iter()
        .any(|tag| tag == "tier_d_extra_guard"));
}

#[test]
fn service_context_updates_enrich_generated_signal() {
    reset_binance_alt_contract_runtime_config();
    let mut config = test_config();
    config.symbol_universe.whitelist = vec!["SOLUSDT".to_string()];
    config.persistence_path =
        std::env::temp_dir().join(format!("{}-bacm-context-update.jsonl", std::process::id()));
    let _ = std::fs::remove_file(&config.persistence_path);
    set_binance_alt_contract_runtime_config(config.clone());
    let now = unix_ms();
    let service = BinanceAltContractService::new(true, true, now - 120_000);

    service.update_open_interest("SOLUSDT", now - 70_000, 1_000_000.0);
    service.update_open_interest("SOLUSDT", now, 1_250_000.0);
    service.update_mark_price_context("SOLUSDT", now, Some(174.9), Some(0.00021));
    service.update_ticker_context("SOLUSDT", now, Some(175.5), Some(600_000_000.0), Some(2.4));
    service.update_funding_context("SOLUSDT", Some(0.00021));
    service.update_liquidation_context("SOLUSDT", now, 1_000_000.0);

    let warmup = service.ingest_trade(AltContractTrade {
        ts: now - 10_000,
        exchange: AltContractExchange::Binance,
        symbol: "SOL".to_string(),
        product_id: "SOLUSDT".to_string(),
        price: 170.0,
        qty_base: 10.0,
        notional_usd: 1_700.0,
        side: AltContractTradeSide::Buy,
        trade_id: Some("pre".to_string()),
    });
    assert!(warmup.is_empty());

    let signals = service.ingest_trade(AltContractTrade {
        ts: now,
        exchange: AltContractExchange::Binance,
        symbol: "SOL".to_string(),
        product_id: "SOLUSDT".to_string(),
        price: 175.0,
        qty_base: 1_000_000.0,
        notional_usd: 175_000_000.0,
        side: AltContractTradeSide::Buy,
        trade_id: Some("agg-1".to_string()),
    });

    let signal = signals.first().expect("context-enriched signal");
    assert_ne!(
        signal.signal_type,
        AltContractSignalType::MainForceLongBuild
    );
    assert!(
        matches!(
            signal.signal_type,
            AltContractSignalType::AbnormalPump
                | AltContractSignalType::LiquidationCascade
                | AltContractSignalType::UpsideResistance
        ),
        "liquidation context should avoid direct main-force wording"
    );
    if signal.window_sec == 60 {
        assert_eq!(signal.oi_change_1m_base, Some(250_000.0));
    } else {
        assert_eq!(signal.window_sec, 15);
        assert!(signal.oi_change_1m_base.is_none());
        assert!(signal.oi_change_5m_base.is_none());
    }
    assert_eq!(signal.funding_rate, Some(0.00021));
    assert_eq!(signal.liquidation_notional_usd, Some(1_000_000.0));
    assert!(signal.force_order_snapshot);
    assert!(signal.read_only);
    assert!(!signal.execution_enabled);

    let summary = service.summary(Some("SOL"));
    assert!(summary.all_market_context.mark_price_connected);
    assert!(summary.all_market_context.ticker_connected);
    assert!(summary.all_market_context.force_order_connected);
    assert_eq!(summary.all_market_context.last_mark_price_at, Some(now));
    assert_eq!(summary.all_market_context.last_ticker_at, Some(now));
    assert!(summary
        .all_market_context
        .candidate_symbols
        .iter()
        .any(|symbol| symbol == "SOLUSDT"));
    assert!(summary
        .all_market_context
        .hot_oi_symbols
        .iter()
        .any(|symbol| symbol == "SOLUSDT"));

    let _ = std::fs::remove_file(config.persistence_path);
    reset_binance_alt_contract_runtime_config();
}

#[test]
fn detector_throttle_does_not_block_large_trade_after_small_trade() {
    reset_binance_alt_contract_runtime_config();
    let mut config = test_config();
    config.detector.scan_interval_ms = 60_000;
    config.symbol_universe.whitelist = vec!["SOLUSDT".to_string()];
    config.persistence_path = std::env::temp_dir().join(format!(
        "{}-bacm-detector-throttle.jsonl",
        std::process::id()
    ));
    let _ = std::fs::remove_file(&config.persistence_path);
    set_binance_alt_contract_runtime_config(config.clone());

    let now = unix_ms();
    let service = BinanceAltContractService::new(true, true, now - 120_000);
    service.update_open_interest("SOLUSDT", now - 70_000, 1_000_000.0);
    service.update_open_interest("SOLUSDT", now, 1_250_000.0);

    let small = service.ingest_trade(AltContractTrade {
        ts: now,
        exchange: AltContractExchange::Binance,
        symbol: "SOL".to_string(),
        product_id: "SOLUSDT".to_string(),
        price: 170.0,
        qty_base: 1.0,
        notional_usd: 170.0,
        side: AltContractTradeSide::Buy,
        trade_id: Some("small".to_string()),
    });
    assert!(small.is_empty());

    let signals = service.ingest_trade(AltContractTrade {
        ts: now + 1_000,
        exchange: AltContractExchange::Binance,
        symbol: "SOL".to_string(),
        product_id: "SOLUSDT".to_string(),
        price: 175.0,
        qty_base: 1_000_000.0,
        notional_usd: 175_000_000.0,
        side: AltContractTradeSide::Buy,
        trade_id: Some("large".to_string()),
    });

    assert!(
        !signals.is_empty(),
        "large trade should force a detector scan despite per-symbol throttling"
    );

    let _ = std::fs::remove_file(config.persistence_path);
    reset_binance_alt_contract_runtime_config();
}

fn unix_ms() -> i64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .expect("system clock before unix epoch")
        .as_millis() as i64
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
