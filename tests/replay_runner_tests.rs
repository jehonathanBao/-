use std::fs;

use btc_toxic_flow_monitor_rs::{
    config::venues::{VenueConfig, VenueConfigs},
    config::AppConfig,
    replay::replay_runner::ReplayRunner,
    types::{market::Venue, toxic::ToxicSeverity},
};

#[test]
fn replay_runner_detects_toxic_event_and_matches_marker() {
    let fixture = fixture_path();
    let mut runner = ReplayRunner::new(test_config());
    let report = runner
        .run_file(fixture.to_str().expect("utf8 path"))
        .expect("run");

    assert!(!report.detected_events.is_empty());
    assert!(report
        .detected_events
        .iter()
        .any(|event| event.toxic_volume_btc >= 1_000.0));
    assert!(report.markers.missed == 0);
    assert!(report.markers.matched >= 1);
}

#[test]
fn replay_report_writes_markdown() {
    let fixture = fixture_path();
    let mut runner = ReplayRunner::new(test_config());
    let report = runner
        .run_file(fixture.to_str().expect("utf8 path"))
        .expect("run");
    let dir = temp_dir("replay_report");
    let path = report.write_to_dir(&dir).expect("write");
    let markdown = fs::read_to_string(path).expect("read markdown");

    assert!(markdown.contains("# BTC Toxic Flow Replay Report"));
    assert!(markdown.contains("Threshold Buckets:"));
    assert!(markdown.contains("Reason Code Frequency:"));
}

#[test]
fn replay_runner_scopes_non_btc_books_prices_sweeps_and_events_to_configured_symbol() {
    let fixture = fixture_path();
    let mut config = test_config();
    config.symbol = "ETH-PERP".to_string();
    let mut runner = ReplayRunner::new(config);
    let report = runner
        .run_file(fixture.to_str().expect("utf8 path"))
        .expect("run ETH replay");

    assert!(!report.detected_events.is_empty());
    assert!(report
        .detected_events
        .iter()
        .all(|event| event.symbol == "ETH-PERP"));
    assert!(report.detected_events.iter().any(|event| {
        event.sweep_detected
            && event
                .liquidity
                .as_ref()
                .is_some_and(|liquidity| liquidity.symbol == "ETH-PERP")
    }));
    assert!(report
        .detected_events
        .iter()
        .any(|event| event.markout_1s_bps.is_some() || event.markout_5s_bps.is_some()));
    assert!(report
        .liquidation_summary
        .as_ref()
        .is_some_and(|summary| summary
            .evidence
            .iter()
            .all(|evidence| evidence.symbol == "ETH-PERP")));
}

fn fixture_path() -> std::path::PathBuf {
    let path = temp_dir("replay_fixture").join("sample-toxic.jsonl");
    fs::create_dir_all(path.parent().expect("parent")).expect("mkdirs");
    fs::write(&path, include_str!("../fixtures/sample-toxic.jsonl")).expect("write fixture");
    path
}

fn temp_dir(name: &str) -> std::path::PathBuf {
    std::env::temp_dir().join(format!(
        "btc_toxic_flow_monitor_rs_{}_{}_{}",
        name,
        std::process::id(),
        chrono::Utc::now().timestamp_nanos_opt().expect("nanos")
    ))
}

fn test_config() -> AppConfig {
    btc_toxic_flow_monitor_rs::config::AppConfig {
        app_env: "test".to_string(),
        read_only: true,
        api_host: "127.0.0.1".parse().expect("valid ip"),
        api_port: 0,
        symbol: "BTC-PERP".to_string(),
        toxic_volume_alert_btc: 1000.0,
        windows_ms: vec![1000, 5000, 15000, 60000],
        markout_horizons_ms: vec![1000, 5000, 15000],
        sweep_windows_ms: vec![1000, 5000, 15000],
        venues: VenueConfigs {
            binance: VenueConfig {
                venue: Venue::Binance,
                enabled: false,
            },
            bybit: VenueConfig {
                venue: Venue::Bybit,
                enabled: false,
            },
            okx: VenueConfig {
                venue: Venue::Okx,
                enabled: false,
            },
        },
        flow_compute_interval_ms: 50,
        markout_resolve_interval_ms: 50,
        sweep_compute_interval_ms: 50,
        toxic_compute_interval_ms: 50,
        telegram_enabled: false,
        telegram_bot_token: String::new(),
        telegram_chat_id: String::new(),
        alert_dedup_window_ms: 30_000,
        alert_min_severity: ToxicSeverity::Alert,
        alert_require_cross_venue: true,
        alert_require_markout: true,
        alert_require_liquidity_drain: false,
        sqlite_enabled: false,
        sqlite_path: ".runtime/test.sqlite".to_string(),
        snapshot_persist_interval_ms: 1000,
        raw_snapshot_enabled: false,
        raw_snapshot_sample_rate_ms: 1000,
        replay_enabled: true,
        replay_report_dir: ".runtime/reports".to_string(),
        vpin_enabled: true,
        vpin_bucket_size_btc: 100.0,
        vpin_lookback_buckets: 50,
        vpin_min_buckets: 10,
        vpin_spike_zscore: 2.5,
        vpin_high_threshold: 0.70,
        vpin_extreme_threshold: 0.85,
        vpin_persist_buckets: true,
        liquidation_enabled: true,
        liquidation_lookback_ms: 120_000,
        liquidation_cluster_band_bps: 6.0,
        liquidation_min_cluster_distance_bps: 5.0,
        liquidation_max_cluster_distance_bps: 150.0,
        liquidation_proximity_threshold_bps: 25.0,
        liquidation_min_cluster_touches: 3,
        liquidation_pressure_threshold: 0.65,
        liq_hunt_cluster_large_notional_usd: 50_000_000.0,
        liq_hunt_near_distance_bps: 25.0,
        liq_hunt_active_score: 75.0,
        liq_hunt_likely_score: 50.0,
        liq_hunt_watch_score: 30.0,
        book_stale_ms: 5000,
        max_buffer_age_ms: 120000,
        system_mode: Default::default(),
        contract_whale_monitor:
            btc_toxic_flow_monitor_rs::config::env::ContractWhaleMonitorConfig {
                enabled: false,
                dry_run: true,
            },
        spot_whale_monitor: btc_toxic_flow_monitor_rs::config::env::SpotWhaleMonitorConfig {
            enabled: false,
            dry_run: true,
        },
    }
}
