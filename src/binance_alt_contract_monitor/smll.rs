use std::collections::{BTreeMap, VecDeque};

use super::types::{
    AltContractAdaptiveWeightConfig, AltContractCalibrationUpdate, AltContractDriftReport,
    AltContractLearningErrorReport, AltContractSignal, AltContractSignalOutcome,
    AltContractSignalOutcomeRecord, AltContractSmllReport,
};

const OUTCOME_LOOKBACK_MS: i64 = 24 * 60 * 60_000;
const MAX_OUTCOME_RECORDS: usize = 12;
const MIN_SAMPLES_FOR_UPDATE: usize = 3;

pub fn audit_self_learning_loop(
    now_ms: i64,
    signals: &VecDeque<AltContractSignal>,
) -> AltContractSmllReport {
    audit_self_learning_loop_with_mode("disabled", now_ms, signals)
}

pub fn audit_self_learning_loop_with_mode(
    mode: &str,
    now_ms: i64,
    signals: &VecDeque<AltContractSignal>,
) -> AltContractSmllReport {
    audit_self_learning_loop_with_outcomes(mode, now_ms, signals, &BTreeMap::new())
}

pub fn audit_self_learning_loop_with_outcomes(
    mode: &str,
    now_ms: i64,
    signals: &VecDeque<AltContractSignal>,
    outcomes: &BTreeMap<String, AltContractSignalOutcome>,
) -> AltContractSmllReport {
    if mode.eq_ignore_ascii_case("disabled") {
        return AltContractSmllReport {
            enabled: false,
            learning_mode: "disabled".to_string(),
            accuracy_available: false,
            reason: "future_outcome_evaluator_not_enabled".to_string(),
            protected_realtime: true,
            status: "disabled".to_string(),
            ..AltContractSmllReport::default()
        };
    }
    if !mode.eq_ignore_ascii_case("heuristic_audit") {
        if mode.eq_ignore_ascii_case("real_outcome") {
            return audit_real_outcome_loop(now_ms, signals, outcomes);
        }
        return AltContractSmllReport {
            enabled: false,
            learning_mode: "disabled".to_string(),
            accuracy_available: false,
            reason: "self_learning_mode_not_enabled".to_string(),
            protected_realtime: true,
            status: "disabled".to_string(),
            ..AltContractSmllReport::default()
        };
    }

    return heuristic_audit_report(signals);
}

fn audit_real_outcome_loop(
    now_ms: i64,
    signals: &VecDeque<AltContractSignal>,
    outcomes: &BTreeMap<String, AltContractSignalOutcome>,
) -> AltContractSmllReport {
    let mut outcome_records = outcomes
        .values()
        .filter(|outcome| now_ms.saturating_sub(outcome.signal_ts) <= OUTCOME_LOOKBACK_MS)
        .filter(|outcome| outcome.markout_1h_bps.is_some())
        .map(real_outcome_record)
        .collect::<Vec<_>>();
    outcome_records.sort_by_key(|record| std::cmp::Reverse(record.timestamp));
    let sample_size = outcome_records.len();
    let correct_count = outcome_records
        .iter()
        .filter(|record| record.accuracy_label == "correct")
        .count();
    let wrong_count = outcome_records
        .iter()
        .filter(|record| record.accuracy_label == "wrong")
        .count();
    let neutral_count = outcome_records
        .iter()
        .filter(|record| record.accuracy_label == "neutral")
        .count();
    let scored_samples = correct_count + wrong_count;
    let accuracy_rate = if scored_samples == 0 {
        0.0
    } else {
        round2(correct_count as f64 / scored_samples as f64 * 100.0)
    };
    let error_reports = attribute_errors(signals, &outcome_records);
    let suggested_weights = adaptive_weights(&error_reports, accuracy_rate, sample_size);
    let drift_report = detect_drift(signals, accuracy_rate, sample_size);
    let calibration_updates = calibration_updates(
        &error_reports,
        &suggested_weights,
        &drift_report,
        accuracy_rate,
        sample_size,
    );
    let learning_score = learning_score(accuracy_rate, &error_reports, &drift_report, sample_size);
    outcome_records.truncate(MAX_OUTCOME_RECORDS);
    AltContractSmllReport {
        enabled: sample_size > 0,
        learning_mode: "real_outcome".to_string(),
        accuracy_available: sample_size > 0,
        reason: if sample_size > 0 {
            "real_future_outcomes_available".to_string()
        } else {
            "no_completed_future_outcomes".to_string()
        },
        protected_realtime: true,
        status: if sample_size < MIN_SAMPLES_FOR_UPDATE {
            "collecting_outcomes".to_string()
        } else if drift_report.drift_detected {
            "drift_watch".to_string()
        } else if calibration_updates.is_empty() {
            "stable_learning".to_string()
        } else {
            "calibration_suggested".to_string()
        },
        learning_score,
        sample_size,
        min_samples_for_update: MIN_SAMPLES_FOR_UPDATE,
        accuracy_rate,
        wrong_count,
        neutral_count,
        outcome_records,
        error_reports,
        suggested_weights,
        drift_report,
        calibration_updates,
    }
}

