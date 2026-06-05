use btc_toxic_flow_monitor_rs::toxicity::toxic_replay::{
    build_toxic_replay_by_signal_id, build_toxic_replay_latest, build_toxic_replay_recent,
    build_toxic_replay_status,
};
use btc_toxic_flow_monitor_rs::types::{
    liquidation::{
        LiquidationToxicDirection, LiquidationToxicSignal, LiquidationToxicSignalType,
        LiquidationToxicityRecentResponse,
    },
    orderbook_wall::{
        OrderbookWallEventType, OrderbookWallInterpretationReport,
        OrderbookWallInterpretationSignal, OrderbookWallInterpretationType,
        OrderbookWallLifecycleEvent, OrderbookWallLifecycleReport, OrderbookWallSide,
    },
    structural_toxicity::{
        StructuralLevelType, StructuralToxicDirection, StructuralToxicSignal,
        StructuralToxicSignalType, StructuralToxicityRecentResponse,
    },
    toxic_flow::{
        ActiveTradeToxicSignal, ActiveTradeToxicSignalType, ActiveTradeToxicityRecentResponse,
        ToxicConfidence, ToxicSide,
    },
    toxic_signal::{
        ToxicChaseRisk, ToxicSignal, ToxicSignalDirection, ToxicSignalRecentResponse,
        ToxicSignalType, ToxicSupportingEvidence,
    },
};

#[test]
fn replay_recent_is_read_only_and_lists_signals() {
    let recent = build_toxic_replay_recent("BTC-PERP", &fusion_recent(vec![source_signal()]));
    assert!(recent.read_only);
    assert!(!recent.runtime_modified);
    assert_eq!(recent.mode, "analysis_only");
    assert_eq!(recent.signals.len(), 1);
}

#[test]
fn replay_detail_breaks_down_evidence_by_layer() {
    let detail = build_toxic_replay_by_signal_id(
        "BTC-PERP",
        "fusion-1",
        &fusion_recent(vec![source_signal()]),
        &active_trade_recent(vec![active_trade_signal()]),
        &liquidation_recent(vec![liquidation_signal()]),
        &wall_lifecycle_report(vec![wall_event()]),
        &wall_interpretation_report(vec![wall_interpretation_signal()]),
        &structural_recent(vec![structural_signal()]),
    );

    assert!(detail.available);
    let replay = detail.replay.expect("replay detail");
    assert_eq!(replay.evidence_breakdown.active_trade.len(), 1);
    assert_eq!(replay.evidence_breakdown.liquidation.len(), 1);
    assert_eq!(replay.evidence_breakdown.orderbook.len(), 1);
    assert_eq!(replay.evidence_breakdown.wall_interpretation.len(), 1);
    assert_eq!(replay.evidence_breakdown.structural.len(), 1);
    assert_eq!(
        replay.reference_levels.wording,
        "Reference only. No order instruction."
    );
    assert!(!replay.operator_narrative.why_not_entry_signal.is_empty());
}

#[test]
fn replay_latest_returns_unavailable_when_fusion_has_no_signal() {
    let latest = build_toxic_replay_latest(
        "BTC-PERP",
        &fusion_recent(Vec::new()),
        &active_trade_recent(Vec::new()),
        &liquidation_recent(Vec::new()),
        &wall_lifecycle_report(Vec::new()),
        &wall_interpretation_report(Vec::new()),
        &structural_recent(Vec::new()),
    );

    assert!(!latest.available);
    assert_eq!(latest.reason.as_deref(), Some("latest_signal_unavailable"));
}

#[test]
fn replay_status_reports_analysis_only_counts() {
    let status = build_toxic_replay_status("BTC-PERP", &fusion_recent(vec![source_signal()]));
    assert!(status.read_only);
    assert!(!status.runtime_modified);
    assert_eq!(status.mode, "analysis_only");
    assert_eq!(status.signal_count, 1);
}

fn fusion_recent(signals: Vec<ToxicSignal>) -> ToxicSignalRecentResponse {
    ToxicSignalRecentResponse {
        read_only: true,
        runtime_modified: false,
        analysis_only: true,
        execution_enabled: false,
        mode: "analysis_only".to_string(),
        selected_symbol: "BTC-PERP".to_string(),
        status: if signals.is_empty() {
            "neutral".to_string()
        } else {
            "fusion_active".to_string()
        },
        warnings: Vec::new(),
        no_trade_reasons: vec!["analysis only".to_string()],
        signals,
    }
}

