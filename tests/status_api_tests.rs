use btc_toxic_flow_monitor_rs::{
    api::routes::build_status_response,
    app::AppState,
    config::{
        venues::{VenueConfig, VenueConfigs},
        AppConfig,
    },
    market_data::event_bus::{MarketDataBus, MarketDataEvent},
    types::{
        market::{Venue, VenueHealth},
        status::MarketDataQualityStatus,
        toxic::ToxicSeverity,
    },
};

#[test]
fn status_response_matches_r0_contract() {
    let state = AppState::new(test_config());
    let status = build_status_response(&state);

    assert_eq!(status.app, "btc-toxic-flow-monitor-rs");
    assert!(status.read_only);
    assert_eq!(
        status.config_source,
        "env_overrides_toml_overrides_defaults"
    );
    assert!(!status.runtime_control.monitoring_started);
    assert!(status.runtime_control.one_click_start_enabled);
    assert_eq!(
        status.runtime_control.start_action_label,
        "One-click Start Monitoring"
    );
    assert_eq!(status.runtime_control.start_action_mode, "monitoring_only");
    assert_eq!(
        status.runtime_control.start_state,
        btc_toxic_flow_monitor_rs::types::status::RuntimeStartState::Stopped
    );
    assert_eq!(status.runtime_control.last_start_at_ms, None);
    assert_eq!(status.runtime_control.last_start_error, None);
    assert_eq!(status.runtime_control.start_attempt_count, 0);
    assert_eq!(
        status.runtime_control.last_start_result,
        btc_toxic_flow_monitor_rs::types::status::RuntimeStartResult::None
    );
    assert_eq!(
        status.runtime_control.stop_state,
        btc_toxic_flow_monitor_rs::types::status::RuntimeStopState::Stopped
    );
    assert_eq!(status.runtime_control.last_stop_at_ms, None);
    assert_eq!(status.runtime_control.last_stop_error, None);
    assert_eq!(status.runtime_control.stop_attempt_count, 0);
    assert_eq!(
        status.runtime_control.last_stop_result,
        btc_toxic_flow_monitor_rs::types::status::RuntimeStopResult::None
    );
    assert_eq!(status.symbol, "BTC-PERP");
    assert_eq!(status.threshold_btc, 1000.0);
    assert_eq!(status.windows_ms, vec![1000, 5000, 15000, 60000]);
    assert!(!status.venues["binance"].enabled);
    assert!(!status.venues["bybit"].enabled);
    assert!(!status.venues["okx"].enabled);
    assert_eq!(status.venues["binance"].enable_flag_name, "ENABLE_BINANCE");
    assert!(!status.venues["binance"].enable_flag_value);
    assert!(!status.venues["binance"].enable_source.is_empty());
    assert_eq!(
        status.venues["binance"].disabled_reason.as_deref(),
        Some("env_or_config_flag_false")
    );
    assert_eq!(status.venues["binance"].requested_symbol, "BTC-PERP");
    assert_eq!(
        status.venues["binance"].venue_symbol.as_deref(),
        Some("BTCUSDT")
    );
    assert_eq!(status.venues["binance"].symbol_mapping_status, "ok");
    assert!(!status.venues["binance"].connector_constructed);
    assert!(!status.venues["binance"].start_attempted);
    assert_eq!(
        status.market_data_quality.status,
        MarketDataQualityStatus::NoData
    );
    assert_eq!(status.market_data_quality.event_bus_dropped_events, 0);
    assert_eq!(status.market_data_quality.event_bus_send_errors, 0);
    assert_eq!(status.market_data_quality.flow_window_lagged_events, 0);
    assert_eq!(status.market_data_quality.markout_lagged_events, 0);
    assert_eq!(status.market_data_quality.vpin_lagged_events, 0);
    assert!(!status.market_data_quality.flow_windows_populated);
    assert!(status
        .market_data_quality
        .operator_warning
        .is_some_and(|warning| warning.contains("No flow window")));
    assert!(status.markout.enabled);
    assert_eq!(status.markout.horizons_ms, vec![1000, 5000, 15000]);
    assert_eq!(status.markout.pending_samples, 0);
    assert_eq!(status.markout.resolved_samples, 0);
    assert_eq!(status.markout.expired_samples, 0);
    assert!(status.sweep.enabled);
    assert_eq!(status.sweep.windows_ms, vec![1000, 5000, 15000]);
    assert!(!status.sweep.last_sweep_detected);
    assert!(status.vpin.enabled);
    assert_eq!(status.vpin.bucket_size_btc, 100.0);
    assert_eq!(status.vpin.completed_bucket_count, 0);
    assert_eq!(status.vpin.vpin, None);
    assert!(!status.vpin.vpin_spike);
    assert!(!status.alerts.telegram_enabled);
    assert_eq!(status.alerts.sent_count, 0);
    assert_eq!(status.alerts.suppressed_count, 0);
}

#[test]
fn market_data_quality_degrades_when_consumers_lag() {
    let state = AppState::new(test_config());
    state.market_data_quality().record_flow_window_lagged(3);
    state.market_data_quality().record_markout_lagged(1);

    let status = build_status_response(&state);

    assert_eq!(
        status.market_data_quality.status,
        MarketDataQualityStatus::Degraded
    );
    assert_eq!(status.market_data_quality.flow_window_lagged_events, 3);
    assert_eq!(status.market_data_quality.markout_lagged_events, 1);
    assert!(status.market_data_quality.last_lagged_at_ms.is_some());
    assert!(status
        .market_data_quality
        .operator_warning
        .is_some_and(|warning| warning.contains("lagged or dropped")));
}

#[test]
fn market_data_bus_send_error_increments_drop_counters() {
    let bus = MarketDataBus::new(1);

    bus.publish(MarketDataEvent::VenueHealth(VenueHealth::disabled(
        Venue::Binance,
    )));

    let snapshot = bus.quality_tracker().snapshot();
    assert_eq!(snapshot.event_bus_send_errors, 1);
    assert_eq!(snapshot.event_bus_dropped_events, 1);
}

fn test_config() -> AppConfig {
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
        flow_compute_interval_ms: 250,
        markout_resolve_interval_ms: 250,
        sweep_compute_interval_ms: 250,
        toxic_compute_interval_ms: 250,
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
        replay_enabled: false,
        replay_report_dir: ".runtime/reports".to_string(),
        vpin_enabled: true,
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
    }
}
