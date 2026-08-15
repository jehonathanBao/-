use btc_toxic_flow_monitor_rs::contract_whale_monitor::{
    aggregator::{
        aggregate_1s_buckets, market_context_from_snapshots, rolling_window_stats,
        rolling_window_stats_with_config, RollingWindowStatsOptions,
    },
    classification::{
        classify_contract_whale_signal_v2, resolve_contract_whale_oi_context_from_window,
    },
    collector_binance::handle_force_order_message,
    collector_okx::handle_liquidation_order_message,
    config::ContractWhaleRuntimeConfig,
    detector::{detect_contract_whale_signal, detect_contract_whale_signal_with_config},
    discord::{
        build_contract_whale_discord_preview, is_historic_s_impact, sanitize_contract_whale_impact,
        should_push_contract_whale_discord,
    },
    event_lifecycle::{apply_contract_whale_event_lifecycle, ContractWhaleLifecycleClock},
    normalizer::{
        normalize_binance_agg_trade, normalize_binance_force_order,
        normalize_binance_force_order_json, normalize_binance_funding_rate_json,
        normalize_binance_open_interest_json, normalize_bitfinex_trade,
        normalize_okx_funding_rate_json, normalize_okx_liquidation_order_json,
        normalize_okx_open_interest_json, normalize_okx_swap_trade,
    },
    outcome_calibration::evaluate_contract_whale_signal_outcome,
    scoring::score_contract_whale_signal_with_config,
    types::{
        ContractExchange, ContractFundingSnapshot, ContractLiquidationSide, ContractOiSnapshot,
        ContractTradeSide, ContractWhaleActiveFlowDirection, ContractWhaleDirection,
        ContractWhaleLiquidationContext, ContractWhaleMarketContext, ContractWhaleOiContextTag,
        ContractWhaleOiWindowContext, ContractWhalePriceResponseType, ContractWhaleSeverity,
        ContractWhaleSignal, ContractWhaleSignalType, ContractWhaleStructureInterpretation,
    },
};

fn three_exchange_config() -> ContractWhaleRuntimeConfig {
    let mut config = ContractWhaleRuntimeConfig::default();
    config.exchanges.okx.enabled = true;
    config.exchanges.okx.perp.enabled = true;
    config.exchanges.okx.funding.enabled = true;
    config.exchanges.okx.oi.enabled = true;
    config.exchanges.okx.liquidation.enabled = true;
    config
}

fn signal_for_impact_sanitization() -> ContractWhaleSignal {
    let now = 1_700_000_015_000;
    let trades = vec![
        normalize_binance_agg_trade(now - 1_000, 70_000.0, 900.0, false).unwrap(),
        normalize_okx_swap_trade(now - 1_000, 70_000.0, 80_000.0, 0.01, "buy").unwrap(),
    ];
    let buckets = aggregate_1s_buckets(&trades);
    let mut stats = rolling_window_stats(&buckets, "BTC", 15, now, Some(0.31), Some(4.2), 85)
        .expect("window stats");
    stats.percentile_level = Some(99.0);
    detect_contract_whale_signal(&stats).expect("impact signal")
}

#[test]
fn ordinary_raw_a_without_major_confirmation_is_downgraded_to_c() {
    let mut signal = signal_for_impact_sanitization();
    signal.impact_level = Some("A".to_string());
    signal.signal_level = Some("L3".to_string());
    signal.signal_label = Some("HIGH IMPACT EVENT".to_string());
    signal.data_quality = 85;
    signal.percentile_level = Some(99.0);
    signal.impact_score = Some(10.0);
    signal.impact_z_score = Some(10.0);
    signal.total_volume_btc = 638.0;
    signal.total_notional_usd = 40_000_000.0;
    signal.price_move_pct = Some(-0.104);
    signal.multi_exchange_confirmed = false;
    signal.liquidation_suspected = false;
    signal.liquidation_long_btc = 0.0;
    signal.liquidation_short_btc = 0.0;

    sanitize_contract_whale_impact(&mut signal);

    assert_eq!(signal.impact_level.as_deref(), Some("C"));
    assert_eq!(signal.signal_level.as_deref(), Some("L1"));
    assert_eq!(signal.signal_label.as_deref(), Some("LOW IMPACT EVENT"));
}

#[test]
fn material_raw_a_with_lower_boundary_evidence_is_downgraded_to_b() {
    let mut signal = signal_for_impact_sanitization();
    signal.impact_level = Some("A".to_string());
    signal.signal_level = Some("L3".to_string());
    signal.signal_label = Some("HIGH IMPACT EVENT".to_string());
    signal.data_quality = 70;
    signal.percentile_level = Some(99.0);
    signal.impact_score = Some(4.0);
    signal.impact_z_score = Some(4.0);
    signal.total_volume_btc = 800.0;
    signal.total_notional_usd = 50_000_000.0;
    signal.price_move_pct = Some(0.15);
    signal.multi_exchange_confirmed = false;
    signal.liquidation_suspected = false;
    signal.liquidation_long_btc = 0.0;
    signal.liquidation_short_btc = 0.0;

    sanitize_contract_whale_impact(&mut signal);

    assert_eq!(signal.impact_level.as_deref(), Some("B"));
    assert_eq!(signal.signal_level.as_deref(), Some("L2"));
    assert_eq!(signal.signal_label.as_deref(), Some("MEDIUM IMPACT EVENT"));
}

#[test]
fn confirmed_major_raw_a_remains_a() {
    let mut signal = signal_for_impact_sanitization();
    signal.impact_level = Some("A".to_string());
    signal.signal_level = Some("L3".to_string());
    signal.signal_label = Some("HIGH IMPACT EVENT".to_string());
    signal.data_quality = 80;
    signal.percentile_level = Some(99.5);
    signal.impact_score = Some(4.0);
    signal.impact_z_score = Some(4.0);
    signal.total_volume_btc = 2_500.0;
    signal.total_notional_usd = 150_000_000.0;
    signal.price_move_pct = Some(0.5);
    signal.multi_exchange_confirmed = true;
    signal.liquidation_suspected = false;
    signal.liquidation_long_btc = 0.0;
    signal.liquidation_short_btc = 0.0;

    sanitize_contract_whale_impact(&mut signal);

    assert_eq!(signal.impact_level.as_deref(), Some("A"));
    assert_eq!(signal.signal_level.as_deref(), Some("L3"));
    assert_eq!(signal.signal_label.as_deref(), Some("HIGH IMPACT EVENT"));
}

#[test]
fn hard_evidence_raw_s_remains_s() {
    let mut signal = signal_for_impact_sanitization();
    signal.impact_level = Some("S".to_string());
    signal.signal_level = Some("S".to_string());
    signal.signal_label = Some("SHOCK IMPACT EVENT".to_string());
    signal.total_volume_btc = 20_000.0;
    signal.window_sec = 60;
    signal.multi_exchange_confirmed = true;
    signal.dynamic_multiple = Some(10.0);
    signal.percentile_level = Some(99.5);
    signal.dominance = 0.65;
    signal.liquidation_suspected = true;
    signal.liquidation_long_btc = 2_500.0;

    sanitize_contract_whale_impact(&mut signal);

    assert_eq!(signal.impact_level.as_deref(), Some("S"));
    assert_eq!(signal.signal_level.as_deref(), Some("S"));
    assert_eq!(signal.signal_label.as_deref(), Some("SHOCK IMPACT EVENT"));
}

#[test]
fn reliability_runtime_defaults_are_explicit_and_conservative() {
    let config = ContractWhaleRuntimeConfig::default();
    assert_eq!(config.producer.interval_ms, 2_000);
    assert!(config.producer.skip_missed_ticks);
    assert!(config.producer.prevent_overlap);
    assert!(config.discord_outbox.enabled);
    assert_eq!(config.discord_outbox.poll_interval_ms, 1_000);
    assert_eq!(config.discord_outbox.batch_size, 20);
    assert_eq!(config.discord_outbox.max_attempts, 6);
    assert_eq!(config.discord_outbox.base_retry_seconds, 2);
    assert_eq!(config.discord_outbox.max_retry_seconds, 300);
    assert!(!config.hourly_delta_alert.enabled);
    assert!(config.hourly_delta_alert.dry_run);
    assert_eq!(config.hourly_delta_alert.threshold_btc, 1000.0);
    assert_eq!(config.hourly_delta_alert.symbol, "BTCUSDT");
    assert_eq!(config.hourly_delta_alert.interval, "1h");
    assert_eq!(config.hourly_delta_alert.startup_backfill_hours, 2);
}

#[test]
fn normalizers_unify_contract_trade_units_to_btc_and_usd() {
    let binance = normalize_binance_agg_trade(1_700_000_000_000, 70_000.0, 2.5, false)
        .expect("binance trade");
    let okx = normalize_okx_swap_trade(1_700_000_000_000, 70_000.0, 25.0, 0.01, "sell")
        .expect("okx trade");
    let bitfinex =
        normalize_bitfinex_trade(1_700_000_000_000, 70_000.0, -0.75).expect("bitfinex trade");

    assert_eq!(binance.qty_btc, 2.5);
    assert_eq!(binance.notional_usd, 175_000.0);
    assert_eq!(okx.qty_btc, 0.25);
    assert_eq!(okx.notional_usd, 17_500.0);
    assert_eq!(bitfinex.qty_btc, 0.75);
    assert_eq!(bitfinex.notional_usd, 52_500.0);
}

#[test]
fn normalizers_map_exchange_direction_and_notional_without_unit_drift() {
    let maker_buy = normalize_binance_agg_trade(1_700_000_000_000, 70_000.0, 1.2, true).unwrap();
    let taker_buy = normalize_binance_agg_trade(1_700_000_000_001, 70_000.0, 1.2, false).unwrap();
    let okx = normalize_okx_swap_trade(1_700_000_000_002, 70_000.0, 250.0, 0.01, "sell").unwrap();
    let bitfinex_buy = normalize_bitfinex_trade(1_700_000_000_003, 70_000.0, 0.75).unwrap();
    let bitfinex_sell = normalize_bitfinex_trade(1_700_000_000_004, 70_000.0, -0.5).unwrap();

    assert_eq!(maker_buy.exchange, ContractExchange::Binance);
    assert_eq!(maker_buy.side, ContractTradeSide::Sell);
    assert_eq!(taker_buy.side, ContractTradeSide::Buy);
    assert_eq!(okx.exchange, ContractExchange::Okx);
    assert_eq!(okx.side, ContractTradeSide::Sell);
    assert_eq!(okx.qty_btc, 2.5);
    assert_eq!(okx.notional_usd, 175_000.0);
    assert_eq!(bitfinex_buy.side, ContractTradeSide::Buy);
    assert_eq!(bitfinex_sell.side, ContractTradeSide::Sell);
    assert_eq!(bitfinex_sell.qty_btc, 0.5);
    assert_eq!(bitfinex_sell.notional_usd, 35_000.0);
}

#[test]
fn normalizers_filter_invalid_price_and_quantity() {
    assert!(normalize_binance_agg_trade(1_700_000_000_000, 0.0, 1.0, false).is_none());
    assert!(normalize_binance_agg_trade(1_700_000_000_000, 70_000.0, -1.0, false).is_none());
    assert!(normalize_okx_swap_trade(1_700_000_000_000, 70_000.0, 1.0, 0.0, "buy").is_none());
    assert!(normalize_okx_swap_trade(1_700_000_000_000, f64::NAN, 1.0, 0.01, "buy").is_none());
    assert!(normalize_bitfinex_trade(1_700_000_000_000, 70_000.0, 0.0).is_none());
    assert!(normalize_okx_swap_trade(1_700_000_000_000, 70_000.0, 1.0, 0.01, "hold").is_none());
}

#[test]
fn binance_force_order_normalizer_maps_long_and_short_liquidations() {
    let long_liq = normalize_binance_force_order(1_700_000_000_000, 70_000.0, 12.0, "SELL")
        .expect("long liquidation");
    let payload = serde_json::json!({
        "e": "forceOrder",
        "E": 1_700_000_001_000_i64,
        "o": {
            "s": "BTCUSDT",
            "S": "BUY",
            "ap": "70010",
            "z": "7.5",
            "T": 1_700_000_001_000_i64
        }
    });
    let short_liq = normalize_binance_force_order_json(&payload).expect("short liquidation");

    assert_eq!(long_liq.qty_btc, 12.0);
    assert_eq!(long_liq.notional_usd, 840_000.0);
    assert_eq!(short_liq.qty_btc, 7.5);
    assert_eq!(short_liq.notional_usd, 525_075.0);
}

