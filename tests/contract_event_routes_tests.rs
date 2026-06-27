mod support;

use std::time::{SystemTime, UNIX_EPOCH};

use btc_toxic_flow_monitor_rs::{
    api::server::router,
    app::AppState,
    config::{
        env::{ContractWhaleMonitorConfig, SpotWhaleMonitorConfig},
        system_mode::SystemModeConfig,
        venues::{VenueConfig, VenueConfigs},
        AppConfig,
    },
    contract_whale_monitor::{
        aggregator::{aggregate_1s_buckets, rolling_window_stats},
        config::reset_contract_whale_runtime_config,
        detector::detect_contract_whale_signal,
        normalizer::{normalize_binance_agg_trade, normalize_bitfinex_trade},
        types::{ContractFlowBucket, ContractWhaleSignal},
    },
    storage::contract_whale_repo::ContractWhaleRepo,
    types::{market::Venue, toxic::ToxicSeverity},
};
use reqwest::StatusCode;

use support::test_http_client;

#[tokio::test]
async fn contract_events_include_hidden_exposes_visibility_metadata() {
    let state = seeded_contract_event_state();
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
        .get(format!(
            "http://{addr}/api/contract-events?symbol=BTC&range=24h&limit=50&include_hidden=true"
        ))
        .send()
        .await
        .expect("contract events response");

    assert_eq!(response.status(), StatusCode::OK);
    let payload: serde_json::Value = response.json().await.expect("contract events json");
    let items = payload["items"].as_array().expect("items array");
    assert_eq!(items.len(), 3);
    assert!(items
        .iter()
        .any(|item| { item["isVisible"] == true && item["hiddenReason"].is_null() }));
    assert!(items.iter().any(|item| {
        item["isVisible"] == false
            && item["hiddenReason"] == "price_deviation_gt_5pct"
            && item["hiddenDetail"]
                .as_str()
                .is_some_and(|detail| detail.contains("5%"))
    }));
    assert!(items
        .iter()
        .any(|item| { item["isVisible"] == false && item["hiddenReason"] == "bad_quality" }));

    server.abort();
}

#[tokio::test]
async fn contract_events_expose_latency_metadata_fields() {
    let state = seeded_contract_event_state();
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
        .get(format!(
            "http://{addr}/api/contract-events?symbol=BTC&range=24h&limit=20"
        ))
        .send()
        .await
        .expect("contract events response");

    assert_eq!(response.status(), StatusCode::OK);
    let payload: serde_json::Value = response.json().await.expect("contract events json");
    assert!(payload["serverTime"].as_i64().is_some());
    assert!(payload["maxEventTs"].as_i64().is_some());
    assert!(payload["maxPersistedAt"].is_number() || payload["maxPersistedAt"].is_null());
    assert!(payload["historyLagSec"].as_i64().is_some());
    assert!(payload["latestLagSec"].as_i64().is_some());
    assert!(payload["cacheAgeSec"].as_i64().is_some());
    assert!(payload["cacheTtlSec"].as_i64().is_some());

    server.abort();
}

#[tokio::test]
async fn contract_events_debug_counts_reports_filter_chain_and_projection_counts() {
    let state = seeded_contract_event_state();
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
        .get(format!(
            "http://{addr}/api/contract-events/debug-counts?symbol=BTC&range=24h&include_hidden=true"
        ))
        .send()
        .await
        .expect("debug counts response");

    assert_eq!(response.status(), StatusCode::OK);
    let payload: serde_json::Value = response.json().await.expect("debug counts json");
    assert_eq!(payload["symbol"], "BTC");
    assert_eq!(payload["range"], "24h");
    assert_eq!(payload["db"]["contractWhaleSignalsBtc24h"], 3);
    assert_eq!(payload["apiQuery"]["matchedAfterRangeFilter"], 3);
    assert_eq!(payload["visibility"]["visibleCount"], 1);
    assert_eq!(payload["visibility"]["hiddenCount"], 2);
    assert_eq!(
        payload["visibility"]["hiddenReasons"]["priceDeviationGt5pct"],
        1
    );
    assert_eq!(payload["visibility"]["hiddenReasons"]["badQuality"], 1);
    assert_eq!(payload["latest"]["latestCount"], 2);
    assert_eq!(payload["finalEventsV2"]["activeCount"], 1);
    assert_eq!(payload["finalEventsV2"]["closedCount"], 0);
    assert!(payload["latestVsHistory"].is_array());
    assert!(payload["finalEventsProjection"].is_object());

    server.abort();
}

