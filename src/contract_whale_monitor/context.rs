use std::collections::BTreeMap;

use super::types::{
    ContractExchange, ContractFundingSnapshot, ContractLiquidationBucket, ContractOiSnapshot,
};

#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum DataSource {
    Binance,
    Bitfinex,
    Okx,
    Coinbase,
}

impl DataSource {
    pub fn from_exchange(exchange: ContractExchange) -> Self {
        match exchange {
            ContractExchange::Binance => Self::Binance,
            ContractExchange::Bitfinex => Self::Bitfinex,
            ContractExchange::Okx => Self::Okx,
            ContractExchange::Coinbase => Self::Coinbase,
        }
    }

    pub fn from_key(value: &str) -> Option<Self> {
        match value.to_ascii_lowercase().as_str() {
            "binance" => Some(Self::Binance),
            "bitfinex" => Some(Self::Bitfinex),
            "okx" => Some(Self::Okx),
            "coinbase" => Some(Self::Coinbase),
            _ => None,
        }
    }
}

#[derive(Debug, Clone, Default, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ContractContextV2 {
    pub symbol: String,
    pub open_interest: Option<f64>,
    pub funding_rate: Option<f64>,
    pub liquidation_volume: Option<f64>,
    pub oi_source: Option<DataSource>,
    pub funding_source: Option<DataSource>,
    pub liquidation_source: Option<DataSource>,
    pub freshness_ms: u64,
    pub oi_stale: bool,
    pub funding_stale: bool,
    pub liquidation_stale: bool,
}

pub fn contract_context_v2_from_snapshots(
    symbol: &str,
    now_ts: i64,
    stale_after_ms: u64,
    oi_snapshots: &[ContractOiSnapshot],
    funding_snapshots: &[ContractFundingSnapshot],
    liquidation_buckets: &[ContractLiquidationBucket],
) -> ContractContextV2 {
    let symbol = normalize_symbol(symbol);
    let latest_oi = latest_oi_by_exchange(oi_snapshots, &symbol, now_ts);
    let latest_funding = latest_funding_by_exchange(funding_snapshots, &symbol, now_ts);
    let recent_liquidations =
        recent_liquidations_by_exchange(liquidation_buckets, &symbol, now_ts, stale_after_ms);

    let open_interest = latest_oi
        .values()
        .map(|snapshot| snapshot.oi_btc)
        .filter(|value| value.is_finite() && *value > 0.0)
        .sum::<f64>();
    let funding_values = latest_funding
        .values()
        .map(|snapshot| snapshot.funding_rate)
        .filter(|value| value.is_finite())
        .collect::<Vec<_>>();
    let liquidation_volume = recent_liquidations
        .values()
        .map(|bucket| bucket.long_liq_btc + bucket.short_liq_btc)
        .filter(|value| value.is_finite() && *value > 0.0)
        .sum::<f64>();

    let latest_oi_row = latest_oi.values().max_by_key(|snapshot| snapshot.ts);
    let latest_funding_row = latest_funding.values().max_by_key(|snapshot| snapshot.ts);
    let latest_liquidation_row = recent_liquidations
        .values()
        .max_by_key(|bucket| bucket.ts_bucket);

    let latest_ts = [
        latest_oi_row.map(|snapshot| snapshot.ts),
        latest_funding_row.map(|snapshot| snapshot.ts),
        latest_liquidation_row.map(|bucket| bucket.ts_bucket),
    ]
    .into_iter()
    .flatten()
    .max();
    let freshness_ms = latest_ts
        .map(|ts| now_ts.saturating_sub(ts).max(0) as u64)
        .unwrap_or(u64::MAX);

    ContractContextV2 {
        symbol,
        open_interest: (open_interest > 0.0).then_some(open_interest),
        funding_rate: if funding_values.is_empty() {
            None
        } else {
            Some(funding_values.iter().sum::<f64>() / funding_values.len() as f64)
        },
        liquidation_volume: (liquidation_volume > 0.0).then_some(liquidation_volume),
        oi_source: latest_oi_row.map(|snapshot| DataSource::from_exchange(snapshot.exchange)),
        funding_source: latest_funding_row
            .map(|snapshot| DataSource::from_exchange(snapshot.exchange)),
        liquidation_source: latest_liquidation_row
            .and_then(|bucket| DataSource::from_key(&bucket.exchange)),
        freshness_ms,
        oi_stale: latest_oi_row
            .map(|snapshot| is_stale(now_ts, snapshot.ts, stale_after_ms))
            .unwrap_or(true),
        funding_stale: latest_funding_row
            .map(|snapshot| is_stale(now_ts, snapshot.ts, stale_after_ms))
            .unwrap_or(true),
        liquidation_stale: latest_liquidation_row
            .map(|bucket| is_stale(now_ts, bucket.ts_bucket, stale_after_ms))
            .unwrap_or(true),
    }
}

fn latest_oi_by_exchange<'a>(
    snapshots: &'a [ContractOiSnapshot],
    symbol: &str,
    target_ts: i64,
) -> BTreeMap<String, &'a ContractOiSnapshot> {
    let mut latest = BTreeMap::new();
    for snapshot in snapshots {
        if !snapshot.symbol.eq_ignore_ascii_case(symbol) || snapshot.ts > target_ts {
            continue;
        }
        let key = snapshot.exchange.as_key().to_string();
        if latest
            .get(&key)
            .is_none_or(|existing: &&ContractOiSnapshot| snapshot.ts > existing.ts)
        {
            latest.insert(key, snapshot);
        }
    }
    latest
}

fn latest_funding_by_exchange<'a>(
    snapshots: &'a [ContractFundingSnapshot],
    symbol: &str,
    target_ts: i64,
) -> BTreeMap<String, &'a ContractFundingSnapshot> {
    let mut latest = BTreeMap::new();
    for snapshot in snapshots {
        if !snapshot.symbol.eq_ignore_ascii_case(symbol) || snapshot.ts > target_ts {
            continue;
        }
        let key = snapshot.exchange.as_key().to_string();
        if latest
            .get(&key)
            .is_none_or(|existing: &&ContractFundingSnapshot| snapshot.ts > existing.ts)
        {
            latest.insert(key, snapshot);
        }
    }
    latest
}

fn recent_liquidations_by_exchange<'a>(
    buckets: &'a [ContractLiquidationBucket],
    symbol: &str,
    target_ts: i64,
    stale_after_ms: u64,
) -> BTreeMap<String, &'a ContractLiquidationBucket> {
    let start_ts = target_ts.saturating_sub(stale_after_ms as i64);
    let mut latest = BTreeMap::new();
    for bucket in buckets {
        if !bucket.symbol.eq_ignore_ascii_case(symbol)
            || bucket.ts_bucket > target_ts
            || bucket.ts_bucket < start_ts
        {
            continue;
        }
        if latest
            .get(&bucket.exchange)
            .is_none_or(|existing: &&ContractLiquidationBucket| {
                bucket.ts_bucket > existing.ts_bucket
            })
        {
            latest.insert(bucket.exchange.clone(), bucket);
        }
    }
    latest
}

fn is_stale(now_ts: i64, data_ts: i64, stale_after_ms: u64) -> bool {
    now_ts.saturating_sub(data_ts).max(0) as u64 > stale_after_ms
}

fn normalize_symbol(symbol: &str) -> String {
    symbol
        .trim()
        .split(['-', '_', '/', ':'])
        .next()
        .unwrap_or(symbol)
        .trim_end_matches("USDT")
        .trim_end_matches("USD")
        .to_ascii_uppercase()
}
