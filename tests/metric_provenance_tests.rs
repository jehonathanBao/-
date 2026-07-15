use btc_toxic_flow_monitor_rs::runtime::metric_provenance::{MetricLineage, MetricProvenance};

#[test]
fn provenance_serializes_with_stable_snake_case_values() {
    let cases = [
        (MetricProvenance::Observed, "observed"),
        (
            MetricProvenance::CalculatedFromObserved,
            "calculated_from_observed",
        ),
        (MetricProvenance::Inferred, "inferred"),
        (MetricProvenance::Unavailable, "unavailable"),
    ];

    for (provenance, expected) in cases {
        assert_eq!(
            serde_json::to_string(&provenance).unwrap(),
            format!("\"{expected}\"")
        );
        assert_eq!(
            serde_json::from_str::<MetricProvenance>(&format!("\"{expected}\"")).unwrap(),
            provenance
        );
    }
}

#[test]
fn missing_lineage_fails_closed() {
    let lineage = MetricLineage::default();

    assert_eq!(lineage.provenance, MetricProvenance::Unavailable);
    assert!(!lineage.available);
    assert!(!lineage.fresh);
    assert!(!lineage.alert_eligible);
    assert!(lineage.observed_at_ms.is_none());
    assert_eq!(
        lineage.unavailable_reason.as_deref(),
        Some("source_unavailable")
    );
}

#[test]
fn constructors_only_make_fresh_observed_evidence_alert_eligible() {
    let observed = MetricLineage::observed("flow_window_service", 1_000, true);
    let stale = MetricLineage::observed("flow_window_service", 1_000, false);
    let calculated = MetricLineage::calculated("tof_observed_formula_v1", 1_000, true);
    let inferred = MetricLineage::inferred("legacy_summary_compatibility", 1_000);

    assert!(observed.alert_eligible);
    assert!(!stale.alert_eligible);
    assert!(calculated.alert_eligible);
    assert!(!inferred.alert_eligible);
}
