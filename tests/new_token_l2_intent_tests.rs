use btc_toxic_flow_monitor_rs::toxic_v3::new_token_watch::{
    intent::{evaluate_intent, IntentState},
    l2::{DepthDiff, DepthLevel, DepthSnapshot, LocalOrderBook, OrderBookReadiness},
};

fn level(price: f64, quantity: f64) -> DepthLevel {
    DepthLevel { price, quantity }
}

fn snapshot(last_update_id: u64) -> DepthSnapshot {
    DepthSnapshot {
        last_update_id,
        bids: vec![level(100.0, 3.0), level(99.5, 2.0)],
        asks: vec![level(100.5, 4.0), level(101.0, 2.0)],
        fetched_at_ms: 1_000,
    }
}

fn diff(first: u64, final_id: u64, previous_final: Option<u64>) -> DepthDiff {
    DepthDiff {
        first_update_id: first,
        final_update_id: final_id,
        previous_final_update_id: previous_final,
        bids: vec![level(100.0, 5.0)],
        asks: vec![level(100.5, 1.0)],
        event_time_ms: 1_100,
    }
}

#[test]
fn book_only_becomes_ready_after_snapshot_and_contiguous_first_diff() {
    let mut book = LocalOrderBook::default();
    book.buffer_diff(diff(100, 101, None));
    book.install_snapshot(snapshot(100));

    assert_eq!(book.readiness(), OrderBookReadiness::Ready);
    assert_eq!(book.last_update_id(), Some(101));
    assert!(book.metrics(3).spread_bps > 0.0);
}

#[test]
fn sequence_gap_invalidates_book_and_blocks_intent_interpretation() {
    let mut book = LocalOrderBook::default();
    book.install_snapshot(snapshot(100));
    assert!(book.apply_diff(diff(101, 101, Some(100))).is_ok());

    let error = book
        .apply_diff(diff(103, 103, Some(102)))
        .expect_err("a missing update must force a resync");
    assert_eq!(error.as_str(), "sequence_gap");
    assert_eq!(book.readiness(), OrderBookReadiness::Gap);
    assert_eq!(
        evaluate_intent(&book.metrics(3)).state,
        IntentState::Unavailable
    );
}

#[test]
fn first_buffered_diff_may_cover_snapshot_without_matching_previous_id() {
    let mut book = LocalOrderBook::default();
    book.buffer_diff(diff(99, 101, Some(98)));
    book.install_snapshot(snapshot(100));

    assert_eq!(book.readiness(), OrderBookReadiness::Ready);
    assert_eq!(book.last_update_id(), Some(101));
}

#[test]
fn intent_is_unavailable_without_ready_l2_evidence() {
    let book = LocalOrderBook::default();
    let assessment = evaluate_intent(&book.metrics(5));

    assert_eq!(assessment.state, IntentState::Unavailable);
    assert_eq!(assessment.reason, "orderbook_not_ready");
    assert!(!assessment.intent_assessment_available);
}

#[test]
fn ready_book_exposes_visible_depth_add_and_remove_proxies() {
    let mut book = LocalOrderBook::default();
    book.install_snapshot(snapshot(100));
    book.apply_diff(DepthDiff {
        first_update_id: 101,
        final_update_id: 101,
        previous_final_update_id: Some(100),
        bids: vec![level(100.0, 6.0)],
        asks: vec![level(100.5, 0.0)],
        event_time_ms: 1_100,
    })
    .expect("contiguous diff");

    let metrics = book.metrics(3);
    assert_eq!(metrics.bid_added_quantity, 3.0);
    assert_eq!(metrics.ask_removed_quantity, 4.0);
    assert!(metrics.visible_cancel_to_add_ratio > 1.0);
}
