use btc_toxic_flow_monitor_rs::{
    alerts::alert_service::alert_key,
    toxicity::{
        cancel_trade_ratio::compute_cancel_to_trade_ratio,
        dedupe::build_signal_dedupe_key,
        orderbook_delta_evidence::{
            apply_venue_reliability, derive_l2_deltas, DeltaDetectorContext, OrderBookDeltaDetector,
        },
        scoring::combine_score_breakdown,
        toxic_service::toxic_event_semantic_key,
    },
    types::{
        market::{AggressorSide, NormalizedBook, NormalizedTrade, Venue},
        orderbook_delta::{
            ManipulationResolutionStatus, ManipulationSignalType, OrderBookDeltaEvent,
            OrderBookDeltaEvidenceSource, OrderBookDeltaType, VenueReliability,
        },
        orderbook_wall::OrderbookWallSide,
        toxic::{ToxicDirection, ToxicEvent, ToxicSeverity},
        toxic_flow::ToxicConfidence,
        toxic_signal::{ScoreBreakdown, ToxicSignalType},
    },
};

#[test]
fn signal_dedupe_key_keeps_detector_symbol_and_window_boundaries() {
    let first = build_signal_dedupe_key(
        "layering",
        "binance",
        "BTC-PERP",
        Some("ask"),
        Some(100_100.123),
        10_050,
        1_000,
    );
    let same_bucket = build_signal_dedupe_key(
        "layering",
        "binance",
        "BTC-PERP",
        Some("ask"),
        Some(100_100.123),
        10_999,
        1_000,
    );
    let different_symbol = build_signal_dedupe_key(
        "layering",
        "binance",
        "ETH-PERP",
        Some("ask"),
        Some(100_100.123),
        10_999,
        1_000,
    );

    assert_eq!(first, same_bucket);
    assert_ne!(first, different_symbol);
    assert_ne!(
        first,
        build_signal_dedupe_key(
            "spoofing",
            "binance",
            "BTC-PERP",
            Some("ask"),
            Some(100_100.123),
            10_999,
            1_000,
        )
    );
}

#[test]
fn score_breakdown_clamps_and_low_quality_downgrades() {
    let high_quality = ScoreBreakdown {
        toxicity_score: 120.0,
        confidence: 100.0,
        data_quality: 100.0,
        markout_evidence: 100.0,
        liquidity_impact: 100.0,
    };
    let low_quality = ScoreBreakdown {
        data_quality: 40.0,
        ..high_quality.clone()
    };

    assert_eq!(combine_score_breakdown(&high_quality), 100.0);
    assert!(combine_score_breakdown(&low_quality) < 50.0);
}

#[test]
fn derives_price_level_l2_deltas_without_claiming_native_order_ids() {
    let previous = book(1_000, vec![(100_000.0, 1.0)], vec![(100_100.0, 3.0)]);
    let current = book(1_100, vec![(100_000.0, 2.0), (99_900.0, 1.0)], vec![]);

    let deltas = derive_l2_deltas(&previous, &current, 42);

    assert!(deltas
        .iter()
        .any(|delta| delta.delta_type == OrderBookDeltaType::Refill
            && delta.side == OrderbookWallSide::Bid
            && delta.order_id.is_none()
            && delta.evidence_source == OrderBookDeltaEvidenceSource::InferredFromL2Delta));
    assert!(deltas
        .iter()
        .any(|delta| delta.delta_type == OrderBookDeltaType::Add));
    assert!(deltas
        .iter()
        .any(|delta| delta.delta_type == OrderBookDeltaType::Remove));
}

#[test]
fn spoofing_candidate_uses_evidence_chain_and_stays_read_only() {
    let detector = OrderBookDeltaDetector::new(DeltaDetectorContext {
        markout_1s_bps: Some(2.5),
        ..DeltaDetectorContext::default()
    });
    let add = delta(
        OrderBookDeltaType::Add,
        OrderbookWallSide::Ask,
        100_100.0,
        0.0,
        3.0,
        1_000,
    );
    let remove = delta(
        OrderBookDeltaType::Remove,
        OrderbookWallSide::Ask,
        100_100.0,
        3.0,
        0.0,
        3_000,
    );
    let trades = vec![trade(3_050, 100_100.0, 0.1, AggressorSide::Buy)];

    let signals = detector.detect(&[add, remove], &trades);
    let spoof = signals
        .iter()
        .find(|signal| signal.signal_type == ManipulationSignalType::SpoofingCandidate)
        .expect("spoof candidate");

    assert!(spoof.read_only);
    assert_eq!(
        spoof.resolution_status,
        ManipulationResolutionStatus::Candidate
    );
    assert_eq!(spoof.confidence, ToxicConfidence::High);
    assert!(spoof.evidence_checklist.large_wall_appeared);
    assert!(spoof.evidence_checklist.near_touch);
    assert!(spoof.evidence_checklist.low_fill_participation);
    assert!(spoof.evidence_checklist.wall_removed);
    assert!(spoof.evidence_checklist.post_remove_markout);
    assert!(spoof.evidence_checklist.opposite_aggressive_flow);
    assert_eq!(
        spoof.evidence_source,
        OrderBookDeltaEvidenceSource::InferredFromL2Delta
    );

    let toxic_signal = spoof.to_toxic_signal();
    assert_eq!(toxic_signal.signal_type, ToxicSignalType::SpoofingCandidate);
    assert!(toxic_signal.detector_version.is_some());
    assert!(toxic_signal.score_breakdown.is_some());
    assert!(toxic_signal.evidence.is_some());
    assert!(toxic_signal.data_quality.is_some());
    assert!(toxic_signal.dedupe_key.is_some());
    assert_eq!(toxic_signal.resolution_status.as_deref(), Some("candidate"));
    assert_eq!(
        toxic_signal.evidence.as_ref().expect("evidence").venue,
        "binance"
    );
}

