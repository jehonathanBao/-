mod support;

use std::{
    sync::atomic::{AtomicU64, Ordering},
    time::{Duration, Instant, SystemTime, UNIX_EPOCH},
};

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
        types::{
            ContractExchange, ContractFlowBucket, ContractOiSnapshot, ContractWhaleMarketType,
            ContractWhaleSignal, ContractWhaleSourceRole,
        },
    },
    storage::contract_whale_repo::ContractWhaleRepo,
    types::{
        market::{AggressorSide, NormalizedBook, NormalizedTrade, Venue},
        toxic::ToxicSeverity,
    },
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
async fn contract_events_can_filter_low_notional_without_deleting_hidden_metadata() {
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
            "http://{addr}/api/contract-events?symbol=BTC&range=24h&limit=50&include_hidden=true&min_notional_usd=10000000"
        ))
        .send()
        .await
        .expect("contract events response");

    assert_eq!(response.status(), StatusCode::OK);
    let payload: serde_json::Value = response.json().await.expect("contract events json");
    let items = payload["items"].as_array().expect("items array");
    assert_eq!(items.len(), 2);
    assert!(items
        .iter()
        .all(|item| item["notionalUsd"].as_f64().unwrap_or_default() >= 10_000_000.0));
    assert!(items
        .iter()
        .any(|item| item["hiddenReason"] == "price_deviation_gt_5pct"));
    assert!(!items
        .iter()
        .any(|item| item["notionalUsd"].as_f64().unwrap_or_default() < 10_000_000.0));

    server.abort();
}

#[tokio::test]
async fn contract_events_hide_btc_sub_500_volume_rows_with_explicit_reason() {
    let config = test_config(temp_sqlite_path("contract-event-low-volume-hidden"));
    let state = AppState::new(config);
    let store = state.contract_whale_store().expect("contract whale store");
    let now = btc_toxic_flow_monitor_rs::normalizers::trade::now_ms();
    let mut low_volume = base_signal("low-volume", now - 5 * 60 * 1000);
    low_volume.total_volume_btc = 499.0;
    low_volume.net_volume_btc = 430.0;
    low_volume.total_volume = 499.0;
    low_volume.net_volume = 430.0;
    low_volume.total_notional_usd = 34_930_000.0;
    low_volume.order_price_usd = Some(70_000.0);
    low_volume.current_market_price_usd = Some(70_000.0);
    low_volume.price_move_pct = Some(0.12);
    store.upsert_contract_whale_signal(&low_volume).unwrap();

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
    assert_eq!(items.len(), 1);
    assert_eq!(items[0]["isVisible"], false);
    assert_eq!(items[0]["hiddenReason"], "below_display_volume_threshold");
    assert!(items[0]["hiddenDetail"]
        .as_str()
        .is_some_and(|detail| detail.contains("500.00 BTC")));

    server.abort();
}

#[tokio::test]
async fn final_events_v2_uses_lifecycle_peak_window_volume_for_btc_display_gate() {
    let config = test_config(temp_sqlite_path("final-events-lifecycle-volume-gate"));
    let state = AppState::new(config);
    let store = state.contract_whale_store().expect("contract whale store");
    let now = btc_toxic_flow_monitor_rs::normalizers::trade::now_ms();

    let mut first = base_signal("lifecycle-first", now - 5 * 60 * 1000 - 15_000);
    first.total_volume_btc = 600.0;
    first.net_volume_btc = 520.0;
    first.total_volume = 600.0;
    first.net_volume = 520.0;
    first.total_notional_usd = 42_000_000.0;
    first.dominance = 520.0 / 600.0;
    first.price_move_pct = Some(0.10);

    let mut second = base_signal("lifecycle-second", now - 5 * 60 * 1000);
    second.total_volume_btc = 600.0;
    second.net_volume_btc = 510.0;
    second.total_volume = 600.0;
    second.net_volume = 510.0;
    second.total_notional_usd = 42_000_000.0;
    second.dominance = 510.0 / 600.0;
    second.price_move_pct = Some(0.09);

    store.upsert_contract_whale_signal(&first).unwrap();
    store.upsert_contract_whale_signal(&second).unwrap();
    let flow_start = first.ts - (first.window_sec as i64 * 1000);
    store
        .upsert_contract_flow_buckets(&[
            ContractFlowBucket {
                ts_bucket: flow_start + 1_000,
                exchange: "binance".to_string(),
                symbol: "BTC".to_string(),
                market_type: ContractWhaleMarketType::Perp,
                source_role: ContractWhaleSourceRole::Primary,
                product_id: Some("BTCUSDT".to_string()),
                buy_volume_btc: 300.0,
                sell_volume_btc: 0.0,
                buy_notional_usd: 21_000_000.0,
                sell_notional_usd: 0.0,
                trade_count: 1,
                max_single_trade_btc: 300.0,
                vwap: Some(70_000.0),
            },
            ContractFlowBucket {
                ts_bucket: second.ts,
                exchange: "binance".to_string(),
                symbol: "BTC".to_string(),
                market_type: ContractWhaleMarketType::Perp,
                source_role: ContractWhaleSourceRole::Primary,
                product_id: Some("BTCUSDT".to_string()),
                buy_volume_btc: 400.0,
                sell_volume_btc: 0.0,
                buy_notional_usd: 28_000_000.0,
                sell_notional_usd: 0.0,
                trade_count: 1,
                max_single_trade_btc: 400.0,
                vwap: Some(70_000.0),
            },
        ])
        .unwrap();

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
    let items = payload["active"]
        .as_array()
        .expect("active array")
        .iter()
        .chain(payload["closed"].as_array().expect("closed array").iter())
        .collect::<Vec<_>>();
    assert_eq!(items.len(), 1, "payload={payload}");
    assert_eq!(items[0]["displayVolumeLabel"], "事件真实换手 BTC");
    assert_eq!(items[0]["displayVolumeBtc"], 700.0);
    assert_eq!(
        items[0]["sourceSignal"]["eventLifecycle"]["uniqueTurnoverBtc"],
        700.0
    );
    assert_eq!(
        items[0]["sourceSignal"]["eventLifecycle"]["uniqueTurnoverAvailable"],
        true
    );

    server.abort();
}

