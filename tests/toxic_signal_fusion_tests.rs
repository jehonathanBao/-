use btc_toxic_flow_monitor_rs::{
    toxicity::toxic_signal_fusion::analyze_toxic_signal_fusion,
    types::{
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
        toxic_signal::ToxicSignalType,
    },
};

#[test]
fn buy_delta_absorption_failed_breakout_produces_short_bias() {
    let report = analyze_toxic_signal_fusion(
        "BTC-PERP",
        &active_trade_recent(vec![one_hour_buy_signal()]),
        &liquidation_recent(vec![upside_liquidation_signal()]),
        &wall_lifecycle_report(vec![ask_wall_event()]),
        &wall_interpretation_report(vec![ask_absorption_signal()]),
        &structural_recent(vec![failed_breakout_signal()]),
    );

    assert!(report
        .signals
        .iter()
        .any(|signal| signal.signal_type == ToxicSignalType::ShortBiasToxicFlow));
}

#[test]
fn sell_delta_absorption_failed_breakdown_produces_long_bias() {
    let report = analyze_toxic_signal_fusion(
        "BTC-PERP",
        &active_trade_recent(vec![one_hour_sell_signal()]),
        &liquidation_recent(vec![downside_liquidation_signal()]),
        &wall_lifecycle_report(vec![bid_wall_event()]),
        &wall_interpretation_report(vec![bid_absorption_signal()]),
        &structural_recent(vec![failed_breakdown_signal()]),
    );

    assert!(report
        .signals
        .iter()
        .any(|signal| signal.signal_type == ToxicSignalType::LongBiasToxicFlow));
}

#[test]
fn upside_liquidation_and_pull_above_without_failure_produces_upside_squeeze() {
    let report = analyze_toxic_signal_fusion(
        "BTC-PERP",
        &active_trade_recent(vec![one_hour_buy_signal()]),
        &liquidation_recent(vec![upside_liquidation_signal()]),
        &wall_lifecycle_report(vec![ask_wall_event()]),
        &wall_interpretation_report(vec![liquidity_pull_above_signal()]),
        &structural_recent(Vec::new()),
    );

    assert!(report
        .signals
        .iter()
        .any(|signal| signal.signal_type == ToxicSignalType::SqueezeRiskUpside));
}

#[test]
fn downside_liquidation_and_pull_below_without_failure_produces_downside_squeeze() {
    let report = analyze_toxic_signal_fusion(
        "BTC-PERP",
        &active_trade_recent(vec![one_hour_sell_signal()]),
        &liquidation_recent(vec![downside_liquidation_signal()]),
        &wall_lifecycle_report(vec![bid_wall_event()]),
        &wall_interpretation_report(vec![liquidity_pull_below_signal()]),
        &structural_recent(Vec::new()),
    );

    assert!(report
        .signals
        .iter()
        .any(|signal| signal.signal_type == ToxicSignalType::SqueezeRiskDownside));
}

#[test]
fn bull_and_bear_trap_structural_signals_are_promoted() {
    let report = analyze_toxic_signal_fusion(
        "BTC-PERP",
        &active_trade_recent(vec![one_hour_buy_signal(), one_hour_sell_signal()]),
        &empty_liquidation_recent(),
        &wall_lifecycle_report(Vec::new()),
        &empty_wall_interpretation_report(),
        &structural_recent(vec![bull_trap_signal(), bear_trap_signal()]),
    );

    assert!(report
        .signals
        .iter()
        .any(|signal| signal.signal_type == ToxicSignalType::BullTrapRisk));
    assert!(report
        .signals
        .iter()
        .any(|signal| signal.signal_type == ToxicSignalType::BearTrapRisk));
}

#[test]
fn conflicting_directional_evidence_produces_trap_and_no_trade() {
    let report = analyze_toxic_signal_fusion(
        "BTC-PERP",
        &active_trade_recent(vec![one_hour_buy_signal(), one_hour_sell_signal()]),
        &liquidation_recent(vec![
            upside_liquidation_signal(),
            downside_liquidation_signal(),
        ]),
        &wall_lifecycle_report(vec![ask_wall_event(), bid_wall_event()]),
        &wall_interpretation_report(vec![spoof_ask_signal(), bid_absorption_signal()]),
        &structural_recent(vec![failed_breakout_signal(), failed_breakdown_signal()]),
    );

    assert!(report
        .signals
        .iter()
        .any(|signal| signal.signal_type == ToxicSignalType::TrapRisk));
    let no_trade = report
        .signals
        .iter()
        .find(|signal| signal.signal_type == ToxicSignalType::NoTradeChopRisk)
        .expect("no-trade signal");
    assert!(!no_trade.no_trade_reasons.is_empty());
}

