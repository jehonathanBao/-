use btc_toxic_flow_monitor_rs::{
    market_data::price_index::PriceSnapshot,
    toxicity::toxic_markout::{
        build_toxic_markout_by_signal_id, build_toxic_markout_recent, build_toxic_markout_status,
    },
    types::{
        toxic_flow::ToxicConfidence,
        toxic_markout::ToxicMarkoutOutcome,
        toxic_signal::{
            ToxicChaseRisk, ToxicSignal, ToxicSignalDirection, ToxicSignalRecentResponse,
            ToxicSignalType,
        },
    },
};

#[test]
fn short_bias_signal_marks_aligned_and_adverse_windows() {
    let recent = build_toxic_markout_recent(
        "BTC-PERP",
        &fusion_recent(vec![signal(
            "fusion-short",
            ToxicSignalType::ShortBiasToxicFlow,
            ToxicSignalDirection::ShortBias,
            1_000,
        )]),
        snapshot_at_or_before,
        |_| snapshots_for_short_bias(),
    );

    assert!(recent.read_only);
    assert!(!recent.runtime_modified);
    let signal = &recent.signals[0];
    assert_eq!(signal.windows[0].outcome, ToxicMarkoutOutcome::Aligned);
    assert_eq!(signal.windows[1].outcome, ToxicMarkoutOutcome::Adverse);
    assert_eq!(
        signal.windows[2].outcome,
        ToxicMarkoutOutcome::NotEnoughData
    );
}

#[test]
fn long_bias_signal_marks_aligned_window() {
    let recent = build_toxic_markout_recent(
        "BTC-PERP",
        &fusion_recent(vec![signal(
            "fusion-long",
            ToxicSignalType::LongBiasToxicFlow,
            ToxicSignalDirection::LongBias,
            1_000,
        )]),
        snapshot_at_or_before,
        |_| snapshots_for_long_bias(),
    );

    assert_eq!(
        recent.signals[0].windows[0].outcome,
        ToxicMarkoutOutcome::Aligned
    );
}

#[test]
fn trap_risk_is_neutral_when_data_exists() {
    let recent = build_toxic_markout_recent(
        "BTC-PERP",
        &fusion_recent(vec![signal(
            "fusion-trap",
            ToxicSignalType::TrapRisk,
            ToxicSignalDirection::TrapRisk,
            1_000,
        )]),
        snapshot_at_or_before,
        |_| snapshots_for_short_bias(),
    );

    assert_eq!(
        recent.signals[0].windows[0].outcome,
        ToxicMarkoutOutcome::Neutral
    );
}

#[test]
fn missing_future_data_returns_not_enough_data() {
    let recent = build_toxic_markout_recent(
        "BTC-PERP",
        &fusion_recent(vec![signal(
            "fusion-missing",
            ToxicSignalType::ShortBiasToxicFlow,
            ToxicSignalDirection::ShortBias,
            1_000,
        )]),
        snapshot_at_or_before,
        |_| {
            vec![PriceSnapshot {
                ts: 1_000,
                index_mid: 100_000.0,
                spread_bps_median: None,
                imbalance_10bps_median: None,
                bid_depth_btc_10bps_median: None,
                ask_depth_btc_10bps_median: None,
            }]
        },
    );

    assert_eq!(
        recent.signals[0].overall_outcome,
        ToxicMarkoutOutcome::NotEnoughData
    );
}

#[test]
fn status_is_analysis_only() {
    let status = build_toxic_markout_status(
        "BTC-PERP",
        &fusion_recent(vec![signal(
            "fusion-status",
            ToxicSignalType::ShortBiasToxicFlow,
            ToxicSignalDirection::ShortBias,
            1_000,
        )]),
        snapshot_at_or_before,
        |_| snapshots_for_short_bias(),
    );

    assert!(status.read_only);
    assert!(!status.runtime_modified);
    assert_eq!(status.mode, "analysis_only");
}

