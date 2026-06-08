use std::{path::PathBuf, process::Command};

use btc_toxic_flow_monitor_rs::contract_whale_monitor::replay::{
    format_contract_whale_replay_report, run_contract_whale_replay,
};
use serde::Deserialize;

#[derive(Debug, Deserialize)]
struct ReplayManifest {
    samples: Vec<ExpectedReplaySample>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct ExpectedReplaySample {
    file: String,
    expected_signal_type: Option<String>,
    expected_severity: String,
    expected_discord_eligible: bool,
    #[serde(default)]
    expected_liquidation_suspected: bool,
}

#[test]
fn cwm_replay_standard_samples_match_expected_classification() {
    let manifest = read_manifest();
    let replay_dir = replay_dir();

    for sample in manifest.samples {
        let path = replay_dir.join(&sample.file);
        let report = run_contract_whale_replay(&path).expect("run cwm replay sample");
        let first_output = format_contract_whale_replay_report(&report);
        let second_output =
            format_contract_whale_replay_report(&run_contract_whale_replay(&path).unwrap());
        assert_eq!(
            first_output, second_output,
            "{} is not deterministic",
            sample.file
        );

        if sample.expected_severity == "none" {
            assert_eq!(
                report.signals_generated, 0,
                "{} should be quiet",
                sample.file
            );
            assert_eq!(report.discord_eligible_count, 0);
            continue;
        }

        assert_eq!(
            report.signals_generated, 1,
            "{} should merge to one representative signal",
            sample.file
        );
        let signal = &report.signals[0];
        assert_eq!(
            Some(signal.signal_type.as_str()),
            sample.expected_signal_type.as_deref(),
            "{} signal type changed",
            sample.file
        );
        assert_eq!(
            signal.severity, sample.expected_severity,
            "{} severity changed",
            sample.file
        );
        assert_eq!(
            signal.discord_eligible, sample.expected_discord_eligible,
            "{} discord gate changed",
            sample.file
        );
        assert_eq!(
            signal.liquidation_suspected, sample.expected_liquidation_suspected,
            "{} liquidation flag changed",
            sample.file
        );
    }
}

#[test]
fn cwm_replay_bin_prints_human_and_json_reports() {
    let sample = replay_dir().join("btc_trades_sample.jsonl");
    let output = Command::new(env!("CARGO_BIN_EXE_cwm_replay"))
        .arg("--input")
        .arg(&sample)
        .output()
        .expect("run cwm_replay");
    assert!(
        output.status.success(),
        "cwm_replay failed: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    let stdout = String::from_utf8(output.stdout).expect("utf8 output");
    assert!(stdout.contains("CWM Replay Report"));
    assert!(stdout.contains("signals generated: 1"));
    assert!(stdout.contains("discord eligible count: 1"));

    let json_output = Command::new(env!("CARGO_BIN_EXE_cwm_replay"))
        .arg("--input")
        .arg(&sample)
        .arg("--json")
        .output()
        .expect("run cwm_replay json");
    assert!(
        json_output.status.success(),
        "cwm_replay --json failed: {}",
        String::from_utf8_lossy(&json_output.stderr)
    );
    let payload: serde_json::Value =
        serde_json::from_slice(&json_output.stdout).expect("json replay report");
    assert_eq!(payload["signalsGenerated"], 1);
    assert_eq!(payload["severityDistribution"]["s"], 1);
}

fn read_manifest() -> ReplayManifest {
    let text = std::fs::read_to_string(replay_dir().join("cwm_samples_manifest.json"))
        .expect("read replay manifest");
    serde_json::from_str(&text).expect("parse replay manifest")
}

fn replay_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("data/replay")
}
