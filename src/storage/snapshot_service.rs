use std::sync::Arc;

use parking_lot::RwLock;

use crate::{
    connectors::manager::ConnectorManager, market_data::flow_window_service::FlowWindowService,
    normalizers::trade::now_ms, toxicity::toxic_service::ToxicService,
};

use super::{
    snapshots_repo::SnapshotsRepo, sqlite::SqliteStore, venue_health_repo::VenueHealthRepo,
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
            loop {
                interval.tick().await;
                service.persist_once(now_ms());
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
}
