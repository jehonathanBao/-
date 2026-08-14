use std::collections::BTreeMap;

use super::{
    config::{contract_whale_runtime_config, ContractWhaleRuntimeConfig},
    types::{
        ContractFlowBucket, ContractFundingSnapshot, ContractLiquidationBucket,
        ContractLiquidationOrder, ContractLiquidationSide, ContractOiSnapshot, ContractTrade,
        ContractTradeSide, ContractWhaleLiquidationContext, ContractWhaleMarketContext,
        ContractWhaleMarketType, ContractWhaleMicroVolatility, ContractWhalePercentileThreshold,
        ContractWhaleWindowStats, ExchangeFlowContribution,
    },
};

pub struct RollingWindowStatsOptions<'a> {
    pub price_move_pct: Option<f64>,
    pub dynamic_multiple: Option<f64>,
    pub dynamic_baseline_btc: Option<f64>,
    pub dynamic_threshold_level: String,
    pub data_quality: u8,
    pub config: &'a ContractWhaleRuntimeConfig,
}

pub fn aggregate_1s_buckets(trades: &[ContractTrade]) -> Vec<ContractFlowBucket> {
    // Keep one coherent runtime-config view for the whole input batch. Besides
    // avoiding per-trade lock/clone overhead, this prevents a single bucket
    // batch from mixing source roles if configuration changes mid-aggregation.
    let config = contract_whale_runtime_config();
    let mut buckets: BTreeMap<(i64, String, String), ContractFlowBucket> = BTreeMap::new();
    for trade in trades {
        let ts_bucket = trade.ts - (trade.ts % 1000);
        let exchange = trade.exchange.as_key().to_string();
        let key = (ts_bucket, exchange.clone(), trade.symbol.clone());
        let source_role = config
            .exchange_platform(&exchange)
            .map(|platform| platform.market_role(ContractWhaleMarketType::Perp))
            .unwrap_or_default();
        let bucket = buckets.entry(key).or_insert_with(|| ContractFlowBucket {
            ts_bucket,
            exchange,
            symbol: trade.symbol.clone(),
            market_type: ContractWhaleMarketType::Perp,
            source_role,
            ..ContractFlowBucket::default()
        });
        match trade.side {
            ContractTradeSide::Buy => {
                bucket.buy_volume_btc += trade.qty_btc;
                bucket.buy_notional_usd += trade.notional_usd;
                bucket.buy_trade_count = bucket
                    .buy_trade_count
                    .saturating_add(trade.raw_trade_count.unwrap_or(1));
            }
            ContractTradeSide::Sell => {
                bucket.sell_volume_btc += trade.qty_btc;
                bucket.sell_notional_usd += trade.notional_usd;
                bucket.sell_trade_count = bucket
                    .sell_trade_count
                    .saturating_add(trade.raw_trade_count.unwrap_or(1));
            }
        }
        bucket.trade_count += trade.raw_trade_count.unwrap_or(1);
        bucket.max_single_trade_btc = bucket.max_single_trade_btc.max(trade.qty_btc);
        let total_volume = bucket.buy_volume_btc + bucket.sell_volume_btc;
        bucket.max_single_trade_share = if total_volume > f64::EPSILON {
            (bucket.max_single_trade_btc / total_volume).clamp(0.0, 1.0)
        } else {
            0.0
        };
        bucket.vwap = if total_volume > 0.0 {
            Some((bucket.buy_notional_usd + bucket.sell_notional_usd) / total_volume)
        } else {
            None
        };
    }
    buckets.into_values().collect()
}

pub fn aggregate_liquidation_1s_buckets(
    liquidations: &[ContractLiquidationOrder],
) -> Vec<ContractLiquidationBucket> {
    let mut buckets: BTreeMap<(i64, String, String), ContractLiquidationBucket> = BTreeMap::new();
    for liquidation in liquidations {
        let ts_bucket = liquidation.ts - (liquidation.ts % 1000);
        let exchange = liquidation.exchange.as_key().to_string();
        let key = (ts_bucket, exchange.clone(), liquidation.symbol.clone());
        let bucket = buckets
            .entry(key)
            .or_insert_with(|| ContractLiquidationBucket {
                ts_bucket,
                exchange,
                symbol: liquidation.symbol.clone(),
                ..ContractLiquidationBucket::default()
            });
        match liquidation.side {
            ContractLiquidationSide::Long => bucket.long_liq_btc += liquidation.qty_btc,
            ContractLiquidationSide::Short => bucket.short_liq_btc += liquidation.qty_btc,
        }
        bucket.liq_notional_usd += liquidation.notional_usd;
        bucket.order_count += 1;
        bucket.max_single_liq_btc = bucket.max_single_liq_btc.max(liquidation.qty_btc);
        let total_liq = bucket.long_liq_btc + bucket.short_liq_btc;
        bucket.vwap = if total_liq > 0.0 {
            Some(bucket.liq_notional_usd / total_liq)
        } else {
            None
        };
    }
    buckets.into_values().collect()
}