#[tokio::test]
async fn contract_events_close_stale_lifecycle_events_against_live_clock() {
    let config = test_config(temp_sqlite_path("contract-event-live-clock"));
    let state = AppState::new(config);
    let store = state.contract_whale_store().expect("contract whale store");
    let now = btc_toxic_flow_monitor_rs::normalizers::trade::now_ms();
    let stale = base_signal("stale-lifecycle", now - 121_000);
    store.upsert_contract_whale_signal(&stale).unwrap();

    let app = router(state);
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0")
        .await
        .expect("bind");
    let addr = listener.local_addr().expect("addr");
    let server = tokio::spawn(async move {
        axum::serve(listener, app).await.expect("server");
    });

    let response = test_http_client()
        .get(format!(
            "http://{addr}/api/contract-events?symbol=BTC&range=24h&limit=20&include_hidden=true"
        ))
        .send()
        .await
        .expect("contract events response");

    assert_eq!(response.status(), StatusCode::OK);
    let payload: serde_json::Value = response.json().await.expect("contract events json");
    assert_eq!(payload["items"][0]["status"], "closed");

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
    assert!(payload["timeline"].is_object());
    assert_eq!(payload["timeline"]["source"], "contract_whale_signals");
    assert_eq!(payload["timeline"]["eventTs"], payload["maxEventTs"]);
    assert!(payload["timeline"]["timelineLagSec"].as_i64().is_some());

    server.abort();
}

#[tokio::test]
async fn contract_events_default_to_compact_tape_payload_without_source_signal() {
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
    let compact = client
        .get(format!(
            "http://{addr}/api/contract-events?symbol=BTC&range=24h&limit=20"
        ))
        .send()
        .await
        .expect("compact contract events response");
    assert_eq!(compact.status(), StatusCode::OK);
    let compact_payload: serde_json::Value = compact.json().await.expect("compact json");
    let compact_items = compact_payload["items"].as_array().expect("compact items");
    assert!(!compact_items.is_empty());
    assert!(
        compact_items
            .iter()
            .all(|item| item.get("sourceSignal").is_none()),
        "default contract-events tape payload must omit nested sourceSignal"
    );
    assert!(
        compact_items.iter().any(|item| {
            item.get("mainExchange").is_some()
                && item.get("discordSent").is_some()
                && item.get("liquidationSuspected").is_some()
                && item.get("flowDirection").is_some()
                && item.get("priceResponseTypeV2").is_some()
        }),
        "compact tape rows must still expose event-bar scalar and classification fields"
    );

    let full = client
        .get(format!(
            "http://{addr}/api/contract-events?symbol=BTC&range=24h&limit=20&include_source_signal=true"
        ))
        .send()
        .await
        .expect("full contract events response");
    assert_eq!(full.status(), StatusCode::OK);
    let full_payload: serde_json::Value = full.json().await.expect("full json");
    let full_items = full_payload["items"].as_array().expect("full items");
    assert!(!full_items.is_empty());
    assert!(
        full_items
            .iter()
            .all(|item| item.get("sourceSignal").is_some()),
        "include_source_signal=true must keep nested sourceSignal for detail enrichment"
    );
    assert!(
        serde_json::to_vec(&compact_payload).expect("compact bytes").len()
            < serde_json::to_vec(&full_payload).expect("full bytes").len(),
        "compact payload must be smaller than full payload"
    );

    server.abort();
}

#[tokio::test]
async fn contract_events_include_resolved_oi_context_fields() {
    let config = test_config(temp_sqlite_path("contract-events-oi-context"));
    let state = AppState::new(config);
    let store = state.contract_whale_store().expect("contract whale store");
    let now = btc_toxic_flow_monitor_rs::normalizers::trade::now_ms();
    let signal = base_signal("oi-context", now - 5 * 60 * 1000);
    let start_ts = signal.ts - (signal.window_sec as i64 * 1000);
    store.upsert_contract_whale_signal(&signal).unwrap();
    store
        .upsert_contract_oi_snapshots(&[
            ContractOiSnapshot {
                ts: start_ts,
                exchange: ContractExchange::Binance,
                symbol: "BTC".to_string(),
                oi_btc: 100_000.0,
                oi_notional_usd: None,
                ct_val_available: true,
                evidence_degraded_reason: None,
            },
            ContractOiSnapshot {
                ts: signal.ts,
                exchange: ContractExchange::Binance,
                symbol: "BTC".to_string(),
                oi_btc: 100_420.0,
                oi_notional_usd: None,
                ct_val_available: true,
                evidence_degraded_reason: None,
            },
        ])
        .unwrap();

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
    let item = payload["items"][0].clone();
    assert_eq!(item["oiContext"], "new_long_build");
    assert_eq!(item["oiContextLabel"], "新多开仓");
    assert_eq!(item["oiDeltaPct"], 0.42);
    assert_eq!(item["oiAvailable"], true);
    assert_eq!(item["oiReason"], "oi_increased_with_buy_pressure");
    assert!(
        item.get("flowDirection").and_then(|v| v.as_str()).is_some(),
        "compact tape must promote flowDirection after stripping sourceSignal"
    );
    assert!(
        item.get("priceResponseTypeV2")
            .and_then(|v| v.as_str())
            .is_some(),
        "compact tape must promote priceResponseTypeV2 after stripping sourceSignal"
    );
    assert!(item.get("sourceSignal").is_none());

    server.abort();
}

