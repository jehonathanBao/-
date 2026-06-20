use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct StrategyPerformanceRecord {
    pub module: String,
    pub accuracy: f64,
    pub profit_factor: f64,
    pub stability: f64,
    pub consistency: f64,
    pub false_positive_rate: f64,
    pub drawdown: f64,
    pub signal_count: usize,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct StrategyEvolutionState {
    pub strategy_weights: BTreeMap<String, f64>,
    pub thresholds: BTreeMap<String, f64>,
    pub active_modules: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct StrategyEvolutionConfig {
    pub baseline_score: f64,
    pub learning_rate: f64,
    pub min_weight: f64,
    pub max_weight: f64,
    pub false_positive_threshold: f64,
    pub high_signal_count_threshold: usize,
    pub threshold_step: f64,
    pub low_accuracy_disable_threshold: f64,
    pub high_drawdown_disable_threshold: f64,
    pub reinforce_accuracy_threshold: f64,
}

impl Default for StrategyEvolutionConfig {
    fn default() -> Self {
        Self {
            baseline_score: 0.55,
            learning_rate: 0.20,
            min_weight: 0.02,
            max_weight: 0.60,
            false_positive_threshold: 0.30,
            high_signal_count_threshold: 60,
            threshold_step: 0.04,
            low_accuracy_disable_threshold: 0.45,
            high_drawdown_disable_threshold: 0.30,
            reinforce_accuracy_threshold: 0.70,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct StrategyScore {
    pub module: String,
    pub score: f64,
    pub accuracy: f64,
    pub profit_factor: f64,
    pub stability: f64,
    pub consistency: f64,
    pub false_positive_rate: f64,
    pub drawdown: f64,
    pub signal_count: usize,
    pub recommendation: StrategyEvolutionAction,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum StrategyEvolutionAction {
    Reinforce,
    Keep,
    Tighten,
    Downgrade,
    Disable,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ThresholdUpdate {
    pub module: String,
    pub current_threshold: f64,
    pub recommended_threshold: f64,
    pub reason: String,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct StrategyEvolutionReport {
    pub strategy_scores: Vec<StrategyScore>,
    pub strategy_weights: BTreeMap<String, f64>,
    pub threshold_updates: Vec<ThresholdUpdate>,
    pub active_modules: Vec<String>,
    pub disabled_modules: Vec<String>,
    pub reinforced_modules: Vec<String>,
    pub best_module: Option<String>,
    pub worst_module: Option<String>,
    pub system_accuracy: f64,
    pub read_only: bool,
    pub runtime_modified: bool,
    pub config_modified: bool,
    pub safety_notes: Vec<String>,
}

pub struct StrategyEvolutionEngine;

impl StrategyEvolutionEngine {
    pub fn evolve(
        current: &StrategyEvolutionState,
        metrics: &[StrategyPerformanceRecord],
        config: &StrategyEvolutionConfig,
    ) -> StrategyEvolutionReport {
        if metrics.is_empty() {
            return StrategyEvolutionReport {
                strategy_scores: Vec::new(),
                strategy_weights: current.strategy_weights.clone(),
                threshold_updates: Vec::new(),
                active_modules: current.active_modules.clone(),
                disabled_modules: Vec::new(),
                reinforced_modules: Vec::new(),
                best_module: None,
                worst_module: None,
                system_accuracy: 0.0,
                read_only: true,
                runtime_modified: false,
                config_modified: false,
                safety_notes: safety_notes(),
            };
        }

        let mut scores = metrics
            .iter()
            .map(|record| build_score(record, config))
            .collect::<Vec<_>>();
        scores.sort_by(|left, right| left.module.cmp(&right.module));

        let disabled_modules = scores
            .iter()
            .filter(|score| score.recommendation == StrategyEvolutionAction::Disable)
            .map(|score| score.module.clone())
            .collect::<Vec<_>>();
        let reinforced_modules = scores
            .iter()
            .filter(|score| score.recommendation == StrategyEvolutionAction::Reinforce)
            .map(|score| score.module.clone())
            .collect::<Vec<_>>();

        let active_modules = current
            .active_modules
            .iter()
            .filter(|module| !disabled_modules.contains(module))
            .cloned()
            .collect::<Vec<_>>();

        let score_by_module = scores
            .iter()
            .map(|score| (score.module.as_str(), score))
            .collect::<BTreeMap<_, _>>();
        let mut proposed_weights = current
            .strategy_weights
            .iter()
            .map(|(module, weight)| {
                let recommended = score_by_module
                    .get(module.as_str())
                    .map(|score| {
                        if score.recommendation == StrategyEvolutionAction::Disable {
                            0.0
                        } else {
                            bounded(
                                weight
                                    + config.learning_rate * (score.score - config.baseline_score),
                                config.min_weight,
                                config.max_weight,
                            )
                        }
                    })
                    .unwrap_or(*weight);
                (module.clone(), recommended)
            })
            .collect::<BTreeMap<_, _>>();
        normalize_active_weights(&mut proposed_weights, &active_modules);

        let threshold_updates = scores
            .iter()
            .filter_map(|score| build_threshold_update(score, current, config))
            .collect::<Vec<_>>();
        let best_module = scores
            .iter()
            .max_by(|left, right| left.score.total_cmp(&right.score))
            .map(|score| score.module.clone());
        let worst_module = scores
            .iter()
            .min_by(|left, right| left.score.total_cmp(&right.score))
            .map(|score| score.module.clone());

        StrategyEvolutionReport {
            strategy_scores: scores,
            strategy_weights: proposed_weights,
            threshold_updates,
            active_modules,
            disabled_modules,
            reinforced_modules,
            best_module,
            worst_module,
            system_accuracy: weighted_average_accuracy(metrics),
            read_only: true,
            runtime_modified: false,
            config_modified: false,
            safety_notes: safety_notes(),
        }
    }
}

fn build_score(
    record: &StrategyPerformanceRecord,
    config: &StrategyEvolutionConfig,
) -> StrategyScore {
    let accuracy = clamp01(record.accuracy);
    let profit_factor = if record.profit_factor.is_finite() {
        record.profit_factor.max(0.0)
    } else {
        0.0
    };
    let stability = clamp01(record.stability);
    let consistency = clamp01(record.consistency);
    let score = clamp01(accuracy * profit_factor * stability * consistency);
    let recommendation = classify(record, score, config);

    StrategyScore {
        module: record.module.clone(),
        score,
        accuracy,
        profit_factor,
        stability,
        consistency,
        false_positive_rate: clamp01(record.false_positive_rate),
        drawdown: clamp01(record.drawdown),
        signal_count: record.signal_count,
        recommendation,
    }
}

fn classify(
    record: &StrategyPerformanceRecord,
    score: f64,
    config: &StrategyEvolutionConfig,
) -> StrategyEvolutionAction {
    if record.accuracy < config.low_accuracy_disable_threshold
        && record.drawdown >= config.high_drawdown_disable_threshold
    {
        return StrategyEvolutionAction::Disable;
    }
    if record.accuracy >= config.reinforce_accuracy_threshold
        && score > config.baseline_score
        && record.false_positive_rate < config.false_positive_threshold
    {
        return StrategyEvolutionAction::Reinforce;
    }
    if record.false_positive_rate >= config.false_positive_threshold {
        return StrategyEvolutionAction::Tighten;
    }
    if score < config.baseline_score {
        return StrategyEvolutionAction::Downgrade;
    }
    StrategyEvolutionAction::Keep
}

fn build_threshold_update(
    score: &StrategyScore,
    current: &StrategyEvolutionState,
    config: &StrategyEvolutionConfig,
) -> Option<ThresholdUpdate> {
    let current_threshold = *current.thresholds.get(&score.module)?;
    let mut recommended_threshold = current_threshold;
    let mut reason = None;

    if score.false_positive_rate >= config.false_positive_threshold {
        recommended_threshold += config.threshold_step;
        reason = Some("false_positive_rate_high");
    } else if score.signal_count > config.high_signal_count_threshold {
        recommended_threshold += config.threshold_step * 0.5;
        reason = Some("too_many_signals");
    } else if score.recommendation == StrategyEvolutionAction::Reinforce {
        recommended_threshold -= config.threshold_step * 0.5;
        reason = Some("high_accuracy_reinforcement");
    }

    reason.map(|reason| ThresholdUpdate {
        module: score.module.clone(),
        current_threshold,
        recommended_threshold: bounded(recommended_threshold, 0.05, 0.95),
        reason: reason.to_string(),
    })
}

fn normalize_active_weights(weights: &mut BTreeMap<String, f64>, active_modules: &[String]) {
    let sum = active_modules
        .iter()
        .filter_map(|module| weights.get(module))
        .sum::<f64>();
    if sum <= 0.0 {
        return;
    }
    for (module, weight) in weights.iter_mut() {
        if active_modules.contains(module) {
            *weight /= sum;
        } else {
            *weight = 0.0;
        }
    }
}

fn weighted_average_accuracy(metrics: &[StrategyPerformanceRecord]) -> f64 {
    let total = metrics
        .iter()
        .map(|record| record.signal_count)
        .sum::<usize>();
    if total == 0 {
        return 0.0;
    }
    metrics
        .iter()
        .map(|record| clamp01(record.accuracy) * record.signal_count as f64)
        .sum::<f64>()
        / total as f64
}

fn bounded(value: f64, min: f64, max: f64) -> f64 {
    if !value.is_finite() {
        return min;
    }
    value.clamp(min, max)
}

fn clamp01(value: f64) -> f64 {
    bounded(value, 0.0, 1.0)
}

fn safety_notes() -> Vec<String> {
    vec![
        "recommendations_only_no_runtime_mutation".to_string(),
        "does_not_write_config".to_string(),
        "manual_review_required_before_apply".to_string(),
    ]
}
