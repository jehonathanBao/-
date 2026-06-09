mod support;
use support::{test_http_client, test_http_get};

use btc_toxic_flow_monitor_rs::{
    api::{
        discord_notification_routes::{maybe_auto_push_discord, DiscordNotificationRequest},
        server::router,
    },
    app::AppState,
    config::{
        venues::{VenueConfig, VenueConfigs},
        AppConfig,
    },
    types::{market::Venue, toxic::ToxicSeverity},
};

static ENV_LOCK: tokio::sync::Mutex<()> = tokio::sync::Mutex::const_new(());

#[tokio::test]
async fn default_loopback_get_and_post_are_allowed() {
    let _guard = ENV_LOCK.lock().await;
    clear_operator_env();
    let (addr, server) = spawn_app(test_config("127.0.0.1")).await;

    let dashboard = test_http_get(format!("http://{addr}/dashboard"))
        .await
        .expect("dashboard response");
    assert_eq!(dashboard.status(), reqwest::StatusCode::OK);

    let start = test_http_client()
        .post(format!("http://{addr}/api/runtime/start"))
        .send()
        .await
        .expect("start response");
    assert_eq!(start.status(), reqwest::StatusCode::OK);

    server.abort();
    clear_operator_env();
}

#[tokio::test]
async fn untrusted_origin_post_is_rejected() {
    let _guard = ENV_LOCK.lock().await;
    clear_operator_env();
    let (addr, server) = spawn_app(test_config("127.0.0.1")).await;

    let start = test_http_client()
        .post(format!("http://{addr}/api/runtime/start"))
        .header(reqwest::header::ORIGIN, "http://evil.example")
        .send()
        .await
        .expect("start response");
    assert_eq!(start.status(), reqwest::StatusCode::FORBIDDEN);
    let payload: serde_json::Value = start.json().await.expect("guard json");
    assert_eq!(payload["reason"], "operator_api_guard_rejected");

    server.abort();
    clear_operator_env();
}

#[tokio::test]
async fn lan_bound_post_without_token_is_rejected() {
    let _guard = ENV_LOCK.lock().await;
    clear_operator_env();
    let (addr, server) = spawn_app(test_config("0.0.0.0")).await;

    let start = test_http_client()
        .post(format!("http://{addr}/api/runtime/start"))
        .send()
        .await
        .expect("start response");
    assert_eq!(start.status(), reqwest::StatusCode::FORBIDDEN);

    server.abort();
    clear_operator_env();
}

#[tokio::test]
async fn lan_bound_post_requires_configured_operator_token() {
    let _guard = ENV_LOCK.lock().await;
    clear_operator_env();
    std::env::set_var("ALLOW_LAN_DASHBOARD", "true");
    std::env::set_var("OPERATOR_API_TOKEN", "test-token");
    std::env::set_var("ALLOWED_DASHBOARD_ORIGIN", "http://dashboard.local");
    let (addr, server) = spawn_app(test_config("0.0.0.0")).await;

    let rejected = test_http_client()
        .post(format!("http://{addr}/api/runtime/start"))
        .header(reqwest::header::ORIGIN, "http://dashboard.local")
        .send()
        .await
        .expect("missing token response");
    assert_eq!(rejected.status(), reqwest::StatusCode::FORBIDDEN);

    let allowed = test_http_client()
        .post(format!("http://{addr}/api/runtime/start"))
        .header(reqwest::header::ORIGIN, "http://dashboard.local")
        .header("x-operator-api-token", "test-token")
        .send()
        .await
        .expect("token response");
    assert_eq!(allowed.status(), reqwest::StatusCode::OK);

    server.abort();
    clear_operator_env();
}

#[tokio::test]
async fn lan_bound_post_accepts_operator_token_without_lan_dashboard_flag() {
    let _guard = ENV_LOCK.lock().await;
    clear_operator_env();
    std::env::set_var("OPERATOR_TOKEN", "test-token");
    let (addr, server) = spawn_app(test_config("0.0.0.0")).await;

    let allowed = test_http_client()
        .post(format!("http://{addr}/api/runtime/start"))
        .header("x-operator-token", "test-token")
        .send()
        .await
        .expect("token response");
    assert_eq!(allowed.status(), reqwest::StatusCode::OK);

    server.abort();
    clear_operator_env();
}

