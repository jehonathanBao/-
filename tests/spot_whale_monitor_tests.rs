use std::{
    path::PathBuf,
    time::{SystemTime, UNIX_EPOCH},
};

use btc_toxic_flow_monitor_rs::{
    normalizers::trade::now_ms,
    spot_whale_monitor::{
        config::SpotWhaleRuntimeConfig,
        detector::{detect_spot_whale_signal_with_config, discord_gate},
        normalizer::{
            normalize_binance_spot_trade, normalize_bitfinex_trade_value,
            normalize_coinbase_market_trades_json, BinanceSpotAggTrade,
        },
        service::{
            decode_spot_history_cursor, encode_spot_history_cursor, SpotWhaleQuery,
            SpotWhaleService,
        },
        types::{
            SpotExchange, SpotExchangeContribution, SpotTrade, SpotTradeSide, SpotWhaleSeverity,
            SpotWhaleSignalType, SpotWhaleWindowStats,
        },
    },
    storage::{
        spot_whale_repo::{
            SpotWhaleRepo, SpotWhaleSignalQuery, SPOT_WHALE_PERMANENT_NET_DIRECTION_THRESHOLD_BASE,
        },
        SqliteStore,
    },
};

#[test]
fn spot_normalizers_map_taker_direction_and_units() {
    let maker_buy = normalize_binance_spot_trade(BinanceSpotAggTrade {
        s: "BTCUSDT".to_string(),
        p: "70000".to_string(),
        q: "1.25".to_string(),
        trade_time: Some(1_700_000_000_000),
        event_time: None,
        m: true,
        a: Some(serde_json::json!(42)),
        t: None,
    })
    .expect("binance maker buy");
    let taker_buy = normalize_binance_spot_trade(BinanceSpotAggTrade {
        s: "ETHUSDT".to_string(),
        p: "3500".to_string(),
        q: "12".to_string(),
        trade_time: Some(1_700_000_000_001),
        event_time: None,
        m: false,
        a: None,
        t: Some(serde_json::json!(43)),
    })
    .expect("binance taker buy");

    assert_eq!(maker_buy.exchange, SpotExchange::Binance);
    assert_eq!(maker_buy.symbol, "BTC");
    assert_eq!(maker_buy.side, SpotTradeSide::Sell);
    assert_eq!(maker_buy.notional_usd, 87_500.0);
    assert_eq!(taker_buy.symbol, "ETH");
    assert_eq!(taker_buy.side, SpotTradeSide::Buy);
    assert_eq!(taker_buy.notional_usd, 42_000.0);

    let coinbase_payload = serde_json::json!({
        "channel": "market_trades",
        "events": [{
            "type": "update",
            "trades": [
                {"trade_id": "1", "product_id": "BTC-USD", "price": "70010", "size": "0.5", "side": "BUY", "time": "2026-06-08T00:00:00Z"},
                {"trade_id": "2", "product_id": "ETH-USD", "price": "3501", "size": "2.0", "side": "SELL", "time": "2026-06-08T00:00:00Z"}
            ]
        }]
    });
    let trades = normalize_coinbase_market_trades_json(&coinbase_payload);

    assert_eq!(trades.len(), 2);
    assert_eq!(trades[0].exchange, SpotExchange::Coinbase);
    assert_eq!(trades[0].side, SpotTradeSide::Sell);
    assert_eq!(trades[1].symbol, "ETH");
    assert_eq!(trades[1].side, SpotTradeSide::Buy);

    let bitfinex_sell = normalize_bitfinex_trade_value(
        "tBTCUSD",
        &serde_json::json!([88, 1_700_000_000_002_i64, -0.75, 70_000.0]),
    )
    .expect("bitfinex spot trade");
    assert_eq!(bitfinex_sell.exchange, SpotExchange::Bitfinex);
    assert_eq!(bitfinex_sell.symbol, "BTC");
    assert_eq!(bitfinex_sell.side, SpotTradeSide::Sell);
    assert_eq!(bitfinex_sell.notional_usd, 52_500.0);
}