pub fn rolling_window_stats(
    buckets: &[ContractFlowBucket],
    symbol: &str,
    window_sec: u64,
    now_ts: i64,
    price_move_pct: Option<f64>,
    dynamic_multiple: Option<f64>,
    data_quality: u8,
) -> Option<ContractWhaleWindowStats> {
    let config = contract_whale_runtime_config();
    rolling_window_stats_with_config(
        buckets,
        symbol,
        window_sec,
        now_ts,
        RollingWindowStatsOptions {
            price_move_pct,
            dynamic_multiple,
            dynamic_baseline_btc: None,
            dynamic_threshold_level: String::new(),
            data_quality,
            config: &config,
        },
    )
}

pub fn rolling_window_stats_with_config(
    buckets: &[ContractFlowBucket],
    symbol: &str,
    window_sec: u64,
    now_ts: i64,
    options: RollingWindowStatsOptions<'_>,
) -> Option<ContractWhaleWindowStats> {
    let config = options.config;
    let window_ms = (window_sec as i64).saturating_mul(1000);
    let start_ts = now_ts.saturating_sub(window_ms);
    let mut by_exchange: BTreeMap<String, ExchangeFlowContribution> = BTreeMap::new();
    for bucket in buckets {
        if !bucket.symbol.eq_ignore_ascii_case(symbol)
            || bucket.ts_bucket < start_ts
            || !config.exchange_enabled(&bucket.exchange)
        {
            continue;
        }
        let contribution = by_exchange
            .entry(bucket.exchange.clone())
            .or_insert_with(|| ExchangeFlowContribution {
                exchange: bucket.exchange.clone(),
                ..ExchangeFlowContribution::default()
            });
        contribution.buy_volume_btc += bucket.buy_volume_btc;
        contribution.sell_volume_btc += bucket.sell_volume_btc;
        contribution.buy_notional_usd += bucket.buy_notional_usd;
        contribution.sell_notional_usd += bucket.sell_notional_usd;
        contribution.trade_count += bucket.trade_count;
    }

    if by_exchange.is_empty() {
        return None;
    }

    let buy_volume_btc = by_exchange
        .values()
        .map(|item| item.buy_volume_btc)
        .sum::<f64>();
    let sell_volume_btc = by_exchange
        .values()
        .map(|item| item.sell_volume_btc)
        .sum::<f64>();
    let net_volume_btc = buy_volume_btc - sell_volume_btc;
    let mut exchanges: Vec<ExchangeFlowContribution> = by_exchange
        .into_values()
        .map(|mut contribution| {
            contribution.total_volume_btc =
                contribution.buy_volume_btc + contribution.sell_volume_btc;
            contribution.buy_share =
                share(contribution.buy_volume_btc, contribution.total_volume_btc);
            contribution.sell_share =
                share(contribution.sell_volume_btc, contribution.total_volume_btc);
            contribution.total_notional_usd =
                contribution.buy_notional_usd + contribution.sell_notional_usd;
            contribution.net_volume_btc =
                contribution.buy_volume_btc - contribution.sell_volume_btc;
            contribution.dominance = dominance(
                contribution.net_volume_btc.abs(),
                contribution.total_volume_btc,
            );
            contribution
        })
        .collect();
    apply_net_contribution_shares(&mut exchanges, net_volume_btc);
    exchanges.sort_by(|left, right| {
        right
            .total_volume_btc
            .partial_cmp(&left.total_volume_btc)
            .unwrap_or(std::cmp::Ordering::Equal)
    });

    let buy_notional_usd = exchanges
        .iter()
        .map(|item| item.buy_notional_usd)
        .sum::<f64>();
    let sell_notional_usd = exchanges
        .iter()
        .map(|item| item.sell_notional_usd)
        .sum::<f64>();
    let total_volume_btc = buy_volume_btc + sell_volume_btc;
    let dominance = if total_volume_btc > 0.0 {
        net_volume_btc.abs() / total_volume_btc
    } else {
        0.0
    };
    let dominant_venue_net_contribution_share = dominant_venue_net_contribution_share(&exchanges);
    let exchange_count = exchanges
        .iter()
        .filter(|item| item.total_volume_btc > 0.0)
        .count();
    let main_exchange = exchanges.first().map(|item| item.exchange.clone());

    Some(ContractWhaleWindowStats {
        symbol: symbol.to_string(),
        window_sec,
        ts: now_ts,
        buy_volume_btc,
        sell_volume_btc,
        total_volume_btc,
        net_volume_btc,
        dominance,
        buy_notional_usd,
        sell_notional_usd,
        total_notional_usd: buy_notional_usd + sell_notional_usd,
        price_move_pct: options.price_move_pct,
        exchange_count,
        main_exchange,
        exchanges,
        dominant_venue_net_contribution_share,
        dynamic_multiple: options.dynamic_multiple,
        dynamic_baseline_btc: options.dynamic_baseline_btc,
        dynamic_threshold_level: options.dynamic_threshold_level,
        percentile_level: None,
        multi_exchange_confirmed: false,
        liquidation_context: ContractWhaleLiquidationContext::default(),
        market_context: ContractWhaleMarketContext::default(),
        price_reversal_ratio: None,
        data_quality: options.data_quality,
        ws_latency_ms: None,
        startup_age_ms: None,
        liquidation_driven: false,
        price_jump_anomaly: false,
        micro_volatility: micro_volatility_from_buckets(buckets, symbol, now_ts, config),
    })
}

