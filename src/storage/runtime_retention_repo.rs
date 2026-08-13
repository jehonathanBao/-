use rusqlite::params;

use super::{
    sqlite::{column_exists, table_exists, SqliteStore},
    storage_health::{
        classify_retention_error, RetentionTableResult, RetentionTableStatus, WalCheckpointResult,
    },
};

const DAY_MS: i64 = 24 * 60 * 60 * 1000;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct RuntimeRetentionPolicy {
    pub toxic_events_retention_ms: i64,
    pub toxic_snapshots_retention_ms: i64,
    pub flow_snapshots_retention_ms: i64,
    pub venue_health_retention_ms: i64,
    pub vpin_buckets_retention_ms: i64,
    pub replay_runs_retention_ms: i64,
    pub new_token_l2_metrics_retention_ms: i64,
    pub new_token_l2_outcomes_retention_ms: i64,
    pub delete_batch_size: usize,
    pub max_batches_per_table: usize,
    pub batch_pause_ms: u64,
    pub lock_wait_ms: u64,
    pub max_table_duration_ms: u64,
}

impl Default for RuntimeRetentionPolicy {
    fn default() -> Self {
        Self {
            toxic_events_retention_ms: 30 * DAY_MS,
            toxic_snapshots_retention_ms: 7 * DAY_MS,
            flow_snapshots_retention_ms: 7 * DAY_MS,
            venue_health_retention_ms: 7 * DAY_MS,
            vpin_buckets_retention_ms: 7 * DAY_MS,
            replay_runs_retention_ms: 30 * DAY_MS,
            new_token_l2_metrics_retention_ms: 7 * DAY_MS,
            new_token_l2_outcomes_retention_ms: 365 * DAY_MS,
            delete_batch_size: 250,
            max_batches_per_table: 80,
            batch_pause_ms: 10,
            lock_wait_ms: 250,
            max_table_duration_ms: 3_000,
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
    pub new_token_l2_metrics_deleted: usize,
    pub new_token_l2_outcomes_deleted: usize,
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
            || self.new_token_l2_metrics_deleted > 0
            || self.new_token_l2_outcomes_deleted > 0
    }

    pub fn total_deleted(&self) -> usize {
        self.toxic_events_deleted
            + self.toxic_snapshots_deleted
            + self.flow_snapshots_deleted
            + self.venue_health_snapshots_deleted
            + self.vpin_buckets_deleted
            + self.replay_runs_deleted
            + self.new_token_l2_metrics_deleted
            + self.new_token_l2_outcomes_deleted
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
        let new_token_l2_metrics_cutoff =
            retention_cutoff(now_ms, policy.new_token_l2_metrics_retention_ms);
        let new_token_l2_outcomes_cutoff =
            retention_cutoff(now_ms, policy.new_token_l2_outcomes_retention_ms);

        self.with_write_connection(|conn| {
            conn.busy_timeout(std::time::Duration::from_millis(policy.lock_wait_ms.max(1)))?;
            let mut result = RuntimeRetentionPruneResult::default();
            result.toxic_events_deleted = prune_table(
                conn,
                "toxic_events",
                "ts",
                "ts",
                toxic_events_cutoff,
                policy,
                &mut result.table_results,
            )?;
            result.toxic_snapshots_deleted = prune_table(
                conn,
                "toxic_snapshots",
                "ts",
                "ts",
                toxic_snapshots_cutoff,
                policy,
                &mut result.table_results,
            )?;
            result.flow_snapshots_deleted = prune_table(
                conn,
                "flow_snapshots",
                "ts",
                "ts",
                flow_snapshots_cutoff,
                policy,
                &mut result.table_results,
            )?;
            result.venue_health_snapshots_deleted = prune_table(
                conn,
                "venue_health_snapshots",
                "ts",
                "ts",
                venue_health_cutoff,
                policy,
                &mut result.table_results,
            )?;
            result.vpin_buckets_deleted = prune_table(
                conn,
                "vpin_buckets",
                "end_ts",
                "end_ts",
                vpin_buckets_cutoff,
                policy,
                &mut result.table_results,
            )?;
            result.replay_runs_deleted = prune_table(
                conn,
                "replay_runs",
                "finished_at",
                "COALESCE(finished_at, started_at)",
                replay_runs_cutoff,
                policy,
                &mut result.table_results,
            )?;
            result.new_token_l2_metrics_deleted = prune_table(
                conn,
                "new_token_l2_metrics",
                "ts",
                "ts",
                new_token_l2_metrics_cutoff,
                policy,
                &mut result.table_results,
            )?;
            result.new_token_l2_outcomes_deleted = prune_table(
                conn,
                "new_token_l2_outcomes",
                "observed_at",
                "observed_at",
                new_token_l2_outcomes_cutoff,
                policy,
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
    time_expression: &str,
    cutoff: i64,
    policy: &RuntimeRetentionPolicy,
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

    let batch_size = policy.delete_batch_size.max(1);
    let max_batches = policy.max_batches_per_table.max(1);
    let sql = format!(
        "DELETE FROM {table}
         WHERE rowid IN (
           SELECT rowid FROM {table}
           WHERE {time_expression} < ?1
           ORDER BY {time_expression} ASC, rowid ASC
           LIMIT ?2
         )"
    );
    let mut total_deleted = 0_usize;
    let mut completed = false;
    let mut time_budget_reached = false;
    let mut failure = None;
    for batch_index in 0..max_batches {
        if batch_index > 0
            && started_at.elapsed()
                >= std::time::Duration::from_millis(policy.max_table_duration_ms.max(1))
        {
            time_budget_reached = true;
            break;
        }
        match conn.execute(&sql, params![cutoff, batch_size as i64]) {
            Ok(deleted_rows) => {
                total_deleted = total_deleted.saturating_add(deleted_rows);
                if deleted_rows < batch_size {
                    completed = true;
                    break;
                }
                if policy.batch_pause_ms > 0 && batch_index + 1 < max_batches {
                    std::thread::sleep(std::time::Duration::from_millis(policy.batch_pause_ms));
                }
            }
            Err(error) => {
                failure = Some(error);
                break;
            }
        }
    }

    match failure {
        None => {
            table_results.push(RetentionTableResult {
                table: table.to_string(),
                time_column: time_column.to_string(),
                status: RetentionTableStatus::Ok,
                deleted_rows: total_deleted,
                duration_ms: started_at.elapsed().as_millis() as u64,
                reason: if completed {
                    None
                } else if time_budget_reached {
                    Some("time_budget_reached".to_string())
                } else {
                    Some("batch_limit_reached".to_string())
                },
                error: None,
                error_kind: None,
            });
            Ok(total_deleted)
        }
        Some(error) => {
            let message = format!("{error:#}");
            table_results.push(RetentionTableResult {
                table: table.to_string(),
                time_column: time_column.to_string(),
                status: RetentionTableStatus::Error,
                deleted_rows: total_deleted,
                duration_ms: started_at.elapsed().as_millis() as u64,
                reason: None,
                error_kind: Some(classify_retention_error(&message)),
                error: Some(message),
            });
            Ok(total_deleted)
        }
    }
}

fn run_wal_checkpoint(conn: &rusqlite::Connection) -> WalCheckpointResult {
    let started_at = std::time::Instant::now();
    match conn.query_row("PRAGMA wal_checkpoint(PASSIVE);", [], |row| {
        Ok((row.get::<_, i64>(0)?, row.get::<_, i64>(1)?, row.get::<_, i64>(2)?))
    }) {
        Ok((busy, log_frames, checkpointed_frames)) => WalCheckpointResult {
            attempted: true,
            ok: busy == 0,
            duration_ms: started_at.elapsed().as_millis() as u64,
            error: (busy != 0).then(|| {
                format!(
                    "wal_checkpoint_busy log_frames={log_frames} checkpointed_frames={checkpointed_frames}"
                )
            }),
        },
        Err(error) => WalCheckpointResult {
            attempted: true,
            ok: false,
            duration_ms: started_at.elapsed().as_millis() as u64,
            error: Some(format!("{error:#}")),
        },
    }
}
