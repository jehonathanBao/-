use axum::{
    extract::{Path, Query, State},
    Json,
};
use serde::Deserialize;

use crate::{
    app::AppState,
    toxicity::{
        active_trade_toxicity::analyze_active_trade_toxicity,
        active_trade_toxicity_service::{
            build_active_trade_toxicity_recent, build_active_trade_toxicity_status,
        },
    },
};

#[derive(Debug, Deserialize)]
pub struct ActiveTradeQuery {
    symbol: Option<String>,
}

pub async fn active_trade_toxicity_status(
    State(state): State<AppState>,
    Query(query): Query<ActiveTradeQuery>,
) -> Json<serde_json::Value> {
    let requested_symbol = query
        .symbol
        .unwrap_or_else(|| state.config().symbol.clone());
    Json(serde_json::json!(build_active_trade_toxicity_status(
        &requested_symbol,
        &state.flow_state(),
        &state.sweep_state(),
        &state.markout_state(),
    )))
}

pub async fn active_trade_toxicity_recent(
    State(state): State<AppState>,
    Query(query): Query<ActiveTradeQuery>,
) -> Json<serde_json::Value> {
    let requested_symbol = query
        .symbol
        .unwrap_or_else(|| state.config().symbol.clone());
    Json(serde_json::json!(build_active_trade_toxicity_recent(
        &requested_symbol,
        &state.flow_state(),
        &state.sweep_state(),
        &state.markout_state(),
    )))
}

pub async fn active_trade_toxicity(State(state): State<AppState>) -> Json<serde_json::Value> {
    Json(serde_json::json!(build_report(&state, None)))
}

pub async fn active_trade_toxicity_for_symbol(
    State(state): State<AppState>,
    Path(symbol): Path<String>,
) -> Json<serde_json::Value> {
    Json(serde_json::json!(build_report(&state, Some(symbol))))
}

fn build_report(
    state: &AppState,
    symbol: Option<String>,
) -> crate::types::toxic_flow::ActiveTradeToxicityReport {
    let requested_symbol = symbol.unwrap_or_else(|| state.config().symbol.clone());
    analyze_active_trade_toxicity(&requested_symbol, &state.flow_state(), &state.sweep_state())
}
