use rusqlite::params;

use super::{
    sqlite::{column_exists, table_exists, SqliteStore},
    storage_health::{
        classify_retention_error, RetentionTableResult, RetentionTableStatus, WalCheckpointResult,
    },
};

const DAY_MS: i64 = 24 * 60 * 60 * 1000;
const HOUR_MS: i64 = 60 * 60 * 1000;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct RuntimeRetentionPolicy {
    pub toxic_events_retention_ms: i64,
    pub toxic_snapshots_retention_ms: i64,
    pub flow_snapshots_retention_ms: i64,
    pub venue_health_retention_ms: i64,
    pub vpin_buckets_retention_ms: i64,
    pub replay_runs_retention_ms: i64,
}

impl Default for RuntimeRetentionPolicy {
    fn default() -> Self {
        Self {
            toxic_events_retention_ms: 30 * DAY_MS,
            toxic_snapshots_retention_ms: 24 * HOUR_MS,
            flow_snapshots_retention_ms: 24 * HOUR_MS,
            venue_health_retention_ms: 24 * HOUR_MS,
            vpin_buckets_retention_ms: 14 * DAY_MS,
            replay_runs_retention_ms: 30 * DAY_MS,
        }
    }
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct RuntimeRetentionPruneResult {
    pub toxic_events_deleted: usize,
    pub toxic_snapshots_deleted: usize,
    pub flow_snapshots_deleted: usize,
    pub venue_health_snapshots_deleted: usize,
    pub vpin_buckets_deleted: usize,
    pub replay_runs_deleted: usize,
    pub table_results: Vec<RetentionTableResult>,
    pub wal_checkpoint: Option<WalCheckpointResult>,
}

impl RuntimeRetentionPruneResult {
    pub fn any_deleted(&self) -> bool {
        self.toxic_events_deleted > 0
            || self.toxic_snapshots_deleted > 0
            || self.flow_snapshots_deleted > 0
            || self.venue_health_snapshots_deleted > 0
            || self.vpin_buckets_deleted > 0
            || self.replay_runs_deleted > 0
    }

