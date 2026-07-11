use btc_toxic_flow_monitor_rs::toxic_v3::new_token_watch::{
    intent::IntentState,
    l2::{DepthDiff, DepthLevel, DepthSnapshot},
    session::{L2SessionRegistry, L2SessionStatus},
};

#[test]
fn registry_creates_per_symbol_flow_only_session_before_l2_sync() {
    let registry = L2SessionRegistry::default();
    let session = registry.register("ASTERUSDT");

    assert_eq!(session.symbol, "ASTERUSDT");
    assert_eq!(session.status, L2SessionStatus::Connecting);
    assert_eq!(session.evidence_mode, "flow_only");
    assert!(!session.orderbook_evidence_available);
    assert!(!session.intent_assessment_available);
}

#[test]
fn snapshot_promotes_only_the_registered_symbol_to_l2_ready() {
    let registry = L2SessionRegistry::default();
    registry.register("ASTERUSDT");
    registry.register("JTOUSDT");
    registry.install_snapshot(
        "ASTERUSDT",
        DepthSnapshot {
            last_update_id: 7,
            bids: vec![DepthLevel {
                price: 1.0,
                quantity: 12.0,
            }],
            asks: vec![DepthLevel {
                price: 1.01,
                quantity: 3.0,
            }],
            fetched_at_ms: 100,
        },
    );
    registry
        .apply_diff(
            "ASTERUSDT",
            DepthDiff {
                first_update_id: 8,
                final_update_id: 8,
                previous_final_update_id: Some(7),
                bids: vec![DepthLevel {
                    price: 1.0,
                    quantity: 12.0,
                }],
                asks: vec![DepthLevel {
                    price: 1.01,
                    quantity: 3.0,
                }],
                event_time_ms: 101,
            },
        )
        .expect("contiguous L2 update");
    registry
        .apply_diff(
            "ASTERUSDT",
            DepthDiff {
                first_update_id: 9,
                final_update_id: 9,
                previous_final_update_id: Some(8),
                bids: vec![DepthLevel {
                    price: 1.0,
                    quantity: 12.0,
                }],
                asks: vec![DepthLevel {
                    price: 1.01,
                    quantity: 3.0,
                }],
                event_time_ms: 102,
            },
        )
        .expect("second contiguous L2 update");

    let aster = registry.session("ASTERUSDT").expect("aster session");
    let jto = registry.session("JTOUSDT").expect("jto session");
    assert_eq!(aster.status, L2SessionStatus::Ready);
    assert_eq!(aster.evidence_mode, "l2_ready");
    assert!(aster.orderbook_evidence_available);
    assert_eq!(aster.intent.state, IntentState::BidPressure);
    assert!(aster.intent_assessment_available);
    assert_eq!(jto.status, L2SessionStatus::Connecting);
    assert!(!jto.orderbook_evidence_available);
}

#[test]
fn remove_clears_session_and_prevents_stale_l2_claims() {
    let registry = L2SessionRegistry::default();
    registry.register("ASTERUSDT");
    assert!(registry.remove("ASTERUSDT"));
    assert!(registry.session("ASTERUSDT").is_none());
    assert_eq!(registry.active_count(), 0);
}

#[test]
fn registry_keeps_fifty_symbol_sessions_isolated() {
    let registry = L2SessionRegistry::default();
    for index in 0..50 {
        registry.register(&format!("T{index}USDT"));
    }

    assert_eq!(registry.active_count(), 50);
    assert_eq!(registry.session("T0USDT").expect("first").symbol, "T0USDT");
    assert_eq!(registry.session("T49USDT").expect("last").symbol, "T49USDT");
}

#[test]
fn repeated_book_ticker_mismatches_invalidate_l2_intent_until_resync() {
    let registry = L2SessionRegistry::default();
    registry.register("ASTERUSDT");
    registry.install_snapshot(
        "ASTERUSDT",
        DepthSnapshot {
            last_update_id: 10,
            bids: vec![DepthLevel {
                price: 1.0,
                quantity: 10.0,
            }],
            asks: vec![DepthLevel {
                price: 1.01,
                quantity: 2.0,
            }],
            fetched_at_ms: 100,
        },
    );
    for update_id in [11, 12] {
        registry
            .apply_diff(
                "ASTERUSDT",
                DepthDiff {
                    first_update_id: update_id,
                    final_update_id: update_id,
                    previous_final_update_id: Some(update_id - 1),
                    bids: vec![DepthLevel {
                        price: 1.0,
                        quantity: 10.0,
                    }],
                    asks: vec![DepthLevel {
                        price: 1.01,
                        quantity: 2.0,
                    }],
                    event_time_ms: update_id as i64,
                },
            )
            .expect("contiguous update");
    }

    // A single bookTicker discrepancy is observed, but repeated mismatches
    // make the local book unsafe and require a fresh snapshot.
    assert!(!registry.record_book_ticker("ASTERUSDT", 0.9, 0.91, 200));
    assert!(
        registry
            .session("ASTERUSDT")
            .expect("session")
            .orderbook_evidence_available
    );
    assert!(registry.record_book_ticker("ASTERUSDT", 0.9, 0.91, 201));

    let snapshot = registry.session("ASTERUSDT").expect("session");
    assert_eq!(snapshot.status, L2SessionStatus::Gap);
    assert!(!snapshot.orderbook_evidence_available);
    assert!(!snapshot.intent_assessment_available);
    assert_eq!(
        snapshot.intent.reason,
        "book_ticker_mismatch_resync_required"
    );
}

#[test]
fn session_uses_real_agg_trade_direction_for_short_flow_windows() {
    let registry = L2SessionRegistry::default();
    registry.register("ASTERUSDT");
    // buyerIsMaker=false means the buyer was the taker: aggressive buy.
    registry.record_agg_trade("ASTERUSDT", 2.0, 10.0, false, 1_000);
    // buyerIsMaker=true means the seller was the taker: aggressive sell.
    registry.record_agg_trade("ASTERUSDT", 2.0, 3.0, true, 1_500);
    registry.record_mark_price("ASTERUSDT", 2.01, 1_600);

    let snapshot = registry.session("ASTERUSDT").expect("session");
    assert_eq!(snapshot.trade_flow.buy_notional_1s, 20.0);
    assert_eq!(snapshot.trade_flow.sell_notional_1s, 6.0);
    assert_eq!(snapshot.trade_flow.mark_price, Some(2.01));
    assert_eq!(snapshot.trade_flow.reason, "binance_agg_trade");
}

#[test]
fn missing_oi_is_explicit_but_does_not_remove_l2_flow_evidence() {
    let registry = L2SessionRegistry::default();
    registry.register("ASTERUSDT");
    registry.record_agg_trade("ASTERUSDT", 2.0, 1.0, false, 1_000);
    let snapshot = registry.session("ASTERUSDT").expect("session");

    assert!(!snapshot.open_interest.available);
    assert_eq!(snapshot.open_interest.reason, "open_interest_not_observed");
    assert_eq!(snapshot.trade_flow.buy_notional_1s, 2.0);
}