fn micro_volatility_from_buckets(
    buckets: &[ContractFlowBucket],
    symbol: &str,
    now_ts: i64,
    config: &ContractWhaleRuntimeConfig,
) -> ContractWhaleMicroVolatility {
    let settings = &config.classification;
    if !settings.micro_volatility_enabled {
        return ContractWhaleMicroVolatility {
            source: "disabled".to_string(),
            stale: true,
            ..ContractWhaleMicroVolatility::default()
        };
    }

    let from_ts = now_ts.saturating_sub(300_000);
    let mut by_second: BTreeMap<i64, (f64, f64)> = BTreeMap::new();
    for bucket in buckets {
        if !bucket.symbol.eq_ignore_ascii_case(symbol)
            || bucket.ts_bucket < from_ts
            || bucket.ts_bucket > now_ts
            || !config.exchange_enabled(&bucket.exchange)
        {
            continue;
        }
        let Some(vwap) = bucket
            .vwap
            .filter(|value| value.is_finite() && *value > 0.0)
        else {
            continue;
        };
        let volume = bucket.buy_volume_btc + bucket.sell_volume_btc;
        if volume <= f64::EPSILON {
            continue;
        }
        let entry = by_second.entry(bucket.ts_bucket).or_default();
        entry.0 += vwap * volume;
        entry.1 += volume;
    }

    let prices = by_second
        .iter()
        .filter_map(|(ts, (weighted_price, volume))| {
            (*volume > f64::EPSILON).then(|| (*ts, weighted_price / volume))
        })
        .collect::<Vec<_>>();
    let sample_count = prices.len().saturating_sub(1);
    let stale = prices
        .last()
        .map(|(ts, _)| {
            now_ts.saturating_sub(*ts) > settings.micro_volatility_max_staleness_seconds * 1_000
        })
        .unwrap_or(true);
    if sample_count < settings.micro_volatility_min_samples || stale {
        return ContractWhaleMicroVolatility {
            sample_count,
            source: "fallback".to_string(),
            stale,
            ..ContractWhaleMicroVolatility::default()
        };
    }

    let mut ewma = None;
    for pair in prices.windows(2) {
        let previous = pair[0].1;
        let current = pair[1].1;
        if previous <= f64::EPSILON || !previous.is_finite() || !current.is_finite() {
            continue;
        }
        let absolute_return_pct = ((current / previous) - 1.0).abs() * 100.0;
        ewma = Some(match ewma {
            Some(value) => {
                settings.micro_volatility_ewma_alpha * absolute_return_pct
                    + (1.0 - settings.micro_volatility_ewma_alpha) * value
            }
            None => absolute_return_pct,
        });
    }
    ContractWhaleMicroVolatility {
        value_pct: ewma.filter(|value| value.is_finite()),
        sample_count,
        source: "flow_1s_vwap".to_string(),
        stale: false,
    }
}

