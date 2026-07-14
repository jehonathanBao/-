use std::sync::Arc;

use parking_lot::RwLock;

use crate::{
    connectors::manager::ConnectorManager, market_data::flow_window_service::FlowWindowService,
    normalizers::trade::now_ms, toxicity::toxic_service::ToxicService,
};

use super::{
    runtime_retention_repo::{RuntimeRetentionPolicy, RuntimeRetentionRepo},
    snapshots_repo::SnapshotsRepo,
    sqlite::SqliteStore,
    storage_health::{RetentionRunHealth, StorageHealthTracker},
    venue_health_repo::VenueHealthRepo,
};

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct StorageState {
    pub enabled: bool,
    pub status: String,
    pub sqlite_path: String,
    pub last_write_ts: Option<i64>,
    pub last_error: Option<String>,
}

#[derive(Clone)]
pub struct SnapshotService {
    enabled: bool,
    store: Option<SqliteStore>,
    flow_service: FlowWindowService,
    toxic_service: ToxicService,
    connector_manager: ConnectorManager,
    persist_interval_ms: u64,
    retention_policy: RuntimeRetentionPolicy,
    retention_interval_ms: u64,
    storage_health: StorageHealthTracker,
    latest_state: Arc<RwLock<StorageState>>,
    task: Arc<RwLock<Option<tokio::task::JoinHandle<()>>>>,
}

fn initial_retention_last_run_ts(
    now_ms: i64,
    retention_interval_ms: u64,
    initial_delay_ms: i64,
) -> i64 {
    let elapsed_before_first_run =
        retention_interval_ms.saturating_sub(initial_delay_ms.max(0) as u64);
    now_ms.saturating_sub(elapsed_before_first_run as i64)
}

impl SnapshotService {
    pub fn new(
        enabled: bool,
        sqlite_path: String,
        persist_interval_ms: u64,
        store: Option<SqliteStore>,
        flow_service: FlowWindowService,
        toxic_service: ToxicService,
        connector_manager: ConnectorManager,
        storage_health: StorageHealthTracker,
    ) -> Self {
        let status = if enabled && store.is_some() {
            "ok"
        } else if enabled {
            "degraded"
        } else {
            "disabled"
        };
        Self {
            enabled,
            store,
            flow_service,
            toxic_service,
            connector_manager,
            persist_interval_ms,
            retention_policy: RuntimeRetentionPolicy::default(),
            retention_interval_ms: 60 * 60 * 1000,
            storage_health,
            latest_state: Arc::new(RwLock::new(StorageState {
                enabled,
                status: status.to_string(),
                sqlite_path,
                last_write_ts: None,
                last_error: None,
            })),
            task: Arc::new(RwLock::new(None)),
        }
    }

    pub fn start(&self) {
        const INITIAL_RETENTION_DELAY_MS: i64 = 30_000;
        if self.task.read().is_some() || !self.enabled || self.store.is_none() {
            return;
        }
        let service = self.clone();
        let handle = tokio::spawn(async move {
            let mut interval = tokio::time::interval(std::time::Duration::from_millis(
                service.persist_interval_ms.max(100),
            ));
            let mut last_retention_run_ts = initial_retention_last_run_ts(
                now_ms(),
                service.retention_interval_ms,
                INITIAL_RETENTION_DELAY_MS,
            );
            loop {
                interval.tick().await;
                let now_ts = now_ms();
                service.persist_once(now_ts);
                if now_ts.saturating_sub(last_retention_run_ts)
                    >= service.retention_interval_ms.max(60_000) as i64
                {
                    service.prune_retention_once(now_ts).await;
                    last_retention_run_ts = now_ts;
                }
            }
        });
        *self.task.write() = Some(handle);
    }

    pub fn stop(&self) {
        if let Some(handle) = self.task.write().take() {
            handle.abort();
        }
    }

    pub fn get_state(&self) -> StorageState {
        self.latest_state.read().clone()
    }

    pub fn persist_once_for_tests(&self, now_ts: i64) -> StorageState {
        self.persist_once(now_ts)
    }

    fn persist_once(&self, now_ts: i64) -> StorageState {
        let Some(store) = &self.store else {
            return self.get_state();
        };

        let flow_state = self.flow_service.latest_state();
        let toxic_state = self.toxic_service.get_state();
        let venue_health = self.connector_manager.get_venue_health();
        let mut state = self.get_state();
        let storage_health = self.storage_health.refresh_if_due(false);
        if storage_health.degraded_mode_active {
            state.status = "degraded".to_string();
            state.last_error = Some(format!(
                "storage_guard_degraded_mode disabled_writes={}",
                storage_health.degraded_writes.join(",")
            ));
            *self.latest_state.write() = state.clone();
            return state;
        }

        match store
            .insert_flow_snapshot(&flow_state)
            .and_then(|_| store.insert_toxic_snapshot(&toxic_state))
            .and_then(|_| store.insert_venue_health_snapshot(now_ts, &venue_health))
        {
            Ok(()) => {
                state.status = "ok".to_string();
                state.last_write_ts = Some(now_ts);
                state.last_error = None;
            }
            Err(err) => {
                state.status = "degraded".to_string();
                state.last_error = Some(err.to_string());
            }
        }

        *self.latest_state.write() = state.clone();
        state
    }

