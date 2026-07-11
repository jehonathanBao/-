use btc_toxic_flow_monitor_rs::toxic_v3::new_token_watch::{
    intent::{IntentFsm, IntentState},
    l2::{OrderBookMetrics, OrderBookReadiness},
};

fn ready_metrics(imbalance: f64) -> OrderBookMetrics {
    OrderBookMetrics {
        readiness: OrderBookReadiness::Ready,
        orderbook_evidence_available: true,
        imbalance,
        reason: "orderbook_ready".to_string(),
        ..Default::default()
    }
}

#[test]
fn fsm_requires_warmup_before_publishing_directional_intent() {
    let mut fsm = IntentFsm::default();

    let first = fsm.observe(&ready_metrics(0.65));
    let second = fsm.observe(&ready_metrics(0.65));

    assert_eq!(first.state, IntentState::Unavailable);
    assert_eq!(first.reason, "intent_warmup");
    assert_eq!(second.state, IntentState::BidPressure);
    assert!(second.intent_assessment_available);
}

#[test]
fn fsm_resets_to_unavailable_when_l2_becomes_stale_or_gapped() {
    let mut fsm = IntentFsm::default();
    let _ = fsm.observe(&ready_metrics(-0.65));
    let _ = fsm.observe(&ready_metrics(-0.65));
    let stale = fsm.observe(&OrderBookMetrics {
        readiness: OrderBookReadiness::Gap,
        reason: "sequence_gap_resync_required".to_string(),
        ..Default::default()
    });

    assert_eq!(stale.state, IntentState::Unavailable);
    assert!(!stale.intent_assessment_available);
}
