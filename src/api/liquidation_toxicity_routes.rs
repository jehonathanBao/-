use axum::{
    extract::{Query, State},
    Json,
};
use serde::Deserialize;

use crate::{
    app::AppState,
    toxicity::liquidation_toxicity_service::{
        build_liquidation_toxicity_recent, build_liquidation_toxicity_status,
    },
};

#[derive(Debug, Deserialize)]
pub struct LiquidationToxicityQuery {
    symbol: Option<String>,
}

pub async fn liquidation_toxicity_status(
    State(state): State<AppState>,
    Query(query): Query<LiquidationToxicityQuery>,
) -> Json<serde_json::Value> {
    let requested_symbol = query
        .symbol
        .unwrap_or_else(|| state.config().symbol.clone());
    Json(serde_json::json!(build_liquidation_toxicity_status(
        &requested_symbol,
        &state.liquidation_state(),
        &state.flow_state(),
        &state.sweep_state(),
        &state.markout_state(),
    )))
}

pub async fn liquidation_toxicity_recent(
    State(state): State<AppState>,
    Query(query): Query<LiquidationToxicityQuery>,
) -> Json<serde_json::Value> {
    let requested_symbol = query
        .symbol
        .unwrap_or_else(|| state.config().symbol.clone());
    Json(serde_json::json!(build_liquidation_toxicity_recent(
        &requested_symbol,
        &state.liquidation_state(),
        &state.flow_state(),
        &state.sweep_state(),
        &state.markout_state(),
    )))
}