pub fn liquidation_context_for_window(
    buckets: &[ContractLiquidationBucket],
    symbol: &str,
    window_sec: u64,
    now_ts: i64,
    total_volume_btc: f64,
) -> ContractWhaleLiquidationContext {
    let config = contract_whale_runtime_config();
    let window_ms = (window_sec as i64).saturating_mul(1000);
    let start_ts = now_ts.saturating_sub(window_ms);
    let mut context = ContractWhaleLiquidationContext::default();
    for bucket in buckets {
        if !bucket.symbol.eq_ignore_ascii_case(symbol)
            || bucket.ts_bucket < start_ts
            || !config.exchange_enabled(&bucket.exchange)
        {
            continue;
        }
        context.long_liq_btc += bucket.long_liq_btc;
        context.short_liq_btc += bucket.short_liq_btc;
        context.liq_notional_usd += bucket.liq_notional_usd;
    }
    context.total_liq_btc = context.long_liq_btc + context.short_liq_btc;
    context.liq_to_volume_ratio = if total_volume_btc > f64::EPSILON {
        Some(context.total_liq_btc / total_volume_btc)
    } else {
        None
    };
    context
}

pub fn market_context_from_snapshots(
    oi_snapshots: &[ContractOiSnapshot],
    funding_snapshots: &[ContractFundingSnapshot],
    symbol: &str,
    now_ts: i64,
) -> ContractWhaleMarketContext {
    let max_gap_ms = contract_whale_runtime_config()
        .classification
        .oi_lookup_max_gap_seconds
        .max(1)
        .saturating_mul(1_000);
    let (latest_oi, latest_oi_stale) =
        sum_latest_oi_before(oi_snapshots, symbol, now_ts, max_gap_ms);
    let (prior_1m_oi, _) = sum_latest_oi_before(
        oi_snapshots,
        symbol,
        now_ts.saturating_sub(60_000),
        max_gap_ms,
    );
    let (prior_5m_oi, _) = sum_latest_oi_before(
        oi_snapshots,
        symbol,
        now_ts.saturating_sub(300_000),
        max_gap_ms,
    );
    let oi_change_1m_btc = option_diff(latest_oi, prior_1m_oi);
    let oi_change_5m_btc = option_diff(latest_oi, prior_5m_oi);
    let oi_change_pct = match (oi_change_5m_btc, prior_5m_oi) {
        (Some(change), Some(prior)) if prior.abs() > f64::EPSILON => Some(change / prior * 100.0),
        _ => None,
    };
    let (latest_funding, latest_funding_stale) =
        average_latest_funding_before(funding_snapshots, symbol, now_ts, max_gap_ms);
    let symbol_oi_snapshots = oi_snapshots
        .iter()
        .filter(|snapshot| snapshot.symbol.eq_ignore_ascii_case(symbol))
        .collect::<Vec<_>>();
    let ct_val_available = symbol_oi_snapshots
        .iter()
        .all(|snapshot| snapshot.ct_val_available);
    let metadata_reason = symbol_oi_snapshots
        .iter()
        .find_map(|snapshot| snapshot.evidence_degraded_reason.clone());
    let oi_reason = evidence_reason(latest_oi.is_some(), latest_oi_stale);
    let funding_reason = evidence_reason(latest_funding.is_some(), latest_funding_stale);
    let evidence_reason = metadata_reason.or_else(|| {
        [oi_reason.as_deref(), funding_reason.as_deref()]
            .into_iter()
            .flatten()
            .find(|reason| *reason != "available")
            .map(str::to_string)
    });

    ContractWhaleMarketContext {
        context_expected: true,
        ct_val_available,
        evidence_degraded: evidence_reason.is_some()
            || !ct_val_available
            || latest_oi.is_none()
            || latest_funding.is_none(),
        evidence_reason,
        oi_available: latest_oi.is_some(),
        oi_reason,
        funding_available: latest_funding.is_some(),
        funding_reason,
        oi_change_1m_btc,
        oi_change_5m_btc,
        oi_change_pct,
        oi_bias: Some(oi_bias(oi_change_pct).to_string()),
        funding_rate: latest_funding,
        funding_bias: Some(funding_bias(latest_funding).to_string()),
    }
}

pub fn historical_window_average_btc(
    buckets: &[ContractFlowBucket],
    symbol: &str,
    window_sec: u64,
    from_ts: i64,
    to_ts: i64,
) -> Option<f64> {
    let samples = window_volume_samples(buckets, symbol, "all", window_sec, from_ts, to_ts);
    if samples.is_empty() {
        return None;
    }
    Some(samples.iter().sum::<f64>() / samples.len() as f64)
}

