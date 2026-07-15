use std::collections::BTreeMap;

use btc_toxic_flow_monitor_rs::{
    config::thresholds::LiqHuntParams,
    toxicity::liq_hunt_detector::{LiqHuntDetector, LiqHuntDetectorInput},
    types::{
        flow::{empty_venue_breakdown, DataQuality, FlowState, FlowWindow},
        liq_hunt::{LiqHuntDirection, LiqHuntSignalLevel},
        liquidation::{
            empty_liquidation_state, EstimatedLiquidationCluster, LiquidationClusterSide,
            LiquidationState,
        },
        sweep::{
            empty_venue_sweep_breakdown, LiquidityThinnessResult, SweepDirection, SweepQuality,
            SweepResult, SweepState,
        },
        toxic::{
            empty_venue_toxic_breakdown, ToxicDirection, ToxicQuality, ToxicSeverity, ToxicState,
            ToxicVolumeResult,
        },
        vpin::{VpinDirection, VpinMetrics, VpinState},
    },
};

#[test]
fn buy_toxic_into_short_cluster_maps_to_short_squeeze() {
    let detector = LiqHuntDetector::new(LiqHuntParams::default());
    let result = detector.detect(input(
        sample_toxic(
            ToxicDirection::Buy,
            ToxicSeverity::Alert,
            1284.0,
            true,
            true,
        ),
        Some(sample_vpin(true, false)),
        sample_sweep(SweepDirection::Buy, true),
        sample_liquidation(
            LiquidationClusterSide::ShortAbove,
            true,
            true,
            Some(18.0),
            Some(72_000_000.0),
        ),
        sample_flow(Some(6.1)),
    ));

    assert_eq!(result.direction, LiqHuntDirection::ShortSqueeze);
    assert!(matches!(
        result.level,
        LiqHuntSignalLevel::Likely | LiqHuntSignalLevel::Active
    ));
    assert!(result
        .reason_codes
        .iter()
        .any(|code| code == "buy_toxic_into_short_cluster"));
}

#[test]
fn sell_toxic_into_long_cluster_maps_to_long_squeeze() {
    let detector = LiqHuntDetector::new(LiqHuntParams::default());
    let result = detector.detect(input(
        sample_toxic(
            ToxicDirection::Sell,
            ToxicSeverity::Alert,
            1180.0,
            true,
            true,
        ),
        Some(sample_vpin(false, false)),
        sample_sweep(SweepDirection::Sell, true),
        sample_liquidation(
            LiquidationClusterSide::LongBelow,
            true,
            true,
            Some(16.0),
            Some(60_000_000.0),
        ),
        sample_flow(Some(-5.5)),
    ));

    assert_eq!(result.direction, LiqHuntDirection::LongSqueeze);
    assert!(matches!(
        result.level,
        LiqHuntSignalLevel::Likely | LiqHuntSignalLevel::Active
    ));
    assert!(result
        .reason_codes
        .iter()
        .any(|code| code == "sell_toxic_into_long_cluster"));
}

#[test]
fn mismatched_direction_does_not_promote_squeeze() {
    let detector = LiqHuntDetector::new(LiqHuntParams::default());
    let result = detector.detect(input(
        sample_toxic(
            ToxicDirection::Buy,
            ToxicSeverity::Alert,
            1284.0,
            true,
            true,
        ),
        Some(sample_vpin(false, false)),
        sample_sweep(SweepDirection::Buy, true),
        sample_liquidation(
            LiquidationClusterSide::LongBelow,
            true,
            true,
            Some(18.0),
            Some(72_000_000.0),
        ),
        sample_flow(Some(6.1)),
    ));

    assert_eq!(result.direction, LiqHuntDirection::None);
    assert!(result.level.rank() <= LiqHuntSignalLevel::Watch.rank());
}

#[test]
fn missing_liquidation_cluster_caps_level_at_watch() {
    let detector = LiqHuntDetector::new(LiqHuntParams::default());
    let result = detector.detect(input(
        sample_toxic(
            ToxicDirection::Buy,
            ToxicSeverity::Alert,
            1284.0,
            true,
            true,
        ),
        Some(sample_vpin(true, false)),
        sample_sweep(SweepDirection::Buy, true),
        empty_liquidation_state(1_770_000_000_000),
        sample_flow(Some(6.1)),
    ));

    assert!(result.level.rank() <= LiqHuntSignalLevel::Watch.rank());
}

