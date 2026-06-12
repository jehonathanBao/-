use std::{
    collections::BTreeMap,
    sync::{Mutex, MutexGuard, OnceLock},
};

use btc_toxic_flow_monitor_rs::{
    api::contract_whale_routes::{
        build_contract_whale_items_response, build_contract_whale_metrics_text,
        build_contract_whale_response, build_contract_whale_response_with_runtime_and_baselines,
        parse_history_query, ContractWhaleQualityBaseline, ContractWhaleQuery,
        ContractWhaleResponseRuntime,
    },
    contract_whale_monitor::{
        config::{
            reset_contract_whale_runtime_config, set_contract_whale_runtime_config,
            ContractWhaleRuntimeConfig,
        },
        types::{
            ContractWhaleLiquidationContext, ContractWhaleMarketContext, ContractWhaleMarketType,
            ContractWhaleSeverity, ContractWhaleTrend60s,
        },
    },
    types::flow::{DataQuality, FlowState, FlowWindow, VenueFlowBreakdown},
};

fn contract_whale_test_guard() -> MutexGuard<'static, ()> {
    static GUARD: OnceLock<Mutex<()>> = OnceLock::new();
    GUARD
        .get_or_init(|| Mutex::new(()))
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner())
}

#[test]
fn contract_whale_response_keeps_eth_flow_separate_from_btc() {
    let _guard = contract_whale_test_guard();
    reset_contract_whale_runtime_config();
    let mut eth_window = high_conviction_window();
    eth_window.symbol = "ETH-PERP".to_string();
    eth_window.aggressive_buy_btc = 24_000.0;
    eth_window.aggressive_sell_btc = 2_000.0;
    eth_window.aggressive_buy_usd = 84_000_000.0;
    eth_window.aggressive_sell_usd = 7_000_000.0;
    eth_window.net_aggressive_btc = 22_000.0;
    eth_window.abs_aggressive_btc = 26_000.0;
    eth_window.venue_breakdown = BTreeMap::from([
        ("binance".to_string(), breakdown(14_000.0, 1_000.0)),
        ("okx".to_string(), breakdown(10_000.0, 1_000.0)),
    ]);
    let flow_state = FlowState {
        symbol: "ETH-PERP".to_string(),
        updated_at: 1_700_000_015_000,
        windows: BTreeMap::from([("15000".to_string(), eth_window)]),
    };

    let eth_response = build_contract_whale_response(&flow_state, "ETH", 50, None, true, true);
    let btc_response = build_contract_whale_response(&flow_state, "BTC", 50, None, true, true);

    assert_eq!(eth_response.items.len(), 1);
    assert_eq!(eth_response.items[0].symbol, "ETH");
    assert!(eth_response.summary.signal_count >= 1);
    assert!(btc_response.items.is_empty());
}

#[test]
fn contract_whale_response_is_calm_without_contract_flow_data() {
    let _guard = contract_whale_test_guard();
    let flow_state = FlowState {
        symbol: "BTC-PERP".to_string(),
        updated_at: 1_700_000_000_000,
        windows: BTreeMap::new(),
    };

    let response = build_contract_whale_response(&flow_state, "BTC", 50, None, true, true);

    assert_eq!(response.summary.status, "calm");
    assert_eq!(response.summary.health_status, "unhealthy");
    assert_eq!(response.summary.latest_direction, "neutral");
    assert_eq!(response.summary.signal_count, 0);
    assert!(response.summary.enabled);
    assert!(response.summary.dry_run);
    assert!(response.items.is_empty());
    assert_eq!(response.filter.get("readOnly"), Some(&"true".to_string()));
    assert_eq!(response.filter.get("enabled"), Some(&"true".to_string()));
    assert_eq!(response.filter.get("dryRun"), Some(&"true".to_string()));
}