#[tokio::test]
async fn contract_whale_latest_exposes_staleness_summary_metadata() {
    let state = seeded_pipeline_debug_state();
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
        .get(format!(
            "http://{addr}/api/contract-whale/latest?symbol=BTC&range=24h"
        ))
        .send()
        .await
        .expect("latest response");

    assert_eq!(response.status(), StatusCode::OK);
    let payload: serde_json::Value = response.json().await.expect("latest json");
    assert!(payload["serverTime"].as_i64().is_some());
    assert!(payload["maxTs"].as_i64().is_some());
    assert!(payload["maxAgeSec"].as_i64().is_some());
    assert!(payload["staleCount"].as_u64().is_some());

    server.abort();
}

#[tokio::test]
async fn contract_whale_trading_decisions_route_exposes_ranked_setups_and_no_trade_zones() {
    let state = seeded_contract_event_state();
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
        .get(format!(
            "http://{addr}/api/contract-whale/trading-decisions?symbol=BTC&range=24h"
        ))
        .send()
        .await
        .expect("trading decisions response");

    assert_eq!(response.status(), StatusCode::OK);
    let payload: serde_json::Value = response.json().await.expect("trading decisions json");
    assert_eq!(payload["symbol"], "BTC");
    assert!(payload["timestamp"].as_i64().is_some());
    assert!(payload["marketBias"].as_str().is_some());
    assert!(payload["biasConfidence"].as_u64().is_some());
    assert!(payload["noiseSuppression"].is_object());
    assert!(payload["topSetups"].is_array());
    assert!(payload["noTradeZones"].is_array());
    if let Some(first_setup) = payload["topSetups"].as_array().and_then(|items| items.first()) {
        assert!(first_setup["direction"].as_str().is_some());
        assert!(first_setup["score"].as_u64().is_some());
        assert!(first_setup["entryZone"]["label"].as_str().is_some());
        assert!(first_setup["invalidation"]["priceLevel"].is_number());
        assert!(first_setup["reasons"].is_array());
    } else {
        let first_zone = payload["noTradeZones"]
            .as_array()
            .and_then(|items| items.first())
            .expect("no-trade zone");
        assert!(first_zone["reason"].as_str().is_some());
    }

    server.abort();
}

#[tokio::test]
async fn final_events_v2_expose_projection_latency_metadata() {
    let state = seeded_contract_event_state();
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
        .get(format!(
            "http://{addr}/api/final-events-v2?symbol=BTC&range=24h&limit=20"
        ))
        .send()
        .await
        .expect("final events response");

    assert_eq!(response.status(), StatusCode::OK);
    let payload: serde_json::Value = response.json().await.expect("final events json");
    assert!(payload["serverTime"].as_i64().is_some());
    assert!(payload["maxEventTs"].as_i64().is_some());
    assert!(payload["generatedAt"].as_i64().is_some());
    assert!(payload["cacheAgeSec"].as_i64().is_some());
    assert!(payload["cacheTtlSec"].as_i64().is_some());
    assert!(payload["projectionLagSec"].as_i64().is_some());

    server.abort();
}

#[tokio::test]
async fn contract_whale_latency_debug_reports_layer_and_reason() {
    std::env::set_var("OPERATOR_TOKEN", "test-operator-token");
    let state = seeded_contract_event_state();
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
        .get(format!(
            "http://{addr}/api/contract-whale/latency-debug?symbol=BTC&range=1h"
        ))
        .header("Authorization", "Bearer test-operator-token")
        .send()
        .await
        .expect("latency debug response");

    assert_eq!(response.status(), StatusCode::OK);
    let payload: serde_json::Value = response.json().await.expect("latency debug json");
    assert_eq!(payload["symbol"], "BTC");
    assert_eq!(payload["range"], "1h");
    assert!(payload["serverTime"].as_i64().is_some());
    assert!(payload["latest"]["ageSec"].as_i64().is_some());
    assert!(payload["contractEvents"]["lagVsLatestSec"]
        .as_i64()
        .is_some());
    assert!(payload["finalEventsV2"]["projectionLagSec"]
        .as_i64()
        .is_some());
    assert!(payload["flow"]["flowLagSec"].as_i64().is_some());
    assert!(payload["diagnosis"]["layer"].as_str().is_some());
    assert!(payload["diagnosis"]["reason"].as_str().is_some());

    server.abort();
    std::env::remove_var("OPERATOR_TOKEN");
}

