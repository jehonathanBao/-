use rusqlite::{params, Transaction};
use serde_json::Value;

use crate::types::{
    flow::FlowState,
    status::VenueHealthMap,
    toxic::{ToxicDirection, ToxicSeverity, ToxicState},
};

use super::sqlite::SqliteStore;

pub trait SnapshotsRepo {
    fn insert_toxic_snapshot(&self, state: &ToxicState) -> anyhow::Result<()>;
    fn insert_flow_snapshot(&self, state: &FlowState) -> anyhow::Result<()>;
    fn insert_runtime_snapshots(
        &self,
        ts: i64,
        flow: &FlowState,
        toxic: &ToxicState,
        venue_health: &VenueHealthMap,
    ) -> anyhow::Result<()>;
    fn list_toxic_snapshots(
        &self,
        since_ts: i64,
        until_ts: i64,
        limit: usize,
    ) -> anyhow::Result<Vec<Value>>;
}

impl SnapshotsRepo for SqliteStore {
    fn insert_toxic_snapshot(&self, state: &ToxicState) -> anyhow::Result<()> {
        self.with_write_connection(|conn| {
            let tx = conn.unchecked_transaction()?;
            insert_toxic_snapshot_tx(&tx, state)?;
            tx.commit()?;
            Ok(())
        })
    }

    fn insert_flow_snapshot(&self, state: &FlowState) -> anyhow::Result<()> {
        self.with_write_connection(|conn| {
            let tx = conn.unchecked_transaction()?;
            insert_flow_snapshot_tx(&tx, state)?;
            tx.commit()?;
            Ok(())
        })
    }

    fn insert_runtime_snapshots(
        &self,
        ts: i64,
        flow: &FlowState,
        toxic: &ToxicState,
        venue_health: &VenueHealthMap,
    ) -> anyhow::Result<()> {
        self.with_transaction(|tx| {
            insert_flow_snapshot_tx(tx, flow)?;
            insert_toxic_snapshot_tx(tx, toxic)?;
            for venue in venue_health.values() {
                tx.execute(
                    r#"
                    INSERT INTO venue_health_snapshots (
                      ts, venue, enabled, status, last_trade_ts, last_book_ts, last_message_ts,
                      reconnect_count, last_error
                    ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9)
                    "#,
                    params![
                        ts,
                        venue.venue.as_key(),
                        bool_to_int(venue.enabled),
                        serde_json::to_string(&venue.status)?,
                        venue.last_trade_ts,
                        venue.last_book_ts,
                        venue.last_message_ts,
                        venue.reconnect_count as i64,
                        venue.last_error,
                    ],
                )?;
            }
            Ok(())
        })
    }

    fn list_toxic_snapshots(
        &self,
        since_ts: i64,
        until_ts: i64,
        limit: usize,
    ) -> anyhow::Result<Vec<Value>> {
        self.with_connection(|conn| {
            let mut stmt = conn.prepare(
                r#"
                SELECT payload_json FROM toxic_snapshots
                WHERE ts >= ?1 AND ts <= ?2
                ORDER BY ts DESC
                LIMIT ?3
                "#,
            )?;
            let rows = stmt.query_map(params![since_ts, until_ts, limit as i64], |row| {
                row.get::<_, String>(0)
            })?;
            let mut snapshots = Vec::new();
            for row in rows {
                snapshots.push(serde_json::from_str::<Value>(&row?)?);
            }
            Ok(snapshots)
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

fn insert_flow_snapshot_tx(tx: &Transaction<'_>, state: &FlowState) -> anyhow::Result<()> {
    for window in state.windows.values() {
        let payload_json = serde_json::json!({
            "symbol": state.symbol,
            "updatedAt": state.updated_at,
            "window": window,
        })
        .to_string();
        tx.execute(
            r#"
            INSERT INTO flow_snapshots (
              ts, symbol, window_ms, aggressive_buy_btc, aggressive_sell_btc,
              net_aggressive_btc, abs_aggressive_btc, price_move_bps, payload_json
            ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9)
            "#,
            params![
                state.updated_at,
                state.symbol,
                window.window_ms as i64,
                window.aggressive_buy_btc,
                window.aggressive_sell_btc,
                window.net_aggressive_btc,
                window.abs_aggressive_btc,
                window.price_move_bps,
                payload_json,
            ],
        )?;
    }
    Ok(())
}

fn insert_toxic_snapshot_tx(tx: &Transaction<'_>, state: &ToxicState) -> anyhow::Result<()> {
    let best = state
        .results
        .values()
        .max_by(|left, right| left.toxic_volume_btc.total_cmp(&right.toxic_volume_btc));
    let (max_toxic_volume_btc, max_toxic_ratio, max_window_ms, direction, severity, alert) =
        if let Some(result) = best {
            (
                result.toxic_volume_btc,
                result.toxic_ratio,
                result.window_ms as i64,
                result.direction,
                result.severity,
                result.alert_triggered,
            )
        } else {
            (
                0.0,
                0.0,
                0_i64,
                ToxicDirection::Neutral,
                ToxicSeverity::Normal,
                false,
            )
        };
    // Persist only the selected window and event summary. The previous code
    // serialized every result, recent event, and quality detail on every
    // interval, even though the durable query surface is scalar.
    let payload_json = serde_json::json!({
        "symbol": state.symbol,
        "updatedAt": state.updated_at,
        "thresholdBtc": state.threshold_btc,
        "maxToxicVolumeBtc": max_toxic_volume_btc,
        "maxToxicRatio": max_toxic_ratio,
        "maxWindowMs": max_window_ms,
        "direction": direction,
        "severity": severity,
        "alertTriggered": alert,
        "latestEvent": state.latest_event,
    })
    .to_string();
    tx.execute(
        r#"
        INSERT INTO toxic_snapshots (
          ts, symbol, max_toxic_volume_btc, max_toxic_ratio, max_window_ms, direction,
          severity, threshold_btc, alert_triggered, payload_json
        ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10)
        "#,
        params![
            state.updated_at,
            state.symbol,
            max_toxic_volume_btc,
            max_toxic_ratio,
            max_window_ms,
            serde_json::to_string(&direction)?,
            serde_json::to_string(&severity)?,
            state.threshold_btc,
            bool_to_int(alert),
            payload_json,
        ],
    )?;
    Ok(())
}
