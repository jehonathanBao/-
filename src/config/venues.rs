use crate::types::market::Venue;

#[derive(Debug, Clone, Copy)]
pub struct VenueConfig {
    pub venue: Venue,
    pub enabled: bool,
}

#[derive(Debug, Clone)]
pub struct VenueConfigs {
    pub binance: VenueConfig,
    pub bybit: VenueConfig,
    pub okx: VenueConfig,
}

impl VenueConfigs {
    pub fn all(&self) -> [VenueConfig; 3] {
        [self.binance, self.bybit, self.okx]
    }
}
