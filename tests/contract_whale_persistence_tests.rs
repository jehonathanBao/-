use std::time::{SystemTime, UNIX_EPOCH};

use btc_toxic_flow_monitor_rs::{
    contract_whale_monitor::{
        aggregator::{
            aggregate_1s_buckets, aggregate_liquidation_1s_buckets, compute_percentile_threshold,
            dynamic_multiple_for_volume, historical_window_average_btc,
            historical_window_average_btc_with_min_samples, liquidation_context_for_window,
            market_context_from_snapshots, percentile_level_for_volume, rolling_window_stats,
        },
        config::reset_contract_whale_runtime_config,
        detector::detect_contract_whale_signal,
        normalizer::{
            normalize_binance_agg_trade, normalize_binance_force_order,
            normalize_binance_funding_rate_json, normalize_binance_open_interest_json,
            normalize_bitfinex_trade, normalize_okx_funding_rate_json,
            normalize_okx_open_interest_json,
        },
        persistence::flush_contract_flow_buckets_nonblocking,
        types::{
            ContractFlowBucket, ContractWhaleDirection, ContractWhaleMarketType,
            ContractWhaleSeverity, ContractWhaleSignalType, ContractWhaleSourceRole,
        },
    },
    storage::{
        contract_whale_repo::{ContractWhaleRepo, ContractWhaleSignalQuery},
        main_force_events_repo::MainForceEventsRepo,
        SqliteStore,
    },
    types::main_force_event::{MainForceEventObservation, MainForceEventQuery},
};

#[test]
fn contract_flow_1s_upsert_is_idempotent() {
    let store = temp_store("contract-flow-1s");
    let mut bucket = ContractFlowBucket {
        ts_bucket: 1_700_000_000_000,
        exchange: "binance".to_string(),
        symbol: "BTC".to_string(),
        market_type: ContractWhaleMarketType::Perp,
        source_role: ContractWhaleSourceRole::Primary,
        product_id: Some("BTCUSDT".to_string()),
        buy_volume_btc: 10.0,
        sell_volume_btc: 2.0,
        buy_notional_usd: 700_000.0,
        sell_notional_usd: 140_000.0,
        trade_count: 3,
        max_single_trade_btc: 8.0,
        vwap: Some(70_000.0),
    };

    assert_eq!(
        store
            .upsert_contract_flow_buckets(&[bucket.clone()])
            .unwrap(),
        1
    );
    bucket.buy_volume_btc = 12.0;
    bucket.trade_count = 4;
    assert_eq!(
        store
            .upsert_contract_flow_buckets(&[bucket.clone()])
            .unwrap(),
        1
    );

    let rows = store.list_recent_contract_flow_buckets("BTC", 10).unwrap();
    assert_eq!(rows.len(), 1);
    assert_eq!(rows[0].buy_volume_btc, 12.0);
    assert_eq!(rows[0].trade_count, 4);
    assert_eq!(rows[0].market_type, ContractWhaleMarketType::Perp);
    assert_eq!(rows[0].source_role, ContractWhaleSourceRole::Primary);
    assert_eq!(rows[0].product_id.as_deref(), Some("BTCUSDT"));
}

#[test]
fn contract_flow_1s_keeps_spot_rows_out_of_perp_queries() {
    let store = temp_store("contract-flow-market-type");
    let perp_bucket = ContractFlowBucket {
        ts_bucket: 1_700_000_002_000,
        exchange: "binance".to_string(),
        symbol: "BTC".to_string(),
        market_type: ContractWhaleMarketType::Perp,
        source_role: ContractWhaleSourceRole::Primary,
        product_id: Some("BTCUSDT".to_string()),
        buy_volume_btc: 10.0,
        sell_volume_btc: 2.0,
        buy_notional_usd: 700_000.0,
        sell_notional_usd: 140_000.0,
        trade_count: 3,
        max_single_trade_btc: 8.0,
        vwap: Some(70_000.0),
    };
    let spot_bucket = ContractFlowBucket {
        market_type: ContractWhaleMarketType::Spot,
        source_role: ContractWhaleSourceRole::Primary,
        product_id: Some("BTC-USDT".to_string()),
        buy_volume_btc: 999.0,
        sell_volume_btc: 0.0,
        buy_notional_usd: 69_930_000.0,
        sell_notional_usd: 0.0,
        trade_count: 99,
        max_single_trade_btc: 999.0,
        ..perp_bucket.clone()
    };

    assert_eq!(
        store
            .upsert_contract_flow_buckets(&[perp_bucket.clone(), spot_bucket])
            .unwrap(),
        2
    );

    let rows = store.list_recent_contract_flow_buckets("BTC", 10).unwrap();
    assert_eq!(rows.len(), 1);
    assert_eq!(rows[0].market_type, ContractWhaleMarketType::Perp);
    assert_eq!(rows[0].buy_volume_btc, perp_bucket.buy_volume_btc);
    let raw_count: i64 = store
        .with_connection(|conn| {
            Ok(conn.query_row(
                "SELECT COUNT(*) FROM contract_flow_1s WHERE ts_bucket = ?1 AND exchange = ?2 AND symbol = ?3",
                rusqlite::params![perp_bucket.ts_bucket, perp_bucket.exchange, perp_bucket.symbol],
                |row| row.get(0),
            )?)
        })
        .unwrap();
    assert_eq!(raw_count, 2);
}

