use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum PredictionScope {
    Signal,
    Event,
    Regime,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PredictionRecord {
    pub id: String,
    pub timestamp: i64,
    pub symbol: String,
    pub module: String,
    pub signal_type: String,
    pub predicted_direction: String,
    pub predicted_regime: Option<String>,
    pub confidence: f64,
    pub scope: PredictionScope,
    pub context: serde_json::Value,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct OutcomeRecord {
    pub prediction_id: String,
    pub validated_at: i64,
    pub horizon_ms: u64,
    pub price_direction: String,
    pub move_percent: f64,
    pub actual_regime: Option<String>,
    pub volatility: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ModuleAnalytics {
    pub module: String,
    pub total_predictions: usize,
    pub resolved_predictions: usize,
    pub true_positive_count: usize,
    pub false_positive_count: usize,
    pub precision: f64,
    pub false_positive_rate: f64,
    pub average_confidence: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct FalsePositiveAnalyticsReport {
    pub total_predictions: usize,
    pub resolved_predictions: usize,
    pub unresolved_predictions: usize,
    pub true_positive_count: usize,
    pub false_positive_count: usize,
    pub signal_accuracy: f64,
    pub signal_precision: f64,
    pub false_positive_rate: f64,
    pub regime_accuracy: f64,
    pub event_reliability: f64,
    pub best_performing_module: Option<String>,
    pub worst_performing_module: Option<String>,
    pub module_breakdown: Vec<ModuleAnalytics>,
}

pub struct FalsePositiveAnalyticsEngine;

impl FalsePositiveAnalyticsEngine {
    pub fn analyze(
        predictions: &[PredictionRecord],
        outcomes: &[OutcomeRecord],
    ) -> FalsePositiveAnalyticsReport {
        let outcome_by_prediction_id = outcomes
            .iter()
            .map(|outcome| (outcome.prediction_id.as_str(), outcome))
            .collect::<BTreeMap<_, _>>();

        let total_predictions = predictions.len();
        let mut resolved_predictions = 0usize;
        let mut true_positive_count = 0usize;
        let mut false_positive_count = 0usize;
        let mut directional_resolved_count = 0usize;
        let mut directional_correct_count = 0usize;
        let mut directional_positive_count = 0usize;
        let mut directional_positive_correct_count = 0usize;
        let mut regime_resolved_count = 0usize;
        let mut regime_correct_count = 0usize;
        let mut event_resolved_count = 0usize;
        let mut event_reliable_count = 0usize;
        let mut module_accumulator = BTreeMap::<String, ModuleAccumulator>::new();

        for prediction in predictions {
            let module = module_accumulator
                .entry(prediction.module.clone())
                .or_default();
            module.total_predictions += 1;
            module.confidence_sum += prediction.confidence.clamp(0.0, 1.0);

            let Some(outcome) = outcome_by_prediction_id.get(prediction.id.as_str()) else {
                continue;
            };

            resolved_predictions += 1;
            module.resolved_predictions += 1;

            let predicted_direction = normalize_direction(&prediction.predicted_direction);
            let actual_direction = normalize_direction(&outcome.price_direction);
            if predicted_direction != DirectionClass::Unknown
                && actual_direction != DirectionClass::Unknown
            {
                directional_resolved_count += 1;
                if is_positive_direction(predicted_direction) {
                    directional_positive_count += 1;
                }

                let direction_correct = predicted_direction == actual_direction;
                if direction_correct {
                    directional_correct_count += 1;
                    if is_positive_direction(predicted_direction) {
                        directional_positive_correct_count += 1;
                    }
                    module.true_positive_count += 1;
                    true_positive_count += 1;
                } else if is_positive_direction(predicted_direction) {
                    module.false_positive_count += 1;
                    false_positive_count += 1;
                }
            }

            if let (Some(predicted_regime), Some(actual_regime)) =
                (&prediction.predicted_regime, &outcome.actual_regime)
            {
                regime_resolved_count += 1;
                if normalize_label(predicted_regime) == normalize_label(actual_regime) {
                    regime_correct_count += 1;
                }
            }

            if prediction.scope == PredictionScope::Event {
                event_resolved_count += 1;
                if predicted_direction == actual_direction
                    && is_positive_direction(predicted_direction)
                    && outcome.move_percent.abs() >= 0.10
                {
                    event_reliable_count += 1;
                }
            }
        }

        let module_breakdown = module_accumulator
            .into_iter()
            .map(|(module, accumulator)| accumulator.into_report(module))
            .collect::<Vec<_>>();
        let best_performing_module = module_breakdown
            .iter()
            .filter(|module| module.resolved_predictions > 0)
            .max_by(|left, right| {
                left.precision
                    .total_cmp(&right.precision)
                    .then_with(|| right.module.cmp(&left.module))
            })
            .map(|module| module.module.clone());
        let worst_performing_module = module_breakdown
            .iter()
            .filter(|module| module.resolved_predictions > 0)
            .min_by(|left, right| {
                left.precision
                    .total_cmp(&right.precision)
                    .then_with(|| left.module.cmp(&right.module))
            })
            .map(|module| module.module.clone());

        FalsePositiveAnalyticsReport {
            total_predictions,
            resolved_predictions,
            unresolved_predictions: total_predictions.saturating_sub(resolved_predictions),
            true_positive_count,
            false_positive_count,
            signal_accuracy: ratio(directional_correct_count, directional_resolved_count),
            signal_precision: ratio(
                directional_positive_correct_count,
                directional_positive_count,
            ),
            false_positive_rate: ratio(false_positive_count, directional_positive_count),
            regime_accuracy: ratio(regime_correct_count, regime_resolved_count),
            event_reliability: ratio(event_reliable_count, event_resolved_count),
            best_performing_module,
            worst_performing_module,
            module_breakdown,
        }
    }
}

#[derive(Debug, Default)]
struct ModuleAccumulator {
    total_predictions: usize,
    resolved_predictions: usize,
    true_positive_count: usize,
    false_positive_count: usize,
    confidence_sum: f64,
}

impl ModuleAccumulator {
    fn into_report(self, module: String) -> ModuleAnalytics {
        ModuleAnalytics {
            module,
            total_predictions: self.total_predictions,
            resolved_predictions: self.resolved_predictions,
            true_positive_count: self.true_positive_count,
            false_positive_count: self.false_positive_count,
            precision: ratio(self.true_positive_count, self.resolved_predictions),
            false_positive_rate: ratio(self.false_positive_count, self.resolved_predictions),
            average_confidence: ratio_float(self.confidence_sum, self.total_predictions),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum DirectionClass {
    Up,
    Down,
    Neutral,
    Unknown,
}

fn normalize_direction(value: &str) -> DirectionClass {
    match normalize_label(value).as_str() {
        "LONG" | "BUY" | "UP" | "BULLISH" => DirectionClass::Up,
        "SHORT" | "SELL" | "DOWN" | "BEARISH" => DirectionClass::Down,
        "NONE" | "NO_TRADE" | "NEUTRAL" | "FLAT" => DirectionClass::Neutral,
        _ => DirectionClass::Unknown,
    }
}

fn normalize_label(value: &str) -> String {
    value.trim().replace(['-', ' '], "_").to_ascii_uppercase()
}

fn is_positive_direction(direction: DirectionClass) -> bool {
    matches!(direction, DirectionClass::Up | DirectionClass::Down)
}

fn ratio(numerator: usize, denominator: usize) -> f64 {
    if denominator == 0 {
        0.0
    } else {
        numerator as f64 / denominator as f64
    }
}

fn ratio_float(numerator: f64, denominator: usize) -> f64 {
    if denominator == 0 {
        0.0
    } else {
        numerator / denominator as f64
    }
}
