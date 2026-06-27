use sha2::{Digest, Sha256};

use super::types::{ContractWhaleEventLifecycle, ContractWhaleEventStatus, ContractWhaleSignal};

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
        && existing.direction == next.direction
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
    let suppress_medium_refresh = repeated_medium_refresh(existing, next);
    let replace_snapshot = snapshot_is_better(next, existing);

    push_unique(&mut existing.merged_from, next.id.clone());
    for id in &next.merged_from {
        push_unique(&mut existing.merged_from, id.clone());
    }

    existing.event_lifecycle.last_update_time =
        existing.event_lifecycle.last_update_time.max(next.ts);
    existing.event_lifecycle.volume_accumulated = existing
        .event_lifecycle
        .volume_accumulated
        .max(next.total_volume_btc);
    existing.event_lifecycle.oi_accumulated = existing
        .event_lifecycle
        .oi_accumulated
        .max(oi_delta_abs(next));
    if !suppress_medium_refresh {
        existing.event_lifecycle.update_count = existing
            .event_lifecycle
            .update_count
            .saturating_add(next.merged_from.len().saturating_add(1));
    }

    if replace_snapshot {
        replace_snapshot_fields(existing, next);
    } else {
        existing.ts = existing.ts.max(next.ts);
        existing.window_sec = existing.window_sec.max(next.window_sec);
        existing.score = existing.score.max(next.score);
        existing.main_force_score = max_option_u8(existing.main_force_score, next.main_force_score);
        existing.spot_score = max_option_u8(existing.spot_score, next.spot_score);
        existing.contract_score = max_option_u8(existing.contract_score, next.contract_score);
        existing.data_quality = existing.data_quality.max(next.data_quality);
        existing.multi_exchange_confirmed |= next.multi_exchange_confirmed;
        existing.liquidation_suspected |= next.liquidation_suspected;
        existing.liquidation_long_btc =
            existing.liquidation_long_btc.max(next.liquidation_long_btc);
        existing.liquidation_short_btc = existing
            .liquidation_short_btc
            .max(next.liquidation_short_btc);
        existing.liquidation_notional_usd = existing
            .liquidation_notional_usd
            .max(next.liquidation_notional_usd);
    }

    existing.total_volume = existing.total_volume_btc;
    existing.net_volume = existing.net_volume_btc;
    existing.dominance = dominance(existing.net_volume_btc.abs(), existing.total_volume_btc);
    existing.liquidation_ratio = (existing.total_volume_btc > f64::EPSILON).then_some(
        (existing.liquidation_long_btc + existing.liquidation_short_btc)
            / existing.total_volume_btc,
    );
}

fn repeated_medium_refresh(existing: &ContractWhaleSignal, next: &ContractWhaleSignal) -> bool {
    existing.severity == super::types::ContractWhaleSeverity::Medium
        && next.severity == super::types::ContractWhaleSeverity::Medium
        && existing.symbol.eq_ignore_ascii_case(&next.symbol)
        && existing.direction == next.direction
        && existing.signal_type == next.signal_type
        && next
            .ts
            .saturating_sub(existing.event_lifecycle.last_update_time)
            <= EVENT_UPDATE_WINDOW_MS
}

fn snapshot_is_better(candidate: &ContractWhaleSignal, current: &ContractWhaleSignal) -> bool {
    current
        .severity
        .rank()
        .cmp(&candidate.severity.rank())
        .then_with(|| {
            current
                .total_volume_btc
                .partial_cmp(&candidate.total_volume_btc)
                .unwrap_or(std::cmp::Ordering::Equal)
        })
        .then_with(|| current.score.cmp(&candidate.score))
        .then_with(|| current.window_sec.cmp(&candidate.window_sec))
        .then_with(|| current.ts.cmp(&candidate.ts))
        .is_lt()
}