#[test]
fn spot_detector_generates_critical_multi_exchange_buy_signal() {
    let config = SpotWhaleRuntimeConfig::default();
    let stats = high_conviction_stats();

    let signal = detect_spot_whale_signal_with_config(&stats, &config).expect("spot signal");

    assert_eq!(signal.symbol, "BTC");
    assert_eq!(signal.signal_type, SpotWhaleSignalType::SpotAggressiveBuy);
    assert_eq!(signal.severity, SpotWhaleSeverity::Critical);
    assert!(signal.score >= 80);
    assert!(signal.discord_eligible);
    assert_eq!(signal.discord_reason, "critical_or_s_gate");
    assert!(signal.read_only);
    assert!(signal.analysis_only);
    assert!(!signal.execution_enabled);
}

#[test]
fn spot_discord_gate_rejects_medium_and_low_quality() {
    let (eligible, reason) = discord_gate(SpotWhaleSeverity::Medium, 95, true, 95);
    assert!(!eligible);
    assert_eq!(reason, "medium_or_low_display_only");

    let (eligible, reason) = discord_gate(SpotWhaleSeverity::Critical, 95, true, 69);
    assert!(!eligible);
    assert_eq!(reason, "data_quality_display_only");
}

#[test]
fn spot_detector_rejects_medium_without_directional_quality_and_price_response() {
    let config = SpotWhaleRuntimeConfig::default();
    let mut stats = high_conviction_stats();
    stats.total_volume_base = 190.0;
    stats.net_volume_base = 5.0;
    stats.dominance = 5.0 / 190.0;
    stats.total_notional_usd = 16_000_000.0;
    stats.price_move_pct = Some(0.30);
    stats.dynamic_multiple = Some(5.5);

    assert!(detect_spot_whale_signal_with_config(&stats, &config).is_none());

    stats.dominance = 0.60;
    stats.net_volume_base = 114.0;
    stats.data_quality = 55;
    assert!(detect_spot_whale_signal_with_config(&stats, &config).is_none());
}

#[test]
fn spot_detector_does_not_infer_absorption_or_suppression_without_price_evidence() {
    let config = SpotWhaleRuntimeConfig::default();
    let mut stats = high_conviction_stats();
    stats.price_move_pct = None;

    let signal = detect_spot_whale_signal_with_config(&stats, &config).expect("volume signal");
    assert_eq!(signal.signal_type, SpotWhaleSignalType::SpotAggressiveBuy);
}

#[test]
fn spot_whale_signal_history_survives_reopen_and_tracks_discord_state() {
    let store = temp_store("spot-whale-signals");
    let config = SpotWhaleRuntimeConfig::default();
    let signal =
        detect_spot_whale_signal_with_config(&high_conviction_stats(), &config).expect("signal");
    store.upsert_spot_whale_signal(&signal).unwrap();

    let rows = store
        .query_spot_whale_signals(&SpotWhaleSignalQuery {
            symbol: Some("BTC".to_string()),
            severity: Some("critical".to_string()),
            signal_type: Some("spotaggressivebuy".to_string()),
            limit: 10,
            ..SpotWhaleSignalQuery::default()
        })
        .unwrap();
    assert_eq!(rows.len(), 1);
    assert_eq!(rows[0].id, signal.id);
    assert!(rows[0].discord_eligible);
    assert!(!rows[0].discord_sent);

    let changed = store
        .update_spot_whale_discord_status(&signal.id, true, Some(signal.ts + 1), "sent")
        .unwrap();
    assert_eq!(changed, 1);

    let reopened = SqliteStore::open(store.path().to_str().unwrap()).unwrap();
    reopened.migrate().unwrap();
    assert_eq!(reopened.count_spot_whale_signals("BTC").unwrap(), 1);
    let rows = reopened
        .query_spot_whale_signals(&SpotWhaleSignalQuery {
            symbol: Some("BTC".to_string()),
            discord_sent: Some(true),
            limit: 10,
            ..SpotWhaleSignalQuery::default()
        })
        .unwrap();
    assert_eq!(rows.len(), 1);
    assert_eq!(rows[0].id, signal.id);
    assert!(rows[0].discord_sent);
    assert_eq!(rows[0].discord_sent_at, Some(signal.ts + 1));
    assert_eq!(rows[0].discord_reason, "sent");
}

