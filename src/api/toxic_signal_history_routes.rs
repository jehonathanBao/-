use axum::{
    extract::{Path, Query, State},
    Json,
};
use serde::Deserialize;

use crate::{
    api::{
        toxic_signal_alert_preview_routes::build_preview as build_alert_preview,
        toxic_signal_group_routes::build_recent as build_group_recent,
        toxic_signal_inbox_routes::{
            build_recent as build_inbox_recent, normalize_symbol_query, normalize_symbol_text,
        },
        toxic_signal_report_routes::build_daily_report,
    },
    app::AppState,
};

#[derive(Debug, Deserialize)]
pub struct ToxicSignalHistoryQuery {
    symbol: Option<String>,
}

pub async fn toxic_signal_history_status_route(
    State(state): State<AppState>,
    Query(query): Query<ToxicSignalHistoryQuery>,
) -> Json<serde_json::Value> {
    let requested_symbol = normalize_symbol_query(query.symbol, &state.config().symbol);
    ensure_signal_history_snapshot(&state, &requested_symbol);
    Json(serde_json::json!(state.signal_history_service().status()))
}

pub async fn toxic_signal_history_recent_route(
    State(state): State<AppState>,
    Query(query): Query<ToxicSignalHistoryQuery>,
) -> Json<serde_json::Value> {
    let requested_symbol = normalize_symbol_query(query.symbol, &state.config().symbol);
    ensure_signal_history_snapshot(&state, &requested_symbol);
    Json(serde_json::json!(state
        .signal_history_service()
        .recent(&requested_symbol)))
}

pub async fn toxic_signal_history_for_symbol(
    State(state): State<AppState>,
    Path(symbol): Path<String>,
) -> Json<serde_json::Value> {
    let requested_symbol = normalize_symbol_text(&symbol, &state.config().symbol);
    ensure_signal_history_snapshot(&state, &requested_symbol);
    Json(serde_json::json!(state
        .signal_history_service()
        .recent(&requested_symbol)))
}

pub async fn toxic_signal_history_signal_route(
    State(state): State<AppState>,
    Path(signal_id): Path<String>,
    Query(query): Query<ToxicSignalHistoryQuery>,
) -> Json<serde_json::Value> {
    let requested_symbol = normalize_symbol_query(query.symbol, &state.config().symbol);
    ensure_signal_history_snapshot(&state, &requested_symbol);
    Json(serde_json::json!(state
        .signal_history_service()
        .signal_by_id(&signal_id)))
}

pub async fn toxic_signal_history_alert_recent_route(
    State(state): State<AppState>,
    Query(query): Query<ToxicSignalHistoryQuery>,
) -> Json<serde_json::Value> {
    let requested_symbol = normalize_symbol_query(query.symbol, &state.config().symbol);
    ensure_signal_history_snapshot(&state, &requested_symbol);
    Json(serde_json::json!(state
        .signal_history_service()
        .recent_alerts(&requested_symbol)))
}

pub async fn toxic_signal_history_report_recent_route(
    State(state): State<AppState>,
    Query(query): Query<ToxicSignalHistoryQuery>,
) -> Json<serde_json::Value> {
    let requested_symbol = normalize_symbol_query(query.symbol, &state.config().symbol);
    ensure_signal_history_snapshot(&state, &requested_symbol);
    Json(serde_json::json!(state
        .signal_history_service()
        .recent_reports(&requested_symbol)))
}

pub(crate) fn ensure_signal_history_snapshot(state: &AppState, requested_symbol: &str) {
    let inbox_recent = build_inbox_recent(state, requested_symbol);
    let group_recent = build_group_recent(state, requested_symbol);
    let alert_preview = build_alert_preview(state, requested_symbol);
    let daily_report = build_daily_report(state, requested_symbol);
    record_current_snapshot(
        state,
        requested_symbol,
        &inbox_recent,
        &group_recent,
        &alert_preview,
        &daily_report,
    );
}

pub(crate) fn record_current_snapshot(
    state: &AppState,
    _requested_symbol: &str,
    inbox_recent: &crate::types::toxic_signal_inbox::ToxicSignalInboxRecentResponse,
    group_recent: &crate::types::toxic_signal_group::ToxicSignalGroupRecentResponse,
    alert_preview: &crate::types::toxic_signal_alert_preview::ToxicSignalAlertPreviewResponse,
    daily_report: &crate::types::toxic_signal_report::ToxicSignalReportDailyResponse,
) {
    let history_recorded_at_ms = crate::normalizers::trade::now_ms().max(0) as u64;

    state.signal_history_service().record_snapshot(
        history_recorded_at_ms,
        inbox_recent,
        group_recent,
        alert_preview,
        daily_report,
    );
}
