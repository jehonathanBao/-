use std::{
    fs::{create_dir_all, OpenOptions},
    io::Write,
    path::PathBuf,
};

use anyhow::Context;
use chrono::{TimeZone, Utc};
use serde::Serialize;
use serde_json::json;

use crate::types::{
    market::Venue,
    toxic::{ToxicDirection, ToxicEvent, ToxicSeverity, ToxicState},
};

const SIDECAR_SCHEMA_VERSION: &str = "toxic-flow-rs.sidecar.v1";

#[derive(Debug, Clone)]
pub struct ToxicFlowSidecarWriter {
    enabled: bool,
    events_path: Option<PathBuf>,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct ToxicFlowSidecarEvent {
    schema_version: &'static str,
    event_id: String,
    source: &'static str,
    ts: String,
    kind: &'static str,
    severity: &'static str,
    symbol: String,
    venue: Option<String>,
    dedupe_key: String,
    title: String,
    summary: String,
    payload: serde_json::Value,
}

impl ToxicFlowSidecarWriter {
    pub fn disabled() -> Self {
        Self {
            enabled: false,
            events_path: None,
        }
    }

    pub fn new(enabled: bool, events_path: Option<String>) -> Self {
        Self {
            enabled,
            events_path: events_path
                .and_then(|path| (!path.trim().is_empty()).then_some(path))
                .map(PathBuf::from),
        }
    }

    pub fn enabled(&self) -> bool {
        self.enabled && self.events_path.is_some()
    }

    pub fn write_alert(
        &self,
        event: &ToxicEvent,
        state: &ToxicState,
        dedupe_key: &str,
        message: &str,
    ) -> anyhow::Result<()> {
        if !self.enabled() {
            return Ok(());
        }
        let path = self.events_path.as_ref().expect("events path checked");
        if let Some(parent) = path.parent() {
            create_dir_all(parent).with_context(|| {
                format!(
                    "failed to create sidecar event directory {}",
                    parent.display()
                )
            })?;
        }

        let sidecar_event = build_sidecar_event(event, state, dedupe_key, message);
        let line = serde_json::to_string(&sidecar_event)
            .context("failed to serialize toxic flow sidecar event")?;
        let mut file = OpenOptions::new()
            .create(true)
            .append(true)
            .open(path)
            .with_context(|| format!("failed to open sidecar events file {}", path.display()))?;
        writeln!(file, "{line}")
            .with_context(|| format!("failed to append sidecar event to {}", path.display()))?;
        Ok(())
    }

    pub fn write_runtime_acceptance_test(
        &self,
        ts_ms: i64,
        severity: ToxicSeverity,
        venue: Venue,
        symbol: &str,
        dedupe_key: &str,
    ) -> anyhow::Result<()> {
        if !self.enabled() {
            return Ok(());
        }
        let path = self.events_path.as_ref().expect("events path checked");
        if let Some(parent) = path.parent() {
            create_dir_all(parent).with_context(|| {
                format!(
                    "failed to create sidecar event directory {}",
                    parent.display()
                )
            })?;
        }

        let sidecar_event = ToxicFlowSidecarEvent {
            schema_version: SIDECAR_SCHEMA_VERSION,
            event_id: format!("runtime-acceptance-test-{ts_ms}"),
            source: "toxic-flow-rs",
            ts: iso_ts(ts_ms),
            kind: "runtime_acceptance_test",
            severity: sidecar_severity(severity),
            symbol: symbol.to_string(),
            venue: Some(venue_key(venue)),
            dedupe_key: dedupe_key.to_string(),
            title: "Runtime acceptance test alert".to_string(),
            summary: "This is a monitor-generated sidecar test alert.".to_string(),
            payload: json!({
                "readOnly": true,
                "test": true,
                "generatedBy": "monitor_dev_test_alert_endpoint",
                "requestedSeverity": severity.label(),
                "venue": venue.as_key(),
                "symbol": symbol,
            }),
        };
        let line = serde_json::to_string(&sidecar_event)
            .context("failed to serialize runtime acceptance sidecar event")?;
        let mut file = OpenOptions::new()
            .create(true)
            .append(true)
            .open(path)
            .with_context(|| format!("failed to open sidecar events file {}", path.display()))?;
        writeln!(file, "{line}")
            .with_context(|| format!("failed to append sidecar event to {}", path.display()))?;
        Ok(())
    }
}

fn build_sidecar_event(
    event: &ToxicEvent,
    state: &ToxicState,
    dedupe_key: &str,
    message: &str,
) -> ToxicFlowSidecarEvent {
    ToxicFlowSidecarEvent {
        schema_version: SIDECAR_SCHEMA_VERSION,
        event_id: event.id.clone(),
        source: "toxic-flow-rs",
        ts: iso_ts(event.ts),
        kind: "toxic_flow_spike",
        severity: sidecar_severity(event.severity),
        symbol: event.symbol.clone(),
        venue: event.leader_venue.map(venue_key),
        dedupe_key: dedupe_key.to_string(),
        title: format!(
            "{} {} toxic flow {}",
            event.symbol,
            direction_label(event.direction),
            event.severity.label()
        ),
        summary: first_summary_line(message).unwrap_or_else(|| {
            format!(
                "{} {} toxic flow: {:.2} BTC over {}s",
                event.symbol,
                direction_label(event.direction),
                event.toxic_volume_btc,
                event.window_ms / 1000
            )
        }),
        payload: json!({
            "readOnly": true,
            "direction": direction_label(event.direction),
            "toxicVolumeBtc": event.toxic_volume_btc,
            "thresholdBtc": event.threshold_btc,
            "windowMs": event.window_ms,
            "leaderVenue": event.leader_venue.map(venue_key),
            "crossVenueConfirmed": event.cross_venue_confirmed,
            "markout1sBps": event.markout_1s_bps,
            "markout5sBps": event.markout_5s_bps,
            "sweepDetected": event.sweep_detected,
            "liquidityThin": event.liquidity_thin,
            "vpin": event.vpin,
            "vpinZscore": event.vpin_zscore,
            "possibleLiqHuntSetup": event.possible_liq_hunt_setup,
            "activeVenueCount": state.quality.active_venues.len(),
            "reasonCodes": event.reason_codes,
        }),
    }
}

fn iso_ts(ts_ms: i64) -> String {
    Utc.timestamp_millis_opt(ts_ms)
        .single()
        .unwrap_or_else(Utc::now)
        .to_rfc3339()
}

fn sidecar_severity(severity: ToxicSeverity) -> &'static str {
    match severity {
        ToxicSeverity::Extreme | ToxicSeverity::Alert => "critical",
        ToxicSeverity::Warning => "warning",
        ToxicSeverity::Watch | ToxicSeverity::Normal => "info",
    }
}

fn direction_label(direction: ToxicDirection) -> &'static str {
    match direction {
        ToxicDirection::Buy => "buy",
        ToxicDirection::Sell => "sell",
        ToxicDirection::Neutral => "neutral",
    }
}

fn venue_key(venue: Venue) -> String {
    venue.as_key().to_string()
}

fn first_summary_line(message: &str) -> Option<String> {
    message
        .lines()
        .find(|line| !line.trim().is_empty())
        .map(|line| line.trim().to_string())
}
