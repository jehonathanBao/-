use std::{collections::BTreeMap, collections::VecDeque};

use super::{
    config::BinanceAltContractRuntimeConfig,
    context::context_data_quality_penalty,
    types::{
        AltContractContext, AltContractDirection, AltContractExchangeContribution,
        AltContractSymbolMeta, AltContractTrade, AltContractTradeSide, AltContractTrend60s,
        AltContractWindowStats,
    },
};

pub fn rolling_window_stats(
    trades: &VecDeque<AltContractTrade>,
    meta: &AltContractSymbolMeta,
    window_sec: u64,
    now: i64,
    context: &AltContractContext,
    booted_at_ms: i64,
    config: &BinanceAltContractRuntimeConfig,
) -> Option<AltContractWindowStats> {
    let window_ms = i64::try_from(window_sec).ok()?.saturating_mul(1000);
    let start = now.saturating_sub(window_ms);
    let mut window_trades = trades
        .iter()
        .filter(|trade| trade.product_id == meta.product_id && trade.ts >= start && trade.ts <= now)
        .cloned()
        .collect::<Vec<_>>();
    if window_trades.is_empty() {
        return None;
    }
    window_trades.sort_by_key(|trade| trade.ts);
    Some(stats_from_trades(
        &window_trades,
        trades,
        meta,
        window_sec,
        now,
        context,
        booted_at_ms,
        config,
    ))
}

pub fn stats_from_trades(
    window_trades: &[AltContractTrade],
    all_trades: &VecDeque<AltContractTrade>,
    meta: &AltContractSymbolMeta,
    window_sec: u64,
    now: i64,
    context: &AltContractContext,
    booted_at_ms: i64,
    config: &BinanceAltContractRuntimeConfig,
) -> AltContractWindowStats {
    let exchanges = exchange_contributions(window_trades);
    let buy_volume_base = exchanges
        .iter()
        .map(|item| item.buy_volume_base)
        .sum::<f64>();
    let sell_volume_base = exchanges
        .iter()
        .map(|item| item.sell_volume_base)
        .sum::<f64>();
    let total_volume_base = buy_volume_base + sell_volume_base;
    let net_volume_base = buy_volume_base - sell_volume_base;
    let total_notional_usd = exchanges
        .iter()
        .map(|item| item.total_notional_usd)
        .sum::<f64>();
    let dominance = if total_volume_base > 0.0 {
        net_volume_base.abs() / total_volume_base
    } else {
        0.0
    };
    let direction = if net_volume_base > 0.0 {
        AltContractDirection::Buy
    } else if net_volume_base < 0.0 {
        AltContractDirection::Sell
    } else {
        AltContractDirection::Neutral
    };
    let first_price = window_trades.first().map(|trade| trade.price);
    let last_price = window_trades.last().map(|trade| trade.price);
    let price_move_pct = first_price
        .zip(last_price)
        .filter(|(first, _)| *first > 0.0)
        .map(|(first, last)| (last / first - 1.0) * 100.0);
    let main_exchange = exchanges
        .iter()
        .max_by(|left, right| left.total_volume_base.total_cmp(&right.total_volume_base))
        .map(|item| item.exchange.clone());
    let dynamic_multiple = if config.dynamic.enabled {
        dynamic_multiple(
            all_trades,
            &meta.product_id,
            window_sec,
            now,
            total_notional_usd,
            config.dynamic.min_samples,
        )
    } else {
        None
    };
    let startup_age_ms = Some(now.saturating_sub(booted_at_ms));
    let mut data_quality = 100_u8;
    if startup_age_ms.is_some_and(|age| age < config.data_quality.warmup_ms) {
        data_quality = data_quality.saturating_sub(20);
    }
    data_quality = data_quality.saturating_sub(context_data_quality_penalty(context));
    AltContractWindowStats {
        symbol: meta.symbol.clone(),
        product_id: meta.product_id.clone(),
        tier: meta.tier,
        window_sec,
        ts: now,
        buy_volume_base,
        sell_volume_base,
        total_volume_base,
        net_volume_base,
        total_notional_usd,
        dominance,
        direction,
        trigger_price_usd: if total_volume_base > 0.0 {
            Some(total_notional_usd / total_volume_base)
        } else {
            last_price
        },
        price_move_pct,
        exchange_count: exchanges.len(),
        main_exchange,
        exchanges,
        dynamic_multiple,
        data_quality,
        startup_age_ms,
    }
}

