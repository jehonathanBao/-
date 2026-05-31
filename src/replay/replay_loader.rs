use std::{
    fs::File,
    io::{BufRead, BufReader},
    path::Path,
};

use anyhow::{anyhow, Context};
use serde_json::Value;

use super::replay_types::{
    ReplayBookRecord, ReplayEvent, ReplayExpectToxicRecord, ReplayTradeRecord,
};

pub fn load_jsonl(path: impl AsRef<Path>) -> anyhow::Result<Vec<ReplayEvent>> {
    let file = File::open(path.as_ref())
        .with_context(|| format!("failed to open replay file {}", path.as_ref().display()))?;
    let reader = BufReader::new(file);
    let mut events = Vec::new();

    for (index, line) in reader.lines().enumerate() {
        let line_no = index + 1;
        let line = line.with_context(|| format!("failed reading line {line_no}"))?;
        let trimmed = line.trim();
        if trimmed.is_empty() {
            continue;
        }
        let value: Value = serde_json::from_str(trimmed)
            .map_err(|err| anyhow!("line {line_no}: invalid json: {err}"))?;
        let kind = value
            .get("type")
            .and_then(Value::as_str)
            .ok_or_else(|| anyhow!("line {line_no}: missing type"))?;
        let event = match kind {
            "trade" => ReplayEvent::Trade(
                serde_json::from_value::<ReplayTradeRecord>(value)
                    .map_err(|err| anyhow!("line {line_no}: invalid trade: {err}"))?,
            ),
            "book" => ReplayEvent::Book(
                serde_json::from_value::<ReplayBookRecord>(value)
                    .map_err(|err| anyhow!("line {line_no}: invalid book: {err}"))?,
            ),
            "expect_toxic" => ReplayEvent::ExpectToxic(
                serde_json::from_value::<ReplayExpectToxicRecord>(value)
                    .map_err(|err| anyhow!("line {line_no}: invalid expect_toxic: {err}"))?,
            ),
            other => return Err(anyhow!("line {line_no}: unknown type {other}")),
        };
        events.push(event);
    }

    Ok(events)
}