#[test]
fn bias_signals_include_invalidation_price() {
    let short_report = analyze_toxic_signal_fusion(
        "BTC-PERP",
        &active_trade_recent(vec![one_hour_buy_signal()]),
        &liquidation_recent(vec![upside_liquidation_signal()]),
        &wall_lifecycle_report(vec![ask_wall_event()]),
        &wall_interpretation_report(vec![ask_absorption_signal()]),
        &structural_recent(vec![failed_breakout_signal()]),
    );
    assert!(short_report
        .signals
        .iter()
        .find(|signal| signal.signal_type == ToxicSignalType::ShortBiasToxicFlow)
        .and_then(|signal| signal.invalidation_price)
        .is_some());

    let long_report = analyze_toxic_signal_fusion(
        "BTC-PERP",
        &active_trade_recent(vec![one_hour_sell_signal()]),
        &liquidation_recent(vec![downside_liquidation_signal()]),
        &wall_lifecycle_report(vec![bid_wall_event()]),
        &wall_interpretation_report(vec![bid_absorption_signal()]),
        &structural_recent(vec![failed_breakdown_signal()]),
    );
    assert!(long_report
        .signals
        .iter()
        .find(|signal| signal.signal_type == ToxicSignalType::LongBiasToxicFlow)
        .and_then(|signal| signal.invalidation_price)
        .is_some());
}

#[test]
fn fusion_signals_are_read_only_scored_and_supported() {
    let report = analyze_toxic_signal_fusion(
        "BTC-PERP",
        &active_trade_recent(vec![one_hour_buy_signal(), one_hour_sell_signal()]),
        &liquidation_recent(vec![
            upside_liquidation_signal(),
            downside_liquidation_signal(),
        ]),
        &wall_lifecycle_report(vec![ask_wall_event(), bid_wall_event()]),
        &wall_interpretation_report(vec![
            ask_absorption_signal(),
            bid_absorption_signal(),
            spoof_ask_signal(),
        ]),
        &structural_recent(vec![
            failed_breakout_signal(),
            failed_breakdown_signal(),
            liquidity_sweep_high_signal(),
        ]),
    );

    assert!(!report.signals.is_empty());
    for signal in &report.signals {
        assert!(signal.read_only);
        assert!(signal.toxicity_score <= 100);
        assert!(!signal.supporting_evidence.is_empty());
        assert!(!signal.reason.is_empty());
    }
}

fn active_trade_recent(signals: Vec<ActiveTradeToxicSignal>) -> ActiveTradeToxicityRecentResponse {
    ActiveTradeToxicityRecentResponse {
        read_only: true,
        runtime_modified: false,
        mode: "analysis_only".to_string(),
        selected_symbol: "BTC-PERP".to_string(),
        status: "high_toxicity_watch".to_string(),
        score: 78.0,
        side_bias: "buy".to_string(),
        warnings: Vec::new(),
        no_trade_reasons: vec!["analysis only".to_string()],
        signals,
    }
}

