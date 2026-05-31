use axum::{
    extract::{Path, Query, State},
    Json,
};
use serde::Deserialize;

use crate::{
    api::toxic_signal_inbox_routes::{
        build_recent as build_inbox_recent, normalize_symbol_query, normalize_symbol_text,
        with_filter_contract,
    },
    app::AppState,
    toxicity::toxic_signal_group_service::{
        toxic_signal_group_detail, toxic_signal_group_recent, toxic_signal_group_status,
    },
    types::toxic_signal_group::ToxicSignalGroupRecentResponse,
};

#[derive(Debug, Deserialize)]
pub struct ToxicSignalGroupQuery {
    symbol: Option<String>,
}

pub async fn toxic_signal_group_status_route(
    State(state): State<AppState>,
    Query(query): Query<ToxicSignalGroupQuery>,
) -> Json<serde_json::Value> {
    let requested_symbol = normalize_symbol_query(query.symbol, &state.config().symbol);
    let recent = build_recent(&state, &requested_symbol);
    Json(with_filter_contract(
        serde_json::json!(toxic_signal_group_status(&recent)),
        &requested_symbol,
    ))
}

pub async fn toxic_signal_group_recent_route(
    State(state): State<AppState>,
    Query(query): Query<ToxicSignalGroupQuery>,
) -> Json<serde_json::Value> {
    let requested_symbol = normalize_symbol_query(query.symbol, &state.config().symbol);
    Json(with_filter_contract(
        serde_json::json!(build_recent(&state, &requested_symbol)),
        &requested_symbol,
    ))
}

pub async fn toxic_signal_group_for_symbol(
    State(state): State<AppState>,
    Path(symbol): Path<String>,
) -> Json<serde_json::Value> {
    let requested_symbol = normalize_symbol_text(&symbol, &state.config().symbol);
    Json(with_filter_contract(
        serde_json::json!(build_recent(&state, &requested_symbol)),
        &requested_symbol,
    ))
}

pub async fn toxic_signal_group_detail_route(
    State(state): State<AppState>,
    Path(group_id): Path<String>,
    Query(query): Query<ToxicSignalGroupQuery>,
) -> Json<serde_json::Value> {
    let requested_symbol = normalize_symbol_query(query.symbol, &state.config().symbol);
    let recent = build_recent(&state, &requested_symbol);
    Json(serde_json::json!(toxic_signal_group_detail(
        &requested_symbol,
        &group_id,
        &recent,
    )))
}

pub(crate) fn build_recent(
    state: &AppState,
    requested_symbol: &str,
) -> ToxicSignalGroupRecentResponse {
    let inbox_recent = build_inbox_recent(state, requested_symbol);
    toxic_signal_group_recent(requested_symbol, &inbox_recent)
}
