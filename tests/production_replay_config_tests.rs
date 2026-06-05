use std::path::PathBuf;

use btc_toxic_flow_monitor_rs::replay::{
    production_report::run_production_replay, replay_config::ProductionReplayConfig,
};

#[test]
fn production_replay_example_config_parses() {
    let config = example_config();

    assert_eq!(config.input.format, "auto");
    assert_eq!(config.input.venue.as_deref(), Some("Binance"));
    assert_eq!(config.input.symbol.as_deref(), Some("BTCUSDT"));
    assert_eq!(config.markout.horizons_ms, vec![1_000, 5_000, 30_000]);
}

#[test]
fn missing_real_input_returns_friendly_error_without_panic() {
    let config = example_config();
    let err = run_production_replay(&config).expect_err("missing real data should fail");
    let message = format!("{err:#}");

    assert!(message.contains("production replay input is unavailable"));
    assert!(message.contains("data/production_replay"));
}

#[test]
fn output_report_dir_is_configurable() {
    let config = example_config();

    assert!(config
        .output_root()
        .ends_with(PathBuf::from("data/production_replay/reports")));
}

#[test]
fn alert_gate_config_reads_expected_defaults() {
    let config = ProductionReplayConfig::default();

    assert_eq!(config.alert_gate.min_score, 80);
    assert_eq!(config.alert_gate.min_data_quality, 70.0);
}

fn example_config() -> ProductionReplayConfig {
    ProductionReplayConfig::from_file(
        PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("config")
            .join("replay.production.example.toml"),
    )
    .expect("example config should parse")
}
