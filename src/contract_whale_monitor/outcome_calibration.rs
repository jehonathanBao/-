use std::collections::BTreeMap;

use super::types::{ContractFlowBucket, ContractWhaleActiveFlowDirection, ContractWhaleSignal};

pub const CONTRACT_WHALE_OUTCOME_VERSION: &str = "v1_shadow";

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ContractWhaleSignalOutcome {
    pub signal_id: String,
    pub symbol: String,
    pub signal_ts: i64,
    pub signal_type: String,
    pub classification_v2: String,
    pub severity: String,
    pub impact_level: Option<String>,
    pub window_sec: u64,
    pub oi_context: String,
    pub regime: String,
    pub entry_price: f64,
    pub markout_30s_bps: Option<f64>,
    pub markout_2m_bps: Option<f64>,
    pub markout_5m_bps: Option<f64>,
    pub mfe_5m_bps: Option<f64>,
    pub mae_5m_bps: Option<f64>,
    pub follow_through_30s: Option<bool>,
    pub follow_through_2m: Option<bool>,
    pub follow_through_5m: Option<bool>,
    pub evaluated_at: i64,
    pub outcome_version: String,
}

pub fn evaluate_contract_whale_signal_outcome(
    signal: &ContractWhaleSignal,
    buckets: &[ContractFlowBucket],
    now_ms: i64,
) -> Option<ContractWhaleSignalOutcome> {
    if now_ms < signal.ts.saturating_add(30_000) {
        return None;
    }
    let direction = match signal.classification_v2.flow_direction {
        ContractWhaleActiveFlowDirection::BuyDominant => 1.0,
        ContractWhaleActiveFlowDirection::SellDominant => -1.0,
        ContractWhaleActiveFlowDirection::Balanced | ContractWhaleActiveFlowDirection::Unknown => {
            return None;
        }
    };
    let prices = weighted_prices_by_second(signal, buckets, now_ms);
    let entry_price = signal
        .order_price_usd
        .filter(|value| value.is_finite() && *value > 0.0)
        .or_else(|| prices.first().map(|(_, price)| *price))?;
    let markout = |horizon_ms: i64| {
        (now_ms >= signal.ts.saturating_add(horizon_ms))
            .then(|| price_at_or_before(&prices, signal.ts.saturating_add(horizon_ms)))
            .flatten()
            .map(|price| signed_markout_bps(entry_price, price, direction))
    };
    let horizon_prices = prices
        .iter()
        .filter(|(ts, _)| *ts <= signal.ts.saturating_add(300_000))
        .map(|(_, price)| signed_markout_bps(entry_price, *price, direction))
        .collect::<Vec<_>>();
    let fully_evaluated = now_ms >= signal.ts.saturating_add(300_000);
    Some(ContractWhaleSignalOutcome {
        signal_id: signal.id.clone(),
        symbol: signal.symbol.clone(),
        signal_ts: signal.ts,
        signal_type: signal.classification_v2.legacy_signal_type.clone(),
        classification_v2: serialized_key(signal.classification_v2.structure_interpretation),
        severity: format!("{:?}", signal.severity).to_ascii_lowercase(),
        impact_level: signal.impact_level.clone(),
        window_sec: signal.window_sec,
        oi_context: serialized_key(signal.classification_v2.oi_context),
        regime: signal.market_driver.market_state.clone(),
        entry_price,
        markout_30s_bps: markout(30_000),
        markout_2m_bps: markout(120_000),
        markout_5m_bps: markout(300_000),
        mfe_5m_bps: fully_evaluated
            .then(|| {
                horizon_prices
                    .iter()
                    .copied()
                    .fold(f64::NEG_INFINITY, f64::max)
            })
            .filter(|value| value.is_finite()),
        mae_5m_bps: fully_evaluated
            .then(|| horizon_prices.iter().copied().fold(f64::INFINITY, f64::min))
            .filter(|value| value.is_finite()),
        follow_through_30s: markout(30_000).map(|value| value > 0.0),
        follow_through_2m: markout(120_000).map(|value| value > 0.0),
        follow_through_5m: markout(300_000).map(|value| value > 0.0),
        evaluated_at: now_ms,
        outcome_version: CONTRACT_WHALE_OUTCOME_VERSION.to_string(),
    })
}

fn weighted_prices_by_second(
    signal: &ContractWhaleSignal,
    buckets: &[ContractFlowBucket],
    now_ms: i64,
) -> Vec<(i64, f64)> {
    let mut grouped: BTreeMap<i64, (f64, f64)> = BTreeMap::new();
    for bucket in buckets {
        if !bucket.symbol.eq_ignore_ascii_case(&signal.symbol)
            || bucket.ts_bucket < signal.ts
            || bucket.ts_bucket > now_ms.min(signal.ts.saturating_add(300_000))
        {
            continue;
        }
        let Some(vwap) = bucket
            .vwap
            .filter(|price| price.is_finite() && *price > 0.0)
        else {
            continue;
        };
        let volume = bucket.buy_volume_btc + bucket.sell_volume_btc;
        if volume <= f64::EPSILON {
            continue;
        }
        let item = grouped.entry(bucket.ts_bucket).or_default();
        item.0 += vwap * volume;
        item.1 += volume;
    }
    grouped
        .into_iter()
        .filter_map(|(ts, (weighted_price, volume))| {
            (volume > f64::EPSILON).then(|| (ts, weighted_price / volume))
        })
        .collect()
}

fn price_at_or_before(prices: &[(i64, f64)], target_ts: i64) -> Option<f64> {
    prices
        .iter()
        .rev()
        .find(|(ts, _)| *ts <= target_ts)
        .map(|(_, price)| *price)
}

fn signed_markout_bps(entry_price: f64, mark_price: f64, direction: f64) -> f64 {
    direction * ((mark_price / entry_price) - 1.0) * 10_000.0
}

fn serialized_key(value: impl serde::Serialize) -> String {
    serde_json::to_value(value)
        .ok()
        .and_then(|value| value.as_str().map(str::to_string))
        .unwrap_or_default()
}