#[test]
fn spoofing_missing_markout_and_opposite_flow_is_downgraded() {
    let detector = OrderBookDeltaDetector::new(DeltaDetectorContext::default());
    let add = delta(
        OrderBookDeltaType::Add,
        OrderbookWallSide::Ask,
        100_100.0,
        0.0,
        3.0,
        1_000,
    );
    let remove = delta(
        OrderBookDeltaType::Remove,
        OrderbookWallSide::Ask,
        100_100.0,
        3.0,
        0.0,
        3_000,
    );

    let signals = detector.detect(&[add, remove], &[]);
    let spoof = signals
        .iter()
        .find(|signal| signal.signal_type == ManipulationSignalType::SpoofingCandidate)
        .expect("spoof candidate");

    assert!(spoof.risk_score < 80);
    assert_ne!(spoof.confidence, ToxicConfidence::High);
    assert!(!spoof.evidence_checklist.post_remove_markout);
    assert!(!spoof.evidence_checklist.opposite_aggressive_flow);
}

#[test]
fn spoofing_missing_cancel_evidence_does_not_trigger_high_confidence() {
    let detector = OrderBookDeltaDetector::new(DeltaDetectorContext {
        markout_1s_bps: Some(2.5),
        ..DeltaDetectorContext::default()
    });
    let add = delta(
        OrderBookDeltaType::Add,
        OrderbookWallSide::Ask,
        100_100.0,
        0.0,
        3.0,
        1_000,
    );
    let tiny_reduce = delta(
        OrderBookDeltaType::Reduce,
        OrderbookWallSide::Ask,
        100_100.0,
        3.0,
        2.9,
        3_000,
    );

    let signals = detector.detect(&[add, tiny_reduce], &[]);

    assert!(!signals.iter().any(|signal| signal.signal_type
        == ManipulationSignalType::SpoofingCandidate
        && signal.confidence == ToxicConfidence::High));
}

#[test]
fn layering_candidate_requires_synchronized_same_side_levels() {
    let detector = OrderBookDeltaDetector::new(DeltaDetectorContext {
        markout_1s_bps: Some(1.8),
        ..DeltaDetectorContext::default()
    });
    let deltas = vec![
        delta(
            OrderBookDeltaType::Add,
            OrderbookWallSide::Ask,
            100_100.0,
            0.0,
            3.0,
            1_000,
        ),
        delta(
            OrderBookDeltaType::Add,
            OrderbookWallSide::Ask,
            100_200.0,
            0.0,
            3.0,
            1_100,
        ),
        delta(
            OrderBookDeltaType::Add,
            OrderbookWallSide::Ask,
            100_300.0,
            0.0,
            3.0,
            1_200,
        ),
        delta(
            OrderBookDeltaType::Remove,
            OrderbookWallSide::Ask,
            100_100.0,
            3.0,
            0.0,
            2_000,
        ),
        delta(
            OrderBookDeltaType::Remove,
            OrderbookWallSide::Ask,
            100_200.0,
            3.0,
            0.0,
            2_100,
        ),
        delta(
            OrderBookDeltaType::Remove,
            OrderbookWallSide::Ask,
            100_300.0,
            3.0,
            0.0,
            2_200,
        ),
    ];

    let signals = detector.detect(&deltas, &[]);
    let layering = signals
        .iter()
        .find(|signal| signal.signal_type == ManipulationSignalType::LayeringCandidate)
        .expect("layering candidate");

    assert!(layering.evidence_checklist.synchronized_levels);
    assert!(layering.evidence_checklist.high_cancel_ratio);
    assert!(layering.risk_score >= 80);
    let toxic_signal = layering.to_toxic_signal();
    assert_eq!(toxic_signal.signal_type, ToxicSignalType::LayeringCandidate);
    assert!(toxic_signal.evidence.is_some());
}

