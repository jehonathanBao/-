use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};

use crate::types::market::{AggressorSide, Venue};

pub type MarkoutHorizonMs = u64;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum MarkoutSampleStatus {
    Pending,
    Resolved,
    Expired,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct MarkoutSample {
    pub id: String,
    pub venue: Venue,
    pub symbol: String,
    pub trade_ts: i64,
    pub horizon_ms: MarkoutHorizonMs,
    pub direction: AggressorSide,
    pub trade_price: f64,
    pub size_btc: f64,
    pub size_usd: f64,
    pub future_ts: Option<i64>,
    pub future_mid: Option<f64>,
    pub markout_bps: Option<f64>,
    pub status: MarkoutSampleStatus,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DirectionalMarkoutStats {
    pub count: u64,
    pub volume_btc: f64,
    pub volume_usd: f64,
    pub avg_markout_bps: Option<f64>,
    pub volume_weighted_markout_bps: Option<f64>,
    pub positive_count: u64,
    pub negative_count: u64,
    pub positive_volume_btc: f64,
    pub negative_volume_btc: f64,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct VenueMarkoutBreakdown {
    pub buy: DirectionalMarkoutStats,
    pub sell: DirectionalMarkoutStats,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct MarkoutWindowSummary {
    pub horizon_ms: MarkoutHorizonMs,
    pub buy: DirectionalMarkoutStats,
    pub sell: DirectionalMarkoutStats,
    pub venue_breakdown: BTreeMap<String, VenueMarkoutBreakdown>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct MarkoutQuality {
    pub pending_samples: usize,
    pub resolved_samples: usize,
    pub expired_samples: usize,
    pub has_price_index: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct MarkoutState {
    pub symbol: String,
    pub updated_at: i64,
    pub horizons_ms: Vec<MarkoutHorizonMs>,
    pub summaries: BTreeMap<String, MarkoutWindowSummary>,
    pub quality: MarkoutQuality,
}

pub fn empty_venue_markout_breakdown() -> BTreeMap<String, VenueMarkoutBreakdown> {
    Venue::ALL
        .into_iter()
        .map(|venue| (venue.as_key().to_string(), VenueMarkoutBreakdown::default()))
        .collect()
}
