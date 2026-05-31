mod support;
use support::test_http_get;

use btc_toxic_flow_monitor_rs::{
    api::{routes::build_venue_diagnostics_response, server::router},
    app::AppState,
    config::{
        venues::{VenueConfig, VenueConfigs},
        AppConfig,
    },
    types::{
        market::{Venue, VenueConnectionStatus},
        toxic::ToxicSeverity,
    },
};

#[tokio::test]
async fn venues_diagnostics_explains_disabled_venues() {
    let state = AppState::new(test_config(false, false, false));
    state.start().await;
    let app = router(state);
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0")
        .await
        .expect("bind");
    let addr = listener.local_addr().expect("addr");
    let server = tokio::spawn(async move {
        axum::serve(listener, app).await.expect("server");
    });

    let response = test_http_get(format!("http://{addr}/api/venues/diagnostics"))
        .await
        .expect("diagnostics response");
    assert_eq!(response.status(), reqwest::StatusCode::OK);
    let payload: serde_json::Value = response.json().await.expect("diagnostics json");

    assert_eq!(payload["readOnly"], true);
    assert_eq!(payload["analysisOnly"], true);
    assert_eq!(payload["executionEnabled"], false);
    assert_eq!(payload["runtimeModified"], false);
    assert_eq!(payload["monitoringStarted"], true);
    assert_eq!(payload["summary"]["configuredVenues"], 3);
    assert_eq!(payload["summary"]["enabledVenues"], 0);
    assert_eq!(payload["summary"]["connectorConstructedVenues"], 0);
    assert_eq!(payload["summary"]["startAttemptedVenues"], 0);
    assert_eq!(payload["summary"]["connectedVenues"], 0);
    assert_eq!(payload["summary"]["wsConnectAttemptedVenues"], 0);
    assert_eq!(payload["summary"]["wsConnectedVenues"], 0);
    assert_eq!(payload["summary"]["symbolMappedVenues"], 3);
    assert_eq!(payload["summary"]["venuesWithNetworkErrors"], 0);
    assert_eq!(payload["summary"]["activeTradeVenues"], 0);
    assert_eq!(payload["summary"]["activeBookVenues"], 0);
    assert_eq!(payload["summary"]["tradeActiveVenues"], 0);
    assert_eq!(payload["summary"]["bookActiveVenues"], 0);
    assert_eq!(payload["summary"]["activeVenues"], 0);
    assert_eq!(
        payload["summary"]["diagnosticStatus"],
        "no_public_stream_enabled"
    );
    assert_eq!(payload["summary"]["latestVenueTradeAvailable"], false);
    assert_eq!(payload["summary"]["latestVenueBookAvailable"], false);
    assert_eq!(payload["summary"]["flowWindowsPopulated"], false);
    assert!(payload["operatorNotes"]
        .to_string()
        .contains("runtime start was requested"));
    assert!(payload["operatorNotes"]
        .to_string()
        .contains("all venue enable flags are false or missing"));

    let venues = payload["venues"].as_array().expect("venues array");
    assert_eq!(venues.len(), 3);
    assert!(venues.iter().any(|venue| {
        venue["venue"] == "binance"
            && venue["enableFlagName"] == "ENABLE_BINANCE"
            && venue["enableFlagValue"] == false
            && venue["enableSource"].is_string()
            && venue["disabledReason"] == "env_or_config_flag_false"
            && venue["requestedSymbol"] == "BTC-PERP"
            && venue["venueSymbol"] == "BTCUSDT"
            && venue["venueMarketType"] == "linear_perpetual"
            && venue["symbolMappingStatus"] == "ok"
            && venue["symbolMappingError"].is_null()
            && venue["connectorConstructed"] == false
            && venue["startAttempted"] == false
            && venue["wsConfigured"] == false
            && venue["wsConnectAttempted"] == false
            && venue["wsConnected"] == false
            && venue["tradeStreamConfigured"] == false
            && venue["bookStreamConfigured"] == false
            && venue["ackMode"] == "not_supported"
            && venue["tradeMessageCount"] == 0
            && venue["bookMessageCount"] == 0
            && venue["tradeActive"] == false
            && venue["bookActive"] == false
            && venue["activityStatus"] == "disabled"
            && venue["proxySupported"] == false
            && venue["networkProbeEnabled"] == false
            && venue["status"] == "disabled"
    }));

    server.abort();
}

