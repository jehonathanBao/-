use std::path::PathBuf;

use btc_toxic_flow_monitor_rs::replay::{
    production_report::{run_production_replay, write_production_report},
    replay_config::{
        AlertGateConfig, MarkoutConfig, ProductionReplayConfig, ReplayInputConfig,
        ReplayOutputConfig, ReplayRuntimeConfig,
    },
};

#[test]
fn production_replay_fixture_writes_report_artifacts() {
    let config = fixture_config("spoofing_candidate_basic.jsonl");
    let report = run_production_replay(&config).expect("production replay fixture");

    assert!(report.read_only);
    assert!(report.total_events > 0);
    assert!(report.total_signals > 0);
    assert!(!report.high_score_candidates.is_empty());
    assert!(report
        .calibration
        .detector_average_markout_5s_bps
        .contains_key("SpoofingCandidate"));

    let output = write_production_report(&report, &config).expect("write report");

    assert!(output.report_dir.exists());
    assert!(output.summary_json.expect("summary").exists());
    assert!(output.signals_json.expect("signals").exists());
    assert!(output.calibration_json.expect("calibration").exists());
    assert!(output.calibration_md.expect("markdown").exists());
    assert!(output
        .high_score_candidates_csv
        .expect("high score csv")
        .exists());
}

fn fixture_config(name: &str) -> ProductionReplayConfig {
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    ProductionReplayConfig {
        input: ReplayInputConfig {
            path: root.join("fixtures").join("replay").join(name),
            format: "auto".to_string(),
            venue: Some("Binance".to_string()),
            symbol: Some("BTC-PERP".to_string()),
            timezone: "UTC".to_string(),
        },
        replay: ReplayRuntimeConfig {
            sort_by_ts: true,
            max_events: 0,
            start_ts_ms: 0,
            end_ts_ms: 0,
        },
        markout: MarkoutConfig {
            horizons_ms: vec![1_000, 5_000, 30_000],
        },
        alert_gate: AlertGateConfig {
            min_score: 80,
            min_data_quality: 70.0,
        },
        output: ReplayOutputConfig {
            report_dir: std::env::temp_dir().join(format!(
                "btc-toxic-flow-production-replay-test-{}",
                std::process::id()
            )),
            write_json: true,
            write_markdown: true,
            write_csv: true,
        },
    }
}
