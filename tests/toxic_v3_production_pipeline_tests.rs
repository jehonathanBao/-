use btc_toxic_flow_monitor_rs::toxic_v3::{
    DecisionEngine, ExecutionRouter, FeatureStore, FlowInferenceEngine, InMemoryFeatureStore,
    InMemorySignalStore, InferenceBus, MarketFlowExchange, MarketFlowTick, ProductionFlowPipeline,
    RecordingProductionFlowPipeline, SignalSource,
};
use tokio::sync::mpsc;

#[test]
fn feature_store_keeps_bounded_rolling_stats_per_symbol() {
    let mut store = InMemoryFeatureStore::new(3);
    for value in [10.0, -5.0, 20.0, 30.0] {
        store.update(&flow("ETHUSDT", value));
    }

    let stats = store.rolling_stats("ETHUSDT");
    assert_eq!(store.sample_count("ETHUSDT"), 3);
    assert_eq!(stats.sample_count, 3);
    assert!(stats.mean_flow > 0.0, "{stats:?}");
    assert!(stats.std_flow > 0.0, "{stats:?}");
    assert!(stats.entropy > 0.0, "{stats:?}");
}

#[tokio::test]
async fn production_pipeline_updates_feature_store_and_dispatches_signal() {
    let (signal_tx, mut signal_rx) = mpsc::channel(8);
    let router = ExecutionRouter::new(signal_tx);
    let engine = FlowInferenceEngine::new(
        SignalSource::FlowInference,
        91.0,
        DecisionEngine::default(),
        router,
    );
    let store = InMemoryFeatureStore::new(10);
    let mut pipeline = ProductionFlowPipeline::new(engine, store);
    let tick = flow("BTCUSDT", 12_000.0);

    let signal = pipeline.process_tick(tick).await.unwrap();
    let dispatched = signal_rx.recv().await.expect("signal should be dispatched");

    assert_eq!(signal.symbol, "BTCUSDT");
    assert_eq!(dispatched.symbol, "BTCUSDT");
    assert!(!signal.external_dispatch_enabled);
    assert!(signal.enrichment.read_only);
}

#[tokio::test]
async fn recording_pipeline_persists_all_processed_signals_in_memory() {
    let (signal_tx, mut signal_rx) = mpsc::channel(8);
    let router = ExecutionRouter::new(signal_tx);
    let engine = FlowInferenceEngine::new(
        SignalSource::FlowInference,
        91.0,
        DecisionEngine::default(),
        router,
    );
    let pipeline = ProductionFlowPipeline::new(engine, InMemoryFeatureStore::new(10));
    let mut recording = RecordingProductionFlowPipeline::new(pipeline, InMemorySignalStore::new(2));

    recording
        .process_tick(flow("BTCUSDT", 12_000.0))
        .await
        .unwrap();
    recording
        .process_tick(flow("BTCUSDT", -8_000.0))
        .await
        .unwrap();
    recording
        .process_tick(flow("BTCUSDT", 4_000.0))
        .await
        .unwrap();

    assert!(signal_rx.recv().await.is_some());
    assert_eq!(recording.recent_signals(10).len(), 2);
    assert_eq!(recording.recent_signals(1)[0].symbol, "BTCUSDT");
}

#[tokio::test]
async fn inference_bus_publishes_ticks_without_external_services() {
    let (tx, mut rx) = mpsc::channel(2);
    let bus = InferenceBus::new(tx);
    bus.publish(flow("SOLUSDT", 250.0)).await.unwrap();

    let tick = rx.recv().await.expect("tick should be queued");
    assert_eq!(tick.symbol, "SOLUSDT");
}

fn flow(symbol: &str, net_flow: f64) -> MarketFlowTick {
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
        ts: 1_700_000_050_000,
        exchange: MarketFlowExchange::Binance,
        symbol: symbol.to_string(),
        buy_volume,
        sell_volume,
        net_flow,
        flow_acceleration: net_flow.abs() / 2.0,
        trade_count: 160,
        avg_trade_size: 16.0,
        large_trade_ratio: 0.58,
        realized_vol: 0.22,
        open_interest_delta: net_flow / 10.0,
        funding_rate: 0.0002,
        liquidation_pressure: 0.04,
        price_move_pct: 0.12,
        dynamic_multiple: 5.0,
        anomaly_persistence_sec: 240.0,
        cross_exchange_dispersion: 0.12,
    }
}
