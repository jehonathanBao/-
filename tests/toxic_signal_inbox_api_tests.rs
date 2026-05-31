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
async fn toxic_signal_inbox_api_returns_read_only_contract() {
    let state = AppState::new(test_config());
    let app = router(state);
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0")
        .await
        .expect("bind");
    let addr = listener.local_addr().expect("addr");
    let server = tokio::spawn(async move {
        axum::serve(listener, app).await.expect("server");
    });

    let status = test_http_get(format!("http://{addr}/api/toxicity/signal-inbox/status"))
        .await
        .expect("status response");
    assert_eq!(status.status(), reqwest::StatusCode::OK);
    let status_payload: serde_json::Value = status.json().await.expect("status json");
    assert_eq!(status_payload["readOnly"], true);
    assert_eq!(status_payload["runtimeModified"], false);
    assert_eq!(status_payload["analysisOnly"], true);
    assert_eq!(status_payload["executionEnabled"], false);
    assert_eq!(status_payload["manualReviewRequired"], true);
    assert_eq!(status_payload["runtimeWeightModified"], false);
    assert_eq!(status_payload["configModified"], false);

    let recent = test_http_get(format!("http://{addr}/api/toxicity/signal-inbox/recent"))
        .await
        .expect("recent response");
    assert_eq!(recent.status(), reqwest::StatusCode::OK);
    let recent_payload: serde_json::Value = recent.json().await.expect("recent json");
    assert_eq!(recent_payload["readOnly"], true);
    assert_eq!(recent_payload["runtimeModified"], false);
    assert!(recent_payload["items"].is_array());
    assert!(recent_payload["items"]
        .as_array()
        .expect("items array")
        .iter()
        .all(|item| item["readOnly"] == true
            && item["runtimeModified"] == false
            && item["analysisOnly"] == true
            && item["executionEnabled"] == false
            && item.get("operatorAction").is_some()));
    assert_eq!(recent_payload["filter"]["viewOnly"], true);
    assert_eq!(
        recent_payload["filter"]["persistentWatchlistEnabled"],
        false
    );
    assert_eq!(recent_payload["filter"]["runtimeMonitorModified"], false);

    let by_symbol = test_http_get(format!(
        "http://{addr}/api/toxicity/signal-inbox/{}",
        "ETH-PERP"
    ))
    .await
    .expect("symbol response");
    assert_eq!(by_symbol.status(), reqwest::StatusCode::OK);
    let by_symbol_payload: serde_json::Value = by_symbol.json().await.expect("symbol json");
    assert_eq!(by_symbol_payload["selectedSymbol"], "ETH-PERP");
    assert!(by_symbol_payload["items"]
        .as_array()
        .expect("symbol items")
        .is_empty());

    let filtered = test_http_get(format!(
        "http://{addr}/api/toxicity/signal-inbox/recent?symbol=%20btc-perp%20"
    ))
    .await
    .expect("filtered response");
    assert_eq!(filtered.status(), reqwest::StatusCode::OK);
    let filtered_payload: serde_json::Value = filtered.json().await.expect("filtered json");
    assert_eq!(filtered_payload["filter"]["symbol"], "BTC-PERP");
    assert_eq!(filtered_payload["selectedSymbol"], "BTC-PERP");

    let detail = test_http_get(format!(
        "http://{addr}/api/toxicity/signal-inbox/signal/missing-signal"
    ))
    .await
    .expect("signal response");
    assert_eq!(detail.status(), reqwest::StatusCode::OK);
    let detail_payload: serde_json::Value = detail.json().await.expect("signal json");
    assert_eq!(detail_payload["readOnly"], true);
    assert_eq!(detail_payload["available"], false);

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
    }
}