#[test]
fn venue_diagnostics_reports_symbol_mapping_configuration_error() {
    let mut config = test_config(true, false, false);
    config.symbol = "ETH-PERP".to_string();
    let state = AppState::new(config);

    let diagnostics = build_venue_diagnostics_response(&state);
    let binance = diagnostics
        .venues
        .iter()
        .find(|venue| venue.venue == Venue::Binance)
        .expect("binance diagnostics");

    assert_eq!(diagnostics.summary.enabled_venues, 1);
    assert_eq!(diagnostics.summary.connector_constructed_venues, 0);
    assert_eq!(diagnostics.summary.start_attempted_venues, 0);
    assert_eq!(binance.requested_symbol, "ETH-PERP");
    assert_eq!(binance.venue_symbol, None);
    assert_eq!(binance.symbol_mapping_status, "missing");
    assert!(binance
        .symbol_mapping_error
        .as_deref()
        .is_some_and(|error| error.contains("ETH-PERP")));
    assert_eq!(
        binance.disabled_reason.as_deref(),
        Some("symbol_mapping_missing")
    );
    assert_eq!(binance.status, VenueConnectionStatus::ConfigurationError);
}

#[tokio::test]
async fn venues_diagnostics_distinguishes_enabled_from_connected() {
    let state = AppState::new(test_config(true, false, false));
    state.start().await;
    let app = router(state.clone());
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0")
        .await
        .expect("bind");
    let addr = listener.local_addr().expect("addr");
    let server = tokio::spawn(async move {
        axum::serve(listener, app).await.expect("server");
    });

    let response = test_http_get(format!("http://{addr}/api/venues/diagnostics"))
        .await
        .expect("diagnostics response");
    assert_eq!(response.status(), reqwest::StatusCode::OK);
    let payload: serde_json::Value = response.json().await.expect("diagnostics json");

    assert_eq!(payload["monitoringStarted"], true);
    assert_eq!(payload["summary"]["enabledVenues"], 1);
    assert_eq!(payload["summary"]["connectorConstructedVenues"], 1);
    assert_eq!(payload["summary"]["startAttemptedVenues"], 1);
    assert_eq!(payload["summary"]["wsConnectAttemptedVenues"], 1);
    let diagnostic_status = payload["summary"]["diagnosticStatus"]
        .as_str()
        .expect("diagnostic status");
    assert!(matches!(
        diagnostic_status,
        "enabled_but_not_connected"
            | "connected_but_no_events"
            | "events_seen_but_flow_empty"
            | "network_error"
    ));
    let venues = payload["venues"].as_array().expect("venues array");
    assert!(venues.iter().any(|venue| {
        venue["venue"] == "binance"
            && venue["enableFlagValue"] == true
            && venue["requestedSymbol"] == "BTC-PERP"
            && venue["venueSymbol"] == "BTCUSDT"
            && venue["symbolMappingStatus"] == "ok"
            && venue["connectorConstructed"] == true
            && venue["startAttempted"] == true
            && venue["wsConfigured"] == true
            && venue["wsConnectAttempted"] == true
            && venue["tradeSubscribeAttempted"] == true
            && venue["bookSubscribeAttempted"] == true
            && venue["ackMode"] == "not_supported"
            && venue["disabledReason"].is_null()
    }));

    state.stop().await;
    server.abort();
}

fn test_config(binance: bool, bybit: bool, okx: bool) -> AppConfig {
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
                enabled: binance,
            },
            bybit: VenueConfig {
                venue: Venue::Bybit,
                enabled: bybit,
            },
            okx: VenueConfig {
                venue: Venue::Okx,
                enabled: okx,
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
