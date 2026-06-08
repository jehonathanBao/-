mod support;
use std::collections::BTreeMap;

use support::test_http_get;

use btc_toxic_flow_monitor_rs::{
    api::server::router,
    app::AppState,
    config::{
        venues::{VenueConfig, VenueConfigs},
        AppConfig,
    },
    types::{
        flow::{DataQuality, FlowState, FlowWindow, VenueFlowBreakdown},
        market::{Venue, VenueConnectionStatus, VenueHealth},
        status::VenueHealthMap,
        toxic::ToxicSeverity,
    },
};

#[tokio::test]
async fn whale_flow_api_returns_analysis_only_candidate_payloads() {
    let state = AppState::new(test_config());
    state.market_data_quality().record_flow_window_lagged(2);
    state.market_data_quality().record_send_error();
    *state.shared_flow_for_tests().write() = sample_flow_state();
    for health in sample_venue_health().into_values() {
        state.set_health_for_tests(health);
    }

    let app = router(state);
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0")
        .await
        .expect("bind");
    let addr = listener.local_addr().expect("addr");
    let server = tokio::spawn(async move {
        axum::serve(listener, app).await.expect("server");
    });

    let status = test_http_get(format!("http://{addr}/api/toxicity/whale-flow/status"))
        .await
        .expect("status response");
    assert_eq!(status.status(), reqwest::StatusCode::OK);
    let status_payload: serde_json::Value = status.json().await.expect("status json");
    assert_eq!(status_payload["readOnly"], true);
    assert_eq!(status_payload["analysisOnly"], true);
    assert_eq!(status_payload["executionEnabled"], false);
    assert_eq!(status_payload["status"], "candidate_active");
    assert_eq!(status_payload["candidateCount"], 1);
    assert_eq!(status_payload["laggedEvents"], 2);
    assert!(status_payload["droppedEvents"].as_u64().unwrap_or_default() >= 1);
    assert_eq!(status_payload["dataQuality"]["status"], "partial");
    assert_eq!(status_payload["venueCoverage"]["enabledVenues"], 3);
    assert_eq!(status_payload["venueCoverage"]["connectedVenues"], 2);
    assert_eq!(
        status_payload["baselineQuality"]["baselineSource"],
        "sixty_second_fallback"
    );
    assert_eq!(status_payload["thresholds"]["minVenueConfirmations"], 2);

    let recent = test_http_get(format!("http://{addr}/api/toxicity/whale-flow/recent"))
        .await
        .expect("recent response");
    assert_eq!(recent.status(), reqwest::StatusCode::OK);
    let recent_payload: serde_json::Value = recent.json().await.expect("recent json");
    assert_eq!(recent_payload["readOnly"], true);
    assert_eq!(recent_payload["analysisOnly"], true);
    assert_eq!(recent_payload["executionEnabled"], false);
    assert_eq!(recent_payload["selectedSymbol"], "BTC-PERP");
    assert_eq!(recent_payload["status"], "candidate_active");
    assert!(recent_payload["candidates"].is_array());
    assert_eq!(recent_payload["dataQuality"]["latestTradeAvailable"], true);
    assert_eq!(recent_payload["dataQuality"]["latestBookAvailable"], false);
    assert_eq!(
        recent_payload["venueCoverage"]["venuesMissingBooks"][0],
        "binance"
    );
    assert_eq!(
        recent_payload["candidates"][0]["candidateType"],
        "aggressive_buy"
    );
    assert!(recent_payload["candidates"][0]["diagnostics"]["whyCandidate"].is_array());

    let by_symbol = test_http_get(format!("http://{addr}/api/toxicity/whale-flow/BTC-PERP"))
        .await
        .expect("symbol response");
    assert_eq!(by_symbol.status(), reqwest::StatusCode::OK);
    let by_symbol_payload: serde_json::Value = by_symbol.json().await.expect("symbol json");
    assert_eq!(by_symbol_payload["selectedSymbol"], "BTC-PERP");

    server.abort();
}

fn sample_flow_state() -> FlowState {
    let mut windows = BTreeMap::new();
    windows.insert("1000".to_string(), flow_window(1_000, 20.0, 10.0));
    windows.insert("5000".to_string(), flow_window(5_000, 420.0, 60.0));
    windows.insert("15000".to_string(), flow_window(15_000, 540.0, 220.0));
    windows.insert("60000".to_string(), flow_window(60_000, 700.0, 200.0));
    FlowState {
        symbol: "BTC-PERP".to_string(),
        updated_at: 5_000,
        windows,
    }
}