#[tokio::test]
async fn contract_events_mark_missing_oi_snapshots_as_unavailable() {
    let config = test_config(temp_sqlite_path("contract-events-oi-missing"));
    let state = AppState::new(config);
    let store = state.contract_whale_store().expect("contract whale store");
    let now = btc_toxic_flow_monitor_rs::normalizers::trade::now_ms();
    let signal = base_signal("oi-missing", now - 5 * 60 * 1000);
    store.upsert_contract_whale_signal(&signal).unwrap();

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
    let item = payload["items"][0].clone();
    assert_eq!(item["oiContext"], "oi_unavailable");
    assert_eq!(item["oiContextLabel"], "OI 不可用");
    assert!(item["oiDeltaPct"].is_null());
    assert_eq!(item["oiAvailable"], false);
    assert_eq!(item["oiReason"], "no_consistent_oi_sources");

    server.abort();
}

#[tokio::test]
async fn contract_events_mark_far_oi_snapshots_as_unavailable() {
    let config = test_config(temp_sqlite_path("contract-events-oi-gap"));
    let state = AppState::new(config);
    let store = state.contract_whale_store().expect("contract whale store");
    let now = btc_toxic_flow_monitor_rs::normalizers::trade::now_ms();
    let signal = base_signal("oi-gap", now - 5 * 60 * 1000);
    let start_ts = signal.ts - (signal.window_sec as i64 * 1000);
    store.upsert_contract_whale_signal(&signal).unwrap();
    store
        .upsert_contract_oi_snapshots(&[
            ContractOiSnapshot {
                ts: start_ts - 91_000,
                exchange: ContractExchange::Binance,
                symbol: "BTC".to_string(),
                oi_btc: 100_000.0,
                oi_notional_usd: None,
                ct_val_available: true,
                evidence_degraded_reason: None,
            },
            ContractOiSnapshot {
                ts: signal.ts - 91_000,
                exchange: ContractExchange::Binance,
                symbol: "BTC".to_string(),
                oi_btc: 100_420.0,
                oi_notional_usd: None,
                ct_val_available: true,
                evidence_degraded_reason: None,
            },
        ])
        .unwrap();

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
    let item = payload["items"][0].clone();
    assert_eq!(item["oiContext"], "oi_unavailable");
    assert_eq!(item["oiAvailable"], false);
    assert_eq!(item["oiReason"], "oi_snapshot_gap_too_large");

    server.abort();
}

#[tokio::test]
async fn final_events_v2_include_resolved_oi_context_fields() {
    let config = test_config(temp_sqlite_path("final-events-v2-oi-context"));
    let state = AppState::new(config);
    let store = state.contract_whale_store().expect("contract whale store");
    let now = btc_toxic_flow_monitor_rs::normalizers::trade::now_ms();
    let signal = base_signal("final-events-oi-context", now - 5 * 60 * 1000);
    let start_ts = signal.ts - (signal.window_sec as i64 * 1000);
    store.upsert_contract_whale_signal(&signal).unwrap();
    store
        .upsert_contract_oi_snapshots(&[
            ContractOiSnapshot {
                ts: start_ts,
                exchange: ContractExchange::Binance,
                symbol: "BTC".to_string(),
                oi_btc: 100_000.0,
                oi_notional_usd: None,
                ct_val_available: true,
                evidence_degraded_reason: None,
            },
            ContractOiSnapshot {
                ts: signal.ts,
                exchange: ContractExchange::Binance,
                symbol: "BTC".to_string(),
                oi_btc: 100_420.0,
                oi_notional_usd: None,
                ct_val_available: true,
                evidence_degraded_reason: None,
            },
        ])
        .unwrap();

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
        .expect("final events v2 response");

    assert_eq!(response.status(), StatusCode::OK);
    let payload: serde_json::Value = response.json().await.expect("final events v2 json");
    let item = payload["active"]
        .as_array()
        .expect("active events")
        .iter()
        .chain(payload["closed"].as_array().expect("closed events").iter())
        .next()
        .expect("final event");
    assert_eq!(item["oiContext"], "new_long_build");
    assert_eq!(item["oiContextLabel"], "新多开仓");
    assert_eq!(item["oiDeltaPct"], 0.42);
    assert_eq!(item["oiAvailable"], true);
    assert_eq!(item["oiReason"], "oi_increased_with_buy_pressure");

    server.abort();
}

#[tokio::test]
async fn contract_whale_timeline_route_reports_single_canonical_event_clock() {
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
            "http://{addr}/api/contract-whale/timeline?symbol=BTC&range=24h"
        ))
        .send()
        .await
        .expect("timeline response");

    assert_eq!(response.status(), StatusCode::OK);
    let payload: serde_json::Value = response.json().await.expect("timeline json");
    assert_eq!(payload["symbol"], "BTC");
    assert_eq!(payload["range"], "24h");
    assert_eq!(payload["source"], "contract_whale_signals");
    assert!(payload["eventTs"].as_i64().is_some());
    assert!(payload["processedTs"].as_i64().is_some());
    assert!(payload["servedTs"].as_i64().is_some());
    assert!(payload["timelineLagSec"].as_i64().is_some());
    assert_eq!(
        payload["views"]["history"]["maxEventTs"],
        payload["eventTs"]
    );
    assert_eq!(
        payload["views"]["finalEventsV2"]["maxEventTs"],
        payload["eventTs"]
    );
    assert!(payload["views"]["latest"]["driftVsCanonicalSec"]
        .as_i64()
        .is_some());

    server.abort();
}

