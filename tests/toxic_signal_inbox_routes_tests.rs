use std::{fs, path::Path};

#[test]
fn signal_inbox_contract_includes_discord_alert_decision_fields() {
    let root = Path::new(env!("CARGO_MANIFEST_DIR"));
    let source = fs::read_to_string(root.join("src/api/toxic_signal_inbox_routes.rs"))
        .expect("inbox source");

    assert!(source.contains("\"alertStatus\""));
    assert!(source.contains("\"alertReason\""));
    assert!(source.contains("\"discordAlert\""));
    assert!(source.contains("\"finalRiskScore\""));
    assert!(source.contains("\"riskScore\""));
    assert!(source.contains("\"perpTofMetrics\""));
    assert!(source.contains("\"perpScore\""));
    assert!(source.contains("\"perpCandidateType\""));
    assert!(source.contains("\"finalCandidateType\""));
    assert!(source.contains("\"metricsDirection\""));
    assert!(source.contains("\"advancedTofMetrics\""));
    assert!(source.contains("\"advancedScore\""));
    assert!(source.contains("\"advancedCandidateType\""));
    assert!(source.contains("\"cwmContribution\""));
    assert!(source.contains("evaluate_discord_alert_gate"));
    assert!(source.contains("build_perp_tof_metrics"));
    assert!(source.contains("build_advanced_tof_metrics"));
    assert!(source.contains("build_cwm_risk_contribution"));
    assert!(source.contains("fused_risk_score_with_cwm"));
    assert!(source.contains("latest_cwm_signal_for_state"));
}