#[test]
fn spot_whale_history_filters_by_absolute_net_direction() {
    let store = temp_store("spot-whale-abs-net-filter");
    let config = SpotWhaleRuntimeConfig::default();
    let base =
        detect_spot_whale_signal_with_config(&high_conviction_stats(), &config).expect("signal");

    let positive = signal_with_net(&base, "positive-250", 1, 250.0);
    let negative = signal_with_net(&base, "negative-520", 2, -520.0);
    let weak = signal_with_net(&base, "weak-150", 3, 150.0);

    store.upsert_spot_whale_signal(&positive).unwrap();
    store.upsert_spot_whale_signal(&negative).unwrap();
    store.upsert_spot_whale_signal(&weak).unwrap();

    let rows = store
        .query_spot_whale_signals(&SpotWhaleSignalQuery {
            symbol: Some("BTC".to_string()),
            min_abs_net_volume_base: Some(200.0),
            limit: 10,
            ..SpotWhaleSignalQuery::default()
        })
        .unwrap();
    let ids = rows
        .iter()
        .map(|signal| signal.id.as_str())
        .collect::<Vec<_>>();
    assert_eq!(ids, vec![negative.id.as_str(), positive.id.as_str()]);

    let rows = store
        .query_spot_whale_signals(&SpotWhaleSignalQuery {
            symbol: Some("BTC".to_string()),
            min_abs_net_volume_base: Some(500.0),
            limit: 10,
            ..SpotWhaleSignalQuery::default()
        })
        .unwrap();
    assert_eq!(rows.len(), 1);
    assert_eq!(rows[0].id, negative.id);
}

#[test]
fn spot_whale_prune_keeps_large_absolute_net_direction_signals() {
    let store = temp_store("spot-whale-prune-preserve-net");
    let config = SpotWhaleRuntimeConfig::default();
    let base =
        detect_spot_whale_signal_with_config(&high_conviction_stats(), &config).expect("signal");
    let cutoff_ts = base.ts + 10_000;

    let old_protected = signal_with_net(&base, "old-protected-negative-60", -20_000, -60.0);
    let old_weak = signal_with_net(&base, "old-weak-30", -19_000, 30.0);
    let recent_weak = signal_with_net(&base, "recent-weak-40", 20_000, 40.0);

    store.upsert_spot_whale_signal(&old_protected).unwrap();
    store.upsert_spot_whale_signal(&old_weak).unwrap();
    store.upsert_spot_whale_signal(&recent_weak).unwrap();

    let pruned = store
        .prune_spot_whale_signals_older_than(
            cutoff_ts,
            SPOT_WHALE_PERMANENT_NET_DIRECTION_THRESHOLD_BASE,
        )
        .unwrap();
    assert_eq!(pruned, 1);

    let rows = store
        .query_spot_whale_signals(&SpotWhaleSignalQuery {
            symbol: Some("BTC".to_string()),
            limit: 10,
            ..SpotWhaleSignalQuery::default()
        })
        .unwrap();
    let ids = rows
        .iter()
        .map(|signal| signal.id.as_str())
        .collect::<Vec<_>>();
    assert!(ids.contains(&old_protected.id.as_str()));
    assert!(ids.contains(&recent_weak.id.as_str()));
    assert!(!ids.contains(&old_weak.id.as_str()));
}

#[test]
fn spot_whale_service_history_supports_offset_and_reports_has_more() {
    let store = temp_store("spot-whale-history-pagination");
    let config = SpotWhaleRuntimeConfig::default();
    let base =
        detect_spot_whale_signal_with_config(&high_conviction_stats(), &config).expect("signal");

    let first = signal_with_net(&base, "page-1", 1_000, 30.0);
    let second = signal_with_net(&base, "page-2", 2_000, 40.0);
    let third = signal_with_net(&base, "page-3", 3_000, 60.0);
    let fourth = signal_with_net(&base, "page-4", 4_000, -80.0);

    store.upsert_spot_whale_signal(&first).unwrap();
    store.upsert_spot_whale_signal(&second).unwrap();
    store.upsert_spot_whale_signal(&third).unwrap();
    store.upsert_spot_whale_signal(&fourth).unwrap();

    let service = SpotWhaleService::new(true, true, third.ts + 60_000, Some(store));
    let history = service.history(SpotWhaleQuery {
        symbol: Some("BTC".to_string()),
        limit: Some(2),
        offset: Some(1),
        ..SpotWhaleQuery::default()
    });

    assert_eq!(history.offset, 1);
    assert_eq!(history.limit, 2);
    assert_eq!(history.total, 4);
    assert!(history.has_more);
    let ids = history
        .items
        .iter()
        .map(|signal| signal.id.as_str())
        .collect::<Vec<_>>();
    assert_eq!(ids, vec![third.id.as_str(), second.id.as_str()]);
}

