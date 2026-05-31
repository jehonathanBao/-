mod support;
use support::test_http_get;

use std::{
    collections::BTreeMap,
    fs,
    path::PathBuf,
    time::{SystemTime, UNIX_EPOCH},
};

use btc_toxic_flow_monitor_rs::{
    api::server::router,
    app::AppState,
    config::{
        venues::{VenueConfig, VenueConfigs},
        AppConfig,
    },
    storage::{
        snapshots_repo::SnapshotsRepo, sqlite::SqliteStore, toxic_events_repo::ToxicEventsRepo,
        venue_health_repo::VenueHealthRepo,
    },
    types::{
        flow::{DataQuality, FlowState, FlowWindow, VenueFlowBreakdown},
        market::{Venue, VenueConnectionStatus, VenueHealth},
        toxic::{
            ToxicDirection, ToxicEvent, ToxicQuality, ToxicSeverity, ToxicState, ToxicVolumeResult,
        },
    },
};

#[test]
fn migration_is_idempotent_and_repos_work() {
    let store = open_store("storage_repo");
    store.migrate().expect("first migrate");
    store.migrate().expect("second migrate");

    let event = sample_event(1_000);
    store.insert_event(&event).expect("insert event");
    store
        .insert_event(&event)
        .expect("duplicate insert ignored");

    let latest = store.get_latest_event().expect("latest");
    assert_eq!(latest.as_ref().map(|e| &e.id), Some(&event.id));

    let events = store.list_recent_events(10).expect("list");
    assert_eq!(events.len(), 1);
    assert_eq!(events[0].id, event.id);

    let flow_state = sample_flow_state(2_000);
    store
        .insert_flow_snapshot(&flow_state)
        .expect("flow snapshot");

    let toxic_state = sample_toxic_state(3_000, &event);
    store
        .insert_toxic_snapshot(&toxic_state)
        .expect("toxic snapshot");

    let venue_health = sample_venue_health();
    store
        .insert_venue_health_snapshot(4_000, &venue_health)
        .expect("venue health snapshot");

    let snapshots = store
        .list_toxic_snapshots(0, 10_000, 10)
        .expect("snapshots");
    assert_eq!(snapshots.len(), 1);
    assert_eq!(snapshots[0]["symbol"], "BTC-PERP");
}

#[test]
fn open_creates_parent_directories() {
    let path = unique_path("nested/storage/test.sqlite");
    let store = SqliteStore::open(path.to_str().expect("utf8 path")).expect("open");
    store.migrate().expect("migrate");
    assert!(path.exists());
}

#[tokio::test]
async fn toxic_events_api_returns_persisted_rows() {
    let sqlite_path = unique_path("api/events.sqlite");
    let state = AppState::new(test_config(
        sqlite_path.to_str().expect("utf8 path").to_string(),
    ));
    let store = SqliteStore::open(sqlite_path.to_str().expect("utf8 path")).expect("open sqlite");
    store.migrate().expect("migrate");
    let event = sample_event(5_000);
    store.insert_event(&event).expect("insert event");

    let app = router(state.clone());
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0")
        .await
        .expect("bind");
    let addr = listener.local_addr().expect("addr");
    let server = tokio::spawn(async move {
        axum::serve(listener, app).await.expect("server");
    });

    let response = test_http_get(format!("http://{addr}/api/toxic-events?limit=10"))
        .await
        .expect("response");
    assert_eq!(response.status(), reqwest::StatusCode::OK);
    let payload: serde_json::Value = response.json().await.expect("json");
    assert_eq!(payload["events"][0]["id"], event.id);

    let latest = test_http_get(format!("http://{addr}/api/toxic-events/latest"))
        .await
        .expect("latest response");
    let latest_payload: serde_json::Value = latest.json().await.expect("latest json");
    assert_eq!(latest_payload["event"]["id"], event.id);

    server.abort();
}

#[tokio::test]
async fn storage_status_api_reports_snapshot_state() {
    let sqlite_path = unique_path("api/storage.sqlite");
    let state = AppState::new(test_config(
        sqlite_path.to_str().expect("utf8 path").to_string(),
    ));
    state
        .snapshot_service_for_tests()
        .persist_once_for_tests(7_000);

    let app = router(state.clone());
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0")
        .await
        .expect("bind");
    let addr = listener.local_addr().expect("addr");
    let server = tokio::spawn(async move {
        axum::serve(listener, app).await.expect("server");
    });

    let response = test_http_get(format!("http://{addr}/api/storage/status"))
        .await
        .expect("response");
    assert_eq!(response.status(), reqwest::StatusCode::OK);
    let payload: serde_json::Value = response.json().await.expect("json");
    assert_eq!(payload["enabled"], true);
    assert!(payload["status"].is_string());
    assert_eq!(payload["lastWriteTs"], 7_000);

    server.abort();
}