#[test]
fn missing_toxic_flow_caps_level_at_watch() {
    let detector = LiqHuntDetector::new(LiqHuntParams::default());
    let result = detector.detect(input(
        sample_toxic(
            ToxicDirection::Neutral,
            ToxicSeverity::Normal,
            0.0,
            false,
            false,
        ),
        Some(sample_vpin(true, false)),
        sample_sweep(SweepDirection::Buy, true),
        sample_liquidation(
            LiquidationClusterSide::ShortAbove,
            true,
            true,
            Some(18.0),
            Some(72_000_000.0),
        ),
        sample_flow(Some(6.1)),
    ));

    assert!(result.level.rank() <= LiqHuntSignalLevel::Watch.rank());
}

#[test]
fn active_requires_price_moving_toward_cluster() {
    let detector = LiqHuntDetector::new(LiqHuntParams::default());
    let result = detector.detect(input(
        sample_toxic(
            ToxicDirection::Buy,
            ToxicSeverity::Extreme,
            2200.0,
            true,
            true,
        ),
        Some(sample_vpin(false, true)),
        sample_sweep(SweepDirection::Buy, true),
        sample_liquidation(
            LiquidationClusterSide::ShortAbove,
            true,
            true,
            Some(12.0),
            Some(80_000_000.0),
        ),
        sample_flow(Some(-1.0)),
    ));

    assert_ne!(result.level, LiqHuntSignalLevel::Active);
}

#[test]
fn vpin_spike_and_large_cluster_raise_score() {
    let detector = LiqHuntDetector::new(LiqHuntParams::default());
    let baseline = detector.detect(input(
        sample_toxic(
            ToxicDirection::Buy,
            ToxicSeverity::Alert,
            1284.0,
            true,
            true,
        ),
        None,
        sample_sweep(SweepDirection::Buy, true),
        sample_liquidation(
            LiquidationClusterSide::ShortAbove,
            true,
            true,
            Some(24.0),
            Some(10_000_000.0),
        ),
        sample_flow(Some(4.0)),
    ));
    let boosted = detector.detect(input(
        sample_toxic(
            ToxicDirection::Buy,
            ToxicSeverity::Alert,
            1284.0,
            true,
            true,
        ),
        Some(sample_vpin(true, true)),
        sample_sweep(SweepDirection::Buy, true),
        sample_liquidation(
            LiquidationClusterSide::ShortAbove,
            true,
            true,
            Some(18.0),
            Some(75_000_000.0),
        ),
        sample_flow(Some(4.0)),
    ));

    assert!(boosted.score >= baseline.score);
    assert!(boosted.reason_codes.iter().any(|code| code == "vpin_spike"));
    assert!(boosted
        .reason_codes
        .iter()
        .any(|code| code == "cluster_notional_large"));
}

fn input(
    toxic_state: ToxicState,
    vpin_state: Option<VpinState>,
    sweep_state: SweepState,
    liquidation_state: LiquidationState,
    flow_state: FlowState,
) -> LiqHuntDetectorInput {
    LiqHuntDetectorInput {
        now_ts: 1_770_000_000_000,
        symbol: "BTC-PERP".to_string(),
        toxic_state,
        vpin_state,
        sweep_state,
        liquidation_state,
        flow_state,
    }
}

