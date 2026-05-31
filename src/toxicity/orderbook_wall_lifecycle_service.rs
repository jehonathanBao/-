use std::sync::Arc;

use parking_lot::RwLock;

use crate::{
    market_data::event_bus::{MarketDataBus, MarketDataEvent},
    toxicity::orderbook_wall_lifecycle::OrderbookWallLifecycleEngine,
    types::{market::NormalizedBook, orderbook_wall::OrderbookWallLifecycleState},
};

#[derive(Clone)]
pub struct OrderbookWallLifecycleService {
    bus: MarketDataBus,
    engine: Arc<RwLock<OrderbookWallLifecycleEngine>>,
    task: Arc<RwLock<Option<tokio::task::JoinHandle<()>>>>,
}

impl OrderbookWallLifecycleService {
    pub fn new(bus: MarketDataBus, symbol: impl Into<String>) -> Self {
        Self {
            bus,
            engine: Arc::new(RwLock::new(OrderbookWallLifecycleEngine::new(symbol))),
            task: Arc::new(RwLock::new(None)),
        }
    }

    pub fn start(&self) {
        if self.task.read().is_some() {
            return;
        }

        let mut rx = self.bus.subscribe();
        let engine = self.engine.clone();

        let handle = tokio::spawn(async move {
            loop {
                match rx.recv().await {
                    Ok(MarketDataEvent::Book(book)) => engine.write().on_book(&book),
                    Ok(MarketDataEvent::Trade(_)) | Ok(MarketDataEvent::VenueHealth(_)) => {}
                    Err(tokio::sync::broadcast::error::RecvError::Lagged(_)) => {}
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

    pub fn get_state(&self) -> OrderbookWallLifecycleState {
        self.engine.read().get_state()
    }

    pub fn on_book_for_tests(&self, book: &NormalizedBook) {
        self.engine.write().on_book(book);
    }
}
