use btc_toxic_flow_monitor_rs::toxic_v3::{
    AdaptiveController, AdaptiveEngine, AdaptiveParameters, DecisionEngine, InMemorySignalStore,
    MarketFlowExchange, MarketFlowTick, SignalAggregator, SignalSource, SignalStore,
    SystemEvaluationState, SystemEvaluationVerdict,
};

#[test]
fn high_false_positive_rate_proposes_stricter_shadow_thresholds_only() {
    let current = AdaptiveParameters::default();
    let evaluation = evaluation_state(0.82, 0.35, 0.02, 0.80, 0.78, 0.76, 0.80);

    let adjustment = AdaptiveEngine::step(&evaluation, &current);

    assert!(
        adjustment.proposed_parameters.global_alert_threshold > current.global_alert_threshold,
        "{adjustment:?}"
    );
    assert!(
        adjustment.proposed_parameters.glce_threshold > current.glce_threshold,
        "{adjustment:?}"
    );
    assert!(adjustment.read_only);
    assert!(!adjustment.applied);
    assert!(!adjustment.external_dispatch_enabled);
    assert!(adjustment
        .safety_notes
        .iter()
        .any(|note| note == "does_not_mutate_discord_gate"));
}

#[test]
fn high_false_negative_rate_proposes_more_sensitive_shadow_parameters() {
    let current = AdaptiveParameters::default();
    let evaluation = evaluation_state(0.68, 0.02, 0.32, 0.76, 0.70, 0.72, 0.74);

    let adjustment = AdaptiveEngine::step(&evaluation, &current);

    assert!(
        adjustment.proposed_parameters.global_alert_threshold < current.global_alert_threshold,
        "{adjustment:?}"
    );
    assert!(
        adjustment.proposed_parameters.stealth_sensitivity > current.stealth_sensitivity,
        "{adjustment:?}"
    );
    assert!(
        adjustment.proposed_parameters.hazard_sensitivity > current.hazard_sensitivity,
        "{adjustment:?}"
    );
    assert!(adjustment
        .feedback_signals
        .iter()
        .any(|feedback| feedback.signal_type == "false_negative_reduction"));
}

#[test]
fn unstable_or_conflicting_system_dampens_reactive_layers() {
    let current = AdaptiveParameters::default();
    let evaluation = evaluation_state(0.52, 0.05, 0.04, 0.30, 0.42, 0.48, 0.36);

    let adjustment = AdaptiveEngine::step(&evaluation, &current);

    assert!(
        adjustment.proposed_parameters.stealth_sensitivity < current.stealth_sensitivity,
        "{adjustment:?}"
    );
    assert!(
        adjustment.proposed_parameters.hazard_sensitivity < current.hazard_sensitivity,
        "{adjustment:?}"
    );
    assert!(
        adjustment.proposed_parameters.gex_weight < current.gex_weight,
        "{adjustment:?}"
    );
    assert!(adjustment
        .feedback_signals
        .iter()
        .any(|feedback| feedback.signal_type == "regime_stability_damping"));
    assert!(adjustment
        .feedback_signals
        .iter()
        .any(|feedback| feedback.signal_type == "cross_layer_alignment"));
}

#[test]
fn adaptive_controller_updates_shadow_parameters_without_applying_runtime_gate() {
    let mut controller = AdaptiveController::default();
    let before = controller.shadow_parameters().clone();
    let evaluation = evaluation_state(0.80, 0.24, 0.01, 0.72, 0.74, 0.76, 0.78);

    let adjustment = controller.step(&evaluation);

    assert_eq!(controller.adjustments().len(), 1);
    assert_ne!(controller.shadow_parameters(), &before);
    assert_eq!(
        controller.shadow_parameters(),
        &adjustment.proposed_parameters
    );
    assert!(adjustment.read_only);
    assert!(!adjustment.applied);
}

#[test]
fn signal_store_can_propose_adaptation_from_recent_system_evaluation() {
    let decision = DecisionEngine::default();
    let signal = SignalAggregator::evaluate_tick(
        &force_flow("BTCUSDT"),
        SignalSource::FlowInference,
        92.0,
        &decision,
    );
    let mut store = InMemorySignalStore::new(4);
    store.record(&signal);

    let adjustment = store.propose_adaptation(&AdaptiveParameters::default());

    assert!(adjustment.read_only);
    assert!(!adjustment.applied);
    assert!(!adjustment.feedback_signals.is_empty() || adjustment.confidence_delta.is_finite());
    assert!(!signal.external_dispatch_enabled);
}

fn evaluation_state(
    confidence: f64,
    fp: f64,
    fn_rate: f64,
    stability: f64,
    cascade: f64,
    squeeze: f64,
    consistency: f64,
) -> SystemEvaluationState {
    SystemEvaluationState {
        prediction_accuracy: confidence,
        false_positive_rate: fp,
        false_negative_rate: fn_rate,
        regime_stability_score: stability,
        cascade_prediction_score: cascade,
        squeeze_prediction_score: squeeze,
        structural_consistency_score: consistency,
        system_confidence: confidence,
        evaluated_sample_count: 20,
        labeled_event_count: 20,
        verdict: SystemEvaluationVerdict::NeedsCalibration,
        reliability_factors: Vec::new(),
        risk_factors: Vec::new(),
    }
}

fn force_flow(symbol: &str) -> MarketFlowTick {
    MarketFlowTick {
        ts: 1_700_000_110_000,
        exchange: MarketFlowExchange::Binance,
        symbol: symbol.to_string(),
        buy_volume: 12_600.0,
        sell_volume: 1_600.0,
        net_flow: 11_000.0,
        flow_acceleration: 6_800.0,
        trade_count: 420,
        avg_trade_size: 30.0,
        large_trade_ratio: 0.84,
        realized_vol: 0.90,
        open_interest_delta: 12_100.0,
        funding_rate: 0.0015,
        liquidation_pressure: 0.90,
        price_move_pct: 0.88,
        dynamic_multiple: 10.0,
        anomaly_persistence_sec: 520.0,
        cross_exchange_dispersion: 0.22,
    }
}
