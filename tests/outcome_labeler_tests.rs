use std::fs;

use btc_toxic_flow_monitor_rs::{
    calibration::{calibration_types::OutcomeLabel, outcome_labeler::OutcomeLabeler},
    types::{
        liquidation::LiquidationClusterSide,
        toxic::{ToxicDirection, ToxicEvent, ToxicSeverity},
    },
};

#[test]
fn outcome_labeler_marks_hit_false_positive_and_unknown() {
    let path = fixture_path();
    let labeler = OutcomeLabeler::from_replay_file(path.to_str().expect("utf8")).expect("labeler");

    let hit = labeler.label_event(&sample_event(5_100, ToxicDirection::Buy));
    assert_eq!(hit.label, OutcomeLabel::Hit);

    let false_positive = labeler.label_event(&sample_event(3_500, ToxicDirection::Buy));
    assert!(matches!(
        false_positive.label,
        OutcomeLabel::FalsePositive | OutcomeLabel::Neutral
    ));

    let unknown = labeler.label_event(&sample_event(9_999_999, ToxicDirection::Buy));
    assert_eq!(unknown.label, OutcomeLabel::Unknown);
}

fn sample_event(ts: i64, direction: ToxicDirection) -> ToxicEvent {
    ToxicEvent {
        id: format!("event-{ts}"),
        ts,
        symbol: "BTC-PERP".to_string(),
        direction,
        severity: ToxicSeverity::Alert,
        toxic_volume_btc: 1200.0,
        threshold_btc: 1000.0,
        window_ms: 5000,
        leader_venue: None,
        aggressive_buy_btc: 1200.0,
        aggressive_sell_btc: 100.0,
        net_aggressive_btc: 1100.0,
        abs_aggressive_btc: 1300.0,
        markout_1s_bps: Some(2.0),
        markout_5s_bps: Some(4.0),
        sweep_detected: true,
        liquidity_thin: true,
        liquidity: None,
        cross_venue_confirmed: true,
        vpin_enabled: true,
        vpin: Some(0.8),
        vpin_zscore: Some(2.5),
        vpin_spike: true,
        vpin_high: true,
        vpin_extreme: false,
        liquidation_enabled: true,
        nearest_cluster_side: Some(LiquidationClusterSide::ShortAbove),
        cluster_distance_bps: Some(10.0),
        cluster_notional_usd: Some(50_000_000.0),
        cluster_density: Some(0.7),
        liq_hunt_pressure: 0.8,
        liq_cluster_nearby: true,
        possible_liq_hunt_setup: true,
        reason_codes: vec!["large_aggressive_flow".to_string()],
    }
}

fn fixture_path() -> std::path::PathBuf {
    let path = std::env::temp_dir().join(format!(
        "btc_toxic_flow_monitor_rs_outcome_labeler_{}_{}.jsonl",
        std::process::id(),
        chrono::Utc::now().timestamp_nanos_opt().expect("nanos")
    ));
    fs::write(
        &path,
        include_str!("../fixtures/sample-liquidation-hunt.jsonl"),
    )
    .expect("fixture");
    path
}