fn sample_event(ts: i64) -> ToxicEvent {
    ToxicEvent {
        id: format!("event-{ts}"),
        ts,
        symbol: "BTC-PERP".to_string(),
        direction: ToxicDirection::Buy,
        severity: ToxicSeverity::Alert,
        toxic_volume_btc: 1_284.2,
        threshold_btc: 1_000.0,
        window_ms: 5_000,
        leader_venue: Some(Venue::Binance),
        aggressive_buy_btc: 1_566.0,
        aggressive_sell_btc: 220.0,
        net_aggressive_btc: 1_346.0,
        abs_aggressive_btc: 1_786.0,
        markout_1s_bps: Some(2.1),
        markout_5s_bps: Some(4.8),
        sweep_detected: true,
        liquidity_thin: true,
        liquidity: None,
        cross_venue_confirmed: true,
        vpin_enabled: true,
        vpin: Some(0.82),
        vpin_zscore: Some(2.8),
        vpin_spike: true,
        vpin_high: false,
        vpin_extreme: false,
        liquidation_enabled: true,
        nearest_cluster_side: Some(
            btc_toxic_flow_monitor_rs::types::liquidation::LiquidationClusterSide::ShortAbove,
        ),
        cluster_distance_bps: Some(10.0),
        cluster_notional_usd: Some(2_000_000.0),
        cluster_density: Some(0.6),
        liq_hunt_pressure: 0.74,
        liq_cluster_nearby: true,
        possible_liq_hunt_setup: true,
        reason_codes: vec![
            "large_aggressive_flow".to_string(),
            "markout_1s_confirmed".to_string(),
            "threshold_crossed".to_string(),
        ],
    }
}

fn sample_toxic_state(ts: i64, event: &ToxicEvent) -> ToxicState {
    let mut results = BTreeMap::new();
    results.insert(
        "5000".to_string(),
        ToxicVolumeResult {
            symbol: "BTC-PERP".to_string(),
            window_ms: 5_000,
            ts,
            direction: ToxicDirection::Buy,
            severity: ToxicSeverity::Alert,
            toxic_ratio: 0.85,
            toxic_volume_btc: event.toxic_volume_btc,
            threshold_btc: 1_000.0,
            alert_triggered: true,
            aggressive_buy_btc: event.aggressive_buy_btc,
            aggressive_sell_btc: event.aggressive_sell_btc,
            net_aggressive_btc: event.net_aggressive_btc,
            abs_aggressive_btc: event.abs_aggressive_btc,
            markout_1s_bps: event.markout_1s_bps,
            markout_5s_bps: event.markout_5s_bps,
            markout_confirmed: true,
            sweep_detected: event.sweep_detected,
            liquidity_thin: event.liquidity_thin,
            liquidity: None,
            cross_venue_confirmed: true,
            vpin_enabled: event.vpin_enabled,
            vpin: event.vpin,
            vpin_zscore: event.vpin_zscore,
            vpin_spike: event.vpin_spike,
            vpin_high: event.vpin_high,
            vpin_extreme: event.vpin_extreme,
            liquidation_enabled: event.liquidation_enabled,
            nearest_cluster_side: event.nearest_cluster_side,
            cluster_distance_bps: event.cluster_distance_bps,
            cluster_notional_usd: event.cluster_notional_usd,
            cluster_density: event.cluster_density,
            liq_hunt_pressure: event.liq_hunt_pressure,
            liq_cluster_nearby: event.liq_cluster_nearby,
            possible_liq_hunt_setup: event.possible_liq_hunt_setup,
            leader_venue: event.leader_venue,
            venue_breakdown: BTreeMap::new(),
            reason_codes: event.reason_codes.clone(),
        },
    );
    ToxicState {
        symbol: "BTC-PERP".to_string(),
        updated_at: ts,
        threshold_btc: 1_000.0,
        windows_ms: vec![1_000, 5_000, 15_000, 60_000],
        results,
        latest_event: Some(event.clone()),
        recent_events: vec![event.clone()],
        quality: ToxicQuality {
            has_flow: true,
            has_markout: true,
            has_sweep: true,
            has_liquidation: true,
            liquidation: None,
            active_venues: vec![Venue::Binance, Venue::Bybit],
            stale_venues: vec![Venue::Okx],
        },
    }
}

