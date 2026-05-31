use btc_toxic_flow_monitor_rs::{
    toxicity::orderbook_wall_interpretation::analyze_orderbook_wall_interpretation,
    types::{
        liquidation::{
            LiquidationToxicDirection, LiquidationToxicSignal, LiquidationToxicSignalType,
            LiquidationToxicityRecentResponse,
        },
        orderbook_wall::{
            OrderbookWallEventType, OrderbookWallInterpretationType, OrderbookWallLifecycleEvent,
            OrderbookWallLifecycleReport, OrderbookWallSide, TrackedOrderbookWall,
        },
        toxic_flow::{
            ActiveTradeToxicSignal, ActiveTradeToxicSignalType, ActiveTradeToxicityRecentResponse,
            ToxicConfidence, ToxicSide,
        },
    },
};

#[test]
fn fake_ask_wall_near_touch_produces_spoof_ask_signal() {
    let report = analyze_orderbook_wall_interpretation(
        "BTC-PERP",
        &lifecycle_report(
            vec![tracked_wall(
                "ask-1",
                OrderbookWallSide::Ask,
                100_050.0,
                1_200_000.0,
                8_000,
                1,
            )],
            vec![event(
                "ask-1",
                OrderbookWallEventType::FakeWallCandidate,
                OrderbookWallSide::Ask,
                4_000,
                100_050.0,
                1_200_000.0,
            )],
        ),
        &empty_active_trade_recent(),
        &empty_liquidation_recent(),
    );

    assert!(report
        .signals
        .iter()
        .any(|signal| signal.signal_type == OrderbookWallInterpretationType::SpoofAskWall));
}

#[test]
fn fake_bid_wall_near_touch_produces_spoof_bid_signal() {
    let report = analyze_orderbook_wall_interpretation(
        "BTC-PERP",
        &lifecycle_report(
            vec![tracked_wall(
                "bid-1",
                OrderbookWallSide::Bid,
                99_950.0,
                1_050_000.0,
                7_000,
                1,
            )],
            vec![event(
                "bid-1",
                OrderbookWallEventType::FakeWallCandidate,
                OrderbookWallSide::Bid,
                4_000,
                99_950.0,
                1_050_000.0,
            )],
        ),
        &empty_active_trade_recent(),
        &empty_liquidation_recent(),
    );

    assert!(report
        .signals
        .iter()
        .any(|signal| signal.signal_type == OrderbookWallInterpretationType::SpoofBidWall));
}

#[test]
fn persistent_ask_wall_is_detected_after_long_lifetime_and_touches() {
    let report = analyze_orderbook_wall_interpretation(
        "BTC-PERP",
        &lifecycle_report(
            vec![tracked_wall(
                "ask-persist",
                OrderbookWallSide::Ask,
                100_080.0,
                1_800_000.0,
                16_000,
                3,
            )],
            vec![event(
                "ask-persist",
                OrderbookWallEventType::WallTouched,
                OrderbookWallSide::Ask,
                16_000,
                100_080.0,
                1_800_000.0,
            )],
        ),
        &empty_active_trade_recent(),
        &empty_liquidation_recent(),
    );

    assert!(report
        .signals
        .iter()
        .any(|signal| signal.signal_type == OrderbookWallInterpretationType::PersistentAskWall));
}

#[test]
fn persistent_bid_wall_is_detected_after_long_lifetime_and_touches() {
    let report = analyze_orderbook_wall_interpretation(
        "BTC-PERP",
        &lifecycle_report(
            vec![tracked_wall(
                "bid-persist",
                OrderbookWallSide::Bid,
                99_920.0,
                1_600_000.0,
                14_000,
                2,
            )],
            vec![event(
                "bid-persist",
                OrderbookWallEventType::WallTouched,
                OrderbookWallSide::Bid,
                14_000,
                99_920.0,
                1_600_000.0,
            )],
        ),
        &empty_active_trade_recent(),
        &empty_liquidation_recent(),
    );

    assert!(report
        .signals
        .iter()
        .any(|signal| signal.signal_type == OrderbookWallInterpretationType::PersistentBidWall));
}

