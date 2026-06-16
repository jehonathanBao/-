//! Shadow adaptive controller for toxic_v3.
//!
//! The controller turns system evaluation output into parameter proposals. It
//! intentionally does not mutate live alert gates, write configuration files, or
//! dispatch external notifications.

use serde::{Deserialize, Serialize};

use super::{evaluation::SystemEvaluationState, types::clamp01};

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AdaptiveParameters {
    pub stealth_sensitivity: f64,
    pub hazard_sensitivity: f64,
    pub glce_threshold: f64,
    pub gex_weight: f64,
    pub global_alert_threshold: f64,
}

impl Default for AdaptiveParameters {
    fn default() -> Self {
        Self {
            stealth_sensitivity: 1.0,
            hazard_sensitivity: 1.0,
            glce_threshold: 0.60,
            gex_weight: 0.25,
            global_alert_threshold: 80.0,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct FeedbackSignal {
    pub signal_type: String,
    pub predicted: f64,
    pub actual: f64,
    pub error: f64,
    pub parameter: String,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AdaptiveAdjustment {
    pub previous_parameters: AdaptiveParameters,
    pub proposed_parameters: AdaptiveParameters,
    pub feedback_signals: Vec<FeedbackSignal>,
    pub confidence_delta: f64,
    pub read_only: bool,
    pub applied: bool,
    pub external_dispatch_enabled: bool,
    pub safety_notes: Vec<String>,
}

#[derive(Debug, Clone)]
pub struct AdaptiveController {
    shadow_parameters: AdaptiveParameters,
    adjustments: Vec<AdaptiveAdjustment>,
}

impl AdaptiveController {
    pub fn new(parameters: AdaptiveParameters) -> Self {
        Self {
            shadow_parameters: parameters,
            adjustments: Vec::new(),
        }
    }

    pub fn shadow_parameters(&self) -> &AdaptiveParameters {
        &self.shadow_parameters
    }

    pub fn adjustments(&self) -> &[AdaptiveAdjustment] {
        &self.adjustments
    }

    pub fn step(&mut self, evaluation: &SystemEvaluationState) -> AdaptiveAdjustment {
        let adjustment = AdaptiveEngine::step(evaluation, &self.shadow_parameters);
        self.shadow_parameters = adjustment.proposed_parameters.clone();
        self.adjustments.push(adjustment.clone());
        adjustment
    }
}

impl Default for AdaptiveController {
    fn default() -> Self {
        Self::new(AdaptiveParameters::default())
    }
}

pub struct AdaptiveEngine;

impl AdaptiveEngine {
    pub fn step(
        evaluation: &SystemEvaluationState,
        current: &AdaptiveParameters,
    ) -> AdaptiveAdjustment {
        let previous = sanitize_parameters(current);
        let mut proposed = previous.clone();
        let mut feedback_signals = Vec::new();

        reduce_false_positives(evaluation, &previous, &mut proposed, &mut feedback_signals);
        reduce_false_negatives(evaluation, &previous, &mut proposed, &mut feedback_signals);
        improve_regime_stability(evaluation, &previous, &mut proposed, &mut feedback_signals);
        align_multi_layer_consistency(evaluation, &previous, &mut proposed, &mut feedback_signals);
        improve_squeeze_calibration(evaluation, &previous, &mut proposed, &mut feedback_signals);

        proposed = sanitize_parameters(&proposed);
        let confidence_delta = evaluation.system_confidence - 0.70;

        AdaptiveAdjustment {
            previous_parameters: previous,
            proposed_parameters: proposed,
            feedback_signals,
            confidence_delta,
            read_only: true,
            applied: false,
            external_dispatch_enabled: false,
            safety_notes: vec![
                "shadow_parameters_only".to_string(),
                "does_not_mutate_discord_gate".to_string(),
                "does_not_write_runtime_config".to_string(),
            ],
        }
    }
}

fn reduce_false_positives(
    evaluation: &SystemEvaluationState,
    previous: &AdaptiveParameters,
    proposed: &mut AdaptiveParameters,
    feedback: &mut Vec<FeedbackSignal>,
) {
    if evaluation.false_positive_rate < 0.12 {
        return;
    }

    let error = evaluation.false_positive_rate;
    proposed.global_alert_threshold += error * 4.0;
    proposed.glce_threshold += error * 0.035;
    proposed.hazard_sensitivity -= error * 0.035;
    feedback.push(FeedbackSignal {
        signal_type: "false_positive_reduction".to_string(),
        predicted: evaluation.false_positive_rate,
        actual: 0.0,
        error,
        parameter: "global_alert_threshold/glce_threshold/hazard_sensitivity".to_string(),
    });

    if previous.global_alert_threshold < proposed.global_alert_threshold {
        feedback.push(FeedbackSignal {
            signal_type: "strictness_increased".to_string(),
            predicted: proposed.global_alert_threshold,
            actual: previous.global_alert_threshold,
            error: proposed.global_alert_threshold - previous.global_alert_threshold,
            parameter: "global_alert_threshold".to_string(),
        });
    }
}

fn reduce_false_negatives(
    evaluation: &SystemEvaluationState,
    previous: &AdaptiveParameters,
    proposed: &mut AdaptiveParameters,
    feedback: &mut Vec<FeedbackSignal>,
) {
    if evaluation.false_negative_rate < 0.12 {
        return;
    }

    let error = evaluation.false_negative_rate;
    proposed.global_alert_threshold -= error * 3.0;
    proposed.glce_threshold -= error * 0.025;
    proposed.stealth_sensitivity += error * 0.03;
    proposed.hazard_sensitivity += error * 0.03;
    feedback.push(FeedbackSignal {
        signal_type: "false_negative_reduction".to_string(),
        predicted: 1.0 - evaluation.false_negative_rate,
        actual: 1.0,
        error,
        parameter: "global_alert_threshold/glce_threshold/stealth_sensitivity/hazard_sensitivity"
            .to_string(),
    });

    if previous.global_alert_threshold > proposed.global_alert_threshold {
        feedback.push(FeedbackSignal {
            signal_type: "sensitivity_increased".to_string(),
            predicted: proposed.global_alert_threshold,
            actual: previous.global_alert_threshold,
            error: previous.global_alert_threshold - proposed.global_alert_threshold,
            parameter: "global_alert_threshold".to_string(),
        });
    }
}

fn improve_regime_stability(
    evaluation: &SystemEvaluationState,
    _previous: &AdaptiveParameters,
    proposed: &mut AdaptiveParameters,
    feedback: &mut Vec<FeedbackSignal>,
) {
    if evaluation.regime_stability_score >= 0.55 {
        return;
    }

    let error = 0.55 - evaluation.regime_stability_score;
    proposed.stealth_sensitivity -= error * 0.04;
    proposed.hazard_sensitivity -= error * 0.04;
    feedback.push(FeedbackSignal {
        signal_type: "regime_stability_damping".to_string(),
        predicted: evaluation.regime_stability_score,
        actual: 0.55,
        error,
        parameter: "stealth_sensitivity/hazard_sensitivity".to_string(),
    });
}

fn align_multi_layer_consistency(
    evaluation: &SystemEvaluationState,
    _previous: &AdaptiveParameters,
    proposed: &mut AdaptiveParameters,
    feedback: &mut Vec<FeedbackSignal>,
) {
    if evaluation.structural_consistency_score >= 0.60 {
        return;
    }

    let error = 0.60 - evaluation.structural_consistency_score;
    proposed.gex_weight -= error * 0.05;
    proposed.glce_threshold += error * 0.02;
    feedback.push(FeedbackSignal {
        signal_type: "cross_layer_alignment".to_string(),
        predicted: evaluation.structural_consistency_score,
        actual: 0.60,
        error,
        parameter: "gex_weight/glce_threshold".to_string(),
    });
}

fn improve_squeeze_calibration(
    evaluation: &SystemEvaluationState,
    _previous: &AdaptiveParameters,
    proposed: &mut AdaptiveParameters,
    feedback: &mut Vec<FeedbackSignal>,
) {
    if evaluation.squeeze_prediction_score >= 0.55 {
        return;
    }

    let error = 0.55 - evaluation.squeeze_prediction_score;
    proposed.gex_weight -= error * 0.04;
    proposed.glce_threshold += error * 0.02;
    feedback.push(FeedbackSignal {
        signal_type: "squeeze_calibration".to_string(),
        predicted: evaluation.squeeze_prediction_score,
        actual: 0.55,
        error,
        parameter: "gex_weight/glce_threshold".to_string(),
    });
}

fn sanitize_parameters(parameters: &AdaptiveParameters) -> AdaptiveParameters {
    AdaptiveParameters {
        stealth_sensitivity: bounded(parameters.stealth_sensitivity, 0.50, 1.50),
        hazard_sensitivity: bounded(parameters.hazard_sensitivity, 0.50, 1.50),
        glce_threshold: bounded(parameters.glce_threshold, 0.35, 0.85),
        gex_weight: bounded(parameters.gex_weight, 0.05, 0.45),
        global_alert_threshold: bounded(parameters.global_alert_threshold, 60.0, 95.0),
    }
}

fn bounded(value: f64, min: f64, max: f64) -> f64 {
    if !value.is_finite() {
        return min;
    }
    min + clamp01((value - min) / (max - min)) * (max - min)
}