fn sample_toxic(
    direction: ToxicDirection,
    severity: ToxicSeverity,
    toxic_volume_btc: f64,
    cross_venue_confirmed: bool,
    liquidity_thin: bool,
) -> ToxicState {
    let result = ToxicVolumeResult {
        symbol: "BTC-PERP".to_string(),
        window_ms: 5000,
        ts: 1_770_000_000_000,
        direction,
        severity,
        toxic_ratio: 0.82,
        toxic_volume_btc,
        threshold_btc: 1000.0,
        alert_triggered: toxic_volume_btc >= 1000.0,
        aggressive_buy_btc: if direction == ToxicDirection::Buy {
            1566.0
        } else {
            220.0
        },
        aggressive_sell_btc: if direction == ToxicDirection::Sell {
            1510.0
        } else {
            220.0
        },
        net_aggressive_btc: match direction {
            ToxicDirection::Buy => 1346.0,
            ToxicDirection::Sell => -1290.0,
            ToxicDirection::Neutral => 0.0,
        },
        abs_aggressive_btc: 1786.0,
        markout_1s_bps: Some(2.1),
        markout_5s_bps: Some(4.8),
        markout_confirmed: true,
        sweep_detected: true,
        liquidity_thin,
        liquidity: Some(LiquidityThinnessResult {
            symbol: "BTC-PERP".to_string(),
            window_ms: 5000,
            bid_depth_start_btc: Some(10.0),
            bid_depth_end_btc: Some(6.0),
            ask_depth_start_btc: Some(10.0),
            ask_depth_end_btc: Some(4.0),
            bid_depth_drop_ratio: Some(0.4),
            ask_depth_drop_ratio: Some(0.6),
            spread_start_bps: Some(1.0),
            spread_end_bps: Some(1.5),
            spread_widen_ratio: Some(0.5),
            bid_thin: true,
            ask_thin: true,
            spread_widened: true,
            reason_codes: vec![],
        }),
        cross_venue_confirmed,
        vpin_enabled: true,
        vpin: Some(0.75),
        vpin_zscore: Some(2.6),
        vpin_spike: false,
        vpin_high: true,
        vpin_extreme: false,
        liquidation_enabled: true,
        nearest_cluster_side: None,
        cluster_distance_bps: None,
        cluster_notional_usd: None,
        cluster_density: None,
        liq_hunt_pressure: 0.7,
        liq_cluster_nearby: false,
        possible_liq_hunt_setup: false,
        leader_venue: None,
        venue_breakdown: empty_venue_toxic_breakdown(),
        reason_codes: vec!["threshold_crossed".to_string()],
    };

    ToxicState {
        symbol: "BTC-PERP".to_string(),
        updated_at: 1_770_000_000_000,
        threshold_btc: 1000.0,
        windows_ms: vec![1000, 5000, 15000, 60000],
        results: BTreeMap::from([("5000".to_string(), result)]),
        latest_event: None,
        recent_events: vec![],
        quality: ToxicQuality {
            has_flow: true,
            has_markout: true,
            has_sweep: true,
            has_liquidation: true,
            liquidation: None,
            active_venues: vec![],
            stale_venues: vec![],
        },
    }
}

fn sample_vpin(spike: bool, extreme: bool) -> VpinState {
    VpinState {
        symbol: "BTC-PERP".to_string(),
        updated_at: 1_770_000_000_000,
        metrics: VpinMetrics {
            symbol: "BTC-PERP".to_string(),
            updated_at: 1_770_000_000_000,
            enabled: true,
            bucket_size_btc: 100.0,
            lookback_buckets: 50,
            min_buckets: 10,
            completed_bucket_count: 20,
            active_bucket_progress_btc: 20.0,
            active_bucket_progress_ratio: 0.2,
            latest_bucket: None,
            vpin: Some(if extreme { 0.9 } else { 0.72 }),
            vpin_zscore: Some(if spike { 2.8 } else { 1.2 }),
            vpin_percentile: Some(0.9),
            per_venue_vpin: BTreeMap::new(),
            latest_bucket_imbalance_ratio: Some(0.8),
            avg_bucket_imbalance_ratio: Some(0.65),
            vpin_high: true,
            vpin_extreme: extreme,
            vpin_spike: spike,
            dominant_direction: VpinDirection::Buy,
            reason_codes: vec![],
        },
        recent_buckets: vec![],
    }
}

