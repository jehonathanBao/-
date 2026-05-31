mod support;
use support::test_http_client;
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
async fn runtime_start_endpoint_ensures_monitoring_is_started() {
    let state = AppState::new(test_config());
    let app = router(state);
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0")
        .await
        .expect("bind");
    let addr = listener.local_addr().expect("addr");
    let server = tokio::spawn(async move {
        axum::serve(listener, app).await.expect("server");
    });

    let client = test_http_client();
    let start = client
        .post(format!("http://{addr}/api/runtime/start"))
        .send()
        .await
        .expect("start response");
    assert_eq!(start.status(), reqwest::StatusCode::OK);
    let payload: serde_json::Value = start.json().await.expect("start json");
    assert_eq!(payload["ok"], true);
    assert_eq!(payload["readOnly"], false);
    assert_eq!(payload["runtimeModified"], true);
    assert_eq!(payload["analysisOnly"], true);
    assert_eq!(payload["monitoringStarted"], true);
    assert_eq!(payload["result"], "started");
    assert_eq!(payload["startState"], "started");
    assert!(payload["message"]
        .as_str()
        .unwrap_or_default()
        .contains("started"));
    assert!(payload["safetyBoundary"]
        .to_string()
        .contains("No order placement"));
    assert!(payload["safetyBoundary"]
        .to_string()
        .contains("No wallet/signing"));
    assert!(payload["safetyBoundary"]
        .to_string()
        .contains("No live trading"));

    let second_start = client
        .post(format!("http://{addr}/api/runtime/start"))
        .send()
        .await
        .expect("second start response");
    assert_eq!(second_start.status(), reqwest::StatusCode::OK);
    let second_payload: serde_json::Value = second_start.json().await.expect("second start json");
    assert_eq!(second_payload["ok"], true);
    assert_eq!(second_payload["runtimeModified"], false);
    assert_eq!(second_payload["result"], "already_started");
    assert_eq!(second_payload["startState"], "started");
    assert!(second_payload["message"]
        .as_str()
        .unwrap_or_default()
        .contains("already started"));

    let status = test_http_get(format!("http://{addr}/api/status"))
        .await
        .expect("status response");
    assert_eq!(status.status(), reqwest::StatusCode::OK);
    let status_payload: serde_json::Value = status.json().await.expect("status json");
    assert_eq!(status_payload["runtimeControl"]["monitoringStarted"], true);
    assert_eq!(
        status_payload["runtimeControl"]["oneClickStartEnabled"],
        true
    );
    assert_eq!(status_payload["runtimeControl"]["startState"], "started");
    assert_eq!(
        status_payload["runtimeControl"]["lastStartResult"],
        "already_started"
    );
    assert_eq!(status_payload["runtimeControl"]["startAttemptCount"], 2);
    assert!(status_payload["runtimeControl"]["lastStartAtMs"].is_number());
    assert!(status_payload["runtimeControl"]["lastStartError"].is_null());
    assert_eq!(
        status_payload["runtimeControl"]["startActionMode"],
        "monitoring_only"
    );
    assert_eq!(
        status_payload["venues"]["binance"]["disabledReason"],
        "env_or_config_flag_false"
    );
    assert_eq!(status_payload["venues"]["binance"]["startAttempted"], false);

    server.abort();
}

#[tokio::test]
async fn runtime_stop_endpoint_stops_monitoring_and_is_idempotent() {
    let state = AppState::new(test_config());
    let app = router(state);
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0")
        .await
        .expect("bind");
    let addr = listener.local_addr().expect("addr");
    let server = tokio::spawn(async move {
        axum::serve(listener, app).await.expect("server");
    });

    let client = test_http_client();
    let start = client
        .post(format!("http://{addr}/api/runtime/start"))
        .send()
        .await
        .expect("start response");
    assert_eq!(start.status(), reqwest::StatusCode::OK);

    let stop = client
        .post(format!("http://{addr}/api/runtime/stop"))
        .send()
        .await
        .expect("stop response");
    assert_eq!(stop.status(), reqwest::StatusCode::OK);
    let payload: serde_json::Value = stop.json().await.expect("stop json");
    assert_eq!(payload["ok"], true);
    assert_eq!(payload["readOnly"], false);
    assert_eq!(payload["runtimeModified"], true);
    assert_eq!(payload["analysisOnly"], true);
    assert_eq!(payload["monitoringStarted"], false);
    assert_eq!(payload["result"], "stopped");
    assert_eq!(payload["stopState"], "stopped");
    assert!(payload["message"]
        .as_str()
        .unwrap_or_default()
        .contains("stopped"));

    let second_stop = client
        .post(format!("http://{addr}/api/runtime/stop"))
        .send()
        .await
        .expect("second stop response");
    assert_eq!(second_stop.status(), reqwest::StatusCode::OK);
    let second_payload: serde_json::Value = second_stop.json().await.expect("second stop json");
    assert_eq!(second_payload["ok"], true);
    assert_eq!(second_payload["runtimeModified"], false);
    assert_eq!(second_payload["result"], "already_stopped");
    assert_eq!(second_payload["stopState"], "stopped");
    assert!(second_payload["message"]
        .as_str()
        .unwrap_or_default()
        .contains("already stopped"));

    let status = test_http_get(format!("http://{addr}/api/status"))
        .await
        .expect("status response");
    assert_eq!(status.status(), reqwest::StatusCode::OK);
    let status_payload: serde_json::Value = status.json().await.expect("status json");
    assert_eq!(status_payload["runtimeControl"]["monitoringStarted"], false);
    assert_eq!(status_payload["runtimeControl"]["startState"], "stopped");
    assert_eq!(status_payload["runtimeControl"]["stopState"], "stopped");
    assert_eq!(
        status_payload["runtimeControl"]["lastStopResult"],
        "already_stopped"
    );
    assert_eq!(status_payload["runtimeControl"]["stopAttemptCount"], 2);
    assert!(status_payload["runtimeControl"]["lastStopAtMs"].is_number());
    assert!(status_payload["runtimeControl"]["lastStopError"].is_null());

    server.abort();
}