#[tokio::test]
async fn contract_whale_timeline_reports_degraded_when_latest_query_fails() {
    let state = seeded_contract_event_state();
    let store = state.contract_whale_store().expect("contract whale store");
    store
        .with_connection(|conn| {
            conn.execute("DROP TABLE contract_whale_signals", [])?;
            Ok(())
        })
        .expect("drop signal table");

    let app = router(state);
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0")
        .await
        .expect("bind");
    let addr = listener.local_addr().expect("addr");
    let server = tokio::spawn(async move {
        axum::serve(listener, app).await.expect("server");
    });

    let response = test_http_client()
        .get(format!(
            "http://{addr}/api/contract-whale/timeline?symbol=BTC&range=24h"
        ))
        .send()
        .await
        .expect("timeline response");

    assert_eq!(response.status(), StatusCode::INTERNAL_SERVER_ERROR);
    let payload: serde_json::Value = response.json().await.expect("timeline error json");
    assert_eq!(payload["dataState"], "degraded");
    assert_eq!(payload["errorCode"], "contract_history_query_failed");
    assert_eq!(payload["lastKnownDataAvailable"], true);

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
    assert_eq!(payload["finalEventsV2"]["activeCount"], 0);
    assert_eq!(payload["finalEventsV2"]["closedCount"], 1);
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
    assert!(payload["timeline"].is_object());
    assert!(payload["timeline"]["timelineLagSec"].as_i64().is_some());

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
    if let Some(first_setup) = payload["topSetups"]
        .as_array()
        .and_then(|items| items.first())
    {
        assert_eq!(first_setup["semanticType"], "decision_support");
        assert!(first_setup["directionBias"].as_str().is_some());
        assert!(first_setup["score"].as_u64().is_some());
        assert!(first_setup["pressureZone"]["label"].as_str().is_some());
        assert!(first_setup["riskBoundary"]["priceLevel"].is_number());
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
async fn contract_whale_intelligence_terminal_route_exposes_signal_compression_trade_ideas_and_risk_context(
) {
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
            "http://{addr}/api/contract-whale/intelligence-terminal?symbol=BTC&range=24h"
        ))
        .send()
        .await
        .expect("intelligence terminal response");

    assert_eq!(response.status(), StatusCode::OK);
    let payload: serde_json::Value = response.json().await.expect("intelligence terminal json");
    assert_eq!(payload["symbol"], "BTC");
    assert!(payload["timestamp"].as_i64().is_some());
    assert!(payload["marketRegime"].is_object());
    assert!(payload["liquidityBehaviors"].is_array());
    assert!(payload["rankedEvents"].is_array());
    assert!(payload["opportunityMap"].is_array());
    assert!(payload["noiseSuppression"].is_object());
    assert!(payload["signalCompression"].is_object());
    assert!(payload["tradeIdeas"].is_array());
    assert!(payload["riskContext"].is_object());
    if let Some(first_idea) = payload["tradeIdeas"]
        .as_array()
        .and_then(|items| items.first())
    {
        assert_eq!(first_idea["semanticType"], "decision_support");
        assert!(first_idea["directionBias"].as_str().is_some());
        assert!(first_idea["pressureZone"]["label"].as_str().is_some());
        assert!(first_idea["riskBoundary"]["priceLevel"].is_number());
    }
    assert_eq!(payload["riskContext"]["semanticType"], "risk_override");
    assert!(payload.get("topSetups").is_none());

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
    assert!(payload["timeline"].is_object());
    assert_eq!(payload["timeline"]["source"], "contract_whale_signals");
    assert_eq!(payload["timeline"]["eventTs"], payload["maxEventTs"]);

    server.abort();
}

#[tokio::test]
async fn final_events_v2_reuses_recent_projection_within_cache_ttl() {
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
    let first = client
        .get(format!(
            "http://{addr}/api/final-events-v2?symbol=BTC&range=24h&limit=20"
        ))
        .send()
        .await
        .expect("first final events response");
    assert_eq!(first.status(), StatusCode::OK);
    let first_payload: serde_json::Value = first.json().await.expect("first final events json");
    let first_generated_at = first_payload["generatedAt"]
        .as_i64()
        .expect("first generatedAt");

    tokio::time::sleep(std::time::Duration::from_millis(25)).await;

    let second = client
        .get(format!(
            "http://{addr}/api/final-events-v2?symbol=BTC&range=24h&limit=20"
        ))
        .send()
        .await
        .expect("second final events response");
    assert_eq!(second.status(), StatusCode::OK);
    let second_payload: serde_json::Value = second.json().await.expect("second final events json");
    let second_generated_at = second_payload["generatedAt"]
        .as_i64()
        .expect("second generatedAt");

    assert_eq!(second_generated_at, first_generated_at);
    assert!(second_payload["cacheAgeSec"].as_i64().unwrap_or_default() >= 0);
    assert_eq!(second_payload["cacheTtlSec"], 30);
    assert!(second_payload["timeline"].is_object());

    server.abort();
}

