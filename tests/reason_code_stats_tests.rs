use btc_toxic_flow_monitor_rs::{
    calibration::{
        calibration_types::{EventOutcome, OutcomeLabel},
        reason_code_stats::build_reason_code_stats,
    },
    types::{
        liquidation::LiquidationClusterSide,
        toxic::{ToxicDirection, ToxicEvent, ToxicSeverity},
    },
};

#[test]
fn reason_code_stats_aggregate_hits_and_false_positives() {
    let outcomes = vec![
        sample_outcome("large_aggressive_flow", OutcomeLabel::Hit),
        sample_outcome("large_aggressive_flow", OutcomeLabel::FalsePositive),
        sample_outcome("vpin_spike", OutcomeLabel::Hit),
    ];
    let stats = build_reason_code_stats(&outcomes);
    let large = stats
        .iter()
        .find(|stat| stat.reason_code == "large_aggressive_flow")
        .expect("large");
    assert_eq!(large.total_count, 2);
    assert_eq!(large.hit_count, 1);
    assert_eq!(large.false_positive_count, 1);
}

fn sample_outcome(reason: &str, label: OutcomeLabel) -> EventOutcome {
    EventOutcome {
        event: ToxicEvent {
            id: format!("event-{reason}"),
            ts: 1,
            symbol: "BTC-PERP".to_string(),
            direction: ToxicDirection::Buy,
            severity: ToxicSeverity::Alert,
            toxic_volume_btc: 1000.0,
            threshold_btc: 1000.0,
            window_ms: 5000,
            leader_venue: None,
            aggressive_buy_btc: 1000.0,
            aggressive_sell_btc: 100.0,
            net_aggressive_btc: 900.0,
            abs_aggressive_btc: 1100.0,
            markout_1s_bps: None,
            markout_5s_bps: None,
            sweep_detected: false,
            liquidity_thin: false,
            liquidity: None,
            cross_venue_confirmed: true,
            vpin_enabled: false,
            vpin: None,
            vpin_zscore: None,
            vpin_spike: false,
            vpin_high: false,
            vpin_extreme: false,
            liquidation_enabled: true,
            nearest_cluster_side: Some(LiquidationClusterSide::ShortAbove),
            cluster_distance_bps: None,
            cluster_notional_usd: None,
            cluster_density: None,
            liq_hunt_pressure: 0.0,
            liq_cluster_nearby: false,
            possible_liq_hunt_setup: false,
            reason_codes: vec![reason.to_string()],
        },
        current_mid: Some(100000.0),
        forward_1s_bps: None,
        forward_5s_bps: Some(2.0),
        forward_15s_bps: None,
        forward_60s_bps: None,
        primary_horizon_ms: Some(5000),
        primary_move_bps: Some(2.0),
        label,
    }
}
