use std::collections::BTreeMap;

use btc_toxic_flow_monitor_rs::{
    config::thresholds::ToxicVolumeParams,
    toxicity::toxic_volume_engine::{detect_direction, map_sweep_window, ToxicVolumeEngine},
    types::{
        flow::{empty_venue_breakdown, DataQuality, FlowWindow, VenueFlowBreakdown},
        liquidation::empty_liquidation_state,
        market::Venue,
        markout::{
            DirectionalMarkoutStats, MarkoutQuality, MarkoutState, MarkoutWindowSummary,
            VenueMarkoutBreakdown,
        },
        sweep::{
            empty_venue_sweep_breakdown, LiquidityThinnessResult, SweepDirection, SweepQuality,
            SweepResult, SweepState,
        },
        toxic::ToxicDirection,
        vpin::{VpinDirection, VpinMetrics, VpinState},
    },
};

#[test]
fn ordinary_two_sided_flow_neutral_does_not_trigger() {
    let engine = engine();
    let result = engine.compute_window(
        &flow_window(1000, 600.0, 600.0, 1000.0, 1000.0),
        &markout_state(0.0, 0.0, 0.0, 0.0),
        &sweep_state(1000, SweepDirection::None, None, None),
        &empty_vpin_state(),
        &empty_liquidation_state(1760000000000),
    );

    assert_eq!(result.direction, ToxicDirection::Neutral);
    assert_eq!(result.toxic_volume_btc, 0.0);
    assert!(!result.alert_triggered);
}

#[test]
fn large_buy_flow_without_support_stays_below_threshold() {
    let engine = engine();
    let result = engine.compute_window(
        &flow_window(5000, 1300.0, 300.0, 1000.0, 1005.0),
        &markout_state(0.0, 0.0, 0.0, 0.0),
        &sweep_state(5000, SweepDirection::None, None, None),
        &empty_vpin_state(),
        &empty_liquidation_state(1760000000000),
    );

    assert_eq!(result.direction, ToxicDirection::Buy);
    assert!(result.toxic_volume_btc < 1000.0);
    assert!(!result.alert_triggered);
    assert!(result
        .reason_codes
        .contains(&"large_aggressive_flow".to_string()));
    assert!(result
        .reason_codes
        .contains(&"insufficient_toxic_ratio".to_string()));
}

#[test]
fn buy_flow_with_markout_and_sweep_triggers_alert() {
    let engine = engine();
    let result = engine.compute_window(
        &flow_window(5000, 1300.0, 0.0, 1000.0, 1008.0),
        &markout_state(2.5, 4.5, 0.0, 0.0),
        &sweep_state(
            5000,
            SweepDirection::Buy,
            Some(liquidity(true, false, true)),
            Some(Venue::Binance),
        ),
        &empty_vpin_state(),
        &empty_liquidation_state(1760000000000),
    );

    assert_eq!(result.direction, ToxicDirection::Buy);
    assert!(result.markout_confirmed);
    assert!(result.sweep_detected);
    assert!(result.liquidity_thin);
    assert!(result.cross_venue_confirmed);
    assert!(result.alert_triggered);
    assert!(result.toxic_volume_btc >= 1000.0);
    assert!(result
        .reason_codes
        .contains(&"markout_1s_confirmed".to_string()));
    assert!(result
        .reason_codes
        .contains(&"markout_5s_confirmed".to_string()));
    assert!(result.reason_codes.contains(&"sweep_detected".to_string()));
    assert!(result.reason_codes.contains(&"liquidity_thin".to_string()));
    assert!(result
        .reason_codes
        .contains(&"cross_venue_confirmed".to_string()));
    assert!(result
        .reason_codes
        .contains(&"leader_venue_diffusion".to_string()));
    assert!(result
        .reason_codes
        .contains(&"threshold_crossed".to_string()));
}

#[test]
fn sell_flow_with_markout_and_sweep_triggers_alert() {
    let engine = engine();
    let result = engine.compute_window(
        &flow_window(5000, 100.0, 1100.0, 1000.0, 995.0),
        &markout_state(0.0, 0.0, 2.0, 4.0),
        &sweep_state(
            5000,
            SweepDirection::Sell,
            Some(liquidity(false, true, true)),
            Some(Venue::Bybit),
        ),
        &empty_vpin_state(),
        &empty_liquidation_state(1760000000000),
    );

    assert_eq!(result.direction, ToxicDirection::Sell);
    assert!(result.markout_confirmed);
    assert!(result.sweep_detected);
    assert!(result.liquidity_thin);
    assert!(result.alert_triggered);
    assert!(result.toxic_volume_btc >= 1000.0);
}