#[test]
fn contract_whale_response_filters_latest_signals_by_severity() {
    let _guard = contract_whale_test_guard();
    reset_contract_whale_runtime_config();
    let flow_state = FlowState {
        symbol: "BTC-PERP".to_string(),
        updated_at: 1_700_000_015_000,
        windows: BTreeMap::from([("15000".to_string(), high_conviction_window())]),
    };

    let response = build_contract_whale_response(&flow_state, "BTC", 50, Some("high"), true, true);

    assert_eq!(response.items.len(), 1);
    assert_eq!(response.items[0].severity, ContractWhaleSeverity::High);
    assert_eq!(response.summary.status, "active");
    assert_eq!(response.summary.latest_direction, "buy");
    assert_eq!(response.summary.latest_signal_at, Some(1_700_000_015_000));
    assert_eq!(response.summary.health_status, "healthy");
    assert_eq!(response.summary.threshold_profile, "binance_bitfinex");
    assert_eq!(response.summary.active_exchange_count, 2);
    assert_eq!(
        response.summary.enabled_exchanges,
        vec!["binance".to_string(), "bitfinex".to_string()]
    );
    assert_eq!(response.summary.disabled_exchanges, vec!["okx".to_string()]);
    let coinbase_status = response
        .summary
        .exchanges
        .get("coinbase")
        .expect("coinbase status");
    assert!(!coinbase_status.connected);
    assert_eq!(coinbase_status.status, "spot_only");
    assert!(coinbase_status.platform_enabled);
    assert!(!coinbase_status.contract_enabled);
    assert_eq!(coinbase_status.enabled_markets, vec!["spot".to_string()]);
    assert_eq!(
        coinbase_status.market_roles.get("spot"),
        Some(&"primary".to_string())
    );
    assert_eq!(response.summary.trend_60s.buy_volume_btc, 0.0);
    assert_eq!(
        response
            .summary
            .exchanges
            .get("binance")
            .expect("binance status")
            .last_trade_at,
        Some(1_700_000_015_000)
    );
    let okx_status = response.summary.exchanges.get("okx").expect("okx status");
    assert!(!okx_status.connected);
    assert_eq!(okx_status.status, "disabled");
    assert!(response.summary.enabled);
    assert!(response.summary.dry_run);
    assert_eq!(response.summary.contract_data_quality, 95);
    assert_eq!(response.summary.spot_data_quality, 78);
    assert_eq!(response.summary.overall_data_quality, 88);
    assert!(!response.items[0].discord_sent);
    assert_eq!(response.items[0].market_type.as_key(), "perp");
    assert_eq!(response.items[0].threshold_profile, "binance_bitfinex");
    assert!(!response.items[0]
        .active_sources
        .contract
        .iter()
        .any(|entry| entry.exchange == "coinbase"
            && entry.market_type == ContractWhaleMarketType::Perp));
    assert!(
        !response.items[0]
            .active_sources
            .contract
            .iter()
            .any(|entry| entry.exchange == "okx"
                && entry.market_type == ContractWhaleMarketType::Perp)
    );
    assert!(response.items[0]
        .active_sources
        .spot
        .iter()
        .any(|entry| entry.exchange == "coinbase" && entry.status == "spot_only"));
}

#[test]
fn contract_whale_summary_includes_60s_trend_and_health() {
    let _guard = contract_whale_test_guard();
    reset_contract_whale_runtime_config();
    set_contract_whale_runtime_config(three_exchange_runtime_config());
    let now = 1_700_000_015_000;
    let mut sixty_sec = high_conviction_window();
    sixty_sec.window_ms = 60_000;
    sixty_sec.now_ts = now;
    sixty_sec.aggressive_buy_btc = 6_200.0;
    sixty_sec.aggressive_sell_btc = 3_800.0;
    sixty_sec.aggressive_buy_usd = 434_000_000.0;
    sixty_sec.aggressive_sell_usd = 266_000_000.0;
    sixty_sec.venue_breakdown = BTreeMap::from([
        ("binance".to_string(), breakdown(3_200.0, 1_800.0)),
        ("okx".to_string(), breakdown(2_000.0, 1_400.0)),
        ("bitfinex".to_string(), breakdown(1_000.0, 600.0)),
    ]);
    let flow_state = FlowState {
        symbol: "BTC-PERP".to_string(),
        updated_at: now,
        windows: BTreeMap::from([("60000".to_string(), sixty_sec)]),
    };

    let response = build_contract_whale_response(&flow_state, "BTC", 50, None, true, true);

    assert_eq!(response.summary.health_status, "healthy");
    assert_eq!(response.summary.threshold_profile, "three_exchange");
    assert_eq!(response.summary.trend_60s.buy_volume_btc, 6_200.0);
    assert_eq!(response.summary.trend_60s.sell_volume_btc, 3_800.0);
    assert_eq!(response.summary.trend_60s.total_volume_btc, 10_000.0);
    assert_eq!(response.summary.trend_60s.net_volume_btc, 2_400.0);
    assert!((response.summary.trend_60s.buy_ratio - 0.62).abs() < 0.0001);
    assert!((response.summary.trend_60s.sell_ratio - 0.38).abs() < 0.0001);
    assert_eq!(response.summary.contract_data_quality, 95);
    assert_eq!(response.summary.spot_data_quality, 78);
    reset_contract_whale_runtime_config();
}