#[test]
fn by_signal_id_returns_signal_and_missing_signal_is_unavailable() {
    let found = build_toxic_markout_by_signal_id(
        "BTC-PERP",
        "fusion-short",
        &fusion_recent(vec![signal(
            "fusion-short",
            ToxicSignalType::ShortBiasToxicFlow,
            ToxicSignalDirection::ShortBias,
            1_000,
        )]),
        snapshot_at_or_before,
        |_| snapshots_for_short_bias(),
    );
    assert!(found.available);
    assert_eq!(
        found.signal.expect("signal").overall_outcome,
        ToxicMarkoutOutcome::Neutral
    );

    let missing = build_toxic_markout_by_signal_id(
        "BTC-PERP",
        "missing",
        &fusion_recent(vec![signal(
            "fusion-short",
            ToxicSignalType::ShortBiasToxicFlow,
            ToxicSignalDirection::ShortBias,
            1_000,
        )]),
        snapshot_at_or_before,
        |_| snapshots_for_short_bias(),
    );
    assert!(!missing.available);
    assert_eq!(missing.reason.as_deref(), Some("signal_not_found"));
}

fn fusion_recent(signals: Vec<ToxicSignal>) -> ToxicSignalRecentResponse {
    ToxicSignalRecentResponse {
        read_only: true,
        runtime_modified: false,
        analysis_only: true,
        execution_enabled: false,
        mode: "analysis_only".to_string(),
        selected_symbol: "BTC-PERP".to_string(),
        status: "fusion_ready".to_string(),
        warnings: Vec::new(),
        no_trade_reasons: vec!["analysis only".to_string()],
        signals,
    }
}

fn signal(
    signal_id: &str,
    signal_type: ToxicSignalType,
    direction: ToxicSignalDirection,
    ts_ms: u64,
) -> ToxicSignal {
    ToxicSignal {
        signal_id: signal_id.to_string(),
        symbol: "BTC-PERP".to_string(),
        ts_ms,
        signal_type,
        direction,
        toxicity_score: 80,
        confidence: ToxicConfidence::High,
        primary_reason: "markout test".to_string(),
        reason: vec!["test".to_string()],
        supporting_evidence: Vec::new(),
        invalidation_price: Some(100_100.0),
        suggested_stop_distance_usd: Some(100.0),
        chase_risk: ToxicChaseRisk::Medium,
        no_trade_reasons: vec!["reference only".to_string()],
        linked_active_trade_signal_ids: Vec::new(),
        linked_liquidation_signal_ids: Vec::new(),
        linked_wall_lifecycle_signal_ids: Vec::new(),
        linked_wall_interpretation_signal_ids: Vec::new(),
        linked_structural_signal_ids: Vec::new(),
        read_only: true,
        detector_version: None,
        score_breakdown: None,
        evidence: None,
        data_quality: None,
        dedupe_key: None,
        resolution_status: None,
    }
}

fn snapshot_at_or_before(ts: i64) -> Option<PriceSnapshot> {
    (ts >= 1_000).then_some(PriceSnapshot {
        ts: 1_000,
        index_mid: 100_000.0,
        spread_bps_median: None,
        imbalance_10bps_median: None,
        bid_depth_btc_10bps_median: None,
        ask_depth_btc_10bps_median: None,
    })
}

fn snapshots_for_short_bias() -> Vec<PriceSnapshot> {
    vec![
        PriceSnapshot {
            ts: 61_000,
            index_mid: 99_900.0,
            spread_bps_median: None,
            imbalance_10bps_median: None,
            bid_depth_btc_10bps_median: None,
            ask_depth_btc_10bps_median: None,
        },
        PriceSnapshot {
            ts: 301_000,
            index_mid: 100_120.0,
            spread_bps_median: None,
            imbalance_10bps_median: None,
            bid_depth_btc_10bps_median: None,
            ask_depth_btc_10bps_median: None,
        },
    ]
}

fn snapshots_for_long_bias() -> Vec<PriceSnapshot> {
    vec![PriceSnapshot {
        ts: 61_000,
        index_mid: 100_120.0,
        spread_bps_median: None,
        imbalance_10bps_median: None,
        bid_depth_btc_10bps_median: None,
        ask_depth_btc_10bps_median: None,
    }]
}