#[test]
fn multi_venue_confirmed_flow_can_cross_threshold() {
    let engine = engine();
    let result = engine.compute_window(
        &flow_window(15000, 1600.0, 200.0, 1000.0, 1007.0),
        &markout_state(1.5, 3.5, 0.0, 0.0),
        &sweep_state(
            15000,
            SweepDirection::Buy,
            Some(liquidity(true, false, false)),
            Some(Venue::Binance),
        ),
        &empty_vpin_state(),
        &empty_liquidation_state(1760000000000),
    );

    assert!(result.cross_venue_confirmed);
    assert!(result.toxic_volume_btc > 1000.0);
    assert!(result.alert_triggered);
}

#[test]
fn mismatch_sweep_direction_does_not_add_sweep_weight() {
    let engine = engine();
    let result = engine.compute_window(
        &flow_window(5000, 1300.0, 0.0, 1000.0, 1008.0),
        &markout_state(2.5, 4.5, 0.0, 0.0),
        &sweep_state(
            5000,
            SweepDirection::Sell,
            Some(liquidity(false, true, true)),
            Some(Venue::Bybit),
        ),
        &empty_vpin_state(),
        &empty_liquidation_state(1760000000000),
    );

    assert_eq!(result.direction, ToxicDirection::Buy);
    assert!(!result.sweep_detected);
    assert!(!result.reason_codes.contains(&"sweep_detected".to_string()));
}

#[test]
fn buy_toxic_uses_ask_thin_not_bid_thin() {
    let engine = engine();
    let buy_result = engine.compute_window(
        &flow_window(5000, 1300.0, 0.0, 1000.0, 1008.0),
        &markout_state(2.5, 4.5, 0.0, 0.0),
        &sweep_state(
            5000,
            SweepDirection::Buy,
            Some(liquidity(true, false, false)),
            Some(Venue::Binance),
        ),
        &empty_vpin_state(),
        &empty_liquidation_state(1760000000000),
    );
    let bid_thin_result = engine.compute_window(
        &flow_window(5000, 1300.0, 0.0, 1000.0, 1008.0),
        &markout_state(2.5, 4.5, 0.0, 0.0),
        &sweep_state(
            5000,
            SweepDirection::Buy,
            Some(liquidity(false, true, false)),
            Some(Venue::Binance),
        ),
        &empty_vpin_state(),
        &empty_liquidation_state(1760000000000),
    );

    assert!(buy_result.liquidity_thin);
    assert!(!bid_thin_result.liquidity_thin);
}

#[test]
fn sell_toxic_uses_bid_thin_not_ask_thin() {
    let engine = engine();
    let sell_result = engine.compute_window(
        &flow_window(5000, 0.0, 1300.0, 1000.0, 995.0),
        &markout_state(0.0, 0.0, 2.5, 4.5),
        &sweep_state(
            5000,
            SweepDirection::Sell,
            Some(liquidity(false, true, false)),
            Some(Venue::Bybit),
        ),
        &empty_vpin_state(),
        &empty_liquidation_state(1760000000000),
    );
    let ask_thin_result = engine.compute_window(
        &flow_window(5000, 0.0, 1300.0, 1000.0, 995.0),
        &markout_state(0.0, 0.0, 2.5, 4.5),
        &sweep_state(
            5000,
            SweepDirection::Sell,
            Some(liquidity(true, false, false)),
            Some(Venue::Bybit),
        ),
        &empty_vpin_state(),
        &empty_liquidation_state(1760000000000),
    );

    assert!(sell_result.liquidity_thin);
    assert!(!ask_thin_result.liquidity_thin);
}

#[test]
fn leader_venue_is_selected_by_directional_volume() {
    let engine = engine();
    let mut flow = flow_window(5000, 100.0, 0.0, 1000.0, 1006.0);
    flow.venue_breakdown = leader_breakdown();
    let result = engine.compute_window(
        &flow,
        &markout_state(2.0, 4.0, 0.0, 0.0),
        &sweep_state(
            5000,
            SweepDirection::Buy,
            Some(liquidity(true, false, false)),
            Some(Venue::Bybit),
        ),
        &empty_vpin_state(),
        &empty_liquidation_state(1760000000000),
    );

    assert_eq!(result.leader_venue, Some(Venue::Bybit));
}

#[test]
fn detect_direction_helper_behaves() {
    assert_eq!(detect_direction(10.0), ToxicDirection::Buy);
    assert_eq!(detect_direction(-10.0), ToxicDirection::Sell);
    assert_eq!(detect_direction(0.0), ToxicDirection::Neutral);
    assert_eq!(map_sweep_window(60000), 15000);
}