pub fn historical_window_average_btc_with_min_samples(
    buckets: &[ContractFlowBucket],
    symbol: &str,
    window_sec: u64,
    from_ts: i64,
    to_ts: i64,
    min_samples: usize,
) -> Option<f64> {
    let samples = window_volume_samples(buckets, symbol, "all", window_sec, from_ts, to_ts);
    if samples.len() < min_samples {
        return None;
    }
    Some(samples.iter().sum::<f64>() / samples.len() as f64)
}

pub fn dynamic_multiple_for_volume(
    current_total_btc: f64,
    average_total_btc: Option<f64>,
) -> Option<f64> {
    let average = average_total_btc?;
    if current_total_btc <= 0.0 || average <= f64::EPSILON {
        return None;
    }
    Some(current_total_btc / average)
}

pub fn compute_percentile_threshold(
    buckets: &[ContractFlowBucket],
    symbol: &str,
    exchange: &str,
    window_sec: u64,
    from_ts: i64,
    to_ts: i64,
    computed_at: i64,
) -> Option<ContractWhalePercentileThreshold> {
    let mut samples = window_volume_samples(buckets, symbol, exchange, window_sec, from_ts, to_ts);
    if samples.is_empty() {
        return None;
    }
    samples.sort_by(|left, right| left.partial_cmp(right).unwrap_or(std::cmp::Ordering::Equal));
    Some(ContractWhalePercentileThreshold {
        computed_at,
        symbol: symbol.to_ascii_uppercase(),
        exchange: exchange.to_ascii_lowercase(),
        window_sec,
        p99_0_btc: percentile_nearest_rank(&samples, 0.990),
        p99_5_btc: percentile_nearest_rank(&samples, 0.995),
        p99_9_btc: percentile_nearest_rank(&samples, 0.999),
        sample_count: samples.len(),
    })
}

pub fn percentile_level_for_volume(
    current_total_btc: f64,
    threshold: Option<&ContractWhalePercentileThreshold>,
) -> Option<f64> {
    let threshold = threshold?;
    if current_total_btc >= threshold.p99_9_btc {
        Some(99.9)
    } else if current_total_btc >= threshold.p99_5_btc {
        Some(99.5)
    } else if current_total_btc >= threshold.p99_0_btc {
        Some(99.0)
    } else {
        None
    }
}

fn option_diff(current: Option<f64>, prior: Option<f64>) -> Option<f64> {
    Some(current? - prior?)
}

fn sum_latest_oi_before(
    snapshots: &[ContractOiSnapshot],
    symbol: &str,
    target_ts: i64,
    max_gap_ms: i64,
) -> (Option<f64>, bool) {
    let mut latest_by_exchange: BTreeMap<String, &ContractOiSnapshot> = BTreeMap::new();
    let mut matching_before_target = false;
    for snapshot in snapshots {
        if !snapshot.symbol.eq_ignore_ascii_case(symbol)
            || snapshot.ts > target_ts
            || !contract_whale_runtime_config().exchange_enabled(snapshot.exchange.as_key())
        {
            continue;
        }
        matching_before_target = true;
        if target_ts.saturating_sub(snapshot.ts) > max_gap_ms {
            continue;
        }
        let key = snapshot.exchange.as_key().to_string();
        let replace = latest_by_exchange
            .get(&key)
            .is_none_or(|existing| snapshot.ts > existing.ts);
        if replace {
            latest_by_exchange.insert(key, snapshot);
        }
    }
    let sum = latest_by_exchange
        .values()
        .map(|snapshot| snapshot.oi_btc)
        .filter(|value| value.is_finite() && *value > 0.0)
        .sum::<f64>();
    let value = (sum > 0.0).then_some(sum);
    (value, matching_before_target && value.is_none())
}

fn average_latest_funding_before(
    snapshots: &[ContractFundingSnapshot],
    symbol: &str,
    target_ts: i64,
    max_gap_ms: i64,
) -> (Option<f64>, bool) {
    let mut latest_by_exchange: BTreeMap<String, &ContractFundingSnapshot> = BTreeMap::new();
    let mut matching_before_target = false;
    for snapshot in snapshots {
        if !snapshot.symbol.eq_ignore_ascii_case(symbol)
            || snapshot.ts > target_ts
            || !contract_whale_runtime_config().exchange_enabled(snapshot.exchange.as_key())
        {
            continue;
        }
        matching_before_target = true;
        if target_ts.saturating_sub(snapshot.ts) > max_gap_ms {
            continue;
        }
        let key = snapshot.exchange.as_key().to_string();
        let replace = latest_by_exchange
            .get(&key)
            .is_none_or(|existing| snapshot.ts > existing.ts);
        if replace {
            latest_by_exchange.insert(key, snapshot);
        }
    }
    let values = latest_by_exchange
        .values()
        .map(|snapshot| snapshot.funding_rate)
        .filter(|value| value.is_finite())
        .collect::<Vec<_>>();
    if values.is_empty() {
        (None, matching_before_target)
    } else {
        (
            Some(values.iter().sum::<f64>() / values.len() as f64),
            false,
        )
    }
}

