use btc_toxic_flow_monitor_rs::toxic_v3::{
    DealerBias, DecisionEngine, ExecutionRouter, FlowInferenceEngine, GEXEngine, GLCEEngine,
    LHCSEngine, MarketFlowExchange, MarketFlowTick, OptionStrike, OptionsSurface, SignalSource,
    StealthEngine,
};
use tokio::sync::mpsc;

#[test]
fn gex_computes_dealer_bias_and_gamma_walls_from_options_surface() {
    let surface = OptionsSurface {
        symbol: "BTCUSDT".to_string(),
        underlying_price: 1.0,
        strikes: vec![
            strike(0.98, 800.0, 500.0, 0.42, 0.38),
            strike(1.00, 2_400.0, 900.0, 0.76, 0.52),
            strike(1.02, 1_700.0, 700.0, 0.58, 0.63),
        ],
    };
    let flow = force_flow("BTCUSDT", 8_800.0);
    let stealth = StealthEngine::analyze(&flow);
    let glce = GLCEEngine::compute(&flow, &stealth);
    let lhcs = LHCSEngine::compute(&flow, &glce);

    let state = GEXEngine::compute_from_surface(&surface, &glce, &lhcs);

    assert_eq!(state.symbol, "BTCUSDT");
    assert!(state.total_gex > 0.5, "{state:?}");
    assert_eq!(state.dealer_position_bias, DealerBias::BuyDips);
    assert!(!state.gamma_wall_levels.is_empty());
    assert!(state.max_pain > 0.0);
    assert!(state.squeeze_probability > 0.0);
}

#[test]
fn negative_gamma_surface_marks_sell_rallies_short_gamma_bias() {
    let surface = OptionsSurface {
        symbol: "ETHUSDT".to_string(),
        underlying_price: 1.0,
        strikes: vec![
            strike(0.98, 400.0, 1_400.0, 0.50, 0.45),
            strike(1.00, 700.0, 2_600.0, 0.82, 0.55),
            strike(1.02, 500.0, 1_900.0, 0.64, 0.67),
        ],
    };
    let flow = force_flow("ETHUSDT", -8_400.0);
    let stealth = StealthEngine::analyze(&flow);
    let glce = GLCEEngine::compute(&flow, &stealth);
    let lhcs = LHCSEngine::compute(&flow, &glce);

    let state = GEXEngine::compute_from_surface(&surface, &glce, &lhcs);

    assert!(state.total_gex < -0.5, "{state:?}");
    assert_eq!(state.dealer_position_bias, DealerBias::SellRallies);
    assert!(state.squeeze_probability > 0.0);
}

#[test]
fn gex_proxy_state_is_carried_by_signal_without_external_dispatch() {
    let (tx, _rx) = mpsc::channel(8);
    let engine = FlowInferenceEngine::new(
        SignalSource::FlowInference,
        95.0,
        DecisionEngine::default(),
        ExecutionRouter::new(tx),
    );
    let signal = engine.evaluate(&force_flow("BTCUSDT", 8_800.0));

    assert_eq!(signal.gex_state.symbol, "BTCUSDT");
    assert!(!signal.gex_state.gamma_wall_levels.is_empty());
    assert!(signal.gex_state.squeeze_probability > 0.0);
    assert!(signal.enrichment.gex_gamma_wall_count > 0);
    assert!(signal.enrichment.read_only);
    assert!(signal.enrichment.analysis_only);
    assert!(!signal.external_dispatch_enabled);
}

fn strike(strike: f64, call_oi: f64, put_oi: f64, gamma: f64, delta: f64) -> OptionStrike {
    OptionStrike {
        strike,
        call_oi,
        put_oi,
        gamma,
        delta,
    }
}

fn force_flow(symbol: &str, net_flow: f64) -> MarketFlowTick {
    let buy_volume = if net_flow >= 0.0 {
        net_flow.abs() + 1_200.0
    } else {
        1_200.0
    };
    let sell_volume = if net_flow < 0.0 {
        net_flow.abs() + 1_200.0
    } else {
        1_200.0
    };
    MarketFlowTick {
        ts: 1_700_000_100_000,
        exchange: MarketFlowExchange::Binance,
        symbol: symbol.to_string(),
        buy_volume,
        sell_volume,
        net_flow,
        flow_acceleration: net_flow.abs() * 0.55,
        trade_count: 320,
        avg_trade_size: 25.0,
        large_trade_ratio: 0.80,
        realized_vol: 0.88,
        open_interest_delta: net_flow.abs() * 0.95,
        funding_rate: 0.0011,
        liquidation_pressure: 0.80,
        price_move_pct: if net_flow >= 0.0 { 0.70 } else { -0.70 },
        dynamic_multiple: 9.2,
        anomaly_persistence_sec: 450.0,
        cross_exchange_dispersion: 0.18,
    }
}