    async fn prune_retention_once(&self, now_ts: i64) {
        let Some(store) = self.store.clone() else {
            return;
        };
        let policy = self.retention_policy;
        let storage_health = self.storage_health.clone();
        let started_at = std::time::Instant::now();
        match tokio::task::spawn_blocking(move || store.prune_runtime_retention(now_ts, &policy))
            .await
        {
            Ok(Ok(result)) => {
                for table_result in &result.table_results {
                    match table_result.status {
                        crate::storage::storage_health::RetentionTableStatus::Ok => tracing::info!(
                            table = table_result.table.as_str(),
                            time_column = table_result.time_column.as_str(),
                            status = table_result.status.as_str(),
                            deleted = table_result.deleted_rows,
                            reason = table_result.reason.as_deref().unwrap_or("completed"),
                            duration_ms = table_result.duration_ms,
                            "runtime_retention table result"
                        ),
                        crate::storage::storage_health::RetentionTableStatus::Skipped => {
                            tracing::warn!(
                                table = table_result.table.as_str(),
                                time_column = table_result.time_column.as_str(),
                                status = table_result.status.as_str(),
                                reason = table_result.reason.as_deref().unwrap_or("unknown"),
                                duration_ms = table_result.duration_ms,
                                "runtime_retention table skipped"
                            )
                        }
                        crate::storage::storage_health::RetentionTableStatus::Error => {
                            tracing::warn!(
                                table = table_result.table.as_str(),
                                time_column = table_result.time_column.as_str(),
                                status = table_result.status.as_str(),
                                error_kind =
                                    table_result.error_kind.as_deref().unwrap_or("unknown"),
                                error = table_result.error.as_deref().unwrap_or("unknown"),
                                duration_ms = table_result.duration_ms,
                                "runtime_retention table failed"
                            )
                        }
                    }
                }
                tracing::info!(
                    deleted_toxic_events = result.toxic_events_deleted,
                    deleted_toxic_snapshots = result.toxic_snapshots_deleted,
                    deleted_flow_snapshots = result.flow_snapshots_deleted,
                    deleted_venue_health_snapshots = result.venue_health_snapshots_deleted,
                    deleted_vpin_buckets = result.vpin_buckets_deleted,
                    deleted_replay_runs = result.replay_runs_deleted,
                    deleted_new_token_l2_metrics = result.new_token_l2_metrics_deleted,
                    deleted_new_token_l2_outcomes = result.new_token_l2_outcomes_deleted,
                    total_deleted_rows = result.total_deleted(),
                    failed_tables = result
                        .table_results
                        .iter()
                        .filter(|entry| entry.status
                            == crate::storage::storage_health::RetentionTableStatus::Error)
                        .count(),
                    skipped_tables = result
                        .table_results
                        .iter()
                        .filter(|entry| entry.status
                            == crate::storage::storage_health::RetentionTableStatus::Skipped)
                        .count(),
                    duration_ms = started_at.elapsed().as_millis() as u64,
                    "runtime retention prune completed"
                );
                storage_health.record_runtime_retention(
                    RetentionRunHealth {
                        ok: result.table_results.iter().all(|entry| {
                            entry.status
                                != crate::storage::storage_health::RetentionTableStatus::Error
                        }),
                        total_deleted_rows: result.total_deleted(),
                        failed_tables: result
                            .table_results
                            .iter()
                            .filter(|entry| {
                                entry.status
                                    == crate::storage::storage_health::RetentionTableStatus::Error
                            })
                            .map(|entry| entry.table.clone())
                            .collect(),
                        skipped_tables: result
                            .table_results
                            .iter()
                            .filter(|entry| {
                                entry.status
                                    == crate::storage::storage_health::RetentionTableStatus::Skipped
                            })
                            .map(|entry| entry.table.clone())
                            .collect(),
                        error: None,
                        duration_ms: Some(started_at.elapsed().as_millis() as u64),
                        finished_at_ms: Some(now_ts),
                    },
                    result.wal_checkpoint.clone(),
                );
                storage_health.refresh_now();
            }
            Ok(Err(error)) => {
                tracing::warn!(error = %error, "runtime retention prune failed");
                storage_health.record_runtime_retention(
                    RetentionRunHealth {
                        ok: false,
                        total_deleted_rows: 0,
                        failed_tables: Vec::new(),
                        skipped_tables: Vec::new(),
                        error: Some(error.to_string()),
                        duration_ms: Some(started_at.elapsed().as_millis() as u64),
                        finished_at_ms: Some(now_ts),
                    },
                    None,
                );
                storage_health.refresh_now();
            }
            Err(error) => {
                tracing::warn!(error = %error, "runtime retention prune task failed");
                storage_health.record_runtime_retention(
                    RetentionRunHealth {
                        ok: false,
                        total_deleted_rows: 0,
                        failed_tables: Vec::new(),
                        skipped_tables: Vec::new(),
                        error: Some(error.to_string()),
                        duration_ms: Some(started_at.elapsed().as_millis() as u64),
                        finished_at_ms: Some(now_ts),
                    },
                    None,
                );
                storage_health.refresh_now();
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::initial_retention_last_run_ts;

    #[test]
    fn initial_runtime_retention_becomes_due_after_configured_delay() {
        let now_ms = 1_800_000_000_000_i64;
        let interval_ms = 60 * 60 * 1_000_u64;
        let delay_ms = 30_000_i64;
        let last_run_ms = initial_retention_last_run_ts(now_ms, interval_ms, delay_ms);

        assert!(now_ms + delay_ms - 1 - last_run_ms < interval_ms as i64);
        assert!(now_ms + delay_ms - last_run_ms >= interval_ms as i64);
    }
}
