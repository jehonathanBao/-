use std::collections::BTreeMap;

use super::calibration_types::{EventOutcome, OutcomeLabel, ReasonCodeStat};

pub fn build_reason_code_stats(outcomes: &[EventOutcome]) -> Vec<ReasonCodeStat> {
    let mut stats = BTreeMap::<String, ReasonCodeAccumulator>::new();
    for outcome in outcomes {
        for reason in &outcome.event.reason_codes {
            let entry = stats.entry(reason.clone()).or_default();
            entry.total_count += 1;
            match outcome.label {
                OutcomeLabel::Hit => entry.hit_count += 1,
                OutcomeLabel::FalsePositive => entry.false_positive_count += 1,
                OutcomeLabel::Neutral => entry.neutral_count += 1,
                OutcomeLabel::Unknown => entry.unknown_count += 1,
            }
        }
    }

    stats
        .into_iter()
        .map(|(reason_code, value)| ReasonCodeStat {
            reason_code,
            total_count: value.total_count,
            hit_count: value.hit_count,
            false_positive_count: value.false_positive_count,
            neutral_count: value.neutral_count,
            unknown_count: value.unknown_count,
            hit_rate: ratio(value.hit_count, value.total_count),
            false_positive_rate: ratio(value.false_positive_count, value.total_count),
        })
        .collect()
}

#[derive(Debug, Default)]
struct ReasonCodeAccumulator {
    total_count: usize,
    hit_count: usize,
    false_positive_count: usize,
    neutral_count: usize,
    unknown_count: usize,
}

fn ratio(numerator: usize, denominator: usize) -> f64 {
    if denominator == 0 {
        0.0
    } else {
        numerator as f64 / denominator as f64
    }
}
