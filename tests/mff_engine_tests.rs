use btc_toxic_flow_monitor_rs::toxic_v3::{
    DecisionEngine, Direction, ExecutionRouter, FlowInferenceEngine, GEXEngine, GLCEEngine,
    LHCSEngine, MarketFlowExchange, MarketFlowTick, MarketForceFieldEngine, MarketRegime,
    SignalSource, StealthEngine,
};
use tokio::sync::mpsc;

#[test]
fn mff_combines_liquidation_cascade_and_gamma_into_high_stress_field() {
    let flow = force_flow("BTCUSDT", 11_000.0);
    let stealth = StealthEngine::analyze(&flow);
    let glce = GLCEEngine::compute(&flow, &stealth);
    let lhcs = LHCSEngine::compute(&flow, &glce);
    let gex = GEXEngine::compute_from_tick(&flow, &glce, &lhcs);
    let field = MarketForceFieldEngine::compute(&flow, &glce, &lhcs, &gex);

    assert_eq!(field.symbol, "BTCUSDT");
    assert!(field.total_stress >= 0.55, "{field:?}");
    assert!(field.liquidity_field > 0.0);
    assert!(field.gamma_field > 0.0);
    assert!(field.liquidation_field > 0.0);
    assert!(field.cascade_field > 0.0);
    assert!(matches!(
        field.directional_bias,
        Direction::Buy | Direction::Neutral
    ));
    assert!(matches!(
        field.regime_state,
        MarketRegime::FragileAccumulation
            | MarketRegime::Compression
            | MarketRegime::CriticalInstability
    ));
}

#[test]
fn mff_keeps_quiet_flow_stable_and_low_stress() {
    let flow = quiet_flow();
    let stealth = StealthEngine::analyze(&flow);
    let glce = GLCEEngine::compute(&flow, &stealth);
    let lhcs = LHCSEngine::compute(&flow, &glce);
    let gex = GEXEngine::compute_from_tick(&flow, &glce, &lhcs);
    let field = MarketForceFieldEngine::compute(&flow, &glce, &lhcs, &gex);

    assert!(field.total_stress < 0.35, "{field:?}");
    assert_eq!(field.directional_bias, Direction::Neutral);
    assert_eq!(field.regime_state, MarketRegime::Stable);
}

#[test]
fn signal_carries_market_force_field_without_changing_dispatch_gate() {
    let (tx, _rx) = mpsc::channel(8);
    let engine = FlowInferenceEngine::new(
        SignalSource::FlowInference,
        95.0,
        DecisionEngine::default(),
        ExecutionRouter::new(tx),
    );
    let signal = engine.evaluate(&force_flow("BTCUSDT", 11_000.0));

    assert_eq!(signal.market_force_field.symbol, "BTCUSDT");
    assert!(signal.market_force_field.total_stress > 0.0);
    assert!(signal.enrichment.mff_total_stress > 0.0);
    assert!(signal.enrichment.read_only);
    assert!(signal.enrichment.analysis_only);
    assert!(!signal.external_dispatch_enabled);
}

fn force_flow(symbol: &str, net_flow: f64) -> MarketFlowTick {
    let buy_volume = if net_flow >= 0.0 {
        net_flow.abs() + 1_600.0
    } else {
        1_600.0
    };
    let sell_volume = if net_flow < 0.0 {
        net_flow.abs() + 1_600.0
    } else {
        1_600.0
    };
    MarketFlowTick {
        ts: 1_700_000_110_000,
        exchange: MarketFlowExchange::Binance,
        symbol: symbol.to_string(),
        buy_volume,
        sell_volume,
        net_flow,
        flow_acceleration: net_flow.abs() * 0.62,
        trade_count: 420,
        avg_trade_size: 30.0,
        large_trade_ratio: 0.84,
        realized_vol: 0.90,
        open_interest_delta: net_flow.abs() * 1.10,
        funding_rate: 0.0015,
        liquidation_pressure: 0.90,
        price_move_pct: if net_flow >= 0.0 { 0.88 } else { -0.88 },
        dynamic_multiple: 10.0,
        anomaly_persistence_sec: 520.0,
        cross_exchange_dispersion: 0.22,
    }
}

fn quiet_flow() -> MarketFlowTick {
    MarketFlowTick {
        ts: 1_700_000_111_000,
        exchange: MarketFlowExchange::Binance,
        symbol: "BTCUSDT".to_string(),
        buy_volume: 1_010.0,
        sell_volume: 1_000.0,
        net_flow: 10.0,
        flow_acceleration: 2.0,
        trade_count: 20,
        avg_trade_size: 3.0,
        large_trade_ratio: 0.01,
        realized_vol: 0.01,
        open_interest_delta: 3.0,
        funding_rate: 0.00001,
        liquidation_pressure: 0.01,
        price_move_pct: 0.0,
        dynamic_multiple: 1.0,
        anomaly_persistence_sec: 5.0,
        cross_exchange_dispersion: 0.0,
    }
}