fn real_outcome_record(outcome: &AltContractSignalOutcome) -> AltContractSignalOutcomeRecord {
    let markout = outcome.markout_1h_bps.unwrap_or_default();
    AltContractSignalOutcomeRecord {
        signal_id: outcome.signal_id.clone(),
        symbol: outcome.product_id.trim_end_matches("USDT").to_string(),
        timestamp: outcome.signal_ts,
        signal_type: outcome.signal_type.clone(),
        mc_score: outcome.ais_score,
        regime: outcome.regime.clone(),
        prediction: String::new(),
        entry_price: outcome.entry_price,
        outcome_price_5m: None,
        outcome_price_15m: None,
        outcome_price_1h: None,
        realized_direction: if markout > 0.0 {
            "follow_through".to_string()
        } else if markout < 0.0 {
            "reversal".to_string()
        } else {
            "flat".to_string()
        },
        accuracy_label: if outcome.follow_through_1h == Some(true) {
            "correct".to_string()
        } else if outcome.follow_through_1h == Some(false) {
            "wrong".to_string()
        } else {
            "neutral".to_string()
        },
    }
}

fn heuristic_audit_report(signals: &VecDeque<AltContractSignal>) -> AltContractSmllReport {
    let sample_size = signals.len();
    let complete_count = signals
        .iter()
        .filter(|signal| {
            signal.evidence_count >= 2
                && signal.data_quality >= 70
                && signal.assessment.evidence_degraded_reasons.is_empty()
        })
        .count();
    let consistency_score = if sample_size == 0 {
        0.0
    } else {
        round2(complete_count as f64 / sample_size as f64 * 100.0)
    };

    AltContractSmllReport {
        enabled: true,
        learning_mode: "heuristic_audit".to_string(),
        accuracy_available: false,
        reason: "heuristic_consistency_only".to_string(),
        protected_realtime: true,
        status: "heuristic_audit".to_string(),
        learning_score: consistency_score,
        sample_size,
        min_samples_for_update: MIN_SAMPLES_FOR_UPDATE,
        accuracy_rate: 0.0,
        wrong_count: 0,
        neutral_count: 0,
        outcome_records: Vec::new(),
        error_reports: Vec::new(),
        suggested_weights: AltContractAdaptiveWeightConfig {
            volume_weight: 1.0,
            oi_weight: 1.0,
            price_weight: 1.0,
            liquidation_weight: 1.0,
            funding_weight: 1.0,
        },
        drift_report: AltContractDriftReport {
            reason: "heuristic_consistency_only".to_string(),
            ..AltContractDriftReport::default()
        },
        calibration_updates: Vec::new(),
    }
}

