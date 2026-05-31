use serde::{Deserialize, Serialize};

use crate::types::{
    market::{AggressorSide, Venue},
    toxic::ToxicDirection,
};

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ReplayTradeRecord {
    pub venue: Venue,
    pub ts: i64,
    pub price: f64,
    pub size_btc: f64,
    pub aggressor_side: AggressorSide,
    pub trade_id: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ReplayBookRecord {
    pub venue: Venue,
    pub ts: i64,
    pub best_bid: f64,
    pub best_ask: f64,
    #[serde(default)]
    pub bids: Vec<(f64, f64)>,
    #[serde(default)]
    pub asks: Vec<(f64, f64)>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ReplayExpectToxicRecord {
    pub ts: i64,
    pub direction: ToxicDirection,
    pub min_toxic_volume_btc: f64,
    pub window_ms: u64,
}

#[derive(Debug, Clone)]
pub enum ReplayEvent {
    Trade(ReplayTradeRecord),
    Book(ReplayBookRecord),
    ExpectToxic(ReplayExpectToxicRecord),
}
