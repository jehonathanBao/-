use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};

pub type VpinBucketId = u64;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum VpinDirection {
    Buy,
    Sell,
    Balanced,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct VpinVenueBreakdown {
    pub buy_btc: f64,
    pub sell_btc: f64,
    pub total_btc: f64,
    pub net_btc: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct VpinBucket {
    pub id: VpinBucketId,
    pub symbol: String,
    pub start_ts: i64,
    pub end_ts: i64,
    pub bucket_size_btc: f64,
    pub total_btc: f64,
    pub buy_btc: f64,
    pub sell_btc: f64,
    pub net_btc: f64,
    pub imbalance_btc: f64,
    pub imbalance_ratio: f64,
    pub direction: VpinDirection,
    pub venue_breakdown: BTreeMap<String, VpinVenueBreakdown>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct VpinMetrics {
    pub symbol: String,
    pub updated_at: i64,
    pub enabled: bool,
    pub bucket_size_btc: f64,
    pub lookback_buckets: usize,
    pub min_buckets: usize,
    pub completed_bucket_count: usize,
    pub active_bucket_progress_btc: f64,
    pub active_bucket_progress_ratio: f64,
    pub latest_bucket: Option<VpinBucket>,
    pub vpin: Option<f64>,
    pub vpin_zscore: Option<f64>,
    pub vpin_percentile: Option<f64>,
    #[serde(default)]
    pub per_venue_vpin: BTreeMap<String, f64>,
    pub latest_bucket_imbalance_ratio: Option<f64>,
    pub avg_bucket_imbalance_ratio: Option<f64>,
    pub vpin_high: bool,
    pub vpin_extreme: bool,
    pub vpin_spike: bool,
    pub dominant_direction: VpinDirection,
    pub reason_codes: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct VpinState {
    pub symbol: String,
    pub updated_at: i64,
    pub metrics: VpinMetrics,
    pub recent_buckets: Vec<VpinBucket>,
}
