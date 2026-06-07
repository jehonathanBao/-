use btc_toxic_flow_monitor_rs::runtime::{
    advanced_tof_metrics::{
        build_advanced_tof_metrics, fused_data_quality, fused_risk_score, AdvancedTofInput,
    },
    perp_tof_metrics::PerpTofMetrics,
    tof_metrics::{TofDirection, TofMetrics},
};

static ENV_LOCK: std::sync::Mutex<()> = std::sync::Mutex::new(());

#[test]
fn fused_risk_score_uses_phase3_weights() {
    assert_eq!(fused_risk_score(80, 90.0, 70), 80);
    assert_eq!(fused_risk_score(90, 90.0, 90), 90);
}

#[test]
fn fused_data_quality_uses_completeness_and_fresh_coverage() {
    let quality = fused_data_quality(86.0, 82.0, 95.0, 90.0);

    assert!(quality > 86.0);
    assert!(quality <= 100.0);
}

#[test]
fn advanced_metrics_merge_spot_tof_and_perp_candidates() {
    let spot_tags = vec![
        "high_vpin_proxy".to_string(),
        "bid_depth_withdrawal".to_string(),
    ];
    let metrics = build_advanced_tof_metrics(&AdvancedTofInput {
        symbol: "BTC-PERP",
        spot_candidate_type: "SpoofingCandidate",
        spot_direction: TofDirection::Bullish,
        spot_risk_score: 88,
        spot_data_quality: 86.0,
        spot_confidence: 0.92,
        tof_metrics: &tof_metrics(),
        spot_tags: &spot_tags,
        perp_metrics: &perp_metrics(),
        summary: "large order flow cluster with market pressure",
    });

    assert!(metrics.final_risk_score >= 85);
    assert_eq!(metrics.metrics_direction, TofDirection::Bullish);
    assert!(metrics.final_candidate_type.contains("Advanced Candidate"));
    assert!(metrics
        .explain_tags
        .contains(&"Large order flow cluster".to_string()));
    assert!(metrics
        .explain_tags
        .contains(&"Historical funding/OI trend".to_string()));
}

#[test]
fn advanced_tof_disabled_keeps_fused_score_without_advanced_indicators() {
    let _guard = ENV_LOCK.lock().expect("env lock");
    std::env::set_var("ADVANCED_TOF_ENABLED", "false");
    let spot_tags = vec!["high_vpin_proxy".to_string()];

    let metrics = build_advanced_tof_metrics(&AdvancedTofInput {
        symbol: "BTC-PERP",
        spot_candidate_type: "SpoofingCandidate",
        spot_direction: TofDirection::Bullish,
        spot_risk_score: 88,
        spot_data_quality: 86.0,
        spot_confidence: 0.92,
        tof_metrics: &tof_metrics(),
        spot_tags: &spot_tags,
        perp_metrics: &perp_metrics(),
        summary: "safe summary",
    });

    std::env::remove_var("ADVANCED_TOF_ENABLED");

    assert_eq!(metrics.candidate_type, "AdvancedTofDisabled");
    assert_eq!(metrics.vpin_enhanced, 0.0);
    assert!(metrics.final_risk_score > 0);
    assert!(metrics
        .explain_tags
        .contains(&"Advanced TOF disabled".to_string()));
}

fn tof_metrics() -> TofMetrics {
    TofMetrics {
        trade_imbalance: 0.72,
        trade_imbalance_score: 72.0,
        vpin_proxy: 88.0,
        vpin_bucket_count: 20,
        vpin_window_volume: 2_000_000.0,
        bid_depth_withdrawal: 18.0,
        ask_depth_withdrawal: 82.0,
        depth_withdrawal_score: 82.0,
        spread_bps: 11.0,
        spread_widening_score: 78.0,
        order_churn_score: 84.0,
        book_update_rate: 130.0,
        trade_rate: 12.0,
        liquidity_vacuum_score: 74.0,
        thin_side: "ask".to_string(),
        metrics_direction: TofDirection::Bullish,
        metrics_confidence: 86.0,
        tof_score: 88.0,
        final_risk_score: 88,
        metrics_completeness: 0.92,
    }
}

fn perp_metrics() -> PerpTofMetrics {
    PerpTofMetrics {
        oi_change: 170_000.0,
        oi_direction: "long_increase".to_string(),
        funding_rate: -0.078,
        funding_side: "short".to_string(),
        liquidation_pressure: 86.0,
        squeeze_side: "short".to_string(),
        agg_buy_volume: 1_700_000.0,
        agg_sell_volume: 420_000.0,
        direction_bias: TofDirection::Bullish,
        metrics_direction: TofDirection::Bullish,
        risk_score: 87,
        data_quality: 88.0,
        candidate_type: "OpenInterestCandidate".to_string(),
        explain_tags: vec!["OI long increase".to_string()],
        confidence: 89.0,
    }
}