fn source_signal() -> ToxicSignal {
    ToxicSignal {
        signal_id: "fusion-1".to_string(),
        symbol: "BTC-PERP".to_string(),
        ts_ms: 12_000,
        signal_type: ToxicSignalType::ShortBiasToxicFlow,
        direction: ToxicSignalDirection::ShortBias,
        toxicity_score: 88,
        confidence: ToxicConfidence::High,
        primary_reason: "Buy-side delta failed near resistance.".to_string(),
        reason: vec!["classified as short-bias toxic flow candidate".to_string()],
        supporting_evidence: vec![ToxicSupportingEvidence {
            source: "active_trade".to_string(),
            signal_id: "active-1".to_string(),
            signal_type: "one_hour_delta_buy_dominant".to_string(),
            contribution_score: 70,
            summary: "1H delta failed to continue higher.".to_string(),
        }],
        invalidation_price: Some(100_300.0),
        suggested_stop_distance_usd: Some(240.0),
        chase_risk: ToxicChaseRisk::Medium,
        no_trade_reasons: vec!["reference only".to_string()],
        linked_active_trade_signal_ids: vec!["active-1".to_string()],
        linked_liquidation_signal_ids: vec!["liq-1".to_string()],
        linked_wall_lifecycle_signal_ids: vec!["wall-event-1".to_string()],
        linked_wall_interpretation_signal_ids: vec!["wall-int-1".to_string()],
        linked_structural_signal_ids: vec!["struct-1".to_string()],
        read_only: true,
        detector_version: None,
        score_breakdown: None,
        evidence: None,
        data_quality: None,
        dedupe_key: None,
        resolution_status: None,
    }
}

fn active_trade_recent(signals: Vec<ActiveTradeToxicSignal>) -> ActiveTradeToxicityRecentResponse {
    ActiveTradeToxicityRecentResponse {
        read_only: true,
        runtime_modified: false,
        mode: "analysis_only".to_string(),
        selected_symbol: "BTC-PERP".to_string(),
        status: "buy_toxicity_watch".to_string(),
        score: 78.0,
        side_bias: "buy".to_string(),
        warnings: Vec::new(),
        no_trade_reasons: vec!["analysis only".to_string()],
        signals,
    }
}

fn active_trade_signal() -> ActiveTradeToxicSignal {
    ActiveTradeToxicSignal {
        signal_id: "active-1".to_string(),
        symbol: "BTC-PERP".to_string(),
        ts_ms: 10_000,
        signal_type: ActiveTradeToxicSignalType::OneHourDeltaBuyDominant,
        side: ToxicSide::Buy,
        timeframe: Some("1h".to_string()),
        candle_open_ms: Some(0),
        candle_close_ms: Some(3_600_000),
        window_ms: 3_600_000,
        delta: Some(2_300.0),
        abs_delta: Some(2_300.0),
        threshold: Some(2_000.0),
        aggressive_volume: 17.0,
        notional_usd: 1_700_000.0,
        trade_count: 20,
        cvd_delta: 800_000.0,
        buy_volume: 18.0,
        sell_volume: 3.0,
        imbalance_ratio: 0.8,
        open: Some(100_000.0),
        high: Some(100_250.0),
        low: Some(99_980.0),
        close: Some(100_050.0),
        price_impact_bps: Some(1.0),
        price_change_bps: Some(5.0),
        upper_wick_ratio: Some(0.3),
        lower_wick_ratio: Some(0.05),
        markout_5s: Some(-0.8),
        markout_15s: Some(-0.6),
        markout_60s: None,
        toxicity_score: 82,
        confidence: ToxicConfidence::High,
        reason: vec!["active trade test".to_string()],
        read_only: true,
    }
}

fn liquidation_recent(signals: Vec<LiquidationToxicSignal>) -> LiquidationToxicityRecentResponse {
    LiquidationToxicityRecentResponse {
        read_only: true,
        runtime_modified: false,
        mode: "analysis_only".to_string(),
        selected_symbol: "BTC-PERP".to_string(),
        warnings: Vec::new(),
        no_trade_reasons: vec!["analysis only".to_string()],
        signals,
    }
}

fn liquidation_signal() -> LiquidationToxicSignal {
    LiquidationToxicSignal {
        signal_id: "liq-1".to_string(),
        symbol: "BTC-PERP".to_string(),
        ts_ms: 10_500,
        signal_type: LiquidationToxicSignalType::UpsideLiquidationMagnet,
        direction: LiquidationToxicDirection::Upside,
        current_price: 100_050.0,
        cluster_price: 100_220.0,
        distance_usd: 170.0,
        distance_bps: 17.0,
        estimated_liquidation_notional: 2_400_000.0,
        cluster_density_score: 80,
        magnet_score: 76,
        cascade_score: 32,
        linked_active_trade_signal_ids: vec!["active-1".to_string()],
        toxicity_score: 75,
        confidence: ToxicConfidence::Medium,
        reason: vec!["liquidation test".to_string()],
        read_only: true,
    }
}