fn evidence_reason(available: bool, stale: bool) -> Option<String> {
    if available {
        None
    } else if stale {
        Some("stale".to_string())
    } else {
        Some("missing".to_string())
    }
}

fn oi_bias(oi_change_pct: Option<f64>) -> &'static str {
    match oi_change_pct {
        Some(value) if value >= 0.10 => "rising",
        Some(value) if value <= -0.10 => "falling",
        Some(_) => "flat",
        None => "unknown",
    }
}

fn funding_bias(funding_rate: Option<f64>) -> &'static str {
    match funding_rate {
        Some(value) if value >= 0.0001 => "long",
        Some(value) if value <= -0.0001 => "short",
        Some(_) => "neutral",
        None => "unknown",
    }
}

fn window_volume_samples(
    buckets: &[ContractFlowBucket],
    symbol: &str,
    exchange: &str,
    window_sec: u64,
    from_ts: i64,
    to_ts: i64,
) -> Vec<f64> {
    let window_ms = (window_sec as i64).saturating_mul(1000).max(1000);
    let exchange_filter = exchange.to_ascii_lowercase();
    let config = contract_whale_runtime_config();
    let mut grouped: BTreeMap<i64, f64> = BTreeMap::new();
    for bucket in buckets {
        if !bucket.symbol.eq_ignore_ascii_case(symbol)
            || bucket.ts_bucket < from_ts
            || bucket.ts_bucket > to_ts
        {
            continue;
        }
        if exchange_filter != "all" && !bucket.exchange.eq_ignore_ascii_case(&exchange_filter) {
            continue;
        }
        if !config.exchange_enabled(&bucket.exchange) {
            continue;
        }
        let window_key = bucket.ts_bucket / window_ms;
        let total = bucket.buy_volume_btc + bucket.sell_volume_btc;
        if total > 0.0 {
            *grouped.entry(window_key).or_insert(0.0) += total;
        }
    }
    grouped.into_values().filter(|value| *value > 0.0).collect()
}

fn percentile_nearest_rank(sorted_samples: &[f64], percentile: f64) -> f64 {
    if sorted_samples.is_empty() {
        return 0.0;
    }
    let rank = (percentile * sorted_samples.len() as f64).ceil() as usize;
    let index = rank.saturating_sub(1).min(sorted_samples.len() - 1);
    sorted_samples[index]
}

fn dominance(abs_net_volume: f64, total_volume: f64) -> f64 {
    if total_volume <= f64::EPSILON {
        0.0
    } else {
        abs_net_volume / total_volume
    }
}

fn share(part: f64, total: f64) -> f64 {
    if total <= f64::EPSILON {
        0.0
    } else {
        part.max(0.0) / total
    }
}

fn apply_net_contribution_shares(
    exchanges: &mut [ExchangeFlowContribution],
    total_net_volume_btc: f64,
) {
    let net_positive = total_net_volume_btc > 0.0;
    let same_direction_net_sum = exchanges
        .iter()
        .filter(|item| item.net_volume_btc.abs() > f64::EPSILON)
        .filter(|item| (item.net_volume_btc > 0.0) == net_positive)
        .map(|item| item.net_volume_btc.abs())
        .sum::<f64>();
    for item in exchanges {
        item.net_contribution_share = if same_direction_net_sum > f64::EPSILON
            && item.net_volume_btc.abs() > f64::EPSILON
            && (item.net_volume_btc > 0.0) == net_positive
        {
            item.net_volume_btc.abs() / same_direction_net_sum
        } else {
            0.0
        };
    }
}

fn dominant_venue_net_contribution_share(exchanges: &[ExchangeFlowContribution]) -> Option<f64> {
    exchanges
        .iter()
        .map(|item| item.net_contribution_share)
        .filter(|value| value.is_finite() && *value > 0.0)
        .max_by(|left, right| left.total_cmp(right))
}