fn sample_flow_state(ts: i64) -> FlowState {
    let mut windows = BTreeMap::new();
    windows.insert(
        "5000".to_string(),
        FlowWindow {
            symbol: "BTC-PERP".to_string(),
            window_ms: 5_000,
            now_ts: ts,
            aggressive_buy_btc: 1_566.0,
            aggressive_sell_btc: 220.0,
            aggressive_buy_usd: 156_600_000.0,
            aggressive_sell_usd: 22_000_000.0,
            net_aggressive_btc: 1_346.0,
            abs_aggressive_btc: 1_786.0,
            trade_count: 12,
            buy_trade_count: 8,
            sell_trade_count: 4,
            avg_trade_size_btc: 148.8,
            max_trade_size_btc: 500.0,
            venue_breakdown: {
                let mut map = BTreeMap::new();
                map.insert(
                    "binance".to_string(),
                    VenueFlowBreakdown {
                        aggressive_buy_btc: 900.0,
                        aggressive_sell_btc: 100.0,
                        aggressive_buy_usd: 90_000_000.0,
                        aggressive_sell_usd: 10_000_000.0,
                        net_aggressive_btc: 800.0,
                        abs_aggressive_btc: 1_000.0,
                        trade_count: 6,
                        buy_trade_count: 4,
                        sell_trade_count: 2,
                        last_trade_ts: Some(ts),
                    },
                );
                map.insert(
                    "bybit".to_string(),
                    VenueFlowBreakdown {
                        aggressive_buy_btc: 500.0,
                        aggressive_sell_btc: 100.0,
                        aggressive_buy_usd: 50_000_000.0,
                        aggressive_sell_usd: 10_000_000.0,
                        net_aggressive_btc: 400.0,
                        abs_aggressive_btc: 600.0,
                        trade_count: 4,
                        buy_trade_count: 3,
                        sell_trade_count: 1,
                        last_trade_ts: Some(ts),
                    },
                );
                map.insert(
                    "okx".to_string(),
                    VenueFlowBreakdown {
                        aggressive_buy_btc: 166.0,
                        aggressive_sell_btc: 20.0,
                        aggressive_buy_usd: 16_600_000.0,
                        aggressive_sell_usd: 2_000_000.0,
                        net_aggressive_btc: 146.0,
                        abs_aggressive_btc: 186.0,
                        trade_count: 2,
                        buy_trade_count: 1,
                        sell_trade_count: 1,
                        last_trade_ts: Some(ts),
                    },
                );
                map
            },
            mid_start: Some(100_000.0),
            mid_end: Some(100_500.0),
            price_move_bps: Some(50.0),
            spread_bps_median: Some(3.0),
            imbalance_10bps_median: Some(0.25),
            data_quality: DataQuality {
                has_trades: true,
                has_books: true,
                active_venues: vec![
                    "binance".to_string(),
                    "bybit".to_string(),
                    "okx".to_string(),
                ],
                stale_venues: Vec::new(),
            },
        },
    );
    FlowState {
        symbol: "BTC-PERP".to_string(),
        updated_at: ts,
        windows,
    }
}

fn sample_venue_health() -> std::collections::BTreeMap<String, VenueHealth> {
    let mut map = std::collections::BTreeMap::new();
    map.insert(
        "binance".to_string(),
        VenueHealth {
            venue: Venue::Binance,
            enabled: true,
            enable_flag_name: "ENABLE_BINANCE".to_string(),
            enable_flag_value: true,
            enable_source: "test".to_string(),
            disabled_reason: None,
            requested_symbol: "BTC-PERP".to_string(),
            venue_symbol: Some("BTCUSDT".to_string()),
            venue_market_type: Some("linear_perpetual".to_string()),
            symbol_mapping_status: "ok".to_string(),
            symbol_mapping_error: None,
            connector_constructed: true,
            start_attempted: true,
            status: VenueConnectionStatus::Connected,
            last_trade_ts: Some(1_000),
            last_book_ts: Some(1_000),
            last_message_ts: Some(1_000),
            reconnect_count: 0,
            last_error: None,
            ws_configured: true,
            ws_connect_attempted: true,
            ws_connected: true,
            ws_last_connect_at_ms: Some(1_000),
            ws_last_disconnect_at_ms: None,
            ws_reconnect_count: 0,
            ws_last_error: None,
            ws_error_class: "none".to_string(),
            trade_stream_configured: true,
            book_stream_configured: true,
            trade_subscribe_attempted: true,
            book_subscribe_attempted: true,
            trade_subscribe_acked: false,
            book_subscribe_acked: false,
            ack_mode: "not_supported".to_string(),
            last_trade_message_at_ms: Some(1_000),
            last_book_message_at_ms: Some(1_000),
            trade_message_count: 1,
            book_message_count: 1,
            last_parsed_trade_at_ms: Some(1_000),
            last_parsed_book_at_ms: Some(1_000),
            last_parse_error: None,
            active_window_ms: 30_000,
            trade_active: true,
            book_active: true,
            activity_status: "active".to_string(),
            proxy_enabled: false,
            proxy_supported: false,
            proxy_source: None,
            proxy_scheme: None,
            proxy_host_masked: None,
            proxy_port_masked: None,
            proxy_configured_for_ws: false,
            network_probe_enabled: false,
            last_network_error_class: "none".to_string(),
        },
    );
    map
}

fn test_config(sqlite_path: String) -> AppConfig {
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
        sqlite_enabled: true,
        sqlite_path,
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

fn unique_path(suffix: &str) -> PathBuf {
    let base = std::env::temp_dir().join(format!(
        "btc_toxic_flow_monitor_rs_{}_{}",
        std::process::id(),
        now_nanos()
    ));
    let path = base.join(suffix);
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).expect("create temp dir");
    }
    path
}

fn open_store(name: &str) -> SqliteStore {
    let path = unique_path(name);
    SqliteStore::open(path.to_str().expect("utf8 path")).expect("open sqlite")
}

fn now_nanos() -> u128 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("time")
        .as_nanos()
}
