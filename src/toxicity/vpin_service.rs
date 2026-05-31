use std::sync::Arc;

use parking_lot::RwLock;

use crate::{
    config::{thresholds::VpinParams, AppConfig},
    market_data::event_bus::{MarketDataBus, MarketDataEvent},
    normalizers::trade::now_ms,
    storage::{sqlite::SqliteStore, vpin_repo::VpinRepo},
    types::vpin::{VpinBucket, VpinState},
};

use super::vpin_bucket_engine::VpinBucketEngine;

#[derive(Clone)]
pub struct VpinService {
    bus: MarketDataBus,
    engine: Arc<RwLock<VpinBucketEngine>>,
    store: Option<SqliteStore>,
    persist_buckets: bool,
    task: Arc<RwLock<Option<tokio::task::JoinHandle<()>>>>,
}

impl VpinService {
    pub fn new(bus: MarketDataBus, config: &AppConfig, store: Option<SqliteStore>) -> Self {
        let params = VpinParams {
            enabled: config.vpin_enabled,
            bucket_size_btc: config.vpin_bucket_size_btc,
            lookback_buckets: config.vpin_lookback_buckets,
            min_buckets: config.vpin_min_buckets,
            spike_zscore: config.vpin_spike_zscore,
            high_threshold: config.vpin_high_threshold,
            extreme_threshold: config.vpin_extreme_threshold,
            persist_buckets: config.vpin_persist_buckets,
            ..VpinParams::default()
        };

        Self {
            bus,
            engine: Arc::new(RwLock::new(VpinBucketEngine::new(params.clone()))),
            store,
            persist_buckets: params.persist_buckets,
            task: Arc::new(RwLock::new(None)),
        }
    }

    pub fn start(&self) {
        if self.task.read().is_some() {
            return;
        }

        let mut rx = self.bus.subscribe();
        let engine = self.engine.clone();
        let store = self.store.clone();
        let persist_buckets = self.persist_buckets;
        let quality = self.bus.quality_tracker();

        let handle = tokio::spawn(async move {
            loop {
                match rx.recv().await {
                    Ok(MarketDataEvent::Trade(trade)) => {
                        let completed = engine.write().on_trade(&trade);
                        if persist_buckets {
                            if let Some(store) = &store {
                                for bucket in completed {
                                    if let Err(err) = store.insert_bucket(&bucket) {
                                        tracing::warn!("failed to persist vpin bucket: {err}");
                                    }
                                }
                            }
                        }
                    }
                    Ok(MarketDataEvent::Book(_)) | Ok(MarketDataEvent::VenueHealth(_)) => {}
                    Err(tokio::sync::broadcast::error::RecvError::Lagged(skipped)) => {
                        quality.record_vpin_lagged(skipped);
                    }
                    Err(tokio::sync::broadcast::error::RecvError::Closed) => break,
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

    pub fn get_state(&self) -> VpinState {
        self.engine.read().get_state(now_ms())
    }

    pub fn recent_buckets(&self, limit: usize) -> anyhow::Result<Vec<VpinBucket>> {
        if self.persist_buckets {
            if let Some(store) = &self.store {
                return store.list_recent_buckets(limit);
            }
        }
        Ok(self.engine.read().recent_buckets(limit))
    }

    pub fn shared_engine_for_tests(&self) -> Arc<RwLock<VpinBucketEngine>> {
        self.engine.clone()
    }
}
