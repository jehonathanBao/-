use std::{cmp::Reverse, collections::VecDeque};

use super::types::{
    AltContractAdaptiveWeightConfig, AltContractCalibrationUpdate, AltContractDirection,
    AltContractDriftReport, AltContractLearningErrorReport, AltContractSignal,
    AltContractSignalOutcomeRecord, AltContractSmllReport,
};

const OUTCOME_LOOKBACK_MS: i64 = 24 * 60 * 60_000;
const MAX_OUTCOME_RECORDS: usize = 12;
const MIN_SAMPLES_FOR_UPDATE: usize = 3;

pub fn audit_self_learning_loop(
    now_ms: i64,
    signals: &VecDeque<AltContractSignal>,
) -> AltContractSmllReport {
    audit_self_learning_loop_with_mode("heuristic_audit", now_ms, signals)
}

pub fn audit_self_learning_loop_with_mode(
    mode: &str,
    now_ms: i64,
    signals: &VecDeque<AltContractSignal>,
) -> AltContractSmllReport {
    if !mode.eq_ignore_ascii_case("heuristic_audit") {
        return AltContractSmllReport {
            enabled: false,
            learning_mode: "disabled".to_string(),
            accuracy_available: false,
            reason: if mode.eq_ignore_ascii_case("disabled") {
                "future_outcome_evaluator_not_enabled".to_string()
            } else {
                "self_learning_mode_not_enabled".to_string()
            },
            protected_realtime: true,
            status: "disabled".to_string(),
            ..AltContractSmllReport::default()
        };
    }
    let mut outcome_records = signals
        .iter()
        .filter(|signal| now_ms.saturating_sub(signal.ts) <= OUTCOME_LOOKBACK_MS)
        .map(signal_outcome_record)
        .collect::<Vec<_>>();
    outcome_records.sort_by_key(|record| Reverse(record.timestamp));

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
        100.0
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
        enabled: true,
        learning_mode: "heuristic_audit".to_string(),
        accuracy_available: false,
        reason: "heuristic_consistency_only".to_string(),
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

fn signal_outcome_record(signal: &AltContractSignal) -> AltContractSignalOutcomeRecord {
    let entry_price = signal.trigger_price_usd;
    let outcome_price = entry_price.zip(signal.price_move_pct).map(|(price, pct)| {
        let ratio = 1.0 + (pct / 100.0);
        round_price(price * ratio)
    });
    AltContractSignalOutcomeRecord {
        signal_id: signal.id.clone(),
        symbol: signal.symbol.clone(),
        timestamp: signal.ts,
        signal_type: format!("{:?}", signal.signal_type),
        mc_score: signal.master_capital_strength.mcss,
        regime: signal.market_regime.regime.clone(),
        prediction: signal.smart_money_prediction.next_state.clone(),
        entry_price,
        outcome_price_5m: outcome_price,
        outcome_price_15m: outcome_price,
        outcome_price_1h: outcome_price,
        realized_direction: realized_direction(signal),
        accuracy_label: accuracy_label(signal),
    }
}

fn accuracy_label(signal: &AltContractSignal) -> String {
    if signal.post_signal_status == "failed" || signal.failed_at.is_some() {
        return "wrong".to_string();
    }
    if signal.post_signal_status == "validated" || signal.validated_at.is_some() {
        return "correct".to_string();
    }
    let Some(price_move_pct) = signal.price_move_pct else {
        return "neutral".to_string();
    };
    if price_move_pct.abs() < 0.03 {
        return "neutral".to_string();
    }
    if direction_matches_price(signal.direction, price_move_pct) {
        "correct".to_string()
    } else {
        "wrong".to_string()
    }
}

fn realized_direction(signal: &AltContractSignal) -> String {
    match signal.price_move_pct {
        Some(value) if value > 0.03 => "up".to_string(),
        Some(value) if value < -0.03 => "down".to_string(),
        Some(_) => "flat".to_string(),
        None => "unknown".to_string(),
    }
}

fn direction_matches_price(direction: AltContractDirection, price_move_pct: f64) -> bool {
    match direction {
        AltContractDirection::Buy => price_move_pct > 0.0,
        AltContractDirection::Sell => price_move_pct < 0.0,
        AltContractDirection::Absorption => price_move_pct >= -0.05,
        AltContractDirection::Suppression => price_move_pct <= 0.05,
        AltContractDirection::Neutral => price_move_pct.abs() <= 0.20,
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

fn round_price(value: f64) -> f64 {
    (value * 100_000_000.0).round() / 100_000_000.0
}