fn attribute_errors(
    signals: &VecDeque<AltContractSignal>,
    outcomes: &[AltContractSignalOutcomeRecord],
) -> Vec<AltContractLearningErrorReport> {
    let mut reports = Vec::new();
    let wrong_ids = outcomes
        .iter()
        .filter(|record| record.accuracy_label == "wrong")
        .map(|record| record.signal_id.as_str())
        .collect::<Vec<_>>();
    let wrong_signals = signals
        .iter()
        .filter(|signal| wrong_ids.contains(&signal.id.as_str()))
        .collect::<Vec<_>>();
    let data_errors = wrong_signals
        .iter()
        .filter(|signal| signal.data_quality < 70)
        .count();
    let oi_errors = wrong_signals
        .iter()
        .filter(|signal| signal.oi_change_pct.is_some() && signal.score_breakdown.oi_score > 10.0)
        .count();
    let liquidation_errors = wrong_signals
        .iter()
        .filter(|signal| signal.liquidation_suspected || signal.force_order_snapshot)
        .count();
    let behavior_errors = wrong_signals
        .iter()
        .filter(|signal| {
            signal.smart_money_lifecycle.state_confidence >= 75.0
                && !signal
                    .market_regime
                    .regime
                    .eq_ignore_ascii_case("manipulation")
        })
        .count();
    let prediction_errors = wrong_signals
        .iter()
        .filter(|signal| signal.smart_money_prediction.confidence >= 70.0)
        .count();

    push_error(
        &mut reports,
        data_errors,
        "data_error",
        "data_quality_or_latency",
        "SMAF/data",
    );
    push_error(
        &mut reports,
        oi_errors,
        "signal_error",
        "oi_confirmation_misled_direction",
        "MCSS",
    );
    push_error(
        &mut reports,
        liquidation_errors,
        "signal_error",
        "liquidation_context_misread_as_build",
        "BACM/MCSS",
    );
    push_error(
        &mut reports,
        behavior_errors,
        "behavior_error",
        "lifecycle_or_regime_confidence_overstated",
        "SMLE",
    );
    push_error(
        &mut reports,
        prediction_errors,
        "prediction_error",
        "smp_direction_or_stage_followthrough_failed",
        "SMP",
    );
    reports
}

fn push_error(
    reports: &mut Vec<AltContractLearningErrorReport>,
    count: usize,
    error_type: &str,
    root_cause: &str,
    affected_module: &str,
) {
    if count == 0 {
        return;
    }
    reports.push(AltContractLearningErrorReport {
        error_type: error_type.to_string(),
        severity: if count >= 3 { "high" } else { "medium" }.to_string(),
        root_cause: root_cause.to_string(),
        affected_module: affected_module.to_string(),
    });
}

fn adaptive_weights(
    error_reports: &[AltContractLearningErrorReport],
    accuracy_rate: f64,
    sample_size: usize,
) -> AltContractAdaptiveWeightConfig {
    let mut weights = AltContractAdaptiveWeightConfig {
        volume_weight: 1.0,
        oi_weight: 1.0,
        price_weight: 1.0,
        liquidation_weight: 1.0,
        funding_weight: 1.0,
    };
    if sample_size < MIN_SAMPLES_FOR_UPDATE {
        return weights;
    }
    if has_root(error_reports, "oi_confirmation_misled_direction") {
        weights.oi_weight = 0.85;
    }
    if has_root(error_reports, "liquidation_context_misread_as_build") {
        weights.liquidation_weight = 0.85;
    }
    if accuracy_rate >= 70.0 {
        weights.price_weight = 1.10;
    }
    if accuracy_rate < 60.0 {
        weights.volume_weight = 0.95;
        weights.funding_weight = 0.95;
    }
    weights
}

