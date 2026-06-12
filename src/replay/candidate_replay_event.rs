use std::{
    collections::BTreeMap,
    fs::File,
    io::{BufRead, BufReader},
    path::Path,
};

use anyhow::{anyhow, Context};
use serde::{Deserialize, Serialize};

use crate::types::{
    market::{AggressorSide, Venue},
    orderbook_wall::OrderbookWallSide,
};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum CandidateReplayEventType {
    Trade,
    BookDelta,
    Snapshot,
    SnapshotReset,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CandidateReplayEvent {
    #[serde(rename = "type")]
    pub event_type: CandidateReplayEventType,
    pub venue: Venue,
    pub symbol: String,
    pub ts_ms: i64,
    pub side: Option<OrderbookWallSide>,
    pub price: Option<f64>,
    pub qty: Option<f64>,
    pub qty_before: Option<f64>,
    pub qty_after: Option<f64>,
    pub sequence: Option<u64>,
    pub trade_id: Option<String>,
    pub order_id: Option<String>,
    pub aggressor_side: Option<AggressorSide>,
}

pub fn load_candidate_replay_jsonl(
    path: impl AsRef<Path>,
) -> anyhow::Result<Vec<CandidateReplayEvent>> {
    let file = File::open(path.as_ref()).with_context(|| {
        format!(
            "failed to open candidate replay file {}",
            path.as_ref().display()
        )
    })?;
    let reader = BufReader::new(file);
    let mut events = Vec::new();

    for (index, line) in reader.lines().enumerate() {
        let line_no = index + 1;
        let line = line.with_context(|| format!("failed reading line {line_no}"))?;
        let trimmed = line.trim();
        if trimmed.is_empty() {
            continue;
        }
        let event = serde_json::from_str::<CandidateReplayEvent>(trimmed)
            .map_err(|err| anyhow!("line {line_no}: invalid candidate replay event: {err}"))?;
        events.push(event);
    }

    Ok(events)
}

pub fn load_candidate_replay_file(
    path: impl AsRef<Path>,
) -> anyhow::Result<Vec<CandidateReplayEvent>> {
    match path
        .as_ref()
        .extension()
        .and_then(|extension| extension.to_str())
        .map(|extension| extension.to_ascii_lowercase())
        .as_deref()
    {
        Some("jsonl") => load_candidate_replay_jsonl(path),
        Some("csv") => load_candidate_replay_csv(path),
        other => anyhow::bail!(
            "unsupported candidate replay input extension {:?}; expected .jsonl or .csv",
            other
        ),
    }
}

pub fn load_candidate_replay_csv(
    path: impl AsRef<Path>,
) -> anyhow::Result<Vec<CandidateReplayEvent>> {
    let file = File::open(path.as_ref()).with_context(|| {
        format!(
            "failed to open candidate replay csv {}",
            path.as_ref().display()
        )
    })?;
    let mut lines = BufReader::new(file).lines();
    let header = lines
        .next()
        .transpose()
        .context("failed reading candidate replay csv header")?
        .ok_or_else(|| anyhow!("candidate replay csv is empty"))?;
    let headers = split_csv_line(&header)
        .into_iter()
        .map(|header| header.to_ascii_lowercase())
        .collect::<Vec<_>>();
    let mut events = Vec::new();

    for (index, line) in lines.enumerate() {
        let line_no = index + 2;
        let line = line.with_context(|| format!("failed reading csv line {line_no}"))?;
        if line.trim().is_empty() {
            continue;
        }
        let values = split_csv_line(&line);
        let row = headers
            .iter()
            .zip(values.iter())
            .map(|(key, value)| (key.as_str(), value.trim()))
            .collect::<BTreeMap<_, _>>();
        events.push(csv_row_to_event(&row, line_no)?);
    }

    Ok(events)
}

fn csv_row_to_event(
    row: &BTreeMap<&str, &str>,
    line_no: usize,
) -> anyhow::Result<CandidateReplayEvent> {
    Ok(CandidateReplayEvent {
        event_type: parse_event_type(required(row, "type", line_no)?, line_no)?,
        venue: parse_venue(required(row, "venue", line_no)?, line_no)?,
        symbol: required(row, "symbol", line_no)?.to_string(),
        ts_ms: parse_i64(required(row, "tsms", line_no)?, "tsMs", line_no)?,
        side: optional(row, "side")
            .map(|value| parse_side(value, line_no))
            .transpose()?,
        price: optional(row, "price")
            .map(|value| parse_f64(value, "price", line_no))
            .transpose()?,
        qty: optional(row, "qty")
            .map(|value| parse_f64(value, "qty", line_no))
            .transpose()?,
        qty_before: optional(row, "qtybefore")
            .map(|value| parse_f64(value, "qtyBefore", line_no))
            .transpose()?,
        qty_after: optional(row, "qtyafter")
            .map(|value| parse_f64(value, "qtyAfter", line_no))
            .transpose()?,
        sequence: optional(row, "sequence")
            .map(|value| parse_u64(value, "sequence", line_no))
            .transpose()?,
        trade_id: optional(row, "tradeid").map(str::to_string),
        order_id: optional(row, "orderid").map(str::to_string),
        aggressor_side: optional(row, "aggressorside")
            .map(|value| parse_aggressor_side(value, line_no))
            .transpose()?,
    })
}

fn required<'a>(
    row: &'a BTreeMap<&str, &str>,
    key: &str,
    line_no: usize,
) -> anyhow::Result<&'a str> {
    optional(row, key).ok_or_else(|| anyhow!("line {line_no}: missing required csv column {key}"))
}