#[test]
fn binance_force_order_collector_message_handler_is_read_only_and_normalizes() {
    let text = serde_json::json!({
        "e": "forceOrder",
        "E": 1_700_000_001_000_i64,
        "o": {
            "s": "BTCUSDT",
            "S": "SELL",
            "ap": "70000",
            "z": "9",
            "T": 1_700_000_001_000_i64
        }
    })
    .to_string();

    let order = handle_force_order_message(&text).expect("force order");

    assert_eq!(order.symbol, "BTC");
    assert_eq!(order.qty_btc, 9.0);
    assert_eq!(order.notional_usd, 630_000.0);
}

#[test]
fn okx_liquidation_orders_normalize_contracts_to_btc_and_aggregate() {
    let payload = serde_json::json!({
        "arg": {"channel": "liquidation-orders", "instType": "SWAP", "uly": "BTC-USDT"},
        "data": [{
            "instId": "BTC-USDT-SWAP",
            "details": [
                {"posSide": "long", "bkPx": "70000", "sz": "3000", "ts": "1700000001000"},
                {"posSide": "short", "bkPx": "70100", "sz": "1200", "ts": "1700000001000"}
            ]
        }]
    });
    let orders = normalize_okx_liquidation_order_json(&payload, 0.01);

    assert_eq!(orders.len(), 2);
    assert_eq!(orders[0].qty_btc, 30.0);
    assert_eq!(orders[0].side, ContractLiquidationSide::Long);
    assert_eq!(orders[1].qty_btc, 12.0);
    assert_eq!(orders[1].side, ContractLiquidationSide::Short);

    let handled = handle_liquidation_order_message(&payload.to_string(), 0.01);
    assert_eq!(handled.len(), 2);
}

#[test]
fn oi_and_funding_normalizers_map_binance_and_okx_context() {
    let binance_oi = normalize_binance_open_interest_json(
        &serde_json::json!({
            "symbol": "BTCUSDT",
            "openInterest": "55000",
            "time": 1_700_000_000_000_i64
        }),
        Some(70_000.0),
        1_700_000_000_000,
    )
    .expect("binance oi");
    let okx_oi = normalize_okx_open_interest_json(
        &serde_json::json!({
            "data": [{
                "instId": "BTC-USDT-SWAP",
                "oi": "1600000",
                "oiUsd": "1120000000",
                "ts": "1700000000000"
            }]
        }),
        0.01,
    )
    .expect("okx oi");
    let binance_funding = normalize_binance_funding_rate_json(
        &serde_json::json!({
            "symbol": "BTCUSDT",
            "lastFundingRate": "0.00018",
            "time": 1_700_000_000_000_i64
        }),
        1_700_000_000_000,
    )
    .expect("binance funding");
    let okx_funding = normalize_okx_funding_rate_json(&serde_json::json!({
        "data": [{
            "instId": "BTC-USDT-SWAP",
            "fundingRate": "-0.00005",
            "ts": "1700000000000"
        }]
    }))
    .expect("okx funding");

    assert_eq!(binance_oi.oi_btc, 55_000.0);
    assert_eq!(binance_oi.oi_notional_usd, Some(3_850_000_000.0));
    assert_eq!(okx_oi.oi_btc, 16_000.0);
    assert_eq!(okx_oi.oi_notional_usd, Some(1_120_000_000.0));
    assert_eq!(binance_funding.funding_rate, 0.00018);
    assert_eq!(okx_funding.funding_rate, -0.00005);
}

#[test]
fn aggregator_builds_directional_multi_exchange_window_stats() {
    let now = 1_700_000_015_000;
    let trades = vec![
        normalize_binance_agg_trade(now - 1_000, 70_000.0, 2_600.0, false).unwrap(),
        normalize_okx_swap_trade(now - 1_000, 70_000.0, 178_000.0, 0.01, "buy").unwrap(),
        normalize_bitfinex_trade(now - 1_000, 70_000.0, 430.0).unwrap(),
        normalize_binance_agg_trade(now - 1_000, 70_000.0, 1_200.0, true).unwrap(),
    ];
    let buckets = aggregate_1s_buckets(&trades);
    let config = three_exchange_config();
    let stats = rolling_window_stats_with_config(
        &buckets,
        "BTC",
        15,
        now,
        RollingWindowStatsOptions {
            price_move_pct: Some(0.31),
            dynamic_multiple: Some(9.4),
            dynamic_baseline_btc: None,
            dynamic_threshold_level: String::new(),
            data_quality: 92,
            config: &config,
        },
    )
    .expect("window stats");

    assert_eq!(stats.exchange_count, 3);
    assert_eq!(stats.main_exchange.as_deref(), Some("binance"));
    assert!(stats.total_volume_btc > 5_000.0);
    assert!(stats.net_volume_btc > 3_000.0);
    assert!(stats.dominance > 0.60);
    assert!(stats.exchanges.iter().all(|item| item.dominance > 0.0));
}

#[test]
fn aggregator_separates_direction_strength_from_net_flow_contribution_share() {
    let now = 1_700_000_015_000;
    let trades = vec![
        normalize_okx_swap_trade(now - 1_000, 70_000.0, 466.0, 1.0, "buy").unwrap(),
        normalize_okx_swap_trade(now - 1_000, 70_000.0, 1_625.0, 1.0, "sell").unwrap(),
        normalize_binance_agg_trade(now - 1_000, 70_000.0, 16.0, true).unwrap(),
    ];
    let buckets = aggregate_1s_buckets(&trades);
    let config = three_exchange_config();
    let stats = rolling_window_stats_with_config(
        &buckets,
        "BTC",
        15,
        now,
        RollingWindowStatsOptions {
            price_move_pct: Some(-0.12),
            dynamic_multiple: Some(7.2),
            dynamic_baseline_btc: None,
            dynamic_threshold_level: String::new(),
            data_quality: 90,
            config: &config,
        },
    )
    .expect("window stats");
    let okx = stats
        .exchanges
        .iter()
        .find(|item| item.exchange == "okx")
        .expect("okx contribution");

    assert!((okx.dominance - (1_159.0 / 2_091.0)).abs() < 0.0001);
    assert!((okx.sell_share - (1_625.0 / 2_091.0)).abs() < 0.0001);
    assert!((okx.net_contribution_share - (1_159.0 / 1_175.0)).abs() < 0.0001);
    assert!(
        (stats
            .dominant_venue_net_contribution_share
            .expect("dominant venue share")
            - (1_159.0 / 1_175.0))
            .abs()
            < 0.0001
    );
}

#[test]
fn aggregator_ignores_okx_volume_when_okx_is_disabled() {
    let now = 1_700_000_015_000;
    let trades = vec![
        normalize_binance_agg_trade(now - 1_000, 70_000.0, 100.0, false).unwrap(),
        normalize_bitfinex_trade(now - 1_000, 70_000.0, 20.0).unwrap(),
        normalize_okx_swap_trade(now - 1_000, 70_000.0, 99_999.0, 1.0, "buy").unwrap(),
    ];
    let buckets = aggregate_1s_buckets(&trades);
    let stats = rolling_window_stats(&buckets, "BTC", 15, now, Some(0.05), Some(3.0), 90)
        .expect("window stats");

    assert_eq!(stats.exchange_count, 2);
    assert_eq!(stats.total_volume_btc, 120.0);
    assert!(stats.exchanges.iter().all(|item| item.exchange != "okx"));
    assert_eq!(stats.main_exchange.as_deref(), Some("binance"));
}

#[test]
fn detector_upgrades_multi_exchange_aggressive_buy_to_s_and_discord_eligible() {
    let now = 1_700_000_015_000;
    let trades = vec![
        normalize_binance_agg_trade(now - 1_000, 70_000.0, 3_000.0, false).unwrap(),
        normalize_okx_swap_trade(now - 1_000, 70_000.0, 200_000.0, 0.01, "buy").unwrap(),
        normalize_bitfinex_trade(now - 1_000, 70_000.0, 500.0).unwrap(),
        normalize_binance_agg_trade(now - 1_000, 70_000.0, 600.0, true).unwrap(),
    ];
    let buckets = aggregate_1s_buckets(&trades);
    let mut stats = rolling_window_stats(&buckets, "BTC", 15, now, Some(0.31), Some(10.2), 94)
        .expect("window stats");
    stats.percentile_level = Some(99.9);
    let signal = detect_contract_whale_signal(&stats).expect("signal");

    assert_eq!(signal.signal_type, ContractWhaleSignalType::AggressiveBuy);
    assert_eq!(signal.severity, ContractWhaleSeverity::S);
    assert!(signal.score >= 90);
    assert!(!should_push_contract_whale_discord(&signal));
    assert!(!signal.discord_eligible);
    assert_eq!(signal.discord_reason, "s_grade_extreme_impact_required");
    assert!(!signal.discord_sent);
}

#[test]
fn detector_does_not_confirm_a_tiny_secondary_exchange_contribution() {
    let now = 1_700_000_015_000;
    let trades = vec![
        normalize_binance_agg_trade(now - 1_000, 70_000.0, 3_000.0, false).unwrap(),
        normalize_bitfinex_trade(now - 1_000, 70_000.0, 1.0).unwrap(),
        normalize_binance_agg_trade(now - 1_000, 70_000.0, 300.0, true).unwrap(),
    ];
    let buckets = aggregate_1s_buckets(&trades);
    let mut stats = rolling_window_stats(&buckets, "BTC", 15, now, Some(0.31), Some(10.2), 94)
        .expect("window stats");
    stats.percentile_level = Some(99.9);
    let signal = detect_contract_whale_signal(&stats).expect("signal");

    assert!(!signal.multi_exchange_confirmed);
    assert_ne!(signal.severity, ContractWhaleSeverity::S);
}

#[test]
fn detector_exposes_score_breakdown_and_price_response_type() {
    let now = 1_700_000_015_000;
    let trades = vec![
        normalize_binance_agg_trade(now - 1_000, 70_000.0, 2_400.0, false).unwrap(),
        normalize_bitfinex_trade(now - 1_000, 70_000.0, 320.0).unwrap(),
        normalize_binance_agg_trade(now - 1_000, 70_000.0, 200.0, true).unwrap(),
    ];
    let buckets = aggregate_1s_buckets(&trades);
    let mut stats = rolling_window_stats(&buckets, "BTC", 15, now, Some(0.18), Some(7.4), 90)
        .expect("window stats");
    stats.percentile_level = Some(99.5);
    let signal = detect_contract_whale_signal(&stats).expect("signal");

    assert_eq!(signal.signal_type, ContractWhaleSignalType::AggressiveBuy);
    assert_eq!(
        signal.price_response_type,
        ContractWhalePriceResponseType::TrendFollowUp
    );
    assert_eq!(signal.price_move_15s_pct, Some(0.18));
    assert!(signal.score_breakdown.volume_score > 0.0);
    assert!(signal.score_breakdown.notional_score > 0.0);
    assert!(signal.score_breakdown.directional_strength_score > 0.0);
    assert!(signal.score_breakdown.price_response_score > 0.0);
    assert_eq!(
        signal.score,
        signal.score_breakdown.final_score.round().clamp(0.0, 100.0) as u8
    );
}

#[test]
fn classification_v2_keeps_net_sell_without_follow_through_as_active_sell_pressure() {
    let now = 1_700_000_015_000;
    let trades = vec![
        normalize_binance_agg_trade(now - 1_000, 70_000.0, 1_000.0, true).unwrap(),
        normalize_bitfinex_trade(now - 1_000, 70_000.0, -300.0).unwrap(),
        normalize_binance_agg_trade(now - 1_000, 70_000.0, 120.0, false).unwrap(),
    ];
    let buckets = aggregate_1s_buckets(&trades);
    let stats = rolling_window_stats(&buckets, "BTC", 15, now, Some(-0.10), Some(5.5), 86)
        .expect("window stats");
    let signal = detect_contract_whale_signal(&stats).expect("signal");

    assert_eq!(signal.signal_type, ContractWhaleSignalType::AggressiveSell);
    assert_eq!(signal.classification_v2.display_signal_type, "主动卖压");
    assert_eq!(
        signal.classification_v2.flow_direction,
        ContractWhaleActiveFlowDirection::SellDominant
    );
    assert_eq!(
        signal.classification_v2.structure_interpretation,
        ContractWhaleStructureInterpretation::ActiveSellPressure
    );
    assert!(!signal.classification_v2.is_strong_main_force_intent);
    assert_eq!(
        signal.classification_v2.oi_context,
        ContractWhaleOiContextTag::OiUnavailable
    );
}