#[test]
fn ask_wall_absorption_needs_bullish_flow_without_clean_follow_through() {
    let report = analyze_orderbook_wall_interpretation(
        "BTC-PERP",
        &lifecycle_report(
            vec![tracked_wall(
                "ask-absorb",
                OrderbookWallSide::Ask,
                100_100.0,
                950_000.0,
                12_000,
                2,
            )],
            vec![event(
                "ask-absorb",
                OrderbookWallEventType::AbsorptionCandidate,
                OrderbookWallSide::Ask,
                12_000,
                100_100.0,
                950_000.0,
            )],
        ),
        &active_trade_recent(vec![active_signal(
            "buy-1",
            ActiveTradeToxicSignalType::LargeAggressiveBuy,
            ToxicSide::Buy,
        )]),
        &empty_liquidation_recent(),
    );

    assert!(report
        .signals
        .iter()
        .any(|signal| signal.signal_type == OrderbookWallInterpretationType::AskAbsorption));
}

#[test]
fn bid_wall_absorption_needs_bearish_flow_without_clean_follow_through() {
    let report = analyze_orderbook_wall_interpretation(
        "BTC-PERP",
        &lifecycle_report(
            vec![tracked_wall(
                "bid-absorb",
                OrderbookWallSide::Bid,
                99_900.0,
                970_000.0,
                12_000,
                2,
            )],
            vec![event(
                "bid-absorb",
                OrderbookWallEventType::AbsorptionCandidate,
                OrderbookWallSide::Bid,
                12_000,
                99_900.0,
                970_000.0,
            )],
        ),
        &active_trade_recent(vec![active_signal(
            "sell-1",
            ActiveTradeToxicSignalType::LargeAggressiveSell,
            ToxicSide::Sell,
        )]),
        &empty_liquidation_recent(),
    );

    assert!(report
        .signals
        .iter()
        .any(|signal| signal.signal_type == OrderbookWallInterpretationType::BidAbsorption));
}

#[test]
fn ask_wall_removal_and_consumption_create_pull_and_resistance_failure_signals() {
    let report = analyze_orderbook_wall_interpretation(
        "BTC-PERP",
        &lifecycle_report(
            vec![tracked_wall(
                "ask-break",
                OrderbookWallSide::Ask,
                100_120.0,
                1_300_000.0,
                9_000,
                1,
            )],
            vec![
                event(
                    "ask-break",
                    OrderbookWallEventType::WallRemoved,
                    OrderbookWallSide::Ask,
                    9_500,
                    100_120.0,
                    1_300_000.0,
                ),
                event(
                    "ask-break",
                    OrderbookWallEventType::WallConsumed,
                    OrderbookWallSide::Ask,
                    10_000,
                    100_120.0,
                    1_300_000.0,
                ),
            ],
        ),
        &empty_active_trade_recent(),
        &empty_liquidation_recent(),
    );

    assert!(report
        .signals
        .iter()
        .any(|signal| signal.signal_type == OrderbookWallInterpretationType::LiquidityPullAbove));
    assert!(
        report
            .signals
            .iter()
            .any(|signal| signal.signal_type
                == OrderbookWallInterpretationType::ResistanceWallFailure)
    );
}

#[test]
fn bid_wall_removal_and_consumption_create_pull_and_support_failure_signals() {
    let report = analyze_orderbook_wall_interpretation(
        "BTC-PERP",
        &lifecycle_report(
            vec![tracked_wall(
                "bid-break",
                OrderbookWallSide::Bid,
                99_880.0,
                1_100_000.0,
                9_000,
                1,
            )],
            vec![
                event(
                    "bid-break",
                    OrderbookWallEventType::WallRemoved,
                    OrderbookWallSide::Bid,
                    9_500,
                    99_880.0,
                    1_100_000.0,
                ),
                event(
                    "bid-break",
                    OrderbookWallEventType::WallConsumed,
                    OrderbookWallSide::Bid,
                    10_000,
                    99_880.0,
                    1_100_000.0,
                ),
            ],
        ),
        &empty_active_trade_recent(),
        &empty_liquidation_recent(),
    );

    assert!(report
        .signals
        .iter()
        .any(|signal| signal.signal_type == OrderbookWallInterpretationType::LiquidityPullBelow));
    assert!(report
        .signals
        .iter()
        .any(|signal| signal.signal_type == OrderbookWallInterpretationType::SupportWallFailure));
}