#[tokio::test]
async fn lan_bound_sensitive_get_without_token_is_rejected() {
    let _guard = ENV_LOCK.lock().await;
    clear_operator_env();
    std::env::set_var("OPERATOR_TOKEN", "test-token");
    let (addr, server) = spawn_app(test_config("0.0.0.0")).await;

    let dashboard = test_http_get(format!("http://{addr}/dashboard"))
        .await
        .expect("dashboard response");
    assert_eq!(dashboard.status(), reqwest::StatusCode::OK);

    let rejected = test_http_client()
        .get(format!("http://{addr}/api/status"))
        .send()
        .await
        .expect("status response");
    assert_eq!(rejected.status(), reqwest::StatusCode::UNAUTHORIZED);
    let payload: serde_json::Value = rejected.json().await.expect("guard json");
    assert_eq!(payload["reason"], "operator_token_required");

    server.abort();
    clear_operator_env();
}

#[tokio::test]
async fn lan_bound_scan_log_get_without_token_is_rejected() {
    let _guard = ENV_LOCK.lock().await;
    clear_operator_env();
    std::env::set_var("OPERATOR_TOKEN", "test-token");
    let (addr, server) = spawn_app(test_config("0.0.0.0")).await;

    let rejected = test_http_client()
        .get(format!("http://{addr}/api/runtime/scan-log/recent"))
        .send()
        .await
        .expect("scan log guard response");
    assert_eq!(rejected.status(), reqwest::StatusCode::UNAUTHORIZED);
    let payload: serde_json::Value = rejected.json().await.expect("guard json");
    assert_eq!(payload["reason"], "operator_token_required");

    server.abort();
    clear_operator_env();
}

#[tokio::test]
async fn lan_bound_sensitive_get_accepts_operator_token_header() {
    let _guard = ENV_LOCK.lock().await;
    clear_operator_env();
    std::env::set_var("OPERATOR_TOKEN", "test-token");
    let (addr, server) = spawn_app(test_config("0.0.0.0")).await;

    let allowed = test_http_client()
        .get(format!("http://{addr}/api/status"))
        .header("x-operator-token", "test-token")
        .send()
        .await
        .expect("status response");
    assert_eq!(allowed.status(), reqwest::StatusCode::OK);

    server.abort();
    clear_operator_env();
}

#[tokio::test]
async fn loopback_scan_log_recent_is_read_only_and_redacted() {
    let _guard = ENV_LOCK.lock().await;
    clear_operator_env();
    std::env::set_var("OPERATOR_TOKEN", "secret-test-token");
    std::env::set_var(
        "DISCORD_WEBHOOK_URL",
        "https://discord.com/api/webhooks/test-id/test-token",
    );
    let (addr, server) = spawn_app(test_config("127.0.0.1")).await;

    let response = test_http_client()
        .get(format!("http://{addr}/api/runtime/scan-log/recent"))
        .send()
        .await
        .expect("scan log response");
    assert_eq!(response.status(), reqwest::StatusCode::OK);
    let payload: serde_json::Value = response.json().await.expect("scan log json");
    assert_eq!(payload["readOnly"], true);
    assert_eq!(payload["runtimeModified"], false);
    let text = payload.to_string();
    assert!(!text.contains("secret-test-token"));
    assert!(!text.contains("discord.com/api/webhooks"));
    assert!(!text.contains("authorization"));
    assert!(!text.contains("rawPayload"));
    assert!(!text.contains("markout"));
    assert!(!text.contains("evidence"));

    server.abort();
    clear_operator_env();
    std::env::remove_var("DISCORD_WEBHOOK_URL");
}

#[tokio::test]
async fn auto_push_scan_logs_gate_reason_and_redacts_sensitive_fields() {
    let _guard = ENV_LOCK.lock().await;
    clear_operator_env();
    std::env::set_var("OPERATOR_TOKEN", "secret-test-token");
    std::env::set_var("DRY_RUN", "true");
    std::env::set_var("DISCORD_AUTO_PUSH_ENABLED", "true");
    std::env::set_var(
        "DISCORD_WEBHOOK_URL",
        "https://discord.com/api/webhooks/test-id/test-token",
    );
    let state = AppState::new(test_config("127.0.0.1"));
    let created_at_ms = (state.booted_at_ms() + 1) as u64;

    let decision = maybe_auto_push_discord(&state, discord_request(), created_at_ms).await;
    assert_eq!(decision.reason, "dry_run");

    let logs = state.recent_scan_logs(20);
    assert!(logs.iter().any(|item| item.kind == "alert_gate_evaluated"));
    assert!(logs
        .iter()
        .any(|item| item.kind == "discord_auto_push_skipped"
            && item.message.to_ascii_lowercase().contains("dry run")));

    let text = serde_json::to_string(&logs).expect("scan logs json");
    for forbidden in [
        "secret-test-token",
        "discord.com/api/webhooks",
        "rawPayload",
        "evidence",
        "markout",
        "webhook",
        "token",
    ] {
        assert!(
            !text.contains(forbidden),
            "forbidden scan log content leaked: {forbidden}"
        );
    }

    clear_operator_env();
    std::env::remove_var("DRY_RUN");
    std::env::remove_var("DISCORD_AUTO_PUSH_ENABLED");
    std::env::remove_var("DISCORD_WEBHOOK_URL");
}