#[test]
fn contract_whale_summary_keeps_coinbase_perp_out_of_profile_until_ready() {
    let _guard = contract_whale_test_guard();
    reset_contract_whale_runtime_config();
    let mut config = ContractWhaleRuntimeConfig::default();
    config.exchanges.coinbase.perp.enabled = true;
    set_contract_whale_runtime_config(config);

    let flow_state = FlowState {
        symbol: "BTC-PERP".to_string(),
        updated_at: 1_700_000_015_000,
        windows: BTreeMap::from([("15000".to_string(), high_conviction_window())]),
    };

    let response = build_contract_whale_response(&flow_state, "BTC", 50, Some("high"), true, true);

    assert_eq!(response.summary.threshold_profile, "binance_bitfinex");
    assert_eq!(
        response.summary.threshold_profile_reason,
        "coinbase_perp_auth_missing"
    );
    assert_eq!(response.summary.active_exchange_count, 2);
    assert_eq!(
        response.summary.configured_contract_sources,
        vec![
            "binance".to_string(),
            "bitfinex".to_string(),
            "coinbase".to_string()
        ]
    );
    assert_eq!(
        response.summary.eligible_contract_sources,
        vec!["binance".to_string(), "bitfinex".to_string()]
    );
    assert_eq!(
        response.summary.enabled_exchanges,
        vec!["binance".to_string(), "bitfinex".to_string()]
    );
    let coinbase_status = response
        .summary
        .exchanges
        .get("coinbase")
        .expect("coinbase status");
    assert!(coinbase_status.contract_enabled);
    assert!(coinbase_status
        .enabled_markets
        .contains(&"perp".to_string()));
    assert_eq!(coinbase_status.status, "disconnected");
    assert_eq!(response.summary.contract_data_quality, 95);

    reset_contract_whale_runtime_config();
}

