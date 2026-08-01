use anyhow::Context;
use rusqlite::{params, OptionalExtension};

use crate::contract_whale_monitor::hourly_delta_alert::types::{
    HourlyDeltaAlertRecord, HourlyDeltaDataStatus, HourlyDeltaDirection,
    HourlyDeltaDiscordOutboxItem, HourlyDeltaDiscordOutboxStats, HourlyDeltaDiscordStatus,
    HourlyDeltaResult,
};

use super::sqlite::SqliteStore;

const DISCORD_OUTBOX_LEASE_MS: i64 = 120_000;

pub trait HourlyDeltaRepo {
    fn upsert_hourly_delta_closed_result(
        &self,
        result: &HourlyDeltaResult,
        now_ms: i64,
    ) -> anyhow::Result<bool>;

    fn upsert_hourly_delta_closed_result_with_outbox(
        &self,
        result: &HourlyDeltaResult,
        now_ms: i64,
        enqueue_discord: bool,
    ) -> anyhow::Result<bool>;

    fn get_hourly_delta_record(
        &self,
        record_key: &str,
    ) -> anyhow::Result<Option<HourlyDeltaAlertRecord>>;

    fn enqueue_hourly_delta_discord_outbox(
        &self,
        record_key: &str,
        now_ms: i64,
    ) -> anyhow::Result<bool>;

    fn claim_hourly_delta_discord_outbox(
        &self,
        limit: usize,
        now_ms: i64,
    ) -> anyhow::Result<Vec<HourlyDeltaDiscordOutboxItem>>;

    fn finish_hourly_delta_discord_outbox(
        &self,
        record_key: &str,
        status: HourlyDeltaDiscordStatus,
        next_attempt_at: Option<i64>,
        sent_at: Option<i64>,
        last_error: Option<&str>,
    ) -> anyhow::Result<usize>;

    fn hourly_delta_discord_outbox_stats(
        &self,
        now_ms: i64,
    ) -> anyhow::Result<HourlyDeltaDiscordOutboxStats>;
}

impl HourlyDeltaRepo for SqliteStore {
    fn upsert_hourly_delta_closed_result(
        &self,
        result: &HourlyDeltaResult,
        now_ms: i64,
    ) -> anyhow::Result<bool> {
        self.with_connection(|conn| {
            let existing: Option<String> = conn
                .query_row(
                    "SELECT discord_status FROM hourly_delta_alert_records WHERE record_key = ?1",
                    params![result.record_key],
                    |row| row.get(0),
                )
                .optional()?;
            if existing.is_some() {
                return Ok(false);
            }

            let payload_json = serde_json::to_string(result)?;
            let changed = conn.execute(
                r#"
                INSERT INTO hourly_delta_alert_records (
                  record_key, exchange, symbol, interval,
                  kline_open_time_ms, kline_close_time_ms,
                  taker_buy_btc, taker_sell_btc, delta_btc, volume_btc,
                  direction, above_threshold, data_status, discord_status,
                  discord_sent_at_ms, attempts, last_error, payload_json,
                  created_at_ms, updated_at_ms
                ) VALUES (
                  ?1, ?2, ?3, ?4,
                  ?5, ?6,
                  ?7, ?8, ?9, ?10,
                  ?11, ?12, ?13, ?14,
                  NULL, 0, NULL, ?15,
                  ?16, ?17
                )
                "#,
                params![
                    result.record_key,
                    result.exchange,
                    result.symbol,
                    result.interval,
                    result.kline_open_time_ms,
                    result.kline_close_time_ms,
                    result.taker_buy_btc,
                    result.taker_sell_btc,
                    result.delta_btc,
                    result.volume_btc,
                    result.direction.as_str(),
                    bool_to_int(result.above_threshold),
                    result.data_status.as_str(),
                    HourlyDeltaDiscordStatus::None.as_str(),
                    payload_json,
                    now_ms,
                    now_ms,
                ],
            )?;
            Ok(changed == 1)
        })
    }