fn replace_snapshot_fields(existing: &mut ContractWhaleSignal, next: &ContractWhaleSignal) {
    existing.ts = next.ts;
    existing.window_sec = next.window_sec;
    existing.signal_type = next.signal_type;
    existing.direction = next.direction;
    existing.severity = next.severity;
    existing.score = next.score;
    existing.main_force_score = next.main_force_score;
    existing.spot_score = next.spot_score;
    existing.contract_score = next.contract_score;
    existing.total_volume_btc = next.total_volume_btc;
    existing.net_volume_btc = next.net_volume_btc;
    existing.total_notional_usd = next.total_notional_usd;
    existing.dominance = next.dominance;
    existing.order_price_usd = next.order_price_usd;
    existing.current_market_price_usd = next.current_market_price_usd;
    existing.price_deviation_pct = next.price_deviation_pct;
    existing.price_deviation_filtered = next.price_deviation_filtered;
    existing.price_move_pct = next.price_move_pct;
    existing.price_move_5s_pct = next.price_move_5s_pct;
    existing.price_move_15s_pct = next.price_move_15s_pct;
    existing.price_move_30s_pct = next.price_move_30s_pct;
    existing.price_response_type = next.price_response_type;
    existing.main_exchange = next.main_exchange.clone();
    existing.market_type = next.market_type;
    existing.source_role = next.source_role;
    existing.exchanges = next.exchanges.clone();
    existing.dominant_venue_net_contribution_share = next.dominant_venue_net_contribution_share;
    existing.dynamic_multiple = next.dynamic_multiple;
    existing.dynamic_baseline_btc = next.dynamic_baseline_btc;
    existing.dynamic_threshold_level = next.dynamic_threshold_level.clone();
    existing.percentile_level = next.percentile_level;
    existing.multi_exchange_confirmed = next.multi_exchange_confirmed;
    existing.liquidation_suspected = next.liquidation_suspected;
    existing.liquidation_long_btc = next.liquidation_long_btc;
    existing.liquidation_short_btc = next.liquidation_short_btc;
    existing.liquidation_notional_usd = next.liquidation_notional_usd;
    existing.liquidation_ratio = next.liquidation_ratio;
    existing.price_reversal_ratio = next.price_reversal_ratio;
    existing.oi_change_1m_btc = next.oi_change_1m_btc;
    existing.oi_change_5m_btc = next.oi_change_5m_btc;
    existing.oi_change_pct = next.oi_change_pct;
    existing.oi_bias = next.oi_bias.clone();
    existing.funding_rate = next.funding_rate;
    existing.funding_bias = next.funding_bias.clone();
    existing.data_quality = next.data_quality;
    existing.score_breakdown = next.score_breakdown.clone();
    existing.threshold_profile = next.threshold_profile.clone();
    existing.threshold_profile_reason = next.threshold_profile_reason.clone();
    existing.configured_contract_sources = next.configured_contract_sources.clone();
    existing.eligible_contract_sources = next.eligible_contract_sources.clone();
    existing.active_contract_sources = next.active_contract_sources.clone();
    existing.active_sources = next.active_sources.clone();
    existing.spot_confirmation = next.spot_confirmation.clone();
    existing.discord_eligible = next.discord_eligible;
    existing.discord_sent = next.discord_sent;
    existing.discord_sent_at = next.discord_sent_at;
    existing.discord_reason = next.discord_reason.clone();
    existing.discord_would_send = next.discord_would_send;
    existing.final_result = next.final_result.clone();
    existing.cluster = next.cluster.clone();
    existing.persistence = next.persistence.clone();
    existing.whale_action = next.whale_action.clone();
    existing.trajectory = next.trajectory.clone();
    existing.liquidation_force = next.liquidation_force.clone();
    existing.market_driver = next.market_driver.clone();
    existing.event_quality = next.event_quality.clone();
    existing.read_only = next.read_only;
    existing.analysis_only = next.analysis_only;
    existing.execution_enabled = next.execution_enabled;
    existing.total_volume = next.total_volume;
    existing.net_volume = next.net_volume;
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
fn push_unique(values: &mut Vec<String>, value: String) {
    if value.is_empty() || values.iter().any(|existing| existing == &value) {
        return;
    }
    values.push(value);
}
