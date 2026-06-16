use btc_toxic_flow_monitor_rs::toxic_v3::{
    BTCLiquidationEngine, DecisionEngine, ExecutionRouter, FlowInferenceEngine, GEXEngine,
    GLCEEngine, LHCSEngine, MarketFlowExchange, MarketFlowTick, SignalSource, StealthEngine,
};
use tokio::sync::mpsc;

#[test]
fn btc_liquidation_engine_outputs_up_squeeze_state_for_btc_only() {
    let flow = liquidation_flow("BTCUSDT", 9_500.0);
    let stealth = StealthEngine::analyze(&flow);
    let glce = GLCEEngine::compute(&flow, &stealth);
    let lhcs = LHCSEngine::compute(&flow, &glce);
    let gex = GEXEngine::compute_from_tick(&flow, &glce, &lhcs);
    let state = BTCLiquidationEngine::compute(&flow, &glce, &lhcs, &gex).expect("BTC state");

    assert_eq!(state.symbol, "BTC");
    assert!(state.short_liquidation_pressure >= 0.45, "{state:?}");
    assert!(
        state.squeeze_up_probability >= state.squeeze_down_probability,
        "{state:?}"
    );
    assert!(state.cascade_risk >= 0.45, "{state:?}");
    assert!(state.gamma_pressure > 0.0);
    assert!(!state.liquidation_clusters.is_empty());
    assert!(state.read_only);
}

#[test]
fn btc_liquidation_engine_outputs_down_squeeze_state_for_long_liquidation_risk() {
    let flow = liquidation_flow("XBTUSDT", -9_500.0);
    let stealth = StealthEngine::analyze(&flow);
    let glce = GLCEEngine::compute(&flow, &stealth);
    let lhcs = LHCSEngine::compute(&flow, &glce);
    let gex = GEXEngine::compute_from_tick(&flow, &glce, &lhcs);
    let state = BTCLiquidationEngine::compute(&flow, &glce, &lhcs, &gex).expect("BTC state");

    assert!(state.long_liquidation_pressure >= 0.45, "{state:?}");
    assert!(
        state.squeeze_down_probability >= state.squeeze_up_probability,
        "{state:?}"
    );
    assert!(state.net_liquidation_bias <= 0.15, "{state:?}");
}

#[test]
fn btc_liquidation_engine_ignores_non_btc_symbols() {
    let flow = liquidation_flow("ETHUSDT", 9_500.0);
    let stealth = StealthEngine::analyze(&flow);
    let glce = GLCEEngine::compute(&flow, &stealth);
    let lhcs = LHCSEngine::compute(&flow, &glce);
    let gex = GEXEngine::compute_from_tick(&flow, &glce, &lhcs);

    assert!(BTCLiquidationEngine::compute(&flow, &glce, &lhcs, &gex).is_none());
}

#[test]
fn signal_carries_btc_liquidation_state_without_enabling_external_dispatch() {
    let (tx, _rx) = mpsc::channel(8);
    let engine = FlowInferenceEngine::new(
        SignalSource::FlowInference,
        95.0,
        DecisionEngine::default(),
        ExecutionRouter::new(tx),
    );
    let signal = engine.evaluate(&liquidation_flow("BTCUSDT", 9_500.0));

    let state = signal
        .btc_liquidation_state
        .as_ref()
        .expect("BTC liquidation state");
    assert_eq!(state.symbol, "BTC");
    assert!(signal.enrichment.btc_liquidation_active);
    assert!(signal.enrichment.btc_cascade_risk > 0.0);
    assert!(signal
        .enrichment
        .explanation_tags
        .iter()
        .any(|tag| tag == "btc_liquidation_engine_active"));
    assert!(signal.enrichment.read_only);
    assert!(signal.enrichment.analysis_only);
    assert!(!signal.external_dispatch_enabled);
}

#[test]
fn non_btc_signal_does_not_receive_btc_liquidation_state() {
    let (tx, _rx) = mpsc::channel(8);
    let engine = FlowInferenceEngine::new(
        SignalSource::FlowInference,
        95.0,
        DecisionEngine::default(),
        ExecutionRouter::new(tx),
    );
    let signal = engine.evaluate(&liquidation_flow("ETHUSDT", 9_500.0));

    assert!(signal.btc_liquidation_state.is_none());
    assert!(!signal.enrichment.btc_liquidation_active);
    assert_eq!(signal.enrichment.btc_liquidation_cluster_count, 0);
}

fn liquidation_flow(symbol: &str, net_flow: f64) -> MarketFlowTick {
    let buy_volume = if net_flow >= 0.0 {
        net_flow.abs() + 1_400.0
    } else {
        1_400.0
    };
    let sell_volume = if net_flow < 0.0 {
        net_flow.abs() + 1_400.0
    } else {
        1_400.0
    };
    MarketFlowTick {
        ts: 1_700_000_160_000,
        exchange: MarketFlowExchange::Binance,
        symbol: symbol.to_string(),
        buy_volume,
        sell_volume,
        net_flow,
        flow_acceleration: net_flow.abs() * 0.62,
        trade_count: 380,
        avg_trade_size: 32.0,
        large_trade_ratio: 0.84,
        realized_vol: 0.88,
        open_interest_delta: net_flow.abs() * 1.02,
        funding_rate: if net_flow >= 0.0 { -0.0011 } else { 0.0011 },
        liquidation_pressure: 0.90,
        price_move_pct: if net_flow >= 0.0 { 0.80 } else { -0.80 },
        dynamic_multiple: 10.0,
        anomaly_persistence_sec: 500.0,
        cross_exchange_dispersion: 0.18,
    }
}
