use btc_toxic_flow_monitor_rs::runtime::tof_metrics::{
    depth_withdrawal, enhance_signal_summary, spread_bps, trade_imbalance, vpin_proxy,
    TofDirection, TofSummaryInput,
};

#[test]
fn trade_imbalance_maps_sell_pressure_to_bearish_input() {
    let imbalance = trade_imbalance(100.0, 300.0);

    assert!((imbalance + 0.5).abs() < 0.0001);
}

#[test]
fn vpin_proxy_uses_average_bucket_imbalance() {
    let score = vpin_proxy(&[0.9, -0.8, 0.7, -0.6]);

    assert!(score >= 70.0);
}

#[test]
fn depth_withdrawal_and_spread_are_bounded() {
    assert_eq!(depth_withdrawal(100.0, 58.0).round(), 42.0);
    assert_eq!(depth_withdrawal(0.0, 58.0), 0.0);
    assert!((spread_bps(100.0, 100.2) - 19.98).abs() < 0.1);
}

#[test]
fn summary_enhancement_adds_safe_tof_metrics_and_tags() {
    let enhancement = enhance_signal_summary(&TofSummaryInput {
        signal_kind: "spoofing_candidate",
        direction_bias: "short_bias",
        severity: "high",
        confidence: 0.82,
        quality_bucket: "good",
        summary: "large ask wall removed after aggressive sell volume",
        existing_risk_score: 85,
        existing_data_quality: 82.0,
    });

    assert_eq!(enhancement.direction, TofDirection::Bearish);
    assert_eq!(enhancement.candidate_type, "spoofing_candidate");
    assert!(enhancement.tof_metrics.vpin_proxy > 70.0);
    assert!(enhancement.tof_metrics.bid_depth_withdrawal > 35.0);
    assert!(enhancement
        .explain_tags
        .contains(&"sell_volume_imbalance".to_string()));
    assert!(enhancement
        .explain_tags
        .contains(&"bid_depth_withdrawal".to_string()));
}