#[test]
fn interpretation_stays_analysis_only_even_with_active_and_liquidation_confluence() {
    let report = analyze_orderbook_wall_interpretation(
        "BTC-PERP",
        &lifecycle_report(
            vec![tracked_wall(
                "ask-confluence",
                OrderbookWallSide::Ask,
                100_150.0,
                1_400_000.0,
                6_000,
                1,
            )],
            vec![event(
                "ask-confluence",
                OrderbookWallEventType::FakeWallCandidate,
                OrderbookWallSide::Ask,
                6_500,
                100_150.0,
                1_400_000.0,
            )],
        ),
        &active_trade_recent(vec![active_signal(
            "buy-2",
            ActiveTradeToxicSignalType::OneHourDeltaBuyDominant,
            ToxicSide::Buy,
        )]),
        &liquidation_recent(vec![liquidation_signal(
            LiquidationToxicSignalType::UpsideLiquidationMagnet,
            LiquidationToxicDirection::Upside,
        )]),
    );

    assert!(report.read_only);
    assert!(!report.runtime_modified);
    assert_eq!(report.analysis_mode, "analysis_only");
    assert!(report.signals.iter().all(|signal| signal.read_only));
}

#[test]
fn all_scores_are_bounded_and_reasons_are_present() {
    let report = analyze_orderbook_wall_interpretation(
        "BTC-PERP",
        &lifecycle_report(
            vec![tracked_wall(
                "ask-all",
                OrderbookWallSide::Ask,
                100_050.0,
                1_200_000.0,
                12_000,
                2,
            )],
            vec![
                event(
                    "ask-all",
                    OrderbookWallEventType::FakeWallCandidate,
                    OrderbookWallSide::Ask,
                    4_000,
                    100_050.0,
                    1_200_000.0,
                ),
                event(
                    "ask-all",
                    OrderbookWallEventType::AbsorptionCandidate,
                    OrderbookWallSide::Ask,
                    5_000,
                    100_050.0,
                    1_200_000.0,
                ),
                event(
                    "ask-all",
                    OrderbookWallEventType::LiquidityInducementCandidate,
                    OrderbookWallSide::Ask,
                    6_000,
                    100_050.0,
                    1_200_000.0,
                ),
            ],
        ),
        &active_trade_recent(vec![active_signal(
            "buy-3",
            ActiveTradeToxicSignalType::LargeAggressiveBuy,
            ToxicSide::Buy,
        )]),
        &liquidation_recent(vec![liquidation_signal(
            LiquidationToxicSignalType::UpsideLiquidationMagnet,
            LiquidationToxicDirection::Upside,
        )]),
    );

    assert!(!report.signals.is_empty());
    for signal in &report.signals {
        assert!(signal.read_only);
        assert!(signal.toxicity_score <= 100);
        assert!(signal.spoof_score <= 100);
        assert!(signal.absorption_score <= 100);
        assert!(signal.inducement_score <= 100);
        assert!(!signal.reason.is_empty());
    }
}

fn lifecycle_report(
    tracked_walls: Vec<TrackedOrderbookWall>,
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
        status: if tracked_walls.is_empty() && recent_events.is_empty() {
            "insufficient_data".to_string()
        } else {
            "tracking".to_string()
        },
        tracked_walls,
        recent_events,
        toxicity_candidates: Vec::new(),
        warnings: Vec::new(),
        no_trade_reasons: vec!["analysis only".to_string()],
    }
}

fn tracked_wall(
    wall_id: &str,
    side: OrderbookWallSide,
    price: f64,
    notional: f64,
    persistence_ms: u64,
    touches: usize,
) -> TrackedOrderbookWall {
    TrackedOrderbookWall {
        wall_id: wall_id.to_string(),
        symbol: "BTC-PERP".to_string(),
        side,
        price,
        notional,
        quantity: 12.0,
        distance_bps: 14.5,
        first_seen_ms: 1_000,
        last_seen_ms: 1_000 + persistence_ms,
        updates: 2,
        touches,
        status: "tracked".to_string(),
    }
}

