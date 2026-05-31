use axum::{
    extract::{Path, Query, State},
    Json,
};
use serde::Deserialize;

use crate::{
    api::{
        toxic_signal_alert_preview_routes::build_preview,
        toxic_signal_group_routes::build_recent as build_group_recent,
        toxic_signal_history_routes::record_current_snapshot,
        toxic_signal_inbox_routes::{
            build_recent as build_inbox_recent, normalize_symbol_query, normalize_symbol_text,
        },
        toxic_signal_report_routes::build_daily_report,
    },
    app::AppState,
    toxicity::toxic_signal_health_service::{
        toxic_signal_health_status, toxic_signal_health_summary,
    },
};

#[derive(Debug, Deserialize)]
pub struct ToxicSignalHealthQuery {
    symbol: Option<String>,
}

pub async fn toxic_signal_health_status_route(
    State(state): State<AppState>,
    Query(query): Query<ToxicSignalHealthQuery>,
) -> Json<serde_json::Value> {
    let requested_symbol = normalize_symbol_query(query.symbol, &state.config().symbol);
    let summary = build_summary(&state, &requested_symbol);
    Json(serde_json::json!(toxic_signal_health_status(&summary)))
}

pub async fn toxic_signal_health_summary_route(
    State(state): State<AppState>,
    Query(query): Query<ToxicSignalHealthQuery>,
) -> Json<serde_json::Value> {
    let requested_symbol = normalize_symbol_query(query.symbol, &state.config().symbol);
    Json(serde_json::json!(build_summary(&state, &requested_symbol)))
}

pub async fn toxic_signal_health_for_symbol(
    State(state): State<AppState>,
    Path(symbol): Path<String>,
) -> Json<serde_json::Value> {
    let requested_symbol = normalize_symbol_text(&symbol, &state.config().symbol);
    Json(serde_json::json!(build_summary(&state, &requested_symbol)))
}

pub(crate) fn build_summary(
    state: &AppState,
    requested_symbol: &str,
) -> crate::types::toxic_signal_health::ToxicSignalHealthSummaryResponse {
    let inbox_recent = build_inbox_recent(state, requested_symbol);
    let group_recent = build_group_recent(state, requested_symbol);
    let alert_preview = build_preview(state, requested_symbol);
    let daily_report = build_daily_report(state, requested_symbol);

    record_current_snapshot(
        state,
        requested_symbol,
        &inbox_recent,
        &group_recent,
        &alert_preview,
        &daily_report,
    );

    let history_status = state.signal_history_service().status();
    let history_recent = state.signal_history_service().recent(requested_symbol);

    toxic_signal_health_summary(
        requested_symbol,
        &inbox_recent,
        &group_recent,
        &daily_report,
        &alert_preview,
        &history_status,
        &history_recent,
    )
}
