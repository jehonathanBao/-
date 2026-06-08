use std::{
    collections::{BTreeMap, BTreeSet},
    fs,
    path::Path,
};

use anyhow::{anyhow, bail, Context};
use serde::{Deserialize, Serialize};

use super::{
    aggregator::{
        aggregate_1s_buckets, dynamic_multiple_for_volume,
        historical_window_average_btc_with_min_samples, rolling_window_stats,
    },
    config::contract_whale_runtime_config,
    detector::detect_contract_whale_signal,
    merge::merge_contract_whale_signals,
    types::{
        ContractExchange, ContractTrade, ContractTradeSide, ContractWhaleDirection,
        ContractWhaleSeverity, ContractWhaleSignal, ContractWhaleSignalType,
    },
};

const REPLAY_WINDOWS_SEC: [u64; 3] = [5, 15, 60];
const REPLAY_STARTUP_AGE_MS: i64 = 61_000;

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ContractWhaleReplayReport {
    pub input: String,
    pub trades_read: usize,
    pub signals_generated: usize,
    pub severity_distribution: BTreeMap<String, usize>,
    pub discord_eligible_count: usize,
    pub false_positive_notes: Vec<String>,
    pub signals: Vec<ContractWhaleReplaySignalSummary>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ContractWhaleReplaySignalSummary {
    pub id: String,
    pub ts: i64,
    pub symbol: String,
    pub window_sec: u64,
    pub signal_type: String,
    pub direction: String,
    pub severity: String,
    pub score: u8,
    pub data_quality: u8,
    pub discord_eligible: bool,
    pub discord_reason: String,
    pub liquidation_suspected: bool,
}

#[derive(Debug, Clone, Deserialize)]
struct ReplayTradeLine {
    pub ts: i64,
    pub exchange: String,
    #[serde(default)]
    pub symbol: Option<String>,
    pub price: f64,
    #[serde(alias = "qtyBtc")]
    pub qty_btc: f64,
    pub side: String,
    #[serde(default, alias = "rawTradeCount")]
    pub raw_trade_count: Option<u64>,
    #[serde(default, alias = "dataQuality")]
    pub data_quality: Option<u8>,
    #[serde(default, alias = "dynamicMultiple")]
    pub dynamic_multiple: Option<f64>,
    #[serde(default, alias = "percentileLevel")]
    pub percentile_level: Option<f64>,
    #[serde(default, alias = "priceMovePct")]
    pub price_move_pct: Option<f64>,
    #[serde(default, alias = "priceReversalRatio")]
    pub price_reversal_ratio: Option<f64>,
    #[serde(default, alias = "liquidationDriven")]
    pub liquidation_driven: Option<bool>,
}

#[derive(Debug, Clone, Default)]
struct ReplayPointMetadata {
    data_quality: Option<u8>,
    dynamic_multiple: Option<f64>,
    percentile_level: Option<f64>,
    price_move_pct: Option<f64>,
    price_reversal_ratio: Option<f64>,
    liquidation_driven: bool,
}

pub fn run_contract_whale_replay(input: &Path) -> anyhow::Result<ContractWhaleReplayReport> {
    let text = fs::read_to_string(input)
        .with_context(|| format!("failed to read replay input {}", input.display()))?;
    run_contract_whale_replay_from_str(&input.display().to_string(), &text)
}

pub fn run_contract_whale_replay_from_str(
    input_name: &str,
    text: &str,
) -> anyhow::Result<ContractWhaleReplayReport> {
    let rows = parse_replay_rows(text)?;
    if rows.is_empty() {
        bail!("replay input contains no trades");
    }
    let symbol = rows
        .iter()
        .find_map(|row| row.symbol.as_deref())
        .map(normalize_symbol)
        .unwrap_or_else(|| "BTC".to_string());
    let mut metadata_by_ts = BTreeMap::<i64, ReplayPointMetadata>::new();
    let mut trades = Vec::with_capacity(rows.len());
    for row in rows {
        merge_metadata(metadata_by_ts.entry(row.ts).or_default(), &row);
        trades.push(replay_line_to_trade(row, &symbol)?);
    }
    trades.sort_by(|left, right| {
        left.ts
            .cmp(&right.ts)
            .then_with(|| left.exchange.cmp(&right.exchange))
            .then_with(|| trade_side_key(left.side).cmp(&trade_side_key(right.side)))
    });
    let buckets = aggregate_1s_buckets(&trades);
    let min_dynamic_samples = contract_whale_runtime_config()
        .data_quality
        .min_dynamic_samples;

    let mut detected = Vec::new();
    for (ts, metadata) in &metadata_by_ts {
        let available_buckets = buckets
            .iter()
            .filter(|bucket| bucket.ts_bucket <= *ts)
            .cloned()
            .collect::<Vec<_>>();
        for window_sec in REPLAY_WINDOWS_SEC {
            let dynamic_multiple = metadata.dynamic_multiple.or_else(|| {
                dynamic_multiple_for_window(
                    &available_buckets,
                    &symbol,
                    window_sec,
                    *ts,
                    min_dynamic_samples,
                )
            });
            let price_move_pct = metadata
                .price_move_pct
                .or_else(|| price_move_pct_for_window(&trades, *ts, window_sec));
            let mut stats = match rolling_window_stats(
                &available_buckets,
                &symbol,
                window_sec,
                *ts,
                price_move_pct,
                dynamic_multiple,
                metadata.data_quality.unwrap_or(85),
            ) {
                Some(stats) => stats,
                None => continue,
            };
            stats.percentile_level = metadata.percentile_level;
            stats.price_reversal_ratio = metadata.price_reversal_ratio;
            stats.liquidation_driven = metadata.liquidation_driven;
            stats.startup_age_ms = Some(REPLAY_STARTUP_AGE_MS);
            if let Some(signal) = detect_contract_whale_signal(&stats) {
                detected.push(signal);
            }
        }
    }
    let mut signals = merge_contract_whale_signals(detected);
    signals.sort_by(|left, right| {
        right
            .severity
            .rank()
            .cmp(&left.severity.rank())
            .then_with(|| right.score.cmp(&left.score))
            .then_with(|| right.ts.cmp(&left.ts))
            .then_with(|| right.window_sec.cmp(&left.window_sec))
    });
    Ok(build_report(input_name, trades.len(), signals))
}

pub fn format_contract_whale_replay_report(report: &ContractWhaleReplayReport) -> String {
    let mut lines = vec![
        "CWM Replay Report".to_string(),
        format!("input: {}", report.input),
        format!("trades read: {}", report.trades_read),
        format!("signals generated: {}", report.signals_generated),
        "severity distribution:".to_string(),
    ];
    for severity in ["s", "critical", "high", "medium", "calm"] {
        let count = report
            .severity_distribution
            .get(severity)
            .copied()
            .unwrap_or(0);
        lines.push(format!("  {severity}: {count}"));
    }
    lines.push(format!(
        "discord eligible count: {}",
        report.discord_eligible_count
    ));
    lines.push("false positive notes:".to_string());
    if report.false_positive_notes.is_empty() {
        lines.push("  - none".to_string());
    } else {
        for note in &report.false_positive_notes {
            lines.push(format!("  - {note}"));
        }
    }
    lines.push("signals:".to_string());
    if report.signals.is_empty() {
        lines.push("  - none".to_string());
    } else {
        for signal in &report.signals {
            lines.push(format!(
                "  - id={} ts={} window={}s severity={} type={} direction={} score={} dataQuality={} discordEligible={} reason={}",
                signal.id,
                signal.ts,
                signal.window_sec,
                signal.severity,
                signal.signal_type,
                signal.direction,
                signal.score,
                signal.data_quality,
                signal.discord_eligible,
                signal.discord_reason
            ));
        }
    }
    lines.join("\n")
}

fn build_report(
    input_name: &str,
    trades_read: usize,
    signals: Vec<ContractWhaleSignal>,
) -> ContractWhaleReplayReport {
    let mut severity_distribution = BTreeMap::from([
        ("s".to_string(), 0),
        ("critical".to_string(), 0),
        ("high".to_string(), 0),
        ("medium".to_string(), 0),
        ("calm".to_string(), 0),
    ]);
    let discord_eligible_count = signals
        .iter()
        .filter(|signal| signal.discord_eligible)
        .count();
    for signal in &signals {
        *severity_distribution
            .entry(severity_key(signal.severity).to_string())
            .or_insert(0) += 1;
    }
    let false_positive_notes = false_positive_notes(&signals);
    let signal_summaries: Vec<ContractWhaleReplaySignalSummary> = signals
        .into_iter()
        .map(|signal| ContractWhaleReplaySignalSummary {
            id: signal.id,
            ts: signal.ts,
            symbol: signal.symbol,
            window_sec: signal.window_sec,
            signal_type: signal_type_key(signal.signal_type).to_string(),
            direction: direction_key(signal.direction).to_string(),
            severity: severity_key(signal.severity).to_string(),
            score: signal.score,
            data_quality: signal.data_quality,
            discord_eligible: signal.discord_eligible,
            discord_reason: signal.discord_reason,
            liquidation_suspected: signal.liquidation_suspected,
        })
        .collect();
    ContractWhaleReplayReport {
        input: input_name.to_string(),
        trades_read,
        signals_generated: signal_summaries.len(),
        severity_distribution,
        discord_eligible_count,
        false_positive_notes,
        signals: signal_summaries,
    }
}

fn false_positive_notes(signals: &[ContractWhaleSignal]) -> Vec<String> {
    let mut notes = BTreeSet::new();
    if signals.is_empty() {
        notes.insert("no_signal_generated".to_string());
    }
    notes.insert("candidate_only_requires_human_label_review".to_string());
    for signal in signals {
        if signal.exchanges.len() < 2 {
            notes.insert("single_exchange_confirmation_missing".to_string());
        }
        if signal.liquidation_suspected {
            notes.insert("liquidation_suspected_reduced_confidence".to_string());
        }
        if !signal.discord_eligible {
            notes.insert("display_only_signal".to_string());
        }
    }
    notes.into_iter().collect()
}

fn parse_replay_rows(text: &str) -> anyhow::Result<Vec<ReplayTradeLine>> {
    let mut rows = Vec::new();
    for (index, line) in text.lines().enumerate() {
        let trimmed = line.trim().trim_start_matches('\u{feff}');
        if trimmed.is_empty() {
            continue;
        }
        let row = serde_json::from_str::<ReplayTradeLine>(trimmed)
            .with_context(|| format!("invalid replay jsonl at line {}", index + 1))?;
        validate_replay_row(index + 1, &row)?;
        rows.push(row);
    }
    Ok(rows)
}

fn validate_replay_row(line_no: usize, row: &ReplayTradeLine) -> anyhow::Result<()> {
    if row.ts <= 0 {
        bail!("line {line_no} ts must be positive");
    }
    if !row.price.is_finite() || row.price <= 0.0 {
        bail!("line {line_no} price must be positive");
    }
    if !row.qty_btc.is_finite() || row.qty_btc <= 0.0 {
        bail!("line {line_no} qty_btc must be positive");
    }
    parse_exchange(&row.exchange).with_context(|| format!("line {line_no} invalid exchange"))?;
    parse_side(&row.side).with_context(|| format!("line {line_no} invalid side"))?;
    Ok(())
}

fn replay_line_to_trade(
    row: ReplayTradeLine,
    default_symbol: &str,
) -> anyhow::Result<ContractTrade> {
    let exchange = parse_exchange(&row.exchange)?;
    let side = parse_side(&row.side)?;
    let symbol = row
        .symbol
        .as_deref()
        .map(normalize_symbol)
        .unwrap_or_else(|| default_symbol.to_string());
    Ok(ContractTrade {
        ts: row.ts,
        exchange,
        symbol,
        market: "perp".to_string(),
        price: row.price,
        qty_btc: row.qty_btc,
        notional_usd: row.price * row.qty_btc,
        side,
        raw_trade_count: row.raw_trade_count,
    })
}

fn merge_metadata(metadata: &mut ReplayPointMetadata, row: &ReplayTradeLine) {
    metadata.data_quality = row.data_quality.or(metadata.data_quality);
    metadata.dynamic_multiple = row.dynamic_multiple.or(metadata.dynamic_multiple);
    metadata.percentile_level = row.percentile_level.or(metadata.percentile_level);
    metadata.price_move_pct = row.price_move_pct.or(metadata.price_move_pct);
    metadata.price_reversal_ratio = row.price_reversal_ratio.or(metadata.price_reversal_ratio);
    metadata.liquidation_driven =
        metadata.liquidation_driven || row.liquidation_driven.unwrap_or(false);
}

fn dynamic_multiple_for_window(
    buckets: &[super::types::ContractFlowBucket],
    symbol: &str,
    window_sec: u64,
    now: i64,
    min_samples: usize,
) -> Option<f64> {
    let window_ms = (window_sec as i64).saturating_mul(1000);
    let dynamic_to = now.saturating_sub(window_ms);
    let dynamic_from = dynamic_to.saturating_sub(60 * 60 * 1000);
    let current_total =
        rolling_window_stats(buckets, symbol, window_sec, now, None, None, 85)?.total_volume_btc;
    dynamic_multiple_for_volume(
        current_total,
        historical_window_average_btc_with_min_samples(
            buckets,
            symbol,
            window_sec,
            dynamic_from,
            dynamic_to,
            min_samples,
        ),
    )
}

fn price_move_pct_for_window(trades: &[ContractTrade], now: i64, window_sec: u64) -> Option<f64> {
    let start = now.saturating_sub((window_sec as i64).saturating_mul(1000));
    let mut in_window = trades
        .iter()
        .filter(|trade| trade.ts >= start && trade.ts <= now && trade.price.is_finite())
        .collect::<Vec<_>>();
    in_window.sort_by_key(|trade| trade.ts);
    let first = in_window.first()?;
    let last = in_window.last()?;
    if first.price <= f64::EPSILON {
        return None;
    }
    Some((last.price - first.price) / first.price * 100.0)
}

fn parse_exchange(value: &str) -> anyhow::Result<ContractExchange> {
    match value.trim().to_ascii_lowercase().as_str() {
        "binance" => Ok(ContractExchange::Binance),
        "okx" => Ok(ContractExchange::Okx),
        "bitfinex" => Ok(ContractExchange::Bitfinex),
        other => Err(anyhow!("unsupported exchange `{other}`")),
    }
}

fn parse_side(value: &str) -> anyhow::Result<ContractTradeSide> {
    match value.trim().to_ascii_lowercase().as_str() {
        "buy" | "bid" | "taker_buy" | "active_buy" => Ok(ContractTradeSide::Buy),
        "sell" | "ask" | "taker_sell" | "active_sell" => Ok(ContractTradeSide::Sell),
        other => Err(anyhow!("unsupported side `{other}`")),
    }
}

fn normalize_symbol(value: &str) -> String {
    let base = value
        .trim()
        .split(['-', '_', '/', ':'])
        .next()
        .unwrap_or(value)
        .to_ascii_uppercase();
    base.strip_suffix("USDT").unwrap_or(&base).to_string()
}

fn severity_key(severity: ContractWhaleSeverity) -> &'static str {
    match severity {
        ContractWhaleSeverity::S => "s",
        ContractWhaleSeverity::Critical => "critical",
        ContractWhaleSeverity::High => "high",
        ContractWhaleSeverity::Medium => "medium",
        ContractWhaleSeverity::Calm => "calm",
    }
}

fn signal_type_key(signal_type: ContractWhaleSignalType) -> &'static str {
    match signal_type {
        ContractWhaleSignalType::AggressiveBuy => "aggressive_buy",
        ContractWhaleSignalType::AggressiveSell => "aggressive_sell",
        ContractWhaleSignalType::DownsideAbsorption => "downside_absorption",
        ContractWhaleSignalType::UpsideSuppression => "upside_suppression",
    }
}

fn direction_key(direction: ContractWhaleDirection) -> &'static str {
    match direction {
        ContractWhaleDirection::Buy => "buy",
        ContractWhaleDirection::Sell => "sell",
        ContractWhaleDirection::Absorption => "absorption",
        ContractWhaleDirection::Suppression => "suppression",
    }
}

fn trade_side_key(side: ContractTradeSide) -> u8 {
    match side {
        ContractTradeSide::Buy => 0,
        ContractTradeSide::Sell => 1,
    }
}