    pub fn total_deleted(&self) -> usize {
        self.toxic_events_deleted
            + self.toxic_snapshots_deleted
            + self.flow_snapshots_deleted
            + self.venue_health_snapshots_deleted
            + self.vpin_buckets_deleted
            + self.replay_runs_deleted
    }
}

pub trait RuntimeRetentionRepo {
    fn prune_runtime_retention(
        &self,
        now_ms: i64,
        policy: &RuntimeRetentionPolicy,
    ) -> anyhow::Result<RuntimeRetentionPruneResult>;
}

impl RuntimeRetentionRepo for SqliteStore {
    fn prune_runtime_retention(
        &self,
        now_ms: i64,
        policy: &RuntimeRetentionPolicy,
    ) -> anyhow::Result<RuntimeRetentionPruneResult> {
        let toxic_events_cutoff = retention_cutoff(now_ms, policy.toxic_events_retention_ms);
        let toxic_snapshots_cutoff = retention_cutoff(now_ms, policy.toxic_snapshots_retention_ms);
        let flow_snapshots_cutoff = retention_cutoff(now_ms, policy.flow_snapshots_retention_ms);
        let venue_health_cutoff = retention_cutoff(now_ms, policy.venue_health_retention_ms);
        let vpin_buckets_cutoff = retention_cutoff(now_ms, policy.vpin_buckets_retention_ms);
        let replay_runs_cutoff = retention_cutoff(now_ms, policy.replay_runs_retention_ms);

        self.with_connection(|conn| {
            let mut result = RuntimeRetentionPruneResult::default();
            result.toxic_events_deleted = prune_table(
                conn,
                "toxic_events",
                "ts",
                "DELETE FROM toxic_events WHERE ts < ?1",
                toxic_events_cutoff,
                &mut result.table_results,
            )?;
            result.toxic_snapshots_deleted = prune_table(
                conn,
                "toxic_snapshots",
                "ts",
                "DELETE FROM toxic_snapshots WHERE ts < ?1",
                toxic_snapshots_cutoff,
                &mut result.table_results,
            )?;
            result.flow_snapshots_deleted = prune_table(
                conn,
                "flow_snapshots",
                "ts",
                "DELETE FROM flow_snapshots WHERE ts < ?1",
                flow_snapshots_cutoff,
                &mut result.table_results,
            )?;
            result.venue_health_snapshots_deleted = prune_table(
                conn,
                "venue_health_snapshots",
                "ts",
                "DELETE FROM venue_health_snapshots WHERE ts < ?1",
                venue_health_cutoff,
                &mut result.table_results,
            )?;
            result.vpin_buckets_deleted = prune_table(
                conn,
                "vpin_buckets",
                "end_ts",
                "DELETE FROM vpin_buckets WHERE end_ts < ?1",
                vpin_buckets_cutoff,
                &mut result.table_results,
            )?;
            result.replay_runs_deleted = prune_table(
                conn,
                "replay_runs",
                "finished_at",
                "DELETE FROM replay_runs WHERE COALESCE(finished_at, started_at) < ?1",
                replay_runs_cutoff,
                &mut result.table_results,
            )?;

            if result.any_deleted() {
                result.wal_checkpoint = Some(run_wal_checkpoint(conn));
            }

            Ok(result)
        })
    }
}

fn retention_cutoff(now_ms: i64, retention_ms: i64) -> i64 {
    now_ms.saturating_sub(retention_ms.max(1_000))
}

fn prune_table(
    conn: &rusqlite::Connection,
    table: &str,
    time_column: &str,
    sql: &str,
    cutoff: i64,
    table_results: &mut Vec<RetentionTableResult>,
) -> anyhow::Result<usize> {
    let started_at = std::time::Instant::now();
    if !table_exists(conn, table)? {
        table_results.push(RetentionTableResult {
            table: table.to_string(),
            time_column: time_column.to_string(),
            status: RetentionTableStatus::Skipped,
            deleted_rows: 0,
            duration_ms: started_at.elapsed().as_millis() as u64,
            reason: Some("table_missing".to_string()),
            error: None,
            error_kind: None,
        });
        return Ok(0);
    }
    if !column_exists(conn, table, time_column)? {
        table_results.push(RetentionTableResult {
            table: table.to_string(),
            time_column: time_column.to_string(),
            status: RetentionTableStatus::Skipped,
            deleted_rows: 0,
            duration_ms: started_at.elapsed().as_millis() as u64,
            reason: Some("time_column_missing".to_string()),
            error: None,
            error_kind: None,
        });
        return Ok(0);
    }

    match conn.execute(sql, params![cutoff]) {
        Ok(deleted_rows) => {
            table_results.push(RetentionTableResult {
                table: table.to_string(),
                time_column: time_column.to_string(),
                status: RetentionTableStatus::Ok,
                deleted_rows,
                duration_ms: started_at.elapsed().as_millis() as u64,
                reason: None,
                error: None,
                error_kind: None,
            });
            Ok(deleted_rows)
        }
        Err(error) => {
            let message = format!("{error:#}");
            table_results.push(RetentionTableResult {
                table: table.to_string(),
                time_column: time_column.to_string(),
                status: RetentionTableStatus::Error,
                deleted_rows: 0,
                duration_ms: started_at.elapsed().as_millis() as u64,
                reason: None,
                error_kind: Some(classify_retention_error(&message)),
                error: Some(message),
            });
            Ok(0)
        }
    }
}

fn run_wal_checkpoint(conn: &rusqlite::Connection) -> WalCheckpointResult {
    let started_at = std::time::Instant::now();
    match conn.execute_batch("PRAGMA wal_checkpoint(TRUNCATE);") {
        Ok(()) => WalCheckpointResult {
            attempted: true,
            ok: true,
            duration_ms: started_at.elapsed().as_millis() as u64,
            error: None,
        },
        Err(error) => WalCheckpointResult {
            attempted: true,
            ok: false,
            duration_ms: started_at.elapsed().as_millis() as u64,
            error: Some(format!("{error:#}")),
        },
    }
}