#[tokio::test]
async fn contract_flow_nonblocking_flush_writes_buckets() {
    let store = temp_store("contract-flow-nonblocking");
    let bucket = ContractFlowBucket {
        ts_bucket: 1_700_000_001_000,
        exchange: "okx".to_string(),
        symbol: "BTC".to_string(),
        market_type: ContractWhaleMarketType::Perp,
        source_role: ContractWhaleSourceRole::Disabled,
        product_id: Some("BTC-USDT-SWAP".to_string()),
        buy_volume_btc: 5.0,
        sell_volume_btc: 1.0,
        buy_notional_usd: 350_000.0,
        sell_notional_usd: 70_000.0,
        trade_count: 2,
        max_single_trade_btc: 5.0,
        vwap: Some(70_000.0),
    };

    let outcome = flush_contract_flow_buckets_nonblocking(Some(store.clone()), vec![bucket]).await;

    assert!(outcome.attempted);
    assert!(outcome.succeeded);
    assert_eq!(outcome.written, 1);
    assert_eq!(
        store
            .list_recent_contract_flow_buckets("BTC", 10)
            .unwrap()
            .len(),
        1
    );
}

#[test]
fn contract_whale_signal_history_survives_reopen_and_tracks_discord_state() {
    let store = temp_store("contract-whale-signals");
    let signal = sample_s_signal();
    store.upsert_contract_whale_signal(&signal).unwrap();

    let rows = store
        .list_contract_whale_signals("BTC", Some(signal.severity), 10)
        .unwrap();
    assert_eq!(rows.len(), 1);
    assert!(rows[0].discord_eligible);
    assert!(!rows[0].discord_sent);

    let changed = store
        .update_contract_whale_discord_status(&signal.id, true, Some(signal.ts + 1))
        .unwrap();
    assert_eq!(changed, 1);

    let reopened = SqliteStore::open(store.path().to_str().unwrap()).unwrap();
    reopened.migrate().unwrap();
    let rows = reopened
        .list_contract_whale_signals("BTC", None, 10)
        .unwrap();
    assert_eq!(rows.len(), 1);
    assert_eq!(rows[0].id, signal.id);
    assert!(rows[0].discord_sent);
    assert_eq!(rows[0].discord_sent_at, Some(signal.ts + 1));
    assert_eq!(rows[0].total_volume_btc, signal.total_volume_btc);
    assert_eq!(rows[0].market_type, ContractWhaleMarketType::Perp);
    assert_eq!(rows[0].threshold_profile, "binance_bitfinex");
    assert_eq!(
        rows[0].threshold_profile_reason,
        "active_contract_sources=binance,bitfinex"
    );
    assert_eq!(
        rows[0].configured_contract_sources,
        vec!["binance".to_string(), "bitfinex".to_string()]
    );
    assert_eq!(
        rows[0].eligible_contract_sources,
        vec!["binance".to_string(), "bitfinex".to_string()]
    );
    assert_eq!(
        rows[0].active_contract_sources,
        vec!["binance".to_string(), "bitfinex".to_string()]
    );
    assert!(!rows[0].active_sources.contract.is_empty());
    assert!(rows[0]
        .active_sources
        .contract
        .iter()
        .any(|entry| entry.exchange == "binance"
            && entry.market_type == ContractWhaleMarketType::Perp
            && entry.enabled
            && entry.status == "active"));
    assert!(!rows[0]
        .active_sources
        .contract
        .iter()
        .any(|entry| entry.exchange == "coinbase"
            && entry.market_type == ContractWhaleMarketType::Perp));
    assert!(
        !rows[0]
            .active_sources
            .contract
            .iter()
            .any(|entry| entry.exchange == "okx"
                && entry.market_type == ContractWhaleMarketType::Perp)
    );
    assert!(rows[0]
        .active_sources
        .spot
        .iter()
        .any(|entry| entry.exchange == "coinbase" && entry.status == "spot_only"));
}