fn flow_window(window_ms: u64, aggressive_buy_btc: f64, aggressive_sell_btc: f64) -> FlowWindow {
    let mut venue_breakdown = BTreeMap::new();
    venue_breakdown.insert(
        "binance".to_string(),
        VenueFlowBreakdown {
            aggressive_buy_btc: aggressive_buy_btc * 0.60,
            aggressive_sell_btc: aggressive_sell_btc * 0.25,
            aggressive_buy_usd: 0.0,
            aggressive_sell_usd: 0.0,
            net_aggressive_btc: aggressive_buy_btc * 0.60 - aggressive_sell_btc * 0.25,
            abs_aggressive_btc: aggressive_buy_btc * 0.60 + aggressive_sell_btc * 0.25,
            trade_count: 8,
            buy_trade_count: 5,
            sell_trade_count: 3,
            last_trade_ts: Some(5_000),
        },
    );
    venue_breakdown.insert(
        "bybit".to_string(),
        VenueFlowBreakdown {
            aggressive_buy_btc: aggressive_buy_btc * 0.30,
            aggressive_sell_btc: aggressive_sell_btc * 0.20,
            aggressive_buy_usd: 0.0,
            aggressive_sell_usd: 0.0,
            net_aggressive_btc: aggressive_buy_btc * 0.30 - aggressive_sell_btc * 0.20,
            abs_aggressive_btc: aggressive_buy_btc * 0.30 + aggressive_sell_btc * 0.20,
            trade_count: 6,
            buy_trade_count: 4,
            sell_trade_count: 2,
            last_trade_ts: Some(5_000),
        },
    );
    venue_breakdown.insert(
        "okx".to_string(),
        VenueFlowBreakdown {
            aggressive_buy_btc: aggressive_buy_btc * 0.10,
            aggressive_sell_btc: aggressive_sell_btc * 0.55,
            aggressive_buy_usd: 0.0,
            aggressive_sell_usd: 0.0,
            net_aggressive_btc: aggressive_buy_btc * 0.10 - aggressive_sell_btc * 0.55,
            abs_aggressive_btc: aggressive_buy_btc * 0.10 + aggressive_sell_btc * 0.55,
            trade_count: 3,
            buy_trade_count: 1,
            sell_trade_count: 2,
            last_trade_ts: Some(5_000),
        },
    );

    FlowWindow {
        symbol: "BTC-PERP".to_string(),
        window_ms,
        now_ts: 5_000,
        aggressive_buy_btc,
        aggressive_sell_btc,
        aggressive_buy_usd: 0.0,
        aggressive_sell_usd: 0.0,
        net_aggressive_btc: aggressive_buy_btc - aggressive_sell_btc,
        abs_aggressive_btc: aggressive_buy_btc + aggressive_sell_btc,
        trade_count: 12,
        buy_trade_count: 8,
        sell_trade_count: 4,
        avg_trade_size_btc: 12.0,
        max_trade_size_btc: aggressive_buy_btc.max(aggressive_sell_btc) / 2.0,
        venue_breakdown,
        mid_start: Some(100_000.0),
        mid_end: Some(100_150.0),
        price_move_bps: Some(if window_ms == 5_000 { 2.4 } else { 0.8 }),
        spread_bps_median: Some(0.8),
        imbalance_10bps_median: Some(0.22),
        data_quality: DataQuality {
            has_trades: true,
            has_books: true,
            active_venues: vec!["binance".to_string(), "bybit".to_string()],
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
                enabled: true,
            },
            bybit: VenueConfig {
                venue: Venue::Bybit,
                enabled: true,
            },
            okx: VenueConfig {
                venue: Venue::Okx,
                enabled: true,
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

fn sample_venue_health() -> VenueHealthMap {
    let mut venues = VenueHealthMap::new();
    venues.insert("binance".to_string(), connected_venue(Venue::Binance, true));
    venues.insert("bybit".to_string(), connected_venue(Venue::Bybit, false));
    venues.insert("okx".to_string(), disconnected_venue(Venue::Okx));
    venues
}

fn connected_venue(venue: Venue, with_book: bool) -> VenueHealth {
    let mut health = VenueHealth::from_config(venue, true);
    health.status = VenueConnectionStatus::Connected;
    health.last_trade_ts = Some(5_000);
    health.last_book_ts = with_book.then_some(5_000);
    health.start_attempted = true;
    health.connector_constructed = true;
    health
}

fn disconnected_venue(venue: Venue) -> VenueHealth {
    let mut health = VenueHealth::from_config(venue, true);
    health.status = VenueConnectionStatus::Disconnected;
    health.start_attempted = true;
    health.connector_constructed = true;
    health
}