    fn upsert_hourly_delta_closed_result_with_outbox(
        &self,
        result: &HourlyDeltaResult,
        now_ms: i64,
        enqueue_discord: bool,
    ) -> anyhow::Result<bool> {
        self.with_connection(|conn| {
            let tx = conn.unchecked_transaction()?;
            let existing: Option<String> = tx
                .query_row(
                    "SELECT discord_status FROM hourly_delta_alert_records WHERE record_key = ?1",
                    params![result.record_key],
                    |row| row.get(0),
                )
                .optional()?;
            if existing.is_some() {
                return Ok(false);
            }

            let payload_json = serde_json::to_string(result)?;
            tx.execute(
                r#"
                INSERT INTO hourly_delta_alert_records (
                  record_key, exchange, symbol, interval,
                  kline_open_time_ms, kline_close_time_ms,
                  taker_buy_btc, taker_sell_btc, delta_btc, volume_btc,
                  direction, above_threshold, data_status, discord_status,
                  discord_sent_at_ms, attempts, last_error, payload_json,
                  created_at_ms, updated_at_ms
                ) VALUES (
                  ?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10,
                  ?11, ?12, ?13, ?14, NULL, 0, NULL, ?15, ?16, ?17
                )
                "#,
                params![
                    result.record_key,
                    result.exchange,
                    result.symbol,
                    result.interval,
                    result.kline_open_time_ms,
                    result.kline_close_time_ms,
                    result.taker_buy_btc,
                    result.taker_sell_btc,
                    result.delta_btc,
                    result.volume_btc,
                    result.direction.as_str(),
                    bool_to_int(result.above_threshold),
                    result.data_status.as_str(),
                    HourlyDeltaDiscordStatus::None.as_str(),
                    payload_json,
                    now_ms,
                    now_ms,
                ],
            )?;

            if enqueue_discord && result.above_threshold {
                tx.execute(
                    r#"
                    INSERT INTO hourly_delta_discord_outbox (
                      record_key, symbol, payload_json, status, attempts, next_attempt_at, created_at
                    ) VALUES (?1, ?2, ?3, 'pending', 0, ?4, ?5)
                    ON CONFLICT(record_key) DO NOTHING
                    "#,
                    params![result.record_key, result.symbol, payload_json, now_ms, now_ms],
                )?;
                tx.execute(
                    r#"
                    UPDATE hourly_delta_alert_records
                    SET discord_status = 'pending', updated_at_ms = ?2
                    WHERE record_key = ?1
                    "#,
                    params![result.record_key, now_ms],
                )?;
            }
            tx.commit()?;
            Ok(true)
        })
    }

    fn get_hourly_delta_record(
        &self,
        record_key: &str,
    ) -> anyhow::Result<Option<HourlyDeltaAlertRecord>> {
        self.with_connection(|conn| {
            conn.query_row(
                r#"
                SELECT
                  record_key, exchange, symbol, interval,
                  kline_open_time_ms, kline_close_time_ms,
                  taker_buy_btc, taker_sell_btc, delta_btc, volume_btc,
                  direction, above_threshold, data_status, discord_status,
                  discord_sent_at_ms, attempts, last_error, payload_json,
                  created_at_ms, updated_at_ms
                FROM hourly_delta_alert_records
                WHERE record_key = ?1
                "#,
                params![record_key],
                map_record_row,
            )
            .optional()
            .context("failed to load hourly delta record")
        })
    }

