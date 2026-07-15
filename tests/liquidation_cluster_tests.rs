use std::collections::BTreeMap;

use btc_toxic_flow_monitor_rs::{
    config::thresholds::LiquidationClusterParams,
    market_data::price_index::PriceSnapshot,
    toxicity::liquidation_cluster_engine::LiquidationClusterEngine,
    types::{
        flow::{empty_venue_breakdown, DataQuality, FlowState, FlowWindow},
        sweep::{SweepDirection, SweepQuality, SweepResult, SweepState},
        toxic::ToxicDirection,
        vpin::{VpinDirection, VpinMetrics, VpinState},
    },
};

#[test]
fn detects_short_cluster_above_and_builds_pressure() {
    let engine = LiquidationClusterEngine::new(LiquidationClusterParams::default());
    let snapshots = vec![
        snapshot(1_000, 100_060.0),
        snapshot(2_000, 100_062.0),
        snapshot(3_000, 100_061.0),
        snapshot(4_000, 100_000.0),
    ];

    let state = engine.compute(
        4_000,
        &flow_state(500.0, 60_000_000.0, 10_000_000.0),
        &sweep_state(SweepDirection::Buy),
        &vpin_state(true, true, false),
        &snapshots,
    );

    assert_eq!(state.metrics.dominant_direction, ToxicDirection::Buy);
    assert!(state.metrics.nearest_short_liq_cluster_above.is_some());
    assert!(state.metrics.liq_cluster_nearby);
    assert!(state.metrics.liq_hunt_pressure > 0.6);
    assert!(state.metrics.possible_liq_hunt_setup);
}

#[test]
fn requires_enough_touches_for_cluster() {
    let engine = LiquidationClusterEngine::new(LiquidationClusterParams {
        min_touches: 4,
        ..LiquidationClusterParams::default()
    });
    let snapshots = vec![
        snapshot(1_000, 100_060.0),
        snapshot(2_000, 100_062.0),
        snapshot(3_000, 100_000.0),
    ];

    let state = engine.compute(
        3_000,
        &flow_state(500.0, 60_000_000.0, 10_000_000.0),
        &sweep_state(SweepDirection::Buy),
        &vpin_state(false, false, false),
        &snapshots,
    );

    assert!(state.metrics.nearest_short_liq_cluster_above.is_none());
    assert!(!state.metrics.possible_liq_hunt_setup);
}

fn snapshot(ts: i64, index_mid: f64) -> PriceSnapshot {
    PriceSnapshot {
        ts,
        index_mid,
        spread_bps_median: None,
        imbalance_10bps_median: None,
        bid_depth_btc_10bps_median: None,
        ask_depth_btc_10bps_median: None,
    }
}

fn flow_state(net_5s: f64, buy_usd: f64, sell_usd: f64) -> FlowState {
    let mut windows = BTreeMap::new();
    windows.insert(
        "5000".to_string(),
        FlowWindow {
            symbol: "BTC-PERP".to_string(),
            window_ms: 5_000,
            now_ts: 5_000,
            aggressive_buy_btc: 600.0,
            aggressive_sell_btc: 100.0,
            aggressive_buy_usd: buy_usd,
            aggressive_sell_usd: sell_usd,
            net_aggressive_btc: net_5s,
            abs_aggressive_btc: 700.0,
            trade_count: 10,
            buy_trade_count: 8,
            sell_trade_count: 2,
            avg_trade_size_btc: 70.0,
            max_trade_size_btc: 200.0,
            venue_breakdown: empty_venue_breakdown(),
            mid_start: Some(100_000.0),
            mid_end: Some(100_100.0),
            price_move_bps: Some(10.0),
            spread_bps_median: Some(2.0),
            imbalance_10bps_median: Some(0.4),
            data_quality: DataQuality {
                has_trades: true,
                has_books: true,
                active_venues: vec!["binance".to_string(), "bybit".to_string()],
                stale_venues: Vec::new(),
            },
        },
    );
    FlowState {
        symbol: "BTC-PERP".to_string(),
        updated_at: 5_000,
        windows,
    }
}

fn sweep_state(direction: SweepDirection) -> SweepState {
    SweepState {
        symbol: "BTC-PERP".to_string(),
        updated_at: 5_000,
        windows_ms: vec![1000, 5000, 15000],
        results: BTreeMap::from([(
            "5000".to_string(),
            SweepResult {
                symbol: "BTC-PERP".to_string(),
                window_ms: 5_000,
                direction,
                sweep_detected: direction != SweepDirection::None,
                swept_volume_btc: 500.0,
                swept_volume_usd: 50_000_000.0,
                aggressive_buy_btc: 600.0,
                aggressive_sell_btc: 100.0,
                net_aggressive_btc: 500.0,
                trade_count: 10,
                same_direction_trade_count: 8,
                price_start: Some(100_000.0),
                price_end: Some(100_100.0),
                price_impact_bps: Some(10.0),
                leader_venue: None,
                venue_breakdown: BTreeMap::new(),
                liquidity: None,
                reason_codes: vec!["sweep_detected".to_string()],
            },
        )]),
        quality: SweepQuality {
            has_trades: true,
            has_books: true,
            active_venues: Vec::new(),
            stale_venues: Vec::new(),
        },
    }
}

fn vpin_state(high: bool, spike: bool, extreme: bool) -> VpinState {
    VpinState {
        symbol: "BTC-PERP".to_string(),
        updated_at: 5_000,
        metrics: VpinMetrics {
            symbol: "BTC-PERP".to_string(),
            updated_at: 5_000,
            enabled: true,
            bucket_size_btc: 100.0,
            lookback_buckets: 50,
            min_buckets: 10,
            completed_bucket_count: 20,
            active_bucket_progress_btc: 0.0,
            active_bucket_progress_ratio: 0.0,
            latest_bucket: None,
            vpin: Some(if extreme { 0.9 } else { 0.75 }),
            vpin_zscore: Some(if spike { 3.0 } else { 1.0 }),
            vpin_percentile: Some(0.9),
            per_venue_vpin: BTreeMap::new(),
            latest_bucket_imbalance_ratio: Some(0.8),
            avg_bucket_imbalance_ratio: Some(0.7),
            vpin_high: high,
            vpin_extreme: extreme,
            vpin_spike: spike,
            dominant_direction: VpinDirection::Buy,
            reason_codes: Vec::new(),
        },
        recent_buckets: Vec::new(),
    }
}