#[tokio::test]
async fn contract_whale_latency_debug_reports_no_recent_signal_when_empty() {
    std::env::set_var("OPERATOR_TOKEN", "test-operator-token");
    let config = test_config(temp_sqlite_path("contract-whale-latency-empty"));
    let state = AppState::new(config);
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
        .get(format!(
            "http://{addr}/api/contract-whale/latency-debug?symbol=BTC&range=1h"
        ))
        .header("Authorization", "Bearer test-operator-token")
        .send()
        .await
        .expect("latency debug response");

    assert_eq!(response.status(), StatusCode::OK);
    let payload: serde_json::Value = response.json().await.expect("latency debug json");
    assert_eq!(payload["diagnosis"]["layer"], "ok");
    assert_eq!(payload["diagnosis"]["reason"], "no_recent_signal");

    server.abort();
    std::env::remove_var("OPERATOR_TOKEN");
}

#[tokio::test]
async fn contract_whale_pipeline_debug_reports_zero_history_and_stale_latest_for_btc() {
    let state = seeded_pipeline_debug_state();
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
        .get(format!(
            "http://{addr}/api/contract-whale/pipeline-debug?symbol=BTC&range=24h"
        ))
        .send()
        .await
        .expect("pipeline debug response");

    assert_eq!(response.status(), StatusCode::OK);
    let payload: serde_json::Value = response.json().await.expect("pipeline debug json");
    assert_eq!(payload["symbol"], "BTC");
    assert_eq!(payload["range"], "24h");
    assert_eq!(payload["rawFlow"]["flow1sRows"], 0);
    assert_eq!(payload["history"]["contractWhaleSignalsRows"], 0);
    assert_eq!(payload["latest"]["latestCount"], 1);
    assert_eq!(payload["latest"]["staleCount"], 1);
    assert_eq!(payload["latest"]["items"][0]["isStale"], true);
    assert_eq!(
        payload["latest"]["items"][0]["staleReason"],
        "older_than_24h"
    );

    server.abort();
}

#[tokio::test]
async fn contract_whale_raw_flow_debug_exposes_upstream_symbol_mismatch_diagnosis() {
    let state = seeded_raw_flow_debug_state("ETH-PERP");
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
        .get(format!(
            "http://{addr}/api/contract-whale/raw-flow-debug?symbol=BTC&range=24h"
        ))
        .send()
        .await
        .expect("raw flow debug response");

    assert_eq!(response.status(), StatusCode::OK);
    let payload: serde_json::Value = response.json().await.expect("raw flow debug json");
    assert_eq!(payload["symbol"], "BTC");
    assert_eq!(payload["range"], "24h");
    assert_eq!(payload["config"]["appRequestedSymbol"], "ETH-PERP");
    assert_eq!(payload["config"]["querySymbol"], "BTC");
    assert_eq!(payload["normalizer"]["connectorSymbolMismatch"], true);
    assert_eq!(payload["contractFlow1s"]["exactSymbolRows"], 0);
    assert_eq!(
        payload["diagnosis"]["primaryReason"],
        "connector_requested_symbol_mismatch"
    );

    server.abort();
}

