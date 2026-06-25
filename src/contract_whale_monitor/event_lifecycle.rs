use std::collections::BTreeMap;

use sha2::{Digest, Sha256};

use super::types::{
    ContractWhaleEventLifecycle, ContractWhaleEventStatus, ContractWhaleSignal,
    ExchangeFlowContribution,
};

const EVENT_UPDATE_WINDOW_MS: i64 = 30_000;
const EVENT_CLOSE_AFTER_MS: i64 = 120_000;

pub fn apply_contract_whale_event_lifecycle(
    signals: Vec<ContractWhaleSignal>,
    reference_now_ms: i64,
) -> Vec<ContractWhaleSignal> {
    if signals.is_empty() {
        return signals;
    }
    let mut sorted = signals;
    sorted.sort_by(|left, right| left.ts.cmp(&right.ts).then_with(|| left.id.cmp(&right.id)));

    let mut events: Vec<ContractWhaleSignal> = Vec::new();
    for signal in sorted {
        if let Some(existing) = events
            .iter_mut()
            .rev()
            .find(|event| same_lifecycle_event(event, &signal))
        {
            update_lifecycle_event(existing, &signal);
        } else {
            events.push(start_lifecycle_event(signal));
        }
    }

    for event in events.iter_mut() {
        if reference_now_ms.saturating_sub(event.event_lifecycle.last_update_time)
            > EVENT_CLOSE_AFTER_MS
        {
            event.event_lifecycle.status = ContractWhaleEventStatus::Closed;
        } else {
            event.event_lifecycle.status = ContractWhaleEventStatus::Active;
        }
    }

    events
}

fn same_lifecycle_event(existing: &ContractWhaleSignal, next: &ContractWhaleSignal) -> bool {
    existing.symbol.eq_ignore_ascii_case(&next.symbol)
        && existing.signal_type == next.signal_type
        && next
            .ts
            .saturating_sub(existing.event_lifecycle.last_update_time)
            <= EVENT_UPDATE_WINDOW_MS
        && next.ts >= existing.event_lifecycle.last_update_time
}

fn start_lifecycle_event(mut signal: ContractWhaleSignal) -> ContractWhaleSignal {
    signal.event_lifecycle = ContractWhaleEventLifecycle {
        event_id: lifecycle_event_id(&signal),
        start_time: signal.ts,
        last_update_time: signal.ts,
        status: ContractWhaleEventStatus::Active,
        volume_accumulated: signal.total_volume_btc,
        oi_accumulated: oi_delta_abs(&signal),
        update_count: signal.merged_from.len().saturating_add(1),
    };
    signal
}

fn update_lifecycle_event(existing: &mut ContractWhaleSignal, next: &ContractWhaleSignal) {
    push_unique(&mut existing.merged_from, next.id.clone());
    for id in &next.merged_from {
        push_unique(&mut existing.merged_from, id.clone());
    }

    existing.event_lifecycle.last_update_time =
        existing.event_lifecycle.last_update_time.max(next.ts);
    existing.event_lifecycle.volume_accumulated += next.total_volume_btc;
    existing.event_lifecycle.oi_accumulated += oi_delta_abs(next);
    existing.event_lifecycle.update_count = existing
        .event_lifecycle
        .update_count
        .saturating_add(next.merged_from.len().saturating_add(1));

    existing.ts = existing.ts.max(next.ts);
    existing.window_sec = existing.window_sec.max(next.window_sec);
    existing.score = existing.score.max(next.score);
    existing.main_force_score = max_option_u8(existing.main_force_score, next.main_force_score);
    existing.spot_score = max_option_u8(existing.spot_score, next.spot_score);
    existing.contract_score = max_option_u8(existing.contract_score, next.contract_score);
    existing.data_quality = existing.data_quality.max(next.data_quality);
    existing.total_volume_btc += next.total_volume_btc;
    existing.net_volume_btc += next.net_volume_btc;
    existing.total_notional_usd += next.total_notional_usd;
    existing.total_volume = existing.total_volume_btc;
    existing.net_volume = existing.net_volume_btc;
    existing.dominance = dominance(existing.net_volume_btc.abs(), existing.total_volume_btc);
    existing.multi_exchange_confirmed |= next.multi_exchange_confirmed;
    existing.liquidation_suspected |= next.liquidation_suspected;
    existing.liquidation_long_btc += next.liquidation_long_btc;
    existing.liquidation_short_btc += next.liquidation_short_btc;
    existing.liquidation_notional_usd += next.liquidation_notional_usd;
    existing.liquidation_ratio = (existing.total_volume_btc > f64::EPSILON).then_some(
        (existing.liquidation_long_btc + existing.liquidation_short_btc)
            / existing.total_volume_btc,
    );
    merge_exchange_contributions(&mut existing.exchanges, &next.exchanges);
}

fn lifecycle_event_id(signal: &ContractWhaleSignal) -> String {
    let price = signal.order_price_usd.unwrap_or_default();
    let payload = format!(
        "{}|{}|{:?}|{}|{:?}|{price:.8}|{net_volume:.8}",
        signal.symbol.to_ascii_uppercase(),
        signal.ts,
        signal.signal_type,
        signal.window_sec,
        signal.direction,
        net_volume = signal.net_volume_btc,
    );
    let digest = Sha256::digest(payload.as_bytes());
    format!("cwm-event-{:x}", digest)
}

fn oi_delta_abs(signal: &ContractWhaleSignal) -> f64 {
    signal
        .oi_change_1m_btc
        .or(signal.oi_change_5m_btc)
        .unwrap_or_default()
        .abs()
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
    incoming_items: &[ExchangeFlowContribution],
) {
    if incoming_items.is_empty() {
        return;
    }
    let mut by_exchange: BTreeMap<String, ExchangeFlowContribution> = keeper
        .drain(..)
        .map(|item| (item.exchange.to_ascii_lowercase(), item))
        .collect();
    for incoming in incoming_items {
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

fn push_unique(values: &mut Vec<String>, value: String) {
    if value.is_empty() || values.iter().any(|existing| existing == &value) {
        return;
    }
    values.push(value);
}
