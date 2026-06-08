use btc_toxic_flow_monitor_rs::{
    config::{
        venues::{VenueConfig, VenueConfigs},
        AppConfig,
    },
    market_data::event_bus::{MarketDataBus, MarketDataEvent},
    toxicity::vpin_service::VpinService,
    types::{
        market::{AggressorSide, NormalizedTrade, Venue},
        toxic::ToxicSeverity,
    },
};

#[tokio::test]
async fn service_updates_progress_and_completed_count() {
    let bus = MarketDataBus::new(128);
    let service = VpinService::new(bus.clone(), &test_config(true), None);
    service.start();

    bus.publish(MarketDataEvent::Trade(trade(1, 60.0)));
    tokio::time::sleep(std::time::Duration::from_millis(20)).await;
    assert_eq!(service.get_state().metrics.active_bucket_progress_btc, 60.0);

    bus.publish(MarketDataEvent::Trade(trade(2, 40.0)));
    tokio::time::sleep(std::time::Duration::from_millis(20)).await;
    assert_eq!(service.get_state().metrics.completed_bucket_count, 1);

    service.stop();
}

#[tokio::test]
async fn disabled_service_stays_noop() {
    let bus = MarketDataBus::new(128);
    let service = VpinService::new(bus.clone(), &test_config(false), None);
    service.start();

    bus.publish(MarketDataEvent::Trade(trade(1, 120.0)));
    tokio::time::sleep(std::time::Duration::from_millis(20)).await;
    let state = service.get_state();
    assert!(!state.metrics.enabled);
    assert_eq!(state.metrics.completed_bucket_count, 0);

    service.stop();
}

fn test_config(vpin_enabled: bool) -> AppConfig {
    AppConfig {
        app_env: "test".to_string(),
        read_only: true,
        api_host: "127.0.0.1".parse().expect("valid ip"),
        api_port: 0,
        symbol: "BTC-PERP".to_string(),
        toxic_volume_alert_btc: 1000.0,
        windows_ms: vec![1000, 5000, 15000, 60000],
        markout_horizons_ms: vec![1000, 5000, 15000],
        sweep_windows_ms: vec![1000, 5000, 15000],
        venues: VenueConfigs {
            binance: VenueConfig {
                venue: Venue::Binance,
                enabled: false,
            },
            bybit: VenueConfig {
                venue: Venue::Bybit,
                enabled: false,
            },
            okx: VenueConfig {
                venue: Venue::Okx,
                enabled: false,
            },
        },
        flow_compute_interval_ms: 50,
        markout_resolve_interval_ms: 50,
        sweep_compute_interval_ms: 50,
        toxic_compute_interval_ms: 50,
        telegram_enabled: false,
        telegram_bot_token: String::new(),
        telegram_chat_id: String::new(),
        alert_dedup_window_ms: 30_000,
        alert_min_severity: ToxicSeverity::Alert,
        alert_require_cross_venue: true,
        alert_require_markout: true,
        alert_require_liquidity_drain: false,
        sqlite_enabled: false,
        sqlite_path: ".runtime/test.sqlite".to_string(),
        snapshot_persist_interval_ms: 1000,
        raw_snapshot_enabled: false,
        raw_snapshot_sample_rate_ms: 1000,
        replay_enabled: true,
        replay_report_dir: ".runtime/reports".to_string(),
        vpin_enabled,
        vpin_bucket_size_btc: 100.0,
        vpin_lookback_buckets: 50,
        vpin_min_buckets: 10,
        vpin_spike_zscore: 2.5,
        vpin_high_threshold: 0.70,
        vpin_extreme_threshold: 0.85,
        vpin_persist_buckets: true,
        liquidation_enabled: true,
        liquidation_lookback_ms: 120_000,
        liquidation_cluster_band_bps: 6.0,
        liquidation_min_cluster_distance_bps: 5.0,
        liquidation_max_cluster_distance_bps: 150.0,
        liquidation_proximity_threshold_bps: 25.0,
        liquidation_min_cluster_touches: 3,
        liquidation_pressure_threshold: 0.65,
        liq_hunt_cluster_large_notional_usd: 50_000_000.0,
        liq_hunt_near_distance_bps: 25.0,
        liq_hunt_active_score: 75.0,
        liq_hunt_likely_score: 50.0,
        liq_hunt_watch_score: 30.0,
        book_stale_ms: 5000,
        max_buffer_age_ms: 120000,
        contract_whale_monitor:
            btc_toxic_flow_monitor_rs::config::env::ContractWhaleMonitorConfig {
                enabled: false,
                dry_run: true,
            },
    }
}

fn trade(ts: i64, size_btc: f64) -> NormalizedTrade {
    NormalizedTrade {
        venue: Venue::Binance,
        symbol: "BTC-PERP".to_string(),
        ts,
        price: 100_000.0,
        size_btc,
        size_usd: size_btc * 100_000.0,
        aggressor_side: AggressorSide::Buy,
        trade_id: Some(format!("trade-{ts}")),
    }
}
