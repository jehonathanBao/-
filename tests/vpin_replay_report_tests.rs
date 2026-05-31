use std::collections::BTreeMap;

use btc_toxic_flow_monitor_rs::{
    replay::{
        replay_report::{ReplayMarkerOutcome, ReplayReport},
        vpin_replay_report::VpinReplaySummary,
    },
    types::{
        toxic::{ToxicDirection, ToxicEvent, ToxicSeverity},
        vpin::{VpinBucket, VpinDirection},
    },
};

#[test]
fn replay_report_includes_vpin_summary() {
    let report = ReplayReport {
        input_path: "fixtures/sample-toxic.jsonl".to_string(),
        event_count: 4,
        trade_count: 2,
        book_count: 2,
        detected_events: vec![sample_event()],
        threshold_buckets: BTreeMap::from([(">=1000 BTC".to_string(), 1)]),
        reason_code_frequency: BTreeMap::from([("threshold_crossed".to_string(), 1)]),
        markers: ReplayMarkerOutcome {
            matched: 1,
            missed: 0,
            unexpected: 0,
        },
        vpin_summary: Some(VpinReplaySummary {
            bucket_size_btc: 100.0,
            lookback_buckets: 50,
            completed_buckets: 12,
            max_vpin: Some(0.82),
            max_vpin_zscore: Some(2.8),
            vpin_high_count: 3,
            vpin_spike_count: 2,
            vpin_extreme_count: 1,
            top_buckets: vec![sample_bucket()],
            dominant_direction: VpinDirection::Buy,
        }),
        liquidation_summary: None,
        liq_hunt_summary: None,
    };

    let markdown = report.to_markdown();
    assert!(markdown.contains("## VPIN Summary"));
    assert!(markdown.contains("Completed buckets: 12"));
    assert!(markdown.contains("Max VPIN: Some(0.82)"));
    assert!(markdown.contains("Max VPIN z-score: Some(2.8)"));
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
        cluster_distance_bps: Some(9.0),
        cluster_notional_usd: Some(1_800_000.0),
        cluster_density: Some(0.58),
        liq_hunt_pressure: 0.72,
        liq_cluster_nearby: true,
        possible_liq_hunt_setup: true,
        reason_codes: vec!["threshold_crossed".to_string()],
    }
}

fn sample_bucket() -> VpinBucket {
    VpinBucket {
        id: 1,
        symbol: "BTC-PERP".to_string(),
        start_ts: 1,
        end_ts: 2,
        bucket_size_btc: 100.0,
        total_btc: 100.0,
        buy_btc: 90.0,
        sell_btc: 10.0,
        net_btc: 80.0,
        imbalance_btc: 80.0,
        imbalance_ratio: 0.8,
        direction: VpinDirection::Buy,
        venue_breakdown: BTreeMap::new(),
    }
}