#[test]
fn classification_v2_only_marks_main_force_dump_when_price_follows_and_flow_confirms() {
    let now = 1_700_000_015_000;
    let trades = vec![
        normalize_binance_agg_trade(now - 1_000, 70_000.0, 1_800.0, true).unwrap(),
        normalize_bitfinex_trade(now - 1_000, 70_000.0, -420.0).unwrap(),
        normalize_binance_agg_trade(now - 1_000, 70_000.0, 120.0, false).unwrap(),
    ];
    let buckets = aggregate_1s_buckets(&trades);
    let mut stats = rolling_window_stats(&buckets, "BTC", 15, now, Some(-0.32), Some(8.2), 92)
        .expect("window stats");
    stats.percentile_level = Some(99.5);
    stats.market_context.oi_available = true;
    stats.market_context.oi_change_pct = Some(0.28);

    let signal = detect_contract_whale_signal(&stats).expect("signal");

    assert_eq!(signal.signal_type, ContractWhaleSignalType::AggressiveSell);
    assert_eq!(signal.classification_v2.display_signal_type, "主力砸盘");
    assert_eq!(
        signal.classification_v2.structure_interpretation,
        ContractWhaleStructureInterpretation::MainForceDumpDown
    );
    assert!(signal.classification_v2.is_strong_main_force_intent);
    assert!(signal.classification_v2.intent_confidence >= 75);
    assert_eq!(
        signal.classification_v2.oi_context,
        ContractWhaleOiContextTag::NewShortBuild
    );
    assert!(signal
        .classification_v2
        .classification_reasons
        .iter()
        .any(|reason| reason.contains("price_follow_through")));
}

#[test]
fn classification_v2_marks_main_force_lift_only_when_buy_flow_follows_price() {
    let now = 1_700_000_015_000;
    let trades = vec![
        normalize_binance_agg_trade(now - 1_000, 70_000.0, 1_800.0, false).unwrap(),
        normalize_bitfinex_trade(now - 1_000, 70_000.0, 420.0).unwrap(),
        normalize_binance_agg_trade(now - 1_000, 70_000.0, 120.0, true).unwrap(),
    ];
    let buckets = aggregate_1s_buckets(&trades);
    let mut stats = rolling_window_stats(&buckets, "BTC", 15, now, Some(0.32), Some(8.2), 92)
        .expect("window stats");
    stats.percentile_level = Some(99.5);
    stats.market_context.oi_available = true;
    stats.market_context.oi_change_pct = Some(0.28);

    let signal = detect_contract_whale_signal(&stats).expect("signal");

    assert_eq!(signal.signal_type, ContractWhaleSignalType::AggressiveBuy);
    assert_eq!(signal.classification_v2.display_signal_type, "主力拉盘");
    assert_eq!(
        signal.classification_v2.structure_interpretation,
        ContractWhaleStructureInterpretation::MainForcePushUp
    );
    assert!(signal.classification_v2.is_strong_main_force_intent);
    assert_eq!(
        signal.classification_v2.oi_context,
        ContractWhaleOiContextTag::NewLongBuild
    );
}

#[test]
fn classification_v2_keeps_buy_flow_without_follow_through_as_active_buy_pressure() {
    let now = 1_700_000_015_000;
    let trades = vec![
        normalize_binance_agg_trade(now - 1_000, 70_000.0, 1_000.0, false).unwrap(),
        normalize_bitfinex_trade(now - 1_000, 70_000.0, 300.0).unwrap(),
        normalize_binance_agg_trade(now - 1_000, 70_000.0, 120.0, true).unwrap(),
    ];
    let buckets = aggregate_1s_buckets(&trades);
    let stats = rolling_window_stats(&buckets, "BTC", 15, now, Some(0.10), Some(5.5), 86)
        .expect("window stats");
    let signal = detect_contract_whale_signal(&stats).expect("signal");

    assert_eq!(signal.signal_type, ContractWhaleSignalType::AggressiveBuy);
    assert_eq!(signal.classification_v2.display_signal_type, "主动买压");
    assert_eq!(
        signal.classification_v2.structure_interpretation,
        ContractWhaleStructureInterpretation::ActiveBuyPressure
    );
    assert_eq!(
        signal.classification_v2.legacy_signal_type,
        "aggressive_buy"
    );
    assert!(signal.classification_v2.semantic_mismatch);
    assert!(signal.final_result.contains("主动买压"));
    assert!(!signal.classification_v2.is_strong_main_force_intent);
}

#[test]
fn classification_v2_marks_suppression_and_absorption_only_with_strong_low_efficiency_flow() {
    let now = 1_700_000_015_000;
    let buy_trades = vec![
        normalize_binance_agg_trade(now - 1_000, 70_000.0, 1_500.0, false).unwrap(),
        normalize_bitfinex_trade(now - 1_000, 70_000.0, 450.0).unwrap(),
        normalize_binance_agg_trade(now - 1_000, 70_000.0, 120.0, true).unwrap(),
    ];
    let sell_trades = vec![
        normalize_binance_agg_trade(now - 1_000, 70_000.0, 1_500.0, true).unwrap(),
        normalize_bitfinex_trade(now - 1_000, 70_000.0, -450.0).unwrap(),
        normalize_binance_agg_trade(now - 1_000, 70_000.0, 120.0, false).unwrap(),
    ];
    let buy_stats = rolling_window_stats(
        &aggregate_1s_buckets(&buy_trades),
        "BTC",
        15,
        now,
        Some(0.03),
        Some(5.4),
        86,
    )
    .expect("buy stats");
    let sell_stats = rolling_window_stats(
        &aggregate_1s_buckets(&sell_trades),
        "BTC",
        15,
        now,
        Some(-0.03),
        Some(5.4),
        86,
    )
    .expect("sell stats");

    let buy_signal = detect_contract_whale_signal(&buy_stats).expect("suppression signal");
    let sell_signal = detect_contract_whale_signal(&sell_stats).expect("absorption signal");

    assert_eq!(buy_signal.classification_v2.display_signal_type, "上方压制");
    assert_eq!(
        buy_signal.classification_v2.structure_interpretation,
        ContractWhaleStructureInterpretation::UpsideSuppression
    );
    assert_eq!(
        sell_signal.classification_v2.display_signal_type,
        "下方吸收"
    );
    assert_eq!(
        sell_signal.classification_v2.structure_interpretation,
        ContractWhaleStructureInterpretation::DownsideAbsorption
    );
}

#[test]
fn classification_v2_marks_low_dominance_as_unclear_contract_flow() {
    let now = 1_700_000_015_000;
    let trades = vec![
        normalize_binance_agg_trade(now - 1_000, 70_000.0, 700.0, false).unwrap(),
        normalize_binance_agg_trade(now - 1_000, 70_000.0, 600.0, true).unwrap(),
    ];
    let buckets = aggregate_1s_buckets(&trades);
    let mut stats = rolling_window_stats(&buckets, "BTC", 15, now, Some(0.20), Some(6.0), 90)
        .expect("window stats");
    stats.dominance = 0.50;

    let classification = classify_contract_whale_signal_v2(
        &stats,
        ContractWhaleSignalType::AggressiveBuy,
        ContractWhalePriceResponseType::TrendFollowUp,
        true,
        &ContractWhaleRuntimeConfig::default(),
    );

    assert_eq!(classification.display_signal_type, "不明确合约流");
    assert_eq!(
        classification.structure_interpretation,
        ContractWhaleStructureInterpretation::UnclearDirectionalFlow
    );
    assert_eq!(
        classification.flow_direction,
        ContractWhaleActiveFlowDirection::Balanced
    );
}

#[test]
fn classification_v2_maps_all_oi_context_tags_without_promoting_oi_to_main_type() {
    let now = 1_700_000_015_000;
    let trades = vec![
        normalize_binance_agg_trade(now - 1_000, 70_000.0, 1_800.0, false).unwrap(),
        normalize_bitfinex_trade(now - 1_000, 70_000.0, 420.0).unwrap(),
        normalize_binance_agg_trade(now - 1_000, 70_000.0, 120.0, true).unwrap(),
    ];
    let buckets = aggregate_1s_buckets(&trades);
    let mut buy_stats = rolling_window_stats(&buckets, "BTC", 15, now, Some(0.32), Some(8.2), 92)
        .expect("buy stats");
    buy_stats.market_context.oi_available = true;

    let mut sell_stats = buy_stats.clone();
    sell_stats.net_volume_btc = -sell_stats.net_volume_btc.abs();
    sell_stats.price_move_pct = Some(-0.32);

    let config = ContractWhaleRuntimeConfig::default();
    let cases = [
        (
            &buy_stats,
            ContractWhaleSignalType::AggressiveBuy,
            Some(0.28),
            ContractWhaleOiContextTag::NewLongBuild,
        ),
        (
            &sell_stats,
            ContractWhaleSignalType::AggressiveSell,
            Some(0.28),
            ContractWhaleOiContextTag::NewShortBuild,
        ),
        (
            &buy_stats,
            ContractWhaleSignalType::AggressiveBuy,
            Some(-0.28),
            ContractWhaleOiContextTag::ShortCovering,
        ),
        (
            &sell_stats,
            ContractWhaleSignalType::AggressiveSell,
            Some(-0.28),
            ContractWhaleOiContextTag::LongUnwind,
        ),
        (
            &buy_stats,
            ContractWhaleSignalType::AggressiveBuy,
            Some(0.01),
            ContractWhaleOiContextTag::OiNotConfirmed,
        ),
        (
            &buy_stats,
            ContractWhaleSignalType::AggressiveBuy,
            None,
            ContractWhaleOiContextTag::OiUnavailable,
        ),
    ];

    for (stats, signal_type, oi_change_pct, expected) in cases {
        let mut stats = stats.clone();
        stats.market_context.oi_change_pct = oi_change_pct;
        if oi_change_pct.is_none() {
            stats.market_context.oi_available = false;
        }
        let classification = classify_contract_whale_signal_v2(
            &stats,
            signal_type,
            ContractWhalePriceResponseType::NoClearResponse,
            true,
            &config,
        );
        assert_eq!(classification.oi_context, expected);
        assert_ne!(classification.display_signal_type, "OI 建仓");
    }
}

#[test]
fn oi_window_context_maps_buy_follow_through_to_new_long_build() {
    let resolved = resolve_contract_whale_oi_context_from_window(
        ContractWhaleStructureInterpretation::ActiveBuyPressure,
        ContractWhaleActiveFlowDirection::BuyDominant,
        ContractWhalePriceResponseType::TrendFollowUp,
        Some(0.31),
        Some(&ContractWhaleOiWindowContext {
            oi_before: Some(100_000.0),
            oi_after: Some(100_420.0),
            oi_delta: Some(420.0),
            oi_delta_pct: Some(0.42),
            before_ts: Some(1_700_000_000_000),
            after_ts: Some(1_700_000_015_000),
            available: true,
            reason: None,
            ..ContractWhaleOiWindowContext::default()
        }),
        &ContractWhaleRuntimeConfig::default(),
    );

    assert_eq!(resolved.oi_context, ContractWhaleOiContextTag::NewLongBuild);
    assert_eq!(resolved.oi_context_label, "新多开仓");
    assert_eq!(resolved.oi_delta_pct, Some(0.42));
    assert_eq!(
        resolved.oi_reason.as_deref(),
        Some("oi_increased_with_buy_pressure")
    );
    assert!(resolved.oi_available);
}

