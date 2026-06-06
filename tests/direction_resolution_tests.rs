use btc_toxic_flow_monitor_rs::runtime::tof_metrics::{
    resolve_final_direction, resolve_metrics_direction, TofDirection,
};

#[test]
fn metrics_direction_uses_trade_imbalance_and_depth_withdrawal() {
    assert_eq!(
        resolve_metrics_direction(-0.45, 58.0, 10.0, 35.0),
        TofDirection::Bearish
    );
    assert_eq!(
        resolve_metrics_direction(0.45, 8.0, 62.0, 35.0),
        TofDirection::Bullish
    );
}

#[test]
fn conflicting_detector_and_metrics_direction_resolves_to_mixed() {
    let resolution = resolve_final_direction(TofDirection::Bullish, TofDirection::Bearish, 80.0);

    assert_eq!(resolution.final_direction, TofDirection::Mixed);
    assert_eq!(resolution.direction_source, "conflict_detector_tof_metrics");
    assert!(resolution.direction_confidence < 50.0);
}

#[test]
fn aligned_detector_and_metrics_direction_raises_confidence() {
    let resolution = resolve_final_direction(TofDirection::Bearish, TofDirection::Bearish, 78.0);

    assert_eq!(resolution.final_direction, TofDirection::Bearish);
    assert_eq!(resolution.direction_source, "detector+tof_metrics");
    assert!(resolution.direction_confidence > 80.0);
}