#[test]
fn single_wall_does_not_trigger_layering() {
    let detector = OrderBookDeltaDetector::new(DeltaDetectorContext::default());
    let deltas = vec![
        delta(
            OrderBookDeltaType::Add,
            OrderbookWallSide::Ask,
            100_100.0,
            0.0,
            3.0,
            1_000,
        ),
        delta(
            OrderBookDeltaType::Remove,
            OrderbookWallSide::Ask,
            100_100.0,
            3.0,
            0.0,
            2_000,
        ),
    ];

    let signals = detector.detect(&deltas, &[]);

    assert!(!signals
        .iter()
        .any(|signal| signal.signal_type == ManipulationSignalType::LayeringCandidate));
}

#[test]
fn iceberg_candidate_requires_repeated_refill_and_hidden_volume_ratio() {
    let detector = OrderBookDeltaDetector::new(DeltaDetectorContext::default());
    let deltas = vec![
        delta(
            OrderBookDeltaType::Refill,
            OrderbookWallSide::Bid,
            100_000.0,
            1.0,
            2.0,
            1_000,
        ),
        delta(
            OrderBookDeltaType::Refill,
            OrderbookWallSide::Bid,
            100_000.0,
            1.0,
            2.0,
            2_000,
        ),
        delta(
            OrderBookDeltaType::Refill,
            OrderbookWallSide::Bid,
            100_000.0,
            1.0,
            2.0,
            3_000,
        ),
    ];
    let trades = vec![
        trade(1_100, 100_000.0, 2.5, AggressorSide::Sell),
        trade(2_100, 100_000.0, 2.5, AggressorSide::Sell),
        trade(3_100, 100_000.0, 2.5, AggressorSide::Sell),
    ];

    let signals = detector.detect(&deltas, &trades);
    let iceberg = signals
        .iter()
        .find(|signal| signal.signal_type == ManipulationSignalType::IcebergCandidate)
        .expect("iceberg candidate");

    assert!(iceberg.evidence_checklist.repeated_refill);
    assert!(iceberg.evidence_checklist.stable_refill_interval);
    assert!(iceberg.evidence_checklist.hidden_liquidity_ratio);
    assert_eq!(iceberg.fill_qty, 7.5);
    let toxic_signal = iceberg.to_toxic_signal();
    assert_eq!(toxic_signal.signal_type, ToxicSignalType::IcebergCandidate);
    assert!(toxic_signal.evidence.is_some());
}

#[test]
fn ordinary_visible_wall_does_not_trigger_iceberg() {
    let detector = OrderBookDeltaDetector::new(DeltaDetectorContext::default());
    let deltas = vec![delta(
        OrderBookDeltaType::Add,
        OrderbookWallSide::Bid,
        100_000.0,
        0.0,
        3.0,
        1_000,
    )];

    let signals = detector.detect(&deltas, &[]);

    assert!(!signals
        .iter()
        .any(|signal| signal.signal_type == ManipulationSignalType::IcebergCandidate));
}

#[test]
fn snapshot_reset_is_not_cancel_evidence() {
    let detector = OrderBookDeltaDetector::new(DeltaDetectorContext {
        markout_1s_bps: Some(2.5),
        ..DeltaDetectorContext::default()
    });
    let add = delta(
        OrderBookDeltaType::Add,
        OrderbookWallSide::Ask,
        100_100.0,
        0.0,
        3.0,
        1_000,
    );
    let reset = delta(
        OrderBookDeltaType::SnapshotReset,
        OrderbookWallSide::Ask,
        100_100.0,
        3.0,
        0.0,
        3_000,
    );

    let signals = detector.detect(&[add, reset], &[]);

    assert!(!signals
        .iter()
        .any(|signal| signal.signal_type == ManipulationSignalType::SpoofingCandidate));
}

#[test]
fn cancel_to_trade_ratio_handles_zero_fill_safely() {
    assert_eq!(compute_cancel_to_trade_ratio(10.0, 0.0), None);
    assert_eq!(compute_cancel_to_trade_ratio(10.0, 2.0), Some(5.0));
}

#[test]
fn venue_reliability_filter_downgrades_low_quality_candidates() {
    let detector = OrderBookDeltaDetector::new(DeltaDetectorContext {
        markout_1s_bps: Some(2.5),
        ..DeltaDetectorContext::default()
    });
    let add = delta(
        OrderBookDeltaType::Add,
        OrderbookWallSide::Ask,
        100_100.0,
        0.0,
        3.0,
        1_000,
    );
    let remove = delta(
        OrderBookDeltaType::Remove,
        OrderbookWallSide::Ask,
        100_100.0,
        3.0,
        0.0,
        3_000,
    );
    let mut signal = detector.detect(&[add, remove], &[]).remove(0);
    let before = signal.risk_score;

    apply_venue_reliability(
        &mut signal,
        VenueReliability {
            venue: Venue::Binance,
            reliability_score: 0.50,
        },
    );

    assert!(signal.risk_score < before);
    assert_eq!(signal.confidence, ToxicConfidence::Low);
    assert_eq!(signal.score_breakdown.venue_reliability_score, 50);
}

