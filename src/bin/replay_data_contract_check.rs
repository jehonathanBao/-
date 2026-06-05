use anyhow::{anyhow, bail, Context};
use chrono::Utc;
use serde_json::{json, Map, Value};
use std::{
    env, fs,
    path::{Path, PathBuf},
};

const REQUIRED_FIELDS: &[&str] = &[
    "event_type",
    "venue",
    "symbol",
    "ts_ms",
    "side",
    "price",
    "qty",
    "qty_before",
    "qty_after",
    "sequence",
    "trade_id",
];

#[derive(Default)]
struct ContractStats {
    venue: Option<String>,
    symbol: Option<String>,
    start_ts_ms: Option<i64>,
    end_ts_ms: Option<i64>,
    event_count: usize,
    trade_count: usize,
    book_delta_count: usize,
    snapshot_count: usize,
    snapshot_reset_count: usize,
    order_id_count: usize,
    warnings: Vec<String>,
}

fn main() -> anyhow::Result<()> {
    let input = parse_input_arg(env::args().skip(1))?;
    let stats = check_contract(&input)?;
    let run_dir = write_reports(&input, &stats)?;

    println!("[PASS] Data Contract Check");
    println!("[PASS] Data Summary Written: {}", run_dir.display());
    if stats.order_id_count < stats.event_count {
        println!("[WARN] order_id missing means Candidate only");
    }
    Ok(())
}

fn parse_input_arg(mut args: impl Iterator<Item = String>) -> anyhow::Result<PathBuf> {
    while let Some(arg) = args.next() {
        if arg == "--input" {
            return args
                .next()
                .map(PathBuf::from)
                .ok_or_else(|| anyhow!("--input requires a file path"));
        }
    }

    bail!("usage: cargo run --bin replay_data_contract_check -- --input <path>");
}

fn check_contract(input: &Path) -> anyhow::Result<ContractStats> {
    if !input.exists() {
        bail!("input file does not exist: {}", input.display());
    }

    let extension = input
        .extension()
        .and_then(|value| value.to_str())
        .unwrap_or_default()
        .to_ascii_lowercase();

    let rows = match extension.as_str() {
        "jsonl" => read_jsonl_rows(input)?,
        "csv" => read_csv_rows(input)?,
        other => bail!("unsupported replay data extension: {other}; expected .jsonl or .csv"),
    };

    if rows.is_empty() {
        bail!("input file contains no replay events: {}", input.display());
    }

    let mut stats = ContractStats::default();
    for (index, row) in rows.iter().enumerate() {
        validate_row(index + 1, row, &mut stats)?;
    }

    Ok(stats)
}

fn read_jsonl_rows(input: &Path) -> anyhow::Result<Vec<Map<String, Value>>> {
    let text =
        fs::read_to_string(input).with_context(|| format!("failed to read {}", input.display()))?;
    let mut rows = Vec::new();
    for (index, line) in text.lines().enumerate() {
        let trimmed = line.trim().trim_start_matches('\u{feff}');
        if trimmed.is_empty() {
            continue;
        }
        let value: Value = serde_json::from_str(trimmed)
            .with_context(|| format!("invalid jsonl at line {}", index + 1))?;
        let object = value
            .as_object()
            .cloned()
            .ok_or_else(|| anyhow!("jsonl line {} must be an object", index + 1))?;
        rows.push(object);
    }
    Ok(rows)
}

fn read_csv_rows(input: &Path) -> anyhow::Result<Vec<Map<String, Value>>> {
    let text =
        fs::read_to_string(input).with_context(|| format!("failed to read {}", input.display()))?;
    let mut lines = text.lines().filter(|line| !line.trim().is_empty());
    let header_line = lines
        .next()
        .ok_or_else(|| anyhow!("csv file has no header: {}", input.display()))?;
    let headers = split_csv_line(header_line.trim_start_matches('\u{feff}'));
    let mut rows = Vec::new();

    for (index, line) in lines.enumerate() {
        let values = split_csv_line(line);
        if values.len() != headers.len() {
            bail!(
                "csv line {} has {} fields, expected {}",
                index + 2,
                values.len(),
                headers.len()
            );
        }
        let mut object = Map::new();
        for (header, value) in headers.iter().zip(values.iter()) {
            object.insert(header.clone(), Value::String(value.clone()));
        }
        rows.push(object);
    }

    Ok(rows)
}

