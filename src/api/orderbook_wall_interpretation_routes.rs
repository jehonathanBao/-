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
        orderbook_wall_interpretation_service::{
            build_orderbook_wall_interpretation_recent, build_orderbook_wall_interpretation_status,
        },
        orderbook_wall_lifecycle::build_orderbook_wall_lifecycle_report,
    },
    types::orderbook_wall::{OrderbookWallInterpretationReport, OrderbookWallLifecycleReport},
};

#[derive(Debug, Deserialize)]
pub struct OrderbookWallInterpretationQuery {
    symbol: Option<String>,
}

pub async fn orderbook_wall_interpretation_status(
    State(state): State<AppState>,
    Query(query): Query<OrderbookWallInterpretationQuery>,
) -> Json<serde_json::Value> {
    let requested_symbol = query
        .symbol
        .unwrap_or_else(|| state.config().symbol.clone());
    let lifecycle_report = build_lifecycle_report(&state, &requested_symbol);
    let active_trade_recent = build_active_trade_toxicity_recent(
        &requested_symbol,
        &state.flow_state(),
        &state.sweep_state(),
        &state.markout_state(),
    );
    let liquidation_recent = build_liquidation_toxicity_recent(
        &requested_symbol,
        &state.liquidation_state(),
        &state.flow_state(),
        &state.sweep_state(),
        &state.markout_state(),
    );
    Json(serde_json::json!(
        build_orderbook_wall_interpretation_status(
            &requested_symbol,
            &lifecycle_report,
            &active_trade_recent,
            &liquidation_recent,
        )
    ))
}

pub async fn orderbook_wall_interpretation_recent(
    State(state): State<AppState>,
    Query(query): Query<OrderbookWallInterpretationQuery>,
) -> Json<serde_json::Value> {
    let requested_symbol = query
        .symbol
        .unwrap_or_else(|| state.config().symbol.clone());
    Json(serde_json::json!(build_report(&state, &requested_symbol)))
}

pub async fn orderbook_wall_interpretation_for_symbol(
    State(state): State<AppState>,
    Path(symbol): Path<String>,
) -> Json<serde_json::Value> {
    Json(serde_json::json!(build_report(&state, &symbol)))
}

fn build_report(state: &AppState, requested_symbol: &str) -> OrderbookWallInterpretationReport {
    let lifecycle_report = build_lifecycle_report(state, requested_symbol);
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
    build_orderbook_wall_interpretation_recent(
        requested_symbol,
        &lifecycle_report,
        &active_trade_recent,
        &liquidation_recent,
    )
}

fn build_lifecycle_report(
    state: &AppState,
    requested_symbol: &str,
) -> OrderbookWallLifecycleReport {
    let wall_state = state.orderbook_wall_lifecycle_state();
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
