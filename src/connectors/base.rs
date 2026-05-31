use crate::types::market::Venue;

#[derive(Debug, Clone, Copy)]
pub struct ConnectorSpec {
    pub venue: Venue,
    pub enabled: bool,
}