#[test]
fn spot_whale_history_filters_by_time_range_and_permanent_only() {
    let store = temp_store("spot-whale-history-time-range");
    let config = SpotWhaleRuntimeConfig::default();
    let base =
        detect_spot_whale_signal_with_config(&high_conviction_stats(), &config).expect("signal");

    let older_non_permanent = signal_with_net(&base, "older-non-permanent", -20_000, 30.0);
    let in_range_permanent = signal_with_net(&base, "in-range-permanent", -5_000, -70.0);
    let newer_non_permanent = signal_with_net(&base, "newer-non-permanent", 20_000, 20.0);

    store
        .upsert_spot_whale_signal(&older_non_permanent)
        .unwrap();
    store.upsert_spot_whale_signal(&in_range_permanent).unwrap();
    store
        .upsert_spot_whale_signal(&newer_non_permanent)
        .unwrap();

    let service = SpotWhaleService::new(true, true, base.ts + 60_000, Some(store));
    let history = service.history(SpotWhaleQuery {
        symbol: Some("BTC".to_string()),
        from_ts: Some(base.ts - 10_000),
        to_ts: Some(base.ts + 10_000),
        permanent_only: Some(true),
        limit: Some(10),
        ..SpotWhaleQuery::default()
    });

    assert_eq!(history.total, 1);
    assert!(!history.has_more);
    assert_eq!(history.items.len(), 1);
    assert_eq!(history.items[0].id, in_range_permanent.id);
    assert!(history.items[0].is_permanent);
}

#[test]
fn spot_whale_service_restores_persisted_history_on_startup() {
    let store = temp_store("spot-whale-service-restore");
    let config = SpotWhaleRuntimeConfig::default();
    let signal =
        detect_spot_whale_signal_with_config(&high_conviction_stats(), &config).expect("signal");
    store.upsert_spot_whale_signal(&signal).unwrap();

    let service = SpotWhaleService::new(true, true, signal.ts + 60_000, Some(store));

    let latest = service.latest("BTC", 10);
    assert_eq!(latest.items.len(), 1);
    assert_eq!(latest.items[0].id, signal.id);
    assert_eq!(latest.summary.signal_count, 1);
    assert_eq!(latest.summary.latest_signal_at, Some(signal.ts));
}

#[test]
fn spot_whale_summary_marks_stale_exchange_unhealthy() {
    let old_trade_ts = now_ms().saturating_sub(120_000);
    let service = SpotWhaleService::new(true, true, old_trade_ts.saturating_sub(60_000), None);

    service.ingest_trade(SpotTrade {
        ts: old_trade_ts,
        exchange: SpotExchange::Coinbase,
        symbol: "BTC".to_string(),
        market: "spot".to_string(),
        price: 70_000.0,
        qty_base: 0.2,
        notional_usd: 14_000.0,
        side: SpotTradeSide::Buy,
        trade_id: Some("coinbase-stale".to_string()),
    });
    service.mark_connected(SpotExchange::Coinbase);

    let summary = service.summary("BTC");
    let coinbase = summary.exchanges.get("coinbase").expect("coinbase status");

    assert_eq!(coinbase.status, "stale");
    assert!(!coinbase.connected);
    assert_eq!(summary.health_status, "unhealthy");
    assert_eq!(summary.health_reason, "spot_sources_stale_or_disconnected");
}

#[test]
fn spot_whale_summary_health_is_scoped_to_requested_symbol() {
    let ts = now_ms();
    let service = SpotWhaleService::new(true, true, ts.saturating_sub(120_000), None);

    service.ingest_trade(SpotTrade {
        ts,
        exchange: SpotExchange::Binance,
        symbol: "BTC".to_string(),
        market: "spot".to_string(),
        price: 70_000.0,
        qty_base: 0.2,
        notional_usd: 14_000.0,
        side: SpotTradeSide::Buy,
        trade_id: Some("btc-only-health".to_string()),
    });

    let btc = service.summary("BTC");
    let eth = service.summary("ETH");
    assert!(btc.exchanges["binance"].connected);
    assert!(!eth.exchanges["binance"].connected);
    assert_ne!(eth.exchanges["binance"].status, "connected");
}