#[tokio::test(flavor = "current_thread")]
async fn contract_event_cache_stays_fresh_across_frontend_poll_interval() {
    let state = seeded_contract_event_state();
    let app = router(state.clone());
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0")
        .await
        .expect("bind");
    let addr = listener.local_addr().expect("addr");
    let server = tokio::spawn(async move {
        axum::serve(listener, app).await.expect("server");
    });
    let client = test_http_client();
    let url = format!("http://{addr}/api/contract-events?symbol=BTC&range=24h&limit=20");

    let first = client.get(&url).send().await.expect("first response");
    assert_eq!(first.status(), StatusCode::OK);
    state
        .expire_contract_event_projection_cache_for_tests(Duration::from_secs(16))
        .await;

    let second = client.get(&url).send().await.expect("second response");
    assert_eq!(second.status(), StatusCode::OK);
    let payload: serde_json::Value = second.json().await.expect("second json");
    assert_eq!(payload["dataState"], "fresh");
    assert_eq!(payload["degraded"], false);
    assert!(
        payload["cacheTtlSec"].as_i64().unwrap_or_default() > 15,
        "cache TTL must exceed the frontend's 15-second event polling interval"
    );

    server.abort();
}

#[test]
fn summary_and_latest_routes_use_nonblocking_singleflight_projection_runtime() {
    const SOURCE: &str = include_str!("../src/api/contract_whale_routes.rs");

    let summary_route = SOURCE
        .split_once("pub async fn contract_whale_summary_route")
        .and_then(|(_, rest)| rest.split_once("fn log_summary_access"))
        .map(|(route, _)| route)
        .expect("summary route source");
    let latest_route = SOURCE
        .split_once("pub async fn contract_whale_latest_route")
        .and_then(|(_, rest)| rest.split_once("pub async fn contract_whale_outcome_summary_route"))
        .map(|(route, _)| route)
        .expect("latest route source");

    assert!(
        summary_route.contains("contract_whale_projection_runtime"),
        "summary route must use the nonblocking projection runtime"
    );
    assert!(
        latest_route.contains("contract_whale_projection_runtime"),
        "latest route must use the nonblocking projection runtime"
    );
}

#[tokio::test(flavor = "current_thread")]
async fn slow_contract_projection_does_not_delay_summary_or_latest() {
    let state = seeded_contract_event_state();
    state.set_contract_event_projection_delay_for_tests(Duration::from_millis(1_200));
    state.set_contract_event_projection_wait_budget_for_tests(Duration::from_secs(2));
    let app = router(state.clone());
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0")
        .await
        .expect("bind");
    let addr = listener.local_addr().expect("addr");
    let server = tokio::spawn(async move {
        axum::serve(listener, app).await.expect("server");
    });
    let client = test_http_client();
    let heavy_client = client.clone();
    let heavy = tokio::spawn(async move {
        heavy_client
            .get(format!(
                "http://{addr}/api/contract-events?symbol=BTC&range=24h&limit=20"
            ))
            .send()
            .await
            .expect("contract events response")
    });
    wait_for_projection_running(&state).await;

    let light_started = Instant::now();
    let (summary, latest) = tokio::join!(
        client
            .get(format!(
                "http://{addr}/api/contract-whale/summary?symbol=BTC"
            ))
            .send(),
        client
            .get(format!(
                "http://{addr}/api/contract-whale/latest?symbol=BTC&limit=50"
            ))
            .send(),
    );
    let light_elapsed = light_started.elapsed();

    assert_eq!(summary.expect("summary response").status(), StatusCode::OK);
    assert_eq!(latest.expect("latest response").status(), StatusCode::OK);
    assert!(
        light_elapsed < Duration::from_millis(800),
        "light routes waited {light_elapsed:?} for the heavy projection"
    );
    assert!(
        !heavy.is_finished(),
        "heavy projection unexpectedly finished first"
    );
    assert_eq!(heavy.await.expect("heavy task").status(), StatusCode::OK);

    server.abort();
}

#[tokio::test(flavor = "current_thread")]
async fn contract_retention_status_returns_immediately_and_coalesces_slow_refreshes() {
    let state = seeded_contract_event_state();
    state.set_contract_retention_delay_for_tests(Duration::from_millis(1_200));
    let app = router(state.clone());
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0")
        .await
        .expect("bind");
    let addr = listener.local_addr().expect("addr");
    let server = tokio::spawn(async move {
        axum::serve(listener, app).await.expect("server");
    });
    let client = test_http_client();
    let url = format!("http://{addr}/api/contract-retention-status");

    let first_started = Instant::now();
    let first = client
        .get(&url)
        .send()
        .await
        .expect("first retention response");
    let first_elapsed = first_started.elapsed();

    assert_eq!(first.status(), StatusCode::OK);
    assert!(
        first_elapsed < Duration::from_millis(250),
        "retention route waited for background refresh: {first_elapsed:?}"
    );
    let first_payload: serde_json::Value = first.json().await.expect("first retention json");
    assert_eq!(first_payload["dataState"], "degraded");
    assert_eq!(
        first_payload["errorCode"],
        "contract_retention_refresh_in_progress"
    );
    assert_eq!(first_payload["lastKnownDataAvailable"], false);

    wait_for_retention_running(&state).await;
    let responses = futures_util::future::join_all((0..8).map(|_| client.get(&url).send())).await;
    for response in responses {
        assert_eq!(
            response.expect("coalesced retention response").status(),
            StatusCode::OK
        );
    }
    assert_eq!(state.contract_retention_stats_for_tests().started, 1);

    wait_for_retention_idle(&state).await;
    let cached = client
        .get(&url)
        .send()
        .await
        .expect("cached retention response");
    assert_eq!(cached.status(), StatusCode::OK);
    let cached_payload: serde_json::Value = cached.json().await.expect("cached retention json");
    assert_eq!(cached_payload["dataState"], "fresh");
    assert_eq!(cached_payload["degraded"], false);
    assert_eq!(cached_payload["lastKnownDataAvailable"], true);
    assert!(cached_payload["generatedAt"].as_i64().is_some());
    assert!(cached_payload["tables"]["contractWhaleSignals"]["rowCount"]
        .as_i64()
        .is_some());

    server.abort();
}