fn optional<'a>(row: &'a BTreeMap<&str, &str>, key: &str) -> Option<&'a str> {
    row.get(key)
        .copied()
        .map(str::trim)
        .filter(|value| !value.is_empty())
}

fn parse_event_type(raw: &str, line_no: usize) -> anyhow::Result<CandidateReplayEventType> {
    match raw.trim().to_ascii_lowercase().as_str() {
        "trade" => Ok(CandidateReplayEventType::Trade),
        "book_delta" | "bookdelta" => Ok(CandidateReplayEventType::BookDelta),
        "snapshot" => Ok(CandidateReplayEventType::Snapshot),
        "snapshot_reset" | "snapshotreset" => Ok(CandidateReplayEventType::SnapshotReset),
        other => anyhow::bail!("line {line_no}: unsupported event type {other}"),
    }
}

fn parse_venue(raw: &str, line_no: usize) -> anyhow::Result<Venue> {
    match raw.trim().to_ascii_lowercase().as_str() {
        "binance" => Ok(Venue::Binance),
        "bybit" => Ok(Venue::Bybit),
        "okx" => Ok(Venue::Okx),
        "bitfinex" => Ok(Venue::Bitfinex),
        other => anyhow::bail!("line {line_no}: unsupported venue {other}"),
    }
}

fn parse_side(raw: &str, line_no: usize) -> anyhow::Result<OrderbookWallSide> {
    match raw.trim().to_ascii_lowercase().as_str() {
        "bid" | "buy" => Ok(OrderbookWallSide::Bid),
        "ask" | "sell" => Ok(OrderbookWallSide::Ask),
        other => anyhow::bail!("line {line_no}: unsupported side {other}"),
    }
}

fn parse_aggressor_side(raw: &str, line_no: usize) -> anyhow::Result<AggressorSide> {
    match raw.trim().to_ascii_lowercase().as_str() {
        "buy" | "bid" => Ok(AggressorSide::Buy),
        "sell" | "ask" => Ok(AggressorSide::Sell),
        other => anyhow::bail!("line {line_no}: unsupported aggressorSide {other}"),
    }
}

fn parse_f64(raw: &str, field: &str, line_no: usize) -> anyhow::Result<f64> {
    raw.parse::<f64>()
        .with_context(|| format!("line {line_no}: invalid {field} value"))
}

fn parse_i64(raw: &str, field: &str, line_no: usize) -> anyhow::Result<i64> {
    raw.parse::<i64>()
        .with_context(|| format!("line {line_no}: invalid {field} value"))
}

fn parse_u64(raw: &str, field: &str, line_no: usize) -> anyhow::Result<u64> {
    raw.parse::<u64>()
        .with_context(|| format!("line {line_no}: invalid {field} value"))
}

fn split_csv_line(line: &str) -> Vec<String> {
    line.split(',')
        .map(|value| value.trim().trim_matches('"').to_string())
        .collect()
}