fn split_csv_line(line: &str) -> Vec<String> {
    line.split(',')
        .map(|value| value.trim().trim_matches('"').to_string())
        .collect()
}

fn validate_row(
    line_no: usize,
    row: &Map<String, Value>,
    stats: &mut ContractStats,
) -> anyhow::Result<()> {
    for field in REQUIRED_FIELDS {
        if !has_non_empty_value(row, field) {
            bail!("line {line_no} missing required field `{field}`");
        }
    }

    let event_type = field_as_string(row, "event_type")?.to_ascii_lowercase();
    match event_type.as_str() {
        "trade" => stats.trade_count += 1,
        "book_delta" | "bookdelta" => stats.book_delta_count += 1,
        "snapshot" => stats.snapshot_count += 1,
        "snapshot_reset" | "snapshotreset" => stats.snapshot_reset_count += 1,
        other => bail!("line {line_no} invalid event_type `{other}`"),
    }

    let side = field_as_string(row, "side")?;
    normalize_side(&side).ok_or_else(|| anyhow!("line {line_no} invalid side `{side}`"))?;

    let ts_ms = parse_i64_field(row, "ts_ms", line_no)?;
    if !(1_000_000_000_000..=4_102_444_800_000).contains(&ts_ms) {
        bail!("line {line_no} ts_ms does not look like UTC milliseconds: {ts_ms}");
    }
    stats.start_ts_ms = Some(
        stats
            .start_ts_ms
            .map_or(ts_ms, |current| current.min(ts_ms)),
    );
    stats.end_ts_ms = Some(stats.end_ts_ms.map_or(ts_ms, |current| current.max(ts_ms)));

    parse_f64_field(row, "price", line_no)?;
    parse_f64_field(row, "qty", line_no)?;
    parse_f64_field(row, "qty_before", line_no)?;
    parse_f64_field(row, "qty_after", line_no)?;
    parse_i64_field(row, "sequence", line_no)?;

    let venue = field_as_string(row, "venue")?;
    let symbol = field_as_string(row, "symbol")?;
    check_consistent_value(&mut stats.venue, &venue, "venue", line_no)?;
    check_consistent_value(&mut stats.symbol, &symbol, "symbol", line_no)?;

    stats.event_count += 1;
    if has_non_empty_value(row, "order_id") {
        stats.order_id_count += 1;
    }

    Ok(())
}

fn has_non_empty_value(row: &Map<String, Value>, field: &str) -> bool {
    row.get(field)
        .and_then(value_to_string)
        .map(|value| !value.trim().is_empty())
        .unwrap_or(false)
}

fn field_as_string(row: &Map<String, Value>, field: &str) -> anyhow::Result<String> {
    row.get(field)
        .and_then(value_to_string)
        .map(|value| value.trim().to_string())
        .ok_or_else(|| anyhow!("missing field `{field}`"))
}

fn value_to_string(value: &Value) -> Option<String> {
    match value {
        Value::String(value) => Some(value.clone()),
        Value::Number(value) => Some(value.to_string()),
        Value::Bool(value) => Some(value.to_string()),
        _ => None,
    }
}

fn normalize_side(side: &str) -> Option<&'static str> {
    match side.trim().to_ascii_lowercase().as_str() {
        "bid" | "buy" | "b" => Some("bid"),
        "ask" | "sell" | "a" => Some("ask"),
        "none" | "neutral" | "n/a" | "na" | "-" => Some("none"),
        _ => None,
    }
}

fn parse_i64_field(row: &Map<String, Value>, field: &str, line_no: usize) -> anyhow::Result<i64> {
    let raw = field_as_string(row, field)?;
    raw.parse::<i64>()
        .with_context(|| format!("line {line_no} field `{field}` is not an integer: {raw}"))
}

