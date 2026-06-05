use btc_toxic_flow_monitor_rs::{
    replay::markout_evaluator::{evaluate_candidate_markout, ReplayPricePoint},
    types::{
        market::Venue,
        orderbook_delta::{
            ManipulationEvidenceChecklist, ManipulationResolutionStatus,
            ManipulationScoreBreakdown, ManipulationSignalType, ManipulationSignalV2,
            OrderBookDeltaEvidenceSource,
        },
        orderbook_wall::OrderbookWallSide,
        toxic_flow::ToxicConfidence,
    },
};

#[test]
fn markout_uses_side_direction_and_future_price_only() {
    let prices = vec![
        ReplayPricePoint {
            ts_ms: 1_000,
            mid: 100_000.0,
        },
        ReplayPricePoint {
            ts_ms: 2_000,
            mid: 100_100.0,
        },
        ReplayPricePoint {
            ts_ms: 6_000,
            mid: 100_200.0,
        },
    ];
    let ask_signal = signal(OrderbookWallSide::Ask);
    let bid_signal = signal(OrderbookWallSide::Bid);

    let ask = evaluate_candidate_markout(&ask_signal, &prices);
    let bid = evaluate_candidate_markout(&bid_signal, &prices);

    assert!(ask.markout_1s_bps.expect("ask 1s") > 0.0);
    assert!(bid.markout_1s_bps.expect("bid 1s") < 0.0);
    assert!(ask.markout_30s_bps.is_none());
}

fn signal(side: OrderbookWallSide) -> ManipulationSignalV2 {
    ManipulationSignalV2 {
        signal_id: "sig".to_string(),
        detector_version: "test".to_string(),
        signal_type: ManipulationSignalType::SpoofingCandidate,
        venue: Venue::Binance,
        symbol: "BTC-PERP".to_string(),
        side,
        window_ms: 1_000,
        observed_start_ms: 0,
        observed_end_ms: 1_000,
        price: Some(100_100.0),
        add_qty: 3.0,
        cancel_qty: 3.0,
        fill_qty: 0.1,
        cancel_to_trade_ratio: Some(30.0),
        depth_before: Some(3.0),
        depth_after: Some(0.0),
        price_impact_bps: None,
        markout_1s_bps: None,
        markout_5s_bps: None,
        markout_30s_bps: None,
        risk_score: 80,
        confidence: ToxicConfidence::High,
        score_breakdown: ManipulationScoreBreakdown {
            toxicity_score: 80,
            confidence_score: 90,
            data_quality_score: 68,
            markout_evidence_score: 20,
            venue_reliability_score: 100,
        },
        data_quality: "inferred_from_l2_delta".to_string(),
        dedupe_key: "dedupe".to_string(),
        raw_evidence_links: vec!["raw".to_string()],
        resolution_status: ManipulationResolutionStatus::Candidate,
        evidence_source: OrderBookDeltaEvidenceSource::InferredFromL2Delta,
        evidence_checklist: ManipulationEvidenceChecklist::default(),
        reasons: vec!["candidate".to_string()],
        read_only: true,
    }
}
