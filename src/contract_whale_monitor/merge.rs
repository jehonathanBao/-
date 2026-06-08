use super::types::{ContractWhaleSeverity, ContractWhaleSignal};

const MERGE_WINDOW_MS: u64 = 10_000;

pub fn merge_contract_whale_signals(
    mut signals: Vec<ContractWhaleSignal>,
) -> Vec<ContractWhaleSignal> {
    signals.sort_by(|left, right| {
        left.ts
            .cmp(&right.ts)
            .then_with(|| left.window_sec.cmp(&right.window_sec))
    });
    let mut merged: Vec<ContractWhaleSignal> = Vec::new();
    for signal in signals {
        if let Some(index) = merged
            .iter()
            .position(|existing| same_merge_group(existing, &signal))
        {
            if representative_is_better(&signal, &merged[index]) {
                let existing = merged.remove(index);
                let mut replacement = signal;
                absorb_merged_signal(&mut replacement, &existing);
                merged.insert(index, replacement);
            } else {
                absorb_merged_signal(&mut merged[index], &signal);
            }
        } else {
            merged.push(signal);
        }
    }
    merged
}

fn same_merge_group(left: &ContractWhaleSignal, right: &ContractWhaleSignal) -> bool {
    left.symbol.eq_ignore_ascii_case(&right.symbol)
        && left.direction == right.direction
        && left.signal_type == right.signal_type
        && left.ts.abs_diff(right.ts) <= MERGE_WINDOW_MS
}

fn representative_is_better(left: &ContractWhaleSignal, right: &ContractWhaleSignal) -> bool {
    severity_rank(left.severity)
        .cmp(&severity_rank(right.severity))
        .then_with(|| left.score.cmp(&right.score))
        .then_with(|| left.window_sec.cmp(&right.window_sec))
        .then_with(|| left.ts.cmp(&right.ts))
        .is_gt()
}

fn severity_rank(severity: ContractWhaleSeverity) -> u8 {
    severity.rank()
}

fn absorb_merged_signal(keeper: &mut ContractWhaleSignal, merged: &ContractWhaleSignal) {
    push_unique(&mut keeper.merged_from, merged.id.clone());
    for id in &merged.merged_from {
        push_unique(&mut keeper.merged_from, id.clone());
    }
}

fn push_unique(values: &mut Vec<String>, value: String) {
    if value.is_empty() || values.iter().any(|existing| existing == &value) {
        return;
    }
    values.push(value);
}
