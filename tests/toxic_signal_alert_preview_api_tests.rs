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
async fn toxic_signal_alert_preview_api_returns_preview_only_contract() {
    let state = AppState::new(test_config());
    let app = router(state);
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0")
        .await
        .expect("bind");
    let addr = listener.local_addr().expect("addr");
    let server = tokio::spawn(async move {
        axum::serve(listener, app).await.expect("server");
    });

    let status = test_http_get(format!(
        "http://{addr}/api/toxicity/signal-alert-preview/status"
    ))
    .await
    .expect("status response");
    assert_eq!(status.status(), reqwest::StatusCode::OK);
    let status_payload: serde_json::Value = status.json().await.expect("status json");
    assert_eq!(status_payload["readOnly"], true);
    assert_eq!(status_payload["analysisOnly"], true);
    assert_eq!(status_payload["executionEnabled"], false);
    assert_eq!(status_payload["notificationSent"], false);
    assert_eq!(status_payload["executionTriggered"], false);
    assert_eq!(status_payload["previewOnly"], true);
    assert_eq!(status_payload["filter"]["viewOnly"], true);
    assert_eq!(
        status_payload["filter"]["persistentWatchlistEnabled"],
        false
    );
    assert_eq!(status_payload["filter"]["runtimeMonitorModified"], false);
    assert_eq!(status_payload["gate"]["telegramEnabled"], false);

    let preview = test_http_get(format!(
        "http://{addr}/api/toxicity/signal-alert-preview/recent"
    ))
    .await
    .expect("preview response");
    assert_eq!(preview.status(), reqwest::StatusCode::OK);
    let preview_payload: serde_json::Value = preview.json().await.expect("preview json");
    assert_eq!(preview_payload["readOnly"], true);
    assert_eq!(preview_payload["analysisOnly"], true);
    assert_eq!(preview_payload["executionEnabled"], false);
    assert_eq!(preview_payload["notificationSent"], false);
    assert_eq!(preview_payload["executionTriggered"], false);
    assert_eq!(preview_payload["previewOnly"], true);
    assert!(preview_payload["summary"].is_object());
    assert!(preview_payload["items"].is_array());
    assert!(preview_payload["markdown"].as_str().is_some());

    let filtered = test_http_get(format!(
        "http://{addr}/api/toxicity/signal-alert-preview/recent?symbol=%20btc-perp%20"
    ))
    .await
    .expect("filtered response");
    assert_eq!(filtered.status(), reqwest::StatusCode::OK);
    let filtered_payload: serde_json::Value = filtered.json().await.expect("filtered json");
    assert_eq!(filtered_payload["filter"]["symbol"], "BTC-PERP");

    let explain = test_http_get(format!(
        "http://{addr}/api/toxicity/signal-alert-preview/explain/missing-signal?symbol=BTC-PERP"
    ))
    .await
    .expect("explain response");
    assert_eq!(explain.status(), reqwest::StatusCode::OK);
    let explain_payload: serde_json::Value = explain.json().await.expect("explain json");
    assert_eq!(explain_payload["readOnly"], true);
    assert_eq!(explain_payload["analysisOnly"], true);
    assert_eq!(explain_payload["executionEnabled"], false);
    assert_eq!(explain_payload["notificationSent"], false);
    assert_eq!(explain_payload["executionTriggered"], false);
    assert_eq!(explain_payload["found"], false);
    assert_eq!(explain_payload["alertDecision"], "not_found");
    assert_eq!(
        explain_payload["reason"],
        "signal_id_not_found_in_alert_preview"
    );

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
        system_mode: Default::default(),
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