#[test]
fn oi_window_context_does_not_confirm_build_when_source_coverage_changes() {
    let resolved = resolve_contract_whale_oi_context_from_window(
        ContractWhaleStructureInterpretation::ActiveBuyPressure,
        ContractWhaleActiveFlowDirection::BuyDominant,
        ContractWhalePriceResponseType::TrendFollowUp,
        Some(0.31),
        Some(&ContractWhaleOiWindowContext {
            oi_before: Some(100_000.0),
            oi_after: Some(100_420.0),
            oi_delta: Some(420.0),
            oi_delta_pct: Some(0.42),
            before_ts: Some(1_700_000_000_000),
            after_ts: Some(1_700_000_015_000),
            available: true,
            source_coverage_changed: true,
            cross_exchange_consensus: Some(false),
            reason: None,
            ..ContractWhaleOiWindowContext::default()
        }),
        &ContractWhaleRuntimeConfig::default(),
    );

    assert_eq!(
        resolved.oi_context,
        ContractWhaleOiContextTag::OiNotConfirmed
    );
    assert_eq!(
        resolved.oi_reason.as_deref(),
        Some("oi_source_coverage_changed")
    );
}

#[test]
fn oi_consensus_guard_can_be_disabled_for_shadow_comparison() {
    let mut config = ContractWhaleRuntimeConfig::default();
    config.classification.oi_consensus_guard_enabled = false;
    let resolved = resolve_contract_whale_oi_context_from_window(
        ContractWhaleStructureInterpretation::ActiveBuyPressure,
        ContractWhaleActiveFlowDirection::BuyDominant,
        ContractWhalePriceResponseType::TrendFollowUp,
        Some(0.31),
        Some(&ContractWhaleOiWindowContext {
            oi_before: Some(100_000.0),
            oi_after: Some(100_420.0),
            oi_delta: Some(420.0),
            oi_delta_pct: Some(0.42),
            before_ts: Some(1_700_000_000_000),
            after_ts: Some(1_700_000_015_000),
            available: true,
            source_coverage_changed: true,
            cross_exchange_consensus: Some(false),
            reason: None,
            ..ContractWhaleOiWindowContext::default()
        }),
        &config,
    );

    assert_eq!(resolved.oi_context, ContractWhaleOiContextTag::NewLongBuild);
}

#[test]
fn oi_window_context_maps_buy_follow_through_to_short_covering_when_oi_falls() {
    let resolved = resolve_contract_whale_oi_context_from_window(
        ContractWhaleStructureInterpretation::ActiveBuyPressure,
        ContractWhaleActiveFlowDirection::BuyDominant,
        ContractWhalePriceResponseType::TrendFollowUp,
        Some(0.31),
        Some(&ContractWhaleOiWindowContext {
            oi_before: Some(100_000.0),
            oi_after: Some(99_690.0),
            oi_delta: Some(-310.0),
            oi_delta_pct: Some(-0.31),
            before_ts: Some(1_700_000_000_000),
            after_ts: Some(1_700_000_015_000),
            available: true,
            reason: None,
            ..ContractWhaleOiWindowContext::default()
        }),
        &ContractWhaleRuntimeConfig::default(),
    );

    assert_eq!(
        resolved.oi_context,
        ContractWhaleOiContextTag::ShortCovering
    );
    assert_eq!(resolved.oi_context_label, "空头回补");
    assert_eq!(resolved.oi_delta_pct, Some(-0.31));
    assert_eq!(
        resolved.oi_reason.as_deref(),
        Some("oi_decreased_with_buy_pressure")
    );
    assert!(resolved.oi_available);
}

#[test]
fn oi_window_context_maps_sell_follow_through_to_new_short_build() {
    let resolved = resolve_contract_whale_oi_context_from_window(
        ContractWhaleStructureInterpretation::ActiveSellPressure,
        ContractWhaleActiveFlowDirection::SellDominant,
        ContractWhalePriceResponseType::TrendFollowDown,
        Some(-0.45),
        Some(&ContractWhaleOiWindowContext {
            oi_before: Some(100_000.0),
            oi_after: Some(100_450.0),
            oi_delta: Some(450.0),
            oi_delta_pct: Some(0.45),
            before_ts: Some(1_700_000_000_000),
            after_ts: Some(1_700_000_015_000),
            available: true,
            reason: None,
            ..ContractWhaleOiWindowContext::default()
        }),
        &ContractWhaleRuntimeConfig::default(),
    );

    assert_eq!(
        resolved.oi_context,
        ContractWhaleOiContextTag::NewShortBuild
    );
    assert_eq!(resolved.oi_context_label, "新空开仓");
    assert_eq!(
        resolved.oi_reason.as_deref(),
        Some("oi_increased_with_sell_pressure")
    );
}

#[test]
fn oi_window_context_maps_sell_follow_through_to_long_unwind_when_oi_falls() {
    let resolved = resolve_contract_whale_oi_context_from_window(
        ContractWhaleStructureInterpretation::ActiveSellPressure,
        ContractWhaleActiveFlowDirection::SellDominant,
        ContractWhalePriceResponseType::TrendFollowDown,
        Some(-0.28),
        Some(&ContractWhaleOiWindowContext {
            oi_before: Some(100_000.0),
            oi_after: Some(99_720.0),
            oi_delta: Some(-280.0),
            oi_delta_pct: Some(-0.28),
            before_ts: Some(1_700_000_000_000),
            after_ts: Some(1_700_000_015_000),
            available: true,
            reason: None,
            ..ContractWhaleOiWindowContext::default()
        }),
        &ContractWhaleRuntimeConfig::default(),
    );

    assert_eq!(resolved.oi_context, ContractWhaleOiContextTag::LongUnwind);
    assert_eq!(resolved.oi_context_label, "多头平仓");
    assert_eq!(
        resolved.oi_reason.as_deref(),
        Some("oi_decreased_with_sell_pressure")
    );
}

#[test]
fn oi_window_context_marks_small_delta_as_not_confirmed() {
    let resolved = resolve_contract_whale_oi_context_from_window(
        ContractWhaleStructureInterpretation::ActiveBuyPressure,
        ContractWhaleActiveFlowDirection::BuyDominant,
        ContractWhalePriceResponseType::TrendFollowUp,
        Some(0.12),
        Some(&ContractWhaleOiWindowContext {
            oi_before: Some(100_000.0),
            oi_after: Some(100_040.0),
            oi_delta: Some(40.0),
            oi_delta_pct: Some(0.04),
            before_ts: Some(1_700_000_000_000),
            after_ts: Some(1_700_000_015_000),
            available: true,
            reason: None,
            ..ContractWhaleOiWindowContext::default()
        }),
        &ContractWhaleRuntimeConfig::default(),
    );

    assert_eq!(
        resolved.oi_context,
        ContractWhaleOiContextTag::OiNotConfirmed
    );
    assert_eq!(resolved.oi_context_label, "OI 不确认");
    assert_eq!(
        resolved.oi_reason.as_deref(),
        Some("oi_delta_below_threshold")
    );
    assert!(resolved.oi_available);
}

#[test]
fn oi_window_context_marks_balanced_flow_as_not_confirmed() {
    let resolved = resolve_contract_whale_oi_context_from_window(
        ContractWhaleStructureInterpretation::UnclearDirectionalFlow,
        ContractWhaleActiveFlowDirection::Balanced,
        ContractWhalePriceResponseType::NoClearResponse,
        Some(0.02),
        Some(&ContractWhaleOiWindowContext {
            oi_before: Some(100_000.0),
            oi_after: Some(100_420.0),
            oi_delta: Some(420.0),
            oi_delta_pct: Some(0.42),
            before_ts: Some(1_700_000_000_000),
            after_ts: Some(1_700_000_015_000),
            available: true,
            reason: None,
            ..ContractWhaleOiWindowContext::default()
        }),
        &ContractWhaleRuntimeConfig::default(),
    );

    assert_eq!(
        resolved.oi_context,
        ContractWhaleOiContextTag::OiNotConfirmed
    );
    assert_eq!(resolved.oi_context_label, "OI 不确认");
    assert_eq!(
        resolved.oi_reason.as_deref(),
        Some("direction_not_clear_for_oi_context")
    );
}

#[test]
fn oi_window_context_keeps_absorption_as_not_confirmed_but_explains_position_conflict() {
    let resolved = resolve_contract_whale_oi_context_from_window(
        ContractWhaleStructureInterpretation::DownsideAbsorption,
        ContractWhaleActiveFlowDirection::SellDominant,
        ContractWhalePriceResponseType::DownsideAbsorption,
        Some(-0.03),
        Some(&ContractWhaleOiWindowContext {
            oi_before: Some(100_000.0),
            oi_after: Some(100_180.0),
            oi_delta: Some(180.0),
            oi_delta_pct: Some(0.18),
            before_ts: Some(1_700_000_000_000),
            after_ts: Some(1_700_000_015_000),
            available: true,
            reason: None,
            ..ContractWhaleOiWindowContext::default()
        }),
        &ContractWhaleRuntimeConfig::default(),
    );

    assert_eq!(
        resolved.oi_context,
        ContractWhaleOiContextTag::OiNotConfirmed
    );
    assert_eq!(resolved.oi_context_label, "OI 增加，新仓对抗");
    assert_eq!(
        resolved.oi_reason.as_deref(),
        Some("oi_increased_during_absorption_or_suppression")
    );
    assert_eq!(resolved.oi_delta_pct, Some(0.18));
}

#[test]
fn detector_rejects_missing_price_data_instead_of_inventing_price_response() {
    let now = 1_700_000_015_000;
    let trades = vec![normalize_binance_agg_trade(now - 1_000, 70_000.0, 1_600.0, false).unwrap()];
    let buckets = aggregate_1s_buckets(&trades);
    let stats =
        rolling_window_stats(&buckets, "BTC", 15, now, None, Some(5.4), 86).expect("window stats");
    assert!(detect_contract_whale_signal(&stats).is_none());
}

#[test]
fn scoring_weights_can_be_adjusted_without_changing_detector_code() {
    let now = 1_700_000_015_000;
    let trades = vec![
        normalize_binance_agg_trade(now - 1_000, 70_000.0, 3_000.0, false).unwrap(),
        normalize_okx_swap_trade(now - 1_000, 70_000.0, 200_000.0, 0.01, "buy").unwrap(),
    ];
    let buckets = aggregate_1s_buckets(&trades);
    let stats = rolling_window_stats(&buckets, "BTC", 15, now, Some(0.31), Some(10.2), 94)
        .expect("window stats");
    let default_score = score_contract_whale_signal_with_config(
        &stats,
        ContractWhaleSignalType::AggressiveBuy,
        &ContractWhaleRuntimeConfig::default(),
    );
    let mut tuned = ContractWhaleRuntimeConfig::default();
    tuned.scoring.volume_strength_weight = 5.0;
    tuned.scoring.dynamic_multiple_weight = 5.0;
    tuned.scoring.dominance_weight = 5.0;
    let tuned_score = score_contract_whale_signal_with_config(
        &stats,
        ContractWhaleSignalType::AggressiveBuy,
        &tuned,
    );

    assert!(tuned_score < default_score);
}

#[test]
fn symbol_threshold_config_keeps_disabled_symbols_out_and_tunes_btc() {
    let now = 1_700_000_015_000;
    let trades = vec![
        normalize_binance_agg_trade(now - 1_000, 70_000.0, 3_000.0, false).unwrap(),
        normalize_okx_swap_trade(now - 1_000, 70_000.0, 200_000.0, 0.01, "buy").unwrap(),
        normalize_bitfinex_trade(now - 1_000, 70_000.0, 500.0).unwrap(),
    ];
    let buckets = aggregate_1s_buckets(&trades);
    let mut tuned = three_exchange_config();
    let mut stats = rolling_window_stats_with_config(
        &buckets,
        "BTC",
        15,
        now,
        RollingWindowStatsOptions {
            price_move_pct: Some(0.31),
            dynamic_multiple: Some(10.2),
            dynamic_baseline_btc: None,
            dynamic_threshold_level: String::new(),
            data_quality: 94,
            config: &tuned,
        },
    )
    .expect("window stats");
    let btc_thresholds = tuned
        .symbols
        .get_mut("BTC")
        .expect("btc symbol")
        .thresholds_btc
        .get_mut(&15)
        .expect("15s threshold");
    btc_thresholds.high_btc = 10_000.0;
    btc_thresholds.critical_btc = 20_000.0;
    btc_thresholds.s_btc = 30_000.0;

    let signal = detect_contract_whale_signal_with_config(&stats, &tuned).expect("btc signal");
    assert_eq!(signal.severity, ContractWhaleSeverity::Medium);

    stats.symbol = "SOL".to_string();
    assert!(detect_contract_whale_signal_with_config(&stats, &tuned).is_none());
}

