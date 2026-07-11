use std::collections::VecDeque;

use btc_toxic_flow_monitor_rs::binance_alt_contract_monitor::smll::audit_self_learning_loop_with_mode;

#[test]
fn disabled_self_learning_never_reports_accuracy_or_calibration() {
    let report =
        audit_self_learning_loop_with_mode("disabled", 1_700_000_000_000, &VecDeque::new());

    assert!(!report.enabled);
    assert_eq!(report.learning_mode, "disabled");
    assert!(!report.accuracy_available);
    assert_eq!(report.reason, "future_outcome_evaluator_not_enabled");
    assert_eq!(report.accuracy_rate, 0.0);
    assert!(report.outcome_records.is_empty());
    assert!(report.calibration_updates.is_empty());
}

#[test]
fn heuristic_audit_never_labels_consistency_as_real_accuracy() {
    let report =
        audit_self_learning_loop_with_mode("heuristic_audit", 1_700_000_000_000, &VecDeque::new());

    assert!(report.enabled);
    assert_eq!(report.learning_mode, "heuristic_audit");
    assert!(!report.accuracy_available);
    assert_eq!(report.accuracy_rate, 0.0);
    assert_eq!(report.reason, "heuristic_consistency_only");
    assert!(report.outcome_records.is_empty());
    assert!(report.calibration_updates.is_empty());
}
