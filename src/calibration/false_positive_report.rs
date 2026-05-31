use super::calibration_types::{EventOutcome, OutcomeLabel};

pub fn top_false_positives(outcomes: &[EventOutcome], limit: usize) -> Vec<EventOutcome> {
    let mut filtered = outcomes
        .iter()
        .filter(|outcome| outcome.label == OutcomeLabel::FalsePositive)
        .cloned()
        .collect::<Vec<_>>();
    filtered.sort_by(|left, right| {
        left.primary_move_bps
            .unwrap_or_default()
            .total_cmp(&right.primary_move_bps.unwrap_or_default())
    });
    filtered.truncate(limit);
    filtered
}

pub fn top_hits(outcomes: &[EventOutcome], limit: usize) -> Vec<EventOutcome> {
    let mut filtered = outcomes
        .iter()
        .filter(|outcome| outcome.label == OutcomeLabel::Hit)
        .cloned()
        .collect::<Vec<_>>();
    filtered.sort_by(|left, right| {
        right
            .primary_move_bps
            .unwrap_or_default()
            .total_cmp(&left.primary_move_bps.unwrap_or_default())
    });
    filtered.truncate(limit);
    filtered
}
