#![allow(clippy::await_holding_lock)]

mod support;
use support::test_http_client;

use std::{
    fs,
    sync::{Mutex, OnceLock},
    time::{SystemTime, UNIX_EPOCH},
};

use btc_toxic_flow_monitor_rs::{
    api::server::router,
    app::AppState,
    config::{
        venues::{VenueConfig, VenueConfigs},
        AppConfig,
    },
    types::{market::Venue, toxic::ToxicSeverity},
};

fn env_lock() -> &'static Mutex<()> {
    static LOCK: OnceLock<Mutex<()>> = OnceLock::new();
    LOCK.get_or_init(|| Mutex::new(()))
}

#[tokio::test]
async fn dev_test_sidecar_endpoint_is_hidden_when_not_enabled() {
    let _guard = env_lock().lock().expect("env lock");
    let previous_enabled = std::env::var("ENABLE_DEV_TEST_ALERTS").ok();
    std::env::remove_var("ENABLE_DEV_TEST_ALERTS");

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
    let response = client
        .post(format!("http://{addr}/api/dev/alerts/test-sidecar"))
        .json(&serde_json::json!({
            "severity": "warning",
            "venue": "binance",
            "symbol": "BTCUSDT",
            "dedupe_suffix": "manual-001"
        }))
        .send()
        .await
        .expect("dev test alert response");
    assert_eq!(response.status(), reqwest::StatusCode::NOT_FOUND);
    let payload: serde_json::Value = response.json().await.expect("dev alert json");
    assert_eq!(payload["ok"], false);
    assert_eq!(payload["reason"], "dev_test_alerts_disabled");
    assert!(!payload.to_string().contains("discord"));
    assert!(!payload.to_string().contains("webhook"));

    server.abort();
    restore_env("ENABLE_DEV_TEST_ALERTS", previous_enabled);
}

#[tokio::test]
async fn dev_test_sidecar_endpoint_returns_clear_error_when_sidecar_is_disabled() {
    let _guard = env_lock().lock().expect("env lock");
    let previous_enabled = std::env::var("ENABLE_DEV_TEST_ALERTS").ok();
    let previous_sidecar_enabled = std::env::var("TOXIC_FLOW_SIDECAR_ENABLED").ok();
    let previous_events_path = std::env::var("TOXIC_FLOW_SIDECAR_EVENTS_PATH").ok();
    std::env::set_var("ENABLE_DEV_TEST_ALERTS", "true");
    std::env::remove_var("TOXIC_FLOW_SIDECAR_ENABLED");
    std::env::remove_var("TOXIC_FLOW_SIDECAR_EVENTS_PATH");

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
    let response = client
        .post(format!("http://{addr}/api/dev/alerts/test-sidecar"))
        .json(&serde_json::json!({
            "severity": "warning",
            "venue": "binance",
            "symbol": "BTCUSDT",
            "dedupe_suffix": "manual-001"
        }))
        .send()
        .await
        .expect("dev test alert response");
    assert_eq!(response.status(), reqwest::StatusCode::CONFLICT);
    let payload: serde_json::Value = response.json().await.expect("dev alert json");
    assert_eq!(payload["ok"], false);
    assert_eq!(payload["error"], "sidecar_disabled_or_path_missing");
    assert!(!payload.to_string().contains("discord"));
    assert!(!payload.to_string().contains("webhook"));

    server.abort();
    restore_env("ENABLE_DEV_TEST_ALERTS", previous_enabled);
    restore_env("TOXIC_FLOW_SIDECAR_ENABLED", previous_sidecar_enabled);
    restore_env("TOXIC_FLOW_SIDECAR_EVENTS_PATH", previous_events_path);
}

#[tokio::test]
async fn dev_test_sidecar_endpoint_writes_runtime_acceptance_event() {
    let _guard = env_lock().lock().expect("env lock");
    let previous_enabled = std::env::var("ENABLE_DEV_TEST_ALERTS").ok();
    let previous_sidecar_enabled = std::env::var("TOXIC_FLOW_SIDECAR_ENABLED").ok();
    let previous_events_path = std::env::var("TOXIC_FLOW_SIDECAR_EVENTS_PATH").ok();
    let events_path = temp_events_path("dev-test-alert-endpoint");
    if let Some(parent) = events_path.parent() {
        let _ = fs::create_dir_all(parent);
    }
    let _ = fs::remove_file(&events_path);
    std::env::set_var("ENABLE_DEV_TEST_ALERTS", "true");
    std::env::set_var("TOXIC_FLOW_SIDECAR_ENABLED", "true");
    std::env::set_var(
        "TOXIC_FLOW_SIDECAR_EVENTS_PATH",
        events_path.to_string_lossy().to_string(),
    );

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
    let response = client
        .post(format!("http://{addr}/api/dev/alerts/test-sidecar"))
        .json(&serde_json::json!({
            "severity": "warning",
            "venue": "binance",
            "symbol": "BTCUSDT",
            "dedupe_suffix": "manual-001"
        }))
        .send()
        .await
        .expect("dev test alert response");
    assert_eq!(response.status(), reqwest::StatusCode::OK);
    let payload: serde_json::Value = response.json().await.expect("dev alert json");
    assert_eq!(payload["ok"], true);
    assert_eq!(payload["kind"], "runtime_acceptance_test");
    assert_eq!(payload["sidecarWritten"], true);
    assert_eq!(payload["deduped"], false);
    assert_eq!(payload["telegramTriggered"], false);
    assert!(payload["dedupeKey"]
        .as_str()
        .unwrap_or_default()
        .ends_with("manual-001"));
    assert!(!payload.to_string().contains("discord"));
    assert!(!payload.to_string().contains("webhook"));

    let content = fs::read_to_string(&events_path).expect("sidecar events");
    let lines = content.lines().collect::<Vec<_>>();
    assert_eq!(lines.len(), 1);
    let event: serde_json::Value = serde_json::from_str(lines[0]).expect("sidecar json");
    assert_eq!(event["schemaVersion"], "toxic-flow-rs.sidecar.v1");
    assert_eq!(event["kind"], "runtime_acceptance_test");
    assert_eq!(event["payload"]["test"], true);
    assert_eq!(
        event["payload"]["generatedBy"],
        "monitor_dev_test_alert_endpoint"
    );
    assert!(!event.to_string().contains("discord"));
    assert!(!event.to_string().contains("webhook"));

    server.abort();
    let _ = fs::remove_file(&events_path);
    restore_env("ENABLE_DEV_TEST_ALERTS", previous_enabled);
    restore_env("TOXIC_FLOW_SIDECAR_ENABLED", previous_sidecar_enabled);
    restore_env("TOXIC_FLOW_SIDECAR_EVENTS_PATH", previous_events_path);
}

fn restore_env(key: &str, value: Option<String>) {
    if let Some(value) = value {
        std::env::set_var(key, value);
    } else {
        std::env::remove_var(key);
    }
}

fn temp_events_path(name: &str) -> std::path::PathBuf {
    let nonce = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("system time")
        .as_nanos();
    std::env::temp_dir()
        .join(format!("{name}-{nonce}"))
        .join("events.jsonl")
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
        book_stale_ms: 5_000,
        max_buffer_age_ms: 120_000,
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