#[test]
fn contract_whale_signal_history_recovers_legacy_profile_snapshot() {
    let store = temp_store("contract-whale-legacy-signal");
    let mut legacy = sample_s_signal();
    legacy.id = "contract-whale:legacy:15:1700000015000:buy".to_string();
    let mut payload = serde_json::to_value(&legacy).unwrap();
    let payload_object = payload.as_object_mut().expect("legacy payload object");
    payload_object.remove("thresholdProfile");
    payload_object.remove("thresholdProfileReason");
    payload_object.remove("configuredContractSources");
    payload_object.remove("eligibleContractSources");
    payload_object.remove("activeContractSources");
    payload_object.remove("activeSources");
    let payload_json = serde_json::to_string(&payload).unwrap();

    store
        .with_connection(|conn| {
            conn.execute(
                r#"
                INSERT INTO contract_whale_signals (
                  signal_id, ts, symbol, window_sec, signal_type, direction, severity, score,
                  total_volume_btc, net_volume_btc, total_notional_usd, dominance,
                  price_move_pct, main_exchange, market_type, source_role, exchanges_json,
                  active_sources_json, threshold_profile, dynamic_multiple, data_quality,
                  discord_eligible, discord_sent, discord_sent_at, payload_json, created_at
                ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12,
                          ?13, ?14, 'perp', 'primary', '[]',
                          '{"contract":[],"spot":[]}', 'three_exchange', ?15, ?16,
                          0, 0, NULL, ?17, ?18)
                "#,
                rusqlite::params![
                    legacy.id,
                    legacy.ts,
                    legacy.symbol,
                    legacy.window_sec as i64,
                    "aggressive_buy",
                    "buy",
                    "s",
                    legacy.score as i64,
                    legacy.total_volume_btc,
                    legacy.net_volume_btc,
                    legacy.total_notional_usd,
                    legacy.dominance,
                    legacy.price_move_pct,
                    legacy.main_exchange,
                    legacy.dynamic_multiple,
                    legacy.data_quality as i64,
                    payload_json,
                    legacy.ts,
                ],
            )?;
            Ok(())
        })
        .unwrap();

    let rows = store.list_contract_whale_signals("BTC", None, 10).unwrap();

    assert_eq!(rows.len(), 1);
    assert_eq!(rows[0].threshold_profile, "unknown");
    assert_eq!(rows[0].threshold_profile_reason, "legacy_signal");
    assert!(rows[0].configured_contract_sources.is_empty());
    assert!(rows[0].eligible_contract_sources.is_empty());
    assert!(rows[0].active_contract_sources.is_empty());
    assert!(rows[0].active_sources.contract.is_empty());
    assert!(rows[0].active_sources.spot.is_empty());
    assert_eq!(rows[0].active_sources.threshold_profile, "unknown");
    assert_eq!(
        rows[0].active_sources.threshold_profile_reason,
        "legacy_signal"
    );
}