#[test]
fn contract_whale_response_includes_dynamic_and_percentile_quality_baselines() {
    let _guard = contract_whale_test_guard();
    reset_contract_whale_runtime_config();
    set_contract_whale_runtime_config(three_exchange_runtime_config());
    let flow_state = FlowState {
        symbol: "BTC-PERP".to_string(),
        updated_at: 1_700_000_015_000,
        windows: BTreeMap::from([("15000".to_string(), high_conviction_window())]),
    };
    let baselines = BTreeMap::from([(
        15,
        ContractWhaleQualityBaseline {
            dynamic_multiple: Some(10.4),
            dynamic_baseline_btc: Some(300.0),
            dynamic_threshold_level: "s".to_string(),
            percentile_level: Some(99.9),
        },
    )]);
    let liquidations = BTreeMap::from([(
        15,
        ContractWhaleLiquidationContext {
            long_liq_btc: 1_200.0,
            short_liq_btc: 0.0,
            total_liq_btc: 1_200.0,
            liq_notional_usd: 84_000_000.0,
            liq_to_volume_ratio: Some(0.20),
        },
    )]);
    let market_context = ContractWhaleMarketContext {
        context_expected: true,
        ct_val_available: true,
        oi_available: true,
        funding_available: true,
        oi_change_1m_btc: Some(250.0),
        oi_change_5m_btc: Some(900.0),
        oi_change_pct: Some(1.2),
        oi_bias: Some("rising".to_string()),
        funding_rate: Some(0.00018),
        funding_bias: Some("long".to_string()),
    };

    let response = build_contract_whale_response_with_runtime_and_baselines(
        &flow_state,
        "BTC",
        50,
        None,
        true,
        true,
        ContractWhaleResponseRuntime {
            venue_health: None,
            baselines: &baselines,
            liquidations: &liquidations,
            market_context: &market_context,
            booted_at_ms: None,
        },
    );

    assert_eq!(response.items.len(), 1);
    assert_eq!(response.items[0].severity, ContractWhaleSeverity::S);
    assert_eq!(response.items[0].dynamic_multiple, Some(10.4));
    assert_eq!(response.items[0].dynamic_baseline_btc, Some(300.0));
    assert_eq!(response.items[0].dynamic_threshold_level, "s");
    assert_eq!(response.items[0].percentile_level, Some(99.9));
    assert!(response.items[0].multi_exchange_confirmed);
    assert_eq!(response.items[0].liquidation_long_btc, 1_200.0);
    assert_eq!(response.items[0].oi_change_5m_btc, Some(900.0));
    assert_eq!(response.items[0].funding_bias.as_deref(), Some("long"));
    reset_contract_whale_runtime_config();
}

#[test]
fn contract_whale_summary_exposes_warmup_and_disables_push_during_warmup() {
    let _guard = contract_whale_test_guard();
    let now = 1_700_000_015_000;
    let flow_state = FlowState {
        symbol: "BTC-PERP".to_string(),
        updated_at: now,
        windows: BTreeMap::from([("15000".to_string(), high_conviction_window())]),
    };
    let response = build_contract_whale_response_with_runtime_and_baselines(
        &flow_state,
        "BTC",
        50,
        None,
        true,
        false,
        ContractWhaleResponseRuntime {
            venue_health: None,
            baselines: &BTreeMap::new(),
            liquidations: &BTreeMap::new(),
            market_context: &ContractWhaleMarketContext::default(),
            booted_at_ms: Some(now - 10_000),
        },
    );

    assert!(response.summary.warmup);
    assert_eq!(response.summary.status, "warmup");
    assert_eq!(response.summary.warmup_remaining_ms, Some(50_000));
    assert_eq!(response.summary.warmup_until_ms, Some(now + 50_000));
    assert_eq!(response.items.len(), 1);
    assert!(!response.items[0].discord_eligible);
    assert_eq!(response.items[0].discord_reason, "warmup_collect_only");
}

#[test]
fn contract_whale_response_merges_same_wave_multi_window_signals() {
    let _guard = contract_whale_test_guard();
    let now = 1_700_000_015_000;
    let mut five_sec = high_conviction_window();
    five_sec.window_ms = 5_000;
    five_sec.now_ts = now;
    let flow_state = FlowState {
        symbol: "BTC-PERP".to_string(),
        updated_at: now,
        windows: BTreeMap::from([
            ("5000".to_string(), five_sec),
            ("15000".to_string(), high_conviction_window()),
        ]),
    };
    let baselines = BTreeMap::from([
        (
            5,
            ContractWhaleQualityBaseline {
                dynamic_multiple: Some(7.2),
                dynamic_baseline_btc: Some(200.0),
                dynamic_threshold_level: "critical".to_string(),
                percentile_level: Some(99.5),
            },
        ),
        (
            15,
            ContractWhaleQualityBaseline {
                dynamic_multiple: Some(10.4),
                dynamic_baseline_btc: Some(300.0),
                dynamic_threshold_level: "s".to_string(),
                percentile_level: Some(99.9),
            },
        ),
    ]);

    let response = build_contract_whale_response_with_runtime_and_baselines(
        &flow_state,
        "BTC",
        50,
        None,
        true,
        false,
        ContractWhaleResponseRuntime {
            venue_health: None,
            baselines: &baselines,
            liquidations: &BTreeMap::new(),
            market_context: &ContractWhaleMarketContext::default(),
            booted_at_ms: Some(now - 61_000),
        },
    );

    assert_eq!(response.items.len(), 1);
    assert_eq!(response.items[0].severity, ContractWhaleSeverity::S);
    assert_eq!(response.items[0].window_sec, 15);
    assert_eq!(response.summary.signal_count, 1);
    assert_eq!(response.items[0].merged_from.len(), 1);
    assert!(response.items[0]
        .merged_from
        .iter()
        .any(|id| id.contains("BTC:5:")));
}

