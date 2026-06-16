use btc_toxic_flow_monitor_rs::toxic_v3::{
    CascadeDirection, DecisionEngine, ExecutionRouter, FlowInferenceEngine, GLCEEngine, LHCSEngine,
    MarketFlowExchange, MarketFlowTick, SignalSource, StealthEngine,
};
use tokio::sync::mpsc;

#[test]
fn lhcs_builds_heatmap_and_upward_cascade_from_glce_pressure() {
    let flow = cascade_flow("BTCUSDT", 10_500.0);
    let stealth = StealthEngine::analyze(&flow);
    let glce = GLCEEngine::compute(&flow, &stealth);
    let lhcs = LHCSEngine::compute(&flow, &glce);

    assert_eq!(lhcs.symbol, "BTCUSDT");
    assert_eq!(lhcs.liquidation_heatmap.price_bins.len(), 11);
    assert_eq!(lhcs.liquidation_heatmap.density_map.len(), 11);
    assert!(lhcs.cascade_state.cascade_probability >= 0.55, "{lhcs:?}");
    assert_eq!(
        lhcs.cascade_state.direction_bias,
        CascadeDirection::UpwardSqueeze
    );
    assert!(!lhcs.cascade_state.propagation_chain.is_empty());
    assert!(!lhcs.trigger_levels.is_empty());
}

#[test]
fn lhcs_detects_downward_cascade_bias_for_negative_flow() {
    let flow = cascade_flow("ETHUSDT", -9_800.0);
    let stealth = StealthEngine::analyze(&flow);
    let glce = GLCEEngine::compute(&flow, &stealth);
    let lhcs = LHCSEngine::compute(&flow, &glce);

    assert_eq!(
        lhcs.cascade_state.direction_bias,
        CascadeDirection::DownwardSqueeze
    );
    assert!(lhcs.trigger_levels.iter().all(|level| level.price <= 1.0));
}

#[test]
fn lhcs_keeps_quiet_flow_low_probability_and_neutral() {
    let flow = MarketFlowTick {
        ts: 1_700_000_080_000,
        exchange: MarketFlowExchange::Binance,
        symbol: "BTCUSDT".to_string(),
        buy_volume: 1_005.0,
        sell_volume: 1_000.0,
        net_flow: 5.0,
        flow_acceleration: 1.0,
        trade_count: 18,
        avg_trade_size: 3.0,
        large_trade_ratio: 0.01,
        realized_vol: 0.01,
        open_interest_delta: 5.0,
        funding_rate: 0.00001,
        liquidation_pressure: 0.01,
        price_move_pct: 0.0,
        dynamic_multiple: 1.0,
        anomaly_persistence_sec: 5.0,
        cross_exchange_dispersion: 0.0,
    };
    let stealth = StealthEngine::analyze(&flow);
    let glce = GLCEEngine::compute(&flow, &stealth);
    let lhcs = LHCSEngine::compute(&flow, &glce);

    assert!(lhcs.cascade_state.cascade_probability < 0.35, "{lhcs:?}");
    assert_eq!(lhcs.cascade_state.direction_bias, CascadeDirection::Neutral);
}

#[test]
fn signal_aggregator_carries_lhcs_state_without_external_dispatch() {
    let (tx, _rx) = mpsc::channel(8);
    let engine = FlowInferenceEngine::new(
        SignalSource::FlowInference,
        95.0,
        DecisionEngine::default(),
        ExecutionRouter::new(tx),
    );
    let signal = engine.evaluate(&cascade_flow("BTCUSDT", 10_500.0));

    assert_eq!(signal.lhcs_state.symbol, "BTCUSDT");
    assert!(signal.lhcs_state.cascade_state.cascade_probability > 0.0);
    assert_eq!(
        signal.enrichment.lhcs_direction_bias,
        "upward_squeeze".to_string()
    );
    assert!(signal.enrichment.read_only);
    assert!(signal.enrichment.analysis_only);
    assert!(!signal.external_dispatch_enabled);
}

fn cascade_flow(symbol: &str, net_flow: f64) -> MarketFlowTick {
    let buy_volume = if net_flow >= 0.0 {
        net_flow.abs() + 1_500.0
    } else {
        1_500.0
    };
    let sell_volume = if net_flow < 0.0 {
        net_flow.abs() + 1_500.0
    } else {
        1_500.0
    };
    MarketFlowTick {
        ts: 1_700_000_090_000,
        exchange: MarketFlowExchange::Binance,
        symbol: symbol.to_string(),
        buy_volume,
        sell_volume,
        net_flow,
        flow_acceleration: net_flow.abs() * 0.60,
        trade_count: 360,
        avg_trade_size: 28.0,
        large_trade_ratio: 0.82,
        realized_vol: 0.92,
        open_interest_delta: net_flow.abs() * 1.05,
        funding_rate: 0.0014,
        liquidation_pressure: 0.88,
        price_move_pct: if net_flow >= 0.0 { 0.82 } else { -0.82 },
        dynamic_multiple: 10.0,
        anomaly_persistence_sec: 480.0,
        cross_exchange_dispersion: 0.20,
    }
}
