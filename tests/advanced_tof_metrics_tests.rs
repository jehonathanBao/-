use btc_toxic_flow_monitor_rs::runtime::{
    advanced_tof_metrics::{
        build_advanced_tof_metrics, fused_data_quality, fused_risk_score, AdvancedTofInput,
    },
    metric_provenance::{MetricLineage, MetricProvenance},
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
fn inferred_liquidation_proxy_does_not_change_alert_eligible_advanced_metrics() {
    let tof = tof_metrics();
    let with_proxy = perp_metrics();
    let mut without_proxy = with_proxy.clone();
    without_proxy.liquidation_pressure = 0.0;
    let build = |perp: &PerpTofMetrics| {
        build_advanced_tof_metrics(&AdvancedTofInput {
            symbol: "BTC-PERP",
            spot_candidate_type: "SpoofingCandidate",
            spot_direction: TofDirection::Bullish,
            spot_risk_score: 88,
            spot_data_quality: 86.0,
            spot_confidence: 0.92,
            tof_metrics: &tof,
            spot_tags: &[],
            perp_metrics: perp,
            summary: "observed inputs",
        })
    };

    let with_proxy = build(&with_proxy);
    let without_proxy = build(&without_proxy);

    assert_eq!(
        with_proxy.market_pressure_heatmap,
        without_proxy.market_pressure_heatmap
    );
    assert_eq!(
        with_proxy.metrics_completeness,
        without_proxy.metrics_completeness
    );
    assert_eq!(
        with_proxy.fresh_data_coverage,
        without_proxy.fresh_data_coverage
    );
    assert_eq!(with_proxy.confidence, without_proxy.confidence);
}

#[test]
fn advanced_vpin_indicator_uses_relative_context() {
    let _guard = ENV_LOCK
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner);
    let mut low_relative = tof_metrics();
    low_relative.vpin_proxy = 90.0;
    low_relative.vpin_zscore = Some(-2.0);
    low_relative.vpin_percentile = Some(0.05);
    let mut high_relative = low_relative.clone();
    high_relative.vpin_zscore = Some(3.0);
    high_relative.vpin_percentile = Some(0.99);

    let low = advanced_metrics_for_tof(&low_relative);
    let high = advanced_metrics_for_tof(&high_relative);

    assert!(
        high.vpin_enhanced > low.vpin_enhanced,
        "relative VPIN anomaly must raise the advanced indicator: low={} high={}",
        low.vpin_enhanced,
        high.vpin_enhanced
    );
}

#[test]
fn advanced_vpin_indicator_ignores_absolute_vpin_level() {
    let _guard = ENV_LOCK
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner);
    let mut low_raw = tof_metrics();
    low_raw.vpin_proxy = 10.0;
    low_raw.vpin_zscore = Some(-2.0);
    low_raw.vpin_percentile = Some(0.05);
    let mut high_raw = low_raw.clone();
    high_raw.vpin_proxy = 90.0;

    let low = advanced_metrics_for_tof(&low_raw);
    let high = advanced_metrics_for_tof(&high_raw);

    assert_eq!(
        low.vpin_enhanced, high.vpin_enhanced,
        "absolute VPIN must not raise the advanced indicator when relative context is unchanged"
    );
}

#[test]
fn advanced_tof_disabled_keeps_detector_score_without_advanced_indicators() {
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
    let json = serde_json::to_value(&metrics).expect("advanced json");
    assert_eq!(metrics.final_risk_score, 88);
    assert_eq!(metrics.lineage.provenance, MetricProvenance::Unavailable);
    assert_eq!(json["vpinEnhanced"], serde_json::Value::Null);
    assert!(metrics
        .explain_tags
        .contains(&"Advanced TOF disabled".to_string()));
}

fn advanced_metrics_for_tof(
    tof_metrics: &TofMetrics,
) -> btc_toxic_flow_monitor_rs::runtime::advanced_tof_metrics::AdvancedTofMetrics {
    build_advanced_tof_metrics(&AdvancedTofInput {
        symbol: "BTC-PERP",
        spot_candidate_type: "SpoofingCandidate",
        spot_direction: TofDirection::Bullish,
        spot_risk_score: 88,
        spot_data_quality: 86.0,
        spot_confidence: 0.92,
        tof_metrics,
        spot_tags: &[],
        perp_metrics: &perp_metrics(),
        summary: "observed inputs",
    })
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
        vpin_zscore: Some(2.2),
        vpin_percentile: Some(0.95),
        per_venue_vpin: std::collections::BTreeMap::new(),
        lineage: MetricLineage::calculated("test_tof", 10_000, true),
        metric_lineage: std::collections::BTreeMap::new(),
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
        observed_liquidation_notional: None,
        lineage: MetricLineage::calculated("test_perp", 10_000, true),
        liquidation_lineage: MetricLineage::inferred("test_proxy", 10_000),
    }
}