fn engine() -> ToxicVolumeEngine {
    ToxicVolumeEngine::new(ToxicVolumeParams::default())
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
    let mut venue_breakdown = empty_venue_breakdown();
    venue_breakdown.insert(
        "binance".to_string(),
        venue_breakdown_for(aggressive_buy_btc * 0.6, aggressive_sell_btc * 0.2, 6),
    );
    venue_breakdown.insert(
        "bybit".to_string(),
        venue_breakdown_for(aggressive_buy_btc * 0.3, aggressive_sell_btc * 0.5, 3),
    );
    venue_breakdown.insert(
        "okx".to_string(),
        venue_breakdown_for(aggressive_buy_btc * 0.1, aggressive_sell_btc * 0.3, 1),
    );

    FlowWindow {
        symbol: "BTC-PERP".to_string(),
        window_ms,
        now_ts: 1760000000000,
        aggressive_buy_btc,
        aggressive_sell_btc,
        aggressive_buy_usd: aggressive_buy_btc * 100.0,
        aggressive_sell_usd: aggressive_sell_btc * 100.0,
        net_aggressive_btc,
        abs_aggressive_btc,
        trade_count: 10,
        buy_trade_count: if aggressive_buy_btc > 0.0 { 6 } else { 0 },
        sell_trade_count: if aggressive_sell_btc > 0.0 { 4 } else { 0 },
        avg_trade_size_btc: abs_aggressive_btc / 10.0,
        max_trade_size_btc: abs_aggressive_btc / 4.0,
        venue_breakdown,
        mid_start: Some(mid_start),
        mid_end: Some(mid_end),
        price_move_bps: Some(((mid_end - mid_start) / mid_start) * 10_000.0),
        spread_bps_median: Some(2.0),
        imbalance_10bps_median: Some(0.0),
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
    }
}

fn venue_breakdown_for(buy: f64, sell: f64, trade_count: u64) -> VenueFlowBreakdown {
    VenueFlowBreakdown {
        aggressive_buy_btc: buy,
        aggressive_sell_btc: sell,
        aggressive_buy_usd: buy * 100.0,
        aggressive_sell_usd: sell * 100.0,
        net_aggressive_btc: buy - sell,
        abs_aggressive_btc: buy + sell,
        trade_count,
        buy_trade_count: if buy > 0.0 { trade_count } else { 0 },
        sell_trade_count: if sell > 0.0 { trade_count } else { 0 },
        last_trade_ts: Some(1),
    }
}

fn markout_state(buy_1s: f64, buy_5s: f64, sell_1s: f64, sell_5s: f64) -> MarkoutState {
    let mut summaries = BTreeMap::new();
    summaries.insert(
        "1000".to_string(),
        MarkoutWindowSummary {
            horizon_ms: 1000,
            buy: stats(Some(buy_1s)),
            sell: stats(Some(sell_1s)),
            venue_breakdown: markout_venue_breakdown(buy_1s, sell_1s),
        },
    );
    summaries.insert(
        "5000".to_string(),
        MarkoutWindowSummary {
            horizon_ms: 5000,
            buy: stats(Some(buy_5s)),
            sell: stats(Some(sell_5s)),
            venue_breakdown: markout_venue_breakdown(buy_5s, sell_5s),
        },
    );
    summaries.insert(
        "15000".to_string(),
        MarkoutWindowSummary {
            horizon_ms: 15000,
            buy: stats(None),
            sell: stats(None),
            venue_breakdown: markout_venue_breakdown(0.0, 0.0),
        },
    );

    MarkoutState {
        symbol: "BTC-PERP".to_string(),
        updated_at: 1760000000000,
        horizons_ms: vec![1000, 5000, 15000],
        summaries,
        quality: MarkoutQuality {
            pending_samples: 0,
            resolved_samples: 10,
            expired_samples: 0,
            has_price_index: true,
        },
    }
}

fn stats(markout: Option<f64>) -> DirectionalMarkoutStats {
    DirectionalMarkoutStats {
        count: 1,
        volume_btc: 100.0,
        volume_usd: 10_000.0,
        avg_markout_bps: markout,
        volume_weighted_markout_bps: markout,
        positive_count: if markout.unwrap_or(0.0) > 0.0 { 1 } else { 0 },
        negative_count: if markout.unwrap_or(0.0) < 0.0 { 1 } else { 0 },
        positive_volume_btc: if markout.unwrap_or(0.0) > 0.0 {
            100.0
        } else {
            0.0
        },
        negative_volume_btc: if markout.unwrap_or(0.0) < 0.0 {
            100.0
        } else {
            0.0
        },
    }
}

fn markout_venue_breakdown(buy: f64, sell: f64) -> BTreeMap<String, VenueMarkoutBreakdown> {
    let mut map = BTreeMap::new();
    for venue in Venue::ALL {
        map.insert(
            venue.as_key().to_string(),
            VenueMarkoutBreakdown {
                buy: stats(Some(buy)),
                sell: stats(Some(sell)),
            },
        );
    }
    map
}