#[test]
fn contract_whale_signal_query_filters_and_paginates_history() {
    let store = temp_store("contract-whale-query");
    let base = sample_s_signal();
    let mut buy_critical = base.clone();
    buy_critical.id = "contract-whale:BTC:15:1700000000000:buy-critical".to_string();
    buy_critical.ts = 1_700_000_000_000;
    buy_critical.severity = ContractWhaleSeverity::Critical;
    buy_critical.net_volume_btc = 800.0;

    let mut buy_s = base.clone();
    buy_s.id = "contract-whale:BTC:15:1700000010000:buy-s".to_string();
    buy_s.ts = 1_700_000_010_000;
    buy_s.severity = ContractWhaleSeverity::S;
    buy_s.discord_sent = true;
    buy_s.discord_sent_at = Some(buy_s.ts + 1);
    buy_s.net_volume_btc = 1_200.0;

    let mut sell_critical = base.clone();
    sell_critical.id = "contract-whale:BTC:15:1700000020000:sell-critical".to_string();
    sell_critical.ts = 1_700_000_020_000;
    sell_critical.severity = ContractWhaleSeverity::Critical;
    sell_critical.direction = ContractWhaleDirection::Sell;
    sell_critical.signal_type = ContractWhaleSignalType::AggressiveSell;
    sell_critical.net_volume_btc = -1_500.0;

    for signal in [&buy_critical, &buy_s, &sell_critical] {
        store.upsert_contract_whale_signal(signal).unwrap();
    }
    let mut spot_signal = base.clone();
    spot_signal.id = "contract-whale:BTC:15:spot-row".to_string();
    spot_signal.ts = 1_700_000_030_000;
    spot_signal.market_type = ContractWhaleMarketType::Spot;
    store.upsert_contract_whale_signal(&spot_signal).unwrap();

    let critical = store
        .query_contract_whale_signals(&ContractWhaleSignalQuery {
            symbol: Some("BTC".to_string()),
            severity: Some(ContractWhaleSeverity::Critical),
            from_ts: Some(1_700_000_000_000),
            to_ts: Some(1_700_086_400_000),
            limit: 10,
            ..ContractWhaleSignalQuery::default()
        })
        .unwrap();
    assert_eq!(critical.len(), 2);
    assert_eq!(critical[0].id, sell_critical.id);

    let buy_only = store
        .query_contract_whale_signals(&ContractWhaleSignalQuery {
            symbol: Some("BTC".to_string()),
            direction: Some(ContractWhaleDirection::Buy),
            signal_type: Some(ContractWhaleSignalType::AggressiveBuy),
            limit: 10,
            ..ContractWhaleSignalQuery::default()
        })
        .unwrap();
    assert_eq!(buy_only.len(), 2);
    assert!(buy_only
        .iter()
        .all(|signal| signal.direction == ContractWhaleDirection::Buy));
    assert!(buy_only
        .iter()
        .all(|signal| signal.market_type == ContractWhaleMarketType::Perp));
    assert!(!buy_only.iter().any(|signal| signal.id == spot_signal.id));

    let paged = store
        .query_contract_whale_signals(&ContractWhaleSignalQuery {
            symbol: Some("BTC".to_string()),
            limit: 1,
            offset: 1,
            ..ContractWhaleSignalQuery::default()
        })
        .unwrap();
    assert_eq!(paged.len(), 1);
    assert_eq!(paged[0].id, buy_s.id);

    let unsent_binance_15s = store
        .query_contract_whale_signals(&ContractWhaleSignalQuery {
            symbol: Some("BTC".to_string()),
            discord_sent: Some(false),
            window_sec: Some(15),
            exchange: Some("binance".to_string()),
            limit: 10,
            ..ContractWhaleSignalQuery::default()
        })
        .unwrap();
    assert_eq!(unsent_binance_15s.len(), 2);
    assert!(unsent_binance_15s
        .iter()
        .all(|signal| !signal.discord_sent && signal.window_sec == 15));

    let abs_net_1000 = store
        .query_contract_whale_signals(&ContractWhaleSignalQuery {
            symbol: Some("BTC".to_string()),
            min_abs_net_volume_btc: Some(1_000.0),
            limit: 10,
            ..ContractWhaleSignalQuery::default()
        })
        .unwrap();
    let ids = abs_net_1000
        .iter()
        .map(|signal| signal.id.as_str())
        .collect::<Vec<_>>();
    assert_eq!(ids, vec![sell_critical.id.as_str(), buy_s.id.as_str()]);
    assert!(abs_net_1000
        .iter()
        .all(|signal| signal.net_volume_btc.abs() >= 1_000.0));
}

