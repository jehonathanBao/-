use btc_toxic_flow_monitor_rs::contract_whale_monitor::shadow::{
    ShadowObservation, ShadowState, ShadowTracker,
};

fn observation(ts: i64) -> ShadowObservation {
    ShadowObservation {
        symbol: "BTC".to_string(),
        ts,
        total_volume_btc: 500.0,
        net_volume_btc: 320.0,
        high_threshold_btc: 1_500.0,
        price_move_pct: Some(0.03),
        oi_change_pct: Some(0.18),
        data_quality: 85,
        multi_exchange_confirmed: true,
        live_liquidation_btc: 0.0,
        trade_count: 40,
    }
}

#[test]
fn persistent_sub_high_flow_progresses_to_corroborated_shadow() {
    let mut tracker = ShadowTracker::default();
    assert_eq!(
        tracker.observe(observation(1_700_000_000_000)).state,
        ShadowState::Suspect
    );
    assert_eq!(
        tracker.observe(observation(1_700_000_030_000)).state,
        ShadowState::Watching
    );
    assert_eq!(
        tracker.observe(observation(1_700_000_060_000)).state,
        ShadowState::Corroborated
    );
}

#[test]
fn one_pulse_does_not_create_shadow_and_missing_oi_cannot_corroborate() {
    let mut tracker = ShadowTracker::default();
    let mut pulse = observation(1_700_000_000_000);
    pulse.total_volume_btc = 2_000.0;
    assert_eq!(tracker.observe(pulse).state, ShadowState::Invalidated);

    let mut no_oi = observation(1_700_000_030_000);
    no_oi.oi_change_pct = None;
    assert_eq!(tracker.observe(no_oi).state, ShadowState::Watching);
    assert_ne!(
        tracker.observe(observation(1_700_000_060_000)).state,
        ShadowState::Corroborated
    );
}

#[test]
fn stale_gap_invalidates_shadow_episode() {
    let mut tracker = ShadowTracker::default();
    tracker.observe(observation(1_700_000_000_000));
    let result = tracker.observe(observation(1_700_000_300_000));
    assert_eq!(result.state, ShadowState::Invalidated);
    assert!(result.invalidation_reason.is_some());
}
