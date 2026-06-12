use std::{
    fs,
    path::PathBuf,
    sync::{Mutex, MutexGuard, OnceLock},
};

use btc_toxic_flow_monitor_rs::binance_alt_contract_monitor::{
    config::{
        reset_binance_alt_contract_runtime_config, set_binance_alt_contract_runtime_config,
        BinanceAltContractRuntimeConfig, BinanceAltDataQualityConfig, BinanceAltDiscordConfig,
    },
    detector::detect_alt_contract_signal,
    service::{BinanceAltContractQuery, BinanceAltContractService},
    types::{
        AltContractContext, AltContractDirection, AltContractExchangeContribution,
        AltContractSymbolTier, AltContractWindowStats,
    },
};

fn guard() -> MutexGuard<'static, ()> {
    static GUARD: OnceLock<Mutex<()>> = OnceLock::new();
    GUARD
        .get_or_init(|| Mutex::new(()))
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner())
}

#[test]
fn service_latest_history_and_persistence_restore_bacm_signals() {
    let _guard = guard();
    reset_binance_alt_contract_runtime_config();
    let path = temp_path("bacm-routes-signals.jsonl");
    let _ = fs::remove_file(&path);
    let config = BinanceAltContractRuntimeConfig {
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
        persistence_path: path.clone(),
        ..BinanceAltContractRuntimeConfig::default()
    };
    set_binance_alt_contract_runtime_config(config.clone());

    let service = BinanceAltContractService::new(true, true, 1_699_999_900_000);
    let mut window = stats("SOL", AltContractDirection::Buy);
    window.ts = unix_ms();
    let mut signal = detect_alt_contract_signal(
        &window,
        &AltContractContext {
            oi_change_1m_base: Some(100_000.0),
            oi_change_pct: Some(1.5),
            funding_rate: Some(0.001),
            persistence_windows: 3,
            ..AltContractContext::default()
        },
        &config,
    )
    .expect("signal");
    signal.discord_eligible = true;
    signal.discord_would_send = true;
    signal.discord_reason = "dry_run".to_string();
    assert!(service.insert_signal_for_tests(signal.clone()));

    let latest = service.latest(Some("SOL"), 50);
    assert_eq!(latest.items.len(), 1);
    assert_eq!(latest.items[0].product_id, "SOLUSDT");
    assert_eq!(latest.summary.signal_count, 1);
    assert_eq!(latest.summary.display_min_notional_usd, 500_000.0);
    assert_eq!(latest.summary.signals1h, 1);
    assert_eq!(latest.summary.dry_run_stats.signals1h, 1);
    assert_eq!(latest.summary.dry_run_stats.would_send1h, 1);
    assert_eq!(latest.summary.dry_run_stats.signals24h, 1);
    assert_eq!(latest.summary.dry_run_stats.would_send24h, 1);
    assert_eq!(latest.summary.symbol_universe.mode, "all_binance_usdt_perp");
    assert!(latest
        .summary
        .symbol_universe
        .excluded_symbols
        .contains(&"BTCUSDT".to_string()));

    let history = service.history(BinanceAltContractQuery {
        symbol: Some("SOL".to_string()),
        severity: Some("s".to_string()),
        signal_type: Some("main_force_long_build".to_string()),
        direction: Some("buy".to_string()),
        would_send: Some(true),
        liquidation: Some(false),
        tier: Some("b".to_string()),
        limit: Some(10),
        ..BinanceAltContractQuery::default()
    });
    assert_eq!(history.items.len(), 1);
    assert!(history.items[0].main_force_confidence >= 75.0);
    assert!(history.items[0].evidence_count >= 4);

    let liquidation_history = service.history(BinanceAltContractQuery {
        symbol: Some("SOL".to_string()),
        liquidation: Some(true),
        limit: Some(10),
        ..BinanceAltContractQuery::default()
    });
    assert_eq!(liquidation_history.items.len(), 0);

    let filtered_out = service.history(BinanceAltContractQuery {
        symbol: Some("SOL".to_string()),
        direction: Some("sell".to_string()),
        limit: Some(10),
        ..BinanceAltContractQuery::default()
    });
    assert_eq!(filtered_out.items.len(), 0);

    let restored = BinanceAltContractService::new(true, true, 1_699_999_900_000);
    let restored_latest = restored.latest(Some("SOL"), 50);
    assert_eq!(restored_latest.items.len(), 1);
    assert_eq!(restored_latest.items[0].id, signal.id);

    let _ = fs::remove_file(&path);
    reset_binance_alt_contract_runtime_config();
}

