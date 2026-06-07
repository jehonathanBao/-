use btc_toxic_flow_monitor_rs::runtime::perp_tof_metrics::classify_funding;

#[test]
fn funding_detects_crowded_long_and_short_candidates() {
    let (candidate, side, score) = classify_funding(0.08, 150_000.0, 0.05);
    assert_eq!(candidate, "CrowdedLongCandidate");
    assert_eq!(side, "long");
    assert!(score >= 80.0);

    let (candidate, side, score) = classify_funding(-0.08, 150_000.0, 0.05);
    assert_eq!(candidate, "CrowdedShortCandidate");
    assert_eq!(side, "short");
    assert!(score >= 80.0);
}

#[test]
fn funding_requires_meaningful_open_interest() {
    let (candidate, side, score) = classify_funding(0.08, 10_000.0, 0.05);

    assert_eq!(candidate, "FundingNeutralCandidate");
    assert_eq!(side, "neutral");
    assert_eq!(score, 0.0);
}