#[tokio::test]
async fn contract_whale_raw_flow_debug_reports_persisted_btc_flow_when_rows_exist() {
    let config = test_config_with_symbol(
        temp_sqlite_path("contract-whale-raw-flow-debug-persisted"),
        "BTC-PERP",
    );
    let state = AppState::new(config);
    let store = state.contract_whale_store().expect("contract whale store");
    let now = btc_toxic_flow_monitor_rs::normalizers::trade::now_ms();
    store
        .upsert_contract_flow_buckets(&[
            ContractFlowBucket {
                ts_bucket: now - 2_000,
                exchange: "binance".to_string(),
                symbol: "BTC".to_string(),
                buy_volume_btc: 0.42,
                sell_volume_btc: 0.18,
                buy_notional_usd: 25_000.0,
                sell_notional_usd: 10_000.0,
                trade_count: 2,
                max_single_trade_btc: 0.31,
                vwap: Some(60_100.0),
                ..ContractFlowBucket::default()
            },
            ContractFlowBucket {
                ts_bucket: now - 1_000,
                exchange: "bitfinex".to_string(),
                symbol: "BTC".to_string(),
                buy_volume_btc: 0.15,
                sell_volume_btc: 0.22,
                buy_notional_usd: 9_000.0,
                sell_notional_usd: 13_000.0,
                trade_count: 3,
                max_single_trade_btc: 0.12,
                vwap: Some(60_120.0),
                ..ContractFlowBucket::default()
            },
        ])
        .expect("seed contract flow buckets");

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
        .get(format!(
            "http://{addr}/api/contract-whale/raw-flow-debug?symbol=BTC&range=24h"
        ))
        .send()
        .await
        .expect("raw flow debug response");

    assert_eq!(response.status(), StatusCode::OK);
    let payload: serde_json::Value = response.json().await.expect("raw flow debug json");
    assert_eq!(payload["contractFlow1s"]["exactSymbolRows"], 2);
    assert_eq!(payload["diagnosis"]["primaryReason"], "raw_flow_present");
    assert_eq!(payload["diagnosis"]["status"], "raw_flow_available");

    server.abort();
}

#[tokio::test]
async fn contract_whale_latest_marks_old_snapshots_stale_and_can_hide_them() {
    let state = seeded_pipeline_debug_state();
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
        .get(format!(
            "http://{addr}/api/contract-whale/latest?symbol=BTC&range=24h"
        ))
        .send()
        .await
        .expect("latest response");

    assert_eq!(response.status(), StatusCode::OK);
    let payload: serde_json::Value = response.json().await.expect("latest json");
    let items = payload["items"].as_array().expect("items array");
    assert_eq!(items.len(), 1);
    assert_eq!(items[0]["isStale"], true);
    assert_eq!(items[0]["staleReason"], "older_than_24h");
    assert!(items[0]["ageSec"].as_i64().unwrap_or_default() >= 24 * 60 * 60);

    let hidden_response = client
        .get(format!(
            "http://{addr}/api/contract-whale/latest?symbol=BTC&range=24h&hide_stale=true"
        ))
        .send()
        .await
        .expect("latest hide stale response");

    assert_eq!(hidden_response.status(), StatusCode::OK);
    let hidden_payload: serde_json::Value = hidden_response
        .json()
        .await
        .expect("latest hide stale json");
    let hidden_items = hidden_payload["items"].as_array().expect("items array");
    assert!(hidden_items.is_empty());

    server.abort();
}

fn seeded_contract_event_state() -> AppState {
    let config = test_config(temp_sqlite_path("contract-event-routes"));
    let state = AppState::new(config);
    let store = state.contract_whale_store().expect("contract whale store");
    let now = btc_toxic_flow_monitor_rs::normalizers::trade::now_ms();
    let visible = base_signal("visible", now - 5 * 60 * 1000);
    let old_visible = base_signal("old-visible", now - 26 * 60 * 60 * 1000);
    let mut hidden_price = base_signal("hidden-price", now - 15 * 60 * 1000);
    hidden_price.order_price_usd = Some(60_000.0);
    hidden_price.current_market_price_usd = Some(70_000.0);
    hidden_price.price_move_pct = Some(-0.12);

    let mut hidden_quality = base_signal("hidden-quality", now - 30 * 60 * 1000);
    hidden_quality.window_sec = 5;
    hidden_quality.total_volume_btc = 50.0;
    hidden_quality.net_volume_btc = 12.0;
    hidden_quality.total_volume = 50.0;
    hidden_quality.net_volume = 12.0;
    hidden_quality.total_notional_usd = 3_500_000.0;
    hidden_quality.dominance = 0.08;
    hidden_quality.price_move_pct = Some(0.0);
    hidden_quality.oi_change_1m_btc = None;
    hidden_quality.oi_change_5m_btc = None;
    hidden_quality.funding_rate = Some(0.0);

    store.upsert_contract_whale_signal(&visible).unwrap();
    store.upsert_contract_whale_signal(&old_visible).unwrap();
    store.upsert_contract_whale_signal(&hidden_price).unwrap();
    store.upsert_contract_whale_signal(&hidden_quality).unwrap();

    state
}