fn one_hour_buy_signal() -> ActiveTradeToxicSignal {
    ActiveTradeToxicSignal {
        signal_id: "active-1h-buy".to_string(),
        symbol: "BTC-PERP".to_string(),
        ts_ms: 10_000,
        signal_type: ActiveTradeToxicSignalType::OneHourDeltaBuyDominant,
        side: ToxicSide::Buy,
        timeframe: Some("1h".to_string()),
        candle_open_ms: Some(0),
        candle_close_ms: Some(3_600_000),
        window_ms: 3_600_000,
        delta: Some(2_400.0),
        abs_delta: Some(2_400.0),
        threshold: Some(2_000.0),
        aggressive_volume: 18.0,
        notional_usd: 1_800_000.0,
        trade_count: 24,
        cvd_delta: 900_000.0,
        buy_volume: 19.0,
        sell_volume: 4.0,
        imbalance_ratio: 0.81,
        open: Some(100_000.0),
        high: Some(100_220.0),
        low: Some(99_940.0),
        close: Some(100_060.0),
        price_impact_bps: Some(1.5),
        price_change_bps: Some(6.0),
        upper_wick_ratio: Some(0.42),
        lower_wick_ratio: Some(0.08),
        markout_5s: Some(-1.2),
        markout_15s: Some(-0.8),
        markout_60s: None,
        toxicity_score: 82,
        confidence: ToxicConfidence::High,
        reason: vec!["test one hour buy".to_string()],
        read_only: true,
    }
}

fn one_hour_sell_signal() -> ActiveTradeToxicSignal {
    ActiveTradeToxicSignal {
        signal_id: "active-1h-sell".to_string(),
        symbol: "BTC-PERP".to_string(),
        ts_ms: 11_000,
        signal_type: ActiveTradeToxicSignalType::OneHourDeltaSellDominant,
        side: ToxicSide::Sell,
        timeframe: Some("1h".to_string()),
        candle_open_ms: Some(0),
        candle_close_ms: Some(3_600_000),
        window_ms: 3_600_000,
        delta: Some(-2_500.0),
        abs_delta: Some(2_500.0),
        threshold: Some(2_000.0),
        aggressive_volume: 17.0,
        notional_usd: 1_700_000.0,
        trade_count: 22,
        cvd_delta: -920_000.0,
        buy_volume: 3.0,
        sell_volume: 18.0,
        imbalance_ratio: 0.79,
        open: Some(100_000.0),
        high: Some(100_040.0),
        low: Some(99_760.0),
        close: Some(99_940.0),
        price_impact_bps: Some(-1.6),
        price_change_bps: Some(-6.0),
        upper_wick_ratio: Some(0.08),
        lower_wick_ratio: Some(0.40),
        markout_5s: Some(1.1),
        markout_15s: Some(0.7),
        markout_60s: None,
        toxicity_score: 81,
        confidence: ToxicConfidence::High,
        reason: vec!["test one hour sell".to_string()],
        read_only: true,
    }
}

