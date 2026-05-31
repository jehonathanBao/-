use btc_toxic_flow_monitor_rs::{
    toxicity::sweep_detector::{SweepDetector, SweepInput, SweepParams},
    types::{
        flow::{empty_venue_breakdown, DataQuality, FlowWindow},
        market::{AggressorSide, NormalizedTrade, Venue},
        sweep::{LiquidityThinnessResult, SweepDirection},
    },
};

#[test]
fn buy_flow_with_price_impact_and_ask_thin_detects_buy_sweep() {
    let result = detector().detect(SweepInput {
        symbol: "BTC-PERP".to_string(),
        window_ms: 5000,
        trades: vec![
            trade(Venue::Binance, 0, 100.0, 300.0, AggressorSide::Buy),
            trade(Venue::Binance, 1, 101.0, 200.0, AggressorSide::Buy),
            trade(Venue::Bybit, 2, 102.0, 100.0, AggressorSide::Buy),
        ],
        flow_window: flow_window(5000, 600.0, 0.0, 100.0, 101.0),
        liquidity: Some(liquidity(true, false, false)),
    });

    assert!(result.sweep_detected);
    assert_eq!(result.direction, SweepDirection::Buy);
    assert_eq!(result.swept_volume_btc, 600.0);
    assert_eq!(result.same_direction_trade_count, 3);
    assert_eq!(result.leader_venue, Some(Venue::Binance));
}

#[test]
fn sell_flow_with_price_impact_and_bid_thin_detects_sell_sweep() {
    let result = detector().detect(SweepInput {
        symbol: "BTC-PERP".to_string(),
        window_ms: 5000,
        trades: vec![
            trade(Venue::Bybit, 0, 100.0, 300.0, AggressorSide::Sell),
            trade(Venue::Bybit, 1, 99.0, 200.0, AggressorSide::Sell),
            trade(Venue::Okx, 2, 98.0, 100.0, AggressorSide::Sell),
        ],
        flow_window: flow_window(5000, 0.0, 600.0, 100.0, 99.0),
        liquidity: Some(liquidity(false, true, false)),
    });

    assert!(result.sweep_detected);
    assert_eq!(result.direction, SweepDirection::Sell);
    assert_eq!(result.swept_volume_btc, 600.0);
    assert_eq!(result.same_direction_trade_count, 3);
    assert_eq!(result.leader_venue, Some(Venue::Bybit));
}

#[test]
fn random_two_sided_volume_does_not_detect_sweep() {
    let result = detector().detect(SweepInput {
        symbol: "BTC-PERP".to_string(),
        window_ms: 5000,
        trades: vec![
            trade(Venue::Binance, 0, 100.0, 250.0, AggressorSide::Buy),
            trade(Venue::Bybit, 1, 100.0, 250.0, AggressorSide::Sell),
            trade(Venue::Okx, 2, 100.0, 250.0, AggressorSide::Buy),
            trade(Venue::Binance, 3, 100.0, 250.0, AggressorSide::Sell),
        ],
        flow_window: flow_window(5000, 500.0, 500.0, 100.0, 100.0),
        liquidity: Some(liquidity(true, true, true)),
    });

    assert!(!result.sweep_detected);
    assert_eq!(result.direction, SweepDirection::None);
}

#[test]
fn buy_volume_without_price_impact_does_not_detect_sweep() {
    let result = detector().detect(SweepInput {
        symbol: "BTC-PERP".to_string(),
        window_ms: 5000,
        trades: buy_trades(600.0),
        flow_window: flow_window(5000, 600.0, 0.0, 100.0, 100.0),
        liquidity: Some(liquidity(true, false, false)),
    });

    assert!(!result.sweep_detected);
}

#[test]
fn sell_volume_with_price_rise_does_not_detect_sweep() {
    let result = detector().detect(SweepInput {
        symbol: "BTC-PERP".to_string(),
        window_ms: 5000,
        trades: vec![
            trade(Venue::Binance, 0, 100.0, 200.0, AggressorSide::Sell),
            trade(Venue::Bybit, 1, 101.0, 200.0, AggressorSide::Sell),
            trade(Venue::Okx, 2, 102.0, 200.0, AggressorSide::Sell),
        ],
        flow_window: flow_window(5000, 0.0, 600.0, 100.0, 101.0),
        liquidity: Some(liquidity(false, true, false)),
    });

    assert!(!result.sweep_detected);
}

