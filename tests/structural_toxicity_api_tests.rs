mod support;
use support::test_http_get;

use btc_toxic_flow_monitor_rs::{
    api::server::router,
    app::AppState,
    config::{
        venues::{VenueConfig, VenueConfigs},
        AppConfig,
    },
    types::{
        market::{NormalizedBook, Venue},
        toxic::ToxicSeverity,
    },
};

#[tokio::test]
async fn structural_toxicity_api_returns_analysis_only_payloads() {
    let state = AppState::new(test_config());
    state
        .orderbook_wall_lifecycle_service_for_tests()
        .on_book_for_tests(&book_snapshot(
            1_000,
            100_000.0,
            vec![(99_950.0, 0.7)],
            vec![(100_050.0, 4.5)],
        ));

    let app = router(state);
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0")
        .await
        .expect("bind");
    let addr = listener.local_addr().expect("addr");
    let server = tokio::spawn(async move {
        axum::serve(listener, app).await.expect("server");
    });

    let status = test_http_get(format!("http://{addr}/api/toxicity/structural/status"))
        .await
        .expect("status response");
    assert_eq!(status.status(), reqwest::StatusCode::OK);
    let status_payload: serde_json::Value = status.json().await.expect("status json");
    assert_eq!(status_payload["readOnly"], true);
    assert_eq!(status_payload["runtimeModified"], false);
    assert_eq!(status_payload["mode"], "analysis_only");

    let recent = test_http_get(format!("http://{addr}/api/toxicity/structural/recent"))
        .await
        .expect("recent response");
    assert_eq!(recent.status(), reqwest::StatusCode::OK);
    let recent_payload: serde_json::Value = recent.json().await.expect("recent json");
    assert_eq!(recent_payload["readOnly"], true);
    assert_eq!(recent_payload["runtimeModified"], false);
    assert!(recent_payload["signals"].is_array());
    assert!(recent_payload["signals"]
        .as_array()
        .expect("signals array")
        .iter()
        .all(|signal| signal["readOnly"] == true));

    let by_symbol = test_http_get(format!(
        "http://{addr}/api/toxicity/structural/{}",
        "BTC-PERP"
    ))
    .await
    .expect("symbol response");
    assert_eq!(by_symbol.status(), reqwest::StatusCode::OK);
    let by_symbol_payload: serde_json::Value = by_symbol.json().await.expect("symbol json");
    assert_eq!(by_symbol_payload["selectedSymbol"], "BTC-PERP");

    server.abort();
}

fn book_snapshot(
    ts: i64,
    mid: f64,
    bids: Vec<(f64, f64)>,
    asks: Vec<(f64, f64)>,
) -> NormalizedBook {
    let best_bid = bids.first().map(|(price, _)| *price).unwrap_or(mid - 1.0);
    let best_ask = asks.first().map(|(price, _)| *price).unwrap_or(mid + 1.0);
    NormalizedBook {
        venue: Venue::Binance,
        symbol: "BTC-PERP".to_string(),
        ts,
        best_bid,
        best_ask,
        bids,
        asks,
        mid,
        spread_bps: 1.0,
        bid_depth_btc_10bps: 0.0,
        ask_depth_btc_10bps: 0.0,
        bid_depth_usd_10bps: 0.0,
        ask_depth_usd_10bps: 0.0,
        imbalance_10bps: 0.0,
    }
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