fn empty_liquidation_recent() -> LiquidationToxicityRecentResponse {
    liquidation_recent(Vec::new())
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

fn upside_liquidation_signal() -> LiquidationToxicSignal {
    LiquidationToxicSignal {
        signal_id: "liq-up".to_string(),
        symbol: "BTC-PERP".to_string(),
        ts_ms: 10_500,
        signal_type: LiquidationToxicSignalType::UpsideLiquidationMagnet,
        direction: LiquidationToxicDirection::Upside,
        current_price: 100_060.0,
        cluster_price: 100_210.0,
        distance_usd: 150.0,
        distance_bps: 15.0,
        estimated_liquidation_notional: 2_400_000.0,
        cluster_density_score: 80,
        magnet_score: 76,
        cascade_score: 38,
        linked_active_trade_signal_ids: vec!["active-1h-buy".to_string()],
        toxicity_score: 75,
        confidence: ToxicConfidence::Medium,
        reason: vec!["upside cluster".to_string()],
        read_only: true,
    }
}

fn downside_liquidation_signal() -> LiquidationToxicSignal {
    LiquidationToxicSignal {
        signal_id: "liq-down".to_string(),
        symbol: "BTC-PERP".to_string(),
        ts_ms: 11_500,
        signal_type: LiquidationToxicSignalType::DownsideLiquidationMagnet,
        direction: LiquidationToxicDirection::Downside,
        current_price: 99_940.0,
        cluster_price: 99_790.0,
        distance_usd: 150.0,
        distance_bps: 15.0,
        estimated_liquidation_notional: 2_300_000.0,
        cluster_density_score: 79,
        magnet_score: 74,
        cascade_score: 36,
        linked_active_trade_signal_ids: vec!["active-1h-sell".to_string()],
        toxicity_score: 74,
        confidence: ToxicConfidence::Medium,
        reason: vec!["downside cluster".to_string()],
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
        generated_at_ms: recent_events
            .iter()
            .map(|event| event.observed_at_ms)
            .max()
            .unwrap_or(0),
        status: if recent_events.is_empty() {
            "insufficient_data".to_string()
        } else {
            "tracking".to_string()
        },
        tracked_walls: Vec::new(),
        recent_events,
        toxicity_candidates: Vec::new(),
        warnings: Vec::new(),
        no_trade_reasons: vec!["analysis only".to_string()],
    }
}

fn ask_wall_event() -> OrderbookWallLifecycleEvent {
    OrderbookWallLifecycleEvent {
        event_id: "wall-event-ask".to_string(),
        wall_id: "wall-ask-1".to_string(),
        symbol: "BTC-PERP".to_string(),
        event_type: OrderbookWallEventType::WallRemoved,
        side: OrderbookWallSide::Ask,
        price: 100_210.0,
        notional: 1_200_000.0,
        distance_bps: 15.0,
        observed_at_ms: 10_400,
        reason: "ask wall event".to_string(),
    }
}

fn bid_wall_event() -> OrderbookWallLifecycleEvent {
    OrderbookWallLifecycleEvent {
        event_id: "wall-event-bid".to_string(),
        wall_id: "wall-bid-1".to_string(),
        symbol: "BTC-PERP".to_string(),
        event_type: OrderbookWallEventType::WallRemoved,
        side: OrderbookWallSide::Bid,
        price: 99_790.0,
        notional: 1_180_000.0,
        distance_bps: 15.0,
        observed_at_ms: 11_400,
        reason: "bid wall event".to_string(),
    }
}

fn empty_wall_interpretation_report() -> OrderbookWallInterpretationReport {
    wall_interpretation_report(Vec::new())
}

fn wall_interpretation_report(
    signals: Vec<OrderbookWallInterpretationSignal>,
) -> OrderbookWallInterpretationReport {
    OrderbookWallInterpretationReport {
        read_only: true,
        runtime_modified: false,
        analysis_mode: "analysis_only".to_string(),
        selected_symbol: "BTC-PERP".to_string(),
        generated_at_ms: signals.iter().map(|signal| signal.ts_ms).max().unwrap_or(0),
        status: if signals.is_empty() {
            "neutral".to_string()
        } else {
            "interpretation_active".to_string()
        },
        signals,
        warnings: Vec::new(),
        no_trade_reasons: vec!["analysis only".to_string()],
    }
}

fn ask_absorption_signal() -> OrderbookWallInterpretationSignal {
    wall_signal(
        "wall-int-ask-absorb",
        OrderbookWallInterpretationType::AskAbsorption,
        OrderbookWallSide::Ask,
        100_210.0,
        10_400,
    )
}

fn bid_absorption_signal() -> OrderbookWallInterpretationSignal {
    wall_signal(
        "wall-int-bid-absorb",
        OrderbookWallInterpretationType::BidAbsorption,
        OrderbookWallSide::Bid,
        99_790.0,
        11_400,
    )
}

fn spoof_ask_signal() -> OrderbookWallInterpretationSignal {
    wall_signal(
        "wall-int-spoof-ask",
        OrderbookWallInterpretationType::SpoofAskWall,
        OrderbookWallSide::Ask,
        100_210.0,
        10_300,
    )
}

fn liquidity_pull_above_signal() -> OrderbookWallInterpretationSignal {
    wall_signal(
        "wall-int-pull-above",
        OrderbookWallInterpretationType::LiquidityPullAbove,
        OrderbookWallSide::Ask,
        100_210.0,
        10_350,
    )
}

fn liquidity_pull_below_signal() -> OrderbookWallInterpretationSignal {
    wall_signal(
        "wall-int-pull-below",
        OrderbookWallInterpretationType::LiquidityPullBelow,
        OrderbookWallSide::Bid,
        99_790.0,
        11_350,
    )
}

fn wall_signal(
    signal_id: &str,
    signal_type: OrderbookWallInterpretationType,
    side: OrderbookWallSide,
    wall_price: f64,
    ts_ms: u64,
) -> OrderbookWallInterpretationSignal {
    OrderbookWallInterpretationSignal {
        signal_id: signal_id.to_string(),
        symbol: "BTC-PERP".to_string(),
        ts_ms,
        wall_id: format!("{signal_id}-wall"),
        signal_type,
        side,
        wall_price,
        wall_notional_usd: 1_200_000.0,
        distance_to_mid_bps: 15.0,
        persistence_ms: 8_000,
        touch_count: 1,
        consumed_ratio: 0.12,
        cancel_ratio: 0.88,
        moved_count: 1,
        aggressive_volume_against_wall: Some(12.0),
        post_touch_markout_bps: Some(-0.9),
        spoof_score: 82,
        absorption_score: 78,
        inducement_score: 74,
        toxicity_score: 80,
        confidence: ToxicConfidence::High,
        reason: vec!["test wall interpretation".to_string()],
        read_only: true,
    }
}

fn structural_recent(signals: Vec<StructuralToxicSignal>) -> StructuralToxicityRecentResponse {
    StructuralToxicityRecentResponse {
        read_only: true,
        runtime_modified: false,
        mode: "analysis_only".to_string(),
        selected_symbol: "BTC-PERP".to_string(),
        status: if signals.is_empty() {
            "neutral".to_string()
        } else {
            "structure_active".to_string()
        },
        warnings: Vec::new(),
        no_trade_reasons: vec!["analysis only".to_string()],
        signals,
    }
}

fn failed_breakout_signal() -> StructuralToxicSignal {
    structural_signal(
        "struct-failed-breakout",
        StructuralToxicSignalType::FailedBreakout,
        StructuralToxicDirection::UpsideTrap,
        StructuralLevelType::RecentSwingHigh,
        100_210.0,
        100_060.0,
        10_400,
    )
}

fn failed_breakdown_signal() -> StructuralToxicSignal {
    structural_signal(
        "struct-failed-breakdown",
        StructuralToxicSignalType::FailedBreakdown,
        StructuralToxicDirection::DownsideTrap,
        StructuralLevelType::RecentSwingLow,
        99_790.0,
        99_940.0,
        11_400,
    )
}

fn liquidity_sweep_high_signal() -> StructuralToxicSignal {
    structural_signal(
        "struct-sweep-high",
        StructuralToxicSignalType::LiquiditySweepHigh,
        StructuralToxicDirection::UpsideTrap,
        StructuralLevelType::RecentSwingHigh,
        100_220.0,
        100_060.0,
        10_450,
    )
}

fn bull_trap_signal() -> StructuralToxicSignal {
    structural_signal(
        "struct-bull-trap",
        StructuralToxicSignalType::BullTrapCandidate,
        StructuralToxicDirection::DownsideTrap,
        StructuralLevelType::RecentSwingLow,
        99_790.0,
        99_940.0,
        11_450,
    )
}

fn bear_trap_signal() -> StructuralToxicSignal {
    structural_signal(
        "struct-bear-trap",
        StructuralToxicSignalType::BearTrapCandidate,
        StructuralToxicDirection::UpsideTrap,
        StructuralLevelType::RecentSwingHigh,
        100_210.0,
        100_060.0,
        10_460,
    )
}

fn structural_signal(
    signal_id: &str,
    signal_type: StructuralToxicSignalType,
    direction: StructuralToxicDirection,
    level_type: StructuralLevelType,
    level_price: f64,
    current_price: f64,
    ts_ms: u64,
) -> StructuralToxicSignal {
    StructuralToxicSignal {
        signal_id: signal_id.to_string(),
        symbol: "BTC-PERP".to_string(),
        ts_ms,
        signal_type,
        direction,
        level_type,
        level_price,
        current_price,
        sweep_distance_usd: Some((level_price - current_price).abs()),
        sweep_distance_bps: Some(15.0),
        reclaim_or_reject: true,
        time_outside_level_ms: Some(60_000),
        linked_active_trade_signal_ids: vec!["active-1h-buy".to_string()],
        linked_liquidation_signal_ids: vec!["liq-up".to_string()],
        linked_wall_signal_ids: vec!["wall-event-ask".to_string()],
        linked_wall_interpretation_signal_ids: vec!["wall-int-ask-absorb".to_string()],
        toxicity_score: 82,
        confidence: ToxicConfidence::High,
        reason: vec!["structural test signal".to_string()],
        read_only: true,
    }
}
