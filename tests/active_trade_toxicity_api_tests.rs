use std::collections::BTreeMap;
use std::time::Duration;

use btc_toxic_flow_monitor_rs::{
    api::server::router,
    app::AppState,
    config::{
        venues::{VenueConfig, VenueConfigs},
        AppConfig,
    },
    types::{
        flow::{empty_venue_breakdown, DataQuality, FlowState, FlowWindow},
        market::Venue,
        toxic::ToxicSeverity,
    },
};

#[tokio::test]
async fn active_trade_toxicity_api_returns_analysis_only_status_and_recent_signals() {
    let state = AppState::new(test_config());
    *state.shared_flow_for_tests().write() =
        sample_flow_state(900_000.0, 100_000.0, 8, 40.0, 10.0, Some(4.5));
    let client = reqwest::Client::builder()
        .no_proxy()
        .build()
        .expect("client");
    let app = router(state);
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0")
        .await
        .expect("bind");
    let addr = listener.local_addr().expect("addr");
    let server = tokio::spawn(async move {
        axum::serve(listener, app).await.expect("server");
    });
    tokio::time::sleep(Duration::from_millis(50)).await;
    assert!(!server.is_finished(), "server task exited before request");

    let status = client
        .get(format!("http://{addr}/api/toxicity/active-trade/status"))
        .send()
        .await
        .expect("status response");
    let status_code = status.status();
    let status_text = status.text().await.expect("status text");
    assert_eq!(status_code, reqwest::StatusCode::OK, "body={status_text}");
    let status_payload: serde_json::Value =
        serde_json::from_str(&status_text).expect("status json");
    assert_eq!(status_payload["readOnly"], true);
    assert_eq!(status_payload["runtimeModified"], false);
    assert_eq!(status_payload["mode"], "analysis_only");

    let recent = client
        .get(format!("http://{addr}/api/toxicity/active-trade/recent"))
        .send()
        .await
        .expect("recent response");
    assert_eq!(recent.status(), reqwest::StatusCode::OK);
    let recent_payload: serde_json::Value = recent.json().await.expect("recent json");
    assert_eq!(recent_payload["readOnly"], true);
    assert_eq!(recent_payload["runtimeModified"], false);
    assert!(recent_payload["signals"].is_array());
    assert!(recent_payload["signals"]
        .as_array()
        .is_some_and(|items| !items.is_empty()));

    server.abort();
}

#[tokio::test]
async fn active_trade_toxicity_api_reports_insufficient_data_for_unknown_symbol() {
    let state = AppState::new(test_config());
    *state.shared_flow_for_tests().write() = sample_flow_state(0.0, 0.0, 0, 0.0, 0.0, Some(0.0));
    let client = reqwest::Client::builder()
        .no_proxy()
        .build()
        .expect("client");
    let app = router(state);
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0")
        .await
        .expect("bind");
    let addr = listener.local_addr().expect("addr");
    let server = tokio::spawn(async move {
        axum::serve(listener, app).await.expect("server");
    });
    tokio::time::sleep(Duration::from_millis(50)).await;
    assert!(!server.is_finished(), "server task exited before request");

    let recent = client
        .get(format!(
            "http://{addr}/api/toxicity/active-trade/recent?symbol=ETH-PERP"
        ))
        .send()
        .await
        .expect("recent response");
    let recent_code = recent.status();
    let recent_text = recent.text().await.expect("recent text");
    assert_eq!(recent_code, reqwest::StatusCode::OK, "body={recent_text}");
    let recent_payload: serde_json::Value =
        serde_json::from_str(&recent_text).expect("recent json");
    assert_eq!(recent_payload["status"], "insufficient_data");
    assert_eq!(recent_payload["readOnly"], true);
    assert_eq!(recent_payload["runtimeModified"], false);

    server.abort();
}

fn sample_flow_state(
    aggressive_buy_usd: f64,
    aggressive_sell_usd: f64,
    trade_count: u64,
    max_trade_size_btc: f64,
    avg_trade_size_btc: f64,
    price_move_bps: Option<f64>,
) -> FlowState {
    let window = sample_window(
        5_000,
        aggressive_buy_usd,
        aggressive_sell_usd,
        trade_count,
        max_trade_size_btc,
        avg_trade_size_btc,
        price_move_bps,
    );
    FlowState {
        symbol: "BTC-PERP".to_string(),
        updated_at: 1_760_000_000_000,
        windows: BTreeMap::from([(window.window_ms.to_string(), window)]),
    }
}

fn sample_window(
    window_ms: u64,
    aggressive_buy_usd: f64,
    aggressive_sell_usd: f64,
    trade_count: u64,
    max_trade_size_btc: f64,
    avg_trade_size_btc: f64,
    price_move_bps: Option<f64>,
) -> FlowWindow {
    let aggressive_buy_btc = aggressive_buy_usd / 100_000.0;
    let aggressive_sell_btc = aggressive_sell_usd / 100_000.0;
    let buy_trade_count = if aggressive_buy_usd >= aggressive_sell_usd {
        trade_count.saturating_sub(2)
    } else {
        2
    };
    let sell_trade_count = trade_count.saturating_sub(buy_trade_count);
    FlowWindow {
        symbol: "BTC-PERP".to_string(),
        window_ms,
        now_ts: 1_760_000_000_000,
        aggressive_buy_btc,
        aggressive_sell_btc,
        aggressive_buy_usd,
        aggressive_sell_usd,
        net_aggressive_btc: aggressive_buy_btc - aggressive_sell_btc,
        abs_aggressive_btc: aggressive_buy_btc + aggressive_sell_btc,
        trade_count,
        buy_trade_count,
        sell_trade_count,
        avg_trade_size_btc,
        max_trade_size_btc,
        venue_breakdown: empty_venue_breakdown(),
        mid_start: Some(100_000.0),
        mid_end: Some(100_003.0),
        price_move_bps,
        spread_bps_median: None,
        imbalance_10bps_median: None,
        data_quality: DataQuality {
            has_trades: trade_count > 0,
            has_books: false,
            active_venues: vec!["binance".to_string()],
            stale_venues: Vec::new(),
        },
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
        contract_whale_monitor:
            btc_toxic_flow_monitor_rs::config::env::ContractWhaleMonitorConfig {
                enabled: false,
                dry_run: true,
            },
    }
}