#[test]
fn same_direction_trade_count_below_threshold_does_not_detect_sweep() {
    let result = detector().detect(SweepInput {
        symbol: "BTC-PERP".to_string(),
        window_ms: 5000,
        trades: vec![
            trade(Venue::Binance, 0, 100.0, 300.0, AggressorSide::Buy),
            trade(Venue::Bybit, 1, 101.0, 300.0, AggressorSide::Buy),
        ],
        flow_window: flow_window(5000, 600.0, 0.0, 100.0, 101.0),
        liquidity: Some(liquidity(true, false, false)),
    });

    assert!(!result.sweep_detected);
}

#[test]
fn venue_breakdown_always_contains_all_venues() {
    let result = detector().detect(SweepInput {
        symbol: "BTC-PERP".to_string(),
        window_ms: 5000,
        trades: buy_trades(600.0),
        flow_window: flow_window(5000, 600.0, 0.0, 100.0, 101.0),
        liquidity: Some(liquidity(true, false, false)),
    });

    assert!(result.venue_breakdown.contains_key("binance"));
    assert!(result.venue_breakdown.contains_key("bybit"));
    assert!(result.venue_breakdown.contains_key("okx"));
}

fn detector() -> SweepDetector {
    SweepDetector::new(SweepParams::default())
}

fn buy_trades(total: f64) -> Vec<NormalizedTrade> {
    vec![
        trade(Venue::Binance, 0, 100.0, total / 3.0, AggressorSide::Buy),
        trade(Venue::Bybit, 1, 101.0, total / 3.0, AggressorSide::Buy),
        trade(Venue::Okx, 2, 102.0, total / 3.0, AggressorSide::Buy),
    ]
}

fn liquidity(ask_thin: bool, bid_thin: bool, spread_widened: bool) -> LiquidityThinnessResult {
    LiquidityThinnessResult {
        symbol: "BTC-PERP".to_string(),
        window_ms: 5000,
        ask_thin,
        bid_thin,
        spread_widened,
        ..LiquidityThinnessResult::default()
    }
}

fn flow_window(
    window_ms: u64,
    aggressive_buy_btc: f64,
    aggressive_sell_btc: f64,
    mid_start: f64,
    mid_end: f64,
) -> FlowWindow {
    let net_aggressive_btc = aggressive_buy_btc - aggressive_sell_btc;
    let abs_aggressive_btc = aggressive_buy_btc + aggressive_sell_btc;
    FlowWindow {
        symbol: "BTC-PERP".to_string(),
        window_ms,
        now_ts: window_ms as i64,
        aggressive_buy_btc,
        aggressive_sell_btc,
        aggressive_buy_usd: aggressive_buy_btc * 100.0,
        aggressive_sell_usd: aggressive_sell_btc * 100.0,
        net_aggressive_btc,
        abs_aggressive_btc,
        trade_count: 3,
        buy_trade_count: if aggressive_buy_btc > 0.0 { 3 } else { 0 },
        sell_trade_count: if aggressive_sell_btc > 0.0 { 3 } else { 0 },
        avg_trade_size_btc: abs_aggressive_btc / 3.0,
        max_trade_size_btc: abs_aggressive_btc / 3.0,
        venue_breakdown: empty_venue_breakdown(),
        mid_start: Some(mid_start),
        mid_end: Some(mid_end),
        price_move_bps: Some(((mid_end - mid_start) / mid_start) * 10_000.0),
        spread_bps_median: Some(2.0),
        imbalance_10bps_median: Some(0.0),
        data_quality: DataQuality {
            has_trades: abs_aggressive_btc > 0.0,
            has_books: true,
            active_venues: vec!["binance".to_string()],
            stale_venues: Vec::new(),
        },
    }
}

fn trade(
    venue: Venue,
    ts: i64,
    price: f64,
    size_btc: f64,
    aggressor_side: AggressorSide,
) -> NormalizedTrade {
    NormalizedTrade {
        venue,
        symbol: "BTC-PERP".to_string(),
        ts,
        price,
        size_btc,
        size_usd: price * size_btc,
        aggressor_side,
        trade_id: Some(format!("{venue}:{ts}:{price}:{size_btc}")),
    }
}