#[test]
fn service_filters_low_notional_signals_from_frontend_lists() {
    let _guard = guard();
    reset_binance_alt_contract_runtime_config();
    let path = temp_path("bacm-routes-display-min-notional.jsonl");
    let _ = fs::remove_file(&path);
    let config = BinanceAltContractRuntimeConfig {
        enabled: true,
        dry_run: true,
        data_quality: BinanceAltDataQualityConfig {
            warmup_ms: 1,
            ..BinanceAltDataQualityConfig::default()
        },
        persistence_path: path.clone(),
        ..BinanceAltContractRuntimeConfig::default()
    };
    set_binance_alt_contract_runtime_config(config.clone());

    let service = BinanceAltContractService::new(true, true, 1_699_999_900_000);
    let high = detect_alt_contract_signal(
        &stats("SOL", AltContractDirection::Buy),
        &AltContractContext {
            oi_change_1m_base: Some(100_000.0),
            oi_change_pct: Some(1.5),
            persistence_windows: 3,
            ..AltContractContext::default()
        },
        &config,
    )
    .expect("high notional signal");
    let mut low = high.clone();
    low.id = "low-notional-doge".to_string();
    low.symbol = "DOGE".to_string();
    low.product_id = "DOGEUSDT".to_string();
    low.total_notional_usd = 499_999.0;

    assert!(service.insert_signal_for_tests(high));
    assert!(service.insert_signal_for_tests(low));

    let latest = service.latest(None, 50);
    assert_eq!(latest.summary.signal_count, 2);
    assert_eq!(latest.items.len(), 1);
    assert_eq!(latest.items[0].product_id, "SOLUSDT");

    let history = service.history(BinanceAltContractQuery {
        limit: Some(50),
        ..BinanceAltContractQuery::default()
    });
    assert_eq!(history.items.len(), 1);
    assert_eq!(history.items[0].product_id, "SOLUSDT");

    let _ = fs::remove_file(&path);
    reset_binance_alt_contract_runtime_config();
}

#[test]
fn disabled_summary_is_read_only_and_lists_configured_symbols() {
    let _guard = guard();
    reset_binance_alt_contract_runtime_config();
    let config = BinanceAltContractRuntimeConfig {
        enabled: false,
        ..BinanceAltContractRuntimeConfig::default()
    };
    set_binance_alt_contract_runtime_config(config);

    let service = BinanceAltContractService::new(false, true, 1_700_000_000_000);
    let summary = service.summary(None);

    assert!(!summary.enabled);
    assert!(summary.read_only);
    assert_eq!(summary.health_status, "disabled");
    assert!(summary
        .monitored_symbols
        .iter()
        .all(|symbol| symbol != "BTCUSDT" && symbol != "ETHUSDT"));
    assert!(summary.monitored_symbols.len() <= 12);
    assert!(summary.symbol_universe.monitored_count >= summary.monitored_symbols.len());
    reset_binance_alt_contract_runtime_config();
}

fn stats(symbol: &str, direction: AltContractDirection) -> AltContractWindowStats {
    let net = if direction == AltContractDirection::Buy {
        8_500.0
    } else {
        -8_500.0
    };
    AltContractWindowStats {
        symbol: symbol.to_string(),
        product_id: format!("{symbol}USDT"),
        tier: AltContractSymbolTier::B,
        window_sec: 60,
        ts: 1_700_000_000_000,
        buy_volume_base: if net > 0.0 { 9_250.0 } else { 750.0 },
        sell_volume_base: if net < 0.0 { 9_250.0 } else { 750.0 },
        total_volume_base: 10_000.0,
        net_volume_base: net,
        total_notional_usd: 120_000_000.0,
        dominance: 0.85,
        direction,
        trigger_price_usd: Some(12_000.0),
        price_move_pct: Some(1.0),
        exchange_count: 1,
        main_exchange: Some("binance".to_string()),
        exchanges: vec![AltContractExchangeContribution {
            exchange: "binance".to_string(),
            total_volume_base: 10_000.0,
            total_notional_usd: 120_000_000.0,
            net_volume_base: net,
            dominance: 0.85,
            trade_count: 100,
            ..AltContractExchangeContribution::default()
        }],
        dynamic_multiple: Some(10.0),
        data_quality: 92,
        startup_age_ms: Some(120_000),
    }
}

fn temp_path(name: &str) -> PathBuf {
    std::env::temp_dir().join(format!("{}-{}", std::process::id(), name))
}

fn unix_ms() -> i64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .expect("system clock before unix epoch")
        .as_millis() as i64
}
