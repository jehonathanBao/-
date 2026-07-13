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
async fn toxic_signal_detail_api_returns_safety_contract_and_not_found_payloads() {
    let state = AppState::new(test_config());
    let app = router(state);
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0")
        .await
        .expect("bind");
    let addr = listener.local_addr().expect("addr");
    let server = tokio::spawn(async move {
        axum::serve(listener, app).await.expect("server");
    });

    let status = test_http_get(format!("http://{addr}/api/toxicity/signal-detail/status"))
        .await
        .expect("status response");
    assert_eq!(status.status(), reqwest::StatusCode::OK);
    let status_payload: serde_json::Value = status.json().await.expect("status json");
    assert_eq!(status_payload["readOnly"], true);
    assert_eq!(status_payload["runtimeModified"], false);
    assert_eq!(status_payload["analysisOnly"], true);
    assert_eq!(status_payload["executionEnabled"], false);
    assert_eq!(status_payload["manualReviewRequired"], true);

    let signal = test_http_get(format!(
        "http://{addr}/api/toxicity/signal-detail/missing-signal"
    ))
    .await
    .expect("signal response");
    assert_eq!(signal.status(), reqwest::StatusCode::OK);
    let signal_payload: serde_json::Value = signal.json().await.expect("signal json");
    assert_eq!(signal_payload["readOnly"], true);
    assert_eq!(signal_payload["available"], false);
    assert_eq!(
        signal_payload["reason"],
        "signal_id_not_found_in_read_only_signal_detail"
    );

    let group = test_http_get(format!(
        "http://{addr}/api/toxicity/signal-detail/group/missing-group"
    ))
    .await
    .expect("group response");
    assert_eq!(group.status(), reqwest::StatusCode::OK);
    let group_payload: serde_json::Value = group.json().await.expect("group json");
    assert_eq!(group_payload["readOnly"], true);
    assert_eq!(group_payload["available"], false);
    assert_eq!(
        group_payload["reason"],
        "group_id_not_found_in_read_only_signal_detail"
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
