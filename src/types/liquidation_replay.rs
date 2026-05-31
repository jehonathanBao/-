use serde::{Deserialize, Serialize};

use super::liquidation::LiquidationClusterSide;

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct LiquidationReplayEvidence {
    pub ts_ms: i64,
    pub symbol: String,
    pub mark_price: f64,
    pub nearest_cluster_price: Option<f64>,
    pub nearest_cluster_distance_bps: Option<f64>,
    pub nearest_cluster_side: Option<LiquidationClusterSide>,
    pub cluster_intensity: f64,
    pub nearby_cluster: bool,
    pub possible_liq_hunt_setup: bool,
    pub explanation: Vec<String>,
}
