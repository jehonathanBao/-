use btc_toxic_flow_monitor_rs::toxic_v3::new_token_watch::{
    l2::{DepthDiff, DepthLevel, DepthSnapshot, LocalOrderBook},
    walls::{WallLifecycle, WallTracker},
};

fn snapshot(bid_quantity: f64, last_update_id: u64) -> DepthSnapshot {
    DepthSnapshot {
        last_update_id,
        bids: vec![
            DepthLevel {
                price: 100.0,
                quantity: bid_quantity,
            },
            DepthLevel {
                price: 99.9,
                quantity: 1.0,
            },
            DepthLevel {
                price: 99.8,
                quantity: 1.0,
            },
        ],
        asks: vec![
            DepthLevel {
                price: 100.1,
                quantity: 1.0,
            },
            DepthLevel {
                price: 100.2,
                quantity: 1.0,
            },
            DepthLevel {
                price: 100.3,
                quantity: 1.0,
            },
        ],
        fetched_at_ms: 1,
    }
}

fn make_ready(book: &mut LocalOrderBook, bid_quantity: f64, update_id: u64) {
    book.install_snapshot(snapshot(bid_quantity, update_id));
    book.apply_diff(DepthDiff {
        first_update_id: update_id + 1,
        final_update_id: update_id + 1,
        previous_final_update_id: Some(update_id),
        bids: vec![DepthLevel {
            price: 100.0,
            quantity: bid_quantity,
        }],
        asks: vec![DepthLevel {
            price: 100.1,
            quantity: 1.0,
        }],
        event_time_ms: 2,
    })
    .expect("contiguous depth diff");
}

#[test]
fn persistent_visible_level_becomes_evidence_not_whale_confirmation() {
    let mut book = LocalOrderBook::default();
    make_ready(&mut book, 8.0, 1);
    let mut tracker = WallTracker::default();

    tracker.observe(&book, 1_000);
    tracker.observe(&book, 2_000);
    tracker.observe(&book, 3_000);

    let wall = tracker
        .evidence()
        .into_iter()
        .next()
        .expect("visible wall evidence");
    assert_eq!(wall.lifecycle, WallLifecycle::Persistent);
    assert!(wall.probabilistic);
    assert!(!wall.participant_identified);
}

#[test]
fn disappeared_level_is_recorded_as_pull_not_spoof_confirmation() {
    let mut book = LocalOrderBook::default();
    make_ready(&mut book, 8.0, 1);
    let mut tracker = WallTracker::default();
    tracker.observe(&book, 1_000);
    tracker.observe(&book, 2_000);
    tracker.observe(&book, 3_000);

    make_ready(&mut book, 1.0, 2);
    tracker.observe(&book, 4_000);

    assert!(tracker
        .evidence()
        .iter()
        .any(|wall| wall.lifecycle == WallLifecycle::Pulled));
    assert!(tracker
        .evidence()
        .iter()
        .all(|wall| wall.label != "spoof_confirmed"));
}
