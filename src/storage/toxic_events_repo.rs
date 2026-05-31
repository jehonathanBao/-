use anyhow::Context;
use rusqlite::{params, OptionalExtension};

use crate::types::toxic::ToxicEvent;

use super::sqlite::SqliteStore;

pub trait ToxicEventsRepo {
    fn insert_event(&self, event: &ToxicEvent) -> anyhow::Result<()>;
    fn list_recent_events(&self, limit: usize) -> anyhow::Result<Vec<ToxicEvent>>;
    fn get_latest_event(&self) -> anyhow::Result<Option<ToxicEvent>>;
}

impl ToxicEventsRepo for SqliteStore {
    fn insert_event(&self, event: &ToxicEvent) -> anyhow::Result<()> {
        let reason_codes_json = serde_json::to_string(&event.reason_codes)?;
        let payload_json = serde_json::to_string(event)?;
        self.with_connection(|conn| {
            conn.execute(
                r#"
                INSERT OR IGNORE INTO toxic_events (
                  id, ts, symbol, direction, severity, toxic_volume_btc, toxic_ratio,
                  threshold_btc, window_ms, leader_venue, aggressive_buy_btc, aggressive_sell_btc,
                  net_aggressive_btc, abs_aggressive_btc, markout_1s_bps, markout_5s_bps,
                  sweep_detected, liquidity_thin, cross_venue_confirmed, reason_codes_json,
                  payload_json, created_at
                ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13, ?14, ?15, ?16, ?17, ?18, ?19, ?20, ?21, ?22)
                "#,
                params![
                    event.id,
                    event.ts,
                    event.symbol,
                    serde_json::to_string(&event.direction)?,
                    serde_json::to_string(&event.severity)?,
                    event.toxic_volume_btc,
                    event
                        .toxic_volume_btc
                        / if event.abs_aggressive_btc > 0.0 {
                            event.abs_aggressive_btc
                        } else {
                            1.0
                        },
                    event.threshold_btc,
                    event.window_ms as i64,
                    event.leader_venue.map(|venue| venue.as_key().to_string()),
                    event.aggressive_buy_btc,
                    event.aggressive_sell_btc,
                    event.net_aggressive_btc,
                    event.abs_aggressive_btc,
                    event.markout_1s_bps,
                    event.markout_5s_bps,
                    bool_to_int(event.sweep_detected),
                    bool_to_int(event.liquidity_thin),
                    bool_to_int(event.cross_venue_confirmed),
                    reason_codes_json,
                    payload_json,
                    event.ts,
                ],
            )
            .context("failed to insert toxic event")?;
            Ok(())
        })
    }

    fn list_recent_events(&self, limit: usize) -> anyhow::Result<Vec<ToxicEvent>> {
        self.with_connection(|conn| {
            let mut stmt =
                conn.prepare("SELECT payload_json FROM toxic_events ORDER BY ts DESC LIMIT ?1")?;
            let rows = stmt.query_map([limit as i64], |row| row.get::<_, String>(0))?;
            let mut events = Vec::new();
            for row in rows {
                let payload = row?;
                events.push(serde_json::from_str::<ToxicEvent>(&payload)?);
            }
            Ok(events)
        })
    }

    fn get_latest_event(&self) -> anyhow::Result<Option<ToxicEvent>> {
        self.with_connection(|conn| {
            let payload = conn
                .query_row(
                    "SELECT payload_json FROM toxic_events ORDER BY ts DESC LIMIT 1",
                    [],
                    |row| row.get::<_, String>(0),
                )
                .optional()?;
            payload
                .map(|json| {
                    serde_json::from_str::<ToxicEvent>(&json)
                        .context("failed to decode toxic event")
                })
                .transpose()
        })
    }
}

fn bool_to_int(value: bool) -> i64 {
    if value {
        1
    } else {
        0
    }
}
