use std::sync::Mutex;

use btc_toxic_flow_monitor_rs::{
    api::routes::build_venue_diagnostics_response,
    app::AppState,
    config::{
        venues::{VenueConfig, VenueConfigs},
        AppConfig,
    },
    types::{
        market::{classify_network_error, Venue, VenueConnectionStatus, VenueHealth},
        toxic::ToxicSeverity,
    },
};

static ENV_LOCK: Mutex<()> = Mutex::new(());

#[test]
fn network_error_classifier_covers_common_public_stream_failures() {
    assert_eq!(
        classify_network_error(Some("public stream connection timeout")),
        "timeout"
    );
    assert_eq!(
        classify_network_error(Some("HTTP status 403 forbidden")),
        "http_403"
    );
    assert_eq!(
        classify_network_error(Some("HTTP status 429 too many requests")),
        "http_429"
    );
    assert_eq!(
        classify_network_error(Some("rate limit exceeded")),
        "rate_limited"
    );
    assert_eq!(
        classify_network_error(Some("proxy tunnel failed")),
        "proxy_error"
    );
}

#[test]
fn diagnostics_marks_stale_activity_and_masks_proxy_credentials() {
    let _guard = ENV_LOCK.lock().expect("env lock poisoned");
    clear_proxy_env();
    std::env::set_var(
        "HTTPS_PROXY",
        "http://proxy_user:super_secret@proxy.example.com:18080",
    );
    let state = AppState::new(test_config());
    let mut health = VenueHealth::start_attempted_with_symbol(Venue::Binance, "BTC-PERP");
    let stale_ts = crate_now_ms().saturating_sub(90_000);
    health.status = VenueConnectionStatus::Connected;
    health.ws_connected = true;
    health.ws_last_connect_at_ms = Some(stale_ts);
    health.last_trade_ts = Some(stale_ts);
    health.last_parsed_trade_at_ms = Some(stale_ts);
    health.last_trade_message_at_ms = Some(stale_ts);
    health.trade_message_count = 1;
    state.set_health_for_tests(health);

    let diagnostics = build_venue_diagnostics_response(&state);
    let serialized = serde_json::to_string(&diagnostics).expect("serialize diagnostics");
    assert!(!serialized.contains("proxy_user"));
    assert!(!serialized.contains("super_secret"));

    let binance = diagnostics
        .venues
        .iter()
        .find(|venue| venue.venue == Venue::Binance)
        .expect("binance diagnostics");
    assert!(binance.proxy_enabled);
    assert!(!binance.proxy_supported);
    assert_eq!(binance.proxy_source.as_deref(), Some("HTTPS_PROXY"));
    assert_eq!(binance.proxy_host_masked.as_deref(), Some("***.com"));
    assert_eq!(binance.proxy_port_masked.as_deref(), Some("***"));
    assert!(!binance.trade_active);
    assert_eq!(binance.activity_status, "stale");

    clear_proxy_env();
}

#[test]
fn diagnostics_summarizes_network_error_without_mutating_runtime() {
    let state = AppState::new(test_config());
    let mut health = VenueHealth::start_attempted_with_symbol(Venue::Binance, "BTC-PERP");
    health.status = VenueConnectionStatus::Error;
    health.ws_connect_attempted = true;
    health.ws_last_error = Some("public stream connection timeout".to_string());
    health.ws_error_class = "timeout".to_string();
    health.last_network_error_class = "timeout".to_string();
    state.set_health_for_tests(health);

    let diagnostics = build_venue_diagnostics_response(&state);

    assert!(diagnostics.read_only);
    assert!(diagnostics.analysis_only);
    assert!(!diagnostics.execution_enabled);
    assert!(!diagnostics.runtime_modified);
    assert_eq!(diagnostics.summary.ws_connect_attempted_venues, 1);
    assert_eq!(diagnostics.summary.ws_connected_venues, 0);
    assert_eq!(diagnostics.summary.venues_with_network_errors, 1);
}

fn clear_proxy_env() {
    for key in ["WSS_PROXY", "HTTPS_PROXY", "HTTP_PROXY", "ALL_PROXY"] {
        std::env::remove_var(key);
    }
}

fn crate_now_ms() -> i64 {
    btc_toxic_flow_monitor_rs::normalizers::trade::now_ms()
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
                enabled: true,
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
        replay_enabled: false,
        replay_report_dir: ".runtime/test-reports".to_string(),
        vpin_enabled: true,
        vpin_bucket_size_btc: 100.0,
        vpin_lookback_buckets: 50,
        vpin_min_buckets: 10,
        vpin_spike_zscore: 2.5,
        vpin_high_threshold: 0.70,
        vpin_extreme_threshold: 0.85,
        vpin_persist_buckets: false,
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
        spot_whale_monitor: btc_toxic_flow_monitor_rs::config::env::SpotWhaleMonitorConfig {
            enabled: false,
            dry_run: true,
        },
    }
}
