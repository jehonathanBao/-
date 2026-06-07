use btc_toxic_flow_monitor_rs::runtime::{
    perp_tof_metrics::classify_liquidation_pressure, tof_metrics::TofDirection,
};

#[test]
fn liquidation_pressure_maps_long_squeeze_to_bearish_and_short_squeeze_to_bullish() {
    let (candidate, direction, score) = classify_liquidation_pressure(82.0, "long");
    assert_eq!(candidate, "LongSqueezeCandidate");
    assert_eq!(direction, TofDirection::Bearish);
    assert_eq!(score, 82.0);

    let (candidate, direction, score) = classify_liquidation_pressure(86.0, "short");
    assert_eq!(candidate, "ShortSqueezeCandidate");
    assert_eq!(direction, TofDirection::Bullish);
    assert_eq!(score, 86.0);
}
