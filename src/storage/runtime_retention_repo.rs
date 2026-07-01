use anyhow::Context;
use rusqlite::params;

use super::sqlite::SqliteStore;

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

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct RuntimeRetentionPruneResult {
    pub toxic_events_deleted: usize,
    pub toxic_snapshots_deleted: usize,
    pub flow_snapshots_deleted: usize,
    pub venue_health_snapshots_deleted: usize,
    pub vpin_buckets_deleted: usize,
    pub replay_runs_deleted: usize,
}

impl RuntimeRetentionPruneResult {
    pub fn any_deleted(self) -> bool {
        self.toxic_events_deleted > 0
            || self.toxic_snapshots_deleted > 0
            || self.flow_snapshots_deleted > 0
            || self.venue_health_snapshots_deleted > 0
            || self.vpin_buckets_deleted > 0
            || self.replay_runs_deleted > 0
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
            let tx = conn.unchecked_transaction()?;
            let toxic_events_deleted = tx
                .execute(
                    "DELETE FROM toxic_events WHERE ts < ?1",
                    params![toxic_events_cutoff],
                )
                .context("failed to prune toxic events")?;
            let toxic_snapshots_deleted = tx
                .execute(
                    "DELETE FROM toxic_snapshots WHERE ts < ?1",
                    params![toxic_snapshots_cutoff],
                )
                .context("failed to prune toxic snapshots")?;
            let flow_snapshots_deleted = tx
                .execute(
                    "DELETE FROM flow_snapshots WHERE ts < ?1",
                    params![flow_snapshots_cutoff],
                )
                .context("failed to prune flow snapshots")?;
            let venue_health_snapshots_deleted = tx
                .execute(
                    "DELETE FROM venue_health_snapshots WHERE ts < ?1",
                    params![venue_health_cutoff],
                )
                .context("failed to prune venue health snapshots")?;
            let vpin_buckets_deleted = tx
                .execute(
                    "DELETE FROM vpin_buckets WHERE end_ts < ?1",
                    params![vpin_buckets_cutoff],
                )
                .context("failed to prune vpin buckets")?;
            let replay_runs_deleted = tx
                .execute(
                    "DELETE FROM replay_runs WHERE COALESCE(finished_at, started_at) < ?1",
                    params![replay_runs_cutoff],
                )
                .context("failed to prune replay runs")?;
            tx.commit()?;

            let result = RuntimeRetentionPruneResult {
                toxic_events_deleted,
                toxic_snapshots_deleted,
                flow_snapshots_deleted,
                venue_health_snapshots_deleted,
                vpin_buckets_deleted,
                replay_runs_deleted,
            };

            if result.any_deleted() {
                conn.execute_batch("PRAGMA wal_checkpoint(TRUNCATE);")
                    .context("failed to checkpoint sqlite wal after runtime retention prune")?;
            }

            Ok(result)
        })
    }
}

fn retention_cutoff(now_ms: i64, retention_ms: i64) -> i64 {
    now_ms.saturating_sub(retention_ms.max(1_000))
}