fn seeded_pipeline_debug_state() -> AppState {
    let config = test_config_with_symbol(
        temp_sqlite_path("contract-whale-pipeline-debug"),
        "BTC-PERP",
    );
    let state = AppState::new(config);
    let store = state.contract_whale_store().expect("contract whale store");
    let now = btc_toxic_flow_monitor_rs::normalizers::trade::now_ms();
    let stale_signal = base_signal("btc-stale", now - 26 * 60 * 60 * 1000);
    let fresh_eth = {
        let mut signal = base_signal("eth-fresh", now - 5 * 60 * 1000);
        signal.symbol = "ETH".to_string();
        signal.base_asset = "ETH".to_string();
        signal.quantity_unit = "ETH".to_string();
        signal.id = format!("contract-whale:ETH:15:{}:eth-fresh", signal.ts);
        signal
    };

    store.upsert_contract_whale_signal(&stale_signal).unwrap();
    store.upsert_contract_whale_signal(&fresh_eth).unwrap();

    state
}

fn seeded_raw_flow_debug_state(app_symbol: &str) -> AppState {
    let config = test_config_with_symbol(
        temp_sqlite_path("contract-whale-raw-flow-debug"),
        app_symbol,
    );
    let state = AppState::new(config);
    let store = state.contract_whale_store().expect("contract whale store");
    let now = btc_toxic_flow_monitor_rs::normalizers::trade::now_ms();
    let stale_signal = base_signal("btc-stale", now - 26 * 60 * 60 * 1000);
    store.upsert_contract_whale_signal(&stale_signal).unwrap();
    state
}

fn base_signal(suffix: &str, ts: i64) -> ContractWhaleSignal {
    reset_contract_whale_runtime_config();
    let trades = vec![
        normalize_binance_agg_trade(ts - 1_000, 70_000.0, 3_200.0, false).unwrap(),
        normalize_bitfinex_trade(ts - 1_000, 70_000.0, 430.0).unwrap(),
        normalize_binance_agg_trade(ts - 1_000, 70_000.0, 500.0, true).unwrap(),
    ];
    let buckets = aggregate_1s_buckets(&trades);
    let mut signal = detect_contract_whale_signal(
        &rolling_window_stats(&buckets, "BTC", 15, ts, Some(0.31), Some(10.4), 94)
            .expect("window stats"),
    )
    .expect("signal");
    signal.id = format!("contract-whale:BTC:15:{ts}:{suffix}");
    signal.ts = ts;
    signal
}

fn temp_sqlite_path(name: &str) -> String {
    let unique = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("time")
        .as_nanos();
    std::env::temp_dir()
        .join(format!(
            "btc-toxic-flow-{name}-{unique}-{}.sqlite",
            std::process::id()
        ))
        .to_string_lossy()
        .to_string()
}

fn test_config(sqlite_path: String) -> AppConfig {
    test_config_with_symbol(sqlite_path, "BTC-PERP")
}

fn test_config_with_symbol(sqlite_path: String, symbol: &str) -> AppConfig {
    AppConfig {
        app_env: "test".to_string(),
        read_only: true,
        api_host: "127.0.0.1".parse().expect("valid ip"),
        api_port: 0,
        symbol: symbol.to_string(),
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
        system_mode: SystemModeConfig::default(),
        contract_whale_monitor: ContractWhaleMonitorConfig {
            enabled: true,
            dry_run: true,
        },
        spot_whale_monitor: SpotWhaleMonitorConfig {
            enabled: false,
            dry_run: true,
        },
    }
}
