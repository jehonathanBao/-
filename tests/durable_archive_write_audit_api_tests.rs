mod support;

use btc_toxic_flow_monitor_rs::{
    api::server::router,
    app::AppState,
    config::{
        venues::{VenueConfig, VenueConfigs},
        AppConfig,
    },
    types::{market::Venue, toxic::ToxicSeverity},
};
use support::test_http_client;

#[tokio::test]
async fn durable_archive_write_audit_endpoints_are_preview_only_and_read_only() {
    let base = std::env::temp_dir().join("btc-toxic-flow-s16a-audit-preview");
    let db_path = base.with_extension("db");
    let jsonl_path = base.with_extension("jsonl");
    let sqlite_path = base.with_extension("sqlite");
    let attempt_log_path = base.with_extension("attemptlog");
    for path in [&db_path, &jsonl_path, &sqlite_path, &attempt_log_path] {
        let _ = std::fs::remove_file(path);
    }

    let state = AppState::new(test_config());
    let app = router(state);
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0")
        .await
        .expect("bind");
    let addr = listener.local_addr().expect("addr");
    let server = tokio::spawn(async move {
        axum::serve(listener, app).await.expect("server");
    });

    let status = test_http_client()
        .get(format!("http://{addr}/api/archive/write/audit/status"))
        .send()
        .await
        .expect("audit status response");
    assert_eq!(status.status(), reqwest::StatusCode::OK);
    let status_payload: serde_json::Value = status.json().await.expect("audit status json");
    assert_eq!(status_payload["auditMode"], "preview_only");
    assert_eq!(status_payload["attemptLogPersistenceEnabled"], false);
    assert_eq!(status_payload["attemptLogFileWriteEnabled"], false);
    assert_eq!(status_payload["archiveWriteEnabled"], false);
    assert_eq!(status_payload["durableStorageEnabled"], false);
    assert_eq!(status_payload["databaseWriteEnabled"], false);
    assert_eq!(status_payload["jsonlWriteEnabled"], false);
    assert_eq!(status_payload["sqliteWriteEnabled"], false);
    assert_eq!(status_payload["fileArchiveWriteEnabled"], false);
    assert_eq!(status_payload["recentAttemptCount"], 0);
    assert_eq!(status_payload["latestAttemptAvailable"], false);

    let recent = test_http_client()
        .get(format!("http://{addr}/api/archive/write/audit/recent"))
        .send()
        .await
        .expect("audit recent response");
    assert_eq!(recent.status(), reqwest::StatusCode::OK);
    let recent_payload: serde_json::Value = recent.json().await.expect("audit recent json");
    assert_eq!(recent_payload["auditMode"], "preview_only");
    assert_eq!(recent_payload["attemptLogPersistenceEnabled"], false);
    assert_eq!(recent_payload["attempts"], serde_json::json!([]));
    assert_eq!(recent_payload["latestAttemptAvailable"], false);
    assert_eq!(
        recent_payload["operatorNote"],
        "No rejected archive write attempts are currently available in preview memory."
    );

    let latest = test_http_client()
        .get(format!("http://{addr}/api/archive/write/audit/latest"))
        .send()
        .await
        .expect("audit latest response");
    assert_eq!(latest.status(), reqwest::StatusCode::OK);
    let latest_payload: serde_json::Value = latest.json().await.expect("audit latest json");
    assert_eq!(latest_payload["auditMode"], "preview_only");
    assert_eq!(latest_payload["attemptLogPersistenceEnabled"], false);
    assert_eq!(latest_payload["latestAttemptAvailable"], false);
    assert_eq!(latest_payload["attempt"], serde_json::Value::Null);
    assert_eq!(
        latest_payload["operatorNote"],
        "No rejected archive write attempts are currently available in preview memory."
    );
    assert_eq!(latest_payload["notificationSent"], false);
    assert_eq!(latest_payload["executionTriggered"], false);

    assert!(!db_path.exists());
    assert!(!jsonl_path.exists());
    assert!(!sqlite_path.exists());
    assert!(!attempt_log_path.exists());

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
