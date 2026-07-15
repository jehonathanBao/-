use std::collections::BTreeMap;

use super::types::{ContractFlowBucket, ContractWhaleActiveFlowDirection, ContractWhaleSignal};

pub const CONTRACT_WHALE_OUTCOME_VERSION: &str = "v2_volatility_shadow";

const HORIZON_PRICE_FRESHNESS_MS: i64 = 5_000;
const HISTORICAL_L2_UNAVAILABLE: &str = "historical_l2_unavailable";

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
    #[serde(default)]
    pub absolute_return_30s_bps: Option<f64>,
    #[serde(default)]
    pub absolute_return_2m_bps: Option<f64>,
    #[serde(default)]
    pub absolute_return_5m_bps: Option<f64>,
    #[serde(default)]
    pub realized_volatility_5m_bps: Option<f64>,
    #[serde(default)]
    pub max_absolute_excursion_5m_bps: Option<f64>,
    #[serde(default)]
    pub price_sample_count_5m: Option<u64>,
    #[serde(default)]
    pub liquidity_recovered_5m: Option<bool>,
    #[serde(default)]
    pub liquidity_recovery_ms: Option<i64>,
    #[serde(default)]
    pub liquidity_recovery_reason: Option<String>,
    #[serde(default)]
    pub setup_outcome: Option<String>,
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
        ContractWhaleActiveFlowDirection::BuyDominant => Some(1.0),
        ContractWhaleActiveFlowDirection::SellDominant => Some(-1.0),
        ContractWhaleActiveFlowDirection::Balanced | ContractWhaleActiveFlowDirection::Unknown => {
            None
        }
    };
    let prices = weighted_prices_by_second(signal, buckets, now_ms);
    let entry_price = signal
        .order_price_usd
        .filter(|value| value.is_finite() && *value > 0.0)
        .or_else(|| fresh_price_near_horizon(&prices, signal.ts, now_ms))?;
    let horizon_price = |horizon_ms: i64| {
        let target_ts = signal.ts.saturating_add(horizon_ms);
        (now_ms >= target_ts)
            .then(|| fresh_price_near_horizon(&prices, target_ts, now_ms))
            .flatten()
    };
    let price_30s = horizon_price(30_000);
    let price_2m = horizon_price(120_000);
    let price_5m = horizon_price(300_000);
    let signed_markout = |price: Option<f64>| {
        direction.and_then(|direction| {
            price.map(|price| signed_markout_bps(entry_price, price, direction))
        })
    };
    let markout_30s_bps = signed_markout(price_30s);
    let markout_2m_bps = signed_markout(price_2m);
    let markout_5m_bps = signed_markout(price_5m);
    let fully_evaluated = now_ms >= signal.ts.saturating_add(300_000);
    let complete_5m_path = fully_evaluated && price_5m.is_some();
    let path_markouts = complete_5m_path.then(|| {
        prices
            .iter()
            .map(|(_, price)| ((*price / entry_price) - 1.0) * 10_000.0)
            .collect::<Vec<_>>()
    });
    let price_sample_count_5m = path_markouts
        .as_ref()
        .map(|_| u64::try_from(prices.len()).unwrap_or(u64::MAX));
    let realized_volatility_5m_bps = complete_5m_path
        .then(|| realized_volatility_bps(&prices))
        .flatten();
    let max_absolute_excursion_5m_bps = path_markouts.as_ref().and_then(|markouts| {
        markouts
            .iter()
            .map(|value| value.abs())
            .reduce(f64::max)
            .filter(|value| value.is_finite())
    });
    let setup_outcome = complete_5m_path.then_some(match markout_5m_bps {
        Some(value) if value > 0.0 => "continuation",
        Some(value) if value < 0.0 => "reversal",
        Some(_) | None => "unclear",
    });
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
        markout_30s_bps,
        markout_2m_bps,
        markout_5m_bps,
        mfe_5m_bps: (complete_5m_path && direction.is_some())
            .then(|| {
                path_markouts
                    .as_ref()
                    .into_iter()
                    .flatten()
                    .map(|value| direction.unwrap_or_default() * value)
                    .fold(f64::NEG_INFINITY, f64::max)
            })
            .filter(|value| value.is_finite()),
        mae_5m_bps: (complete_5m_path && direction.is_some())
            .then(|| {
                path_markouts
                    .as_ref()
                    .into_iter()
                    .flatten()
                    .map(|value| direction.unwrap_or_default() * value)
                    .fold(f64::INFINITY, f64::min)
            })
            .filter(|value| value.is_finite()),
        absolute_return_30s_bps: price_30s.map(|price| absolute_return_bps(entry_price, price)),
        absolute_return_2m_bps: price_2m.map(|price| absolute_return_bps(entry_price, price)),
        absolute_return_5m_bps: price_5m.map(|price| absolute_return_bps(entry_price, price)),
        realized_volatility_5m_bps,
        max_absolute_excursion_5m_bps,
        price_sample_count_5m,
        liquidity_recovered_5m: None,
        liquidity_recovery_ms: None,
        liquidity_recovery_reason: Some(HISTORICAL_L2_UNAVAILABLE.to_string()),
        setup_outcome: setup_outcome.map(str::to_string),
        follow_through_30s: markout_30s_bps.map(|value| value > 0.0),
        follow_through_2m: markout_2m_bps.map(|value| value > 0.0),
        follow_through_5m: markout_5m_bps.map(|value| value > 0.0),
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
        .filter(|(_, (_, volume))| *volume > f64::EPSILON)
        .map(|(ts, (weighted_price, volume))| (ts, weighted_price / volume))
        .collect()
}

fn fresh_price_near_horizon(prices: &[(i64, f64)], target_ts: i64, now_ms: i64) -> Option<f64> {
    prices
        .iter()
        .filter(|(ts, _)| {
            *ts <= now_ms && (*ts).abs_diff(target_ts) <= HORIZON_PRICE_FRESHNESS_MS as u64
        })
        .min_by_key(|(ts, _)| {
            (
                (*ts).abs_diff(target_ts),
                if *ts >= target_ts { 0_u8 } else { 1_u8 },
            )
        })
        .map(|(_, price)| *price)
}

fn absolute_return_bps(entry_price: f64, mark_price: f64) -> f64 {
    (((mark_price / entry_price) - 1.0) * 10_000.0).abs()
}

fn signed_markout_bps(entry_price: f64, mark_price: f64, direction: f64) -> f64 {
    direction * ((mark_price / entry_price) - 1.0) * 10_000.0
}

fn realized_volatility_bps(prices: &[(i64, f64)]) -> Option<f64> {
    let mut squared_log_returns = 0.0;
    let mut return_count = 0_u64;
    for pair in prices.windows(2) {
        let previous = pair[0].1;
        let current = pair[1].1;
        if previous <= 0.0 || current <= 0.0 {
            continue;
        }
        let log_return = (current / previous).ln();
        if !log_return.is_finite() {
            continue;
        }
        squared_log_returns += log_return * log_return;
        return_count = return_count.saturating_add(1);
    }
    (return_count > 0)
        .then(|| squared_log_returns.sqrt() * 10_000.0)
        .filter(|value| value.is_finite())
}

fn serialized_key(value: impl serde::Serialize) -> String {
    serde_json::to_value(value)
        .ok()
        .and_then(|value| value.as_str().map(str::to_string))
        .unwrap_or_default()
}