    fn enqueue_hourly_delta_discord_outbox(
        &self,
        record_key: &str,
        now_ms: i64,
    ) -> anyhow::Result<bool> {
        self.with_connection(|conn| {
            let tx = conn.unchecked_transaction()?;
            let record = tx
                .query_row(
                    r#"
                    SELECT
                      record_key, exchange, symbol, interval,
                      kline_open_time_ms, kline_close_time_ms,
                      taker_buy_btc, taker_sell_btc, delta_btc, volume_btc,
                      direction, above_threshold, data_status, discord_status,
                      discord_sent_at_ms, attempts, last_error, payload_json,
                      created_at_ms, updated_at_ms
                    FROM hourly_delta_alert_records
                    WHERE record_key = ?1
                    "#,
                    params![record_key],
                    map_record_row,
                )
                .optional()?;
            let Some(record) = record else {
                return Ok(false);
            };
            if !record.above_threshold {
                return Ok(false);
            }
            if matches!(
                record.discord_status,
                HourlyDeltaDiscordStatus::Sent | HourlyDeltaDiscordStatus::DryRun
            ) {
                return Ok(false);
            }

            let payload_json = serde_json::to_string(&record)?;
            let inserted = tx.execute(
                r#"
                INSERT INTO hourly_delta_discord_outbox (
                  record_key, symbol, payload_json, status, attempts, next_attempt_at, created_at
                ) VALUES (?1, ?2, ?3, 'pending', 0, ?4, ?5)
                ON CONFLICT(record_key) DO NOTHING
                "#,
                params![
                    record.record_key,
                    record.symbol,
                    payload_json,
                    now_ms,
                    now_ms
                ],
            )?;
            if inserted == 1 {
                tx.execute(
                    r#"
                    UPDATE hourly_delta_alert_records
                    SET discord_status = 'pending', updated_at_ms = ?2
                    WHERE record_key = ?1
                      AND discord_status NOT IN ('sent', 'dry_run')
                    "#,
                    params![record_key, now_ms],
                )?;
            }
            tx.commit()?;
            Ok(inserted == 1)
        })
    }

    fn claim_hourly_delta_discord_outbox(
        &self,
        limit: usize,
        now_ms: i64,
    ) -> anyhow::Result<Vec<HourlyDeltaDiscordOutboxItem>> {
        let limit = limit.clamp(1, 100) as i64;
        self.with_connection(|conn| {
            let tx = conn.unchecked_transaction()?;
            let mut stmt = tx.prepare(
                r#"
                SELECT record_key, payload_json, attempts
                FROM hourly_delta_discord_outbox
                WHERE (
                    (status IN ('pending', 'retry') AND (next_attempt_at IS NULL OR next_attempt_at <= ?1))
                    OR (status = 'sending' AND next_attempt_at IS NOT NULL AND next_attempt_at <= ?1)
                )
                ORDER BY created_at ASC, id ASC
                LIMIT ?2
                "#,
            )?;
            let rows = stmt.query_map(params![now_ms, limit], |row| {
                Ok((
                    row.get::<_, String>(0)?,
                    row.get::<_, String>(1)?,
                    row.get::<_, i64>(2)?,
                ))
            })?;
            let mut claimed = Vec::new();
            for row in rows {
                let (record_key, payload_json, attempts) = row?;
                let changed = tx.execute(
                    r#"
                    UPDATE hourly_delta_discord_outbox
                    SET status = 'sending', attempts = attempts + 1,
                        next_attempt_at = ?2
                    WHERE record_key = ?1
                      AND (
                        status IN ('pending', 'retry')
                        OR (status = 'sending' AND next_attempt_at IS NOT NULL AND next_attempt_at <= ?2)
                      )
                    "#,
                    params![record_key, now_ms.saturating_add(DISCORD_OUTBOX_LEASE_MS)],
                )?;
                if changed == 1 {
                    let record: HourlyDeltaAlertRecord = serde_json::from_str(&payload_json)
                        .context("invalid hourly delta outbox payload")?;
                    claimed.push(HourlyDeltaDiscordOutboxItem {
                        record_key,
                        record,
                        attempts: attempts.max(0) as usize + 1,
                    });
                }
            }
            drop(stmt);
            tx.commit()?;
            Ok(claimed)
        })
    }