#[test]
fn data_quality_blocks_discord_during_warmup_and_ctval_missing() {
    let now = 1_700_000_015_000;
    let trades = vec![
        normalize_binance_agg_trade(now - 1_000, 70_000.0, 3_000.0, false).unwrap(),
        normalize_okx_swap_trade(now - 1_000, 70_000.0, 200_000.0, 0.01, "buy").unwrap(),
        normalize_bitfinex_trade(now - 1_000, 70_000.0, 500.0).unwrap(),
    ];
    let buckets = aggregate_1s_buckets(&trades);
    let mut stats = rolling_window_stats(&buckets, "BTC", 15, now, Some(0.31), Some(10.2), 94)
        .expect("window stats");
    stats.startup_age_ms = Some(10_000);
    let warmup_signal =
        detect_contract_whale_signal_with_config(&stats, &ContractWhaleRuntimeConfig::default())
            .expect("warmup signal");
    assert!(warmup_signal.severity < ContractWhaleSeverity::Critical);
    assert!(!warmup_signal.discord_eligible);
    assert_eq!(warmup_signal.discord_reason, "warmup_collect_only");

    stats.startup_age_ms = None;
    stats.market_context = ContractWhaleMarketContext {
        context_expected: true,
        ct_val_available: false,
        ..ContractWhaleMarketContext::default()
    };
    let missing_ctval_signal =
        detect_contract_whale_signal_with_config(&stats, &ContractWhaleRuntimeConfig::default())
            .expect("ctval signal");
    assert!(missing_ctval_signal.data_quality < 70);
    assert!(!missing_ctval_signal.discord_eligible);
}

#[test]
fn detector_marks_btc_high_signal_pushable_while_data_quality_controls_eligibility() {
    let now = 1_700_000_015_000;
    let trades = vec![normalize_binance_agg_trade(now - 1_000, 70_000.0, 1_600.0, false).unwrap()];
    let buckets = aggregate_1s_buckets(&trades);
    let mut stats = rolling_window_stats(&buckets, "BTC", 15, now, Some(0.12), Some(5.2), 80)
        .expect("window stats");
    stats.percentile_level = Some(99.0);
    let signal = detect_contract_whale_signal(&stats).expect("signal");

    assert_eq!(signal.severity, ContractWhaleSeverity::High);
    assert!(!should_push_contract_whale_discord(&signal));
    assert!(!signal.discord_eligible);
    assert_eq!(signal.discord_reason, "s_grade_extreme_impact_required");
    assert!(!signal.discord_sent);
}

#[test]
fn detector_allows_primary_single_exchange_extreme_high_override() {
    let now = 1_700_000_060_000;
    let trades = vec![
        normalize_binance_agg_trade(now - 1_000, 63_000.0, 1_453.5, false).unwrap(),
        normalize_binance_agg_trade(now - 1_000, 63_000.0, 308.5, true).unwrap(),
    ];
    let buckets = aggregate_1s_buckets(&trades);
    let stats =
        rolling_window_stats(&buckets, "BTC", 60, now, Some(0.22), None, 68).expect("60s stats");
    let signal = detect_contract_whale_signal(&stats).expect("high override signal");

    assert_eq!(signal.severity, ContractWhaleSeverity::High);
    assert!(signal.data_quality <= 68);
    assert!(!signal.discord_eligible);
    assert_eq!(signal.discord_reason, "data_quality_display_only");
    assert!(!should_push_contract_whale_discord(&signal));
}

#[test]
fn detector_does_not_promote_multi_exchange_flow_when_dynamic_evidence_is_missing() {
    let now = 1_700_000_015_000;
    let trades = vec![
        normalize_binance_agg_trade(now - 1_000, 70_000.0, 1_000.0, false).unwrap(),
        normalize_okx_swap_trade(now - 1_000, 70_000.0, 200_000.0, 0.01, "buy").unwrap(),
    ];
    let buckets = aggregate_1s_buckets(&trades);
    let config = three_exchange_config();
    let mut stats = rolling_window_stats_with_config(
        &buckets,
        "BTC",
        15,
        now,
        RollingWindowStatsOptions {
            price_move_pct: Some(0.31),
            dynamic_multiple: None,
            dynamic_baseline_btc: None,
            dynamic_threshold_level: String::new(),
            data_quality: 90,
            config: &config,
        },
    )
    .expect("window stats");
    stats.percentile_level = Some(99.9);

    let signal = detect_contract_whale_signal_with_config(&stats, &config).expect("signal");

    assert_ne!(signal.severity, ContractWhaleSeverity::High);
    assert_ne!(signal.severity, ContractWhaleSeverity::Critical);
    assert_ne!(signal.severity, ContractWhaleSeverity::S);
}

#[test]
fn detector_can_restore_legacy_missing_evidence_behavior_for_shadow_comparison() {
    let now = 1_700_000_015_000;
    let trades = vec![
        normalize_binance_agg_trade(now - 1_000, 70_000.0, 1_000.0, false).unwrap(),
        normalize_okx_swap_trade(now - 1_000, 70_000.0, 200_000.0, 0.01, "buy").unwrap(),
    ];
    let buckets = aggregate_1s_buckets(&trades);
    let mut config = three_exchange_config();
    config.classification.evidence_fail_closed_enabled = false;
    let mut stats = rolling_window_stats_with_config(
        &buckets,
        "BTC",
        15,
        now,
        RollingWindowStatsOptions {
            price_move_pct: Some(0.31),
            dynamic_multiple: None,
            dynamic_baseline_btc: None,
            dynamic_threshold_level: String::new(),
            data_quality: 90,
            config: &config,
        },
    )
    .expect("window stats");
    stats.percentile_level = Some(99.9);

    let signal = detect_contract_whale_signal_with_config(&stats, &config).expect("signal");

    assert!(signal.severity >= ContractWhaleSeverity::High);
}

#[test]
fn detector_does_not_promote_multi_exchange_flow_when_percentile_evidence_is_missing() {
    let now = 1_700_000_015_000;
    let trades = vec![
        normalize_binance_agg_trade(now - 1_000, 70_000.0, 1_000.0, false).unwrap(),
        normalize_okx_swap_trade(now - 1_000, 70_000.0, 200_000.0, 0.01, "buy").unwrap(),
    ];
    let buckets = aggregate_1s_buckets(&trades);
    let config = three_exchange_config();
    let stats = rolling_window_stats_with_config(
        &buckets,
        "BTC",
        15,
        now,
        RollingWindowStatsOptions {
            price_move_pct: Some(0.31),
            dynamic_multiple: Some(10.5),
            dynamic_baseline_btc: Some(100.0),
            dynamic_threshold_level: "p99".to_string(),
            data_quality: 90,
            config: &config,
        },
    )
    .expect("window stats");

    let signal = detect_contract_whale_signal_with_config(&stats, &config).expect("signal");

    assert_ne!(signal.severity, ContractWhaleSeverity::High);
    assert_ne!(signal.severity, ContractWhaleSeverity::Critical);
    assert_ne!(signal.severity, ContractWhaleSeverity::S);
}

#[test]
fn detector_recovers_critical_when_dynamic_baseline_is_temporarily_unavailable() {
    let now = 1_700_000_015_000;
    let trades = vec![
        normalize_binance_agg_trade(now - 1_000, 70_000.0, 3_200.0, false).unwrap(),
        normalize_binance_agg_trade(now - 1_000, 70_000.0, 400.0, true).unwrap(),
    ];
    let buckets = aggregate_1s_buckets(&trades);
    let stats =
        rolling_window_stats(&buckets, "BTC", 15, now, Some(0.21), None, 80).expect("15s stats");
    let signal = detect_contract_whale_signal(&stats).expect("critical fallback signal");

    assert_eq!(signal.severity, ContractWhaleSeverity::Critical);
    assert!(signal.discord_eligible);
    assert_eq!(signal.discord_reason, "critical_or_s_gate");
    let classification =
        serde_json::to_value(&signal.classification_v2).expect("classification json");
    assert_eq!(classification["evidence"]["evidenceDegraded"], true);
    assert_eq!(
        classification["evidence"]["evidenceReason"],
        "critical_absolute_fallback"
    );
}

#[test]
fn detector_filters_low_price_response_medium_noise() {
    let now = 1_700_000_015_000;
    let trades = vec![
        normalize_binance_agg_trade(now - 1_000, 70_000.0, 900.0, false).unwrap(),
        normalize_okx_swap_trade(now - 1_000, 70_000.0, 80_000.0, 0.01, "buy").unwrap(),
    ];
    let buckets = aggregate_1s_buckets(&trades);
    let stats = rolling_window_stats(&buckets, "BTC", 15, now, Some(0.14), Some(4.2), 85)
        .expect("window stats");
    let signal = detect_contract_whale_signal(&stats);

    assert!(
        signal.is_none(),
        "low price-response noise should not emit a Medium signal"
    );
}

#[test]
fn detector_keeps_medium_when_price_response_confirms_trend() {
    let now = 1_700_000_015_000;
    let trades = vec![
        normalize_binance_agg_trade(now - 1_000, 70_000.0, 900.0, false).unwrap(),
        normalize_okx_swap_trade(now - 1_000, 70_000.0, 80_000.0, 0.01, "buy").unwrap(),
    ];
    let buckets = aggregate_1s_buckets(&trades);
    let stats = rolling_window_stats(&buckets, "BTC", 15, now, Some(0.31), Some(4.2), 85)
        .expect("window stats");
    let signal = detect_contract_whale_signal(&stats).expect("medium signal");

    assert_eq!(signal.severity, ContractWhaleSeverity::Medium);
    assert_eq!(signal.dynamic_multiple, Some(4.2));
    assert!(!signal.discord_eligible);
    assert_eq!(signal.discord_reason, "medium_observe_only");
    assert!(!should_push_contract_whale_discord(&signal));
}

#[test]
fn detector_rejects_medium_when_directional_dominance_or_quality_is_weak() {
    let now = 1_700_000_015_000;
    let trades = vec![
        normalize_binance_agg_trade(now - 1_000, 70_000.0, 900.0, false).unwrap(),
        normalize_binance_agg_trade(now - 1_000, 70_000.0, 850.0, true).unwrap(),
    ];
    let buckets = aggregate_1s_buckets(&trades);
    let mut stats = rolling_window_stats(&buckets, "BTC", 15, now, Some(0.31), Some(5.2), 85)
        .expect("window stats");

    assert!(detect_contract_whale_signal(&stats).is_none());

    stats.dominance = 0.60;
    stats.data_quality = 55;
    assert!(detect_contract_whale_signal(&stats).is_none());
}

#[test]
fn market_context_rejects_stale_oi_and_funding_snapshots() {
    let now = 1_700_000_300_000;
    let stale_ts = now - 120_000;
    let context = market_context_from_snapshots(
        &[ContractOiSnapshot {
            ts: stale_ts,
            exchange: ContractExchange::Binance,
            symbol: "BTC".to_string(),
            oi_btc: 10_000.0,
            oi_notional_usd: Some(700_000_000.0),
            ct_val_available: true,
            evidence_degraded_reason: None,
        }],
        &[ContractFundingSnapshot {
            ts: stale_ts,
            exchange: ContractExchange::Binance,
            symbol: "BTC".to_string(),
            funding_rate: 0.0004,
        }],
        "BTC",
        now,
    );

    assert!(!context.oi_available);
    assert!(!context.funding_available);
    assert_eq!(context.oi_reason.as_deref(), Some("stale"));
    assert_eq!(context.funding_reason.as_deref(), Some("stale"));
    assert!(context.evidence_degraded);
}

#[test]
fn detector_populates_market_impact_fields() {
    let now = 1_700_000_015_000;
    let trades = vec![
        normalize_binance_agg_trade(now - 1_000, 70_000.0, 900.0, false).unwrap(),
        normalize_okx_swap_trade(now - 1_000, 70_000.0, 80_000.0, 0.01, "buy").unwrap(),
    ];
    let buckets = aggregate_1s_buckets(&trades);
    let mut stats = rolling_window_stats(&buckets, "BTC", 15, now, Some(0.31), Some(4.2), 85)
        .expect("window stats");
    stats.percentile_level = Some(90.0);
    let signal = detect_contract_whale_signal(&stats).expect("medium signal");

    assert_eq!(signal.impact_level.as_deref(), Some("C"));
    assert_eq!(signal.signal_level.as_deref(), Some("L1"));
    assert_eq!(signal.signal_label.as_deref(), Some("LOW IMPACT EVENT"));
    assert_eq!(signal.normalized_strength.as_deref(), Some("EXTREME"));
    assert_eq!(signal.impact_score, Some(4.2));
    assert_eq!(signal.impact_z_score, Some(4.2));
    assert!(!signal.discord_eligible);
    assert!(!signal.discord_would_send);
}

