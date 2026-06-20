use std::collections::BTreeMap;

use btc_toxic_flow_monitor_rs::calibration::strategy_evolution::{
    StrategyEvolutionConfig, StrategyEvolutionEngine, StrategyEvolutionState,
    StrategyPerformanceRecord,
};

#[test]
fn evolution_recommends_weight_threshold_and_module_changes_without_runtime_mutation() {
    let current = StrategyEvolutionState {
        strategy_weights: BTreeMap::from([
            ("liquidation_engine".to_string(), 0.30),
            ("flow_engine".to_string(), 0.25),
            ("manipulation_engine".to_string(), 0.25),
            ("weak_fake_breakout".to_string(), 0.20),
        ]),
        thresholds: BTreeMap::from([
            ("liquidation_engine".to_string(), 0.70),
            ("flow_engine".to_string(), 0.65),
            ("manipulation_engine".to_string(), 0.62),
            ("weak_fake_breakout".to_string(), 0.58),
        ]),
        active_modules: vec![
            "liquidation_engine".to_string(),
            "flow_engine".to_string(),
            "manipulation_engine".to_string(),
            "weak_fake_breakout".to_string(),
        ],
    };
    let metrics = vec![
        perf("liquidation_engine", 0.82, 1.55, 0.88, 0.84, 0.08, 0.06, 42),
        perf("flow_engine", 0.71, 1.18, 0.76, 0.74, 0.16, 0.11, 38),
        perf(
            "manipulation_engine",
            0.52,
            0.82,
            0.62,
            0.58,
            0.41,
            0.24,
            76,
        ),
        perf("weak_fake_breakout", 0.39, 0.61, 0.44, 0.40, 0.48, 0.36, 28),
    ];

    let report =
        StrategyEvolutionEngine::evolve(&current, &metrics, &StrategyEvolutionConfig::default());

    assert!(report.read_only);
    assert!(!report.runtime_modified);
    assert!(!report.config_modified);
    assert!(report
        .safety_notes
        .iter()
        .any(|note| note == "recommendations_only_no_runtime_mutation"));

    assert!(
        report.strategy_weights["liquidation_engine"]
            > current.strategy_weights["liquidation_engine"]
    );
    assert!(
        report.strategy_weights["manipulation_engine"]
            < current.strategy_weights["manipulation_engine"]
    );
    assert!(report
        .disabled_modules
        .contains(&"weak_fake_breakout".to_string()));
    assert!(report
        .active_modules
        .contains(&"liquidation_engine".to_string()));
    assert!(!report
        .active_modules
        .contains(&"weak_fake_breakout".to_string()));
    assert!(report
        .reinforced_modules
        .contains(&"liquidation_engine".to_string()));

    let manipulation_threshold = report
        .threshold_updates
        .iter()
        .find(|update| update.module == "manipulation_engine")
        .expect("manipulation threshold update");
    assert!(
        manipulation_threshold.recommended_threshold > manipulation_threshold.current_threshold
    );
    assert_eq!(manipulation_threshold.reason, "false_positive_rate_high");

    assert!(report.system_accuracy > 0.0);
    assert_eq!(report.best_module.as_deref(), Some("liquidation_engine"));
    assert_eq!(report.worst_module.as_deref(), Some("weak_fake_breakout"));
}

#[test]
fn evolution_handles_empty_metrics_as_no_op_shadow_report() {
    let current = StrategyEvolutionState {
        strategy_weights: BTreeMap::from([("flow_engine".to_string(), 1.0)]),
        thresholds: BTreeMap::from([("flow_engine".to_string(), 0.65)]),
        active_modules: vec!["flow_engine".to_string()],
    };

    let report =
        StrategyEvolutionEngine::evolve(&current, &[], &StrategyEvolutionConfig::default());

    assert_eq!(report.strategy_weights, current.strategy_weights);
    assert_eq!(report.active_modules, current.active_modules);
    assert!(report.disabled_modules.is_empty());
    assert!(report.threshold_updates.is_empty());
    assert_eq!(report.system_accuracy, 0.0);
    assert!(report.read_only);
    assert!(!report.runtime_modified);
    assert!(!report.config_modified);
}

fn perf(
    module: &str,
    accuracy: f64,
    profit_factor: f64,
    stability: f64,
    consistency: f64,
    false_positive_rate: f64,
    drawdown: f64,
    signal_count: usize,
) -> StrategyPerformanceRecord {
    StrategyPerformanceRecord {
        module: module.to_string(),
        accuracy,
        profit_factor,
        stability,
        consistency,
        false_positive_rate,
        drawdown,
        signal_count,
    }
}
