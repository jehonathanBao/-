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
async fn durable_archive_write_status_api_reports_disabled_gate() {
    let state = AppState::new(test_config());
    let app = router(state);
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0")
        .await
        .expect("bind");
    let addr = listener.local_addr().expect("addr");
    let server = tokio::spawn(async move {
        axum::serve(listener, app).await.expect("server");
    });

    let response = test_http_client()
        .get(format!("http://{addr}/api/archive/write/status"))
        .send()
        .await
        .expect("write status response");
    assert_eq!(response.status(), reqwest::StatusCode::OK);
    let payload: serde_json::Value = response.json().await.expect("write status json");

    assert_eq!(payload["readOnly"], true);
    assert_eq!(payload["analysisOnly"], true);
    assert_eq!(payload["manualReviewRequired"], true);
    assert_eq!(payload["archiveWriteEnabled"], false);
    assert_eq!(payload["durableStorageEnabled"], false);
    assert_eq!(payload["databaseWriteEnabled"], false);
    assert_eq!(payload["jsonlWriteEnabled"], false);
    assert_eq!(payload["sqliteWriteEnabled"], false);
    assert_eq!(payload["fileArchiveWriteEnabled"], false);
    assert_eq!(payload["executionEnabled"], false);
    assert_eq!(payload["runtimeModified"], false);
    assert_eq!(payload["notificationSent"], false);
    assert_eq!(payload["executionTriggered"], false);
    assert_eq!(payload["dryRunContractPreserved"], true);
    assert_eq!(payload["reviewPackContractPreserved"], true);
    assert_eq!(payload["recordsWritten"], 0);
    assert_eq!(payload["bytesWritten"], 0);

    server.abort();
}

#[tokio::test]
async fn durable_archive_write_api_rejects_post_without_writing_or_runtime_mutation() {
    let base = std::env::temp_dir().join("btc-toxic-flow-s16-api-write-gate");
    let db_path = base.with_extension("db");
    let jsonl_path = base.with_extension("jsonl");
    let sqlite_path = base.with_extension("sqlite");
    let archive_path = base.with_extension("archive");
    for path in [&db_path, &jsonl_path, &sqlite_path, &archive_path] {
        let _ = std::fs::remove_file(path);
    }

    let state = AppState::new(test_config());
    let state_probe = state.clone();
    let app = router(state);
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0")
        .await
        .expect("bind");
    let addr = listener.local_addr().expect("addr");
    let server = tokio::spawn(async move {
        axum::serve(listener, app).await.expect("server");
    });

    let response = test_http_client()
        .post(format!("http://{addr}/api/archive/write"))
        .json(&serde_json::json!({
            "requestedBy": "operator_review",
            "dryRunId": "dryrun-btcusdt-1",
            "requestedRecords": 3,
            "writeIntent": base.display().to_string()
        }))
        .send()
        .await
        .expect("write response");
    assert_eq!(response.status(), reqwest::StatusCode::OK);
    let payload: serde_json::Value = response.json().await.expect("write json");

    assert_eq!(payload["ok"], false);
    assert_eq!(payload["writeAccepted"], false);
    assert_eq!(payload["writeRejected"], true);
    assert_eq!(
        payload["rejectionReason"],
        "archive_write_disabled_by_default"
    );
    assert_eq!(payload["recordsWritten"], 0);
    assert_eq!(payload["bytesWritten"], 0);
    assert_eq!(payload["archiveWriteEnabled"], false);
    assert_eq!(payload["databaseWriteEnabled"], false);
    assert_eq!(payload["jsonlWriteEnabled"], false);
    assert_eq!(payload["sqliteWriteEnabled"], false);
    assert_eq!(payload["fileArchiveWriteEnabled"], false);
    assert_eq!(payload["runtimeModified"], false);
    assert_eq!(payload["notificationSent"], false);
    assert_eq!(payload["executionTriggered"], false);
    assert!(!state_probe.runtime_started());
    assert!(!db_path.exists());
    assert!(!jsonl_path.exists());
    assert!(!sqlite_path.exists());
    assert!(!archive_path.exists());

    server.abort();
}

#[tokio::test]
async fn durable_archive_write_gate_preserves_s15_dryrun_and_review_endpoints() {
    let state = AppState::new(test_config());
    let app = router(state);
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0")
        .await
        .expect("bind");
    let addr = listener.local_addr().expect("addr");
    let server = tokio::spawn(async move {
        axum::serve(listener, app).await.expect("server");
    });

    let dry_run = test_http_client()
        .post(format!("http://{addr}/api/archive/dry-run/write"))
        .send()
        .await
        .expect("dry-run response");
    assert_eq!(dry_run.status(), reqwest::StatusCode::OK);
    let dry_run_payload: serde_json::Value = dry_run.json().await.expect("dry-run json");
    assert_eq!(dry_run_payload["action"], "dry_run_write");
    assert_eq!(dry_run_payload["archiveWriteEnabled"], false);

    let validation = test_http_client()
        .post(format!("http://{addr}/api/archive/dry-run/write"))
        .json(&serde_json::json!({
            "records": [{
                "symbol": "BTCUSDT",
                "privateKey": "never-archive",
                "placeOrder": true,
                "notificationSent": true
            }]
        }))
        .send()
        .await
        .expect("dry-run validation response");
    assert_eq!(validation.status(), reqwest::StatusCode::OK);
    let validation_payload: serde_json::Value =
        validation.json().await.expect("dry-run validation json");
    assert_eq!(validation_payload["validation"]["valid"], false);
    assert_eq!(validation_payload["notificationSent"], false);
    assert_eq!(validation_payload["executionTriggered"], false);

    let review_pack = test_http_client()
        .get(format!(
            "http://{addr}/api/archive/dry-run/review-pack/latest"
        ))
        .send()
        .await
        .expect("review pack response");
    assert_eq!(review_pack.status(), reqwest::StatusCode::OK);
    let review_payload: serde_json::Value = review_pack.json().await.expect("review pack json");
    assert_eq!(
        review_payload["reviewPackType"],
        "durable_archive_dryrun_review_pack"
    );
    assert_eq!(review_payload["archiveWriteEnabled"], false);

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