fn detect_drift(
    signals: &VecDeque<AltContractSignal>,
    accuracy_rate: f64,
    sample_size: usize,
) -> AltContractDriftReport {
    let recent = signals.iter().rev().take(12).collect::<Vec<_>>();
    let mut affected_components = Vec::new();
    let lifecycle_flip_rate = if recent.len() < 2 {
        0.0
    } else {
        let flips = recent
            .windows(2)
            .filter(|pair| {
                pair[0].smart_money_lifecycle.lifecycle_state
                    != pair[1].smart_money_lifecycle.lifecycle_state
            })
            .count();
        flips as f64 / recent.len().saturating_sub(1) as f64 * 100.0
    };
    let prediction_flip_rate = if recent.len() < 2 {
        0.0
    } else {
        let flips = recent
            .windows(2)
            .filter(|pair| {
                pair[0].smart_money_prediction.next_state
                    != pair[1].smart_money_prediction.next_state
            })
            .count();
        flips as f64 / recent.len().saturating_sub(1) as f64 * 100.0
    };
    if sample_size >= MIN_SAMPLES_FOR_UPDATE && accuracy_rate < 60.0 {
        affected_components.push("prediction_accuracy".to_string());
    }
    if lifecycle_flip_rate >= 55.0 {
        affected_components.push("lifecycle_transition".to_string());
    }
    if prediction_flip_rate >= 55.0 {
        affected_components.push("prediction_flip".to_string());
    }
    let drift_detected = !affected_components.is_empty();
    AltContractDriftReport {
        drift_detected,
        suggested_retrain: drift_detected && sample_size >= MIN_SAMPLES_FOR_UPDATE,
        reason: if drift_detected {
            "accuracy_or_state_transition_changed".to_string()
        } else {
            "no_material_drift".to_string()
        },
        affected_components,
    }
}

fn calibration_updates(
    error_reports: &[AltContractLearningErrorReport],
    suggested_weights: &AltContractAdaptiveWeightConfig,
    drift_report: &AltContractDriftReport,
    accuracy_rate: f64,
    sample_size: usize,
) -> Vec<AltContractCalibrationUpdate> {
    if sample_size < MIN_SAMPLES_FOR_UPDATE {
        return Vec::new();
    }
    let mut updates = Vec::new();
    if suggested_weights.oi_weight < 1.0 {
        updates.push(update(
            "mcss.oi_weight",
            1.0,
            suggested_weights.oi_weight,
            "OI 多次误导方向，建议降低 OI 确认权重",
        ));
    }
    if suggested_weights.liquidation_weight < 1.0 {
        updates.push(update(
            "mcss.liquidation_weight",
            1.0,
            suggested_weights.liquidation_weight,
            "清算上下文误判建仓，建议降低清算确认权重",
        ));
    }
    if accuracy_rate < 60.0 {
        updates.push(update(
            "smp.confidence_cap",
            100.0,
            80.0,
            "SMP accuracy 低于 60%，建议收紧预测置信度上限",
        ));
    }
    if drift_report.suggested_retrain {
        updates.push(update(
            "smle.transition_recalibration",
            0.0,
            1.0,
            "检测到结构漂移，建议复盘多窗口状态转移样本",
        ));
    }
    if has_root(error_reports, "data_quality_or_latency") {
        updates.push(update(
            "bacm.min_data_quality",
            70.0,
            75.0,
            "错误信号伴随数据质量偏低，建议提高最小 dataQuality",
        ));
    }
    updates
}

fn update(
    parameter: &str,
    old_value: f64,
    new_value: f64,
    reason: &str,
) -> AltContractCalibrationUpdate {
    AltContractCalibrationUpdate {
        parameter: parameter.to_string(),
        old_value,
        new_value,
        reason: reason.to_string(),
    }
}

fn has_root(error_reports: &[AltContractLearningErrorReport], root_cause: &str) -> bool {
    error_reports
        .iter()
        .any(|report| report.root_cause == root_cause)
}

fn learning_score(
    accuracy_rate: f64,
    error_reports: &[AltContractLearningErrorReport],
    drift_report: &AltContractDriftReport,
    sample_size: usize,
) -> f64 {
    if sample_size == 0 {
        return 0.0;
    }
    let error_penalty = error_reports.len() as f64 * 6.0;
    let drift_penalty = if drift_report.drift_detected {
        12.0
    } else {
        0.0
    };
    round2((accuracy_rate - error_penalty - drift_penalty).clamp(0.0, 100.0))
}

fn round2(value: f64) -> f64 {
    (value * 100.0).round() / 100.0
}
