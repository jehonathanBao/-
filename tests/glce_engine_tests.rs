use btc_toxic_flow_monitor_rs::toxic_v3::{
    BreakoutBias, DecisionEngine, ExecutionRouter, FlowInferenceEngine, GLCEEngine,
    MarketFlowExchange, MarketFlowTick, SignalSource, StealthEngine,
};
use tokio::sync::mpsc;

#[test]
fn glce_detects_long_squeeze_confluence_from_oi_funding_and_liquidations() {
    let flow = confluence_flow("BTCUSDT", 9_000.0);
    let stealth = StealthEngine::analyze(&flow);
    let glce = GLCEEngine::compute(&flow, &stealth);

    assert_eq!(glce.symbol, "BTCUSDT");
    assert!(glce.gamma_pressure >= 0.45, "{glce:?}");
    assert!(glce.liquidation_risk >= 0.55, "{glce:?}");
    assert!(glce.squeeze_probability >= 0.60, "{glce:?}");
    assert_eq!(glce.breakout_bias, BreakoutBias::LongSqueeze);
    assert!(glce.liquidity_bands.iter().all(|level| level.price >= 1.0));
}

#[test]
fn glce_detects_short_squeeze_bias_for_negative_net_flow() {
    let flow = confluence_flow("ETHUSDT", -8_500.0);
    let stealth = StealthEngine::analyze(&flow);
    let glce = GLCEEngine::compute(&flow, &stealth);

    assert_eq!(glce.breakout_bias, BreakoutBias::ShortSqueeze);
    assert!(glce.liquidity_bands.iter().all(|level| level.price <= 1.0));
}

#[test]
fn glce_keeps_quiet_flow_neutral() {
    let flow = MarketFlowTick {
        ts: 1_700_000_070_000,
        exchange: MarketFlowExchange::Binance,
        symbol: "BTCUSDT".to_string(),
        buy_volume: 1_020.0,
        sell_volume: 1_000.0,
        net_flow: 20.0,
        flow_acceleration: 5.0,
        trade_count: 22,
        avg_trade_size: 4.0,
        large_trade_ratio: 0.02,
        realized_vol: 0.01,
        open_interest_delta: 15.0,
        funding_rate: 0.00001,
        liquidation_pressure: 0.01,
        price_move_pct: 0.01,
        dynamic_multiple: 1.1,
        anomaly_persistence_sec: 10.0,
        cross_exchange_dispersion: 0.01,
    };
    let stealth = StealthEngine::analyze(&flow);
    let glce = GLCEEngine::compute(&flow, &stealth);

    assert!(glce.squeeze_probability < 0.30, "{glce:?}");
    assert_eq!(glce.breakout_bias, BreakoutBias::Neutral);
}

#[test]
fn signal_aggregator_carries_glce_state_without_enabling_external_dispatch() {
    let (tx, _rx) = mpsc::channel(8);
    let engine = FlowInferenceEngine::new(
        SignalSource::FlowInference,
        95.0,
        DecisionEngine::default(),
        ExecutionRouter::new(tx),
    );
    let signal = engine.evaluate(&confluence_flow("BTCUSDT", 9_000.0));

    assert_eq!(signal.glce_state.symbol, "BTCUSDT");
    assert!(signal.glce_state.squeeze_probability > 0.0);
    assert_eq!(
        signal.enrichment.glce_breakout_bias,
        "long_squeeze".to_string()
    );
    assert!(signal.enrichment.read_only);
    assert!(signal.enrichment.analysis_only);
    assert!(!signal.external_dispatch_enabled);
}

fn confluence_flow(symbol: &str, net_flow: f64) -> MarketFlowTick {
    let buy_volume = if net_flow >= 0.0 {
        net_flow.abs() + 1_000.0
    } else {
        1_000.0
    };
    let sell_volume = if net_flow < 0.0 {
        net_flow.abs() + 1_000.0
    } else {
        1_000.0
    };
    MarketFlowTick {
        ts: 1_700_000_060_000,
        exchange: MarketFlowExchange::Binance,
        symbol: symbol.to_string(),
        buy_volume,
        sell_volume,
        net_flow,
        flow_acceleration: net_flow.abs() * 0.55,
        trade_count: 280,
        avg_trade_size: 24.0,
        large_trade_ratio: 0.78,
        realized_vol: 0.85,
        open_interest_delta: net_flow.abs() * 0.9,
        funding_rate: 0.0012,
        liquidation_pressure: 0.82,
        price_move_pct: if net_flow >= 0.0 { 0.72 } else { -0.72 },
        dynamic_multiple: 9.5,
        anomaly_persistence_sec: 420.0,
        cross_exchange_dispersion: 0.18,
    }
}
