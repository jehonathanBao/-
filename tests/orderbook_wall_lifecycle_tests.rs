use btc_toxic_flow_monitor_rs::{
    toxicity::orderbook_wall_lifecycle::{
        build_orderbook_wall_lifecycle_report, OrderbookWallLifecycleEngine,
    },
    types::{
        liquidation::{
            LiquidationToxicDirection, LiquidationToxicSignal, LiquidationToxicSignalType,
            LiquidationToxicityRecentResponse,
        },
        market::{NormalizedBook, Venue},
        orderbook_wall::{OrderbookWallCandidateType, OrderbookWallEventType},
        toxic_flow::{
            ActiveTradeToxicSignal, ActiveTradeToxicSignalType, ActiveTradeToxicityRecentResponse,
            ToxicConfidence, ToxicSide,
        },
    },
};

#[test]
fn empty_engine_state_is_insufficient_data() {
    let engine = OrderbookWallLifecycleEngine::new("BTC-PERP");
    let state = engine.get_state();
    assert_eq!(state.status, "insufficient_data");
    assert!(state.tracked_walls.is_empty());
    assert!(state.recent_events.is_empty());
}

#[test]
fn large_bid_wall_creates_support_wall_appeared() {
    let mut engine = OrderbookWallLifecycleEngine::new("BTC-PERP");
    engine.on_book(&book_snapshot(
        1_000,
        100_000.0,
        vec![(99_950.0, 5.0), (99_900.0, 0.8)],
        vec![(100_050.0, 0.6), (100_100.0, 0.5)],
    ));
    let state = engine.get_state();
    assert!(state
        .recent_events
        .iter()
        .any(|event| event.event_type == OrderbookWallEventType::SupportWallAppeared));
}

#[test]
fn large_ask_wall_creates_resistance_wall_appeared() {
    let mut engine = OrderbookWallLifecycleEngine::new("BTC-PERP");
    engine.on_book(&book_snapshot(
        1_000,
        100_000.0,
        vec![(99_950.0, 0.6), (99_900.0, 0.5)],
        vec![(100_050.0, 5.0), (100_100.0, 0.7)],
    ));
    let state = engine.get_state();
    assert!(state
        .recent_events
        .iter()
        .any(|event| event.event_type == OrderbookWallEventType::ResistanceWallAppeared));
}

#[test]
fn wall_size_change_creates_wall_updated() {
    let mut engine = OrderbookWallLifecycleEngine::new("BTC-PERP");
    engine.on_book(&book_snapshot(
        1_000,
        100_000.0,
        vec![(99_950.0, 4.0)],
        vec![(100_050.0, 0.6)],
    ));
    engine.on_book(&book_snapshot(
        2_000,
        100_000.0,
        vec![(99_950.0, 4.6)],
        vec![(100_050.0, 0.6)],
    ));
    let state = engine.get_state();
    assert!(state
        .recent_events
        .iter()
        .any(|event| event.event_type == OrderbookWallEventType::WallUpdated));
}

#[test]
fn wall_relocation_creates_wall_moved() {
    let mut engine = OrderbookWallLifecycleEngine::new("BTC-PERP");
    engine.on_book(&book_snapshot(
        1_000,
        100_000.0,
        vec![(99_950.0, 4.0)],
        vec![(100_050.0, 0.6)],
    ));
    engine.on_book(&book_snapshot(
        2_000,
        100_000.0,
        vec![(99_970.0, 4.0)],
        vec![(100_050.0, 0.6)],
    ));
    let state = engine.get_state();
    assert!(state.recent_events.iter().any(|event| {
        matches!(
            event.event_type,
            OrderbookWallEventType::WallMovedUp | OrderbookWallEventType::WallMovedDown
        )
    }));
}

#[test]
fn short_lived_wall_disappearance_creates_fake_wall_candidate() {
    let mut engine = OrderbookWallLifecycleEngine::new("BTC-PERP");
    engine.on_book(&book_snapshot(
        1_000,
        100_000.0,
        vec![(99_950.0, 4.0)],
        vec![(100_050.0, 0.6)],
    ));
    engine.on_book(&book_snapshot(
        4_000,
        100_000.0,
        vec![(99_700.0, 0.4)],
        vec![(100_150.0, 0.5)],
    ));
    let state = engine.get_state();
    let report = build_orderbook_wall_lifecycle_report(
        &state,
        &empty_active_trade_recent(),
        &empty_liquidation_recent(),
    );
    assert!(state
        .recent_events
        .iter()
        .any(|event| event.event_type == OrderbookWallEventType::FakeWallCandidate));
    assert!(report.toxicity_candidates.iter().any(|candidate| {
        candidate.candidate_type == OrderbookWallCandidateType::FakeSupportWall
    }));
}