#[test]
fn contract_whale_retention_prunes_old_flow_buckets_and_old_signals() {
    let store = temp_store("contract-whale-retention");
    let now = 1_700_000_000_000;
    let buckets = vec![
        ContractFlowBucket {
            ts_bucket: now - 20 * 24 * 60 * 60 * 1000,
            exchange: "binance".to_string(),
            symbol: "BTC".to_string(),
            market_type: ContractWhaleMarketType::Perp,
            source_role: ContractWhaleSourceRole::Primary,
            product_id: None,
            buy_volume_btc: 1.0,
            sell_volume_btc: 0.0,
            buy_notional_usd: 70_000.0,
            sell_notional_usd: 0.0,
            trade_count: 1,
            max_single_trade_btc: 1.0,
            vwap: Some(70_000.0),
        },
        ContractFlowBucket {
            ts_bucket: now - 1_000,
            exchange: "binance".to_string(),
            symbol: "BTC".to_string(),
            market_type: ContractWhaleMarketType::Perp,
            source_role: ContractWhaleSourceRole::Primary,
            product_id: None,
            buy_volume_btc: 2.0,
            sell_volume_btc: 0.0,
            buy_notional_usd: 140_000.0,
            sell_notional_usd: 0.0,
            trade_count: 1,
            max_single_trade_btc: 2.0,
            vwap: Some(70_000.0),
        },
    ];
    store.upsert_contract_flow_buckets(&buckets).unwrap();

    let mut old_signal = sample_s_signal();
    old_signal.id = "contract-whale:BTC:15:old:s".to_string();
    old_signal.ts = now - 400 * 24 * 60 * 60 * 1000;
    let mut fresh_signal = sample_s_signal();
    fresh_signal.id = "contract-whale:BTC:15:fresh:s".to_string();
    fresh_signal.ts = now;
    store.upsert_contract_whale_signal(&old_signal).unwrap();
    store.upsert_contract_whale_signal(&fresh_signal).unwrap();

    let result = store
        .prune_contract_whale_retention(
            now - 14 * 24 * 60 * 60 * 1000,
            now - 365 * 24 * 60 * 60 * 1000,
        )
        .unwrap();

    assert_eq!(result.flow_1s_deleted, 1);
    assert_eq!(result.signal_deleted, 1);
    assert_eq!(
        store
            .list_recent_contract_flow_buckets("BTC", 10)
            .unwrap()
            .len(),
        1
    );
    let remaining = store
        .query_contract_whale_signals(&ContractWhaleSignalQuery {
            symbol: Some("BTC".to_string()),
            limit: 10,
            ..ContractWhaleSignalQuery::default()
        })
        .unwrap();
    assert_eq!(remaining.len(), 1);
    assert_eq!(remaining[0].id, fresh_signal.id);
}

#[test]
fn contract_flow_history_builds_dynamic_average_and_percentile_thresholds() {
    let store = temp_store("contract-whale-percentiles");
    let base_ts = 1_700_000_000_000;
    let buckets = (0..120)
        .map(|index| ContractFlowBucket {
            ts_bucket: base_ts + (index * 15_000),
            exchange: if index % 2 == 0 { "binance" } else { "okx" }.to_string(),
            symbol: "BTC".to_string(),
            market_type: ContractWhaleMarketType::Perp,
            source_role: if index % 2 == 0 {
                ContractWhaleSourceRole::Primary
            } else {
                ContractWhaleSourceRole::Disabled
            },
            product_id: None,
            buy_volume_btc: 100.0 + index as f64,
            sell_volume_btc: 20.0,
            buy_notional_usd: (100.0 + index as f64) * 70_000.0,
            sell_notional_usd: 20.0 * 70_000.0,
            trade_count: 10,
            max_single_trade_btc: 100.0 + index as f64,
            vwap: Some(70_000.0),
        })
        .collect::<Vec<_>>();
    store.upsert_contract_flow_buckets(&buckets).unwrap();

    let rows = store
        .list_contract_flow_buckets_between("BTC", base_ts, base_ts + 120 * 15_000)
        .unwrap();
    let average = historical_window_average_btc(&rows, "BTC", 15, base_ts, base_ts + 120 * 15_000)
        .expect("average");
    assert!(historical_window_average_btc_with_min_samples(
        &rows,
        "BTC",
        15,
        base_ts,
        base_ts + 120 * 15_000,
        200,
    )
    .is_none());
    assert!(historical_window_average_btc_with_min_samples(
        &rows,
        "BTC",
        15,
        base_ts,
        base_ts + 120 * 15_000,
        20,
    )
    .is_some());
    let multiple = dynamic_multiple_for_volume(1_500.0, Some(average)).expect("multiple");
    assert!(multiple > 8.0);

    let threshold = compute_percentile_threshold(
        &rows,
        "BTC",
        "all",
        15,
        base_ts,
        base_ts + 120 * 15_000,
        base_ts + 2_000_000,
    )
    .expect("percentile threshold");
    assert_eq!(threshold.window_sec, 15);
    assert_eq!(threshold.exchange, "all");
    assert!(threshold.p99_9_btc >= threshold.p99_5_btc);
    assert!(threshold.p99_5_btc >= threshold.p99_0_btc);

    assert_eq!(
        store
            .upsert_contract_whale_percentiles(std::slice::from_ref(&threshold))
            .unwrap(),
        1
    );
    let latest = store
        .latest_contract_whale_percentile("BTC", "all", 15)
        .unwrap()
        .expect("latest percentile");
    assert_eq!(latest.computed_at, threshold.computed_at);
    assert_eq!(
        percentile_level_for_volume(latest.p99_9_btc + 1.0, Some(&latest)),
        Some(99.9)
    );
}

