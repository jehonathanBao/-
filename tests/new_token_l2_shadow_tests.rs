use btc_toxic_flow_monitor_rs::toxic_v3::new_token_watch::{
    intent::{IntentAssessment, IntentState},
    shadow::{evaluate_shadow_outcome, ShadowOutcomeLabel},
};

fn assessment(state: IntentState, available: bool) -> IntentAssessment {
    IntentAssessment {
        state,
        confidence: 0.8,
        intent_assessment_available: available,
        reason: "test".to_string(),
        evidence: vec![],
        read_only: true,
    }
}

#[test]
fn shadow_evaluation_is_read_only_and_never_promotes_discord() {
    let outcome = evaluate_shadow_outcome(&assessment(IntentState::BidPressure, true), 25.0);

    assert_eq!(outcome.label, ShadowOutcomeLabel::Aligned);
    assert!(outcome.shadow_only);
    assert!(!outcome.discord_eligible);
    assert!(!outcome.execution_enabled);
}

#[test]
fn unavailable_intent_remains_insufficient_even_if_price_moves() {
    let outcome = evaluate_shadow_outcome(&assessment(IntentState::Unavailable, false), -50.0);

    assert_eq!(outcome.label, ShadowOutcomeLabel::InsufficientEvidence);
}
