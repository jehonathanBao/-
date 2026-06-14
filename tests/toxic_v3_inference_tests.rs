use btc_toxic_flow_monitor_rs::toxic_v3::{
    enrich_signal, Direction, HazardStateKind, IntentType, MarketFlowExchange, MarketFlowTick,
    SignalSource, StealthRegime, ToxicV3SignalInput, TrajectoryStateKind,
};

#[test]
fn fragmented_low_impact_flow_is_marked_as_stealth_camouflage() {
    let flow = MarketFlowTick {
        ts: 1_700_000_000_000,
        exchange: MarketFlowExchange::Binance,
        symbol: "SOLUSDT".to_string(),
        buy_volume: 520.0,
        sell_volume: 480.0,
        net_flow: 40.0,
        flow_acceleration: 10.0,
        trade_count: 260,
        avg_trade_size: 2.0,
        large_trade_ratio: 0.05,
        realized_vol: 0.02,
        open_interest_delta: 180.0,
        funding_rate: 0.0002,
        liquidation_pressure: 0.02,
        price_move_pct: 0.01,
        dynamic_multiple: 2.4,
        anomaly_persistence_sec: 420.0,
        cross_exchange_dispersion: 0.20,
    };
    let enrichment = enrich_signal(&input(flow, Direction::Buy));

    assert!(enrichment.stealth_score >= 55.0, "{enrichment:?}");
    assert!(matches!(
        enrichment.stealth_regime,
        StealthRegime::ActiveCamouflage | StealthRegime::ExtremeStealth
    ));
    assert!(enrichment
        .explanation_tags
        .iter()
        .any(|tag| tag == "stealth_camouflage"));
    assert!(enrichment.read_only);
    assert!(enrichment.analysis_only);
    assert!(!enrichment.direct_discord_gate);
}

#[test]
fn persistent_stealth_and_dynamic_flow_raise_hazard_without_changing_gate() {
    let flow = MarketFlowTick {
        ts: 1_700_000_010_000,
        exchange: MarketFlowExchange::Binance,
        symbol: "XRPUSDT".to_string(),
        buy_volume: 8_000.0,
        sell_volume: 2_000.0,
        net_flow: 6_000.0,
        flow_acceleration: 4_000.0,
        trade_count: 220,
        avg_trade_size: 18.0,
        large_trade_ratio: 0.62,
        realized_vol: 0.38,
        open_interest_delta: 1_500.0,
        funding_rate: 0.0005,
        liquidation_pressure: 0.18,
        price_move_pct: 0.22,
        dynamic_multiple: 8.0,
        anomaly_persistence_sec: 520.0,
        cross_exchange_dispersion: 0.15,
    };
    let enrichment = enrich_signal(&input(flow, Direction::Buy));

    assert!(
        matches!(
            enrichment.hazard_state,
            HazardStateKind::Elevated | HazardStateKind::Critical
        ),
        "{enrichment:?}"
    );
    assert!(
        matches!(
            enrichment.trajectory_state,
            TrajectoryStateKind::Building | TrajectoryStateKind::Persistent
        ),
        "{enrichment:?}"
    );
    assert!(!enrichment.direct_discord_gate);
}

#[test]
fn liquidation_with_oi_drop_is_classified_as_exit_or_stop_hunt() {
    let flow = MarketFlowTick {
        ts: 1_700_000_020_000,
        exchange: MarketFlowExchange::Binance,
        symbol: "DOGEUSDT".to_string(),
        buy_volume: 500.0,
        sell_volume: 5_500.0,
        net_flow: -5_000.0,
        flow_acceleration: 3_000.0,
        trade_count: 80,
        avg_trade_size: 75.0,
        large_trade_ratio: 0.80,
        realized_vol: 0.55,
        open_interest_delta: -2_000.0,
        funding_rate: 0.001,
        liquidation_pressure: 0.55,
        price_move_pct: -0.80,
        dynamic_multiple: 7.0,
        anomaly_persistence_sec: 120.0,
        cross_exchange_dispersion: 0.05,
    };
    let enrichment = enrich_signal(&input(flow, Direction::Sell));

    assert!(
        matches!(
            enrichment.intent,
            IntentType::PanicExit | IntentType::StopHunt
        ),
        "{enrichment:?}"
    );
    assert_ne!(enrichment.intent, IntentType::StealthBuildUp);
    assert!(enrichment
        .explanation_tags
        .iter()
        .any(|tag| tag == "intent_panic_exit" || tag == "intent_stop_hunt"));
}

#[test]
fn price_reversal_trajectory_is_not_treated_as_trend_confirmation() {
    let flow = MarketFlowTick {
        ts: 1_700_000_030_000,
        exchange: MarketFlowExchange::Bitfinex,
        symbol: "ETHUSDT".to_string(),
        buy_volume: 9_000.0,
        sell_volume: 1_000.0,
        net_flow: 8_000.0,
        flow_acceleration: 1_200.0,
        trade_count: 90,
        avg_trade_size: 100.0,
        large_trade_ratio: 0.42,
        realized_vol: 0.28,
        open_interest_delta: 400.0,
        funding_rate: 0.0001,
        liquidation_pressure: 0.05,
        price_move_pct: -0.18,
        dynamic_multiple: 4.5,
        anomaly_persistence_sec: 180.0,
        cross_exchange_dispersion: 0.10,
    };
    let enrichment = enrich_signal(&input(flow, Direction::Buy));

    assert_eq!(enrichment.trajectory_state, TrajectoryStateKind::Reversal);
    assert!(enrichment
        .explanation_tags
        .iter()
        .any(|tag| tag == "trajectory_reversal"));
}

fn input(flow: MarketFlowTick, direction: Direction) -> ToxicV3SignalInput {
    ToxicV3SignalInput {
        source: SignalSource::BinanceAltContract,
        direction,
        risk_score: 82.0,
        data_quality: 88.0,
        flow,
    }
}
