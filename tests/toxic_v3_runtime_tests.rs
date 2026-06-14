use btc_toxic_flow_monitor_rs::toxic_v3::{
    Direction, ExecutionRouter, FlowInferenceEngine, HazardEngine, IntentEngine,
    MarketFlowExchange, MarketFlowTick, SignalAggregator, SignalSource, SignalType, StealthEngine,
};
use tokio::sync::mpsc;

#[test]
fn aggregator_marks_high_quality_flow_without_enabling_external_dispatch() {
    let (router, _rx) = router();
    let engine = FlowInferenceEngine::new(
        SignalSource::ContractWhale,
        92.0,
        aggressive_decision(),
        router,
    );
    let signal = engine.evaluate(&strong_flow());

    assert_eq!(signal.source, SignalSource::ContractWhale);
    assert_eq!(signal.direction, Direction::Buy);
    assert!(
        signal.risk_score >= aggressive_decision().alert_threshold,
        "{signal:?}"
    );
    assert!(signal.should_alert, "{signal:?}");
    assert!(!signal.external_dispatch_enabled);
    assert!(matches!(
        signal.signal_type,
        SignalType::StealthEntry
            | SignalType::WhaleAccumulation
            | SignalType::MarketManipulationRisk
            | SignalType::TofAnomaly
    ));
}

#[test]
fn aggregator_accepts_precomputed_engine_outputs() {
    let flow = strong_flow();
    let direction = Direction::Buy;
    let stealth = StealthEngine::analyze(&flow);
    let hazard = HazardEngine::compute(&flow, &stealth);
    let lambda = HazardEngine::compute_lambda(&flow, &stealth);
    let intent = IntentEngine::infer(&flow, direction, &stealth, &hazard);
    let decision = aggressive_decision();

    let signal = SignalAggregator::evaluate(
        &flow,
        SignalSource::FlowInference,
        90.0,
        direction,
        &stealth,
        &hazard,
        &intent,
        &decision,
    );

    assert_eq!(signal.source, SignalSource::FlowInference);
    assert_eq!(signal.hazard_lambda, lambda);
    assert_eq!(signal.stealth_score, stealth.stealth_score);
    assert_eq!(signal.confidence >= 0.0, true);
}

#[tokio::test]
async fn realtime_loop_dispatches_signal_events_over_channel() {
    let (flow_tx, flow_rx) = mpsc::channel(8);
    let (signal_tx, mut signal_rx) = mpsc::channel(8);
    let router = ExecutionRouter::new(signal_tx);
    let engine = FlowInferenceEngine::new(
        SignalSource::BinanceAltContract,
        88.0,
        aggressive_decision(),
        router,
    );

    let handle = tokio::spawn(engine.run(flow_rx));
    flow_tx.send(strong_flow()).await.unwrap();
    drop(flow_tx);

    let signal = signal_rx.recv().await.expect("signal should be dispatched");
    assert_eq!(signal.symbol, "BTCUSDT");
    assert!(signal.enrichment.read_only);
    assert!(signal.enrichment.analysis_only);
    assert!(!signal.enrichment.direct_discord_gate);

    handle.await.unwrap().unwrap();
}

#[test]
fn low_quality_flow_is_enriched_but_not_alertable() {
    let (router, _rx) = router();
    let engine =
        FlowInferenceEngine::new(SignalSource::TofLite, 42.0, aggressive_decision(), router);
    let signal = engine.evaluate(&strong_flow());

    assert!(!signal.should_alert, "{signal:?}");
    assert!(signal
        .enrichment
        .explanation_tags
        .iter()
        .any(|tag| tag == "data_quality_low"));
}

fn router() -> (
    ExecutionRouter,
    mpsc::Receiver<btc_toxic_flow_monitor_rs::toxic_v3::SignalEvent>,
) {
    let (tx, rx) = mpsc::channel(8);
    (ExecutionRouter::new(tx), rx)
}

fn aggressive_decision() -> btc_toxic_flow_monitor_rs::toxic_v3::DecisionEngine {
    btc_toxic_flow_monitor_rs::toxic_v3::DecisionEngine {
        alert_threshold: 55.0,
        stealth_weight: 0.30,
        hazard_weight: 0.40,
        intent_weight: 0.30,
        min_hazard_lambda: 0.45,
        min_stealth_score: 35.0,
        min_confidence: 60.0,
        min_data_quality: 70.0,
        external_dispatch_enabled: false,
    }
}

fn strong_flow() -> MarketFlowTick {
    MarketFlowTick {
        ts: 1_700_000_040_000,
        exchange: MarketFlowExchange::Binance,
        symbol: "BTCUSDT".to_string(),
        buy_volume: 12_000.0,
        sell_volume: 2_000.0,
        net_flow: 10_000.0,
        flow_acceleration: 5_000.0,
        trade_count: 240,
        avg_trade_size: 18.0,
        large_trade_ratio: 0.72,
        realized_vol: 0.42,
        open_interest_delta: 2_500.0,
        funding_rate: 0.0004,
        liquidation_pressure: 0.12,
        price_move_pct: 0.31,
        dynamic_multiple: 8.5,
        anomaly_persistence_sec: 540.0,
        cross_exchange_dispersion: 0.12,
    }
}
