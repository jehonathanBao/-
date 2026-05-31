use std::collections::BTreeMap;

use btc_toxic_flow_monitor_rs::{
    replay::replay_report::{ReplayMarkerOutcome, ReplayReport},
    types::toxic::{ToxicDirection, ToxicEvent, ToxicSeverity},
};

#[test]
fn report_contains_expected_sections_and_writes() {
    let report = ReplayReport {
        input_path: "fixtures/sample-toxic.jsonl".to_string(),
        event_count: 4,
        trade_count: 2,
        book_count: 2,
        detected_events: vec![sample_event()],
        threshold_buckets: BTreeMap::from([
            (">=300 BTC".to_string(), 1),
            (">=600 BTC".to_string(), 1),
            (">=1000 BTC".to_string(), 1),
            (">=2000 BTC".to_string(), 0),
        ]),
        reason_code_frequency: BTreeMap::from([
            ("large_aggressive_flow".to_string(), 1),
            ("threshold_crossed".to_string(), 1),
        ]),
        markers: ReplayMarkerOutcome {
            matched: 1,
            missed: 0,
            unexpected: 0,
        },
        vpin_summary: None,
        liquidation_summary: None,
        liq_hunt_summary: None,
    };

    let markdown = report.to_markdown();
    assert!(markdown.contains("BTC Toxic Flow Replay Report"));
    assert!(markdown.contains("Threshold Buckets:"));
    assert!(markdown.contains("Reason Code Frequency:"));

    let dir = std::env::temp_dir().join(format!(
        "btc_toxic_flow_monitor_rs_replay_report_{}_{}",
        std::process::id(),
        chrono::Utc::now().timestamp_nanos_opt().expect("nanos")
    ));
    let path = report.write_to_dir(&dir).expect("write");
    assert!(path.exists());
}

fn sample_event() -> ToxicEvent {
    ToxicEvent {
        id: "event-1".to_string(),
        ts: 1_000,
        symbol: "BTC-PERP".to_string(),
        direction: ToxicDirection::Buy,
        severity: ToxicSeverity::Alert,
        toxic_volume_btc: 1_284.2,
        threshold_btc: 1_000.0,
        window_ms: 5_000,
        leader_venue: None,
        aggressive_buy_btc: 1_500.0,
        aggressive_sell_btc: 200.0,
        net_aggressive_btc: 1_300.0,
        abs_aggressive_btc: 1_700.0,
        markout_1s_bps: Some(2.1),
        markout_5s_bps: Some(4.8),
        sweep_detected: true,
        liquidity_thin: true,
        liquidity: None,
        cross_venue_confirmed: true,
        vpin_enabled: true,
        vpin: Some(0.82),
        vpin_zscore: Some(2.8),
        vpin_spike: true,
        vpin_high: false,
        vpin_extreme: false,
        liquidation_enabled: true,
        nearest_cluster_side: Some(
            btc_toxic_flow_monitor_rs::types::liquidation::LiquidationClusterSide::ShortAbove,
        ),
        cluster_distance_bps: Some(10.0),
        cluster_notional_usd: Some(2_000_000.0),
        cluster_density: Some(0.6),
        liq_hunt_pressure: 0.75,
        liq_cluster_nearby: true,
        possible_liq_hunt_setup: true,
        reason_codes: vec!["large_aggressive_flow".to_string()],
    }
}
