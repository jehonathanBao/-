use std::sync::Mutex;

use btc_toxic_flow_monitor_rs::runtime::{
    metric_provenance::{MetricLineage, MetricProvenance},
    perp_tof_metrics::{
        build_perp_tof_metrics, build_perp_tof_metrics_from_observed, classify_open_interest,
        merge_spot_perp_candidate, ObservedPerpSnapshot, PerpTofInput, PerpTofMetrics,
    },
    tof_metrics::TofDirection,
};

static ENV_LOCK: Mutex<()> = Mutex::new(());

#[test]
fn open_interest_direction_follows_price_and_oi_change() {
    assert_eq!(
        classify_open_interest(10.0, 120_000.0),
        ("long_increase".to_string(), TofDirection::Bullish)
    );
    assert_eq!(
        classify_open_interest(10.0, -120_000.0),
        ("short_decrease".to_string(), TofDirection::Bullish)
    );
    assert_eq!(
        classify_open_interest(-10.0, 120_000.0),
        ("short_increase".to_string(), TofDirection::Bearish)
    );
    assert_eq!(
        classify_open_interest(-10.0, -120_000.0),
        ("long_decrease".to_string(), TofDirection::Bearish)
    );
}

#[test]
fn spot_and_perp_merge_boosts_aligned_high_risk_candidate() {
    let perp = perp_metrics("OpenInterestCandidate", TofDirection::Bullish, 86);
    let merged = merge_spot_perp_candidate(
        "SpoofingCandidate",
        TofDirection::Bullish,
        84,
        &["high_vpin_proxy".to_string()],
        &perp,
    );

    assert!(merged.risk_score >= 86);
    assert_eq!(merged.metrics_direction, TofDirection::Bullish);
    assert!(merged.final_candidate_type.contains("Bullish Candidate"));
    assert!(merged
        .explain_tags
        .contains(&"SpoofingCandidate".to_string()));
    assert!(merged
        .explain_tags
        .contains(&"OI long increase".to_string()));
}

#[test]
fn perp_builder_without_observed_evidence_returns_null_unavailable_metrics() {
    let _guard = ENV_LOCK.lock().expect("env lock");
    let metrics = build_perp_tof_metrics(&PerpTofInput {
        symbol: "BTC-PERP",
        spot_candidate_type: "SpoofingCandidate",
        spot_direction: TofDirection::Bullish,
        spot_risk_score: 88,
        spot_data_quality: 86.0,
        spot_confidence: 0.9,
        summary: "safe summary",
    });

    let json = serde_json::to_value(&metrics).expect("perp json");

    assert_eq!(metrics.candidate_type, "PerpEvidenceUnavailable");
    assert_eq!(metrics.metrics_direction, TofDirection::Neutral);
    assert_eq!(metrics.risk_score, 0);
    assert_eq!(metrics.data_quality, 0.0);
    assert_eq!(metrics.lineage.provenance, MetricProvenance::Unavailable);
    assert_eq!(json["oiChange"], serde_json::Value::Null);
    assert_eq!(json["fundingRate"], serde_json::Value::Null);
    assert_eq!(json["liquidationPressure"], serde_json::Value::Null);
}

#[test]
fn observed_perp_flow_calculates_volume_but_inferred_liquidation_stays_ineligible() {
    let metrics = build_perp_tof_metrics_from_observed(
        &ObservedPerpSnapshot {
            symbol: "BTC-PERP".to_string(),
            observed_at_ms: 10_000,
            price_change_bps: Some(25.0),
            oi_change: Some(0.40),
            funding_rate: Some(-0.0002),
            total_volume: 1_800_000.0,
            net_volume: 1_000_000.0,
            observed_liquidation_notional: None,
            squeeze_risk_proxy: Some(82.0),
            data_quality: Some(88.0),
        },
        "BTC-PERP",
        9_500,
        10_000,
    );

    assert_eq!(metrics.agg_buy_volume, 1_400_000.0);
    assert_eq!(metrics.agg_sell_volume, 400_000.0);
    assert_eq!(metrics.liquidation_pressure, 82.0);
    assert_eq!(
        metrics.lineage.provenance,
        MetricProvenance::CalculatedFromObserved
    );
    assert!(!metrics.liquidation_lineage.alert_eligible);
    assert!(metrics.lineage.alert_eligible);
    assert!(
        metrics.risk_score >= 40,
        "real CWM units must contribute to risk"
    );
}

