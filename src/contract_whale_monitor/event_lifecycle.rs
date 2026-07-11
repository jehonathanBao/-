use std::collections::BTreeSet;

use sha2::{Digest, Sha256};

use super::{
    config::contract_whale_runtime_config,
    discord::{contract_whale_gate_symbol, effective_push_total_volume},
    discord_gate::discord_gate,
    types::{
        ContractFlowBucket, ContractWhaleEventLifecycle, ContractWhaleEventStatus,
        ContractWhaleSignal, ContractWhaleSignalSnapshot,
    },
};

const DEFAULT_EVENT_UPDATE_WINDOW_MS: i64 = 30_000;
const DEFAULT_EVENT_CLOSE_AFTER_MS: i64 = 120_000;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ContractWhaleLifecycleClock {
    Live { now_ms: i64 },
    Replay { replay_now_ms: i64 },
}

impl ContractWhaleLifecycleClock {
    fn reference_now_ms(self) -> i64 {
        match self {
            Self::Live { now_ms } => now_ms,
            Self::Replay { replay_now_ms } => replay_now_ms,
        }
    }
}

pub fn apply_contract_whale_event_lifecycle(
    signals: Vec<ContractWhaleSignal>,
    clock: ContractWhaleLifecycleClock,
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

    let reference_now_ms = clock.reference_now_ms();
    for event in events.iter_mut() {
        if reference_now_ms.saturating_sub(event.event_lifecycle.last_update_time)
            > lifecycle_close_after_ms()
        {
            event.event_lifecycle.status = ContractWhaleEventStatus::Closed;
        } else {
            event.event_lifecycle.status = ContractWhaleEventStatus::Active;
        }
        refresh_lifecycle_discord_gate(event);
    }

    events
}

pub fn enrich_lifecycle_unique_turnover(
    events: &mut [ContractWhaleSignal],
    buckets: &[ContractFlowBucket],
    failed_symbols: &BTreeSet<String>,
) {
    if !contract_whale_runtime_config()
        .lifecycle
        .unique_turnover_enabled
    {
        for event in events {
            event.event_lifecycle.unique_turnover_btc = None;
            event.event_lifecycle.unique_turnover_available = false;
            event.event_lifecycle.unique_turnover_reason = Some("feature_disabled".to_string());
        }
        return;
    }
    for event in events {
        let symbol = event.symbol.to_ascii_uppercase();
        if failed_symbols.contains(&symbol) {
            event.event_lifecycle.unique_turnover_btc = None;
            event.event_lifecycle.unique_turnover_available = false;
            event.event_lifecycle.unique_turnover_reason = Some("flow_query_failed".to_string());
            continue;
        }

        let start_ts = lifecycle_raw_start_ts(event);
        let end_ts = event.event_lifecycle.last_update_time.max(event.ts);
        let mut unique_buckets = BTreeSet::new();
        let mut turnover = 0.0;
        for bucket in buckets.iter().filter(|bucket| {
            bucket.symbol.eq_ignore_ascii_case(&event.symbol)
                && bucket.ts_bucket >= start_ts
                && bucket.ts_bucket <= end_ts
        }) {
            let key = (
                bucket.symbol.to_ascii_uppercase(),
                bucket.exchange.to_ascii_lowercase(),
                bucket.ts_bucket,
            );
            if unique_buckets.insert(key) {
                turnover +=
                    (bucket.buy_volume_btc.max(0.0) + bucket.sell_volume_btc.max(0.0)).max(0.0);
            }
        }
        if unique_buckets.is_empty() {
            event.event_lifecycle.unique_turnover_btc = None;
            event.event_lifecycle.unique_turnover_available = false;
            event.event_lifecycle.unique_turnover_reason = Some("raw_flow_missing".to_string());
        } else {
            event.event_lifecycle.unique_turnover_btc = Some(turnover);
            event.event_lifecycle.unique_turnover_available = true;
            event.event_lifecycle.unique_turnover_reason = None;
        }
    }
}

pub fn lifecycle_raw_start_ts(signal: &ContractWhaleSignal) -> i64 {
    let first_snapshot_ts = if signal.event_lifecycle.start_time > 0 {
        signal.event_lifecycle.start_time
    } else {
        signal.ts
    };
    first_snapshot_ts.saturating_sub((signal.window_sec as i64).saturating_mul(1000))
}

fn same_lifecycle_event(existing: &ContractWhaleSignal, next: &ContractWhaleSignal) -> bool {
    existing.symbol.eq_ignore_ascii_case(&next.symbol)
        && existing.direction == next.direction
        && existing.signal_type == next.signal_type
        && next
            .ts
            .saturating_sub(existing.event_lifecycle.last_update_time)
            <= lifecycle_update_window_ms()
        && next.ts >= existing.event_lifecycle.last_update_time
}