#[tokio::test(flavor = "current_thread")]
async fn slow_contract_retention_refresh_does_not_delay_summary_or_latest() {
    let state = seeded_contract_event_state();
    state.set_contract_retention_delay_for_tests(Duration::from_millis(1_200));
    let app = router(state.clone());
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0")
        .await
        .expect("bind");
    let addr = listener.local_addr().expect("addr");
    let server = tokio::spawn(async move {
        axum::serve(listener, app).await.expect("server");
    });
    let client = test_http_client();

    let retention = client
        .get(format!("http://{addr}/api/contract-retention-status"))
        .send()
        .await
        .expect("retention response");
    assert_eq!(retention.status(), StatusCode::OK);
    wait_for_retention_running(&state).await;

    let light_started = Instant::now();
    let (summary, latest) = tokio::join!(
        client
            .get(format!(
                "http://{addr}/api/contract-whale/summary?symbol=BTC"
            ))
            .send(),
        client
            .get(format!(
                "http://{addr}/api/contract-whale/latest?symbol=BTC&limit=50"
            ))
            .send(),
    );
    let light_elapsed = light_started.elapsed();

    assert_eq!(summary.expect("summary response").status(), StatusCode::OK);
    assert_eq!(latest.expect("latest response").status(), StatusCode::OK);
    assert!(
        light_elapsed < Duration::from_millis(800),
        "light routes waited {light_elapsed:?} for retention refresh"
    );
    wait_for_retention_idle(&state).await;

    server.abort();
}

#[tokio::test(flavor = "current_thread")]
async fn equivalent_contract_event_requests_execute_projection_once() {
    let state = seeded_contract_event_state();
    state.set_contract_event_projection_delay_for_tests(Duration::from_millis(150));
    state.set_contract_event_projection_wait_budget_for_tests(Duration::from_secs(1));
    let app = router(state.clone());
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0")
        .await
        .expect("bind");
    let addr = listener.local_addr().expect("addr");
    let server = tokio::spawn(async move {
        axum::serve(listener, app).await.expect("server");
    });
    let client = test_http_client();
    let mut requests = Vec::new();
    for _ in 0..8 {
        let client = client.clone();
        requests.push(tokio::spawn(async move {
            client
                .get(format!(
                    "http://{addr}/api/contract-events?symbol=BTC&range=24h&limit=20"
                ))
                .send()
                .await
                .expect("contract events response")
        }));
    }

    for request in requests {
        assert_eq!(
            request.await.expect("request task").status(),
            StatusCode::OK
        );
    }
    let stats = state.contract_event_projection_stats_for_tests();
    assert_eq!(stats.started, 1);
    assert_eq!(stats.max_running, 1);

    server.abort();
}

#[tokio::test(flavor = "current_thread")]
async fn equivalent_summary_and_latest_requests_execute_each_projection_once() {
    let state = seeded_contract_event_state();
    state.set_contract_whale_projection_delay_for_tests(Duration::from_millis(150));
    state.set_contract_whale_projection_wait_budget_for_tests(Duration::from_secs(1));
    let app = router(state.clone());
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0")
        .await
        .expect("bind");
    let addr = listener.local_addr().expect("addr");
    let server = tokio::spawn(async move {
        axum::serve(listener, app).await.expect("server");
    });
    let client = test_http_client();
    let mut requests = Vec::new();
    for _ in 0..8 {
        let summary_client = client.clone();
        requests.push(tokio::spawn(async move {
            summary_client
                .get(format!(
                    "http://{addr}/api/contract-whale/summary?symbol=BTC"
                ))
                .send()
                .await
                .expect("summary response")
        }));
        let latest_client = client.clone();
        requests.push(tokio::spawn(async move {
            latest_client
                .get(format!(
                    "http://{addr}/api/contract-whale/latest?symbol=BTC&limit=50"
                ))
                .send()
                .await
                .expect("latest response")
        }));
    }

    for request in requests {
        assert_eq!(
            request.await.expect("request task").status(),
            StatusCode::OK
        );
    }
    let stats = state.contract_whale_projection_stats_for_tests();
    assert_eq!(stats.started, 2);
    assert_eq!(stats.max_running, 2);

    server.abort();
}

#[tokio::test(flavor = "current_thread")]
async fn contract_events_timeout_returns_structured_503_without_cache() {
    let state = seeded_contract_event_state();
    state.set_contract_event_projection_delay_for_tests(Duration::from_millis(300));
    state.set_contract_event_projection_wait_budget_for_tests(Duration::from_millis(50));
    let app = router(state.clone());
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0")
        .await
        .expect("bind");
    let addr = listener.local_addr().expect("addr");
    let server = tokio::spawn(async move {
        axum::serve(listener, app).await.expect("server");
    });

    let response = test_http_client()
        .get(format!(
            "http://{addr}/api/contract-events?symbol=BTC&range=24h&limit=20"
        ))
        .send()
        .await
        .expect("contract events response");

    assert_eq!(response.status(), StatusCode::SERVICE_UNAVAILABLE);
    assert_eq!(
        response
            .headers()
            .get(reqwest::header::RETRY_AFTER)
            .and_then(|value| value.to_str().ok()),
        Some("2")
    );
    let payload: serde_json::Value = response.json().await.expect("503 json");
    assert_eq!(payload["dataState"], "degraded");
    assert_eq!(payload["degraded"], true);
    assert_eq!(payload["errorCode"], "contract_projection_timeout");
    assert_eq!(payload["lastKnownDataAvailable"], false);
    assert_eq!(payload["retryAfterMs"], 2_000);
    wait_for_projection_idle(&state).await;

    server.abort();
}

