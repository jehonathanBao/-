use std::{
    fs,
    path::PathBuf,
    sync::{Mutex, MutexGuard, OnceLock},
    time::Duration,
};

use btc_toxic_flow_monitor_rs::binance_alt_contract_monitor::{
    config::{
        enable_binance_alt_contract_symbol_for_watch, reset_binance_alt_contract_runtime_config,
        set_binance_alt_contract_runtime_config, BinanceAltContractRuntimeConfig,
        BinanceAltDataQualityConfig, BinanceAltDiscordConfig, BinanceAltStorageConfig,
    },
    detector::detect_alt_contract_signal,
    service::{BinanceAltContractQuery, BinanceAltContractService},
    types::{
        AltContractContext, AltContractDirection, AltContractExchangeContribution,
        AltContractImpactScore, AltContractMarketTier, AltContractSeverity, AltContractSymbolMeta,
        AltContractSymbolTier, AltContractWindowStats,
    },
};
use btc_toxic_flow_monitor_rs::storage::{
    binance_alt_contract_repo::BinanceAltContractRepo, SqliteStore,
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
        storage: btc_toxic_flow_monitor_rs::binance_alt_contract_monitor::config::BinanceAltStorageConfig {
            jsonl_archive_enabled: true,
            ..BinanceAltStorageConfig::default()
        },
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
            ticker_quote_volume_24h_usd: Some(3_000_000_000.0),
            ticker_updated_at: Some(unix_ms() - 10_000),
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
    assert_eq!(
        latest.items[0].market_tier,
        AltContractMarketTier::UltraCore
    );
    assert_eq!(latest.items[0].display_threshold_usd, 750_000.0);
    assert_eq!(latest.summary.signal_count, 1);
    assert_eq!(latest.summary.display_min_notional_usd, 150_000.0);
    assert_eq!(latest.summary.display_thresholds_usd.ultra_core, 750_000.0);
    assert_eq!(latest.summary.display_thresholds_usd.mainstream, 500_000.0);
    assert_eq!(latest.summary.display_thresholds_usd.alt, 150_000.0);
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
fn service_prunes_bacm_signal_cache_to_seven_days_and_compacts_persistence() {
    let _guard = guard();
    reset_binance_alt_contract_runtime_config();
    let path = temp_path("bacm-routes-cache-retention.jsonl");
    let _ = fs::remove_file(&path);
    let mut config = BinanceAltContractRuntimeConfig {
        enabled: true,
        dry_run: true,
        data_quality: BinanceAltDataQualityConfig {
            warmup_ms: 1,
            ..BinanceAltDataQualityConfig::default()
        },
        persistence_path: path.clone(),
        storage: btc_toxic_flow_monitor_rs::binance_alt_contract_monitor::config::BinanceAltStorageConfig {
            jsonl_archive_enabled: true,
            ..BinanceAltStorageConfig::default()
        },
        ..BinanceAltContractRuntimeConfig::default()
    };
    config.storage.signals_retention_days = 7;
    config.storage.cleanup_interval_sec = 60;
    set_binance_alt_contract_runtime_config(config.clone());

    let now = unix_ms();
    let service = BinanceAltContractService::new(true, true, now - 120_000);
    let context = AltContractContext {
        oi_change_1m_base: Some(100_000.0),
        oi_change_pct: Some(1.5),
        persistence_windows: 3,
        ticker_quote_volume_24h_usd: Some(3_000_000_000.0),
        ticker_updated_at: Some(unix_ms() - 10_000),
        ..AltContractContext::default()
    };
    let mut recent_window = stats("SOL", AltContractDirection::Buy);
    recent_window.ts = now - 60_000;
    let recent =
        detect_alt_contract_signal(&recent_window, &context, &config).expect("recent signal");
    let mut old = recent.clone();
    old.id = "old-bacm-cache-signal".to_string();
    old.ts = now - 8 * 86_400_000;
    old.severity = AltContractSeverity::Medium;
    let mut old_s = recent.clone();
    old_s.id = "old-s-bacm-cache-signal".to_string();
    old_s.ts = now - 8 * 86_400_000;
    old_s.severity = AltContractSeverity::S;

    assert!(service.insert_signal_for_tests(old.clone()));
    assert!(service.insert_signal_for_tests(old_s.clone()));
    assert!(service.insert_signal_for_tests(recent.clone()));
    assert_eq!(service.latest(Some("SOL"), 50).items.len(), 3);

    service.prune_expired_cache_for_tests(now);

    let latest = service.latest(Some("SOL"), 50);
    let latest_ids = latest
        .items
        .iter()
        .map(|signal| signal.id.as_str())
        .collect::<Vec<_>>();
    assert_eq!(latest_ids.len(), 2);
    assert!(latest_ids.contains(&recent.id.as_str()));
    assert!(latest_ids.contains(&old_s.id.as_str()));
    assert!(!latest_ids.contains(&old.id.as_str()));
    let persisted = fs::read_to_string(&path).expect("compacted persistence file");
    assert!(!persisted.contains(&old.id));
    assert!(persisted.contains(&old_s.id));
    assert!(persisted.contains(&recent.id));

    let restored = BinanceAltContractService::new(true, true, now - 120_000);
    let restored_latest = restored.latest(Some("SOL"), 50);
    let restored_ids = restored_latest
        .items
        .iter()
        .map(|signal| signal.id.as_str())
        .collect::<Vec<_>>();
    assert_eq!(restored_ids.len(), 2);
    assert!(restored_ids.contains(&recent.id.as_str()));
    assert!(restored_ids.contains(&old_s.id.as_str()));

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
        storage: btc_toxic_flow_monitor_rs::binance_alt_contract_monitor::config::BinanceAltStorageConfig {
            jsonl_archive_enabled: true,
            ..BinanceAltStorageConfig::default()
        },
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
            ticker_quote_volume_24h_usd: Some(3_000_000_000.0),
            ticker_updated_at: Some(unix_ms() - 10_000),
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
    low.alt_impact_score = impact_score(42.0);

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
fn service_filters_new_signals_by_alt_impact_and_keeps_legacy_threshold_fallback() {
    let _guard = guard();
    reset_binance_alt_contract_runtime_config();
    let path = temp_path("bacm-routes-tiered-display-thresholds.jsonl");
    let _ = fs::remove_file(&path);
    let config = BinanceAltContractRuntimeConfig {
        enabled: true,
        dry_run: true,
        data_quality: BinanceAltDataQualityConfig {
            warmup_ms: 1,
            ..BinanceAltDataQualityConfig::default()
        },
        persistence_path: path.clone(),
        storage: BinanceAltStorageConfig {
            jsonl_archive_enabled: true,
            ..BinanceAltStorageConfig::default()
        },
        ..BinanceAltContractRuntimeConfig::default()
    };
    set_binance_alt_contract_runtime_config(config.clone());

    let service = BinanceAltContractService::new(true, true, 1_699_999_900_000);
    let base = detect_alt_contract_signal(
        &stats("SOL", AltContractDirection::Buy),
        &AltContractContext {
            oi_change_1m_base: Some(100_000.0),
            oi_change_pct: Some(1.5),
            persistence_windows: 3,
            ticker_quote_volume_24h_usd: Some(3_000_000_000.0),
            ticker_updated_at: Some(unix_ms() - 10_000),
            ..AltContractContext::default()
        },
        &config,
    )
    .expect("base signal");

    for (product_id, notional, final_impact_score) in [
        ("BTCUSDT", 400_000.0, 41.0),
        ("ETHUSDT", 800_000.0, 82.0),
        ("XRPUSDT", 400_000.0, 45.0),
        ("ADAUSDT", 600_000.0, 75.0),
        ("PEPEUSDT", 100_000.0, 42.0),
        ("WIFUSDT", 200_000.0, 80.0),
    ] {
        let mut signal = base.clone();
        signal.id = format!("{product_id}-{notional}");
        signal.product_id = product_id.to_string();
        signal.symbol = product_id.trim_end_matches("USDT").to_string();
        signal.total_notional_usd = notional;
        signal.market_tier = config.classify_market_tier(product_id);
        signal.display_threshold_usd = 0.0;
        signal.alt_impact_score = impact_score(final_impact_score);
        assert!(service.insert_signal_for_tests(signal));
    }

    let latest = service.latest(None, 50);
    let product_ids = latest
        .items
        .iter()
        .map(|signal| signal.product_id.as_str())
        .collect::<Vec<_>>();

    assert_eq!(latest.summary.signal_count, 6);
    assert!(product_ids.contains(&"ETHUSDT"));
    assert!(product_ids.contains(&"ADAUSDT"));
    assert!(product_ids.contains(&"WIFUSDT"));
    assert!(!product_ids.contains(&"BTCUSDT"));
    assert!(!product_ids.contains(&"XRPUSDT"));
    assert!(!product_ids.contains(&"PEPEUSDT"));

    let _ = fs::remove_file(&path);
    reset_binance_alt_contract_runtime_config();
}

#[test]
fn bacm_views_keep_latest_ranked_impact_and_cursor_ordering_separate() {
    let _guard = guard();
    reset_binance_alt_contract_runtime_config();
    let path = temp_path("bacm-view-ordering.jsonl");
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
    let context = AltContractContext {
        oi_change_1m_base: Some(100_000.0),
        oi_change_pct: Some(1.5),
        persistence_windows: 3,
        ticker_quote_volume_24h_usd: Some(3_000_000_000.0),
        ticker_updated_at: Some(unix_ms() - 10_000),
        ..AltContractContext::default()
    };
    let base =
        detect_alt_contract_signal(&stats("SOL", AltContractDirection::Buy), &context, &config)
            .expect("base signal");

    let mut newest_low = base.clone();
    newest_low.id = "newest-low".to_string();
    newest_low.ts = 3_000;
    newest_low.abnormal_score = 50;
    newest_low.build_score = 50;
    newest_low.alt_impact_score = impact_score(72.0);

    let mut ranked = base.clone();
    ranked.id = "ranked-high".to_string();
    ranked.ts = 2_000;
    ranked.abnormal_score = 99;
    ranked.build_score = 99;
    ranked.alt_impact_score = impact_score(82.0);

    let mut impact = base;
    impact.id = "impact-high".to_string();
    impact.ts = 1_000;
    impact.abnormal_score = 80;
    impact.build_score = 80;
    impact.alt_impact_score = impact_score(95.0);

    for signal in [impact, ranked, newest_low] {
        assert!(service.insert_signal_for_tests(signal));
    }
    assert_eq!(service.latest(None, 10).items[0].id, "newest-low");
    assert_eq!(service.ranked(None, 10).items[0].id, "ranked-high");
    assert_eq!(service.top_impact(None, 10).items[0].id, "impact-high");

    let history = service.history(BinanceAltContractQuery {
        cursor_ts: Some(3_000),
        limit: Some(10),
        ..BinanceAltContractQuery::default()
    });
    assert_eq!(history.items.len(), 2);
    assert_eq!(history.items[0].id, "ranked-high");
    let _ = fs::remove_file(&path);
    reset_binance_alt_contract_runtime_config();
}

#[tokio::test(flavor = "current_thread")]
async fn bacm_sqlite_worker_persists_without_jsonl_realtime_writes() {
    let _guard = guard();
    reset_binance_alt_contract_runtime_config();
    let path = temp_path("bacm-sqlite-worker.sqlite");
    let _ = fs::remove_file(&path);
    let store = SqliteStore::open(path.to_str().expect("utf8 sqlite path")).expect("open sqlite");
    store.migrate().expect("migrate sqlite");
    let config = BinanceAltContractRuntimeConfig {
        enabled: true,
        dry_run: true,
        exchange: btc_toxic_flow_monitor_rs::binance_alt_contract_monitor::config::BinanceAltExchangeConfig {
            binance_enabled: false,
        },
        data_quality: BinanceAltDataQualityConfig {
            warmup_ms: 1,
            ..BinanceAltDataQualityConfig::default()
        },
        persistence_path: temp_path("bacm-worker-compatibility.jsonl"),
        ..BinanceAltContractRuntimeConfig::default()
    };
    set_binance_alt_contract_runtime_config(config.clone());
    let service =
        BinanceAltContractService::with_store(true, true, 1_699_999_900_000, Some(store.clone()));
    service.start();
    let signal = detect_alt_contract_signal(
        &stats("SOL", AltContractDirection::Buy),
        &AltContractContext {
            oi_change_1m_base: Some(100_000.0),
            oi_change_pct: Some(1.5),
            persistence_windows: 3,
            ticker_quote_volume_24h_usd: Some(3_000_000_000.0),
            ticker_updated_at: Some(unix_ms() - 10_000),
            ..AltContractContext::default()
        },
        &config,
    )
    .expect("signal");
    let id = signal.id.clone();
    assert!(service.insert_signal_for_tests(signal));

    for _ in 0..20 {
        if store
            .load_alt_contract_signals(10)
            .expect("read persisted signals")
            .iter()
            .any(|item| item.id == id)
        {
            break;
        }
        tokio::time::sleep(Duration::from_millis(50)).await;
    }
    assert!(store
        .load_alt_contract_signals(10)
        .expect("read persisted signals")
        .iter()
        .any(|item| item.id == id));
    assert!(store
        .load_alt_contract_events(10)
        .expect("read persisted events")
        .iter()
        .any(|event| {
            event.latest_signal_id.as_deref() == Some(id.as_str())
                && event.latest_snapshot.is_some()
                && event.peak_snapshot.is_some()
        }));
    service.stop();
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
    assert_eq!(
        summary.health_reason,
        "binance_alt_contract_monitor_disabled"
    );
    assert!(summary.monitored_symbols.is_empty());
    assert_eq!(summary.signal_count, 0);
    assert_eq!(summary.symbol_universe.mode, "disabled");
    assert_eq!(summary.symbol_universe.monitored_count, 0);
    assert!(summary.symbol_universe.tier_counts.is_empty());
    reset_binance_alt_contract_runtime_config();
}

#[test]
fn watch_activation_after_disabled_boot_is_visible_in_alt_contract_summary_and_latest() {
    let _guard = guard();
    reset_binance_alt_contract_runtime_config();
    let path = temp_path("bacm-watch-activation-after-disabled-boot.jsonl");
    let _ = fs::remove_file(&path);
    let mut config = BinanceAltContractRuntimeConfig {
        enabled: false,
        dry_run: true,
        data_quality: BinanceAltDataQualityConfig {
            warmup_ms: 1,
            ..BinanceAltDataQualityConfig::default()
        },
        persistence_path: path.clone(),
        ..BinanceAltContractRuntimeConfig::default()
    };
    config.exchange.binance_enabled = false;
    config.oi_scheduler.enabled = false;
    config.discord.enabled = false;
    set_binance_alt_contract_runtime_config(config.clone());

    let service = BinanceAltContractService::new(false, true, 1_699_999_900_000);

    let product_id = enable_binance_alt_contract_symbol_for_watch("aster")
        .expect("new token watch should activate matching alt contract symbol");
    assert_eq!(product_id, "ASTERUSDT");

    let summary = service.summary(None);
    assert!(summary.enabled);
    assert_eq!(summary.health_status, "unhealthy");
    assert_eq!(summary.symbol_universe.mode, "whitelist_only");
    assert_eq!(summary.symbol_universe.monitored_count, 1);
    assert_eq!(summary.symbol_universe.whitelist, vec!["ASTERUSDT"]);
    assert_eq!(summary.monitored_symbols, vec!["ASTERUSDT"]);

    let updated_config = btc_toxic_flow_monitor_rs::binance_alt_contract_monitor::config::binance_alt_contract_runtime_config();
    let mut signal = detect_alt_contract_signal(
        &stats("ASTER", AltContractDirection::Buy),
        &AltContractContext {
            oi_change_1m_base: Some(100_000.0),
            oi_change_pct: Some(1.5),
            persistence_windows: 3,
            ticker_quote_volume_24h_usd: Some(90_000_000.0),
            ticker_updated_at: Some(unix_ms() - 10_000),
            ..AltContractContext::default()
        },
        &updated_config,
    )
    .expect("watch signal");
    signal.alt_impact_score = impact_score(80.0);
    assert!(service.insert_signal_for_tests(signal));

    let latest = service.latest(Some("ASTER"), 50);
    assert!(latest.summary.enabled);
    assert_eq!(latest.summary.symbol_universe.monitored_count, 1);
    assert_eq!(latest.items.len(), 1);
    assert_eq!(latest.items[0].product_id, "ASTERUSDT");

    let _ = fs::remove_file(&path);
    reset_binance_alt_contract_runtime_config();
}

#[test]
fn disabled_service_does_not_restore_or_return_persisted_bacm_signals() {
    let _guard = guard();
    reset_binance_alt_contract_runtime_config();
    let path = temp_path("bacm-disabled-does-not-restore.jsonl");
    let _ = fs::remove_file(&path);
    let mut enabled_config = BinanceAltContractRuntimeConfig {
        enabled: true,
        dry_run: true,
        data_quality: BinanceAltDataQualityConfig {
            warmup_ms: 1,
            ..BinanceAltDataQualityConfig::default()
        },
        persistence_path: path.clone(),
        storage: BinanceAltStorageConfig {
            jsonl_archive_enabled: true,
            ..BinanceAltStorageConfig::default()
        },
        ..BinanceAltContractRuntimeConfig::default()
    };
    set_binance_alt_contract_runtime_config(enabled_config.clone());

    let enabled_service = BinanceAltContractService::new(true, true, 1_699_999_900_000);
    let signal = detect_alt_contract_signal(
        &stats("SOL", AltContractDirection::Buy),
        &AltContractContext {
            oi_change_1m_base: Some(100_000.0),
            oi_change_pct: Some(1.5),
            persistence_windows: 3,
            ticker_quote_volume_24h_usd: Some(3_000_000_000.0),
            ticker_updated_at: Some(unix_ms() - 10_000),
            ..AltContractContext::default()
        },
        &enabled_config,
    )
    .expect("signal");
    assert!(enabled_service.insert_signal_for_tests(signal));
    assert!(path.exists());

    enabled_config.enabled = false;
    set_binance_alt_contract_runtime_config(enabled_config);
    let disabled_service = BinanceAltContractService::new(false, true, 1_699_999_900_000);

    let latest = disabled_service.latest(None, 50);
    assert!(!latest.summary.enabled);
    assert!(latest.items.is_empty());
    assert_eq!(latest.summary.signal_count, 0);

    let history = disabled_service.history(BinanceAltContractQuery {
        limit: Some(50),
        ..BinanceAltContractQuery::default()
    });
    assert!(history.items.is_empty());
    assert_eq!(history.summary.signal_count, 0);

    let _ = fs::remove_file(&path);
    reset_binance_alt_contract_runtime_config();
}

#[test]
fn runtime_diagnostics_read_maintained_bacm_counters_without_signal_history_scans() {
    let _guard = guard();
    reset_binance_alt_contract_runtime_config();
    let config = BinanceAltContractRuntimeConfig {
        enabled: true,
        ..BinanceAltContractRuntimeConfig::default()
    };
    set_binance_alt_contract_runtime_config(config);

    let service = BinanceAltContractService::new(true, true, 1_700_000_000_000);
    service.update_symbol_universe(vec![
        AltContractSymbolMeta {
            symbol: "SOLUSDT".to_string(),
            product_id: "SOLUSDT".to_string(),
            tier: AltContractSymbolTier::B,
            market_tier: AltContractMarketTier::UltraCore,
            quote_volume_24h_usd: 3_000_000_000.0,
        },
        AltContractSymbolMeta {
            symbol: "DOGEUSDT".to_string(),
            product_id: "DOGEUSDT".to_string(),
            tier: AltContractSymbolTier::C,
            market_tier: AltContractMarketTier::Mainstream,
            quote_volume_24h_usd: 600_000_000.0,
        },
    ]);

    let diagnostics = service.runtime_diagnostics();

    assert_eq!(diagnostics.universe_symbol_count, 2);
    assert_eq!(diagnostics.active_symbol_count, 0);
    assert_eq!(diagnostics.trade_buffer_total, 0);
    assert_eq!(diagnostics.per_symbol_state_count, 0);
    assert_eq!(diagnostics.persistence_queue_depth, 0);
    assert!(diagnostics.oldest_persistence_age_ms.is_none());
    assert!(diagnostics.universe_last_refreshed_at.is_some());
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
        price_threshold_pct: None,
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

fn impact_score(final_score: f64) -> AltContractImpactScore {
    AltContractImpactScore {
        market_impact_ratio: if final_score >= 70.0 { 0.03 } else { 0.0008 },
        market_impact_score: if final_score >= 70.0 { 40.0 } else { 4.0 },
        liquidity_impact: if final_score >= 70.0 { 24.0 } else { 6.0 },
        cap_impact: 0.0,
        directional_strength: 0.74,
        directional_score: if final_score >= 70.0 { 20.0 } else { 10.0 },
        oi_confirmation: if final_score >= 70.0 { 10.0 } else { 0.0 },
        final_score,
        display_threshold: 70.0,
        discord_threshold: 85.0,
        s_threshold: 90.0,
        reference_volume_24h_usd: Some(3_000_000_000.0),
        reference_age_sec: Some(0),
        evidence_degraded: false,
        reference_source: "ticker_quote_volume_24h".to_string(),
        interpretation: if final_score >= 70.0 {
            "有效相对冲击".to_string()
        } else {
            "相对市场冲击偏弱".to_string()
        },
    }
}

fn unix_ms() -> i64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .expect("system clock before unix epoch")
        .as_millis() as i64
}