#[tokio::test]
async fn lan_bound_websocket_get_without_token_is_rejected() {
    let _guard = ENV_LOCK.lock().await;
    clear_operator_env();
    std::env::set_var("OPERATOR_TOKEN", "test-token");
    let (addr, server) = spawn_app(test_config("0.0.0.0")).await;

    let rejected = test_http_client()
        .get(format!("http://{addr}/ws/signals"))
        .send()
        .await
        .expect("ws guard response");
    assert_eq!(rejected.status(), reqwest::StatusCode::UNAUTHORIZED);
    let payload: serde_json::Value = rejected.json().await.expect("guard json");
    assert_eq!(payload["reason"], "operator_token_required");

    server.abort();
    clear_operator_env();
}

#[tokio::test]
async fn lan_bound_scan_log_websocket_get_without_token_is_rejected() {
    let _guard = ENV_LOCK.lock().await;
    clear_operator_env();
    std::env::set_var("OPERATOR_TOKEN", "test-token");
    let (addr, server) = spawn_app(test_config("0.0.0.0")).await;

    let rejected = test_http_client()
        .get(format!("http://{addr}/ws/scan-logs"))
        .send()
        .await
        .expect("scan log ws guard response");
    assert_eq!(rejected.status(), reqwest::StatusCode::UNAUTHORIZED);
    let payload: serde_json::Value = rejected.json().await.expect("guard json");
    assert_eq!(payload["reason"], "operator_token_required");

    server.abort();
    clear_operator_env();
}

#[tokio::test]
async fn lan_bound_websocket_rejects_invalid_operator_token() {
    let _guard = ENV_LOCK.lock().await;
    clear_operator_env();
    std::env::set_var("OPERATOR_TOKEN", "test-token");
    let (addr, server) = spawn_app(test_config("0.0.0.0")).await;

    let rejected = test_http_client()
        .get(format!("http://{addr}/ws/signals"))
        .header("x-operator-token", "wrong-token")
        .send()
        .await
        .expect("ws guard response");
    assert_eq!(rejected.status(), reqwest::StatusCode::UNAUTHORIZED);

    server.abort();
    clear_operator_env();
}

#[tokio::test]
async fn lan_bound_websocket_accepts_valid_operator_token_before_upgrade() {
    let _guard = ENV_LOCK.lock().await;
    clear_operator_env();
    std::env::set_var("OPERATOR_TOKEN", "test-token");
    let (addr, server) = spawn_app(test_config("0.0.0.0")).await;

    let response = test_http_client()
        .get(format!("http://{addr}/ws/signals"))
        .header("x-operator-token", "test-token")
        .send()
        .await
        .expect("ws response");
    assert_ne!(response.status(), reqwest::StatusCode::UNAUTHORIZED);
    assert_ne!(response.status(), reqwest::StatusCode::FORBIDDEN);

    server.abort();
    clear_operator_env();
}

#[tokio::test]
async fn healthz_and_readyz_return_operator_safe_status() {
    let _guard = ENV_LOCK.lock().await;
    clear_operator_env();
    let (addr, server) = spawn_app(test_config("127.0.0.1")).await;

    let health = test_http_get(format!("http://{addr}/healthz"))
        .await
        .expect("health response");
    assert_eq!(health.status(), reqwest::StatusCode::OK);
    let health_payload: serde_json::Value = health.json().await.expect("health json");
    assert_eq!(health_payload["ok"], true);
    assert_eq!(health_payload["runtimeModified"], false);

    let ready = test_http_get(format!("http://{addr}/readyz"))
        .await
        .expect("ready response");
    assert_eq!(ready.status(), reqwest::StatusCode::OK);
    let ready_payload: serde_json::Value = ready.json().await.expect("ready json");
    assert_eq!(ready_payload["ok"], true);
    assert_eq!(ready_payload["runtimeModified"], false);

    server.abort();
    clear_operator_env();
}