fn event(
    wall_id: &str,
    event_type: OrderbookWallEventType,
    side: OrderbookWallSide,
    observed_at_ms: u64,
    price: f64,
    notional: f64,
) -> OrderbookWallLifecycleEvent {
    OrderbookWallLifecycleEvent {
        event_id: format!("{wall_id}-{observed_at_ms}"),
        wall_id: wall_id.to_string(),
        symbol: "BTC-PERP".to_string(),
        event_type,
        side,
        price,
        notional,
        distance_bps: 14.5,
        observed_at_ms,
        reason: "test event".to_string(),
    }
}

fn empty_active_trade_recent() -> ActiveTradeToxicityRecentResponse {
    active_trade_recent(Vec::new())
}

fn active_trade_recent(signals: Vec<ActiveTradeToxicSignal>) -> ActiveTradeToxicityRecentResponse {
    ActiveTradeToxicityRecentResponse {
        read_only: true,
        runtime_modified: false,
        mode: "analysis_only".to_string(),
        selected_symbol: "BTC-PERP".to_string(),
        status: "neutral".to_string(),
        score: 0.0,
        side_bias: "neutral".to_string(),
        warnings: Vec::new(),
        no_trade_reasons: vec!["analysis only".to_string()],
        signals,
    }
}

fn active_signal(
    signal_id: &str,
    signal_type: ActiveTradeToxicSignalType,
    side: ToxicSide,
) -> ActiveTradeToxicSignal {
    ActiveTradeToxicSignal {
        signal_id: signal_id.to_string(),
        symbol: "BTC-PERP".to_string(),
        ts_ms: 8_000,
        signal_type,
        side,
        timeframe: None,
        candle_open_ms: None,
        candle_close_ms: None,
        window_ms: 5_000,
        delta: None,
        abs_delta: None,
        threshold: None,
        aggressive_volume: 14.0,
        notional_usd: 1_500_000.0,
        trade_count: 12,
        cvd_delta: if side == ToxicSide::Buy {
            800_000.0
        } else {
            -800_000.0
        },
        buy_volume: if side == ToxicSide::Buy { 15.0 } else { 2.0 },
        sell_volume: if side == ToxicSide::Sell { 15.0 } else { 2.0 },
        imbalance_ratio: 0.78,
        open: None,
        high: None,
        low: None,
        close: None,
        price_impact_bps: Some(if side == ToxicSide::Buy { 0.4 } else { -0.4 }),
        price_change_bps: Some(if side == ToxicSide::Buy { 0.2 } else { -0.2 }),
        upper_wick_ratio: None,
        lower_wick_ratio: None,
        markout_5s: Some(if side == ToxicSide::Buy { 0.1 } else { -0.1 }),
        markout_15s: Some(0.0),
        markout_60s: None,
        toxicity_score: 76,
        confidence: ToxicConfidence::High,
        reason: vec!["test active signal".to_string()],
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

fn liquidation_signal(
    signal_type: LiquidationToxicSignalType,
    direction: LiquidationToxicDirection,
) -> LiquidationToxicSignal {
    LiquidationToxicSignal {
        signal_id: "liq-1".to_string(),
        symbol: "BTC-PERP".to_string(),
        ts_ms: 8_000,
        signal_type,
        direction,
        current_price: 100_000.0,
        cluster_price: match direction {
            LiquidationToxicDirection::Upside => 100_250.0,
            LiquidationToxicDirection::Downside => 99_750.0,
            LiquidationToxicDirection::Neutral => 100_000.0,
        },
        distance_usd: 250.0,
        distance_bps: 25.0,
        estimated_liquidation_notional: 2_500_000.0,
        cluster_density_score: 74,
        magnet_score: 71,
        cascade_score: 44,
        linked_active_trade_signal_ids: Vec::new(),
        toxicity_score: 72,
        confidence: ToxicConfidence::Medium,
        reason: vec!["test liquidation signal".to_string()],
        read_only: true,
    }
}
