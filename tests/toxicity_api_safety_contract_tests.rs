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
async fn t6_to_t12_toxicity_apis_expose_consistent_safety_contract() {
    let state = AppState::new(test_config());
    let app = router(state);
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0")
        .await
        .expect("bind");
    let addr = listener.local_addr().expect("addr");
    let server = tokio::spawn(async move {
        axum::serve(listener, app).await.expect("server");
    });

    let core_endpoints = [
        "/api/toxicity/fusion/status",
        "/api/toxicity/fusion/recent",
        "/api/toxicity/fusion/BTC-PERP",
        "/api/toxicity/replay/status",
        "/api/toxicity/replay/recent",
        "/api/toxicity/replay/BTC-PERP",
        "/api/toxicity/replay/BTC-PERP/latest",
        "/api/toxicity/replay/BTC-PERP/missing-signal",
        "/api/toxicity/markout/status",
        "/api/toxicity/markout/recent",
        "/api/toxicity/markout/BTC-PERP",
        "/api/toxicity/markout/signal/missing-signal",
        "/api/toxicity/quality-scorecard/status",
        "/api/toxicity/quality-scorecard/summary",
        "/api/toxicity/quality-scorecard/BTC-PERP",
    ];

    for endpoint in core_endpoints {
        let body = get_json(addr, endpoint).await;
        assert_core_safety_contract(endpoint, &body);
        assert_eq!(body["mode"], "analysis_only", "{endpoint} mode");
    }

    let manual_review_endpoints = [
        "/api/toxicity/weight-recommendation/status",
        "/api/toxicity/weight-recommendation/summary",
        "/api/toxicity/weight-recommendation/BTC-PERP",
        "/api/toxicity/weight-review/status",
        "/api/toxicity/weight-review/summary",
        "/api/toxicity/weight-review/latest",
        "/api/toxicity/weight-review/export",
        "/api/toxicity/weight-review/BTC-PERP",
        "/api/toxicity/weight-review/BTC-PERP/export",
        "/api/toxicity/governance-ledger/status",
        "/api/toxicity/governance-ledger/summary",
        "/api/toxicity/governance-ledger/recent",
        "/api/toxicity/governance-ledger/export",
        "/api/toxicity/governance-ledger/BTC-PERP",
    ];

    for endpoint in manual_review_endpoints {
        let body = get_json(addr, endpoint).await;
        assert_core_safety_contract(endpoint, &body);
        assert_manual_review_contract(endpoint, &body);
        assert!(
            body["mode"].as_str().is_some_and(|mode| !mode.is_empty()),
            "{endpoint} mode should be preserved"
        );
    }

    server.abort();
}

async fn get_json(addr: std::net::SocketAddr, endpoint: &str) -> serde_json::Value {
    let response = test_http_get(format!("http://{addr}{endpoint}"))
        .await
        .unwrap_or_else(|err| panic!("{endpoint} response error: {err}"));
    assert_eq!(
        response.status(),
        reqwest::StatusCode::OK,
        "{endpoint} should return 200"
    );
    response
        .json::<serde_json::Value>()
        .await
        .unwrap_or_else(|err| panic!("{endpoint} json error: {err}"))
}

fn assert_core_safety_contract(endpoint: &str, body: &serde_json::Value) {
    assert_eq!(body["readOnly"], true, "{endpoint} readOnly");
    assert_eq!(body["runtimeModified"], false, "{endpoint} runtimeModified");
    assert_eq!(body["analysisOnly"], true, "{endpoint} analysisOnly");
    assert_eq!(
        body["executionEnabled"], false,
        "{endpoint} executionEnabled"
    );
}

fn assert_manual_review_contract(endpoint: &str, body: &serde_json::Value) {
    assert_eq!(
        body["manualReviewRequired"], true,
        "{endpoint} manualReviewRequired"
    );
    assert_eq!(
        body["runtimeWeightModified"], false,
        "{endpoint} runtimeWeightModified"
    );
    assert_eq!(body["configModified"], false, "{endpoint} configModified");
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