#[test]
fn detector_uses_percentile_level_to_suppress_active_market_noise() {
    let now = 1_700_000_015_000;
    let trades = vec![
        normalize_binance_agg_trade(now - 1_000, 70_000.0, 3_000.0, false).unwrap(),
        normalize_okx_swap_trade(now - 1_000, 70_000.0, 200_000.0, 0.01, "buy").unwrap(),
    ];
    let buckets = aggregate_1s_buckets(&trades);
    let config = three_exchange_config();
    let mut stats = rolling_window_stats_with_config(
        &buckets,
        "BTC",
        15,
        now,
        RollingWindowStatsOptions {
            price_move_pct: Some(0.31),
            dynamic_multiple: Some(10.5),
            dynamic_baseline_btc: None,
            dynamic_threshold_level: String::new(),
            data_quality: 94,
            config: &config,
        },
    )
    .expect("window stats");
    stats.percentile_level = Some(98.0);
    let signal =
        detect_contract_whale_signal_with_config(&stats, &config).expect("downgraded signal");

    assert_eq!(signal.severity, ContractWhaleSeverity::Medium);
    assert_eq!(signal.percentile_level, Some(98.0));

    stats.percentile_level = Some(99.9);
    let signal = detect_contract_whale_signal_with_config(&stats, &config).expect("s signal");
    assert_eq!(signal.severity, ContractWhaleSeverity::S);
    assert!(signal.multi_exchange_confirmed);
    assert_eq!(signal.impact_level.as_deref(), Some("B"));
    assert_eq!(signal.signal_level.as_deref(), Some("L2"));
}

#[test]
fn historic_s_impact_requires_hard_extreme_evidence() {
    let now = 1_700_000_015_000;
    let trades = vec![
        normalize_binance_agg_trade(now - 1_000, 70_000.0, 3_000.0, false).unwrap(),
        normalize_okx_swap_trade(now - 1_000, 70_000.0, 200_000.0, 0.01, "buy").unwrap(),
    ];
    let buckets = aggregate_1s_buckets(&trades);
    let config = three_exchange_config();
    let mut stats = rolling_window_stats_with_config(
        &buckets,
        "BTC",
        15,
        now,
        RollingWindowStatsOptions {
            price_move_pct: Some(0.31),
            dynamic_multiple: Some(10.5),
            dynamic_baseline_btc: None,
            dynamic_threshold_level: String::new(),
            data_quality: 94,
            config: &config,
        },
    )
    .expect("window stats");
    stats.percentile_level = Some(99.9);
    let mut signal = detect_contract_whale_signal_with_config(&stats, &config).expect("signal");
    assert_eq!(signal.impact_level.as_deref(), Some("B"));

    signal.total_volume_btc = 20_000.0;
    signal.window_sec = 60;
    signal.multi_exchange_confirmed = true;
    signal.dynamic_multiple = Some(10.0);
    signal.percentile_level = Some(99.5);
    signal.dominance = 0.65;
    signal.impact_level = Some("S".to_string());
    assert!(is_historic_s_impact(&signal));
}

#[test]
fn one_pulse_and_fragmented_flow_expose_distinct_footprints() {
    let now = 1_700_000_000_000;
    let pulse = normalize_binance_agg_trade(now, 70_000.0, 400.0, false).unwrap();
    let fragmented = (0..40)
        .map(|index| normalize_binance_agg_trade(now + index, 70_000.0, 10.0, false).unwrap())
        .collect::<Vec<_>>();

    let pulse_bucket = aggregate_1s_buckets(&[pulse]).pop().expect("pulse bucket");
    let fragmented_bucket = aggregate_1s_buckets(&fragmented)
        .pop()
        .expect("fragmented bucket");

    assert_eq!(pulse_bucket.buy_trade_count, 1);
    assert_eq!(pulse_bucket.sell_trade_count, 0);
    assert_eq!(fragmented_bucket.buy_trade_count, 40);
    assert!(pulse_bucket.max_single_trade_share > fragmented_bucket.max_single_trade_share);
}

#[test]
fn detector_keeps_low_score_5s_btc_high_display_only() {
    let now = 1_700_000_005_000;
    let trades = vec![
        normalize_binance_agg_trade(now - 1_000, 70_000.0, 600.0, false).unwrap(),
        normalize_okx_swap_trade(now - 1_000, 70_000.0, 30_000.0, 0.01, "buy").unwrap(),
        normalize_binance_agg_trade(now - 1_000, 70_000.0, 50.0, true).unwrap(),
    ];
    let buckets = aggregate_1s_buckets(&trades);
    let mut stats =
        rolling_window_stats(&buckets, "BTC", 5, now, Some(0.12), Some(5.2), 86).expect("5s stats");
    stats.percentile_level = Some(99.0);
    let signal = detect_contract_whale_signal(&stats).expect("5s signal");

    assert_eq!(signal.window_sec, 5);
    assert_eq!(signal.signal_type, ContractWhaleSignalType::AggressiveBuy);
    assert_eq!(signal.severity, ContractWhaleSeverity::High);
    assert!(signal.score < 70);
    assert!(!signal.discord_eligible);
    assert!(!should_push_contract_whale_discord(&signal));
    assert_eq!(signal.discord_reason, "s_grade_extreme_impact_required");
}

#[test]
fn discord_push_requires_symbol_min_total_volume_thresholds() {
    let now = 1_700_000_005_000;
    let trades = vec![
        normalize_binance_agg_trade(now - 1_000, 70_000.0, 600.0, false).unwrap(),
        normalize_okx_swap_trade(now - 1_000, 70_000.0, 30_000.0, 0.01, "buy").unwrap(),
        normalize_binance_agg_trade(now - 1_000, 70_000.0, 50.0, true).unwrap(),
    ];
    let buckets = aggregate_1s_buckets(&trades);
    let mut stats =
        rolling_window_stats(&buckets, "BTC", 5, now, Some(0.12), Some(5.2), 86).expect("5s stats");
    stats.percentile_level = Some(99.0);
    let signal = detect_contract_whale_signal(&stats).expect("high signal");

    let mut signal = signal;
    signal.impact_level = Some("A".to_string());
    signal.signal_level = Some("L3".to_string());

    let mut btc_below_gate = signal.clone();
    btc_below_gate.symbol = "BTC".to_string();
    btc_below_gate.base_asset = "BTC".to_string();
    btc_below_gate.quantity_unit = "BTC".to_string();
    btc_below_gate.total_volume_btc = 499.0;
    btc_below_gate.total_volume = 499.0;
    assert!(!should_push_contract_whale_discord(&btc_below_gate));

    let mut btc_lifecycle_at_gate = btc_below_gate.clone();
    btc_lifecycle_at_gate.event_lifecycle.volume_accumulated = 520.0;
    assert!(should_push_contract_whale_discord(&btc_lifecycle_at_gate));

    let mut first_lifecycle_update = btc_below_gate.clone();
    first_lifecycle_update.id = "contract-whale:BTC:15:1700000000000:lifecycle-first".to_string();
    first_lifecycle_update.ts = now;
    first_lifecycle_update.total_volume_btc = 260.0;
    first_lifecycle_update.total_volume = 260.0;
    first_lifecycle_update.net_volume_btc = 230.0;
    first_lifecycle_update.net_volume = 230.0;
    first_lifecycle_update.discord_eligible = false;
    first_lifecycle_update.discord_would_send = false;
    first_lifecycle_update.discord_reason = "below_push_volume_threshold".to_string();

    let mut second_lifecycle_update = first_lifecycle_update.clone();
    second_lifecycle_update.id = "contract-whale:BTC:15:1700000015000:lifecycle-second".to_string();
    second_lifecycle_update.ts = now + 15_000;

    let lifecycle_events = apply_contract_whale_event_lifecycle(
        vec![first_lifecycle_update, second_lifecycle_update],
        ContractWhaleLifecycleClock::Replay {
            replay_now_ms: now + 15_000,
        },
    );
    assert_eq!(lifecycle_events.len(), 1);
    assert_eq!(
        lifecycle_events[0].event_lifecycle.volume_accumulated, 260.0,
        "overlapping lifecycle windows must map the deprecated accumulated field to peak volume"
    );
    assert!(!lifecycle_events[0].discord_eligible);
    assert!(!lifecycle_events[0].discord_would_send);
    assert!(!should_push_contract_whale_discord(&lifecycle_events[0]));

    let mut btc_at_gate = btc_below_gate.clone();
    btc_at_gate.total_volume_btc = 500.0;
    btc_at_gate.total_volume = 500.0;
    assert!(should_push_contract_whale_discord(&btc_at_gate));

    let mut eth_below_gate = signal.clone();
    eth_below_gate.symbol = "ETH".to_string();
    eth_below_gate.base_asset = "ETH".to_string();
    eth_below_gate.quantity_unit = "ETH".to_string();
    eth_below_gate.discord_reason = "high_primary_source_extreme".to_string();
    eth_below_gate.total_volume_btc = 29_999.0;
    eth_below_gate.total_volume = 29_999.0;
    assert!(!should_push_contract_whale_discord(&eth_below_gate));

    let mut eth_at_gate = eth_below_gate.clone();
    eth_at_gate.total_volume_btc = 30_000.0;
    eth_at_gate.total_volume = 30_000.0;
    assert!(should_push_contract_whale_discord(&eth_at_gate));
}

#[test]
fn lifecycle_keeps_peak_snapshot_separate_from_latest_update() {
    let now = 1_700_000_005_000;
    let trades = vec![
        normalize_binance_agg_trade(now - 1_000, 70_000.0, 600.0, false).unwrap(),
        normalize_okx_swap_trade(now - 1_000, 70_000.0, 30_000.0, 0.01, "buy").unwrap(),
        normalize_binance_agg_trade(now - 1_000, 70_000.0, 50.0, true).unwrap(),
    ];
    let buckets = aggregate_1s_buckets(&trades);
    let stats =
        rolling_window_stats(&buckets, "BTC", 5, now, Some(0.12), Some(5.2), 86).expect("stats");
    let mut peak = detect_contract_whale_signal(&stats).expect("peak signal");
    peak.id = "contract-whale:BTC:5:peak".to_string();
    peak.ts = now;
    peak.total_volume_btc = 1_000.0;
    peak.total_volume = 1_000.0;
    peak.net_volume_btc = 860.0;
    peak.net_volume = 860.0;
    peak.score = 90;

    let mut latest = peak.clone();
    latest.id = "contract-whale:BTC:5:latest".to_string();
    latest.ts = now + 15_000;
    latest.total_volume_btc = 600.0;
    latest.total_volume = 600.0;
    latest.net_volume_btc = 520.0;
    latest.net_volume = 520.0;
    latest.score = 80;

    let events = apply_contract_whale_event_lifecycle(
        vec![peak.clone(), latest.clone()],
        ContractWhaleLifecycleClock::Replay {
            replay_now_ms: latest.ts,
        },
    );

    assert_eq!(events.len(), 1);
    assert_eq!(
        events[0].ts, peak.ts,
        "display fields must remain the peak snapshot"
    );
    assert_eq!(events[0].event_lifecycle.last_update_time, latest.ts);
    let lifecycle = serde_json::to_value(&events[0].event_lifecycle).expect("lifecycle json");
    assert_eq!(lifecycle["latestSnapshotTs"], latest.ts);
    assert_eq!(lifecycle["peakSnapshotTs"], peak.ts);
    assert_eq!(lifecycle["displaySnapshotKind"], "peak");
    assert_eq!(lifecycle["latestSnapshot"]["ts"], latest.ts);
    assert_eq!(lifecycle["latestSnapshot"]["totalVolumeBtc"], 600.0);
    assert_eq!(lifecycle["peakSnapshot"]["ts"], peak.ts);
    assert_eq!(lifecycle["peakSnapshot"]["totalVolumeBtc"], 1_000.0);
}