#[test]
fn toxic_event_semantic_key_ignores_random_event_id_within_same_bucket() {
    let mut first = toxic_event("event-a", "BTC-PERP", 10_050);
    let second = toxic_event("event-b", "BTC-PERP", 10_999);

    assert_eq!(
        toxic_event_semantic_key(&first),
        toxic_event_semantic_key(&second)
    );

    first.symbol = "ETH-PERP".to_string();
    assert_ne!(
        toxic_event_semantic_key(&first),
        toxic_event_semantic_key(&second)
    );
}

#[test]
fn alert_key_contains_symbol_so_symbols_do_not_suppress_each_other() {
    let btc = toxic_event("event-a", "BTC-PERP", 10_050);
    let eth = toxic_event("event-b", "ETH-PERP", 10_050);

    assert!(alert_key(&btc).contains("BTC-PERP"));
    assert_ne!(alert_key(&btc), alert_key(&eth));
}

fn delta(
    delta_type: OrderBookDeltaType,
    side: OrderbookWallSide,
    price: f64,
    before: f64,
    after: f64,
    ts: i64,
) -> OrderBookDeltaEvent {
    let delta_qty = after - before;
    OrderBookDeltaEvent {
        venue: Venue::Binance,
        symbol: "BTC-PERP".to_string(),
        side,
        price,
        qty_before: before,
        qty_after: after,
        delta_qty,
        delta_type,
        ts,
        sequence: ts as u64,
        order_id: None,
        lifetime_ms: None,
        fill_qty: None,
        cancel_qty: matches!(
            delta_type,
            OrderBookDeltaType::Reduce | OrderBookDeltaType::Remove
        )
        .then_some(delta_qty.abs()),
        evidence_source: OrderBookDeltaEvidenceSource::InferredFromL2Delta,
        distance_to_touch_bps: Some(2.0),
        depth_before: Some(before),
        depth_after: Some(after),
    }
}

fn book(ts: i64, bids: Vec<(f64, f64)>, asks: Vec<(f64, f64)>) -> NormalizedBook {
    NormalizedBook {
        venue: Venue::Binance,
        symbol: "BTC-PERP".to_string(),
        ts,
        best_bid: bids.first().map(|(price, _)| *price).unwrap_or(100_000.0),
        best_ask: asks.first().map(|(price, _)| *price).unwrap_or(100_100.0),
        bids,
        asks,
        mid: 100_050.0,
        spread_bps: 1.0,
        bid_depth_btc_10bps: 10.0,
        ask_depth_btc_10bps: 10.0,
        bid_depth_usd_10bps: 1_000_000.0,
        ask_depth_usd_10bps: 1_000_000.0,
        imbalance_10bps: 0.0,
    }
}

fn trade(ts: i64, price: f64, size_btc: f64, aggressor_side: AggressorSide) -> NormalizedTrade {
    NormalizedTrade {
        venue: Venue::Binance,
        symbol: "BTC-PERP".to_string(),
        ts,
        price,
        size_btc,
        size_usd: size_btc * price,
        aggressor_side,
        trade_id: None,
    }
}

fn toxic_event(id: &str, symbol: &str, ts: i64) -> ToxicEvent {
    ToxicEvent {
        id: id.to_string(),
        ts,
        symbol: symbol.to_string(),
        direction: ToxicDirection::Buy,
        severity: ToxicSeverity::Alert,
        toxic_volume_btc: 1_200.0,
        threshold_btc: 1_000.0,
        window_ms: 1_000,
        leader_venue: Some(Venue::Binance),
        aggressive_buy_btc: 1_500.0,
        aggressive_sell_btc: 0.0,
        net_aggressive_btc: 1_500.0,
        abs_aggressive_btc: 1_500.0,
        markout_1s_bps: Some(2.0),
        markout_5s_bps: Some(4.0),
        sweep_detected: true,
        liquidity_thin: true,
        liquidity: None,
        cross_venue_confirmed: true,
        vpin_enabled: false,
        vpin: None,
        vpin_zscore: None,
        vpin_spike: false,
        vpin_high: false,
        vpin_extreme: false,
        liquidation_enabled: false,
        nearest_cluster_side: None,
        cluster_distance_bps: None,
        cluster_notional_usd: None,
        cluster_density: None,
        liq_hunt_pressure: 0.0,
        liq_cluster_nearby: false,
        possible_liq_hunt_setup: false,
        reason_codes: vec!["threshold_crossed".to_string()],
    }
}
