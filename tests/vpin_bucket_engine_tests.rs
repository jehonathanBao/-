use btc_toxic_flow_monitor_rs::{
    config::thresholds::VpinParams,
    toxicity::vpin_bucket_engine::VpinBucketEngine,
    types::{
        market::{AggressorSide, NormalizedTrade, Venue},
        vpin::VpinDirection,
    },
};

#[test]
fn large_trade_splits_across_buckets() {
    let mut engine = VpinBucketEngine::new(VpinParams {
        bucket_size_btc: 100.0,
        min_buckets: 1,
        lookback_buckets: 3,
        ..VpinParams::default()
    });

    let completed = engine.on_trade(&trade(1, Venue::Binance, AggressorSide::Buy, 250.0));
    assert_eq!(completed.len(), 2);
    assert_eq!(completed[0].buy_btc, 100.0);
    assert_eq!(completed[1].buy_btc, 100.0);

    let state = engine.get_state(1);
    assert_eq!(state.metrics.completed_bucket_count, 2);
    assert_eq!(state.metrics.active_bucket_progress_btc, 50.0);
    assert_eq!(state.metrics.active_bucket_progress_ratio, 0.5);
}

#[test]
fn mixed_bucket_computes_imbalance_ratio_and_direction() {
    let mut engine = VpinBucketEngine::new(VpinParams {
        bucket_size_btc: 100.0,
        min_buckets: 1,
        lookback_buckets: 3,
        ..VpinParams::default()
    });

    let mut completed = engine.on_trade(&trade(1, Venue::Binance, AggressorSide::Buy, 70.0));
    completed.extend(engine.on_trade(&trade(2, Venue::Bybit, AggressorSide::Sell, 30.0)));
    assert_eq!(completed.len(), 1);
    assert_eq!(completed[0].direction, VpinDirection::Buy);
    assert!((completed[0].imbalance_ratio - 0.4).abs() < 1e-9);
    assert_eq!(completed[0].venue_breakdown["binance"].buy_btc, 70.0);
    assert_eq!(completed[0].venue_breakdown["bybit"].sell_btc, 30.0);
}

#[test]
fn vpin_and_flags_update_after_min_buckets() {
    let mut engine = VpinBucketEngine::new(VpinParams {
        bucket_size_btc: 100.0,
        min_buckets: 3,
        lookback_buckets: 3,
        high_threshold: 0.70,
        extreme_threshold: 0.85,
        spike_zscore: 1.0,
        ..VpinParams::default()
    });

    engine.on_trade(&trade(1, Venue::Binance, AggressorSide::Buy, 100.0));
    engine.on_trade(&trade(2, Venue::Binance, AggressorSide::Buy, 100.0));
    engine.on_trade(&trade(3, Venue::Bybit, AggressorSide::Buy, 50.0));
    engine.on_trade(&trade(4, Venue::Bybit, AggressorSide::Sell, 50.0));
    let state_before = engine.get_state(4);
    assert_eq!(state_before.metrics.vpin, Some((1.0 + 1.0 + 0.0) / 3.0));
    assert!(!state_before.metrics.vpin_high);

    engine.on_trade(&trade(5, Venue::Okx, AggressorSide::Buy, 100.0));
    let state_after = engine.get_state(5);
    assert!(state_after.metrics.vpin.is_some());
    assert!(!state_after.metrics.vpin_extreme);
    assert!(!state_after.metrics.vpin_high);
    assert!(state_after.metrics.vpin_zscore.is_some());
    assert!(!state_after.metrics.vpin_spike);
}

#[test]
fn std_zero_gives_no_zscore_and_clear_resets_state() {
    let mut engine = VpinBucketEngine::new(VpinParams {
        bucket_size_btc: 100.0,
        min_buckets: 2,
        lookback_buckets: 2,
        ..VpinParams::default()
    });
    engine.on_trade(&trade(1, Venue::Binance, AggressorSide::Buy, 100.0));
    engine.on_trade(&trade(2, Venue::Bybit, AggressorSide::Buy, 100.0));
    let state = engine.get_state(2);
    assert_eq!(state.metrics.vpin_zscore, None);

    engine.clear();
    let cleared = engine.get_state(3);
    assert_eq!(cleared.metrics.completed_bucket_count, 0);
    assert_eq!(cleared.metrics.active_bucket_progress_btc, 0.0);
}

fn trade(ts: i64, venue: Venue, side: AggressorSide, size_btc: f64) -> NormalizedTrade {
    NormalizedTrade {
        venue,
        symbol: "BTC-PERP".to_string(),
        ts,
        price: 100_000.0,
        size_btc,
        size_usd: size_btc * 100_000.0,
        aggressor_side: side,
        trade_id: Some(format!("{venue:?}-{ts}-{size_btc}")),
    }
}