#[test]
fn detector_triggers_15s_critical_threshold() {
    let now = 1_700_000_015_000;
    let trades = vec![
        normalize_binance_agg_trade(now - 1_000, 70_000.0, 2_400.0, false).unwrap(),
        normalize_okx_swap_trade(now - 1_000, 70_000.0, 180_000.0, 0.01, "buy").unwrap(),
        normalize_binance_agg_trade(now - 1_000, 70_000.0, 200.0, true).unwrap(),
    ];
    let buckets = aggregate_1s_buckets(&trades);
    let config = three_exchange_config();
    let mut stats = rolling_window_stats_with_config(
        &buckets,
        "BTC",
        15,
        now,
        RollingWindowStatsOptions {
            price_move_pct: Some(0.18),
            dynamic_multiple: Some(7.4),
            dynamic_baseline_btc: None,
            dynamic_threshold_level: String::new(),
            data_quality: 90,
            config: &config,
        },
    )
    .expect("15s stats");
    stats.percentile_level = Some(99.5);
    let signal = detect_contract_whale_signal_with_config(&stats, &config).expect("15s signal");

    assert_eq!(signal.window_sec, 15);
    assert_eq!(signal.severity, ContractWhaleSeverity::Critical);
    assert_eq!(signal.direction, ContractWhaleDirection::Buy);
    assert!(!signal.discord_eligible);
    assert_eq!(signal.discord_reason, "s_grade_extreme_impact_required");
}

#[test]
fn detector_triggers_60s_high_threshold() {
    let now = 1_700_000_060_000;
    let trades = vec![
        normalize_binance_agg_trade(now - 1_000, 70_000.0, 2_000.0, false).unwrap(),
        normalize_okx_swap_trade(now - 1_000, 70_000.0, 150_000.0, 0.01, "buy").unwrap(),
        normalize_bitfinex_trade(now - 1_000, 70_000.0, -200.0).unwrap(),
    ];
    let buckets = aggregate_1s_buckets(&trades);
    let config = three_exchange_config();
    let mut stats = rolling_window_stats_with_config(
        &buckets,
        "BTC",
        60,
        now,
        RollingWindowStatsOptions {
            price_move_pct: Some(0.14),
            dynamic_multiple: Some(5.3),
            dynamic_baseline_btc: None,
            dynamic_threshold_level: String::new(),
            data_quality: 88,
            config: &config,
        },
    )
    .expect("60s stats");
    stats.percentile_level = Some(99.0);
    let signal = detect_contract_whale_signal_with_config(&stats, &config).expect("60s signal");

    assert_eq!(signal.window_sec, 60);
    assert_eq!(signal.severity, ContractWhaleSeverity::High);
    assert_eq!(signal.signal_type, ContractWhaleSignalType::AggressiveBuy);
}

#[test]
fn detector_filters_weak_dominance_crossflow_noise() {
    let now = 1_700_000_015_000;
    let trades = vec![
        normalize_binance_agg_trade(now - 1_000, 70_000.0, 3_000.0, false).unwrap(),
        normalize_okx_swap_trade(now - 1_000, 70_000.0, 260_000.0, 0.01, "sell").unwrap(),
    ];
    let buckets = aggregate_1s_buckets(&trades);
    let config = three_exchange_config();
    let mut stats = rolling_window_stats_with_config(
        &buckets,
        "BTC",
        15,
        now,
        RollingWindowStatsOptions {
            price_move_pct: Some(0.20),
            dynamic_multiple: Some(10.0),
            dynamic_baseline_btc: None,
            dynamic_threshold_level: String::new(),
            data_quality: 92,
            config: &config,
        },
    )
    .expect("weak dominance stats");
    stats.percentile_level = Some(99.9);
    let signal = detect_contract_whale_signal_with_config(&stats, &config);

    assert!(
        signal.is_none(),
        "weak-dominance crossflow should no longer survive as a display signal"
    );
}

#[test]
fn scoring_rewards_multi_exchange_confirmation() {
    let now = 1_700_000_015_000;
    let single_exchange_trades = vec![
        normalize_binance_agg_trade(now - 1_000, 70_000.0, 2_800.0, false).unwrap(),
        normalize_binance_agg_trade(now - 1_000, 70_000.0, 400.0, true).unwrap(),
    ];
    let multi_exchange_trades = vec![
        normalize_binance_agg_trade(now - 1_000, 70_000.0, 1_400.0, false).unwrap(),
        normalize_okx_swap_trade(now - 1_000, 70_000.0, 140_000.0, 0.01, "buy").unwrap(),
        normalize_binance_agg_trade(now - 1_000, 70_000.0, 400.0, true).unwrap(),
    ];
    let config = three_exchange_config();
    let single_stats = rolling_window_stats_with_config(
        &aggregate_1s_buckets(&single_exchange_trades),
        "BTC",
        15,
        now,
        RollingWindowStatsOptions {
            price_move_pct: Some(0.18),
            dynamic_multiple: Some(7.0),
            dynamic_baseline_btc: None,
            dynamic_threshold_level: String::new(),
            data_quality: 90,
            config: &config,
        },
    )
    .expect("single stats");
    let multi_stats = rolling_window_stats_with_config(
        &aggregate_1s_buckets(&multi_exchange_trades),
        "BTC",
        15,
        now,
        RollingWindowStatsOptions {
            price_move_pct: Some(0.18),
            dynamic_multiple: Some(7.0),
            dynamic_baseline_btc: None,
            dynamic_threshold_level: String::new(),
            data_quality: 90,
            config: &config,
        },
    )
    .expect("multi stats");

    let single_score = score_contract_whale_signal_with_config(
        &single_stats,
        ContractWhaleSignalType::AggressiveBuy,
        &config,
    );
    let multi_score = score_contract_whale_signal_with_config(
        &multi_stats,
        ContractWhaleSignalType::AggressiveBuy,
        &config,
    );

    assert!(multi_stats.exchange_count > single_stats.exchange_count);
    assert!(multi_score > single_score);
}

#[test]
fn detector_marks_absorption_as_high_when_sellers_fail_to_move_price() {
    let now = 1_700_000_015_000;
    let trades = vec![
        normalize_binance_agg_trade(now - 1_000, 70_000.0, 1_500.0, true).unwrap(),
        normalize_okx_swap_trade(now - 1_000, 70_000.0, 60_000.0, 0.01, "sell").unwrap(),
        normalize_binance_agg_trade(now - 1_000, 70_000.0, 150.0, false).unwrap(),
    ];
    let buckets = aggregate_1s_buckets(&trades);
    let mut stats = rolling_window_stats(&buckets, "BTC", 15, now, Some(-0.03), Some(5.4), 86)
        .expect("window stats");
    stats.percentile_level = Some(99.0);
    let signal = detect_contract_whale_signal(&stats).expect("absorption signal");

    assert_eq!(
        signal.signal_type,
        ContractWhaleSignalType::DownsideAbsorption
    );
    assert_eq!(
        signal.price_response_type,
        ContractWhalePriceResponseType::DownsideAbsorption
    );
    assert_eq!(signal.severity, ContractWhaleSeverity::High);
    assert_eq!(
        signal.classification_v2.structure_interpretation,
        ContractWhaleStructureInterpretation::ActiveSellPressure
    );
    assert!(signal.classification_v2.semantic_mismatch);
    assert!(signal.final_result.contains("主动卖压"));
}

#[test]
fn detector_marks_suppression_as_high_when_buyers_fail_to_move_price() {
    let now = 1_700_000_015_000;
    let trades = vec![
        normalize_binance_agg_trade(now - 1_000, 70_000.0, 1_500.0, false).unwrap(),
        normalize_okx_swap_trade(now - 1_000, 70_000.0, 60_000.0, 0.01, "buy").unwrap(),
        normalize_binance_agg_trade(now - 1_000, 70_000.0, 150.0, true).unwrap(),
    ];
    let buckets = aggregate_1s_buckets(&trades);
    let mut stats = rolling_window_stats(&buckets, "BTC", 15, now, Some(0.03), Some(5.4), 86)
        .expect("window stats");
    stats.percentile_level = Some(99.0);
    let signal = detect_contract_whale_signal(&stats).expect("suppression signal");

    assert_eq!(
        signal.signal_type,
        ContractWhaleSignalType::UpsideSuppression
    );
    assert_eq!(
        signal.price_response_type,
        ContractWhalePriceResponseType::UpsideResistance
    );
    assert_eq!(signal.severity, ContractWhaleSeverity::High);
    assert_eq!(
        signal.classification_v2.structure_interpretation,
        ContractWhaleStructureInterpretation::ActiveBuyPressure
    );
    assert!(signal.classification_v2.semantic_mismatch);
    assert!(signal.final_result.contains("主动买压"));
}

#[test]
fn detector_marks_liquidation_suspected_and_reduces_master_confidence() {
    let now = 1_700_000_015_000;
    let trades = vec![
        normalize_binance_agg_trade(now - 1_000, 70_000.0, 2_800.0, true).unwrap(),
        normalize_okx_swap_trade(now - 1_000, 70_000.0, 180_000.0, 0.01, "sell").unwrap(),
        normalize_binance_agg_trade(now - 1_000, 70_000.0, 120.0, false).unwrap(),
    ];
    let buckets = aggregate_1s_buckets(&trades);
    let mut stats = rolling_window_stats(&buckets, "BTC", 15, now, Some(-0.31), Some(8.5), 92)
        .expect("window stats");
    stats.percentile_level = Some(99.5);
    stats.price_reversal_ratio = Some(0.60);
    stats.liquidation_context = ContractWhaleLiquidationContext {
        long_liq_btc: 1_200.0,
        short_liq_btc: 0.0,
        total_liq_btc: 1_200.0,
        liq_notional_usd: 84_000_000.0,
        liq_to_volume_ratio: Some(0.35),
    };
    stats.liquidation_driven = true;
    stats.market_context = ContractWhaleMarketContext {
        context_expected: true,
        ct_val_available: true,
        evidence_degraded: false,
        evidence_reason: None,
        oi_available: true,
        oi_reason: None,
        funding_available: true,
        funding_reason: None,
        oi_change_1m_btc: Some(-400.0),
        oi_change_5m_btc: Some(-1_100.0),
        oi_change_pct: Some(-1.8),
        oi_bias: Some("falling".to_string()),
        funding_rate: Some(0.00022),
        funding_bias: Some("long".to_string()),
    };
    let signal = detect_contract_whale_signal(&stats).expect("liquidation-aware signal");

    assert!(signal.liquidation_suspected);
    assert_eq!(signal.classification_v2.evidence.liquidation_status, "live");
    assert_eq!(signal.classification_v2.evidence.liquidation_reason, None);
    assert_eq!(signal.liquidation_long_btc, 1_200.0);
    assert_eq!(
        signal.liquidation_force.primary_driver,
        "liquidation_cascade"
    );
    assert_eq!(
        signal.liquidation_force.active_zone,
        "long_liquidation_zone"
    );
    assert_eq!(signal.market_driver.primary_driver, "derivatives_pressure");
    assert_eq!(
        signal.market_driver.market_state,
        "liquidation_cascade_regime"
    );
    assert!(signal.market_driver.derivatives_pressure_pct > signal.market_driver.whale_intent_pct);
    assert!(signal.market_driver.interpretation.contains("强制流"));
    assert!(signal.liquidation_force.long_liquidation_pressure >= 60);
    assert!(signal.liquidation_force.flow_attribution.liquidation_pct > 0.40);
    assert_eq!(signal.oi_bias.as_deref(), Some("falling"));
    assert_eq!(signal.funding_bias.as_deref(), Some("long"));
    assert!(signal.score < 90);
    assert!(signal.final_result.contains("强平"));
}

#[test]
fn classification_labels_shape_only_liquidation_as_inferred_evidence() {
    let now = 1_700_000_015_000;
    let trades = vec![
        normalize_binance_agg_trade(now - 1_000, 70_000.0, 2_800.0, true).unwrap(),
        normalize_bitfinex_trade(now - 1_000, 70_000.0, -400.0).unwrap(),
    ];
    let buckets = aggregate_1s_buckets(&trades);
    let mut stats = rolling_window_stats(&buckets, "BTC", 15, now, Some(-0.31), Some(8.5), 92)
        .expect("window stats");
    stats.liquidation_driven = true;
    stats.liquidation_context = ContractWhaleLiquidationContext::default();

    let classification = classify_contract_whale_signal_v2(
        &stats,
        ContractWhaleSignalType::AggressiveSell,
        ContractWhalePriceResponseType::TrendFollowUp,
        false,
        &ContractWhaleRuntimeConfig::default(),
    );

    assert_eq!(classification.evidence.liquidation_status, "inferred");
    assert_eq!(
        classification.evidence.liquidation_reason.as_deref(),
        Some("price_volume_shape_only")
    );
}

