use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct MainForceEvent {
    pub id: i64,
    pub symbol: String,
    pub started_at: i64,
    pub ended_at: Option<i64>,
    pub peak_at: i64,
    pub last_observed_at: i64,
    pub inactive_since: Option<i64>,
    pub regime_type: String,
    pub severity: String,
    pub peak_main_force_score: f64,
    pub peak_extreme_impact_score: f64,
    pub peak_structure_bias: f64,
    pub confidence: f64,
    pub spot_score: Option<f64>,
    pub contract_score: Option<f64>,
    pub cross_confirm_score: Option<f64>,
    pub cwm_score: Option<f64>,
    pub oi_score: Option<f64>,
    pub liquidation_score: Option<f64>,
    pub funding_crowding_score: Option<f64>,
    pub main_force_confirmed: bool,
    pub extreme_impact_confirmed: bool,
    pub liquidation_driven: bool,
    pub reasons_json: serde_json::Value,
    pub created_at: i64,
}

#[derive(Debug, Clone, Default)]
pub struct MainForceEventQuery {
    pub symbol: Option<String>,
    pub regime_type: Option<String>,
    pub severity: Option<String>,
    pub active_only: Option<bool>,
    pub from_ts: Option<i64>,
    pub to_ts: Option<i64>,
    pub limit: usize,
    pub offset: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct MainForceEventsResponse {
    pub read_only: bool,
    pub execution_enabled: bool,
    pub items: Vec<MainForceEvent>,
    pub filter: BTreeMap<String, String>,
}

#[derive(Debug, Clone)]
pub struct MainForceEventObservation {
    pub symbol: String,
    pub observed_at: i64,
    pub regime_type: String,
    pub severity: String,
    pub main_force_score: f64,
    pub extreme_impact_score: f64,
    pub structure_bias: f64,
    pub confidence: f64,
    pub spot_score: Option<f64>,
    pub contract_score: Option<f64>,
    pub cross_confirm_score: Option<f64>,
    pub cwm_score: Option<f64>,
    pub oi_score: Option<f64>,
    pub liquidation_score: Option<f64>,
    pub funding_crowding_score: Option<f64>,
    pub main_force_confirmed: bool,
    pub extreme_impact_confirmed: bool,
    pub liquidation_driven: bool,
    pub reasons_json: serde_json::Value,
}

impl MainForceEventObservation {
    pub fn start_triggered(&self) -> bool {
        let config = crate::runtime::score_config::score_runtime_config();
        self.main_force_score >= f64::from(config.market_structure.event_start_main_force_score)
            || self.extreme_impact_score
                >= f64::from(config.market_structure.event_start_extreme_impact_score)
    }

    pub fn keeps_event_open(&self) -> bool {
        let config = crate::runtime::score_config::score_runtime_config();
        self.main_force_score >= f64::from(config.market_structure.event_end_main_force_score)
            || self.extreme_impact_score
                >= f64::from(config.market_structure.event_end_extreme_impact_score)
    }
}