fn sweep_state(
    window_ms: u64,
    direction: SweepDirection,
    liquidity: Option<LiquidityThinnessResult>,
    leader: Option<Venue>,
) -> SweepState {
    let mut results = BTreeMap::new();
    for candidate in [1000_u64, 5000, 15000] {
        results.insert(
            candidate.to_string(),
            SweepResult {
                symbol: "BTC-PERP".to_string(),
                window_ms: candidate,
                direction: if candidate == window_ms {
                    direction
                } else {
                    SweepDirection::None
                },
                sweep_detected: candidate == window_ms && direction != SweepDirection::None,
                swept_volume_btc: if candidate == window_ms { 100.0 } else { 0.0 },
                swept_volume_usd: if candidate == window_ms {
                    10_000.0
                } else {
                    0.0
                },
                aggressive_buy_btc: if direction == SweepDirection::Buy {
                    100.0
                } else {
                    0.0
                },
                aggressive_sell_btc: if direction == SweepDirection::Sell {
                    100.0
                } else {
                    0.0
                },
                net_aggressive_btc: if direction == SweepDirection::Buy {
                    100.0
                } else if direction == SweepDirection::Sell {
                    -100.0
                } else {
                    0.0
                },
                trade_count: 10,
                same_direction_trade_count: 6,
                price_start: Some(1000.0),
                price_end: Some(1005.0),
                price_impact_bps: Some(if direction == SweepDirection::Sell {
                    -5.0
                } else {
                    5.0
                }),
                leader_venue: leader,
                venue_breakdown: empty_venue_sweep_breakdown(),
                liquidity: if candidate == window_ms {
                    liquidity.clone()
                } else {
                    None
                },
                reason_codes: Vec::new(),
            },
        );
    }
    SweepState {
        symbol: "BTC-PERP".to_string(),
        updated_at: 1760000000000,
        windows_ms: vec![1000, 5000, 15000],
        results,
        quality: SweepQuality {
            has_trades: true,
            has_books: true,
            active_venues: vec![Venue::Binance, Venue::Bybit],
            stale_venues: vec![],
        },
    }
}

fn liquidity(ask_thin: bool, bid_thin: bool, spread_widened: bool) -> LiquidityThinnessResult {
    LiquidityThinnessResult {
        symbol: "BTC-PERP".to_string(),
        window_ms: 5000,
        bid_depth_start_btc: Some(1000.0),
        bid_depth_end_btc: Some(if bid_thin { 600.0 } else { 1000.0 }),
        ask_depth_start_btc: Some(1000.0),
        ask_depth_end_btc: Some(if ask_thin { 600.0 } else { 1000.0 }),
        bid_depth_drop_ratio: Some(if bid_thin { 0.4 } else { 0.0 }),
        ask_depth_drop_ratio: Some(if ask_thin { 0.4 } else { 0.0 }),
        spread_start_bps: Some(2.0),
        spread_end_bps: Some(if spread_widened { 3.0 } else { 2.0 }),
        spread_widen_ratio: Some(if spread_widened { 0.5 } else { 0.0 }),
        bid_thin,
        ask_thin,
        spread_widened,
        reason_codes: Vec::new(),
    }
}

fn leader_breakdown() -> BTreeMap<String, VenueFlowBreakdown> {
    let mut map = empty_venue_breakdown();
    map.insert("binance".to_string(), venue_breakdown_for(10.0, 0.0, 2));
    map.insert("bybit".to_string(), venue_breakdown_for(80.0, 0.0, 4));
    map.insert("okx".to_string(), venue_breakdown_for(5.0, 0.0, 1));
    map
}

fn empty_vpin_state() -> VpinState {
    VpinState {
        symbol: "BTC-PERP".to_string(),
        updated_at: 1760000000000,
        metrics: VpinMetrics {
            symbol: "BTC-PERP".to_string(),
            updated_at: 1760000000000,
            enabled: true,
            bucket_size_btc: 100.0,
            lookback_buckets: 50,
            min_buckets: 10,
            completed_bucket_count: 0,
            active_bucket_progress_btc: 0.0,
            active_bucket_progress_ratio: 0.0,
            latest_bucket: None,
            vpin: None,
            vpin_zscore: None,
            vpin_percentile: None,
            latest_bucket_imbalance_ratio: None,
            avg_bucket_imbalance_ratio: None,
            vpin_high: false,
            vpin_extreme: false,
            vpin_spike: false,
            dominant_direction: VpinDirection::Balanced,
            reason_codes: Vec::new(),
        },
        recent_buckets: Vec::new(),
    }
}