#[test]
fn contract_whale_response_returns_disabled_empty_state_when_config_disabled() {
    let _guard = contract_whale_test_guard();
    let flow_state = FlowState {
        symbol: "BTC-PERP".to_string(),
        updated_at: 1_700_000_015_000,
        windows: BTreeMap::from([("15000".to_string(), high_conviction_window())]),
    };

    let response = build_contract_whale_response(&flow_state, "BTC", 50, None, false, true);

    assert_eq!(response.summary.status, "disabled");
    assert_eq!(response.summary.direction, "disabled");
    assert_eq!(response.summary.latest_direction, "disabled");
    assert_eq!(
        response.summary.latest_severity,
        ContractWhaleSeverity::Calm
    );
    assert!(
        !response
            .summary
            .exchanges
            .get("binance")
            .expect("binance status")
            .connected
    );
    assert!(!response.summary.enabled);
    assert!(response.summary.dry_run);
    assert_eq!(response.summary.contract_data_quality, 0);
    assert_eq!(response.summary.spot_data_quality, 0);
    assert_eq!(response.summary.overall_data_quality, 0);
    assert!(response.items.is_empty());
    assert_eq!(response.filter.get("enabled"), Some(&"false".to_string()));
    assert_eq!(response.filter.get("dryRun"), Some(&"true".to_string()));
}

#[test]
fn contract_whale_history_response_can_surface_coinbase_perp_disabled_meta() {
    let _guard = contract_whale_test_guard();
    let response = btc_toxic_flow_monitor_rs::api::contract_whale_routes::build_contract_whale_history_response(
        Vec::new(),
        "BTC",
        50,
        None,
        true,
        true,
        Some(
            btc_toxic_flow_monitor_rs::contract_whale_monitor::types::ContractWhaleResponseMeta {
                exchange: Some("coinbase".to_string()),
                market_type: Some("perp".to_string()),
                exchange_status: Some("spot_only".to_string()),
                reason: Some("coinbase_perp_disabled".to_string()),
            },
        ),
    );

    assert!(response.items.is_empty());
    assert_eq!(
        response
            .meta
            .as_ref()
            .and_then(|meta| meta.exchange.as_deref()),
        Some("coinbase")
    );
    assert_eq!(
        response
            .meta
            .as_ref()
            .and_then(|meta| meta.exchange_status.as_deref()),
        Some("spot_only")
    );
    assert_eq!(
        response
            .meta
            .as_ref()
            .and_then(|meta| meta.reason.as_deref()),
        Some("coinbase_perp_disabled")
    );
}

#[test]
fn contract_whale_latest_response_clamps_limit_and_keeps_persisted_items() {
    let _guard = contract_whale_test_guard();
    let signal = persisted_signal(1_700_000_030_000, ContractWhaleSeverity::Critical);
    let mut spot_signal = persisted_signal(1_700_000_031_000, ContractWhaleSeverity::S);
    spot_signal.id = "contract-whale:BTC:15:spot-row".to_string();
    spot_signal.market_type = ContractWhaleMarketType::Spot;
    let response = build_contract_whale_items_response(
        vec![signal.clone(), spot_signal],
        "BTC",
        200,
        true,
        true,
        BTreeMap::new(),
        ContractWhaleTrend60s::default(),
    );

    assert_eq!(response.items.len(), 1);
    assert_eq!(response.items[0].id, signal.id);
    assert_eq!(response.items[0].market_type, ContractWhaleMarketType::Perp);
    assert_eq!(response.summary.latest_signal_at, Some(signal.ts));
    assert_eq!(response.filter.get("marketType"), Some(&"perp".to_string()));
}

