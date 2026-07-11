use btc_toxic_flow_monitor_rs::toxic_v3::new_token_watch::{
    intent::{IntentAssessment, IntentState},
    shadow::{ShadowOutcomeLabel, ShadowOutcomeTracker},
};

fn bid_pressure() -> IntentAssessment {
    IntentAssessment {
        state: IntentState::BidPressure,
        confidence: 0.7,
        intent_assessment_available: true,
        reason: "l2_top_depth_imbalance".to_string(),
        evidence: vec!["public_l2_orderbook".to_string()],
        read_only: true,
    }
}

#[test]
fn shadow_tracker_records_each_due_horizon_without_execution_or_discord() {
    let mut tracker = ShadowOutcomeTracker::default();
    let intent = bid_pressure();
    tracker.observe_intent("ASTERUSDT", 1_000, 1.0, &intent);

    assert!(tracker.observe_price("ASTERUSDT", 10_999, 1.01).is_empty());
    let ten_second = tracker.observe_price("ASTERUSDT", 11_000, 1.01);
    assert_eq!(ten_second.len(), 1);
    assert_eq!(ten_second[0].horizon_seconds, 10);
    assert_eq!(ten_second[0].outcome.label, ShadowOutcomeLabel::Aligned);
    assert!(ten_second[0].outcome.shadow_only);
    assert!(!ten_second[0].outcome.discord_eligible);
    assert!(!ten_second[0].outcome.execution_enabled);

    assert_eq!(
        tracker.observe_price("ASTERUSDT", 31_000, 0.99)[0].horizon_seconds,
        30
    );
    assert_eq!(
        tracker.observe_price("ASTERUSDT", 121_000, 1.02)[0].horizon_seconds,
        120
    );
    assert_eq!(
        tracker.observe_price("ASTERUSDT", 301_000, 1.03)[0].horizon_seconds,
        300
    );
    assert!(tracker.observe_price("ASTERUSDT", 302_000, 1.04).is_empty());
}

#[test]
fn shadow_tracker_does_not_create_events_without_available_directional_intent() {
    let mut tracker = ShadowOutcomeTracker::default();
    tracker.observe_intent(
        "ASTERUSDT",
        1_000,
        1.0,
        &IntentAssessment {
            state: IntentState::Neutral,
            confidence: 0.0,
            intent_assessment_available: true,
            reason: "balanced_book".to_string(),
            evidence: vec![],
            read_only: true,
        },
    );
    assert!(tracker.observe_price("ASTERUSDT", 301_000, 1.1).is_empty());
}