fn start_lifecycle_event(mut signal: ContractWhaleSignal) -> ContractWhaleSignal {
    signal.event_lifecycle = ContractWhaleEventLifecycle {
        event_id: lifecycle_event_id(&signal),
        start_time: signal.ts,
        last_update_time: signal.ts,
        status: ContractWhaleEventStatus::Active,
        latest_window_volume_btc: signal.total_volume_btc.max(0.0),
        peak_window_volume_btc: signal.total_volume_btc.max(0.0),
        unique_turnover_btc: None,
        unique_turnover_available: false,
        unique_turnover_reason: Some("raw_flow_not_enriched".to_string()),
        net_oi_delta_btc: oi_delta(&signal),
        peak_abs_oi_delta_btc: Some(oi_delta_abs(&signal)),
        latest_snapshot_ts: signal.ts,
        peak_snapshot_ts: signal.ts,
        display_snapshot_kind: "peak".to_string(),
        latest_snapshot: Some(ContractWhaleSignalSnapshot::from_signal(&signal)),
        peak_snapshot: Some(ContractWhaleSignalSnapshot::from_signal(&signal)),
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
    existing.event_lifecycle.latest_window_volume_btc = next.total_volume_btc.max(0.0);
    existing.event_lifecycle.peak_window_volume_btc = existing
        .event_lifecycle
        .peak_window_volume_btc
        .max(next.total_volume_btc.max(0.0));
    existing.event_lifecycle.volume_accumulated = existing.event_lifecycle.peak_window_volume_btc;
    existing.event_lifecycle.net_oi_delta_btc = oi_delta(next);
    existing.event_lifecycle.peak_abs_oi_delta_btc = Some(
        existing
            .event_lifecycle
            .peak_abs_oi_delta_btc
            .unwrap_or_default()
            .max(oi_delta_abs(next)),
    );
    existing.event_lifecycle.oi_accumulated = existing
        .event_lifecycle
        .peak_abs_oi_delta_btc
        .unwrap_or_default();
    existing.event_lifecycle.latest_snapshot_ts = next.ts;
    existing.event_lifecycle.latest_snapshot = Some(ContractWhaleSignalSnapshot::from_signal(next));
    if !suppress_medium_refresh {
        existing.event_lifecycle.update_count = existing
            .event_lifecycle
            .update_count
            .saturating_add(next.merged_from.len().saturating_add(1));
    }

    if replace_snapshot {
        replace_snapshot_fields(existing, next);
        existing.event_lifecycle.peak_snapshot_ts = next.ts;
        existing.event_lifecycle.peak_snapshot =
            Some(ContractWhaleSignalSnapshot::from_signal(next));
    }

    existing.total_volume = existing.total_volume_btc;
    existing.net_volume = existing.net_volume_btc;
    existing.dominance = dominance(existing.net_volume_btc.abs(), existing.total_volume_btc);
    existing.liquidation_ratio = (existing.total_volume_btc > f64::EPSILON).then_some(
        (existing.liquidation_long_btc + existing.liquidation_short_btc)
            / existing.total_volume_btc,
    );
}

fn refresh_lifecycle_discord_gate(signal: &mut ContractWhaleSignal) {
    let warmup_collect_only = signal.discord_reason == "warmup_collect_only";
    let primary_source_override = signal.discord_reason == "high_primary_source_extreme";
    let (mut discord_eligible, mut discord_reason) = discord_gate(
        signal.severity,
        signal.score,
        signal.multi_exchange_confirmed,
        signal.data_quality,
        primary_source_override,
        contract_whale_gate_symbol(signal),
        effective_push_total_volume(signal),
        signal.impact_level.as_deref(),
        &contract_whale_runtime_config(),
    );
    if warmup_collect_only {
        discord_eligible = false;
        discord_reason = "warmup_collect_only".to_string();
    }
    signal.discord_eligible = discord_eligible;
    signal.discord_would_send = discord_eligible;
    signal.discord_reason = discord_reason;
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
            <= lifecycle_update_window_ms()
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
    existing.classification_v2 = next.classification_v2.clone();
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
    oi_delta(signal).unwrap_or_default().abs()
}

fn oi_delta(signal: &ContractWhaleSignal) -> Option<f64> {
    signal.oi_change_1m_btc.or(signal.oi_change_5m_btc)
}

fn dominance(net_abs: f64, total: f64) -> f64 {
    if total > f64::EPSILON {
        (net_abs / total).clamp(0.0, 1.0)
    } else {
        0.0
    }
}

fn lifecycle_update_window_ms() -> i64 {
    contract_whale_runtime_config()
        .lifecycle
        .update_window_seconds
        .max(1)
        .saturating_mul(1_000)
        .max(DEFAULT_EVENT_UPDATE_WINDOW_MS.min(1_000))
}

fn lifecycle_close_after_ms() -> i64 {
    contract_whale_runtime_config()
        .lifecycle
        .close_after_seconds
        .max(1)
        .saturating_mul(1_000)
        .max(DEFAULT_EVENT_CLOSE_AFTER_MS.min(1_000))
}
fn push_unique(values: &mut Vec<String>, value: String) {
    if value.is_empty() || values.iter().any(|existing| existing == &value) {
        return;
    }
    values.push(value);
}