#[tokio::test(flavor = "current_thread")]
async fn contract_events_timeout_serves_stale_payload() {
    let state = seeded_contract_event_state();
    state.set_contract_event_projection_wait_budget_for_tests(Duration::from_secs(2));
    let app = router(state.clone());
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0")
        .await
        .expect("bind");
    let addr = listener.local_addr().expect("addr");
    let server = tokio::spawn(async move {
        axum::serve(listener, app).await.expect("server");
    });
    let client = test_http_client();
    let url = format!("http://{addr}/api/contract-events?symbol=BTC&range=24h&limit=20");

    let first = client.get(&url).send().await.expect("first response");
    assert_eq!(first.status(), StatusCode::OK);
    let first_payload: serde_json::Value = first.json().await.expect("first json");
    let first_ids = first_payload["items"]
        .as_array()
        .expect("first items")
        .iter()
        .map(|item| item["eventId"].clone())
        .collect::<Vec<_>>();
    assert!(!first_ids.is_empty());

    state
        .expire_contract_event_projection_cache_for_tests(Duration::from_secs(45))
        .await;
    state.set_contract_event_projection_wait_budget_for_tests(Duration::from_millis(50));
    state.set_contract_event_projection_delay_for_tests(Duration::from_millis(300));
    let stale = client.get(&url).send().await.expect("stale response");

    assert_eq!(stale.status(), StatusCode::OK);
    let stale_payload: serde_json::Value = stale.json().await.expect("stale json");
    let stale_ids = stale_payload["items"]
        .as_array()
        .expect("stale items")
        .iter()
        .map(|item| item["eventId"].clone())
        .collect::<Vec<_>>();
    assert_eq!(stale_ids, first_ids);
    assert_eq!(stale_payload["dataState"], "stale");
    assert_eq!(stale_payload["degraded"], true);
    assert_eq!(
        stale_payload["errorCode"],
        "contract_projection_refresh_in_progress"
    );
    assert_eq!(stale_payload["lastKnownDataAvailable"], true);
    wait_for_projection_idle(&state).await;

    server.abort();
}

#[tokio::test]
async fn contract_events_and_final_events_v2_share_same_timeline_event_ts() {
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
    let contract_events = client
        .get(format!(
            "http://{addr}/api/contract-events?symbol=BTC&range=24h&limit=20"
        ))
        .send()
        .await
        .expect("contract events response");
    let final_events = client
        .get(format!(
            "http://{addr}/api/final-events-v2?symbol=BTC&range=24h&limit=20"
        ))
        .send()
        .await
        .expect("final events response");

    assert_eq!(contract_events.status(), StatusCode::OK);
    assert_eq!(final_events.status(), StatusCode::OK);
    let contract_events_payload: serde_json::Value =
        contract_events.json().await.expect("contract events json");
    let final_events_payload: serde_json::Value =
        final_events.json().await.expect("final events json");
    assert_eq!(
        contract_events_payload["timeline"]["eventTs"],
        final_events_payload["timeline"]["eventTs"]
    );
    assert_eq!(
        contract_events_payload["timeline"]["source"],
        final_events_payload["timeline"]["source"]
    );

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
    assert_eq!(
        payload["runtime"]["oiResolver"]["queryMode"],
        "batch_per_exchange"
    );

    server.abort();
}

