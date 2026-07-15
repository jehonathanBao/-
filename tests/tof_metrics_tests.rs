use btc_toxic_flow_monitor_rs::runtime::{
    metric_provenance::MetricProvenance,
    tof_metrics::{
        build_tof_metrics_from_observed, depth_withdrawal, enhance_signal_summary, spread_bps,
        trade_imbalance, vpin_proxy, ObservedTofSnapshot, TofDirection, TofSummaryInput,
    },
};
use std::collections::BTreeMap;

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
fn summary_only_enrichment_is_unavailable_and_does_not_change_detector_risk() {
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

    let json = serde_json::to_value(&enhancement).expect("enhancement json");

    assert_eq!(enhancement.direction, TofDirection::Bearish);
    assert_eq!(enhancement.candidate_type, "spoofing_candidate");
    assert_eq!(enhancement.final_risk_score, 85);
    assert_eq!(enhancement.tof_metrics.final_risk_score, 85);
    assert_eq!(
        enhancement.tof_metrics.lineage.provenance,
        MetricProvenance::Unavailable
    );
    assert!(!enhancement.tof_metrics.lineage.alert_eligible);
    assert_eq!(
        json["tofMetrics"]["tradeImbalance"],
        serde_json::Value::Null
    );
    assert_eq!(json["tofMetrics"]["vpinProxy"], serde_json::Value::Null);
    assert_eq!(json["tofMetrics"]["spreadBps"], serde_json::Value::Null);
    assert_eq!(json["tofScore"], serde_json::Value::Null);
    assert_eq!(enhancement.direction_source, "detector");
    assert!(enhancement.explain_tags.is_empty());
}

#[test]
fn observed_snapshot_calculates_hazard_without_changing_detector_risk_or_direction() {
    let snapshot = observed_snapshot_at(10_000);

    let bullish = build_tof_metrics_from_observed(&snapshot, "BTC-PERP", 9_500, 10_000, 83);
    let bearish = build_tof_metrics_from_observed(&snapshot, "BTC-PERP", 9_500, 10_000, 83);

    assert_eq!(bullish.final_risk_score, 83);
    assert_eq!(bearish.final_risk_score, 83);
    assert_eq!(bullish.tof_score, bearish.tof_score);
    assert_eq!(bullish.vpin_zscore, Some(2.4));
    assert_eq!(bullish.vpin_percentile, Some(0.96));
    assert_eq!(bullish.per_venue_vpin["binance"], 0.84);
    assert_eq!(
        bullish.lineage.provenance,
        MetricProvenance::CalculatedFromObserved
    );
    assert!(bullish.lineage.alert_eligible);
}

#[test]
fn observed_tof_hazard_uses_relative_vpin_context() {
    let mut low_relative = observed_snapshot_at(10_000);
    low_relative.vpin = Some(0.90);
    low_relative.vpin_zscore = Some(-2.0);
    low_relative.vpin_percentile = Some(0.05);
    let mut high_relative = low_relative.clone();
    high_relative.vpin_zscore = Some(3.0);
    high_relative.vpin_percentile = Some(0.99);

    let low = build_tof_metrics_from_observed(&low_relative, "BTC-PERP", 9_500, 10_000, 83);
    let high = build_tof_metrics_from_observed(&high_relative, "BTC-PERP", 9_500, 10_000, 83);

    assert_eq!(low.vpin_proxy, high.vpin_proxy);
    assert!(
        high.tof_score > low.tof_score,
        "relative VPIN anomaly must raise hazard: low={} high={}",
        low.tof_score,
        high.tof_score
    );
}

#[test]
fn observed_tof_hazard_ignores_absolute_vpin_level() {
    let mut low_raw = observed_snapshot_at(10_000);
    low_raw.vpin = Some(0.10);
    low_raw.vpin_zscore = Some(-2.0);
    low_raw.vpin_percentile = Some(0.05);
    let mut high_raw = low_raw.clone();
    high_raw.vpin = Some(0.90);

    let low = build_tof_metrics_from_observed(&low_raw, "BTC-PERP", 9_500, 10_000, 83);
    let high = build_tof_metrics_from_observed(&high_raw, "BTC-PERP", 9_500, 10_000, 83);

    assert_ne!(
        low.vpin_proxy, high.vpin_proxy,
        "raw VPIN remains display data"
    );
    assert_eq!(
        low.tof_score, high.tof_score,
        "absolute VPIN must not raise the hazard score when relative context is unchanged"
    );
}

#[test]
fn observed_snapshot_beyond_candidate_future_skew_is_unavailable() {
    let snapshot = observed_snapshot_at(15_001);

    let metrics = build_tof_metrics_from_observed(&snapshot, "BTC-PERP", 10_000, 15_001, 83);

    assert_eq!(metrics.lineage.provenance, MetricProvenance::Unavailable);
    assert!(!metrics.lineage.alert_eligible);
    assert_eq!(
        metrics.lineage.unavailable_reason.as_deref(),
        Some("observed_tof_stale")
    );
    assert_eq!(metrics.final_risk_score, 83);
}

#[test]
fn future_candidate_cannot_bind_current_tof_observation() {
    let snapshot = observed_snapshot_at(10_000);

    let metrics = build_tof_metrics_from_observed(&snapshot, "BTC-PERP", 15_001, 10_000, 83);

    assert_eq!(metrics.lineage.provenance, MetricProvenance::Unavailable);
    assert!(!metrics.lineage.alert_eligible);
    assert_eq!(
        metrics.lineage.unavailable_reason.as_deref(),
        Some("observed_tof_stale")
    );
    assert_eq!(metrics.final_risk_score, 83);
}

fn observed_snapshot_at(observed_at_ms: i64) -> ObservedTofSnapshot {
    ObservedTofSnapshot {
        symbol: "BTC-PERP".to_string(),
        observed_at_ms,
        buy_volume: 300.0,
        sell_volume: 100.0,
        trade_count: 40,
        window_ms: 5_000,
        vpin: Some(0.82),
        vpin_zscore: Some(2.4),
        vpin_percentile: Some(0.96),
        vpin_bucket_count: 20,
        vpin_window_volume: 2_000.0,
        per_venue_vpin: BTreeMap::from([
            ("binance".to_string(), 0.84),
            ("bybit".to_string(), 0.78),
        ]),
        bid_depth_withdrawal: Some(12.0),
        ask_depth_withdrawal: Some(62.0),
        spread_bps: Some(9.0),
        book_update_rate: None,
        sweep_score: Some(70.0),
    }
}
