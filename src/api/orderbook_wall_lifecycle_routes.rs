use axum::{
    extract::{Path, Query, State},
    Json,
};
use serde::Deserialize;

use crate::{
    app::AppState,
    toxicity::{
        active_trade_toxicity_service::build_active_trade_toxicity_recent,
        liquidation_toxicity_service::build_liquidation_toxicity_recent,
        orderbook_wall_lifecycle::build_orderbook_wall_lifecycle_report,
    },
    types::orderbook_wall::{OrderbookWallLifecycleReport, OrderbookWallLifecycleStatusResponse},
};

#[derive(Debug, Deserialize)]
pub struct OrderbookWallQuery {
    symbol: Option<String>,
}

pub async fn orderbook_wall_lifecycle_status(
    State(state): State<AppState>,
    Query(query): Query<OrderbookWallQuery>,
) -> Json<serde_json::Value> {
    let requested_symbol = query
        .symbol
        .unwrap_or_else(|| state.config().symbol.clone());
    let report = build_report(&state, &requested_symbol);
    Json(serde_json::json!(OrderbookWallLifecycleStatusResponse {
        read_only: true,
        runtime_modified: false,
        analysis_mode: "analysis_only".to_string(),
        enabled: true,
        selected_symbol: requested_symbol,
        status: report.status.clone(),
        tracked_wall_count: report.tracked_walls.len(),
        recent_event_count: report.recent_events.len(),
        candidate_count: report.toxicity_candidates.len(),
        last_event_at_ms: report
            .recent_events
            .iter()
            .map(|event| event.observed_at_ms)
            .max(),
        safety_boundary: vec![
            "readOnly=true".to_string(),
            "runtimeModified=false".to_string(),
            "analysis_only".to_string(),
            "No order execution".to_string(),
            "No wallet".to_string(),
            "No signing".to_string(),
            "No transaction construction".to_string(),
        ],
    }))
}

pub async fn orderbook_wall_lifecycle_recent(
    State(state): State<AppState>,
    Query(query): Query<OrderbookWallQuery>,
) -> Json<serde_json::Value> {
    let requested_symbol = query
        .symbol
        .unwrap_or_else(|| state.config().symbol.clone());
    Json(serde_json::json!(build_report(&state, &requested_symbol)))
}

pub async fn orderbook_wall_lifecycle_for_symbol(
    State(state): State<AppState>,
    Path(symbol): Path<String>,
) -> Json<serde_json::Value> {
    Json(serde_json::json!(build_report(&state, &symbol)))
}

fn build_report(state: &AppState, requested_symbol: &str) -> OrderbookWallLifecycleReport {
    let wall_state = state.orderbook_wall_lifecycle_state();
    if !wall_state.symbol.eq_ignore_ascii_case(requested_symbol) {
        return OrderbookWallLifecycleReport {
            read_only: true,
            runtime_modified: false,
            analysis_mode: "analysis_only".to_string(),
            symbol: requested_symbol.to_string(),
            generated_at_ms: wall_state.generated_at_ms,
            status: "insufficient_data".to_string(),
            tracked_walls: Vec::new(),
            recent_events: Vec::new(),
            toxicity_candidates: Vec::new(),
            warnings: vec![format!(
                "runtime wall tracker is currently scoped to {}",
                wall_state.symbol
            )],
            no_trade_reasons: vec![
                "selected symbol is not the runtime wall-tracking symbol".to_string()
            ],
        };
    }
    let active_trade_recent = build_active_trade_toxicity_recent(
        requested_symbol,
        &state.flow_state(),
        &state.sweep_state(),
        &state.markout_state(),
    );
    let liquidation_recent = build_liquidation_toxicity_recent(
        requested_symbol,
        &state.liquidation_state(),
        &state.flow_state(),
        &state.sweep_state(),
        &state.markout_state(),
    );
    build_orderbook_wall_lifecycle_report(&wall_state, &active_trade_recent, &liquidation_recent)
}
