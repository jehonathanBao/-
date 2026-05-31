use tokio::sync::broadcast;

use crate::{
    market_data::quality::MarketDataQualityTracker,
    types::market::{NormalizedBook, NormalizedTrade, VenueHealth},
};

#[derive(Debug, Clone)]
#[allow(clippy::large_enum_variant)]
pub enum MarketDataEvent {
    Trade(NormalizedTrade),
    Book(NormalizedBook),
    VenueHealth(VenueHealth),
}

#[derive(Clone)]
pub struct MarketDataBus {
    tx: broadcast::Sender<MarketDataEvent>,
    quality: MarketDataQualityTracker,
}

impl MarketDataBus {
    pub fn new(capacity: usize) -> Self {
        let (tx, _) = broadcast::channel(capacity);
        Self {
            tx,
            quality: MarketDataQualityTracker::default(),
        }
    }

    pub fn subscribe(&self) -> broadcast::Receiver<MarketDataEvent> {
        self.tx.subscribe()
    }

    pub fn publish(&self, event: MarketDataEvent) {
        if self.tx.send(event).is_err() {
            self.quality.record_send_error();
        }
    }

    pub fn quality_tracker(&self) -> MarketDataQualityTracker {
        self.quality.clone()
    }
}