#[test]
fn detector_classifies_giant_sell_without_price_drop_as_absorption() {
    let now = 1_700_000_060_000;
    let trades = vec![
        normalize_binance_agg_trade(now - 1_000, 70_000.0, 4_000.0, true).unwrap(),
        normalize_okx_swap_trade(now - 1_000, 70_000.0, 250_000.0, 0.01, "sell").unwrap(),
        normalize_bitfinex_trade(now - 1_000, 70_000.0, -700.0).unwrap(),
        normalize_binance_agg_trade(now - 1_000, 70_000.0, 600.0, false).unwrap(),
    ];
    let buckets = aggregate_1s_buckets(&trades);
    let mut stats = rolling_window_stats(&buckets, "BTC", 60, now, Some(-0.03), Some(8.0), 90)
        .expect("window stats");
    stats.percentile_level = Some(99.5);
    let signal = detect_contract_whale_signal(&stats).expect("signal");

    assert_eq!(
        signal.signal_type,
        ContractWhaleSignalType::DownsideAbsorption
    );
    assert_eq!(signal.severity, ContractWhaleSeverity::Critical);
    assert!(signal.final_result.contains("承接"));
}

#[test]
fn discord_preview_exposes_only_final_alert_fields() {
    let now = 1_700_000_015_000;
    let trades = vec![
        normalize_binance_agg_trade(now - 1_000, 70_000.0, 3_000.0, false).unwrap(),
        normalize_okx_swap_trade(now - 1_000, 70_000.0, 200_000.0, 0.01, "buy").unwrap(),
    ];
    let buckets = aggregate_1s_buckets(&trades);
    let stats = rolling_window_stats(&buckets, "BTC", 15, now, Some(0.31), Some(10.2), 94)
        .expect("window stats");
    let signal = detect_contract_whale_signal(&stats).expect("signal");
    let preview = build_contract_whale_discord_preview(&signal);
    let text = preview.to_string();

    assert!(text.contains("contract_whale_flow"));
    assert!(!text.contains("rawPayload"));
    assert!(!text.contains("webhook"));
    assert!(!text.contains("token"));
    assert!(!text.contains("evidence"));
    assert!(!text.contains("markout"));
}

#[test]
fn classification_uses_recent_1s_vwap_micro_volatility_for_price_thresholds() {
    let now = 1_700_000_060_000;
    let trades = (0..=60)
        .map(|second| {
            let price = 70_000.0 * 1.01_f64.powi(second);
            normalize_binance_agg_trade(now - (60 - second) as i64 * 1_000, price, 25.0, false)
                .expect("trade")
        })
        .collect::<Vec<_>>();
    let buckets = aggregate_1s_buckets(&trades);
    let stats = rolling_window_stats(&buckets, "BTC", 60, now, Some(0.12), Some(6.0), 90)
        .expect("window stats");

    let classification = classify_contract_whale_signal_v2(
        &stats,
        ContractWhaleSignalType::AggressiveBuy,
        ContractWhalePriceResponseType::TrendFollowUp,
        false,
        &ContractWhaleRuntimeConfig::default(),
    );

    assert_eq!(
        classification.dynamic_thresholds.volatility_source,
        "flow_1s_vwap"
    );
    assert!(classification.dynamic_thresholds.follow_pct > 0.12);
}

fn outcome_test_signal(
    now: i64,
    direction: ContractWhaleActiveFlowDirection,
) -> btc_toxic_flow_monitor_rs::contract_whale_monitor::types::ContractWhaleSignal {
    let detection_trades = vec![
        normalize_binance_agg_trade(now, 70_000.0, 2_000.0, false).expect("buy trade"),
        normalize_bitfinex_trade(now, 70_000.0, 500.0).expect("confirming buy trade"),
    ];
    let mut stats = rolling_window_stats(
        &aggregate_1s_buckets(&detection_trades),
        "BTC",
        15,
        now,
        Some(0.30),
        Some(8.0),
        90,
    )
    .expect("stats");
    stats.percentile_level = Some(99.5);
    let mut signal = detect_contract_whale_signal(&stats).expect("signal");
    signal.order_price_usd = Some(100.0);
    signal.classification_v2.flow_direction = direction;
    signal
}

#[test]
fn outcome_calibration_measures_direction_free_volatility_for_unknown_direction() {
    let now = 1_700_000_000_000;
    let signal = outcome_test_signal(now, ContractWhaleActiveFlowDirection::Unknown);
    let outcome_trades = vec![
        normalize_binance_agg_trade(now, 100.0, 1.0, false).expect("entry price"),
        normalize_binance_agg_trade(now + 30_000, 103.0, 1.0, false).expect("30s price"),
        normalize_binance_agg_trade(now + 60_000, 97.0, 1.0, false).expect("60s price"),
        normalize_binance_agg_trade(now + 120_000, 104.0, 1.0, false).expect("2m price"),
        normalize_binance_agg_trade(now + 180_000, 96.0, 1.0, false).expect("3m price"),
        normalize_binance_agg_trade(now + 300_000, 102.0, 1.0, false).expect("5m price"),
    ];

    let outcome = evaluate_contract_whale_signal_outcome(
        &signal,
        &aggregate_1s_buckets(&outcome_trades),
        now + 300_000,
    )
    .expect("direction-free outcome");

    assert!(
        (outcome
            .absolute_return_30s_bps
            .expect("30s absolute return")
            - 300.0)
            .abs()
            < 1e-6
    );
    assert!((outcome.absolute_return_2m_bps.expect("2m absolute return") - 400.0).abs() < 1e-6);
    assert!((outcome.absolute_return_5m_bps.expect("5m absolute return") - 200.0).abs() < 1e-6);
    assert!(
        outcome
            .realized_volatility_5m_bps
            .expect("realized volatility")
            > 1_000.0
    );
    assert!(
        (outcome
            .max_absolute_excursion_5m_bps
            .expect("max absolute excursion")
            - 400.0)
            .abs()
            < 1e-6
    );
    assert_eq!(outcome.price_sample_count_5m, Some(6));
    assert_eq!(outcome.markout_30s_bps, None);
    assert_eq!(outcome.markout_2m_bps, None);
    assert_eq!(outcome.markout_5m_bps, None);
    assert_eq!(outcome.follow_through_5m, None);
    assert_eq!(outcome.setup_outcome.as_deref(), Some("unclear"));
    assert_eq!(outcome.liquidity_recovered_5m, None);
    assert_eq!(outcome.liquidity_recovery_ms, None);
    assert_eq!(
        outcome.liquidity_recovery_reason.as_deref(),
        Some("historical_l2_unavailable")
    );
    assert_eq!(outcome.outcome_version, "v2_volatility_shadow");
}

#[test]
fn outcome_calibration_keeps_directional_reversal_secondary_to_volatility() {
    let now = 1_700_000_000_000;
    let signal = outcome_test_signal(now, ContractWhaleActiveFlowDirection::BuyDominant);

    let markout_trades = vec![
        normalize_binance_agg_trade(now, 100.0, 1.0, false).expect("entry price"),
        normalize_binance_agg_trade(now + 30_000, 106.0, 1.0, false).expect("30s price"),
        normalize_binance_agg_trade(now + 120_000, 94.0, 1.0, false).expect("2m price"),
        normalize_binance_agg_trade(now + 300_000, 98.0, 1.0, false).expect("5m price"),
    ];
    let outcome = evaluate_contract_whale_signal_outcome(
        &signal,
        &aggregate_1s_buckets(&markout_trades),
        now + 300_000,
    )
    .expect("outcome");

    assert!(outcome.markout_30s_bps.expect("30s markout") > 0.0);
    assert!(outcome.markout_5m_bps.expect("5m markout") < 0.0);
    assert_eq!(outcome.follow_through_5m, Some(false));
    assert_eq!(outcome.setup_outcome.as_deref(), Some("reversal"));
    assert!((outcome.absolute_return_5m_bps.expect("5m absolute return") - 200.0).abs() < 1e-6);
    assert!(
        outcome
            .realized_volatility_5m_bps
            .expect("realized volatility")
            > 1_000.0
    );
    assert!(
        (outcome
            .max_absolute_excursion_5m_bps
            .expect("max absolute excursion")
            - 600.0)
            .abs()
            < 1e-6
    );
}

#[test]
fn outcome_calibration_flat_path_has_zero_realized_volatility() {
    let now = 1_700_000_000_000;
    let signal = outcome_test_signal(now, ContractWhaleActiveFlowDirection::Unknown);
    let flat_trades = vec![
        normalize_binance_agg_trade(now, 100.0, 1.0, false).expect("entry price"),
        normalize_binance_agg_trade(now + 30_000, 100.0, 1.0, false).expect("30s price"),
        normalize_binance_agg_trade(now + 120_000, 100.0, 1.0, false).expect("2m price"),
        normalize_binance_agg_trade(now + 300_000, 100.0, 1.0, false).expect("5m price"),
    ];

    let outcome = evaluate_contract_whale_signal_outcome(
        &signal,
        &aggregate_1s_buckets(&flat_trades),
        now + 300_000,
    )
    .expect("flat outcome");

    assert_eq!(outcome.realized_volatility_5m_bps, Some(0.0));
    assert_eq!(outcome.max_absolute_excursion_5m_bps, Some(0.0));
    assert_eq!(outcome.price_sample_count_5m, Some(4));
}

#[test]
fn outcome_calibration_requires_fresh_horizon_prices_and_hides_future_values() {
    let now = 1_700_000_000_000;
    let signal = outcome_test_signal(now, ContractWhaleActiveFlowDirection::Unknown);
    let sparse_trades = vec![
        normalize_binance_agg_trade(now, 100.0, 1.0, false).expect("entry price"),
        normalize_binance_agg_trade(now + 120_000, 102.0, 1.0, false).expect("2m price"),
        normalize_binance_agg_trade(now + 300_000, 104.0, 1.0, false).expect("5m price"),
    ];
    let buckets = aggregate_1s_buckets(&sparse_trades);

    assert!(evaluate_contract_whale_signal_outcome(&signal, &buckets, now + 29_999).is_none());

    let at_30s = evaluate_contract_whale_signal_outcome(&signal, &buckets, now + 30_000)
        .expect("30s evaluation shell");
    assert_eq!(at_30s.absolute_return_30s_bps, None);
    assert_eq!(at_30s.markout_30s_bps, None);
    assert_eq!(at_30s.absolute_return_2m_bps, None);
    assert_eq!(at_30s.absolute_return_5m_bps, None);
    assert_eq!(at_30s.realized_volatility_5m_bps, None);

    let at_2m = evaluate_contract_whale_signal_outcome(&signal, &buckets, now + 120_000)
        .expect("2m evaluation shell");
    assert_eq!(at_2m.absolute_return_30s_bps, None);
    assert!((at_2m.absolute_return_2m_bps.expect("fresh 2m price") - 200.0).abs() < 1e-6);
    assert_eq!(at_2m.absolute_return_5m_bps, None);
}

#[test]
fn detection_time_builders_do_not_reference_future_outcome_fields() {
    let detection_sources = [
        include_str!("../src/contract_whale_monitor/scoring.rs"),
        include_str!("../src/contract_whale_monitor/detector.rs"),
        include_str!("../src/contract_whale_monitor/classification.rs"),
    ];
    let future_outcome_fields = [
        "absolute_return_30s_bps",
        "absolute_return_2m_bps",
        "absolute_return_5m_bps",
        "realized_volatility_5m_bps",
        "max_absolute_excursion_5m_bps",
        "price_sample_count_5m",
        "liquidity_recovered_5m",
        "liquidity_recovery_ms",
        "liquidity_recovery_reason",
        "setup_outcome",
    ];

    for source in detection_sources {
        for field in future_outcome_fields {
            assert!(
                !source.contains(field),
                "detection-time source must not consume future outcome field {field}"
            );
        }
    }
}