#[test]
fn contract_liquidation_1s_upsert_and_window_context_are_available() {
    let store = temp_store("contract-liquidation-1s");
    let now = 1_700_000_015_000;
    let liquidations = vec![
        normalize_binance_force_order(now - 1_000, 70_000.0, 300.0, "SELL").unwrap(),
        normalize_binance_force_order(now - 1_000, 70_000.0, 120.0, "BUY").unwrap(),
    ];
    let buckets = aggregate_liquidation_1s_buckets(&liquidations);

    assert_eq!(
        store.upsert_contract_liquidation_buckets(&buckets).unwrap(),
        1
    );
    let rows = store
        .list_contract_liquidation_buckets_between("BTC", now - 15_000, now)
        .unwrap();
    assert_eq!(rows.len(), 1);
    assert_eq!(rows[0].long_liq_btc, 300.0);
    assert_eq!(rows[0].short_liq_btc, 120.0);

    let context = liquidation_context_for_window(&rows, "BTC", 15, now, 1_500.0);
    assert_eq!(context.total_liq_btc, 420.0);
    assert_eq!(context.liq_to_volume_ratio, Some(0.28));
}

#[test]
fn contract_oi_and_funding_snapshots_build_market_context() {
    reset_contract_whale_runtime_config();
    let store = temp_store("contract-market-context");
    let now = 1_700_000_300_000;
    let oi_snapshots = vec![
        normalize_binance_open_interest_json(
            &serde_json::json!({
                "symbol": "BTCUSDT",
                "openInterest": "50000",
                "time": now - 300_000
            }),
            Some(70_000.0),
            now - 300_000,
        )
        .unwrap(),
        normalize_binance_open_interest_json(
            &serde_json::json!({
                "symbol": "BTCUSDT",
                "openInterest": "51500",
                "time": now
            }),
            Some(70_000.0),
            now,
        )
        .unwrap(),
        normalize_okx_open_interest_json(
            &serde_json::json!({
                "data": [{
                    "instId": "BTC-USDT-SWAP",
                    "oi": "1000000",
                    "ts": (now - 300_000).to_string()
                }]
            }),
            0.01,
        )
        .unwrap(),
        normalize_okx_open_interest_json(
            &serde_json::json!({
                "data": [{
                    "instId": "BTC-USDT-SWAP",
                    "oi": "1050000",
                    "ts": now.to_string()
                }]
            }),
            0.01,
        )
        .unwrap(),
    ];
    let funding_snapshots = vec![
        normalize_binance_funding_rate_json(
            &serde_json::json!({
                "symbol": "BTCUSDT",
                "lastFundingRate": "0.00020",
                "time": now
            }),
            now,
        )
        .unwrap(),
        normalize_okx_funding_rate_json(&serde_json::json!({
            "data": [{
                "instId": "BTC-USDT-SWAP",
                "fundingRate": "0.00010",
                "ts": now.to_string()
            }]
        }))
        .unwrap(),
    ];

    assert_eq!(
        store.upsert_contract_oi_snapshots(&oi_snapshots).unwrap(),
        4
    );
    assert_eq!(
        store
            .upsert_contract_funding_snapshots(&funding_snapshots)
            .unwrap(),
        2
    );
    let stored_oi = store
        .list_contract_oi_snapshots_between("BTC", now - 360_000, now)
        .unwrap();
    let stored_funding = store
        .list_contract_funding_snapshots_between("BTC", now - 360_000, now)
        .unwrap();
    let context = market_context_from_snapshots(&stored_oi, &stored_funding, "BTC", now);

    assert!(context.oi_available);
    assert!(context.funding_available);
    assert_eq!(context.oi_bias.as_deref(), Some("rising"));
    assert_eq!(context.oi_change_5m_btc, Some(1_500.0));
    assert_eq!(context.funding_bias.as_deref(), Some("long"));
    assert!((context.funding_rate.expect("funding rate") - 0.00020).abs() < 0.0000001);
}