#[test]
fn spot_whale_summary_marks_restored_latest_signal_stale() {
    let store = temp_store("spot-whale-stale-restored-summary");
    let config = SpotWhaleRuntimeConfig::default();
    let mut signal =
        detect_spot_whale_signal_with_config(&high_conviction_stats(), &config).expect("signal");
    signal.ts = now_ms().saturating_sub(10 * 60_000);
    store.upsert_spot_whale_signal(&signal).unwrap();

    let service = SpotWhaleService::new(true, true, now_ms(), Some(store));
    let summary = service.summary("BTC");

    assert!(summary.latest_is_stale);
    assert!(summary.latest_age_sec.is_some_and(|age| age >= 9 * 60));
    assert_eq!(
        summary.latest_stale_reason.as_deref(),
        Some("latest_signal_ttl_exceeded")
    );
    assert_eq!(summary.status, "calm");
    assert_eq!(summary.direction, "neutral");
}

#[test]
fn spot_whale_repo_cursor_is_stable_for_equal_timestamps() {
    let store = temp_store("spot-whale-stable-cursor");
    let config = SpotWhaleRuntimeConfig::default();
    let base =
        detect_spot_whale_signal_with_config(&high_conviction_stats(), &config).expect("signal");
    let ts = base.ts + 10_000;
    for suffix in ["a", "b", "c"] {
        let mut signal = signal_with_net(&base, suffix, 10_000, 60.0);
        signal.id = format!("cursor-{suffix}");
        signal.ts = ts;
        store.upsert_spot_whale_signal(&signal).unwrap();
    }

    let first = store
        .query_spot_whale_signals(&SpotWhaleSignalQuery {
            symbol: Some("BTC".to_string()),
            limit: 2,
            ..SpotWhaleSignalQuery::default()
        })
        .unwrap();
    assert_eq!(
        first
            .iter()
            .map(|item| item.id.as_str())
            .collect::<Vec<_>>(),
        vec!["cursor-c", "cursor-b"]
    );

    let second = store
        .query_spot_whale_signals(&SpotWhaleSignalQuery {
            symbol: Some("BTC".to_string()),
            cursor_ts: Some(ts),
            cursor_signal_id: Some("cursor-b".to_string()),
            limit: 2,
            ..SpotWhaleSignalQuery::default()
        })
        .unwrap();
    assert_eq!(
        second
            .iter()
            .map(|item| item.id.as_str())
            .collect::<Vec<_>>(),
        vec!["cursor-a"]
    );
}

#[test]
fn spot_whale_cursor_round_trips_timestamp_and_stable_id() {
    let config = SpotWhaleRuntimeConfig::default();
    let signal =
        detect_spot_whale_signal_with_config(&high_conviction_stats(), &config).expect("signal");
    let cursor = encode_spot_history_cursor(&signal);

    assert_eq!(
        decode_spot_history_cursor(&cursor),
        Some((signal.ts, signal.id.clone()))
    );
    assert_eq!(decode_spot_history_cursor("not-a-cursor"), None);
}

#[test]
fn spot_whale_service_cursor_does_not_offer_an_empty_extra_page() {
    let store = temp_store("spot-whale-cursor-final-page");
    let config = SpotWhaleRuntimeConfig::default();
    let base =
        detect_spot_whale_signal_with_config(&high_conviction_stats(), &config).expect("signal");
    for (index, suffix) in ["a", "b", "c", "d"].into_iter().enumerate() {
        let signal = signal_with_net(&base, suffix, index as i64 * 1_000, 60.0);
        store.upsert_spot_whale_signal(&signal).unwrap();
    }

    let service = SpotWhaleService::new(true, true, base.ts + 60_000, Some(store));
    let first = service.history(SpotWhaleQuery {
        symbol: Some("BTC".to_string()),
        limit: Some(2),
        ..SpotWhaleQuery::default()
    });
    let (cursor_ts, cursor_signal_id) =
        decode_spot_history_cursor(first.next_cursor.as_deref().expect("next cursor"))
            .expect("valid cursor");
    let final_page = service.history(SpotWhaleQuery {
        symbol: Some("BTC".to_string()),
        limit: Some(2),
        cursor_ts: Some(cursor_ts),
        cursor_signal_id: Some(cursor_signal_id),
        ..SpotWhaleQuery::default()
    });

    assert_eq!(final_page.items.len(), 2);
    assert!(!final_page.has_more);
    assert!(final_page.next_cursor.is_none());
}

