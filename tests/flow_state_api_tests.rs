mod support;
use support::test_http_get;

use std::collections::BTreeMap;

use btc_toxic_flow_monitor_rs::{
    api::server::router,
    app::AppState,
    config::{
        venues::{VenueConfig, VenueConfigs},
        AppConfig,
    },
    normalizers::trade::now_ms,
    types::{
        flow::{empty_venue_breakdown, DataQuality, FlowState, FlowWindow},
        market::{AggressorSide, NormalizedBook, NormalizedTrade, Venue},
        toxic::ToxicSeverity,
    },
};

#[tokio::test]
async fn flow_state_api_returns_empty_windows() {
    let state = AppState::new(test_config());
    state.start().await;
    state.flow_state();
    let app = router(state.clone());
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0")
        .await
        .expect("bind");
    let addr = listener.local_addr().expect("addr");
    let server = tokio::spawn(async move {
        axum::serve(listener, app).await.expect("server");
    });

    let response = test_http_get(format!("http://{addr}/api/flow-state"))
        .await
        .expect("response");
    assert_eq!(response.status(), reqwest::StatusCode::OK);
    let payload: serde_json::Value = response.json().await.expect("json");

    assert_eq!(payload["symbol"], "BTC-PERP");

    state.stop().await;
    server.abort();
}

#[tokio::test]
async fn markout_runtime_filters_configured_symbol_and_uses_its_price_index() {
    let mut config = test_config();
    config.markout_horizons_ms = vec![20];
    config.markout_resolve_interval_ms = 5;
    let state = AppState::new(config);
    state.start().await;

    let base_ts = now_ms();
    let flow_service = state.flow_service_for_tests();
    flow_service.add_book_for_tests(book(Venue::Binance, "BTC-PERP", base_ts + 20, 101.0));
    flow_service.add_book_for_tests(book(Venue::Bybit, "ETH-PERP", base_ts + 20, 201.0));

    assert_eq!(
        flow_service.get_mid_at_or_before_for_symbol(base_ts + 20, "BTC-PERP"),
        Some(101.0)
    );
    assert_eq!(
        flow_service.get_mid_at_or_before_for_symbol(base_ts + 20, "ETH-PERP"),
        Some(201.0)
    );

    state.ingest_trade_event_for_tests(trade(Venue::Bybit, "ETH-PERP", base_ts, 200.0, "same"));
    state.ingest_trade_event_for_tests(trade(Venue::Binance, "BTC-PERP", base_ts, 100.0, "same"));
    tokio::time::sleep(std::time::Duration::from_millis(80)).await;

    let markout = state.markout_state();
    assert_eq!(markout.symbol, "BTC-PERP");
    assert_eq!(markout.summaries["20"].buy.count, 1);
    assert_eq!(
        markout.summaries["20"].buy.volume_weighted_markout_bps,
        Some(100.0)
    );

    state.stop().await;
}

#[tokio::test]
async fn liquidation_cascade_eth_request_does_not_reuse_btc_flow_state() {
    let state = AppState::new(test_config());
    *state.shared_flow_for_tests().write() = strong_buy_flow_state("BTC-PERP");

    let app = router(state);
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0")
        .await
        .expect("bind");
    let addr = listener.local_addr().expect("addr");
    let server = tokio::spawn(async move {
        axum::serve(listener, app).await.expect("server");
    });

    let response = test_http_get(format!(
        "http://{addr}/api/liquidation/cascade?symbol=ETHUSDT"
    ))
    .await
    .expect("response");
    assert_eq!(response.status(), reqwest::StatusCode::OK);
    let payload: serde_json::Value = response.json().await.expect("json");

    assert_eq!(payload["symbol"], "ETH");
    assert_eq!(payload["direction"], "NEUTRAL");
    assert_eq!(payload["components"]["liquidityGap"], 0.0);
    assert_eq!(payload["components"]["triggerProximity"], 0.0);

    server.abort();
}

fn trade(venue: Venue, symbol: &str, ts: i64, price: f64, trade_id: &str) -> NormalizedTrade {
    NormalizedTrade {
        venue,
        symbol: symbol.to_string(),
        ts,
        price,
        size_btc: 1.0,
        size_usd: price,
        aggressor_side: AggressorSide::Buy,
        trade_id: Some(trade_id.to_string()),
    }
}

fn book(venue: Venue, symbol: &str, ts: i64, mid: f64) -> NormalizedBook {
    NormalizedBook {
        venue,
        symbol: symbol.to_string(),
        ts,
        best_bid: mid - 0.5,
        best_ask: mid + 0.5,
        bids: vec![(mid - 0.5, 1.0)],
        asks: vec![(mid + 0.5, 1.0)],
        mid,
        spread_bps: 1.0,
        bid_depth_btc_10bps: 1.0,
        ask_depth_btc_10bps: 1.0,
        bid_depth_usd_10bps: mid,
        ask_depth_usd_10bps: mid,
        imbalance_10bps: 0.0,
    }
}

fn strong_buy_flow_state(symbol: &str) -> FlowState {
    FlowState {
        symbol: symbol.to_string(),
        updated_at: 5_000,
        windows: BTreeMap::from([(
            "5000".to_string(),
            FlowWindow {
                symbol: symbol.to_string(),
                window_ms: 5_000,
                now_ts: 5_000,
                aggressive_buy_btc: 100.0,
                aggressive_sell_btc: 0.0,
                aggressive_buy_usd: 10_000_000.0,
                aggressive_sell_usd: 0.0,
                net_aggressive_btc: 100.0,
                abs_aggressive_btc: 100.0,
                trade_count: 10,
                buy_trade_count: 10,
                sell_trade_count: 0,
                avg_trade_size_btc: 10.0,
                max_trade_size_btc: 20.0,
                venue_breakdown: empty_venue_breakdown(),
                mid_start: Some(100_000.0),
                mid_end: Some(100_100.0),
                price_move_bps: Some(10.0),
                spread_bps_median: Some(1.0),
                imbalance_10bps_median: Some(1.0),
                data_quality: DataQuality {
                    has_trades: true,
                    has_books: true,
                    active_venues: vec!["binance".to_string()],
                    stale_venues: Vec::new(),
                },
            },
        )]),
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
