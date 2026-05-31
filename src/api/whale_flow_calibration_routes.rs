use axum::{
    extract::{Path, Query, State},
    Json,
};
use serde::Deserialize;

use crate::{
    api::{
        toxic_markout_routes::build_recent as build_markout_recent,
        whale_flow_routes::build_current_report as build_current_whale_flow_report,
    },
    app::AppState,
    toxicity::whale_flow_calibration_service::{
        build_whale_flow_threshold_calibration_report,
        build_whale_flow_threshold_calibration_status,
    },
    types::whale_flow_calibration::WhaleFlowCalibrationReportResponse,
};

#[derive(Debug, Deserialize)]
pub struct WhaleFlowCalibrationQuery {
    symbol: Option<String>,
}

pub async fn whale_flow_calibration_status(
    State(state): State<AppState>,
    Query(query): Query<WhaleFlowCalibrationQuery>,
) -> Json<serde_json::Value> {
    let requested_symbol = query
        .symbol
        .unwrap_or_else(|| state.config().symbol.clone());
    let report = build_calibration_report(&state, &requested_symbol);
    Json(serde_json::json!(
        build_whale_flow_threshold_calibration_status(&report)
    ))
}

pub async fn whale_flow_calibration_report(
    State(state): State<AppState>,
    Query(query): Query<WhaleFlowCalibrationQuery>,
) -> Json<serde_json::Value> {
    let requested_symbol = query
        .symbol
        .unwrap_or_else(|| state.config().symbol.clone());
    Json(serde_json::json!(build_calibration_report(
        &state,
        &requested_symbol
    )))
}

pub async fn whale_flow_calibration_for_symbol(
    State(state): State<AppState>,
    Path(symbol): Path<String>,
) -> Json<serde_json::Value> {
    Json(serde_json::json!(build_calibration_report(&state, &symbol)))
}

fn build_calibration_report(
    state: &AppState,
    requested_symbol: &str,
) -> WhaleFlowCalibrationReportResponse {
    let mut whale_flow_recent = build_current_whale_flow_report(state, requested_symbol);
    let historical_candidates = state
        .whale_flow_candidate_history_service()
        .recent_candidates(requested_symbol);
    if !historical_candidates.is_empty() {
        whale_flow_recent.candidates = historical_candidates;
        whale_flow_recent.history_baseline_mode = "whale_candidate_history".to_string();
        whale_flow_recent
            .warnings
            .push("Calibration is using bounded in-memory whale candidate history.".to_string());
    }
    let markout_recent = build_markout_recent(state, requested_symbol);
    let history_status = state.signal_history_service().status();
    build_whale_flow_threshold_calibration_report(
        requested_symbol,
        &whale_flow_recent,
        &markout_recent,
        &history_status,
    )
}