#[test]
fn contract_whale_history_query_validates_filters_and_clamps_limit() {
    let _guard = contract_whale_test_guard();
    let query = ContractWhaleQuery {
        symbol: Some("btc".to_string()),
        severity: Some("critical".to_string()),
        signal_type: Some("aggressive_buy".to_string()),
        direction: Some("buy".to_string()),
        discord_sent: Some("true".to_string()),
        window_sec: Some("15".to_string()),
        exchange: Some("binance".to_string()),
        from: Some("1700000000000".to_string()),
        to: Some("1700086400000".to_string()),
        limit: Some("999".to_string()),
        offset: Some("25".to_string()),
    };

    let parsed = parse_history_query(&query).expect("valid query");

    assert_eq!(parsed.symbol.as_deref(), Some("BTC"));
    assert_eq!(parsed.severity, Some(ContractWhaleSeverity::Critical));
    assert_eq!(parsed.discord_sent, Some(true));
    assert_eq!(parsed.window_sec, Some(15));
    assert_eq!(parsed.exchange.as_deref(), Some("binance"));
    assert_eq!(parsed.limit, 200);
    assert_eq!(parsed.offset, 25);
}

#[test]
fn contract_whale_history_query_rejects_invalid_params() {
    let _guard = contract_whale_test_guard();
    let invalid_severity = ContractWhaleQuery {
        severity: Some("panic".to_string()),
        ..empty_query()
    };
    assert_eq!(
        parse_history_query(&invalid_severity)
            .expect_err("invalid severity")
            .0,
        axum::http::StatusCode::BAD_REQUEST
    );

    let invalid_range = ContractWhaleQuery {
        from: Some("200".to_string()),
        to: Some("100".to_string()),
        ..empty_query()
    };
    assert_eq!(
        parse_history_query(&invalid_range)
            .expect_err("invalid range")
            .0,
        axum::http::StatusCode::BAD_REQUEST
    );

    let invalid_window = ContractWhaleQuery {
        window_sec: Some("30".to_string()),
        ..empty_query()
    };
    assert_eq!(
        parse_history_query(&invalid_window)
            .expect_err("invalid window")
            .0,
        axum::http::StatusCode::BAD_REQUEST
    );
}

#[test]
fn contract_whale_metrics_text_is_prometheus_safe() {
    let _guard = contract_whale_test_guard();
    let signal = persisted_signal(1_700_000_030_000, ContractWhaleSeverity::Critical);
    let response = build_contract_whale_items_response(
        vec![signal],
        "BTC",
        50,
        true,
        true,
        default_test_exchanges(),
        ContractWhaleTrend60s {
            buy_volume_btc: 10.0,
            sell_volume_btc: 20.0,
            total_volume_btc: 30.0,
            net_volume_btc: -10.0,
            dominance: 10.0 / 30.0,
            buy_ratio: 10.0 / 30.0,
            sell_ratio: 20.0 / 30.0,
            updated_at_ms: Some(1_700_000_030_000),
        },
    );

    let metrics = build_contract_whale_metrics_text(
        true,
        &response.summary.exchanges,
        &response.summary.trend_60s,
        &response.items,
    );

    assert!(metrics.contains("cwm_ws_connected{exchange=\"binance\"}"));
    assert!(metrics.contains("cwm_signals_generated_total"));
    assert!(metrics.contains("cwm_discord_skipped_total"));
    assert!(metrics.contains("cwm_data_quality"));
    assert!(!metrics.to_ascii_lowercase().contains("webhook"));
    assert!(!metrics.to_ascii_lowercase().contains("token"));
}