pub fn trend_for_symbol(
    trades: &VecDeque<AltContractTrade>,
    product_id: &str,
    now: i64,
) -> AltContractTrend60s {
    let start = now.saturating_sub(60_000);
    let mut trend = AltContractTrend60s::default();
    for trade in trades
        .iter()
        .filter(|trade| trade.product_id == product_id && trade.ts >= start && trade.ts <= now)
    {
        match trade.side {
            AltContractTradeSide::Buy => trend.buy_volume_base += trade.qty_base,
            AltContractTradeSide::Sell => trend.sell_volume_base += trade.qty_base,
        }
        trend.total_notional_usd += trade.notional_usd;
        trend.updated_at_ms = Some(trade.ts);
    }
    trend.total_volume_base = trend.buy_volume_base + trend.sell_volume_base;
    trend.net_volume_base = trend.buy_volume_base - trend.sell_volume_base;
    if trend.total_volume_base > 0.0 {
        trend.dominance = trend.net_volume_base.abs() / trend.total_volume_base;
        trend.buy_ratio = trend.buy_volume_base / trend.total_volume_base;
        trend.sell_ratio = trend.sell_volume_base / trend.total_volume_base;
    }
    trend
}

fn exchange_contributions(trades: &[AltContractTrade]) -> Vec<AltContractExchangeContribution> {
    let mut by_exchange: BTreeMap<String, AltContractExchangeContribution> = BTreeMap::new();
    for trade in trades {
        let entry = by_exchange
            .entry(trade.exchange.as_key().to_string())
            .or_insert_with(|| AltContractExchangeContribution {
                exchange: trade.exchange.as_key().to_string(),
                ..AltContractExchangeContribution::default()
            });
        match trade.side {
            AltContractTradeSide::Buy => {
                entry.buy_volume_base += trade.qty_base;
                entry.buy_notional_usd += trade.notional_usd;
            }
            AltContractTradeSide::Sell => {
                entry.sell_volume_base += trade.qty_base;
                entry.sell_notional_usd += trade.notional_usd;
            }
        }
        entry.trade_count = entry.trade_count.saturating_add(1);
    }
    by_exchange
        .into_values()
        .map(|mut item| {
            item.total_volume_base = item.buy_volume_base + item.sell_volume_base;
            item.total_notional_usd = item.buy_notional_usd + item.sell_notional_usd;
            item.net_volume_base = item.buy_volume_base - item.sell_volume_base;
            item.dominance = if item.total_volume_base > 0.0 {
                item.net_volume_base.abs() / item.total_volume_base
            } else {
                0.0
            };
            item
        })
        .collect()
}

fn dynamic_multiple(
    trades: &VecDeque<AltContractTrade>,
    product_id: &str,
    window_sec: u64,
    now: i64,
    current_notional_usd: f64,
    min_samples: usize,
) -> Option<f64> {
    let window_ms = i64::try_from(window_sec).ok()?.saturating_mul(1000);
    let lookback_start = now.saturating_sub(3_600_000);
    let current_start = now.saturating_sub(window_ms);
    let mut buckets: BTreeMap<i64, f64> = BTreeMap::new();
    for trade in trades.iter().filter(|trade| {
        trade.product_id == product_id && trade.ts >= lookback_start && trade.ts < current_start
    }) {
        let bucket = (trade.ts - lookback_start) / window_ms;
        *buckets.entry(bucket).or_insert(0.0) += trade.notional_usd;
    }
    if buckets.len() < min_samples {
        return None;
    }
    let average = buckets.values().sum::<f64>() / buckets.len() as f64;
    (average > 0.0).then_some(current_notional_usd / average)
}
