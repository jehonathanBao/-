use btc_toxic_flow_monitor_rs::signal_semantics::{
    classify_signal_semantic, SignalSemanticInput, SignalSemanticTier,
};

fn semantic_input() -> SignalSemanticInput {
    SignalSemanticInput {
        severity_rank: 2,
        score: 85,
        confidence: Some(70.0),
        data_quality: 80,
        consistency_confirmed: true,
        strong_price_response: true,
        multi_window_aligned: true,
        multi_exchange_confirmed: true,
        has_price_response: true,
    }
}

#[test]
fn semantic_classifier_keeps_medium_as_observe() {
    let mut input = semantic_input();
    input.severity_rank = 1;
    input.score = 95;
    input.confidence = Some(95.0);
    input.consistency_confirmed = true;

    assert_eq!(classify_signal_semantic(input), SignalSemanticTier::Observe);
}

#[test]
fn semantic_classifier_promotes_alert_without_execution_confirmations() {
    let mut input = semantic_input();
    input.score = 85;
    input.strong_price_response = false;
    input.multi_window_aligned = false;

    assert_eq!(classify_signal_semantic(input), SignalSemanticTier::Alert);
}

#[test]
fn semantic_classifier_requires_confirmation_for_execution() {
    let mut input = semantic_input();
    input.score = 90;
    input.strong_price_response = true;
    input.multi_window_aligned = true;
    input.multi_exchange_confirmed = true;

    assert_eq!(
        classify_signal_semantic(input),
        SignalSemanticTier::Execution
    );
}
