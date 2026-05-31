use btc_toxic_flow_monitor_rs::{
    market_data::price_index::PriceSnapshot, toxicity::liquidity_thinness::LiquidityThinness,
};

#[test]
fn ask_depth_drop_marks_ask_thin() {
    let detector = LiquidityThinness::default();
    let result = detector.detect(
        "BTC-PERP",
        5000,
        Some(snapshot(0, Some(1000.0), Some(1000.0), Some(2.0))),
        Some(snapshot(5000, Some(1000.0), Some(600.0), Some(2.0))),
    );

    assert!(result.ask_thin);
    assert!(!result.bid_thin);
    assert_eq!(result.ask_depth_drop_ratio, Some(0.4));
    assert!(result
        .reason_codes
        .contains(&"ask_liquidity_thinned".to_string()));
}

#[test]
fn bid_depth_drop_marks_bid_thin() {
    let detector = LiquidityThinness::default();
    let result = detector.detect(
        "BTC-PERP",
        5000,
        Some(snapshot(0, Some(1000.0), Some(1000.0), Some(2.0))),
        Some(snapshot(5000, Some(600.0), Some(1000.0), Some(2.0))),
    );

    assert!(result.bid_thin);
    assert!(!result.ask_thin);
    assert_eq!(result.bid_depth_drop_ratio, Some(0.4));
}

#[test]
fn spread_widen_marks_spread_widened() {
    let detector = LiquidityThinness::default();
    let result = detector.detect(
        "BTC-PERP",
        5000,
        Some(snapshot(0, Some(1000.0), Some(1000.0), Some(2.0))),
        Some(snapshot(5000, Some(1000.0), Some(1000.0), Some(3.0))),
    );

    assert!(result.spread_widened);
    assert_eq!(result.spread_widen_ratio, Some(0.5));
}

#[test]
fn zero_start_depth_does_not_panic_or_emit_ratio() {
    let detector = LiquidityThinness::default();
    let result = detector.detect(
        "BTC-PERP",
        5000,
        Some(snapshot(0, Some(0.0), Some(0.0), Some(2.0))),
        Some(snapshot(5000, Some(100.0), Some(100.0), Some(2.0))),
    );

    assert_eq!(result.bid_depth_drop_ratio, None);
    assert_eq!(result.ask_depth_drop_ratio, None);
    assert!(!result.bid_thin);
    assert!(!result.ask_thin);
}

#[test]
fn missing_snapshot_returns_empty_result() {
    let detector = LiquidityThinness::default();
    let result = detector.detect("BTC-PERP", 5000, None, None);

    assert!(!result.bid_thin);
    assert!(!result.ask_thin);
    assert!(!result.spread_widened);
    assert_eq!(result.bid_depth_drop_ratio, None);
    assert!(result
        .reason_codes
        .contains(&"missing_price_snapshot".to_string()));
}

fn snapshot(
    ts: i64,
    bid_depth: Option<f64>,
    ask_depth: Option<f64>,
    spread_bps: Option<f64>,
) -> PriceSnapshot {
    PriceSnapshot {
        ts,
        index_mid: 100.0,
        spread_bps_median: spread_bps,
        imbalance_10bps_median: Some(0.0),
        bid_depth_btc_10bps_median: bid_depth,
        ask_depth_btc_10bps_median: ask_depth,
    }
}
