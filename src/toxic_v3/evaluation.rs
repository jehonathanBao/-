//! System-level reliability evaluation for the read-only toxic_v3 stack.
//!
//! This module is an audit/calibration layer. It does not change signal
//! generation, alert decisions, Discord gates, or any execution path.

use std::collections::HashMap;

use serde::{Deserialize, Serialize};

use super::{
    gex::DealerBias,
    glce::BreakoutBias,
    lhcs::CascadeDirection,
    mff::MarketRegime,
    signal::{SignalEvent, SignalType},
    types::{clamp01, Direction},
};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "snake_case")]
pub enum SystemEvaluationVerdict {
    Reliable,
    NeedsCalibration,
    Unreliable,
    #[default]
    Unknown,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SystemEvaluationState {
    pub prediction_accuracy: f64,
    pub false_positive_rate: f64,
    pub false_negative_rate: f64,
    pub regime_stability_score: f64,
    pub cascade_prediction_score: f64,
    pub squeeze_prediction_score: f64,
    pub structural_consistency_score: f64,
    pub system_confidence: f64,
    pub evaluated_sample_count: usize,
    pub labeled_event_count: usize,
    pub verdict: SystemEvaluationVerdict,
    pub reliability_factors: Vec<String>,
    pub risk_factors: Vec<String>,
}

impl Default for SystemEvaluationState {
    fn default() -> Self {
        Self {
            prediction_accuracy: 0.0,
            false_positive_rate: 0.0,
            false_negative_rate: 0.0,
            regime_stability_score: 0.0,
            cascade_prediction_score: 0.0,
            squeeze_prediction_score: 0.0,
            structural_consistency_score: 0.0,
            system_confidence: 0.0,
            evaluated_sample_count: 0,
            labeled_event_count: 0,
            verdict: SystemEvaluationVerdict::Unknown,
            reliability_factors: Vec::new(),
            risk_factors: Vec::new(),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct SystemHistory {
    pub samples: Vec<SystemEvaluationSample>,
}

impl SystemHistory {
    pub fn from_signals(signals: &[SignalEvent]) -> Self {
        Self {
            samples: signals
                .iter()
                .map(SystemEvaluationSample::from_signal)
                .collect(),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SystemEvaluationSample {
    pub ts: i64,
    pub symbol: String,
    pub direction: Direction,
    pub signal_type: SignalType,
    pub predicted_liquidation: bool,
    pub predicted_squeeze: bool,
    pub predicted_breakout: bool,
    pub observed_liquidation: Option<bool>,
    pub observed_squeeze: Option<bool>,
    pub observed_breakout: Option<bool>,
    pub regime: MarketRegime,
    pub glce_bias: BreakoutBias,
    pub lhcs_direction: CascadeDirection,
    pub dealer_bias: DealerBias,
    pub mff_direction: Direction,
    pub mff_stress: f64,
    pub glce_squeeze_probability: f64,
    pub lhcs_cascade_probability: f64,
    pub gex_squeeze_probability: f64,
}

impl SystemEvaluationSample {
    pub fn from_signal(signal: &SignalEvent) -> Self {
        let predicted_liquidation = signal.signal_type == SignalType::LiquidationCascade
            || signal.glce_state.liquidation_risk >= 0.60
            || signal.lhcs_state.cascade_state.cascade_probability >= 0.72;
        let predicted_squeeze = signal.glce_state.squeeze_probability >= 0.60
            || signal.gex_state.squeeze_probability >= 0.60;
        let predicted_breakout = signal.market_force_field.total_stress >= 0.55
            && signal.market_force_field.directional_bias != Direction::Neutral;

        Self {
            ts: signal.ts,
            symbol: signal.symbol.clone(),
            direction: signal.direction,
            signal_type: signal.signal_type,
            predicted_liquidation,
            predicted_squeeze,
            predicted_breakout,
            observed_liquidation: None,
            observed_squeeze: None,
            observed_breakout: None,
            regime: signal.market_force_field.regime_state,
            glce_bias: signal.glce_state.breakout_bias,
            lhcs_direction: signal.lhcs_state.cascade_state.direction_bias,
            dealer_bias: signal.gex_state.dealer_position_bias,
            mff_direction: signal.market_force_field.directional_bias,
            mff_stress: signal.market_force_field.total_stress,
            glce_squeeze_probability: signal.glce_state.squeeze_probability,
            lhcs_cascade_probability: signal.lhcs_state.cascade_state.cascade_probability,
            gex_squeeze_probability: signal.gex_state.squeeze_probability,
        }
    }

    pub fn with_observed_liquidation(mut self, observed: bool) -> Self {
        self.observed_liquidation = Some(observed);
        self
    }

    pub fn with_observed_squeeze(mut self, observed: bool) -> Self {
        self.observed_squeeze = Some(observed);
        self
    }

    pub fn with_observed_breakout(mut self, observed: bool) -> Self {
        self.observed_breakout = Some(observed);
        self
    }
}

pub struct EvaluationEngine;

impl EvaluationEngine {
    pub fn evaluate(history: &SystemHistory) -> SystemEvaluationState {
        if history.samples.is_empty() {
            return SystemEvaluationState::default();
        }

        let event_stats = event_accuracy(history);
        let regime_stability_score = regime_stability(&history.samples);
        let structural_consistency_score = structural_consistency(history);
        let cascade_prediction_score =
            cascade_prediction_score(history, structural_consistency_score);
        let squeeze_prediction_score =
            squeeze_prediction_score(history, structural_consistency_score);

        let prediction_accuracy = event_stats.accuracy.unwrap_or_else(|| {
            clamp01(structural_consistency_score * 0.70 + regime_stability_score * 0.30)
        });
        let false_positive_rate = event_stats.false_positive_rate.unwrap_or(0.0);
        let false_negative_rate = event_stats.false_negative_rate.unwrap_or(0.0);
        let system_confidence = clamp01(
            prediction_accuracy * 0.35
                + regime_stability_score * 0.20
                + cascade_prediction_score * 0.18
                + squeeze_prediction_score * 0.17
                + structural_consistency_score * 0.10,
        );
        let verdict = verdict(
            system_confidence,
            event_stats.false_positive_rate,
            event_stats.false_negative_rate,
        );
        let reliability_factors = reliability_factors(
            prediction_accuracy,
            regime_stability_score,
            structural_consistency_score,
            cascade_prediction_score,
            squeeze_prediction_score,
            event_stats.labeled_event_count,
        );
        let risk_factors = risk_factors(
            false_positive_rate,
            false_negative_rate,
            regime_stability_score,
            structural_consistency_score,
            event_stats.labeled_event_count,
        );

        SystemEvaluationState {
            prediction_accuracy,
            false_positive_rate,
            false_negative_rate,
            regime_stability_score,
            cascade_prediction_score,
            squeeze_prediction_score,
            structural_consistency_score,
            system_confidence,
            evaluated_sample_count: history.samples.len(),
            labeled_event_count: event_stats.labeled_event_count,
            verdict,
            reliability_factors,
            risk_factors,
        }
    }
}

#[derive(Debug, Clone, Copy, Default)]
struct EventStats {
    accuracy: Option<f64>,
    false_positive_rate: Option<f64>,
    false_negative_rate: Option<f64>,
    labeled_event_count: usize,
    cascade_accuracy: Option<f64>,
    squeeze_accuracy: Option<f64>,
}

fn event_accuracy(history: &SystemHistory) -> EventStats {
    let mut total = 0usize;
    let mut correct = 0usize;
    let mut fp = 0usize;
    let mut fn_ = 0usize;
    let mut cascade_total = 0usize;
    let mut cascade_correct = 0usize;
    let mut squeeze_total = 0usize;
    let mut squeeze_correct = 0usize;
    let mut breakout_total = 0usize;
    let mut breakout_correct = 0usize;

    for sample in &history.samples {
        update_label(
            sample.predicted_liquidation,
            sample.observed_liquidation,
            &mut total,
            &mut correct,
            &mut fp,
            &mut fn_,
            &mut cascade_total,
            &mut cascade_correct,
        );
        update_label(
            sample.predicted_squeeze,
            sample.observed_squeeze,
            &mut total,
            &mut correct,
            &mut fp,
            &mut fn_,
            &mut squeeze_total,
            &mut squeeze_correct,
        );
        update_label(
            sample.predicted_breakout,
            sample.observed_breakout,
            &mut total,
            &mut correct,
            &mut fp,
            &mut fn_,
            &mut breakout_total,
            &mut breakout_correct,
        );
    }

    if total == 0 {
        return EventStats::default();
    }

    EventStats {
        accuracy: Some(correct as f64 / total as f64),
        false_positive_rate: Some(fp as f64 / total as f64),
        false_negative_rate: Some(fn_ as f64 / total as f64),
        labeled_event_count: total,
        cascade_accuracy: non_empty_accuracy(cascade_correct, cascade_total),
        squeeze_accuracy: non_empty_accuracy(squeeze_correct, squeeze_total),
    }
}

#[allow(clippy::too_many_arguments)]
fn update_label(
    predicted: bool,
    observed: Option<bool>,
    total: &mut usize,
    correct: &mut usize,
    fp: &mut usize,
    fn_: &mut usize,
    kind_total: &mut usize,
    kind_correct: &mut usize,
) {
    let Some(observed) = observed else {
        return;
    };

    *total += 1;
    *kind_total += 1;
    if predicted == observed {
        *correct += 1;
        *kind_correct += 1;
    } else if predicted && !observed {
        *fp += 1;
    } else if !predicted && observed {
        *fn_ += 1;
    }
}

fn non_empty_accuracy(correct: usize, total: usize) -> Option<f64> {
    if total == 0 {
        None
    } else {
        Some(correct as f64 / total as f64)
    }
}

fn regime_stability(samples: &[SystemEvaluationSample]) -> f64 {
    if samples.len() <= 1 {
        return 1.0;
    }

    let transitions = samples
        .windows(2)
        .filter(|window| window[0].regime != window[1].regime)
        .count();
    let transition_ratio = transitions as f64 / (samples.len() - 1) as f64;
    let entropy = normalized_regime_entropy(samples);

    clamp01(1.0 - transition_ratio * 0.55 - entropy * 0.45)
}

fn normalized_regime_entropy(samples: &[SystemEvaluationSample]) -> f64 {
    let mut counts: HashMap<MarketRegime, usize> = HashMap::new();
    for sample in samples {
        *counts.entry(sample.regime).or_insert(0) += 1;
    }

    let total = samples.len() as f64;
    let entropy = counts.values().fold(0.0, |acc, count| {
        let p = *count as f64 / total;
        if p <= f64::EPSILON {
            acc
        } else {
            acc - p * p.log2()
        }
    });
    let max_entropy = (counts.len().max(2) as f64).log2();

    clamp01(entropy / max_entropy)
}

fn structural_consistency(history: &SystemHistory) -> f64 {
    let total = history
        .samples
        .iter()
        .map(sample_structural_consistency)
        .sum::<f64>();
    clamp01(total / history.samples.len() as f64)
}

fn sample_structural_consistency(sample: &SystemEvaluationSample) -> f64 {
    let mut votes = Vec::new();
    push_direction_vote(&mut votes, sample.direction);
    push_direction_vote(&mut votes, direction_from_glce(sample.glce_bias));
    push_direction_vote(&mut votes, direction_from_lhcs(sample.lhcs_direction));
    push_direction_vote(
        &mut votes,
        direction_from_gex(sample.dealer_bias, sample.direction),
    );
    push_direction_vote(&mut votes, sample.mff_direction);

    if votes.len() <= 1 {
        return 0.50;
    }

    let directional_alignment = if matches!(sample.direction, Direction::Buy | Direction::Sell) {
        let aligned_with_signal = votes
            .iter()
            .filter(|direction| **direction == sample.direction)
            .count();
        aligned_with_signal as f64 / votes.len() as f64
    } else {
        let buy_votes = votes
            .iter()
            .filter(|direction| **direction == Direction::Buy)
            .count();
        let sell_votes = votes
            .iter()
            .filter(|direction| **direction == Direction::Sell)
            .count();
        buy_votes.max(sell_votes) as f64 / votes.len() as f64
    };
    let stress_alignment = if sample.mff_stress >= 0.55
        && (sample.glce_squeeze_probability >= 0.50 || sample.lhcs_cascade_probability >= 0.50)
    {
        1.0
    } else if sample.mff_stress < 0.35
        && sample.glce_squeeze_probability < 0.55
        && sample.lhcs_cascade_probability < 0.55
    {
        0.85
    } else {
        0.55
    };
    let gex_alignment = if sample.gex_squeeze_probability >= 0.60 {
        if sample.glce_squeeze_probability >= 0.45 || sample.lhcs_cascade_probability >= 0.45 {
            1.0
        } else {
            0.45
        }
    } else {
        0.70
    };

    clamp01(directional_alignment * 0.55 + stress_alignment * 0.30 + gex_alignment * 0.15)
}

fn push_direction_vote(votes: &mut Vec<Direction>, direction: Direction) {
    if matches!(direction, Direction::Buy | Direction::Sell) {
        votes.push(direction);
    }
}

fn direction_from_glce(bias: BreakoutBias) -> Direction {
    match bias {
        BreakoutBias::LongSqueeze => Direction::Buy,
        BreakoutBias::ShortSqueeze => Direction::Sell,
        BreakoutBias::Neutral => Direction::Neutral,
    }
}

fn direction_from_lhcs(direction: CascadeDirection) -> Direction {
    match direction {
        CascadeDirection::UpwardSqueeze => Direction::Buy,
        CascadeDirection::DownwardSqueeze => Direction::Sell,
        CascadeDirection::Neutral => Direction::Neutral,
    }
}

fn direction_from_gex(bias: DealerBias, fallback: Direction) -> Direction {
    match bias {
        DealerBias::SellRallies => Direction::Sell,
        DealerBias::BuyDips => Direction::Buy,
        DealerBias::Neutral => fallback,
    }
}

fn cascade_prediction_score(history: &SystemHistory, structural_consistency: f64) -> f64 {
    let event_stats = event_accuracy(history);
    event_stats.cascade_accuracy.unwrap_or_else(|| {
        let average_cascade = history
            .samples
            .iter()
            .map(|sample| sample.lhcs_cascade_probability)
            .sum::<f64>()
            / history.samples.len() as f64;
        clamp01(structural_consistency * 0.70 + average_cascade * 0.30)
    })
}

fn squeeze_prediction_score(history: &SystemHistory, structural_consistency: f64) -> f64 {
    let event_stats = event_accuracy(history);
    event_stats.squeeze_accuracy.unwrap_or_else(|| {
        let average_squeeze = history
            .samples
            .iter()
            .map(|sample| {
                sample
                    .glce_squeeze_probability
                    .max(sample.gex_squeeze_probability)
            })
            .sum::<f64>()
            / history.samples.len() as f64;
        clamp01(structural_consistency * 0.70 + average_squeeze * 0.30)
    })
}

fn verdict(
    system_confidence: f64,
    false_positive_rate: Option<f64>,
    false_negative_rate: Option<f64>,
) -> SystemEvaluationVerdict {
    if system_confidence >= 0.78
        && false_positive_rate.unwrap_or(0.0) <= 0.20
        && false_negative_rate.unwrap_or(0.0) <= 0.20
    {
        SystemEvaluationVerdict::Reliable
    } else if system_confidence >= 0.50 {
        SystemEvaluationVerdict::NeedsCalibration
    } else {
        SystemEvaluationVerdict::Unreliable
    }
}

fn reliability_factors(
    prediction_accuracy: f64,
    regime_stability: f64,
    structural_consistency: f64,
    cascade_score: f64,
    squeeze_score: f64,
    labeled_event_count: usize,
) -> Vec<String> {
    let mut factors = Vec::new();
    if prediction_accuracy >= 0.75 {
        factors.push("event_accuracy_high".to_string());
    }
    if regime_stability >= 0.70 {
        factors.push("regime_stable".to_string());
    }
    if structural_consistency >= 0.70 {
        factors.push("cross_layer_consistent".to_string());
    }
    if cascade_score >= 0.70 {
        factors.push("cascade_prediction_supported".to_string());
    }
    if squeeze_score >= 0.70 {
        factors.push("squeeze_prediction_supported".to_string());
    }
    if labeled_event_count == 0 {
        factors.push("unlabeled_history_structural_fallback".to_string());
    }
    factors
}

fn risk_factors(
    false_positive_rate: f64,
    false_negative_rate: f64,
    regime_stability: f64,
    structural_consistency: f64,
    labeled_event_count: usize,
) -> Vec<String> {
    let mut factors = Vec::new();
    if false_positive_rate >= 0.25 {
        factors.push("false_positive_rate_high".to_string());
    }
    if false_negative_rate >= 0.25 {
        factors.push("false_negative_rate_high".to_string());
    }
    if regime_stability < 0.45 {
        factors.push("regime_churn_high".to_string());
    }
    if structural_consistency < 0.50 {
        factors.push("cross_layer_conflict".to_string());
    }
    if labeled_event_count == 0 {
        factors.push("no_observed_outcome_labels".to_string());
    }
    factors
}
