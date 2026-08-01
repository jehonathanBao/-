use std::{
    collections::BTreeMap,
    sync::{Mutex, MutexGuard, OnceLock},
};

use btc_toxic_flow_monitor_rs::{
    api::contract_whale_routes::{
        build_contract_whale_history_response, build_contract_whale_intelligence_response,
        build_contract_whale_items_response, build_contract_whale_metrics_text,
        build_contract_whale_response, build_contract_whale_response_with_runtime_and_baselines,
        build_trading_decision_response, encode_contract_history_cursor, parse_history_query,
        ContractWhaleQualityBaseline, ContractWhaleQuery, ContractWhaleResponseRuntime,
    },
    contract_whale_monitor::{
        config::{
            reset_contract_whale_runtime_config, set_contract_whale_runtime_config,
            ContractWhaleRuntimeConfig,
        },
        types::{
            ContractWhaleDirection, ContractWhaleLiquidationContext, ContractWhaleMarketContext,
            ContractWhaleMarketStructureLite, ContractWhaleMarketType,
            ContractWhaleNoiseSuppressionSummary, ContractWhalePriceResponseType,
            ContractWhaleSeverity, ContractWhaleSignalType, ContractWhaleTrend60s,
        },
    },
    core_event::final_store::final_event_store::{
        build_final_event_store_response_from_contract_whale_response,
        build_final_events_from_contract_whale_signals, VolumeDisplayContext,
    },
    types::flow::{DataQuality, FlowState, FlowWindow, VenueFlowBreakdown},
    types::market::{Venue, VenueConnectionStatus, VenueHealth},
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
        (
            "binance".to_string(),
            breakdown_at_price(14_000.0, 1_000.0, 3_500.0),
        ),
        (
            "okx".to_string(),
            breakdown_at_price(10_000.0, 1_000.0, 3_500.0),
        ),
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
fn contract_whale_exchange_health_does_not_use_another_symbols_global_trade() {
    let _guard = contract_whale_test_guard();
    reset_contract_whale_runtime_config();
    let now = 1_700_000_015_000;
    let flow_state = FlowState {
        symbol: "ETH-PERP".to_string(),
        updated_at: now,
        windows: BTreeMap::new(),
    };
    let mut binance = VenueHealth::from_config(Venue::Binance, true);
    binance.status = VenueConnectionStatus::Connected;
    binance.ws_connected = true;
    binance.last_trade_ts = Some(now);
    let venue_health = BTreeMap::from([("binance".to_string(), binance)]);

    let response = build_contract_whale_response_with_runtime_and_baselines(
        &flow_state,
        "ETH",
        50,
        None,
        true,
        true,
        ContractWhaleResponseRuntime {
            venue_health: Some(&venue_health),
            baselines: &BTreeMap::new(),
            liquidations: &BTreeMap::new(),
            market_context: &ContractWhaleMarketContext::default(),
            booted_at_ms: None,
        },
    );

    let status = response
        .summary
        .exchanges
        .get("binance")
        .expect("binance status");
    assert!(!status.connected);
    assert_eq!(status.status, "waiting_for_data");
    assert_eq!(status.last_trade_at, None);
}

#[test]
fn contract_whale_exchange_health_prioritizes_reconnecting_over_stale_symbol_flow() {
    let _guard = contract_whale_test_guard();
    reset_contract_whale_runtime_config();
    let now = btc_toxic_flow_monitor_rs::normalizers::trade::now_ms();
    let mut window = high_conviction_window();
    window.now_ts = now.saturating_sub(60_000);
    let mut binance_flow = breakdown_at_price(600.0, 100.0, 70_000.0);
    binance_flow.last_trade_ts = Some(now.saturating_sub(60_000));
    window.venue_breakdown = BTreeMap::from([("binance".to_string(), binance_flow)]);
    let flow_state = FlowState {
        symbol: "BTC-PERP".to_string(),
        updated_at: now,
        windows: BTreeMap::from([("15000".to_string(), window)]),
    };
    let mut binance = VenueHealth::from_config(Venue::Binance, true);
    binance.status = VenueConnectionStatus::Reconnecting;
    binance.ws_connected = false;
    let venue_health = BTreeMap::from([("binance".to_string(), binance)]);

    let response = build_contract_whale_response_with_runtime_and_baselines(
        &flow_state,
        "BTC",
        50,
        None,
        true,
        true,
        ContractWhaleResponseRuntime {
            venue_health: Some(&venue_health),
            baselines: &BTreeMap::new(),
            liquidations: &BTreeMap::new(),
            market_context: &ContractWhaleMarketContext::default(),
            booted_at_ms: None,
        },
    );

    let status = response
        .summary
        .exchanges
        .get("binance")
        .expect("binance status");
    assert!(!status.connected);
    assert_eq!(status.status, "reconnecting");
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

    let response = build_contract_whale_response_with_runtime_and_baselines(
        &flow_state,
        "BTC",
        50,
        Some("high"),
        true,
        true,
        ContractWhaleResponseRuntime {
            venue_health: None,
            baselines: &BTreeMap::from([(
                15,
                ContractWhaleQualityBaseline {
                    dynamic_multiple: Some(5.5),
                    dynamic_baseline_btc: Some(490.0),
                    dynamic_threshold_level: "high".to_string(),
                    percentile_level: Some(99.0),
                },
            )]),
            liquidations: &BTreeMap::new(),
            market_context: &ContractWhaleMarketContext::default(),
            booted_at_ms: None,
        },
    );

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
fn contract_whale_summary_exposes_requested_symbol_unit_for_eth() {
    let _guard = contract_whale_test_guard();
    reset_contract_whale_runtime_config();
    set_contract_whale_runtime_config(three_exchange_runtime_config());
    let now = 1_700_000_020_000;
    let mut sixty_sec = high_conviction_window();
    sixty_sec.symbol = "ETH-PERP".to_string();
    sixty_sec.window_ms = 60_000;
    sixty_sec.now_ts = now;
    sixty_sec.aggressive_buy_btc = 8_739.0;
    sixty_sec.aggressive_sell_btc = 8_127.0;
    sixty_sec.aggressive_buy_usd = 14_642_000.0;
    sixty_sec.aggressive_sell_usd = 13_615_000.0;
    sixty_sec.venue_breakdown = BTreeMap::from([
        ("binance".to_string(), breakdown(8_739.0, 8_127.0)),
        ("bitfinex".to_string(), breakdown(1.0, 2.0)),
    ]);
    let flow_state = FlowState {
        symbol: "ETH-PERP".to_string(),
        updated_at: now,
        windows: BTreeMap::from([("60000".to_string(), sixty_sec)]),
    };

    let response = build_contract_whale_response(&flow_state, "ETH", 50, None, true, true);

    assert_eq!(response.summary.symbol, "ETH");
    assert_eq!(response.summary.base_asset, "ETH");
    assert_eq!(response.summary.quantity_unit, "ETH");
    assert_eq!(response.summary.trend_60s.symbol, "ETH");
    assert_eq!(response.summary.trend_60s.base_asset, "ETH");
    assert_eq!(response.summary.trend_60s.quantity_unit, "ETH");
    assert!(response.summary.trend_60s.total_volume_btc > 0.0);
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
        evidence_degraded: false,
        evidence_reason: None,
        oi_available: true,
        oi_reason: None,
        funding_available: true,
        funding_reason: None,
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
    assert!(response.items[0].total_volume_btc >= 3_900.0);
    assert!(response.items[0].total_volume_btc <= 6_100.0);
    assert!(response.items[0].net_volume_btc >= 3_100.0);
    assert!(response.items[0].net_volume_btc <= 4_900.0);
    assert_eq!(response.items[0].merged_from.len(), 1);
    assert_eq!(response.summary.noise_suppression.raw_candidates, 2);
    assert_eq!(response.summary.noise_suppression.merged_events, 1);
    assert_eq!(response.summary.noise_suppression.lifecycle_events, 1);
    assert_eq!(response.summary.noise_suppression.filtered_events, 1);
    assert_eq!(response.summary.noise_suppression.suppressed_duplicates, 1);
    assert_eq!(response.summary.noise_suppression.tradeable_setups, 1);
    assert_eq!(response.summary.trade_opportunities.len(), 1);
    assert_eq!(response.summary.trade_opportunities[0].rank, 1);
    assert_eq!(response.summary.trade_opportunities[0].action, "LONG");
    assert_eq!(
        response.summary.trade_opportunities[0].direction_bias,
        "buy"
    );
    assert!(response.summary.trade_opportunities[0].trade_score >= 70);
    assert!(response.items[0]
        .merged_from
        .iter()
        .any(|id| id.contains("BTC:5:")));
}

#[test]
fn trading_decision_response_ranks_tradeable_setup_and_emits_no_trade_zone_for_chop() {
    let _guard = contract_whale_test_guard();
    let mut long_setup = persisted_signal(1_700_000_030_000, ContractWhaleSeverity::Critical);
    long_setup.id = "contract-whale:BTC:15:1700000030000:long".to_string();
    long_setup.signal_type = ContractWhaleSignalType::AggressiveBuy;
    long_setup.direction = ContractWhaleDirection::Buy;
    long_setup.total_volume_btc = 4_820.0;
    long_setup.net_volume_btc = 3_260.0;
    long_setup.total_notional_usd = 337_000_000.0;
    long_setup.dominance = 0.676;
    long_setup.score = 88;
    long_setup.main_force_score = Some(87);
    long_setup.price_move_pct = Some(0.31);
    long_setup.price_response_type = ContractWhalePriceResponseType::TrendFollowUp;
    long_setup.current_market_price_usd = Some(69_917.0);
    long_setup.order_price_usd = Some(69_880.0);
    long_setup.merged_from = vec!["contract-whale:BTC:5:1700000005000:long".to_string()];
    long_setup.multi_exchange_confirmed = true;
    long_setup.event_lifecycle.update_count = 3;
    long_setup.event_quality.quality_score = 0.86;

    let mut chop = persisted_signal(1_700_000_060_000, ContractWhaleSeverity::Medium);
    chop.id = "contract-whale:BTC:5:1700000060000:chop".to_string();
    chop.signal_type = ContractWhaleSignalType::UpsideSuppression;
    chop.direction = ContractWhaleDirection::Sell;
    chop.total_volume_btc = 180.0;
    chop.net_volume_btc = 42.0;
    chop.total_notional_usd = 11_000_000.0;
    chop.dominance = 0.23;
    chop.score = 41;
    chop.main_force_score = Some(38);
    chop.price_move_pct = Some(0.01);
    chop.price_response_type = ContractWhalePriceResponseType::NoClearResponse;
    chop.current_market_price_usd = Some(69_920.0);
    chop.order_price_usd = Some(69_910.0);
    chop.merged_from.clear();
    chop.multi_exchange_confirmed = false;
    chop.event_lifecycle.update_count = 1;
    chop.event_quality.quality_score = 0.58;

    let decision = build_trading_decision_response(
        "BTC",
        &[long_setup, chop],
        &ContractWhaleMarketStructureLite {
            status: "confirmed".to_string(),
            regime_type: "main_force_long_build".to_string(),
            main_force_score: 84,
            confidence: 76,
            ..Default::default()
        },
        ContractWhaleNoiseSuppressionSummary {
            raw_candidates: 2,
            merged_events: 2,
            lifecycle_events: 2,
            filtered_events: 1,
            tradeable_setups: 1,
            suppressed_duplicates: 0,
            noise_reduction_pct: 50,
        },
        1_700_000_090_000,
    );

    assert_eq!(decision.symbol, "BTC");
    assert_eq!(decision.market_bias, "BULLISH");
    assert!(decision.bias_confidence >= 70);
    assert_eq!(decision.top_setups.len(), 1);
    assert_eq!(decision.top_setups[0].direction_bias, "BULLISH_BIAS");
    assert!(decision.top_setups[0].score >= 70);
    assert!(!decision.top_setups[0].pressure_zone.label.is_empty());
    assert!(decision.top_setups[0].risk_boundary.price_level > 0.0);
    assert!(!decision.top_setups[0].reasons.is_empty());
    assert_eq!(decision.noise_suppression.tradeable_setups, 1);
    assert!(!decision.no_trade_zones.is_empty());
    assert!(!decision.no_trade_zones[0].reason.is_empty());
}

#[test]
fn institutional_analysis_response_surfaces_regime_strength_and_opportunities() {
    let _guard = contract_whale_test_guard();
    let mut trend_buy = persisted_signal(1_700_000_030_000, ContractWhaleSeverity::Critical);
    trend_buy.id = "contract-whale:BTC:15:1700000030000:trend-buy".to_string();
    trend_buy.signal_type = ContractWhaleSignalType::AggressiveBuy;
    trend_buy.direction = ContractWhaleDirection::Buy;
    trend_buy.total_volume_btc = 4_820.0;
    trend_buy.net_volume_btc = 3_260.0;
    trend_buy.total_notional_usd = 337_000_000.0;
    trend_buy.dominance = 0.676;
    trend_buy.score = 88;
    trend_buy.main_force_score = Some(87);
    trend_buy.price_move_pct = Some(0.31);
    trend_buy.price_response_type = ContractWhalePriceResponseType::TrendFollowUp;
    trend_buy.current_market_price_usd = Some(69_917.0);
    trend_buy.order_price_usd = Some(69_880.0);
    trend_buy.merged_from = vec!["contract-whale:BTC:5:1700000005000:trend-buy".to_string()];
    trend_buy.multi_exchange_confirmed = true;
    trend_buy.event_lifecycle.update_count = 3;
    trend_buy.event_quality.quality_score = 0.86;

    let mut absorption = persisted_signal(1_700_000_040_000, ContractWhaleSeverity::High);
    absorption.id = "contract-whale:BTC:5:1700000040000:absorption".to_string();
    absorption.signal_type = ContractWhaleSignalType::DownsideAbsorption;
    absorption.direction = ContractWhaleDirection::Absorption;
    absorption.total_volume_btc = 1_920.0;
    absorption.net_volume_btc = 1_080.0;
    absorption.total_notional_usd = 128_000_000.0;
    absorption.dominance = 0.562;
    absorption.score = 77;
    absorption.main_force_score = Some(74);
    absorption.price_move_pct = Some(0.09);
    absorption.price_response_type = ContractWhalePriceResponseType::DownsideAbsorption;
    absorption.current_market_price_usd = Some(69_905.0);
    absorption.order_price_usd = Some(69_860.0);
    absorption.merged_from = vec!["contract-whale:BTC:15:1700000035000:absorption".to_string()];
    absorption.multi_exchange_confirmed = true;
    absorption.event_lifecycle.update_count = 2;
    absorption.event_quality.quality_score = 0.81;

    let mut fakeout = persisted_signal(1_700_000_060_000, ContractWhaleSeverity::Medium);
    fakeout.id = "contract-whale:BTC:5:1700000060000:fakeout".to_string();
    fakeout.signal_type = ContractWhaleSignalType::UpsideSuppression;
    fakeout.direction = ContractWhaleDirection::Suppression;
    fakeout.total_volume_btc = 1_180.0;
    fakeout.net_volume_btc = -120.0;
    fakeout.total_notional_usd = 71_000_000.0;
    fakeout.dominance = 0.31;
    fakeout.score = 58;
    fakeout.main_force_score = Some(54);
    fakeout.price_move_pct = Some(0.01);
    fakeout.price_response_type = ContractWhalePriceResponseType::NoClearResponse;
    fakeout.current_market_price_usd = Some(69_998.0);
    fakeout.order_price_usd = Some(70_010.0);
    fakeout.merged_from.clear();
    fakeout.multi_exchange_confirmed = false;
    fakeout.event_lifecycle.update_count = 1;
    fakeout.event_quality.quality_score = 0.62;

    let response = build_contract_whale_intelligence_response(
        "BTC",
        &[trend_buy, absorption, fakeout],
        &ContractWhaleMarketStructureLite {
            status: "confirmed".to_string(),
            regime_type: "main_force_long_build".to_string(),
            main_force_score: 84,
            confidence: 76,
            ..Default::default()
        },
        ContractWhaleNoiseSuppressionSummary {
            raw_candidates: 3,
            merged_events: 3,
            lifecycle_events: 3,
            filtered_events: 1,
            tradeable_setups: 1,
            suppressed_duplicates: 0,
            noise_reduction_pct: 33,
        },
        1_700_000_090_000,
    );

    assert_eq!(response.symbol, "BTC");
    assert_eq!(response.market_regime.regime, "TRENDING_UP");
    assert!(response.market_regime.confidence >= 70);
    assert!(!response.market_regime.reason.is_empty());
    assert!(response.ranked_events.len() >= 2);
    assert!(response.ranked_events[0].strength_score >= response.ranked_events[1].strength_score);
    assert!(response
        .liquidity_behaviors
        .iter()
        .any(|item| item.behavior == "absorption"));
    assert!(response
        .liquidity_behaviors
        .iter()
        .any(|item| item.behavior == "fake_breakout"));
    assert!(response
        .opportunity_map
        .iter()
        .any(|item| item.zone_type == "absorption_zone"
            || item.zone_type == "breakout_pressure_zone"));
    assert_eq!(response.noise_suppression.filtered_events, 1);
    assert_eq!(
        response.signal_compression.top_signal_count,
        response.trade_ideas.len()
    );
    assert!(response.trade_ideas.len() <= 3);
    assert!(response.signal_compression.quality_score <= 100);
    assert!(!response.risk_context.fake_breakout_risk.is_empty());
}

#[test]
fn intelligence_response_includes_signal_compression_trade_ideas_and_risk_context() {
    let _guard = contract_whale_test_guard();
    let mut trend_buy = persisted_signal(1_700_000_015_000, ContractWhaleSeverity::High);
    trend_buy.signal_type = ContractWhaleSignalType::AggressiveBuy;
    trend_buy.direction = ContractWhaleDirection::Buy;
    trend_buy.total_volume_btc = 2_240.0;
    trend_buy.total_notional_usd = 146_000_000.0;
    trend_buy.net_volume_btc = 1_880.0;
    trend_buy.price_move_pct = Some(0.24);
    trend_buy.current_market_price_usd = Some(60_420.0);
    trend_buy.order_price_usd = Some(60_390.0);
    trend_buy.main_force_score = Some(88);
    trend_buy.dominance = 0.71;
    trend_buy.multi_exchange_confirmed = true;

    let response = build_contract_whale_intelligence_response(
        "BTC",
        &[trend_buy],
        &ContractWhaleMarketStructureLite {
            status: "confirmed".to_string(),
            regime_type: "main_force_long_build".to_string(),
            main_force_score: 84,
            confidence: 77,
            ..Default::default()
        },
        ContractWhaleNoiseSuppressionSummary {
            raw_candidates: 1,
            merged_events: 1,
            lifecycle_events: 1,
            filtered_events: 1,
            tradeable_setups: 1,
            suppressed_duplicates: 0,
            noise_reduction_pct: 0,
        },
        1_700_000_090_000,
    );

    assert_eq!(
        response.signal_compression.top_signal_count,
        response.trade_ideas.len()
    );
    assert!(response.signal_compression.quality_score <= 100);
    assert!(response.trade_ideas.len() <= 3);
    assert!(response.risk_context.no_trade_zones.len() <= 3);
}

#[test]
fn intelligence_response_uses_semantic_safe_decision_contract() {
    let _guard = contract_whale_test_guard();
    let mut trend_buy = persisted_signal(1_700_000_015_000, ContractWhaleSeverity::High);
    trend_buy.id = "contract-whale:BTC:15:semantic-safe-trend".to_string();
    trend_buy.signal_type = ContractWhaleSignalType::AggressiveBuy;
    trend_buy.direction = ContractWhaleDirection::Buy;
    trend_buy.total_volume_btc = 2_640.0;
    trend_buy.total_notional_usd = 168_000_000.0;
    trend_buy.net_volume_btc = 1_920.0;
    trend_buy.price_move_pct = Some(0.28);
    trend_buy.price_response_type = ContractWhalePriceResponseType::TrendFollowUp;
    trend_buy.current_market_price_usd = Some(60_420.0);
    trend_buy.order_price_usd = Some(60_390.0);
    trend_buy.main_force_score = Some(90);
    trend_buy.dominance = 0.73;
    trend_buy.multi_exchange_confirmed = true;
    trend_buy.event_lifecycle.update_count = 3;
    trend_buy.event_quality.quality_score = 0.88;
    trend_buy.merged_from = vec!["contract-whale:BTC:5:semantic-safe-trend".to_string()];

    let response = build_contract_whale_intelligence_response(
        "BTC",
        &[trend_buy],
        &ContractWhaleMarketStructureLite {
            status: "confirmed".to_string(),
            regime_type: "main_force_long_build".to_string(),
            main_force_score: 84,
            confidence: 77,
            ..Default::default()
        },
        ContractWhaleNoiseSuppressionSummary {
            raw_candidates: 1,
            merged_events: 1,
            lifecycle_events: 1,
            filtered_events: 1,
            tradeable_setups: 1,
            suppressed_duplicates: 0,
            noise_reduction_pct: 0,
        },
        1_700_000_090_000,
    );

    let payload = serde_json::to_value(&response).expect("semantic response json");
    let first_idea = payload["tradeIdeas"]
        .as_array()
        .and_then(|items| items.first())
        .expect("decision-support idea");

    assert_eq!(first_idea["semanticType"], "decision_support");
    assert_eq!(first_idea["riskState"], "low");
    assert_eq!(first_idea["directionBias"], "BULLISH_BIAS");
    assert!(first_idea.get("pressureZone").is_some());
    assert!(first_idea.get("riskBoundary").is_some());
    assert!(first_idea.get("entryZone").is_none());
    assert!(first_idea.get("invalidation").is_none());
    assert_ne!(first_idea["directionBias"], "LONG");
    assert_ne!(first_idea["directionBias"], "SHORT");
    assert_eq!(payload["riskContext"]["semanticType"], "risk_override");
    assert_eq!(payload["riskContext"]["riskState"], "low");
    assert_eq!(payload["rankedEvents"][0]["semanticType"], "analysis");
}

#[test]
fn intelligence_response_high_risk_overrides_decision_support_outputs() {
    let _guard = contract_whale_test_guard();
    let mut clean_trend = persisted_signal(1_700_000_010_000, ContractWhaleSeverity::High);
    clean_trend.id = "contract-whale:BTC:15:clean-trend".to_string();
    clean_trend.signal_type = ContractWhaleSignalType::AggressiveBuy;
    clean_trend.direction = ContractWhaleDirection::Buy;
    clean_trend.total_volume_btc = 3_600.0;
    clean_trend.total_notional_usd = 222_000_000.0;
    clean_trend.net_volume_btc = 2_700.0;
    clean_trend.price_move_pct = Some(0.32);
    clean_trend.price_response_type = ContractWhalePriceResponseType::TrendFollowUp;
    clean_trend.current_market_price_usd = Some(60_220.0);
    clean_trend.order_price_usd = Some(60_190.0);
    clean_trend.main_force_score = Some(90);
    clean_trend.dominance = 0.75;
    clean_trend.multi_exchange_confirmed = true;
    clean_trend.event_lifecycle.update_count = 4;
    clean_trend.event_quality.quality_score = 0.90;
    clean_trend.merged_from = vec!["contract-whale:BTC:5:clean-trend".to_string()];

    let mut fake_breakout = persisted_signal(1_700_000_015_000, ContractWhaleSeverity::Critical);
    fake_breakout.id = "contract-whale:BTC:15:high-risk-fakeout".to_string();
    fake_breakout.signal_type = ContractWhaleSignalType::UpsideSuppression;
    fake_breakout.direction = ContractWhaleDirection::Suppression;
    fake_breakout.total_volume_btc = 7_200.0;
    fake_breakout.total_notional_usd = 446_000_000.0;
    fake_breakout.net_volume_btc = -4_900.0;
    fake_breakout.price_move_pct = Some(-0.22);
    fake_breakout.price_response_type = ContractWhalePriceResponseType::UpsideResistance;
    fake_breakout.current_market_price_usd = Some(60_420.0);
    fake_breakout.order_price_usd = Some(60_390.0);
    fake_breakout.main_force_score = Some(94);
    fake_breakout.dominance = 0.82;
    fake_breakout.multi_exchange_confirmed = true;
    fake_breakout.event_lifecycle.update_count = 5;
    fake_breakout.event_quality.quality_score = 0.93;
    fake_breakout.merged_from = vec!["contract-whale:BTC:5:high-risk-fakeout".to_string()];

    let response = build_contract_whale_intelligence_response(
        "BTC",
        &[clean_trend, fake_breakout],
        &ContractWhaleMarketStructureLite {
            status: "confirmed".to_string(),
            regime_type: "main_force_short_build".to_string(),
            main_force_score: 88,
            confidence: 82,
            ..Default::default()
        },
        ContractWhaleNoiseSuppressionSummary {
            raw_candidates: 1,
            merged_events: 1,
            lifecycle_events: 1,
            filtered_events: 1,
            tradeable_setups: 1,
            suppressed_duplicates: 0,
            noise_reduction_pct: 0,
        },
        1_700_000_090_000,
    );

    assert_eq!(response.risk_context.fake_breakout_risk, "HIGH");
    assert!(response.trade_ideas.is_empty());
    assert_eq!(response.signal_compression.top_signal_count, 0);
    assert!(response.risk_context.summary.contains("风险抑制"));
}

#[test]
fn intelligence_response_fine_tuning_filters_ranging_noise_out_of_ranked_and_trade_ideas() {
    let _guard = contract_whale_test_guard();
    let mut confirmed_trend = persisted_signal(1_700_000_015_000, ContractWhaleSeverity::High);
    confirmed_trend.id = "contract-whale:BTC:15:confirmed-trend".to_string();
    confirmed_trend.signal_type = ContractWhaleSignalType::AggressiveBuy;
    confirmed_trend.direction = ContractWhaleDirection::Buy;
    confirmed_trend.total_volume_btc = 2_640.0;
    confirmed_trend.total_notional_usd = 168_000_000.0;
    confirmed_trend.net_volume_btc = 1_920.0;
    confirmed_trend.price_move_pct = Some(0.28);
    confirmed_trend.current_market_price_usd = Some(60_420.0);
    confirmed_trend.order_price_usd = Some(60_390.0);
    confirmed_trend.main_force_score = Some(90);
    confirmed_trend.dominance = 0.73;
    confirmed_trend.multi_exchange_confirmed = true;
    confirmed_trend.event_lifecycle.update_count = 3;
    confirmed_trend.event_quality.quality_score = 0.88;
    confirmed_trend.merged_from = vec!["contract-whale:BTC:5:confirmed-trend".to_string()];

    let mut clean_absorption = persisted_signal(1_700_000_025_000, ContractWhaleSeverity::High);
    clean_absorption.id = "contract-whale:BTC:5:clean-absorption".to_string();
    clean_absorption.signal_type = ContractWhaleSignalType::DownsideAbsorption;
    clean_absorption.direction = ContractWhaleDirection::Absorption;
    clean_absorption.total_volume_btc = 1_960.0;
    clean_absorption.total_notional_usd = 121_000_000.0;
    clean_absorption.net_volume_btc = 1_180.0;
    clean_absorption.price_move_pct = Some(0.15);
    clean_absorption.price_response_type = ContractWhalePriceResponseType::DownsideAbsorption;
    clean_absorption.current_market_price_usd = Some(60_405.0);
    clean_absorption.order_price_usd = Some(60_380.0);
    clean_absorption.main_force_score = Some(82);
    clean_absorption.dominance = 0.61;
    clean_absorption.multi_exchange_confirmed = true;
    clean_absorption.event_lifecycle.update_count = 2;
    clean_absorption.event_quality.quality_score = 0.83;
    clean_absorption.merged_from = vec!["contract-whale:BTC:15:clean-absorption".to_string()];

    let mut ranging_noise = persisted_signal(1_700_000_035_000, ContractWhaleSeverity::High);
    ranging_noise.id = "contract-whale:BTC:15:ranging-noise".to_string();
    ranging_noise.signal_type = ContractWhaleSignalType::DownsideAbsorption;
    ranging_noise.direction = ContractWhaleDirection::Absorption;
    ranging_noise.total_volume_btc = 2_100.0;
    ranging_noise.total_notional_usd = 133_000_000.0;
    ranging_noise.net_volume_btc = 1_240.0;
    ranging_noise.price_move_pct = Some(0.03);
    ranging_noise.price_response_type = ContractWhalePriceResponseType::DownsideAbsorption;
    ranging_noise.current_market_price_usd = Some(60_410.0);
    ranging_noise.order_price_usd = Some(60_404.0);
    ranging_noise.main_force_score = Some(87);
    ranging_noise.dominance = 0.59;
    ranging_noise.multi_exchange_confirmed = false;
    ranging_noise.event_lifecycle.update_count = 1;
    ranging_noise.event_quality.quality_score = 0.84;
    ranging_noise.merged_from = vec!["contract-whale:BTC:5:ranging-noise".to_string()];

    let response = build_contract_whale_intelligence_response(
        "BTC",
        &[confirmed_trend, clean_absorption, ranging_noise],
        &ContractWhaleMarketStructureLite {
            status: "confirmed".to_string(),
            regime_type: "range_rotation".to_string(),
            main_force_score: 58,
            confidence: 70,
            ..Default::default()
        },
        ContractWhaleNoiseSuppressionSummary {
            raw_candidates: 3,
            merged_events: 3,
            lifecycle_events: 3,
            filtered_events: 0,
            tradeable_setups: 3,
            suppressed_duplicates: 0,
            noise_reduction_pct: 0,
        },
        1_700_000_090_000,
    );

    assert_eq!(response.trade_ideas.len(), 2);
    assert_eq!(response.ranked_events.len(), 2);
    assert!(response
        .trade_ideas
        .iter()
        .all(|idea| idea.signal_id != "contract-whale:BTC:15:ranging-noise"));
    assert!(response
        .ranked_events
        .iter()
        .all(|item| item.signal_id != "contract-whale:BTC:15:ranging-noise"));
}

#[test]
fn intelligence_response_fine_tuning_downgrades_high_confidence_without_multi_window_or_persistence(
) {
    let _guard = contract_whale_test_guard();
    let mut fragile_breakout = persisted_signal(1_700_000_015_000, ContractWhaleSeverity::High);
    fragile_breakout.id = "contract-whale:BTC:15:fragile-breakout".to_string();
    fragile_breakout.signal_type = ContractWhaleSignalType::AggressiveBuy;
    fragile_breakout.direction = ContractWhaleDirection::Buy;
    fragile_breakout.total_volume_btc = 3_420.0;
    fragile_breakout.total_notional_usd = 215_000_000.0;
    fragile_breakout.net_volume_btc = 2_890.0;
    fragile_breakout.price_move_pct = Some(0.24);
    fragile_breakout.price_response_type = ContractWhalePriceResponseType::TrendFollowUp;
    fragile_breakout.current_market_price_usd = Some(60_520.0);
    fragile_breakout.order_price_usd = Some(60_470.0);
    fragile_breakout.main_force_score = Some(98);
    fragile_breakout.dominance = 0.84;
    fragile_breakout.multi_exchange_confirmed = true;
    fragile_breakout.event_lifecycle.update_count = 2;
    fragile_breakout.event_quality.quality_score = 0.91;
    fragile_breakout.merged_from.clear();

    let response = build_contract_whale_intelligence_response(
        "BTC",
        &[fragile_breakout],
        &ContractWhaleMarketStructureLite {
            status: "confirmed".to_string(),
            regime_type: "main_force_long_build".to_string(),
            main_force_score: 82,
            confidence: 75,
            ..Default::default()
        },
        ContractWhaleNoiseSuppressionSummary {
            raw_candidates: 1,
            merged_events: 1,
            lifecycle_events: 1,
            filtered_events: 0,
            tradeable_setups: 1,
            suppressed_duplicates: 0,
            noise_reduction_pct: 0,
        },
        1_700_000_090_000,
    );

    assert_eq!(response.trade_ideas.len(), 1);
    assert_ne!(response.trade_ideas[0].confidence_label, "HIGH");
    assert!(response.trade_ideas[0].confidence < 85);
}

#[test]
fn contract_whale_history_response_merges_same_event_time_window_slices() {
    let _guard = contract_whale_test_guard();
    let mut fifteen_sec = persisted_signal(1_700_000_030_000, ContractWhaleSeverity::Medium);
    fifteen_sec.id = "contract-whale:BTC:15:1700000030000:buy".to_string();
    fifteen_sec.window_sec = 15;
    fifteen_sec.total_volume_btc = 665.0;
    fifteen_sec.net_volume_btc = 620.0;
    fifteen_sec.total_volume = 665.0;
    fifteen_sec.net_volume = 620.0;
    fifteen_sec.total_notional_usd = 43_000_000.0;
    fifteen_sec.score = 52;
    let mut five_sec = persisted_signal(1_700_000_015_000, ContractWhaleSeverity::Medium);
    five_sec.id = "contract-whale:BTC:5:1700000015000:buy".to_string();
    five_sec.window_sec = 5;
    five_sec.total_volume_btc = 473.0;
    five_sec.net_volume_btc = 430.0;
    five_sec.total_volume = 473.0;
    five_sec.net_volume = 430.0;
    five_sec.total_notional_usd = 30_000_000.0;
    five_sec.score = 47;

    let response = build_contract_whale_history_response(
        vec![five_sec, fifteen_sec],
        "BTC",
        50,
        None,
        true,
        true,
        None,
    );

    assert_eq!(response.items.len(), 1);
    assert_eq!(response.items[0].window_sec, 15);
    assert_eq!(response.items[0].total_volume_btc, 665.0);
    assert_eq!(response.items[0].net_volume_btc, 620.0);
    assert_eq!(response.items[0].total_notional_usd, 43_000_000.0);
    assert_eq!(response.items[0].score, 52);
    assert_eq!(response.items[0].merged_from.len(), 1);
    assert!(response.items[0]
        .merged_from
        .iter()
        .any(|id| id.contains("BTC:5:")));
}

#[test]
fn contract_whale_history_response_marks_active_and_closed_event_lifecycle() {
    let _guard = contract_whale_test_guard();
    let mut closed = persisted_signal(1_700_000_000_000, ContractWhaleSeverity::Medium);
    closed.id = "contract-whale:BTC:15:1700000000000:buy".to_string();
    closed.total_volume_btc = 520.0;
    closed.total_volume = 520.0;
    closed.net_volume_btc = 460.0;
    closed.net_volume = 460.0;
    let mut active = persisted_signal(1_700_000_180_000, ContractWhaleSeverity::Medium);
    active.id = "contract-whale:BTC:15:1700000180000:buy".to_string();
    active.total_volume_btc = 520.0;
    active.total_volume = 520.0;
    active.net_volume_btc = 490.0;
    active.net_volume = 490.0;

    let response = build_contract_whale_history_response(
        vec![closed, active],
        "BTC",
        50,
        None,
        true,
        true,
        None,
    );

    let items = serde_json::to_value(&response.items).expect("items json");
    let items = items.as_array().expect("items array");
    let active_item = items
        .iter()
        .find(|item| item["id"] == "contract-whale:BTC:15:1700000180000:buy")
        .expect("active event");
    let closed_item = items
        .iter()
        .find(|item| item["id"] == "contract-whale:BTC:15:1700000000000:buy")
        .expect("closed event");

    assert_eq!(active_item["eventLifecycle"]["status"], "active");
    assert_eq!(closed_item["eventLifecycle"]["status"], "closed");
    assert_eq!(closed_item["eventLifecycle"]["volumeAccumulated"], 520.0);
    assert_eq!(closed_item["eventLifecycle"]["updateCount"], 1);
}

#[test]
fn contract_whale_history_response_updates_same_event_within_thirty_seconds() {
    let _guard = contract_whale_test_guard();
    let mut first = persisted_signal(1_700_000_000_000, ContractWhaleSeverity::High);
    first.id = "contract-whale:BTC:15:1700000000000:buy".to_string();
    first.total_volume_btc = 300.0;
    first.total_volume = 300.0;
    first.net_volume_btc = 240.0;
    first.net_volume = 240.0;
    let mut second = persisted_signal(1_700_000_020_000, ContractWhaleSeverity::High);
    second.id = "contract-whale:BTC:15:1700000020000:buy".to_string();
    second.total_volume_btc = 500.0;
    second.total_volume = 500.0;
    second.net_volume_btc = 410.0;
    second.net_volume = 410.0;

    let response = build_contract_whale_history_response(
        vec![first, second],
        "BTC",
        50,
        None,
        true,
        true,
        None,
    );

    let items = serde_json::to_value(&response.items).expect("items json");
    let items = items.as_array().expect("items array");
    assert_eq!(items.len(), 1);
    assert_eq!(items[0]["eventLifecycle"]["status"], "active");
    assert_eq!(items[0]["eventLifecycle"]["volumeAccumulated"], 500.0);
    assert_eq!(items[0]["eventLifecycle"]["updateCount"], 2);
    assert_eq!(items[0]["totalVolumeBtc"], 500.0);
    assert_eq!(items[0]["netVolumeBtc"], 410.0);
}

#[test]
fn contract_whale_history_response_suppresses_repeated_medium_updates_within_thirty_seconds() {
    let _guard = contract_whale_test_guard();
    let mut first = persisted_signal(1_700_000_000_000, ContractWhaleSeverity::Medium);
    first.id = "contract-whale:BTC:15:1700000000000:buy".to_string();
    first.total_volume_btc = 520.0;
    first.total_volume = 520.0;
    first.net_volume_btc = 420.0;
    first.net_volume = 420.0;

    let mut second = persisted_signal(1_700_000_010_000, ContractWhaleSeverity::Medium);
    second.id = "contract-whale:BTC:15:1700000010000:buy".to_string();
    second.total_volume_btc = 540.0;
    second.total_volume = 540.0;
    second.net_volume_btc = 440.0;
    second.net_volume = 440.0;

    let response = build_contract_whale_history_response(
        vec![first, second],
        "BTC",
        50,
        None,
        true,
        true,
        None,
    );

    let items = serde_json::to_value(&response.items).expect("items json");
    let items = items.as_array().expect("items array");
    assert_eq!(items.len(), 1);
    assert_eq!(items[0]["eventLifecycle"]["status"], "active");
    assert_eq!(items[0]["eventLifecycle"]["updateCount"], 1);
    assert_eq!(items[0]["eventLifecycle"]["volumeAccumulated"], 540.0);
    assert_eq!(items[0]["totalVolumeBtc"], 540.0);
    assert_eq!(items[0]["netVolumeBtc"], 440.0);
}

#[test]
fn contract_whale_history_response_scores_clean_event_quality() {
    let _guard = contract_whale_test_guard();
    let mut first = persisted_signal(1_700_000_000_000, ContractWhaleSeverity::Medium);
    first.id = "contract-whale:BTC:15:1700000000000:buy".to_string();
    first.total_volume_btc = 520.0;
    first.total_volume = 520.0;
    first.net_volume_btc = 420.0;
    first.net_volume = 420.0;
    first.total_notional_usd = 36_400_000.0;
    first.price_move_pct = Some(0.22);
    first.oi_change_1m_btc = Some(80.0);

    let mut second = persisted_signal(1_700_000_020_000, ContractWhaleSeverity::Medium);
    second.id = "contract-whale:BTC:15:1700000020000:buy".to_string();
    second.total_volume_btc = 510.0;
    second.total_volume = 510.0;
    second.net_volume_btc = 430.0;
    second.net_volume = 430.0;
    second.total_notional_usd = 34_000_000.0;
    second.price_move_pct = Some(0.28);
    second.oi_change_1m_btc = Some(110.0);

    let response = build_contract_whale_history_response(
        vec![first, second],
        "BTC",
        50,
        None,
        true,
        true,
        None,
    );

    let items = serde_json::to_value(&response.items).expect("items json");
    let items = items.as_array().expect("items array");
    assert_eq!(items.len(), 1);
    assert_eq!(items[0]["eventQuality"]["valid"], true);
    assert!(
        items[0]["eventQuality"]["qualityScore"]
            .as_f64()
            .expect("quality score")
            > 0.6
    );
    assert!(
        items[0]["eventQuality"]["mergeSimilarityScore"]
            .as_f64()
            .expect("merge similarity")
            > 0.75
    );
    assert_eq!(
        items[0]["eventQuality"]["falseEventFlags"]
            .as_array()
            .expect("false event flags")
            .len(),
        0
    );
}

#[test]
fn final_event_projection_is_single_truth_for_merged_contract_whale_event() {
    let _guard = contract_whale_test_guard();
    let mut fifteen_sec = persisted_signal(1_700_000_015_000, ContractWhaleSeverity::Medium);
    fifteen_sec.id = "contract-whale:BTC:15:1700000015000:sell".to_string();
    fifteen_sec.window_sec = 15;
    fifteen_sec.signal_type =
        btc_toxic_flow_monitor_rs::contract_whale_monitor::types::ContractWhaleSignalType::DownsideAbsorption;
    fifteen_sec.total_volume_btc = 3_100.0;
    fifteen_sec.net_volume_btc = -2_950.0;
    fifteen_sec.total_volume = 3_100.0;
    fifteen_sec.net_volume = -2_950.0;
    fifteen_sec.total_notional_usd = 199_000_000.0;
    fifteen_sec.price_move_pct = Some(0.19);
    fifteen_sec.oi_change_1m_btc = Some(80.0);

    let mut five_sec = fifteen_sec.clone();
    five_sec.id = "contract-whale:BTC:5:1700000015000:sell".to_string();
    five_sec.window_sec = 5;
    five_sec.total_volume_btc = 1_776.0;
    five_sec.net_volume_btc = -1_669.0;
    five_sec.total_volume = 1_776.0;
    five_sec.net_volume = -1_669.0;
    five_sec.total_notional_usd = 114_000_000.0;

    let response = build_contract_whale_history_response(
        vec![five_sec, fifteen_sec],
        "BTC",
        50,
        None,
        true,
        true,
        None,
    );
    let final_events = build_final_events_from_contract_whale_signals(
        &response.items,
        VolumeDisplayContext::FinalLifecycleEvent,
    );

    assert_eq!(response.items.len(), 1);
    assert_eq!(final_events.len(), 1);
    let event = &final_events[0];
    assert_eq!(
        event.event_id, response.items[0].event_lifecycle.event_id,
        "FinalEvent must use the lifecycle event id as the canonical id"
    );
    assert_eq!(event.symbol, "BTC");
    assert_eq!(event.event_type, "downside_absorption");
    assert_eq!(event.status, "active");
    assert_eq!(event.window_sec, 15);
    assert_eq!(event.volume, 3_100.0);
    assert_eq!(event.notional, 199_000_000.0);
    assert_eq!(event.net_volume, -2_950.0);
    assert_eq!(event.direction_bias, "sell");
    assert!(event.quality_score > 0.80);
    assert_eq!(event.source_signal_ids.len(), 2);
    assert!(event
        .source_signal_ids
        .iter()
        .any(|id| id.contains("BTC:15:")));
    assert!(event
        .source_signal_ids
        .iter()
        .any(|id| id.contains("BTC:5:")));
}

#[test]
fn final_event_store_response_keeps_source_signal_as_read_only_projection_evidence() {
    let _guard = contract_whale_test_guard();
    let mut first = persisted_signal(1_700_000_000_000, ContractWhaleSeverity::Medium);
    first.id = "contract-whale:BTC:15:1700000000000:buy".to_string();
    first.total_volume_btc = 520.0;
    first.total_volume = 520.0;
    first.net_volume_btc = 420.0;
    first.net_volume = 420.0;
    first.total_notional_usd = 36_400_000.0;
    first.price_move_pct = Some(0.22);
    first.oi_change_1m_btc = Some(80.0);

    let response =
        build_contract_whale_history_response(vec![first], "BTC", 50, None, true, true, None);
    let final_response = build_final_event_store_response_from_contract_whale_response(&response);

    assert_eq!(final_response.count, 1);
    assert_eq!(final_response.items.len(), 1);
    assert_eq!(
        final_response.items[0].source_signal.id,
        response.items[0].id
    );
    assert_eq!(
        final_response.items[0]
            .source_signal
            .event_lifecycle
            .event_id,
        final_response.items[0].event_id
    );
    assert_eq!(
        final_response.items[0].source_signal.event_quality.valid,
        true
    );
}

#[test]
fn final_event_store_computes_cross_event_impact_normalization() {
    let mut low = persisted_signal(1_700_000_000_000, ContractWhaleSeverity::Medium);
    low.id = "contract-whale:BTC:15:1700000000000:low".to_string();
    low.total_volume_btc = 520.0;
    low.net_volume_btc = 360.0;
    low.total_notional_usd = 33_280_000.0;

    let mut mid = persisted_signal(1_700_000_600_000, ContractWhaleSeverity::Medium);
    mid.id = "contract-whale:BTC:15:1700000600000:mid".to_string();
    mid.ts = 1_700_000_600_000;
    mid.event_lifecycle.event_id = "cwm-event:BTC:aggressive_buy:1700000600000".to_string();
    mid.event_lifecycle.start_time = 1_700_000_600_000;
    mid.event_lifecycle.last_update_time = 1_700_000_600_000;
    mid.total_volume_btc = 620.0;
    mid.net_volume_btc = 430.0;
    mid.total_notional_usd = 39_680_000.0;

    let mut high = persisted_signal(1_700_001_200_000, ContractWhaleSeverity::High);
    high.id = "contract-whale:BTC:15:1700001200000:high".to_string();
    high.ts = 1_700_001_200_000;
    high.event_lifecycle.event_id = "cwm-event:BTC:aggressive_buy:1700001200000".to_string();
    high.event_lifecycle.start_time = 1_700_001_200_000;
    high.event_lifecycle.last_update_time = 1_700_001_200_000;
    high.total_volume_btc = 1_500.0;
    high.net_volume_btc = 1_280.0;
    high.total_notional_usd = 96_000_000.0;

    let response = build_contract_whale_history_response(
        vec![high, mid, low],
        "BTC",
        50,
        None,
        true,
        true,
        None,
    );
    let final_response = build_final_event_store_response_from_contract_whale_response(&response);

    assert_eq!(final_response.count, 3);
    let strongest = final_response
        .items
        .iter()
        .find(|event| (event.raw_volume - 1_500.0).abs() < f64::EPSILON)
        .expect("highest-volume event should be present");
    assert!(strongest.impact_score > 1.0);
    assert!(strongest.z_score > 1.0);
    assert!(strongest.percentile >= 90.0);
    assert_eq!(strongest.normalized_strength, "EXTREME");
    assert_eq!(strongest.direction_bias, "buy");
}

#[test]
fn contract_whale_history_response_filters_micro_spike_false_events() {
    let _guard = contract_whale_test_guard();
    let mut micro_spike = persisted_signal(1_700_000_000_000, ContractWhaleSeverity::Medium);
    micro_spike.id = "contract-whale:BTC:5:1700000000000:buy".to_string();
    micro_spike.window_sec = 5;
    micro_spike.total_volume_btc = 42.0;
    micro_spike.total_volume = 42.0;
    micro_spike.net_volume_btc = 4.0;
    micro_spike.net_volume = 4.0;
    micro_spike.total_notional_usd = 2_700_000.0;
    micro_spike.dominance = 4.0 / 42.0;
    micro_spike.price_move_pct = Some(0.01);
    micro_spike.oi_change_1m_btc = None;
    micro_spike.oi_change_5m_btc = None;
    micro_spike.multi_exchange_confirmed = false;

    let response =
        build_contract_whale_history_response(vec![micro_spike], "BTC", 50, None, true, true, None);

    assert!(response.items.is_empty());
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
fn contract_whale_history_response_clusters_same_intent_trajectory() {
    let _guard = contract_whale_test_guard();
    let mut first = persisted_signal(1_700_000_030_000, ContractWhaleSeverity::High);
    first.score = 82;
    first.order_price_usd = Some(70_000.0);
    first.current_market_price_usd = Some(70_000.0);
    let mut second = first.clone();
    second.id = "contract-whale:BTC:15:1700000075000:buy".to_string();
    second.ts = 1_700_000_075_000;
    second.score = 85;
    second.order_price_usd = Some(70_080.0);
    second.current_market_price_usd = Some(70_000.0);

    let response = build_contract_whale_history_response(
        vec![first.clone(), second.clone()],
        "BTC",
        50,
        None,
        true,
        true,
        None,
    );

    assert_eq!(response.items.len(), 2);
    let latest = &response.items[0];
    let earlier = &response.items[1];
    assert_eq!(latest.id, second.id);
    assert_eq!(latest.cluster.cluster_id, earlier.cluster.cluster_id);
    assert_eq!(latest.cluster.signal_count, 2);
    assert_eq!(latest.cluster.dominant_intent, "liquidity_probe_buy");
    assert!(latest.cluster.duration_ms >= 45_000);
    assert!(latest.cluster.price_range_pct.unwrap_or_default() < 0.3);
    assert!(latest.persistence.persistence_score > 0.0);
    assert!(latest.persistence.redundant_with_previous);
    assert_eq!(
        latest.persistence.redundant_reason,
        "same_intent_within_60s"
    );
    assert_eq!(latest.whale_action.action_type, "aggressive_buy");
    assert_eq!(latest.trajectory.actions.len(), 2);
    assert_eq!(latest.trajectory.intent, "accumulation");
    assert_eq!(
        latest.trajectory.regime_path,
        vec!["accumulation".to_string()]
    );
    assert!(latest.trajectory.stealth_profile.gamma > 0.0);
    assert!(latest.trajectory.conclusion.contains("主力分批吸筹"));
}

#[test]
fn contract_whale_history_response_keeps_wide_price_range_outside_cluster() {
    let _guard = contract_whale_test_guard();
    let mut first = persisted_signal(1_700_000_030_000, ContractWhaleSeverity::High);
    first.score = 82;
    first.order_price_usd = Some(70_000.0);
    first.current_market_price_usd = Some(70_000.0);
    let mut second = first.clone();
    second.id = "contract-whale:BTC:15:1700000075000:buy".to_string();
    second.ts = 1_700_000_075_000;
    second.score = 85;
    second.order_price_usd = Some(70_700.0);
    second.current_market_price_usd = Some(70_000.0);

    let response = build_contract_whale_history_response(
        vec![first, second],
        "BTC",
        50,
        None,
        true,
        true,
        None,
    );

    assert_eq!(response.items.len(), 2);
    assert_ne!(
        response.items[0].cluster.cluster_id,
        response.items[1].cluster.cluster_id
    );
    assert_eq!(response.items[0].cluster.signal_count, 1);
    assert_eq!(response.items[1].cluster.signal_count, 1);
}

#[test]
fn contract_whale_latest_response_filters_price_deviated_items() {
    let _guard = contract_whale_test_guard();
    let mut kept = persisted_signal(1_700_000_030_000, ContractWhaleSeverity::Critical);
    kept.id = "contract-whale:BTC:15:kept".to_string();
    kept.order_price_usd = Some(69_000.0);
    kept.current_market_price_usd = Some(70_000.0);
    let mut filtered = persisted_signal(1_700_000_031_000, ContractWhaleSeverity::S);
    filtered.id = "contract-whale:BTC:15:filtered".to_string();
    filtered.ts = 1_700_000_010_000;
    filtered.order_price_usd = Some(60_000.0);
    filtered.current_market_price_usd = Some(70_000.0);

    let response = build_contract_whale_items_response(
        vec![kept.clone(), filtered],
        "BTC",
        50,
        true,
        true,
        BTreeMap::new(),
        ContractWhaleTrend60s::default(),
    );

    assert_eq!(response.items.len(), 1);
    assert_eq!(response.items[0].id, kept.id);
    assert_eq!(response.items[0].current_market_price_usd, Some(70_000.0));
    assert_eq!(response.items[0].order_price_usd, Some(69_000.0));
    assert!(response.items[0]
        .price_deviation_pct
        .is_some_and(|value| value < 5.0));
    assert!(!response.items[0].price_deviation_filtered);
}

#[test]
fn contract_whale_latest_response_hides_btc_signals_below_500_volume_display_threshold() {
    let _guard = contract_whale_test_guard();
    let mut hidden = persisted_signal(1_700_000_030_000, ContractWhaleSeverity::High);
    hidden.id = "contract-whale:BTC:15:hidden-below-500".to_string();
    hidden.total_volume_btc = 499.0;
    hidden.total_volume = 499.0;
    hidden.net_volume_btc = 430.0;
    hidden.net_volume = 430.0;
    hidden.total_notional_usd = 34_930_000.0;
    hidden.order_price_usd = Some(70_000.0);
    hidden.current_market_price_usd = Some(70_000.0);

    let response = build_contract_whale_items_response(
        vec![hidden],
        "BTC",
        50,
        true,
        true,
        BTreeMap::new(),
        ContractWhaleTrend60s::default(),
    );

    assert!(response.items.is_empty());
    assert_eq!(response.summary.signal_count, 0);
}

#[test]
fn contract_whale_generated_response_marks_price_deviation_context() {
    let _guard = contract_whale_test_guard();
    let flow_state = FlowState {
        symbol: "BTC-PERP".to_string(),
        updated_at: 1_700_000_030_000,
        windows: BTreeMap::from([("15000".to_string(), high_conviction_window())]),
    };

    let response = build_contract_whale_response(&flow_state, "BTC", 50, None, true, true);

    assert_eq!(response.items.len(), 1);
    assert!(response.items[0].order_price_usd.is_some());
    assert!(response.items[0].current_market_price_usd.is_some());
    assert!(!response.items[0].price_deviation_filtered);
    assert!(response.items[0]
        .price_deviation_pct
        .is_some_and(|value| value <= 5.0));
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
        net_direction: Some("abs500".to_string()),
        min_notional_usd: None,
        from: Some("1700000000000".to_string()),
        to: Some("1700086400000".to_string()),
        limit: Some("999".to_string()),
        offset: Some("25".to_string()),
        status: None,
        range: None,
        cursor: None,
        include_hidden: None,
        hide_stale: None,
        impact_level: None,
        include_source_signal: None,
    };

    let parsed = parse_history_query(&query).expect("valid query");

    assert_eq!(parsed.symbol.as_deref(), Some("BTC"));
    assert_eq!(parsed.severity, Some(ContractWhaleSeverity::Critical));
    assert_eq!(parsed.discord_sent, Some(true));
    assert_eq!(parsed.window_sec, Some(15));
    assert_eq!(parsed.exchange.as_deref(), Some("binance"));
    assert_eq!(parsed.min_abs_net_volume_btc, Some(500.0));
    assert_eq!(parsed.limit, 500);
    assert_eq!(parsed.offset, 25);
    assert_eq!(parsed.cursor_ts, None);
    assert_eq!(parsed.cursor_signal_id, None);
}

#[test]
fn contract_whale_history_query_accepts_stable_cursor() {
    let _guard = contract_whale_test_guard();
    let cursor = encode_contract_history_cursor(1_700_000_000_123, "sig_abc");
    let query = ContractWhaleQuery {
        cursor: Some(cursor),
        limit: Some("100".to_string()),
        ..empty_query()
    };

    let parsed = parse_history_query(&query).expect("valid cursor query");

    assert_eq!(parsed.limit, 100);
    assert_eq!(parsed.offset, 0);
    assert_eq!(parsed.cursor_ts, Some(1_700_000_000_123));
    assert_eq!(parsed.cursor_signal_id.as_deref(), Some("sig_abc"));
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

    let invalid_net_direction = ContractWhaleQuery {
        net_direction: Some("abs250".to_string()),
        ..empty_query()
    };
    assert_eq!(
        parse_history_query(&invalid_net_direction)
            .expect_err("invalid net direction")
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
            symbol: "BTC".to_string(),
            base_asset: "BTC".to_string(),
            quantity_unit: "BTC".to_string(),
            buy_volume: 10.0,
            sell_volume: 20.0,
            total_volume: 30.0,
            net_volume: -10.0,
            buy_volume_btc: 10.0,
            sell_volume_btc: 20.0,
            total_volume_btc: 30.0,
            net_volume_btc: -10.0,
            dominance: 10.0 / 30.0,
            buy_ratio: 10.0 / 30.0,
            sell_ratio: 20.0 / 30.0,
            updated_at_ms: Some(1_700_000_030_000),
            ..Default::default()
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
    assert!(metrics
        .contains("cwm_trend_60s_buy_volume{symbol=\"btc\",quantity_unit=\"btc\"} 10.000000"));
    assert!(!metrics.to_ascii_lowercase().contains("webhook"));
    assert!(!metrics.to_ascii_lowercase().contains("token"));
}

#[test]
fn contract_whale_metrics_text_labels_eth_trend_units() {
    let _guard = contract_whale_test_guard();
    let metrics = build_contract_whale_metrics_text(
        true,
        &default_test_exchanges(),
        &ContractWhaleTrend60s {
            symbol: "ETH".to_string(),
            base_asset: "ETH".to_string(),
            quantity_unit: "ETH".to_string(),
            buy_volume: 688.0,
            sell_volume: 73.0,
            total_volume: 761.0,
            net_volume: 615.0,
            buy_volume_btc: 688.0,
            sell_volume_btc: 73.0,
            total_volume_btc: 761.0,
            net_volume_btc: 615.0,
            dominance: 615.0 / 761.0,
            buy_ratio: 688.0 / 761.0,
            sell_ratio: 73.0 / 761.0,
            updated_at_ms: Some(1_700_000_030_000),
        },
        &[],
    );

    assert!(metrics
        .contains("cwm_trend_60s_buy_volume{symbol=\"eth\",quantity_unit=\"eth\"} 688.000000"));
    assert!(metrics
        .contains("cwm_trend_60s_net_volume{symbol=\"eth\",quantity_unit=\"eth\"} 615.000000"));
    assert!(!metrics.contains("cwm_trend_60s_buy_volume{symbol=\"btc\",quantity_unit=\"btc\"}"));
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
    breakdown_at_price(buy, sell, 70_000.0)
}

fn breakdown_at_price(buy: f64, sell: f64, price: f64) -> VenueFlowBreakdown {
    VenueFlowBreakdown {
        aggressive_buy_btc: buy,
        aggressive_sell_btc: sell,
        aggressive_buy_usd: buy * price,
        aggressive_sell_usd: sell * price,
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
        net_direction: None,
        discord_sent: None,
        window_sec: None,
        exchange: None,
        min_notional_usd: None,
        from: None,
        to: None,
        offset: None,
        status: None,
        range: None,
        cursor: None,
        include_hidden: None,
        hide_stale: None,
        impact_level: None,
        include_source_signal: None,
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
