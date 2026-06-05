use std::path::PathBuf;

use btc_toxic_flow_monitor_rs::replay::{
    calibration_report::{build_calibration_report, run_candidate_calibration_file},
    candidate_replay_runner::run_candidate_replay_file,
};

#[test]
fn calibration_report_groups_score_buckets_and_markout() {
    let summary = run_candidate_replay_file(fixture_path("spoofing_candidate_basic.jsonl"))
        .expect("candidate replay");
    let report = build_calibration_report(&summary.signals);

    assert_eq!(report.score_buckets.len(), 5);
    assert!(
        report
            .score_buckets
            .iter()
            .map(|bucket| bucket.signal_count)
            .sum::<usize>()
            > 0
    );
    assert!(report
        .detector_average_markout_5s_bps
        .contains_key("SpoofingCandidate"));
    assert!(report.venue_average_data_quality.contains_key("binance"));
}

#[test]
fn replay_calibration_report_is_read_only_and_production_data_ready() {
    let report = run_candidate_calibration_file(fixture_path("spoofing_candidate_basic.csv"))
        .expect("candidate replay calibration");

    assert!(report.read_only);
    assert!(report.production_data_ready);
    assert!(report.summary.total_events > 0);
    assert_eq!(report.calibration.score_buckets.len(), 5);
}

fn fixture_path(name: &str) -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("fixtures")
        .join("replay")
        .join(name)
}