#[test]
fn observed_perp_without_price_change_cannot_infer_direction_or_be_alert_eligible() {
    let metrics = build_perp_tof_metrics_from_observed(
        &ObservedPerpSnapshot {
            symbol: "BTC-PERP".to_string(),
            observed_at_ms: 10_000,
            price_change_bps: None,
            oi_change: Some(140_000.0),
            funding_rate: Some(-0.06),
            total_volume: 1_800_000.0,
            net_volume: 1_000_000.0,
            observed_liquidation_notional: None,
            squeeze_risk_proxy: None,
            data_quality: Some(88.0),
        },
        "BTC-PERP",
        9_500,
        10_000,
    );

    assert_eq!(metrics.metrics_direction, TofDirection::Neutral);
    assert!(!metrics.lineage.alert_eligible);
    assert_eq!(metrics.risk_score, 0);
}

#[test]
fn observed_perp_beyond_candidate_future_skew_is_unavailable() {
    let metrics = build_perp_tof_metrics_from_observed(
        &ObservedPerpSnapshot {
            symbol: "BTC-PERP".to_string(),
            observed_at_ms: 15_001,
            price_change_bps: Some(25.0),
            oi_change: Some(0.40),
            funding_rate: Some(-0.0002),
            total_volume: 1_800_000.0,
            net_volume: 1_000_000.0,
            observed_liquidation_notional: None,
            squeeze_risk_proxy: Some(82.0),
            data_quality: Some(88.0),
        },
        "BTC-PERP",
        10_000,
        15_001,
    );

    assert_eq!(metrics.lineage.provenance, MetricProvenance::Unavailable);
    assert!(!metrics.lineage.alert_eligible);
    assert_eq!(
        metrics.lineage.unavailable_reason.as_deref(),
        Some("observed_perp_stale")
    );
    assert_eq!(metrics.metrics_direction, TofDirection::Neutral);
    assert_eq!(metrics.risk_score, 0);
}

#[test]
fn future_candidate_cannot_bind_current_perp_observation() {
    let metrics = build_perp_tof_metrics_from_observed(
        &ObservedPerpSnapshot {
            symbol: "BTC-PERP".to_string(),
            observed_at_ms: 10_000,
            price_change_bps: Some(25.0),
            oi_change: Some(0.40),
            funding_rate: Some(-0.0002),
            total_volume: 1_800_000.0,
            net_volume: 1_000_000.0,
            observed_liquidation_notional: None,
            squeeze_risk_proxy: Some(82.0),
            data_quality: Some(88.0),
        },
        "BTC-PERP",
        15_001,
        10_000,
    );

    assert_eq!(metrics.lineage.provenance, MetricProvenance::Unavailable);
    assert!(!metrics.lineage.alert_eligible);
    assert_eq!(
        metrics.lineage.unavailable_reason.as_deref(),
        Some("observed_perp_stale")
    );
    assert_eq!(metrics.metrics_direction, TofDirection::Neutral);
    assert_eq!(metrics.risk_score, 0);
}

fn perp_metrics(candidate_type: &str, direction: TofDirection, risk_score: u8) -> PerpTofMetrics {
    PerpTofMetrics {
        oi_change: 140_000.0,
        oi_direction: "long_increase".to_string(),
        funding_rate: -0.06,
        funding_side: "short".to_string(),
        liquidation_pressure: 82.0,
        squeeze_side: "short".to_string(),
        agg_buy_volume: 1_400_000.0,
        agg_sell_volume: 400_000.0,
        direction_bias: direction,
        metrics_direction: direction,
        risk_score,
        data_quality: 88.0,
        candidate_type: candidate_type.to_string(),
        explain_tags: vec!["OI long increase".to_string()],
        confidence: 86.0,
        observed_liquidation_notional: None,
        lineage: MetricLineage::calculated("test_perp", 10_000, true),
        liquidation_lineage: MetricLineage::inferred("test_proxy", 10_000),
    }
}
