use btc_toxic_flow_monitor_rs::calibration::false_positive_analytics::{
    FalsePositiveAnalyticsEngine, OutcomeRecord, PredictionRecord, PredictionScope,
};

#[test]
fn analytics_report_scores_signal_regime_event_and_module_quality() {
    let predictions = vec![
        prediction(
            "p1",
            "BTCUSDT",
            "LIQUIDATION_ENGINE",
            "LONG",
            Some("LIQUIDATION"),
            PredictionScope::Signal,
            0.86,
        ),
        prediction(
            "p2",
            "BTCUSDT",
            "MANIPULATION_ENGINE",
            "SHORT",
            Some("MANIPULATION"),
            PredictionScope::Signal,
            0.78,
        ),
        prediction(
            "p3",
            "ETHUSDT",
            "REGIME_ENGINE",
            "LONG",
            Some("ACCUMULATION"),
            PredictionScope::Regime,
            0.74,
        ),
        prediction(
            "p4",
            "SOLUSDT",
            "EVENT_ENGINE",
            "SHORT",
            None,
            PredictionScope::Event,
            0.69,
        ),
    ];
    let outcomes = vec![
        outcome("p1", "UP", Some("LIQUIDATION"), 1.20),
        outcome("p2", "UP", Some("ACCUMULATION"), 0.80),
        outcome("p3", "UP", Some("ACCUMULATION"), 0.55),
        outcome("p4", "DOWN", None, 0.42),
    ];

    let report = FalsePositiveAnalyticsEngine::analyze(&predictions, &outcomes);

    assert_eq!(report.total_predictions, 4);
    assert_eq!(report.resolved_predictions, 4);
    assert_eq!(report.true_positive_count, 3);
    assert_eq!(report.false_positive_count, 1);
    assert_close(report.signal_accuracy, 0.75);
    assert_close(report.signal_precision, 0.75);
    assert_close(report.false_positive_rate, 0.25);
    assert_close(report.regime_accuracy, 2.0 / 3.0);
    assert_close(report.event_reliability, 1.0);
    assert_eq!(
        report.best_performing_module.as_deref(),
        Some("EVENT_ENGINE")
    );
    assert_eq!(
        report.worst_performing_module.as_deref(),
        Some("MANIPULATION_ENGINE")
    );
    assert!(report
        .module_breakdown
        .iter()
        .any(|module| module.module == "LIQUIDATION_ENGINE" && module.precision == 1.0));
}

#[test]
fn analytics_report_is_safe_for_empty_or_unresolved_inputs() {
    let report = FalsePositiveAnalyticsEngine::analyze(&[], &[]);

    assert_eq!(report.total_predictions, 0);
    assert_eq!(report.resolved_predictions, 0);
    assert_eq!(report.signal_accuracy, 0.0);
    assert_eq!(report.signal_precision, 0.0);
    assert_eq!(report.false_positive_rate, 0.0);
    assert_eq!(report.regime_accuracy, 0.0);
    assert_eq!(report.event_reliability, 0.0);
    assert!(report.best_performing_module.is_none());
    assert!(report.worst_performing_module.is_none());

    let unresolved = vec![prediction(
        "p1",
        "BTCUSDT",
        "LIQUIDATION_ENGINE",
        "LONG",
        Some("LIQUIDATION"),
        PredictionScope::Signal,
        0.86,
    )];
    let report = FalsePositiveAnalyticsEngine::analyze(&unresolved, &[]);

    assert_eq!(report.total_predictions, 1);
    assert_eq!(report.resolved_predictions, 0);
    assert_eq!(report.unresolved_predictions, 1);
    assert_eq!(report.false_positive_rate, 0.0);
}

fn prediction(
    id: &str,
    symbol: &str,
    module: &str,
    direction: &str,
    regime: Option<&str>,
    scope: PredictionScope,
    confidence: f64,
) -> PredictionRecord {
    PredictionRecord {
        id: id.to_string(),
        timestamp: 1_700_000_000_000,
        symbol: symbol.to_string(),
        module: module.to_string(),
        signal_type: format!("{module}_SIGNAL"),
        predicted_direction: direction.to_string(),
        predicted_regime: regime.map(str::to_string),
        confidence,
        scope,
        context: serde_json::json!({
            "oi": 1200.0,
            "flow": 0.72,
            "funding": 0.0004
        }),
    }
}

fn outcome(
    prediction_id: &str,
    direction: &str,
    regime: Option<&str>,
    move_percent: f64,
) -> OutcomeRecord {
    OutcomeRecord {
        prediction_id: prediction_id.to_string(),
        validated_at: 1_700_000_300_000,
        horizon_ms: 300_000,
        price_direction: direction.to_string(),
        move_percent,
        actual_regime: regime.map(str::to_string),
        volatility: 0.33,
    }
}

fn assert_close(actual: f64, expected: f64) {
    assert!(
        (actual - expected).abs() < 0.0001,
        "actual={actual} expected={expected}"
    );
}
