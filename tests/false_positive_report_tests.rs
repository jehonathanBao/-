use btc_toxic_flow_monitor_rs::{
    calibration::{
        calibration_types::{EventOutcome, OutcomeLabel},
        false_positive_report::{top_false_positives, top_hits},
    },
    types::{
        liquidation::LiquidationClusterSide,
        toxic::{ToxicDirection, ToxicEvent, ToxicSeverity},
    },
};

#[test]
fn false_positive_helpers_sort_expected_events() {
    let false_positive = sample_outcome(1, -4.0, OutcomeLabel::FalsePositive);
    let hit = sample_outcome(2, 5.0, OutcomeLabel::Hit);
    let mild_hit = sample_outcome(3, 1.0, OutcomeLabel::Hit);
    let outcomes = vec![mild_hit.clone(), hit.clone(), false_positive.clone()];

    let top_fp = top_false_positives(&outcomes, 5);
    assert_eq!(top_fp[0].event.id, false_positive.event.id);

    let top_hit = top_hits(&outcomes, 5);
    assert_eq!(top_hit[0].event.id, hit.event.id);
}

fn sample_outcome(ts: i64, primary_move_bps: f64, label: OutcomeLabel) -> EventOutcome {
    EventOutcome {
        event: ToxicEvent {
            id: format!("event-{ts}"),
            ts,
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
            reason_codes: vec![],
        },
        current_mid: Some(100000.0),
        forward_1s_bps: None,
        forward_5s_bps: Some(primary_move_bps),
        forward_15s_bps: None,
        forward_60s_bps: None,
        primary_horizon_ms: Some(5000),
        primary_move_bps: Some(primary_move_bps),
        label,
    }
}
