use std::collections::BTreeMap;

use super::types::{ContractWhaleSeverity, ContractWhaleSignal, ExchangeFlowContribution};

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
    keeper.total_volume_btc += merged.total_volume_btc;
    keeper.net_volume_btc += merged.net_volume_btc;
    keeper.total_notional_usd += merged.total_notional_usd;
    keeper.total_volume = keeper.total_volume_btc;
    keeper.net_volume = keeper.net_volume_btc;
    keeper.dominance = dominance(keeper.net_volume_btc.abs(), keeper.total_volume_btc);
    keeper.liquidation_suspected |= merged.liquidation_suspected;
    keeper.liquidation_long_btc += merged.liquidation_long_btc;
    keeper.liquidation_short_btc += merged.liquidation_short_btc;
    keeper.liquidation_notional_usd += merged.liquidation_notional_usd;
    keeper.liquidation_ratio = (keeper.total_volume_btc > f64::EPSILON).then_some(
        (keeper.liquidation_long_btc + keeper.liquidation_short_btc) / keeper.total_volume_btc,
    );
    keeper.multi_exchange_confirmed |= merged.multi_exchange_confirmed;
    merge_exchange_contributions(&mut keeper.exchanges, &merged.exchanges);
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

fn merge_exchange_contributions(
    keeper: &mut Vec<ExchangeFlowContribution>,
    merged: &[ExchangeFlowContribution],
) {
    if merged.is_empty() {
        return;
    }
    let mut by_exchange: BTreeMap<String, ExchangeFlowContribution> = keeper
        .drain(..)
        .map(|item| (item.exchange.to_ascii_lowercase(), item))
        .collect();
    for incoming in merged {
        let key = incoming.exchange.to_ascii_lowercase();
        by_exchange
            .entry(key)
            .and_modify(|existing| {
                existing.buy_volume_btc += incoming.buy_volume_btc;
                existing.sell_volume_btc += incoming.sell_volume_btc;
                existing.buy_notional_usd += incoming.buy_notional_usd;
                existing.sell_notional_usd += incoming.sell_notional_usd;
                existing.trade_count = existing.trade_count.saturating_add(incoming.trade_count);
                refresh_exchange_contribution(existing);
            })
            .or_insert_with(|| {
                let mut item = incoming.clone();
                refresh_exchange_contribution(&mut item);
                item
            });
    }
    let total_abs_net: f64 = by_exchange
        .values()
        .map(|item| item.net_volume_btc.abs())
        .sum();
    *keeper = by_exchange.into_values().collect();
    for item in keeper.iter_mut() {
        item.net_contribution_share = if total_abs_net > f64::EPSILON {
            item.net_volume_btc.abs() / total_abs_net
        } else {
            0.0
        };
    }
    keeper.sort_by(|left, right| {
        right
            .total_volume_btc
            .partial_cmp(&left.total_volume_btc)
            .unwrap_or(std::cmp::Ordering::Equal)
    });
}

fn refresh_exchange_contribution(item: &mut ExchangeFlowContribution) {
    item.total_volume_btc = item.buy_volume_btc + item.sell_volume_btc;
    item.net_volume_btc = item.buy_volume_btc - item.sell_volume_btc;
    item.buy_share = dominance(item.buy_volume_btc, item.total_volume_btc);
    item.sell_share = dominance(item.sell_volume_btc, item.total_volume_btc);
    item.total_notional_usd = item.buy_notional_usd + item.sell_notional_usd;
    item.dominance = dominance(item.net_volume_btc.abs(), item.total_volume_btc);
}
