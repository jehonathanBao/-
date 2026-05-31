mod support;
use support::test_http_get;

use btc_toxic_flow_monitor_rs::{
    api::server::router,
    app::AppState,
    config::{
        venues::{VenueConfig, VenueConfigs},
        AppConfig,
    },
    types::{market::Venue, toxic::ToxicSeverity},
};

#[tokio::test]
async fn whale_flow_compact_mode_static_ui_stays_view_only() {
    let state = AppState::new(test_config());
    let app = router(state);
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0")
        .await
        .expect("bind");
    let addr = listener.local_addr().expect("addr");
    let server = tokio::spawn(async move {
        axum::serve(listener, app).await.expect("server");
    });

    let dashboard = test_http_get(format!("http://{addr}/dashboard"))
        .await
        .expect("dashboard response");
    assert_eq!(dashboard.status(), reqwest::StatusCode::OK);
    let dashboard_html = dashboard.text().await.expect("dashboard text");
    assert!(dashboard_html.contains("whaleFlowCompactModeCard"));
    assert!(dashboard_html.contains("Whale Flow Operator Presets"));
    assert!(dashboard_html.contains("Whale Flow Compact View"));
    assert!(dashboard_html.contains("view-only"));
    assert!(dashboard_html.contains("Persistent preset disabled"));
    assert!(dashboard_html.contains("Runtime modified: false"));
    assert!(dashboard_html.contains("No threshold modified"));
    assert!(dashboard_html.contains("No config write"));
    assert!(dashboard_html.contains("No apply/reload"));

    let script = test_http_get(format!("http://{addr}/web/app.js"))
        .await
        .expect("script response");
    assert_eq!(script.status(), reqwest::StatusCode::OK);
    let script_text = script.text().await.expect("script text");
    assert!(script_text.contains("\"all\", \"All\""));
    assert!(script_text.contains("\"high_volume\", \"High Volume\""));
    assert!(script_text.contains("\"venue_confluence_satisfied\", \"Venue Confluence\""));
    assert!(script_text.contains("\"degraded_or_partial_data\", \"Degraded Data\""));
    assert!(script_text.contains("\"calibration_not_ready\", \"Calibration Not Ready\""));
    assert!(script_text.contains("\"needs_more_data\", \"Needs More Data\""));
    assert!(script_text.contains("\"not_enough_data\", \"Not Enough Data\""));
    assert!(script_text.contains("Reset Preset"));
    assert!(script_text.contains("Copy Preset View JSON"));
    assert!(script_text.contains("No whale flow items matched this preset"));
    assert!(script_text.contains("No high volume candidates"));
    assert!(script_text.contains("No degraded data quality candidates"));
    assert!(script_text.contains("No calibration blocked candidates"));
    assert!(script_text.contains("No needs_more_data candidates"));
    assert!(script_text.contains("No not_enough_data candidates"));
    assert!(!script_text.contains("Apply Preset"));
    assert!(!script_text.contains("Save Preset"));
    assert!(!script_text.contains("Persist Preset"));
    assert!(!script_text.contains("localStorage"));
    assert!(!script_text.contains("sessionStorage"));

    server.abort();
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