#[test]
fn main_force_events_open_update_and_close_after_quiet_period() {
    let store = temp_store("main-force-events");
    let started_at = 1_700_000_000_000;
    let observation = MainForceEventObservation {
        symbol: "BTC".to_string(),
        observed_at: started_at,
        regime_type: "main_force_long_build".to_string(),
        severity: "Major".to_string(),
        main_force_score: 84.0,
        extreme_impact_score: 58.0,
        structure_bias: 62.0,
        confidence: 76.0,
        spot_score: Some(71.0),
        contract_score: Some(86.0),
        cross_confirm_score: Some(74.0),
        cwm_score: Some(89.0),
        oi_score: Some(82.0),
        liquidation_score: Some(31.0),
        funding_crowding_score: Some(24.0),
        main_force_confirmed: true,
        extreme_impact_confirmed: false,
        liquidation_driven: false,
        reasons_json: serde_json::json!({
            "finalResult": "高概率主力建多，不是单纯清算推动。"
        }),
    };

    let opened = store
        .observe_main_force_event("BTC", Some(&observation), started_at)
        .unwrap()
        .expect("opened event");
    assert_eq!(opened.started_at, started_at);
    assert_eq!(opened.regime_type, "main_force_long_build");

    let stronger = MainForceEventObservation {
        observed_at: started_at + 300_000,
        main_force_score: 88.0,
        extreme_impact_score: 64.0,
        confidence: 81.0,
        ..observation.clone()
    };
    let updated = store
        .observe_main_force_event("BTC", Some(&stronger), stronger.observed_at)
        .unwrap()
        .expect("updated event");
    assert_eq!(updated.id, opened.id);
    assert_eq!(updated.peak_main_force_score, 88.0);
    assert_eq!(updated.peak_at, stronger.observed_at);

    let cooling = MainForceEventObservation {
        observed_at: stronger.observed_at + 60_000,
        severity: "Watch".to_string(),
        main_force_score: 42.0,
        extreme_impact_score: 51.0,
        confidence: 63.0,
        ..stronger.clone()
    };
    let inactive = store
        .observe_main_force_event("BTC", Some(&cooling), cooling.observed_at)
        .unwrap()
        .expect("inactive event");
    assert_eq!(inactive.inactive_since, Some(cooling.observed_at));

    let closed = store
        .observe_main_force_event("BTC", None, cooling.observed_at + 15 * 60 * 1000 + 1_000)
        .unwrap();
    assert!(closed.is_none());

    let events = store
        .list_main_force_events(&MainForceEventQuery {
            symbol: Some("BTC".to_string()),
            limit: 10,
            ..MainForceEventQuery::default()
        })
        .unwrap();
    assert_eq!(events.len(), 1);
    assert_eq!(events[0].id, opened.id);
    assert_eq!(events[0].ended_at, Some(cooling.observed_at));
    assert_eq!(events[0].peak_main_force_score, 88.0);
}

fn sample_s_signal() -> btc_toxic_flow_monitor_rs::contract_whale_monitor::types::ContractWhaleSignal
{
    reset_contract_whale_runtime_config();
    let now = 1_700_000_015_000;
    let trades = vec![
        normalize_binance_agg_trade(now - 1_000, 70_000.0, 3_200.0, false).unwrap(),
        normalize_bitfinex_trade(now - 1_000, 70_000.0, 430.0).unwrap(),
        normalize_binance_agg_trade(now - 1_000, 70_000.0, 500.0, true).unwrap(),
    ];
    let buckets = aggregate_1s_buckets(&trades);
    let stats = rolling_window_stats(&buckets, "BTC", 15, now, Some(0.31), Some(10.4), 94)
        .expect("window stats");
    detect_contract_whale_signal(&stats).expect("signal")
}

fn temp_store(name: &str) -> SqliteStore {
    let unique = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_nanos();
    let path = std::env::temp_dir().join(format!(
        "btc-toxic-flow-{name}-{unique}-{}.sqlite",
        std::process::id()
    ));
    let store = SqliteStore::open(path.to_str().unwrap()).unwrap();
    store.migrate().unwrap();
    store
}