#[test]
fn spot_whale_service_retention_prunes_only_old_unprotected_rows() {
    let store = temp_store("spot-whale-service-retention");
    let config = SpotWhaleRuntimeConfig::default();
    let base =
        detect_spot_whale_signal_with_config(&high_conviction_stats(), &config).expect("signal");
    let old_weak = signal_with_net(&base, "service-old-weak", 0, 20.0);
    let old_protected = signal_with_net(&base, "service-old-protected", 1, -60.0);
    store.upsert_spot_whale_signal(&old_weak).unwrap();
    store.upsert_spot_whale_signal(&old_protected).unwrap();
    let service = SpotWhaleService::new(true, true, base.ts, Some(store.clone()));

    let deleted = service
        .run_retention_once(base.ts + 366 * 86_400_000)
        .expect("retention run");

    assert_eq!(deleted, 1);
    assert_eq!(store.count_spot_whale_signals("BTC").unwrap(), 1);
}

fn high_conviction_stats() -> SpotWhaleWindowStats {
    SpotWhaleWindowStats {
        symbol: "BTC".to_string(),
        window_sec: 15,
        ts: 1_700_000_015_000,
        buy_volume_base: 850.0,
        sell_volume_base: 120.0,
        total_volume_base: 970.0,
        net_volume_base: 730.0,
        total_notional_usd: 67_900_000.0,
        dominance: 730.0 / 970.0,
        price_move_pct: Some(0.22),
        coinbase_premium_pct: Some(0.03),
        exchange_count: 2,
        main_exchange: Some("binance".to_string()),
        exchanges: vec![
            contribution("binance", 520.0, 80.0),
            contribution("coinbase", 330.0, 40.0),
        ],
        dynamic_multiple: Some(9.0),
        multi_exchange_confirmed: true,
        data_quality: 92,
        startup_age_ms: Some(120_000),
    }
}

fn temp_store(name: &str) -> SqliteStore {
    let mut path = std::env::temp_dir();
    path.push("toxic-order-monitor-rs-tests");
    path.push(unique_path(name));
    let _ = std::fs::remove_file(&path);
    let store = SqliteStore::open(path.to_str().expect("utf8 sqlite path")).expect("open sqlite");
    store.migrate().expect("migrate sqlite");
    store
}

fn unique_path(name: &str) -> PathBuf {
    let nanos = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("time")
        .as_nanos();
    PathBuf::from(format!("{name}-{nanos}.sqlite"))
}

fn contribution(exchange: &str, buy: f64, sell: f64) -> SpotExchangeContribution {
    let total = buy + sell;
    let net = buy - sell;
    SpotExchangeContribution {
        exchange: exchange.to_string(),
        buy_volume_base: buy,
        sell_volume_base: sell,
        total_volume_base: total,
        buy_notional_usd: buy * 70_000.0,
        sell_notional_usd: sell * 70_000.0,
        total_notional_usd: total * 70_000.0,
        net_volume_base: net,
        dominance: net.abs() / total,
        trade_count: 10,
    }
}

fn signal_with_net(
    base: &btc_toxic_flow_monitor_rs::spot_whale_monitor::types::SpotWhaleSignal,
    suffix: &str,
    ts_delta_ms: i64,
    net_volume_base: f64,
) -> btc_toxic_flow_monitor_rs::spot_whale_monitor::types::SpotWhaleSignal {
    let mut signal = base.clone();
    signal.id = format!("{}-{suffix}", base.id);
    signal.ts = base.ts + ts_delta_ms;
    signal.net_volume_base = net_volume_base;
    signal.dominance = net_volume_base.abs() / signal.total_volume_base.max(1.0);
    signal
}
