use btc_toxic_flow_monitor_rs::runtime::{
    perp_tof_metrics::classify_aggressive_order_flow, tof_metrics::TofDirection,
};

#[test]
fn aggressive_order_flow_resolves_direction_from_buy_sell_volume() {
    let (direction, score) = classify_aggressive_order_flow(1_500_000.0, 450_000.0);
    assert_eq!(direction, TofDirection::Bullish);
    assert!(score >= 50.0);

    let (direction, score) = classify_aggressive_order_flow(450_000.0, 1_500_000.0);
    assert_eq!(direction, TofDirection::Bearish);
    assert!(score >= 50.0);
}

#[test]
fn balanced_aggressive_flow_is_neutral() {
    let (direction, score) = classify_aggressive_order_flow(1_000_000.0, 920_000.0);

    assert_eq!(direction, TofDirection::Neutral);
    assert!(score < 18.0);
}
