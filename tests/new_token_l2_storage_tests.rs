use std::time::{SystemTime, UNIX_EPOCH};

use btc_toxic_flow_monitor_rs::{
    storage::{new_token_l2_repo::NewTokenL2Repo, SqliteStore},
    toxic_v3::new_token_watch::{
        intent::{IntentAssessment, IntentState},
        l2::{DepthDiff, DepthLevel, DepthSnapshot},
        session::L2SessionRegistry,
        shadow::ShadowOutcomeTracker,
    },
};

#[test]
fn persists_compact_l2_metrics_and_prunes_by_timestamp() {
    let path = std::env::temp_dir().join(format!(
        "new-token-l2-storage-{}.sqlite",
        SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("clock")
            .as_nanos()
    ));
    let store = SqliteStore::open(path.to_string_lossy().as_ref()).expect("open store");
    store.migrate().expect("migrate store");
    let registry = L2SessionRegistry::default();
    registry.register("ASTERUSDT");
    registry.install_snapshot(
        "ASTERUSDT",
        DepthSnapshot {
            last_update_id: 1,
            bids: vec![DepthLevel {
                price: 1.0,
                quantity: 8.0,
            }],
            asks: vec![DepthLevel {
                price: 1.01,
                quantity: 2.0,
            }],
            fetched_at_ms: 10,
        },
    );
    registry
        .apply_diff(
            "ASTERUSDT",
            DepthDiff {
                first_update_id: 2,
                final_update_id: 2,
                previous_final_update_id: Some(1),
                bids: vec![DepthLevel {
                    price: 1.0,
                    quantity: 8.0,
                }],
                asks: vec![DepthLevel {
                    price: 1.01,
                    quantity: 2.0,
                }],
                event_time_ms: 20,
            },
        )
        .expect("ready diff");
    let snapshot = registry.session("ASTERUSDT").expect("session");

    store
        .insert_new_token_l2_metric(1_000, &snapshot)
        .expect("insert compact metric");
    assert_eq!(
        store
            .count_new_token_l2_metrics("ASTERUSDT")
            .expect("count"),
        1
    );
    assert_eq!(
        store
            .prune_new_token_l2_metrics_older_than(1_001)
            .expect("prune"),
        1
    );

    let _ = std::fs::remove_file(path);
}

#[test]
fn persists_shadow_outcomes_without_enabling_discord_or_execution() {
    let path = std::env::temp_dir().join(format!(
        "new-token-l2-outcome-storage-{}.sqlite",
        SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("clock")
            .as_nanos()
    ));
    let store = SqliteStore::open(path.to_string_lossy().as_ref()).expect("open store");
    store.migrate().expect("migrate store");
    let mut tracker = ShadowOutcomeTracker::default();
    tracker.observe_intent(
        "ASTERUSDT",
        1_000,
        1.0,
        &IntentAssessment {
            state: IntentState::BidPressure,
            confidence: 0.7,
            intent_assessment_available: true,
            reason: "test".to_string(),
            evidence: vec![],
            read_only: true,
        },
    );
    let outcomes = tracker.observe_price("ASTERUSDT", 11_000, 1.01);
    assert_eq!(
        store
            .upsert_new_token_l2_shadow_outcomes(&outcomes)
            .expect("persist outcome"),
        1
    );
    let stored = store
        .list_new_token_l2_shadow_outcomes("ASTERUSDT", 10)
        .expect("load outcomes");
    assert_eq!(stored.len(), 1);
    assert!(stored[0].shadow_only);
    assert!(!stored[0].discord_eligible);
    assert!(!stored[0].execution_enabled);
    assert_eq!(
        store
            .prune_new_token_l2_outcomes_older_than(1_001)
            .expect("prune outcomes"),
        1
    );
    let _ = std::fs::remove_file(path);
}
