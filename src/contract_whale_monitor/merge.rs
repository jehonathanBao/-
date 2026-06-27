use super::types::{ContractWhaleSeverity, ContractWhaleSignal};

const MERGE_WINDOW_MS: u64 = 60_000;

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
        && left.window_sec != right.window_sec
        && left.ts.abs_diff(right.ts) <= MERGE_WINDOW_MS
        && price_range_within_event(left, right)
}

fn representative_is_better(left: &ContractWhaleSignal, right: &ContractWhaleSignal) -> bool {
    severity_rank(left.severity)
        .cmp(&severity_rank(right.severity))
        .then_with(|| {
            left.total_volume_btc
                .partial_cmp(&right.total_volume_btc)
                .unwrap_or(std::cmp::Ordering::Equal)
        })
        .then_with(|| left.score.cmp(&right.score))
        .then_with(|| left.window_sec.cmp(&right.window_sec))
        .then_with(|| left.ts.cmp(&right.ts))
        .is_gt()
}

fn severity_rank(severity: ContractWhaleSeverity) -> u8 {
    severity.rank()
}

fn price_range_within_event(left: &ContractWhaleSignal, right: &ContractWhaleSignal) -> bool {
    let Some(left_price) = event_price(left) else {
        return true;
    };
    let Some(right_price) = event_price(right) else {
        return true;
    };
    let anchor = left_price.abs().max(right_price.abs()).max(1.0);
    ((left_price - right_price).abs() / anchor) <= 0.003
}

fn event_price(signal: &ContractWhaleSignal) -> Option<f64> {
    signal
        .order_price_usd
        .or(signal.current_market_price_usd)
        .filter(|price| price.is_finite() && *price > 0.0)
}

fn absorb_merged_signal(keeper: &mut ContractWhaleSignal, merged: &ContractWhaleSignal) {
    push_unique(&mut keeper.merged_from, merged.id.clone());
    for id in &merged.merged_from {
        push_unique(&mut keeper.merged_from, id.clone());
    }
    keeper.ts = keeper.ts.max(merged.ts);
    keeper.window_sec = keeper.window_sec.max(merged.window_sec);
    keeper.score = keeper.score.max(merged.score);
    keeper.main_force_score = max_option_u8(keeper.main_force_score, merged.main_force_score);
    keeper.spot_score = max_option_u8(keeper.spot_score, merged.spot_score);
    keeper.contract_score = max_option_u8(keeper.contract_score, merged.contract_score);
    keeper.data_quality = keeper.data_quality.max(merged.data_quality);
    keeper.total_volume = keeper.total_volume_btc;
    keeper.net_volume = keeper.net_volume_btc;
    keeper.dominance = dominance(keeper.net_volume_btc.abs(), keeper.total_volume_btc);
    keeper.liquidation_suspected |= merged.liquidation_suspected;
    keeper.liquidation_long_btc = keeper.liquidation_long_btc.max(merged.liquidation_long_btc);
    keeper.liquidation_short_btc = keeper
        .liquidation_short_btc
        .max(merged.liquidation_short_btc);
    keeper.liquidation_notional_usd = keeper
        .liquidation_notional_usd
        .max(merged.liquidation_notional_usd);
    keeper.liquidation_ratio = (keeper.total_volume_btc > f64::EPSILON).then_some(
        (keeper.liquidation_long_btc + keeper.liquidation_short_btc) / keeper.total_volume_btc,
    );
    keeper.multi_exchange_confirmed |= merged.multi_exchange_confirmed;
}

fn push_unique(values: &mut Vec<String>, value: String) {
    if value.is_empty() || values.iter().any(|existing| existing == &value) {
        return;
    }
    values.push(value);
}

fn max_option_u8(left: Option<u8>, right: Option<u8>) -> Option<u8> {
    match (left, right) {
        (Some(left), Some(right)) => Some(left.max(right)),
        (Some(value), None) | (None, Some(value)) => Some(value),
        (None, None) => None,
    }
}

fn dominance(net_abs: f64, total: f64) -> f64 {
    if total > f64::EPSILON {
        (net_abs / total).clamp(0.0, 1.0)
    } else {
        0.0
    }
}
