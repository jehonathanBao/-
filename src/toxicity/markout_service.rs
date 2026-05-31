use std::sync::Arc;

use parking_lot::RwLock;

use crate::{
    config::AppConfig,
    market_data::{
        event_bus::{MarketDataBus, MarketDataEvent},
        flow_window_service::FlowWindowService,
    },
    normalizers::trade::now_ms,
    toxicity::markout_engine::{
        MarkoutEngine, DEFAULT_MARKOUT_EXPIRE_GRACE_MS, DEFAULT_MARKOUT_MAX_AGE_MS,
    },
    types::markout::MarkoutState,
};

#[derive(Clone)]
pub struct MarkoutService {
    bus: MarketDataBus,
    flow_service: FlowWindowService,
    engine: Arc<RwLock<MarkoutEngine>>,
    resolve_interval_ms: u64,
    task: Arc<RwLock<Option<tokio::task::JoinHandle<()>>>>,
}

impl MarkoutService {
    pub fn new(bus: MarketDataBus, flow_service: FlowWindowService, config: &AppConfig) -> Self {
        Self {
            bus,
            flow_service,
            engine: Arc::new(RwLock::new(MarkoutEngine::new(
                config.markout_horizons_ms.clone(),
                config.max_buffer_age_ms.max(DEFAULT_MARKOUT_MAX_AGE_MS),
                DEFAULT_MARKOUT_EXPIRE_GRACE_MS,
            ))),
            resolve_interval_ms: config.markout_resolve_interval_ms,
            task: Arc::new(RwLock::new(None)),
        }
    }

    pub fn start(&self) {
        if self.task.read().is_some() {
            return;
        }

        let mut rx = self.bus.subscribe();
        let engine = self.engine.clone();
        let flow_service = self.flow_service.clone();
        let resolve_interval_ms = self.resolve_interval_ms;
        let quality = self.bus.quality_tracker();

        let handle = tokio::spawn(async move {
            let mut interval =
                tokio::time::interval(std::time::Duration::from_millis(resolve_interval_ms));
            loop {
                tokio::select! {
                    event = rx.recv() => {
                        match event {
                            Ok(MarketDataEvent::Trade(trade)) => engine.write().on_trade(&trade),
                            Ok(MarketDataEvent::Book(_)) | Ok(MarketDataEvent::VenueHealth(_)) => {}
                            Err(tokio::sync::broadcast::error::RecvError::Lagged(skipped)) => {
                                quality.record_markout_lagged(skipped);
                            }
                            Err(tokio::sync::broadcast::error::RecvError::Closed) => break,
                        }
                    }
                    _ = interval.tick() => {
                        let now = now_ms();
                        engine.write().resolve_due_samples(now, |ts| {
                            flow_service.get_mid_at_or_before(ts)
                        });
                    }
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

    pub fn get_state(&self) -> MarkoutState {
        self.engine
            .read()
            .get_state(now_ms(), self.flow_service.has_price_index())
    }

    pub fn shared_engine_for_tests(&self) -> Arc<RwLock<MarkoutEngine>> {
        self.engine.clone()
    }
}