fn wall_lifecycle_report(
    recent_events: Vec<OrderbookWallLifecycleEvent>,
) -> OrderbookWallLifecycleReport {
    OrderbookWallLifecycleReport {
        read_only: true,
        runtime_modified: false,
        analysis_mode: "analysis_only".to_string(),
        symbol: "BTC-PERP".to_string(),
        generated_at_ms: 12_000,
        status: "tracking".to_string(),
        tracked_walls: Vec::new(),
        recent_events,
        toxicity_candidates: Vec::new(),
        warnings: Vec::new(),
        no_trade_reasons: vec!["analysis only".to_string()],
    }
}

fn wall_event() -> OrderbookWallLifecycleEvent {
    OrderbookWallLifecycleEvent {
        event_id: "wall-event-1".to_string(),
        wall_id: "wall-1".to_string(),
        symbol: "BTC-PERP".to_string(),
        event_type: OrderbookWallEventType::WallRemoved,
        side: OrderbookWallSide::Ask,
        price: 100_220.0,
        notional: 1_100_000.0,
        distance_bps: 12.0,
        observed_at_ms: 10_600,
        reason: "wall event".to_string(),
    }
}

fn wall_interpretation_report(
    signals: Vec<OrderbookWallInterpretationSignal>,
) -> OrderbookWallInterpretationReport {
    OrderbookWallInterpretationReport {
        read_only: true,
        runtime_modified: false,
        analysis_mode: "analysis_only".to_string(),
        selected_symbol: "BTC-PERP".to_string(),
        generated_at_ms: 12_000,
        status: "interpretation_active".to_string(),
        signals,
        warnings: Vec::new(),
        no_trade_reasons: vec!["analysis only".to_string()],
    }
}

fn wall_interpretation_signal() -> OrderbookWallInterpretationSignal {
    OrderbookWallInterpretationSignal {
        signal_id: "wall-int-1".to_string(),
        symbol: "BTC-PERP".to_string(),
        ts_ms: 10_700,
        wall_id: "wall-1".to_string(),
        signal_type: OrderbookWallInterpretationType::AskAbsorption,
        side: OrderbookWallSide::Ask,
        wall_price: 100_220.0,
        wall_notional_usd: 1_200_000.0,
        distance_to_mid_bps: 15.0,
        persistence_ms: 8_000,
        touch_count: 1,
        consumed_ratio: 0.2,
        cancel_ratio: 0.6,
        moved_count: 1,
        aggressive_volume_against_wall: Some(12.0),
        post_touch_markout_bps: Some(-0.7),
        spoof_score: 30,
        absorption_score: 82,
        inducement_score: 40,
        toxicity_score: 79,
        confidence: ToxicConfidence::High,
        reason: vec!["wall interpretation test".to_string()],
        read_only: true,
    }
}

fn structural_recent(signals: Vec<StructuralToxicSignal>) -> StructuralToxicityRecentResponse {
    StructuralToxicityRecentResponse {
        read_only: true,
        runtime_modified: false,
        mode: "analysis_only".to_string(),
        selected_symbol: "BTC-PERP".to_string(),
        status: "structure_active".to_string(),
        warnings: Vec::new(),
        no_trade_reasons: vec!["analysis only".to_string()],
        signals,
    }
}

fn structural_signal() -> StructuralToxicSignal {
    StructuralToxicSignal {
        signal_id: "struct-1".to_string(),
        symbol: "BTC-PERP".to_string(),
        ts_ms: 10_800,
        signal_type: StructuralToxicSignalType::FailedBreakout,
        direction: StructuralToxicDirection::UpsideTrap,
        level_type: StructuralLevelType::RecentSwingHigh,
        level_price: 100_220.0,
        current_price: 100_050.0,
        sweep_distance_usd: Some(170.0),
        sweep_distance_bps: Some(17.0),
        reclaim_or_reject: true,
        time_outside_level_ms: Some(120_000),
        linked_active_trade_signal_ids: vec!["active-1".to_string()],
        linked_liquidation_signal_ids: vec!["liq-1".to_string()],
        linked_wall_signal_ids: vec!["wall-event-1".to_string()],
        linked_wall_interpretation_signal_ids: vec!["wall-int-1".to_string()],
        toxicity_score: 84,
        confidence: ToxicConfidence::High,
        reason: vec!["structural test".to_string()],
        read_only: true,
    }
}