fn parse_f64_field(row: &Map<String, Value>, field: &str, line_no: usize) -> anyhow::Result<f64> {
    let raw = field_as_string(row, field)?;
    raw.parse::<f64>()
        .with_context(|| format!("line {line_no} field `{field}` is not numeric: {raw}"))
}

fn check_consistent_value(
    current: &mut Option<String>,
    next: &str,
    label: &str,
    line_no: usize,
) -> anyhow::Result<()> {
    match current {
        Some(value) if value != next => {
            bail!("line {line_no} has {label} `{next}`, expected `{value}`")
        }
        Some(_) => Ok(()),
        None => {
            *current = Some(next.to_string());
            Ok(())
        }
    }
}

fn write_reports(input: &Path, stats: &ContractStats) -> anyhow::Result<PathBuf> {
    let run_id = format!(
        "data_contract_{}_{}_{}",
        stats.venue.as_deref().unwrap_or("unknown"),
        stats.symbol.as_deref().unwrap_or("unknown"),
        Utc::now().format("%Y%m%dT%H%M%SZ")
    );
    let run_dir = PathBuf::from("data")
        .join("production_replay")
        .join("reports")
        .join(sanitize_path_component(&run_id));
    fs::create_dir_all(&run_dir)?;

    let order_id_ratio = if stats.event_count == 0 {
        0.0
    } else {
        stats.order_id_count as f64 / stats.event_count as f64
    };

    let summary = json!({
        "input": input.display().to_string(),
        "venue": stats.venue.as_deref().unwrap_or("N/A"),
        "symbol": stats.symbol.as_deref().unwrap_or("N/A"),
        "start_ts_ms": stats.start_ts_ms,
        "end_ts_ms": stats.end_ts_ms,
        "event_count": stats.event_count,
        "trade_count": stats.trade_count,
        "book_delta_count": stats.book_delta_count,
        "snapshot_count": stats.snapshot_count,
        "snapshot_reset_count": stats.snapshot_reset_count,
        "has_order_id_ratio": order_id_ratio,
        "snapshot_reset_was_not_cancel_evidence": true,
        "warnings": build_warnings(stats),
    });
    fs::write(
        run_dir.join("data_contract.json"),
        serde_json::to_string_pretty(&summary)?,
    )?;
    fs::write(
        run_dir.join("data_contract.md"),
        render_markdown(stats, order_id_ratio),
    )?;

    Ok(run_dir)
}

fn build_warnings(stats: &ContractStats) -> Vec<String> {
    let mut warnings = stats.warnings.clone();
    if stats.order_id_count < stats.event_count {
        warnings
            .push("order_id missing on one or more events; results remain Candidate only".into());
    }
    warnings
}

fn render_markdown(stats: &ContractStats, order_id_ratio: f64) -> String {
    format!(
        r#"# Replay Data Contract

## Summary

- Venue: {}
- Symbol: {}
- Start ts ms: {}
- End ts ms: {}
- Event count: {}
- Trade count: {}
- Book delta count: {}
- Snapshot count: {}
- Snapshot reset count: {}
- Has order id ratio: {:.4}

## Safety

- SnapshotReset is counted separately and is not treated as cancel evidence.
- Missing order_id is a warning, not a failure; replay results remain Candidate only.
"#,
        stats.venue.as_deref().unwrap_or("N/A"),
        stats.symbol.as_deref().unwrap_or("N/A"),
        stats
            .start_ts_ms
            .map(|value| value.to_string())
            .unwrap_or_else(|| "N/A".to_string()),
        stats
            .end_ts_ms
            .map(|value| value.to_string())
            .unwrap_or_else(|| "N/A".to_string()),
        stats.event_count,
        stats.trade_count,
        stats.book_delta_count,
        stats.snapshot_count,
        stats.snapshot_reset_count,
        order_id_ratio,
    )
}

fn sanitize_path_component(value: &str) -> String {
    value
        .chars()
        .map(|ch| {
            if ch.is_ascii_alphanumeric() || ch == '_' || ch == '-' {
                ch
            } else {
                '_'
            }
        })
        .collect()
}