fn high_conviction_window() -> FlowWindow {
    FlowWindow {
        symbol: "BTC-PERP".to_string(),
        window_ms: 15_000,
        now_ts: 1_700_000_015_000,
        aggressive_buy_btc: 5_500.0,
        aggressive_sell_btc: 600.0,
        aggressive_buy_usd: 385_000_000.0,
        aggressive_sell_usd: 42_000_000.0,
        net_aggressive_btc: 4_900.0,
        abs_aggressive_btc: 4_900.0,
        trade_count: 30,
        buy_trade_count: 24,
        sell_trade_count: 6,
        avg_trade_size_btc: 203.3,
        max_trade_size_btc: 900.0,
        venue_breakdown: BTreeMap::from([
            ("binance".to_string(), breakdown(3_000.0, 300.0)),
            ("okx".to_string(), breakdown(2_000.0, 200.0)),
            ("bitfinex".to_string(), breakdown(500.0, 100.0)),
        ]),
        mid_start: Some(70_000.0),
        mid_end: Some(70_217.0),
        price_move_bps: Some(31.0),
        spread_bps_median: Some(2.0),
        imbalance_10bps_median: Some(0.2),
        data_quality: DataQuality {
            has_trades: true,
            has_books: true,
            active_venues: vec!["binance".to_string(), "okx".to_string()],
            stale_venues: vec![],
        },
    }
}

fn breakdown(buy: f64, sell: f64) -> VenueFlowBreakdown {
    VenueFlowBreakdown {
        aggressive_buy_btc: buy,
        aggressive_sell_btc: sell,
        aggressive_buy_usd: buy * 70_000.0,
        aggressive_sell_usd: sell * 70_000.0,
        net_aggressive_btc: buy - sell,
        abs_aggressive_btc: (buy - sell).abs(),
        trade_count: 10,
        buy_trade_count: 8,
        sell_trade_count: 2,
        last_trade_ts: Some(1_700_000_015_000),
    }
}

fn persisted_signal(
    ts: i64,
    severity: ContractWhaleSeverity,
) -> btc_toxic_flow_monitor_rs::contract_whale_monitor::types::ContractWhaleSignal {
    let flow_state = FlowState {
        symbol: "BTC-PERP".to_string(),
        updated_at: ts,
        windows: BTreeMap::from([("15000".to_string(), high_conviction_window())]),
    };
    let response = build_contract_whale_response(&flow_state, "BTC", 50, None, true, true);
    let mut signal = response.items[0].clone();
    signal.ts = ts;
    signal.id = format!("contract-whale:BTC:15:{ts}:buy");
    signal.severity = severity;
    signal
}

fn empty_query() -> ContractWhaleQuery {
    ContractWhaleQuery {
        limit: None,
        symbol: None,
        severity: None,
        signal_type: None,
        direction: None,
        discord_sent: None,
        window_sec: None,
        exchange: None,
        from: None,
        to: None,
        offset: None,
    }
}

fn default_test_exchanges() -> BTreeMap<
    String,
    btc_toxic_flow_monitor_rs::contract_whale_monitor::types::ContractWhaleExchangeStatus,
> {
    BTreeMap::from([
        (
            "binance".to_string(),
            btc_toxic_flow_monitor_rs::contract_whale_monitor::types::ContractWhaleExchangeStatus {
                connected: true,
                status: "connected".to_string(),
                last_trade_at: Some(1_700_000_030_000),
                latency_ms: Some(100),
                reconnect_count: 0,
                platform_enabled: true,
                contract_enabled: true,
                enabled_markets: vec!["perp".to_string()],
                market_roles: BTreeMap::from([("perp".to_string(), "primary".to_string())]),
            },
        ),
        (
            "okx".to_string(),
            btc_toxic_flow_monitor_rs::contract_whale_monitor::types::ContractWhaleExchangeStatus {
                connected: true,
                status: "connected".to_string(),
                last_trade_at: Some(1_700_000_030_000),
                latency_ms: Some(120),
                reconnect_count: 1,
                platform_enabled: true,
                contract_enabled: true,
                enabled_markets: vec!["perp".to_string()],
                market_roles: BTreeMap::from([("perp".to_string(), "confirmation".to_string())]),
            },
        ),
    ])
}

fn three_exchange_runtime_config() -> ContractWhaleRuntimeConfig {
    let mut config = ContractWhaleRuntimeConfig::default();
    config.exchanges.okx.enabled = true;
    config.exchanges.okx.perp.enabled = true;
    config.exchanges.okx.funding.enabled = true;
    config.exchanges.okx.oi.enabled = true;
    config.exchanges.okx.liquidation.enabled = true;
    config
}
