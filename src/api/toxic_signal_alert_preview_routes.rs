use axum::{
    extract::{Path, Query, State},
    Json,
};
use serde::Deserialize;

use crate::{
    app::AppState,
    config::thresholds::AlertGateConfig,
    toxicity::toxic_signal_alert_preview_service::{
        toxic_signal_alert_explain, toxic_signal_alert_preview, toxic_signal_alert_preview_status,
    },
    types::toxic_signal_alert_preview::ToxicSignalAlertPreviewGate,
};

use super::toxic_signal_inbox_routes::{build_recent, normalize_symbol_query};

#[derive(Debug, Deserialize)]
pub struct ToxicSignalAlertPreviewQuery {
    symbol: Option<String>,
}

pub async fn toxic_signal_alert_preview_status_route(
    State(state): State<AppState>,
    Query(query): Query<ToxicSignalAlertPreviewQuery>,
) -> Json<serde_json::Value> {
    let requested_symbol = normalize_symbol_query(query.symbol, &state.config().symbol);
    let preview = build_preview(&state, &requested_symbol);
    Json(serde_json::json!(toxic_signal_alert_preview_status(
        &preview
    )))
}

pub async fn toxic_signal_alert_preview_recent_route(
    State(state): State<AppState>,
    Query(query): Query<ToxicSignalAlertPreviewQuery>,
) -> Json<serde_json::Value> {
    let requested_symbol = normalize_symbol_query(query.symbol, &state.config().symbol);
    Json(serde_json::json!(build_preview(&state, &requested_symbol)))
}

pub async fn toxic_signal_alert_preview_explain_route(
    State(state): State<AppState>,
    Path(signal_id): Path<String>,
    Query(query): Query<ToxicSignalAlertPreviewQuery>,
) -> Json<serde_json::Value> {
    let requested_symbol = normalize_symbol_query(query.symbol, &state.config().symbol);
    let inbox_recent = build_recent(&state, &requested_symbol);
    let gate = build_gate(&state);
    Json(serde_json::json!(toxic_signal_alert_explain(
        &signal_id,
        &inbox_recent,
        &gate,
    )))
}

pub(crate) fn build_preview(
    state: &AppState,
    requested_symbol: &str,
) -> crate::types::toxic_signal_alert_preview::ToxicSignalAlertPreviewResponse {
    let inbox_recent = build_recent(state, requested_symbol);
    toxic_signal_alert_preview(requested_symbol, &inbox_recent, build_gate(state))
}

fn build_gate(state: &AppState) -> ToxicSignalAlertPreviewGate {
    let config = state.config();
    let gate = AlertGateConfig {
        dedup_window_ms: config.alert_dedup_window_ms,
        min_severity: config.alert_min_severity,
        require_cross_venue: config.alert_require_cross_venue,
        require_markout: config.alert_require_markout,
        require_liquidity_drain: config.alert_require_liquidity_drain,
    };
    ToxicSignalAlertPreviewGate {
        dedup_window_ms: gate.dedup_window_ms,
        min_severity: gate.min_severity.label().to_string(),
        require_cross_venue: gate.require_cross_venue,
        require_markout: gate.require_markout,
        require_liquidity_drain: gate.require_liquidity_drain,
        telegram_enabled: state.alert_state().telegram_enabled,
        notification_sent: false,
        execution_triggered: false,
    }
}
