use btc_toxic_flow_monitor_rs::runtime::tof_metrics::{
    final_risk_score, tof_score, TofDirection, TofMetrics,
};

#[test]
fn tof_score_uses_weighted_microstructure_metrics() {
    let metrics = TofMetrics {
        trade_imbalance: -0.50,
        trade_imbalance_score: 50.0,
        vpin_proxy: 90.0,
        vpin_bucket_count: 20,
        vpin_window_volume: 2_000_000.0,
        bid_depth_withdrawal: 80.0,
        ask_depth_withdrawal: 12.0,
        depth_withdrawal_score: 80.0,
        spread_bps: 12.0,
        spread_widening_score: 90.0,
        order_churn_score: 55.0,
        book_update_rate: 120.0,
        trade_rate: 20.0,
        liquidity_vacuum_score: 68.0,
        thin_side: "bid".to_string(),
        metrics_direction: TofDirection::Bearish,
        metrics_confidence: 86.0,
        tof_score: 0.0,
        final_risk_score: 0,
        metrics_completeness: 0.9,
    };

    assert!((tof_score(&metrics) - 74.3).abs() < 0.2);
}

#[test]
fn final_risk_score_keeps_existing_score_weighted_with_tof_score() {
    std::env::set_var("TOF_SCORE_WEIGHT_EXISTING", "0.60");
    std::env::set_var("TOF_SCORE_WEIGHT_METRICS", "0.40");

    assert_eq!(final_risk_score(80.0, 90.0), 84);
    assert_eq!(final_risk_score(10.0, 500.0), 46);

    std::env::remove_var("TOF_SCORE_WEIGHT_EXISTING");
    std::env::remove_var("TOF_SCORE_WEIGHT_METRICS");
}
