use std::path::PathBuf;

use btc_toxic_flow_monitor_rs::{
    replay::candidate_replay_runner::run_candidate_replay_file,
    types::toxic_signal::ToxicSignalType,
};

#[test]
fn spoofing_fixture_triggers_spoofing_candidate() {
    let summary = run_fixture("spoofing_candidate_basic.jsonl");

    assert!(summary
        .signals
        .iter()
        .any(|signal| signal.signal_type == ToxicSignalType::SpoofingCandidate));
    assert!(summary.signals[0].evidence.is_some());
}

#[test]
fn csv_spoofing_fixture_triggers_spoofing_candidate() {
    let summary = run_fixture("spoofing_candidate_basic.csv");

    assert!(summary
        .signals
        .iter()
        .any(|signal| signal.signal_type == ToxicSignalType::SpoofingCandidate));
}

#[test]
fn layering_fixture_triggers_layering_candidate() {
    let summary = run_fixture("layering_candidate_basic.jsonl");

    assert!(summary
        .signals
        .iter()
        .any(|signal| signal.signal_type == ToxicSignalType::LayeringCandidate));
}

#[test]
fn iceberg_fixture_triggers_iceberg_candidate() {
    let summary = run_fixture("iceberg_candidate_basic.jsonl");

    assert!(summary
        .signals
        .iter()
        .any(|signal| signal.signal_type == ToxicSignalType::IcebergCandidate));
}

#[test]
fn snapshot_reset_fixture_does_not_trigger_cancel_evidence_candidate() {
    let summary = run_fixture("snapshot_reset_negative.jsonl");

    assert!(summary.signals.is_empty());
}

#[test]
fn normal_market_fixture_does_not_trigger_high_risk_signal() {
    let summary = run_fixture("normal_market_negative.jsonl");

    assert!(summary
        .signals
        .iter()
        .all(|signal| signal.toxicity_score < 80));
}

#[test]
fn replay_summary_reports_dedupe_and_grouping() {
    let summary = run_fixture("layering_candidate_basic.jsonl");

    assert!(summary.total_events > 0);
    assert!(summary.total_signals > 0);
    assert!(summary.deduped_count > 0);
    assert!(summary.signals_by_symbol.contains_key("BTC-PERP"));
}

fn run_fixture(
    name: &str,
) -> btc_toxic_flow_monitor_rs::replay::candidate_replay_runner::CandidateReplaySummary {
    run_candidate_replay_file(fixture_path(name)).expect("candidate replay")
}

fn fixture_path(name: &str) -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("fixtures")
        .join("replay")
        .join(name)
}