#[tokio::test]
async fn runtime_start_endpoint_records_failed_start_attempts() {
    let state = AppState::new(test_config());
    state.set_start_failure_for_tests(Some("test startup failure".to_string()));
    let app = router(state);
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0")
        .await
        .expect("bind");
    let addr = listener.local_addr().expect("addr");
    let server = tokio::spawn(async move {
        axum::serve(listener, app).await.expect("server");
    });

    let client = test_http_client();
    let start = client
        .post(format!("http://{addr}/api/runtime/start"))
        .send()
        .await
        .expect("failed start response");
    assert_eq!(start.status(), reqwest::StatusCode::OK);
    let payload: serde_json::Value = start.json().await.expect("failed start json");
    assert_eq!(payload["ok"], false);
    assert_eq!(payload["readOnly"], false);
    assert_eq!(payload["runtimeModified"], false);
    assert_eq!(payload["result"], "failed");
    assert_eq!(payload["startState"], "failed");
    assert_eq!(payload["error"], "test startup failure");

    let status = test_http_get(format!("http://{addr}/api/status"))
        .await
        .expect("status response");
    assert_eq!(status.status(), reqwest::StatusCode::OK);
    let status_payload: serde_json::Value = status.json().await.expect("status json");
    assert_eq!(status_payload["runtimeControl"]["monitoringStarted"], false);
    assert_eq!(status_payload["runtimeControl"]["startState"], "failed");
    assert_eq!(
        status_payload["runtimeControl"]["lastStartResult"],
        "failed"
    );
    assert_eq!(status_payload["runtimeControl"]["startAttemptCount"], 1);
    assert_eq!(
        status_payload["runtimeControl"]["lastStartError"],
        "test startup failure"
    );

    server.abort();
}

#[tokio::test]
async fn runtime_stop_endpoint_records_failed_stop_attempts() {
    let state = AppState::new(test_config());
    state.start().await;
    state.set_stop_failure_for_tests(Some("test stop failure".to_string()));
    let app = router(state);
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0")
        .await
        .expect("bind");
    let addr = listener.local_addr().expect("addr");
    let server = tokio::spawn(async move {
        axum::serve(listener, app).await.expect("server");
    });

    let client = test_http_client();
    let stop = client
        .post(format!("http://{addr}/api/runtime/stop"))
        .send()
        .await
        .expect("failed stop response");
    assert_eq!(stop.status(), reqwest::StatusCode::OK);
    let payload: serde_json::Value = stop.json().await.expect("failed stop json");
    assert_eq!(payload["ok"], false);
    assert_eq!(payload["readOnly"], false);
    assert_eq!(payload["runtimeModified"], false);
    assert_eq!(payload["result"], "failed");
    assert_eq!(payload["stopState"], "failed");
    assert_eq!(payload["error"], "test stop failure");

    let status = test_http_get(format!("http://{addr}/api/status"))
        .await
        .expect("status response");
    assert_eq!(status.status(), reqwest::StatusCode::OK);
    let status_payload: serde_json::Value = status.json().await.expect("status json");
    assert_eq!(status_payload["runtimeControl"]["monitoringStarted"], true);
    assert_eq!(status_payload["runtimeControl"]["stopState"], "failed");
    assert_eq!(status_payload["runtimeControl"]["lastStopResult"], "failed");
    assert_eq!(status_payload["runtimeControl"]["stopAttemptCount"], 1);
    assert_eq!(
        status_payload["runtimeControl"]["lastStopError"],
        "test stop failure"
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
    }
}
