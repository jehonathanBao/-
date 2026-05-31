use std::collections::BTreeMap;

use serde::Serialize;

use super::{
    liquidation::LiquidationClusterSide,
    market::{Venue, VenueHealth},
    sweep::SweepDirection,
    toxic::ToxicDirection,
    vpin::VpinDirection,
};

pub type VenueHealthMap = BTreeMap<String, VenueHealth>;

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct StatusResponse {
    pub app: &'static str,
    pub read_only: bool,
    pub config_source: &'static str,
    pub runtime_control: RuntimeControlSummary,
    pub symbol: String,
    pub threshold_btc: f64,
    pub windows_ms: Vec<u64>,
    pub venues: VenueHealthMap,
    pub market_data_quality: MarketDataQualitySummary,
    pub markout: MarkoutStatusSummary,
    pub sweep: SweepStatusSummary,
    pub vpin: VpinStatusSummary,
    pub liquidation: LiquidationStatusSummary,
    pub liq_hunt: LiqHuntStatusSummary,
    pub toxic: ToxicStatusSummary,
    pub alerts: AlertStatusSummary,
    pub storage: StorageStatusSummary,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum MarketDataQualityStatus {
    Healthy,
    Degraded,
    Stale,
    NoData,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct MarketDataQualitySummary {
    pub status: MarketDataQualityStatus,
    pub event_bus_dropped_events: u64,
    pub event_bus_send_errors: u64,
    pub flow_window_lagged_events: u64,
    pub markout_lagged_events: u64,
    pub vpin_lagged_events: u64,
    pub last_lagged_at_ms: Option<i64>,
    pub last_message_ts: Option<i64>,
    pub latest_trade_ts: Option<i64>,
    pub flow_windows_populated: bool,
    pub operator_warning: Option<&'static str>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct VenueDiagnosticsResponse {
    pub read_only: bool,
    pub analysis_only: bool,
    pub execution_enabled: bool,
    pub runtime_modified: bool,
    pub monitoring_started: bool,
    pub diagnostic_status: &'static str,
    pub summary: VenueDiagnosticsSummary,
    pub venues: Vec<super::market::VenueHealth>,
    pub operator_notes: Vec<String>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct VenueDiagnosticsSummary {
    pub configured_venues: usize,
    pub enabled_venues: usize,
    pub connector_constructed_venues: usize,
    pub start_attempted_venues: usize,
    pub connected_venues: usize,
    pub ws_connect_attempted_venues: usize,
    pub ws_connected_venues: usize,
    pub symbol_mapped_venues: usize,
    pub venues_with_network_errors: usize,
    pub active_trade_venues: usize,
    pub active_book_venues: usize,
    pub trade_active_venues: usize,
    pub book_active_venues: usize,
    pub active_venues: usize,
    pub diagnostic_status: &'static str,
    pub latest_venue_trade_available: bool,
    pub latest_venue_book_available: bool,
    pub flow_windows_populated: bool,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct RuntimeControlSummary {
    pub monitoring_started: bool,
    pub one_click_start_enabled: bool,
    pub start_action_label: &'static str,
    pub start_action_mode: &'static str,
    pub start_state: RuntimeStartState,
    pub last_start_at_ms: Option<i64>,
    pub last_start_error: Option<String>,
    pub start_attempt_count: u64,
    pub last_start_result: RuntimeStartResult,
    pub stop_state: RuntimeStopState,
    pub last_stop_at_ms: Option<i64>,
    pub last_stop_error: Option<String>,
    pub stop_attempt_count: u64,
    pub last_stop_result: RuntimeStopResult,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum RuntimeStartState {
    Stopped,
    Starting,
    Started,
    Failed,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum RuntimeStartResult {
    None,
    Started,
    AlreadyStarted,
    Failed,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum RuntimeStopState {
    Stopped,
    Stopping,
    Failed,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum RuntimeStopResult {
    None,
    Stopped,
    AlreadyStopped,
    Failed,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct MarkoutStatusSummary {
    pub enabled: bool,
    pub horizons_ms: Vec<u64>,
    pub pending_samples: usize,
    pub resolved_samples: usize,
    pub expired_samples: usize,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SweepStatusSummary {
    pub enabled: bool,
    pub windows_ms: Vec<u64>,
    pub last_direction: SweepDirection,
    pub last_sweep_detected: bool,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct VpinStatusSummary {
    pub enabled: bool,
    pub bucket_size_btc: f64,
    pub completed_bucket_count: usize,
    pub vpin: Option<f64>,
    pub vpin_spike: bool,
    pub vpin_high: bool,
    pub vpin_extreme: bool,
    pub dominant_direction: VpinDirection,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct LiquidationStatusSummary {
    pub enabled: bool,
    pub nearest_cluster_side: Option<LiquidationClusterSide>,
    pub distance_bps: Option<f64>,
    pub liq_hunt_pressure: f64,
    pub liq_cluster_nearby: bool,
    pub possible_liq_hunt_setup: bool,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct LiqHuntStatusSummary {
    pub enabled: bool,
    pub level: crate::types::liq_hunt::LiqHuntSignalLevel,
    pub direction: crate::types::liq_hunt::LiqHuntDirection,
    pub score: f64,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ToxicStatusSummary {
    pub enabled: bool,
    pub threshold_btc: f64,
    pub latest_direction: ToxicDirection,
    pub latest_toxic_volume_btc: f64,
    pub latest_alert_triggered: bool,
    pub recent_event_count: usize,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AlertStatusSummary {
    pub telegram_enabled: bool,
    pub last_sent_ts: Option<i64>,
    pub sent_count: u64,
    pub suppressed_count: u64,
    pub last_error: Option<String>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct StorageStatusSummary {
    pub enabled: bool,
    pub status: String,
    pub sqlite_path: String,
    pub last_write_ts: Option<i64>,
    pub last_error: Option<String>,
}

pub fn empty_venue_health_map() -> VenueHealthMap {
    Venue::ALL
        .into_iter()
        .map(|venue| (venue.as_key().to_string(), VenueHealth::disabled(venue)))
        .collect()
}