    fn finish_hourly_delta_discord_outbox(
        &self,
        record_key: &str,
        status: HourlyDeltaDiscordStatus,
        next_attempt_at: Option<i64>,
        sent_at: Option<i64>,
        last_error: Option<&str>,
    ) -> anyhow::Result<usize> {
        self.with_connection(|conn| {
            let tx = conn.unchecked_transaction()?;
            let changed = tx.execute(
                r#"
                UPDATE hourly_delta_discord_outbox
                SET status = ?2,
                    next_attempt_at = ?3,
                    sent_at = ?4,
                    last_error = ?5
                WHERE record_key = ?1
                "#,
                params![
                    record_key,
                    status.as_str(),
                    next_attempt_at,
                    sent_at,
                    last_error
                ],
            )?;
            let now_ms = crate::normalizers::trade::now_ms();
            tx.execute(
                r#"
                UPDATE hourly_delta_alert_records
                SET discord_status = ?2,
                    discord_sent_at_ms = COALESCE(?3, discord_sent_at_ms),
                    attempts = (
                      SELECT attempts FROM hourly_delta_discord_outbox WHERE record_key = ?1
                    ),
                    last_error = ?4,
                    updated_at_ms = ?5
                WHERE record_key = ?1
                "#,
                params![record_key, status.as_str(), sent_at, last_error, now_ms],
            )?;
            tx.commit()?;
            Ok(changed)
        })
    }

    fn hourly_delta_discord_outbox_stats(
        &self,
        now_ms: i64,
    ) -> anyhow::Result<HourlyDeltaDiscordOutboxStats> {
        self.with_connection(|conn| {
            conn.query_row(
                r#"
                SELECT
                  SUM(CASE WHEN status IN ('pending', 'sending') THEN 1 ELSE 0 END),
                  SUM(CASE WHEN status = 'retry' THEN 1 ELSE 0 END),
                  SUM(CASE WHEN status = 'dead' THEN 1 ELSE 0 END),
                  MIN(CASE WHEN status IN ('pending', 'sending', 'retry') THEN created_at END)
                FROM hourly_delta_discord_outbox
                "#,
                [],
                |row| {
                    let oldest_pending_at = row.get::<_, Option<i64>>(3)?;
                    Ok(HourlyDeltaDiscordOutboxStats {
                        pending: row.get::<_, Option<i64>>(0)?.unwrap_or_default().max(0) as usize,
                        retrying: row.get::<_, Option<i64>>(1)?.unwrap_or_default().max(0) as usize,
                        failed: row.get::<_, Option<i64>>(2)?.unwrap_or_default().max(0) as usize,
                        oldest_pending_age_sec: oldest_pending_at
                            .map(|created_at| now_ms.saturating_sub(created_at) / 1000)
                            .unwrap_or_default(),
                    })
                },
            )
            .context("failed to load hourly delta outbox stats")
        })
    }
}

fn map_record_row(row: &rusqlite::Row<'_>) -> rusqlite::Result<HourlyDeltaAlertRecord> {
    let direction = HourlyDeltaDirection::parse(&row.get::<_, String>(10)?)
        .unwrap_or(HourlyDeltaDirection::Flat);
    let data_status = HourlyDeltaDataStatus::parse(&row.get::<_, String>(12)?)
        .unwrap_or(HourlyDeltaDataStatus::Closed);
    let discord_status = HourlyDeltaDiscordStatus::parse(&row.get::<_, String>(13)?)
        .unwrap_or(HourlyDeltaDiscordStatus::None);
    Ok(HourlyDeltaAlertRecord {
        record_key: row.get(0)?,
        exchange: row.get(1)?,
        symbol: row.get(2)?,
        interval: row.get(3)?,
        kline_open_time_ms: row.get(4)?,
        kline_close_time_ms: row.get(5)?,
        taker_buy_btc: row.get(6)?,
        taker_sell_btc: row.get(7)?,
        delta_btc: row.get(8)?,
        volume_btc: row.get(9)?,
        direction,
        above_threshold: row.get::<_, i64>(11)? != 0,
        data_status,
        discord_status,
        discord_sent_at_ms: row.get(14)?,
        attempts: row.get::<_, i64>(15)?.max(0) as usize,
        last_error: row.get(16)?,
        payload_json: row.get(17)?,
        created_at_ms: row.get(18)?,
        updated_at_ms: row.get(19)?,
    })
}

fn bool_to_int(value: bool) -> i64 {
    if value {
        1
    } else {
        0
    }
}