#[tokio::test]
async fn contract_whale_pipeline_debug_reports_history_query_failure_instead_of_empty_history() {
    let state = seeded_pipeline_debug_state();
    let store = state.contract_whale_store().expect("contract whale store");
    store
        .with_connection(|conn| {
            conn.execute("DROP TABLE contract_whale_signals", [])?;
            Ok(())
        })
        .expect("drop signal table");

    let app = router(state);
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0")
        .await
        .expect("bind");
    let addr = listener.local_addr().expect("addr");
    let server = tokio::spawn(async move {
        axum::serve(listener, app).await.expect("server");
    });

    let response = test_http_client()
        .get(format!(
            "http://{addr}/api/contract-whale/pipeline-debug?symbol=BTC&range=24h"
        ))
        .send()
        .await
        .expect("pipeline debug response");

    assert_eq!(response.status(), StatusCode::OK);
    let payload: serde_json::Value = response.json().await.expect("pipeline debug json");
    assert_eq!(payload["history"]["contractWhaleSignalsRows"], 0);
    assert_eq!(payload["error"], "history_query_failed");

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
async fn contract_whale_raw_flow_debug_reports_persistence_query_failure() {
    let state = seeded_raw_flow_debug_state("BTC-PERP");
    let store = state.contract_whale_store().expect("contract whale store");
    store
        .with_connection(|conn| {
            conn.execute("DROP TABLE contract_flow_1s", [])?;
            Ok(())
        })
        .expect("drop flow table");

    let app = router(state);
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0")
        .await
        .expect("bind");
    let addr = listener.local_addr().expect("addr");
    let server = tokio::spawn(async move {
        axum::serve(listener, app).await.expect("server");
    });

    let response = test_http_client()
        .get(format!(
            "http://{addr}/api/contract-whale/raw-flow-debug?symbol=BTC&range=24h"
        ))
        .send()
        .await
        .expect("raw flow debug response");

    assert_eq!(response.status(), StatusCode::OK);
    let payload: serde_json::Value = response.json().await.expect("raw flow debug json");
    assert_eq!(payload["contractFlow1s"]["exactSymbolRows"], 0);
    assert_eq!(payload["error"], "raw_flow_persistence_query_failed");

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

#[tokio::test]
async fn contract_whale_latest_does_not_replay_stale_persisted_items_when_live_flow_is_authoritative(
) {
    let config = test_config_with_symbol(
        temp_sqlite_path("contract-whale-latest-prefers-live"),
        "ETH-PERP",
    );
    let state = AppState::new(config);
    let store = state.contract_whale_store().expect("contract whale store");
    let now = btc_toxic_flow_monitor_rs::normalizers::trade::now_ms();
    let stale_signal = base_signal("btc-stale", now - 26 * 60 * 60 * 1000);
    store.upsert_contract_whale_signal(&stale_signal).unwrap();
    seed_live_btc_flow_for_tests(&state, now);

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
    assert!(
        items.is_empty(),
        "authoritative live flow should not replay stale persisted latest rows; payload={payload}"
    );
    assert_eq!(payload["staleCount"], 0);
    assert!(payload["maxTs"].is_null());

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

async fn wait_for_projection_running(state: &AppState) {
    tokio::time::timeout(Duration::from_secs(1), async {
        while state.contract_event_projection_stats_for_tests().running == 0 {
            tokio::task::yield_now().await;
        }
    })
    .await
    .expect("projection did not start");
}

async fn wait_for_projection_idle(state: &AppState) {
    tokio::time::timeout(Duration::from_secs(2), async {
        while state.contract_event_projection_stats_for_tests().in_flight > 0 {
            tokio::task::yield_now().await;
        }
    })
    .await
    .expect("projection did not become idle");
}

async fn wait_for_retention_running(state: &AppState) {
    tokio::time::timeout(Duration::from_secs(1), async {
        while state.contract_retention_stats_for_tests().running == 0 {
            tokio::task::yield_now().await;
        }
    })
    .await
    .expect("retention refresh did not start");
}

async fn wait_for_retention_idle(state: &AppState) {
    tokio::time::timeout(Duration::from_secs(3), async {
        while state.contract_retention_stats_for_tests().running > 0 {
            tokio::time::sleep(Duration::from_millis(10)).await;
        }
    })
    .await
    .expect("retention refresh did not finish");
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

fn seed_live_btc_flow_for_tests(state: &AppState, now: i64) {
    let flow_service = state.flow_service_for_tests();
    for (venue, earlier_mid, later_mid) in [
        (Venue::Binance, 60_000.0, 60_360.0),
        (Venue::Bitfinex, 60_010.0, 60_380.0),
    ] {
        flow_service.add_book_for_tests(NormalizedBook {
            venue,
            symbol: "BTC".to_string(),
            ts: now - 4_500,
            best_bid: earlier_mid - 5.0,
            best_ask: earlier_mid + 5.0,
            bids: vec![(earlier_mid - 5.0, 500.0)],
            asks: vec![(earlier_mid + 5.0, 500.0)],
            mid: earlier_mid,
            spread_bps: 1.7,
            bid_depth_btc_10bps: 500.0,
            ask_depth_btc_10bps: 500.0,
            bid_depth_usd_10bps: earlier_mid * 500.0,
            ask_depth_usd_10bps: earlier_mid * 500.0,
            imbalance_10bps: 0.0,
        });
        flow_service.add_book_for_tests(NormalizedBook {
            venue,
            symbol: "BTC".to_string(),
            ts: now - 1_000,
            best_bid: later_mid - 5.0,
            best_ask: later_mid + 5.0,
            bids: vec![(later_mid - 5.0, 500.0)],
            asks: vec![(later_mid + 5.0, 500.0)],
            mid: later_mid,
            spread_bps: 1.7,
            bid_depth_btc_10bps: 500.0,
            ask_depth_btc_10bps: 500.0,
            bid_depth_usd_10bps: later_mid * 500.0,
            ask_depth_usd_10bps: later_mid * 500.0,
            imbalance_10bps: 0.0,
        });
    }

    for trade in [
        NormalizedTrade {
            venue: Venue::Binance,
            symbol: "BTC".to_string(),
            ts: now - 3_500,
            price: 60_250.0,
            size_btc: 1_150.0,
            size_usd: 69_287_500.0,
            aggressor_side: AggressorSide::Buy,
            trade_id: Some("live-btc-binance-1".to_string()),
        },
        NormalizedTrade {
            venue: Venue::Bitfinex,
            symbol: "BTC".to_string(),
            ts: now - 2_500,
            price: 60_310.0,
            size_btc: 980.0,
            size_usd: 59_103_800.0,
            aggressor_side: AggressorSide::Buy,
            trade_id: Some("live-btc-bitfinex-1".to_string()),
        },
        NormalizedTrade {
            venue: Venue::Binance,
            symbol: "BTC".to_string(),
            ts: now - 1_500,
            price: 60_360.0,
            size_btc: 910.0,
            size_usd: 54_927_600.0,
            aggressor_side: AggressorSide::Buy,
            trade_id: Some("live-btc-binance-2".to_string()),
        },
        NormalizedTrade {
            venue: Venue::Bitfinex,
            symbol: "BTC".to_string(),
            ts: now - 900,
            price: 60_340.0,
            size_btc: 120.0,
            size_usd: 7_240_800.0,
            aggressor_side: AggressorSide::Sell,
            trade_id: Some("live-btc-bitfinex-2".to_string()),
        },
    ] {
        flow_service.add_trade_for_tests(trade);
    }
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
    static TEMP_PATH_SEQUENCE: AtomicU64 = AtomicU64::new(0);

    let unique = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("time")
        .as_nanos();
    let sequence = TEMP_PATH_SEQUENCE.fetch_add(1, Ordering::Relaxed);
    std::env::temp_dir()
        .join(format!(
            "btc-toxic-flow-{name}-{unique}-{sequence}-{}.sqlite",
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