fn sample_sweep(direction: SweepDirection, detected: bool) -> SweepState {
    SweepState {
        symbol: "BTC-PERP".to_string(),
        updated_at: 1_770_000_000_000,
        windows_ms: vec![1000, 5000, 15000],
        results: BTreeMap::from([(
            "5000".to_string(),
            SweepResult {
                symbol: "BTC-PERP".to_string(),
                window_ms: 5000,
                direction,
                sweep_detected: detected,
                swept_volume_btc: 400.0,
                swept_volume_usd: 40_000_000.0,
                aggressive_buy_btc: 500.0,
                aggressive_sell_btc: 100.0,
                net_aggressive_btc: 400.0,
                trade_count: 10,
                same_direction_trade_count: 8,
                price_start: Some(100_000.0),
                price_end: Some(100_060.0),
                price_impact_bps: Some(6.0),
                leader_venue: None,
                venue_breakdown: empty_venue_sweep_breakdown(),
                liquidity: Some(LiquidityThinnessResult {
                    symbol: "BTC-PERP".to_string(),
                    window_ms: 5000,
                    bid_depth_start_btc: Some(10.0),
                    bid_depth_end_btc: Some(5.0),
                    ask_depth_start_btc: Some(10.0),
                    ask_depth_end_btc: Some(4.0),
                    bid_depth_drop_ratio: Some(0.5),
                    ask_depth_drop_ratio: Some(0.6),
                    spread_start_bps: Some(1.0),
                    spread_end_bps: Some(1.6),
                    spread_widen_ratio: Some(0.6),
                    bid_thin: true,
                    ask_thin: true,
                    spread_widened: true,
                    reason_codes: vec![],
                }),
                reason_codes: vec![],
            },
        )]),
        quality: SweepQuality {
            has_trades: true,
            has_books: true,
            active_venues: vec![],
            stale_venues: vec![],
        },
    }
}

fn sample_liquidation(
    side: LiquidationClusterSide,
    nearby: bool,
    possible: bool,
    distance_bps: Option<f64>,
    cluster_notional_usd: Option<f64>,
) -> LiquidationState {
    let mut state = empty_liquidation_state(1_770_000_000_000);
    state.metrics.enabled = true;
    state.metrics.current_mid = Some(100_000.0);
    state.metrics.nearest_cluster_side = Some(side);
    state.metrics.distance_bps = distance_bps;
    state.metrics.cluster_notional_usd = cluster_notional_usd;
    state.metrics.cluster_density = Some(0.7);
    state.metrics.liq_cluster_nearby = nearby;
    state.metrics.possible_liq_hunt_setup = possible;
    state.metrics.reason_codes = vec!["liq_cluster_nearby".to_string()];
    let cluster = EstimatedLiquidationCluster {
        side,
        price: if side == LiquidationClusterSide::ShortAbove {
            100_180.0
        } else {
            99_820.0
        },
        distance_bps: distance_bps.unwrap_or(18.0),
        cluster_notional_usd: cluster_notional_usd.unwrap_or(70_000_000.0),
        cluster_density: 0.7,
        touched_snapshots: 4,
        first_seen_ts: 1_770_000_000_000 - 5000,
        last_seen_ts: 1_770_000_000_000,
        reason_codes: vec![],
    };
    match side {
        LiquidationClusterSide::ShortAbove => {
            state.metrics.nearest_short_liq_cluster_above = Some(cluster.clone());
        }
        LiquidationClusterSide::LongBelow => {
            state.metrics.nearest_long_liq_cluster_below = Some(cluster.clone());
        }
    }
    state.recent_clusters = vec![cluster];
    state
}

fn sample_flow(price_move_bps: Option<f64>) -> FlowState {
    FlowState {
        symbol: "BTC-PERP".to_string(),
        updated_at: 1_770_000_000_000,
        windows: BTreeMap::from([(
            "5000".to_string(),
            FlowWindow {
                symbol: "BTC-PERP".to_string(),
                window_ms: 5000,
                now_ts: 1_770_000_000_000,
                aggressive_buy_btc: 1566.0,
                aggressive_sell_btc: 220.0,
                aggressive_buy_usd: 156_600_000.0,
                aggressive_sell_usd: 22_000_000.0,
                net_aggressive_btc: 1346.0,
                abs_aggressive_btc: 1786.0,
                trade_count: 10,
                buy_trade_count: 8,
                sell_trade_count: 2,
                avg_trade_size_btc: 178.6,
                max_trade_size_btc: 600.0,
                venue_breakdown: empty_venue_breakdown(),
                mid_start: Some(100_000.0),
                mid_end: Some(100_060.0),
                price_move_bps,
                spread_bps_median: Some(1.5),
                imbalance_10bps_median: Some(0.6),
                data_quality: DataQuality {
                    has_trades: true,
                    has_books: true,
                    active_venues: vec![],
                    stale_venues: vec![],
                },
            },
        )]),
    }
}