#[test]
fn wall_consumption_after_touch_creates_absorption_candidate() {
    let mut engine = OrderbookWallLifecycleEngine::new("BTC-PERP");
    engine.on_book(&book_snapshot(
        1_000,
        100_000.0,
        vec![(99_950.0, 0.6)],
        vec![(100_020.0, 4.0)],
    ));
    engine.on_book(&book_snapshot(
        2_000,
        100_018.0,
        vec![(99_950.0, 0.6)],
        vec![(100_020.0, 3.0)],
    ));
    engine.on_book(&book_snapshot(
        3_000,
        100_060.0,
        vec![(99_950.0, 0.6)],
        vec![(100_300.0, 0.5)],
    ));
    let state = engine.get_state();
    assert!(state
        .recent_events
        .iter()
        .any(|event| event.event_type == OrderbookWallEventType::WallConsumed));
    assert!(state
        .recent_events
        .iter()
        .any(|event| event.event_type == OrderbookWallEventType::AbsorptionCandidate));
}

#[test]
fn confluence_with_active_trade_and_liquidation_remains_analysis_only() {
    let mut engine = OrderbookWallLifecycleEngine::new("BTC-PERP");
    engine.on_book(&book_snapshot(
        1_000,
        100_000.0,
        vec![(99_950.0, 0.8)],
        vec![(100_020.0, 4.0)],
    ));
    engine.on_book(&book_snapshot(
        4_000,
        100_060.0,
        vec![(99_950.0, 0.8)],
        vec![(100_300.0, 0.5)],
    ));
    let state = engine.get_state();
    let report = build_orderbook_wall_lifecycle_report(
        &state,
        &active_trade_recent(vec![active_signal(
            "delta-buy-1",
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
    assert!(report.toxicity_candidates.iter().any(|candidate| {
        candidate.candidate_type == OrderbookWallCandidateType::WallDeltaConfluence
    }));
    assert!(report.toxicity_candidates.iter().any(|candidate| {
        candidate.candidate_type == OrderbookWallCandidateType::WallLiquidationConfluence
    }));
}

fn book_snapshot(
    ts: i64,
    mid: f64,
    bids: Vec<(f64, f64)>,
    asks: Vec<(f64, f64)>,
) -> NormalizedBook {
    let best_bid = bids.first().map(|(price, _)| *price).unwrap_or(mid - 1.0);
    let best_ask = asks.first().map(|(price, _)| *price).unwrap_or(mid + 1.0);
    NormalizedBook {
        venue: Venue::Binance,
        symbol: "BTC-PERP".to_string(),
        ts,
        best_bid,
        best_ask,
        bids,
        asks,
        mid,
        spread_bps: 1.0,
        bid_depth_btc_10bps: 0.0,
        ask_depth_btc_10bps: 0.0,
        bid_depth_usd_10bps: 0.0,
        ask_depth_usd_10bps: 0.0,
        imbalance_10bps: 0.0,
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
        ts_ms: 4_000,
        signal_type,
        side,
        timeframe: Some("1h".to_string()),
        candle_open_ms: Some(0),
        candle_close_ms: Some(3_600_000),
        window_ms: 3_600_000,
        delta: Some(2_100.0),
        abs_delta: Some(2_100.0),
        threshold: Some(2_000.0),
        aggressive_volume: 12.0,
        notional_usd: 1_000_000.0,
        trade_count: 12,
        cvd_delta: 2_100.0,
        buy_volume: 900_000.0,
        sell_volume: 100_000.0,
        imbalance_ratio: 0.8,
        open: Some(100_000.0),
        high: Some(100_100.0),
        low: Some(99_900.0),
        close: Some(100_050.0),
        price_impact_bps: Some(4.0),
        price_change_bps: Some(5.0),
        upper_wick_ratio: Some(0.1),
        lower_wick_ratio: Some(0.1),
        markout_5s: None,
        markout_15s: None,
        markout_60s: None,
        toxicity_score: 80,
        confidence: ToxicConfidence::Medium,
        reason: vec!["analysis only confluence".to_string()],
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
        ts_ms: 4_000,
        signal_type,
        direction,
        current_price: 100_000.0,
        cluster_price: 100_250.0,
        distance_usd: 250.0,
        distance_bps: 25.0,
        estimated_liquidation_notional: 2_500_000.0,
        cluster_density_score: 70,
        magnet_score: 75,
        cascade_score: 55,
        linked_active_trade_signal_ids: vec!["delta-buy-1".to_string()],
        toxicity_score: 74,
        confidence: ToxicConfidence::Medium,
        reason: vec!["analysis only confluence".to_string()],
        read_only: true,
    }
}
