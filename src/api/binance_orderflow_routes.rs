use axum::{
    extract::{Query, State},
    http::StatusCode,
    response::IntoResponse,
    Json,
};
use serde::{Deserialize, Serialize};
use std::{
    collections::HashMap,
    sync::{
        atomic::{AtomicI64, Ordering},
        OnceLock,
    },
};
use tokio::sync::Mutex as AsyncMutex;

use crate::{
    app::AppState,
    normalizers::trade::now_ms,
    storage::binance_orderflow_repo::BinanceOrderflowDeltaCacheRow,
    storage::contract_whale_repo::ContractWhaleRepo,
    types::{toxic::ToxicSeverity, vpin::VpinBucket},
};

const BINANCE_FUTURES_KLINES_URL: &str = "https://fapi.binance.com/fapi/v1/klines";
const DEFAULT_SYMBOL: &str = "BTCUSDT";
const DEFAULT_INTERVAL: &str = "1h";
const DEFAULT_LIMIT: usize = 300;
const DELTA_CACHE_RETENTION_MS: i64 = 21 * 24 * 60 * 60 * 1_000;
const DELTA_CACHE_PURGE_INTERVAL_MS: i64 = 60 * 60 * 1_000;
const BINANCE_KLINE_CACHE_TTL_MS: i64 = 1_500;
const BINANCE_KLINE_CACHE_MAX_ENTRIES: usize = 64;
static LAST_DELTA_CACHE_PURGE_MS: AtomicI64 = AtomicI64::new(0);
static BINANCE_HTTP_CLIENT: OnceLock<reqwest::Client> = OnceLock::new();
static BINANCE_KLINE_CACHE: OnceLock<AsyncMutex<HashMap<String, CachedKlines>>> = OnceLock::new();

#[derive(Clone)]
struct CachedKlines {
    fetched_at_ms: i64,
    rows: Vec<serde_json::Value>,
}

