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
    latest_state: Arc<RwLock<StorageState>>,
    task: Arc<RwLock<Option<tokio::task::JoinHandle<()>>>>,
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
        if self.task.read().is_some() || !self.enabled || self.store.is_none() {
            return;
        }
        let service = self.clone();
        let handle = tokio::spawn(async move {
            let mut interval = tokio::time::interval(std::time::Duration::from_millis(
                service.persist_interval_ms.max(100),
            ));
            let mut last_retention_run_ts = 0_i64;
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
        let started_at = std::time::Instant::now();
        match tokio::task::spawn_blocking(move || store.prune_runtime_retention(now_ts, &policy))
            .await
        {
            Ok(Ok(result)) => {
                tracing::info!(
                    deleted_toxic_events = result.toxic_events_deleted,
                    deleted_toxic_snapshots = result.toxic_snapshots_deleted,
                    deleted_flow_snapshots = result.flow_snapshots_deleted,
                    deleted_venue_health_snapshots = result.venue_health_snapshots_deleted,
                    deleted_vpin_buckets = result.vpin_buckets_deleted,
                    deleted_replay_runs = result.replay_runs_deleted,
                    duration_ms = started_at.elapsed().as_millis() as u64,
                    "runtime retention prune completed"
                );
            }
            Ok(Err(error)) => {
                tracing::warn!(error = %error, "runtime retention prune failed");
            }
            Err(error) => {
                tracing::warn!(error = %error, "runtime retention prune task failed");
            }
        }
    }
}