#[tokio::test]
async fn lan_bound_sensitive_get_without_configured_token_is_rejected() {
    let _guard = ENV_LOCK.lock().await;
    clear_operator_env();
    let (addr, server) = spawn_app(test_config("0.0.0.0")).await;

    let rejected = test_http_client()
        .get(format!("http://{addr}/api/status"))
        .send()
        .await
        .expect("status response");
    assert_eq!(rejected.status(), reqwest::StatusCode::FORBIDDEN);
    let payload: serde_json::Value = rejected.json().await.expect("guard json");
    assert_eq!(
        payload["reason"],
        "operator_token_missing_for_non_loopback_api"
    );

    server.abort();
    clear_operator_env();
}

#[tokio::test]
async fn cors_does_not_allow_every_origin() {
    let _guard = ENV_LOCK.lock().await;
    clear_operator_env();
    let (addr, server) = spawn_app(test_config("127.0.0.1")).await;

    let response = test_http_client()
        .get(format!("http://{addr}/api/status"))
        .header(reqwest::header::ORIGIN, "http://evil.example")
        .send()
        .await
        .expect("cors response");
    assert_ne!(
        response
            .headers()
            .get(reqwest::header::ACCESS_CONTROL_ALLOW_ORIGIN)
            .and_then(|value| value.to_str().ok()),
        Some("*")
    );

    server.abort();
    clear_operator_env();
}

async fn spawn_app(config: AppConfig) -> (std::net::SocketAddr, tokio::task::JoinHandle<()>) {
    let state = AppState::new(config);
    let app = router(state);
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0")
        .await
        .expect("bind");
    let addr = listener.local_addr().expect("addr");
    let server = tokio::spawn(async move {
        axum::serve(listener, app).await.expect("server");
    });
    (addr, server)
}

fn clear_operator_env() {
    for key in [
        "ALLOW_LAN_DASHBOARD",
        "OPERATOR_TOKEN",
        "OPERATOR_API_TOKEN",
        "ALLOWED_DASHBOARD_ORIGIN",
    ] {
        std::env::remove_var(key);
    }
}

fn discord_request() -> DiscordNotificationRequest {
    DiscordNotificationRequest {
        alert_family: Some("short_toxic_order".to_string()),
        signal_id: Some("scan-log-high".to_string()),
        id: Some("scan-log-high".to_string()),
        dedupe_key: Some("scan-log-high".to_string()),
        exchange: Some("Runtime".to_string()),
        symbol: Some("BTC-PERP".to_string()),
        signal_type: Some("spoofing_candidate".to_string()),
        level: Some("high".to_string()),
        side: Some("Ask/Sell".to_string()),
        score: Some(92),
        confidence: Some(88.0),
        data_quality: Some(88.0),
        reason: Some("safe final result only".to_string()),
        impact: Some("rawPayload evidence markout webhook token".to_string()),
        time: None,
        price_range: None,
        add_qty: None,
        cancel_qty: None,
        fill_qty: None,
        cancel_to_trade_ratio: None,
        depth_before: None,
        depth_after: None,
        depth_impact: None,
        price_impact_bps: None,
        markout_1s_bps: None,
        markout_5s_bps: None,
        markout_30s_bps: None,
        tof_metrics: None,
        tof_score: None,
        candidate_type: Some("spoofing_candidate".to_string()),
        explain_tags: Some(vec!["high_vpin_proxy".to_string()]),
        direction_confidence: Some(82.0),
        perp_tof_metrics: None,
        perp_score: None,
        perp_candidate_type: None,
        final_candidate_type: None,
        metrics_direction: None,
        advanced_tof_metrics: None,
        advanced_score: None,
        advanced_candidate_type: None,
        main_force_score: None,
        extreme_impact_score: None,
        structure_bias: None,
        market_structure_confidence: None,
        market_structure_data_quality: None,
        market_structure_severity: None,
        regime_type: None,
        spot_score: None,
        contract_score: None,
        cross_confirm_score: None,
        main_force_confirmed: None,
        signal_agreement: None,
        source_coverage: None,
        oi_score: None,
        liquidation_score: None,
        test: None,
    }
}

fn test_config(api_host: &str) -> AppConfig {
    AppConfig {
        app_env: "test".to_string(),
        read_only: true,
        api_host: api_host.parse().expect("valid ip"),
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