#[derive(Debug, Deserialize)]
pub struct BinanceOrderflowQuery {
    pub symbol: Option<String>,
    pub interval: Option<String>,
    pub limit: Option<usize>,
    #[serde(rename = "startTime")]
    pub start_time: Option<i64>,
    #[serde(rename = "endTime")]
    pub end_time: Option<i64>,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct BinanceOrderflowResponse {
    pub exchange: &'static str,
    pub symbol: String,
    pub market: &'static str,
    pub interval: String,
    pub candles: Vec<BinanceOrderflowCandle>,
    pub source: String,
    pub monitor_flow_candles: usize,
    pub as_of_ms: i64,
    pub read_only: bool,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct BinanceOrderflowCandle {
    pub time: i64,
    pub close_time: i64,
    pub open: f64,
    pub high: f64,
    pub low: f64,
    pub close: f64,
    pub volume_base: f64,
    pub volume_quote: f64,
    pub buy_base: f64,
    pub sell_base: f64,
    pub buy_quote: f64,
    pub sell_quote: f64,
    pub delta_base: f64,
    pub delta_quote: f64,
    pub delta_pct: f64,
    pub trade_count: u64,
    pub closed: bool,
    pub vpin: Option<f64>,
    pub vpin_zscore: Option<f64>,
    pub vpin_percentile: Option<f64>,
    pub vpin_spike: bool,
    pub vpin_high: bool,
    pub vpin_extreme: bool,
    pub tof_volume_btc: Option<f64>,
    pub tof_severity: Option<ToxicSeverity>,
    pub tof_alert: bool,
    pub tof_reasons: Vec<String>,
}

pub async fn binance_orderflow_route(
    State(state): State<AppState>,
    Query(query): Query<BinanceOrderflowQuery>,
) -> impl IntoResponse {
    let symbol = match canonical_symbol(query.symbol.as_deref()) {
        Ok(symbol) => symbol,
        Err(message) => return error_response(StatusCode::BAD_REQUEST, message),
    };
    let interval = query
        .interval
        .unwrap_or_else(|| DEFAULT_INTERVAL.to_string());
    if !is_supported_interval(&interval) {
        return error_response(
            StatusCode::BAD_REQUEST,
            format!("unsupported Binance interval: {interval}"),
        );
    }
    let limit = query.limit.unwrap_or(DEFAULT_LIMIT).clamp(1, 1_000);
    if query.start_time.is_some_and(|value| value < 0)
        || query.end_time.is_some_and(|value| value < 0)
    {
        return error_response(
            StatusCode::BAD_REQUEST,
            "time bounds must be positive".to_string(),
        );
    }

    let rows =
        match fetch_binance_klines(&symbol, &interval, limit, query.start_time, query.end_time)
            .await
        {
            Ok(rows) => rows,
            Err(error) => return error_response(StatusCode::BAD_GATEWAY, error),
        };
    let as_of_ms = now_ms();
    let mut candles = match rows
        .iter()
        .map(|row| parse_kline_row(row, as_of_ms))
        .collect::<anyhow::Result<Vec<_>>>()
    {
        Ok(candles) => candles,
        Err(error) => {
            return error_response(
                StatusCode::BAD_GATEWAY,
                format!("Binance K 线字段解析失败: {error}"),
            )
        }
    };

    // The monitor-flow and TOF overlays read SQLite synchronously. Keep that
    // work off the async HTTP executor so a locked/compacting database cannot
    // stall unrelated websocket and REST traffic.
    let overlay_state = state.clone();
    let overlay_symbol = symbol.clone();
    let overlay_interval = interval.clone();
    let overlay_result = tokio::task::spawn_blocking(move || {
        let monitor_flow_candles = merge_monitor_flow(
            &overlay_state,
            &overlay_symbol,
            &overlay_interval,
            &mut candles,
            as_of_ms,
        );
        maybe_purge_delta_cache(&overlay_state, as_of_ms);
        merge_tof_overlay(&overlay_state, &overlay_interval, &mut candles);
        (monitor_flow_candles, candles)
    })
    .await;
    let (monitor_flow_candles, candles) = match overlay_result {
        Ok(result) => result,
        Err(error) => {
            tracing::warn!(?error, "orderflow overlay task failed");
            (0, Vec::new())
        }
    };
    Json(BinanceOrderflowResponse {
        exchange: "binance",
        symbol,
        market: "usdt_perpetual",
        interval,
        candles,
        source: if monitor_flow_candles > 0 {
            "binance_monitor_flow_with_futures_kline".to_string()
        } else {
            "binance_futures_kline".to_string()
        },
        monitor_flow_candles,
        as_of_ms,
        read_only: true,
    })
    .into_response()
}

async fn fetch_binance_klines(
    symbol: &str,
    interval: &str,
    limit: usize,
    start_time: Option<i64>,
    end_time: Option<i64>,
) -> Result<Vec<serde_json::Value>, String> {
    let key = format!("{symbol}|{interval}|{limit}|{start_time:?}|{end_time:?}");
    let cache = BINANCE_KLINE_CACHE.get_or_init(|| AsyncMutex::new(HashMap::new()));
    // The mutex intentionally covers the upstream request as a small
    // singleflight gate. A burst of dashboard tabs therefore shares one
    // Binance request instead of multiplying upstream traffic.
    let mut cache_guard = cache.lock().await;
    let now = now_ms();
    if let Some(cached) = cache_guard.get(&key) {
        if now.saturating_sub(cached.fetched_at_ms) <= BINANCE_KLINE_CACHE_TTL_MS {
            return Ok(cached.rows.clone());
        }
    }

    let client = BINANCE_HTTP_CLIENT.get_or_init(reqwest::Client::new);
    let mut request = client
        .get(BINANCE_FUTURES_KLINES_URL)
        .query(&[("symbol", symbol), ("interval", interval)])
        .query(&[("limit", limit.to_string())]);
    if let Some(start_time) = start_time {
        request = request.query(&[("startTime", start_time.to_string())]);
    }
    if let Some(end_time) = end_time {
        request = request.query(&[("endTime", end_time.to_string())]);
    }

    let response = request
        .send()
        .await
        .map_err(|error| format!("Binance K 线请求失败: {error}"))?;
    if !response.status().is_success() {
        let status = response.status();
        let detail = response.text().await.unwrap_or_default();
        return Err(format!("Binance K 线返回 {status}: {detail}"));
    }
    let value = response
        .json::<serde_json::Value>()
        .await
        .map_err(|error| format!("Binance K 线响应解析失败: {error}"))?;
    let rows = value
        .as_array()
        .ok_or_else(|| "Binance K 线格式错误".to_string())?
        .to_vec();
    if cache_guard.len() >= BINANCE_KLINE_CACHE_MAX_ENTRIES {
        if let Some(oldest_key) = cache_guard
            .iter()
            .min_by_key(|(_, value)| value.fetched_at_ms)
            .map(|(key, _)| key.clone())
        {
            cache_guard.remove(&oldest_key);
        }
    }
    cache_guard.insert(
        key,
        CachedKlines {
            fetched_at_ms: now_ms(),
            rows: rows.clone(),
        },
    );
    Ok(rows)
}

#[derive(Default)]
struct MonitorFlowAccumulator {
    buy_base: f64,
    sell_base: f64,
    buy_quote: f64,
    sell_quote: f64,
    trade_count: u64,
}

fn merge_monitor_flow(
    state: &AppState,
    symbol: &str,
    interval: &str,
    candles: &mut [BinanceOrderflowCandle],
    as_of_ms: i64,
) -> usize {
    let Some(interval_ms) = interval_millis(interval) else {
        return 0;
    };
    let Some(first) = candles.first() else {
        return 0;
    };
    let base_symbol = symbol.trim_end_matches("USDT");
    let Some(store) = state.contract_whale_store() else {
        return 0;
    };
    let cached = store
        .list_binance_orderflow_delta_cache(
            symbol,
            interval,
            first.time,
            candles.last().map(|candle| candle.time).unwrap_or(as_of_ms),
        )
        .unwrap_or_else(|error| {
            tracing::debug!(symbol, interval, error = %error, "binance orderflow delta cache unavailable");
            Vec::new()
        });
    let mut cached_by_candle = cached
        .into_iter()
        .map(|row| (row.candle_time, row))
        .collect::<HashMap<_, _>>();
    let flow_from = candles
        .iter()
        .filter(|candle| {
            !candle.closed
                || cached_by_candle
                    .get(&candle.time)
                    .is_none_or(|row| !volumes_match(row.volume_base, candle.volume_base))
        })
        .map(|candle| candle.time)
        .min();
    let buckets = match flow_from
        .map(|from_ts| store.list_contract_flow_buckets_between(base_symbol, from_ts, as_of_ms))
        .transpose()
    {
        Ok(Some(buckets)) => buckets,
        Ok(None) => Vec::new(),
        Err(error) => {
            tracing::debug!(symbol, error = %error, "binance orderflow monitor flow unavailable");
            return 0;
        }
    };
    let mut by_candle = HashMap::<i64, MonitorFlowAccumulator>::new();
    for bucket in buckets {
        if !bucket.exchange.eq_ignore_ascii_case("binance") {
            continue;
        }
        let key = bucket.ts_bucket - bucket.ts_bucket.rem_euclid(interval_ms);
        let entry = by_candle.entry(key).or_default();
        entry.buy_base += bucket.buy_volume_btc;
        entry.sell_base += bucket.sell_volume_btc;
        entry.buy_quote += bucket.buy_notional_usd;
        entry.sell_quote += bucket.sell_notional_usd;
        entry.trade_count = entry.trade_count.saturating_add(bucket.trade_count);
    }

    let mut merged = 0;
    let mut cache_rows = Vec::new();
    for candle in candles.iter_mut() {
        // Completed candles are immutable from the chart's point of view. Use
        // the recorded aggregation first so polling does not repeatedly scan
        // the 1-second monitor buckets or make old Delta values flicker.
        if candle.closed {
            if let Some(row) = cached_by_candle.remove(&candle.time) {
                if volumes_match(row.volume_base, candle.volume_base) {
                    apply_cached_flow(candle, &row);
                    merged += 1;
                    continue;
                }
            }
        }
        let Some(flow) = by_candle.remove(&candle.time) else {
            continue;
        };
        let flow_volume = flow.buy_base + flow.sell_base;
        if candle.volume_base <= f64::EPSILON
            || ((flow_volume - candle.volume_base).abs() / candle.volume_base) > 0.03
        {
            continue;
        }
        candle.buy_base = flow.buy_base;
        candle.sell_base = flow.sell_base;
        candle.buy_quote = flow.buy_quote;
        candle.sell_quote = flow.sell_quote;
        candle.trade_count = flow.trade_count;
        refresh_delta(candle);
        merged += 1;
        if candle.closed {
            cache_rows.push(BinanceOrderflowDeltaCacheRow {
                symbol: symbol.to_string(),
                interval: interval.to_string(),
                candle_time: candle.time,
                close_time: candle.close_time,
                volume_base: candle.volume_base,
                volume_quote: candle.volume_quote,
                buy_base: candle.buy_base,
                sell_base: candle.sell_base,
                buy_quote: candle.buy_quote,
                sell_quote: candle.sell_quote,
                delta_base: candle.delta_base,
                delta_quote: candle.delta_quote,
                delta_pct: candle.delta_pct,
                trade_count: candle.trade_count,
                computed_at_ms: as_of_ms,
            });
        }
    }
    if let Err(error) = store.upsert_binance_orderflow_delta_cache(&cache_rows) {
        tracing::debug!(symbol, interval, error = %error, "failed to persist Binance orderflow delta cache");
    }
    merged
}

fn apply_cached_flow(candle: &mut BinanceOrderflowCandle, row: &BinanceOrderflowDeltaCacheRow) {
    candle.buy_base = row.buy_base;
    candle.sell_base = row.sell_base;
    candle.buy_quote = row.buy_quote;
    candle.sell_quote = row.sell_quote;
    candle.trade_count = row.trade_count;
    candle.delta_base = row.delta_base;
    candle.delta_quote = row.delta_quote;
    candle.delta_pct = row.delta_pct;
}

fn volumes_match(cached: f64, current: f64) -> bool {
    current > f64::EPSILON && cached.is_finite() && ((cached - current).abs() / current) <= 0.03
}

fn maybe_purge_delta_cache(state: &AppState, now_ms: i64) {
    let previous = LAST_DELTA_CACHE_PURGE_MS.load(Ordering::Relaxed);
    if now_ms.saturating_sub(previous) < DELTA_CACHE_PURGE_INTERVAL_MS {
        return;
    }
    if LAST_DELTA_CACHE_PURGE_MS
        .compare_exchange(previous, now_ms, Ordering::Relaxed, Ordering::Relaxed)
        .is_err()
    {
        return;
    }
    if let Some(store) = state.contract_whale_store() {
        let cutoff = now_ms.saturating_sub(DELTA_CACHE_RETENTION_MS);
        if let Err(error) = store.purge_binance_orderflow_delta_cache_before(cutoff) {
            tracing::debug!(error = %error, "failed to purge Binance orderflow delta cache");
        }
    }
}

fn refresh_delta(candle: &mut BinanceOrderflowCandle) {
    candle.delta_base = candle.buy_base - candle.sell_base;
    candle.delta_quote = candle.buy_quote - candle.sell_quote;
    candle.delta_pct = if candle.volume_quote.abs() > f64::EPSILON {
        candle.delta_quote / candle.volume_quote * 100.0
    } else {
        0.0
    };
}

fn merge_tof_overlay(state: &AppState, interval: &str, candles: &mut [BinanceOrderflowCandle]) {
    let Some(interval_ms) = interval_millis(interval) else {
        return;
    };
    let vpin_state = state.vpin_state();
    // Prefer the persisted bucket history so a backend restart does not make
    // every K-line lose its VPIN overlay during the warm-up window.
    let history_limit = vpin_state
        .metrics
        .lookback_buckets
        .saturating_mul(8)
        .max(500);
    let mut buckets = state
        .recent_vpin_buckets(history_limit)
        .unwrap_or(vpin_state.recent_buckets);
    buckets.sort_by_key(|bucket| bucket.end_ts);
    let lookback = vpin_state.metrics.lookback_buckets.max(2);
    let min_buckets = vpin_state.metrics.min_buckets.max(2);
    let ratios = buckets
        .iter()
        .filter_map(binance_bucket_ratio)
        .collect::<Vec<_>>();
    let mut observations = Vec::with_capacity(ratios.len());
    for (index, (end_ts, ratio)) in ratios.iter().enumerate() {
        let start = index.saturating_add(1).saturating_sub(lookback);
        let window = &ratios[start..=index];
        let rolling_vpin = (window.len() >= min_buckets)
            .then(|| window.iter().map(|(_, value)| *value).sum::<f64>() / window.len() as f64);
        let baseline_start = index.saturating_sub(lookback);
        let baseline = &ratios[baseline_start..index];
        let zscore = if baseline.len() >= min_buckets {
            let mean =
                baseline.iter().map(|(_, value)| *value).sum::<f64>() / baseline.len() as f64;
            let variance = baseline
                .iter()
                .map(|(_, value)| (*value - mean).powi(2))
                .sum::<f64>()
                / baseline.len() as f64;
            let stddev = variance.sqrt();
            (stddev > f64::EPSILON).then(|| (*ratio - mean) / stddev)
        } else {
            None
        };
        let percentile = if baseline.len() >= min_buckets {
            Some(
                baseline
                    .iter()
                    .filter(|(_, value)| *value <= *ratio)
                    .count() as f64
                    / baseline.len() as f64,
            )
        } else {
            None
        };
        observations.push((*end_ts, rolling_vpin, zscore, percentile));
    }
    let mut overlays =
        HashMap::<i64, (Option<f64>, Option<f64>, Option<f64>, bool, bool, bool)>::new();
    let high_threshold = env_f64("VPIN_HIGH_THRESHOLD", 0.70);
    let extreme_threshold = env_f64("VPIN_EXTREME_THRESHOLD", 0.85);
    let spike_threshold = env_f64("VPIN_SPIKE_ZSCORE", 2.5);
    for (end_ts, vpin, zscore, percentile) in observations {
        let key = end_ts - end_ts.rem_euclid(interval_ms);
        let entry = overlays
            .entry(key)
            .or_insert((None, None, None, false, false, false));
        // A candle represents the state at its latest completed bucket. Do not
        // use the maximum raw bucket imbalance; that turns a single print into
        // a false whole-candle VPIN alert.
        entry.0 = vpin;
        entry.1 = zscore;
        entry.2 = percentile;
        entry.3 = zscore.is_some_and(|value| value >= spike_threshold);
        entry.4 = vpin.is_some_and(|value| value >= high_threshold);
        entry.5 = vpin.is_some_and(|value| value >= extreme_threshold);
    }
    for candle in candles.iter_mut() {
        let Some((vpin, zscore, percentile, spike, high, extreme)) = overlays.remove(&candle.time)
        else {
            continue;
        };
        candle.vpin = vpin;
        candle.vpin_zscore = zscore.filter(|value| value.is_finite());
        candle.vpin_percentile = percentile.filter(|value| value.is_finite());
        candle.vpin_spike = spike;
        candle.vpin_high = high;
        candle.vpin_extreme = extreme;
    }
    let toxic_state = state.toxic_state();
    for event in toxic_state.recent_events {
        let key = event.ts - event.ts.rem_euclid(interval_ms);
        if let Some(candle) = candles.iter_mut().find(|candle| candle.time == key) {
            let replace = candle
                .tof_volume_btc
                .is_none_or(|value| event.toxic_volume_btc > value);
            if replace {
                candle.tof_volume_btc = Some(event.toxic_volume_btc);
                candle.tof_severity = Some(event.severity);
                candle.tof_alert = event.severity.is_at_least(ToxicSeverity::Alert);
                candle.tof_reasons = event.reason_codes;
            }
        }
    }
}

fn binance_bucket_ratio(bucket: &VpinBucket) -> Option<(i64, f64)> {
    let (_, venue) = bucket
        .venue_breakdown
        .iter()
        .find(|(venue, _)| venue.eq_ignore_ascii_case("binance"))?;
    let total = venue.buy_btc + venue.sell_btc;
    (total > f64::EPSILON).then(|| {
        (
            bucket.end_ts,
            ((venue.buy_btc - venue.sell_btc).abs() / total).clamp(0.0, 1.0),
        )
    })
}

fn env_f64(key: &str, default: f64) -> f64 {
    std::env::var(key)
        .ok()
        .and_then(|value| value.parse::<f64>().ok())
        .filter(|value| value.is_finite())
        .unwrap_or(default)
}

fn interval_millis(interval: &str) -> Option<i64> {
    Some(match interval {
        "1m" => 60_000,
        "3m" => 180_000,
        "5m" => 300_000,
        "15m" => 900_000,
        "30m" => 1_800_000,
        "1h" => 3_600_000,
        "2h" => 7_200_000,
        "4h" => 14_400_000,
        "6h" => 21_600_000,
        "8h" => 28_800_000,
        "12h" => 43_200_000,
        "1d" => 86_400_000,
        "3d" => 259_200_000,
        "1w" => 604_800_000,
        "1M" => return None,
        _ => return None,
    })
}

fn parse_kline_row(
    row: &serde_json::Value,
    as_of_ms: i64,
) -> anyhow::Result<BinanceOrderflowCandle> {
    let values = row
        .as_array()
        .ok_or_else(|| anyhow::anyhow!("K 线行不是数组"))?;
    if values.len() < 11 {
        anyhow::bail!("K 线行字段不足: {}", values.len());
    }
    let time = json_i64(&values[0])?;
    let open = json_f64(&values[1])?;
    let high = json_f64(&values[2])?;
    let low = json_f64(&values[3])?;
    let close = json_f64(&values[4])?;
    let volume_base = json_f64(&values[5])?;
    let close_time = json_i64(&values[6])?;
    let volume_quote = json_f64(&values[7])?;
    let trade_count = json_i64(&values[8])?.max(0) as u64;
    let buy_base = json_f64(&values[9])?;
    let buy_quote = json_f64(&values[10])?;
    let sell_base = (volume_base - buy_base).max(0.0);
    let sell_quote = (volume_quote - buy_quote).max(0.0);
    let delta_base = buy_base - sell_base;
    let delta_quote = buy_quote - sell_quote;
    let delta_pct = if volume_quote.abs() > f64::EPSILON {
        delta_quote / volume_quote * 100.0
    } else {
        0.0
    };

    Ok(BinanceOrderflowCandle {
        time,
        close_time,
        open,
        high,
        low,
        close,
        volume_base,
        volume_quote,
        buy_base,
        sell_base,
        buy_quote,
        sell_quote,
        delta_base,
        delta_quote,
        delta_pct,
        trade_count,
        closed: close_time < as_of_ms,
        vpin: None,
        vpin_zscore: None,
        vpin_percentile: None,
        vpin_spike: false,
        vpin_high: false,
        vpin_extreme: false,
        tof_volume_btc: None,
        tof_severity: None,
        tof_alert: false,
        tof_reasons: Vec::new(),
    })
}

fn canonical_symbol(raw: Option<&str>) -> Result<String, String> {
    let raw = raw.unwrap_or(DEFAULT_SYMBOL).trim().to_ascii_uppercase();
    let symbol = if raw.ends_with("-PERP") {
        format!("{}USDT", raw.trim_end_matches("-PERP"))
    } else if raw.ends_with("PERP") {
        format!("{}USDT", raw.trim_end_matches("PERP"))
    } else if raw.ends_with("USDT") {
        raw
    } else {
        format!("{raw}USDT")
    };
    if symbol.len() < 7
        || symbol.len() > 20
        || !symbol.ends_with("USDT")
        || !symbol.bytes().all(|byte| byte.is_ascii_alphanumeric())
    {
        return Err("symbol must be a Binance USDT perpetual symbol".to_string());
    }
    Ok(symbol)
}

fn is_supported_interval(interval: &str) -> bool {
    matches!(
        interval,
        "1m" | "3m"
            | "5m"
            | "15m"
            | "30m"
            | "1h"
            | "2h"
            | "4h"
            | "6h"
            | "8h"
            | "12h"
            | "1d"
            | "3d"
            | "1w"
            | "1M"
    )
}

fn json_i64(value: &serde_json::Value) -> anyhow::Result<i64> {
    if let Some(value) = value.as_i64() {
        return Ok(value);
    }
    if let Some(value) = value.as_u64() {
        return i64::try_from(value).map_err(|_| anyhow::anyhow!("整数超出范围"));
    }
    value
        .as_str()
        .ok_or_else(|| anyhow::anyhow!("字段不是整数"))?
        .parse::<i64>()
        .map_err(|error| anyhow::anyhow!("整数解析失败: {error}"))
}

fn json_f64(value: &serde_json::Value) -> anyhow::Result<f64> {
    let number = if let Some(value) = value.as_f64() {
        value
    } else {
        value
            .as_str()
            .ok_or_else(|| anyhow::anyhow!("字段不是数字"))?
            .parse::<f64>()?
    };
    if !number.is_finite() {
        anyhow::bail!("数字不是有限值");
    }
    Ok(number)
}

fn error_response(status: StatusCode, message: String) -> axum::response::Response {
    (
        status,
        Json(serde_json::json!({
            "ok": false,
            "readOnly": true,
            "error": message,
        })),
    )
        .into_response()
}

#[cfg(test)]
mod tests {
    use super::{canonical_symbol, parse_kline_row};

    #[test]
    fn canonicalizes_perpetual_symbols() {
        assert_eq!(canonical_symbol(Some("BTC-PERP")).unwrap(), "BTCUSDT");
        assert_eq!(canonical_symbol(Some("ethusdt")).unwrap(), "ETHUSDT");
    }

    #[test]
    fn parses_buy_sell_delta_from_binance_row() {
        let row =
            serde_json::json!([1, "100", "110", "90", "105", "10", 59, "1000", 12, "6", "600"]);
        let candle = parse_kline_row(&row, 100).unwrap();
        assert_eq!(candle.buy_base, 6.0);
        assert_eq!(candle.sell_base, 4.0);
        assert_eq!(candle.delta_base, 2.0);
        assert_eq!(candle.delta_quote, 200.0);
        assert!(candle.closed);
    }
}
