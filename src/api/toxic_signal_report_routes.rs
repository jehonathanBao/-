use axum::{
    extract::{Query, State},
    Json,
};
use chrono::Local;
use serde::Deserialize;

use crate::{
    api::{
        toxic_quality_scorecard_routes::build_summary as build_quality_summary,
        toxic_signal_alert_preview_routes::build_preview,
        toxic_signal_group_routes::build_recent as build_group_recent,
        toxic_signal_history_routes::record_current_snapshot,
        toxic_signal_inbox_routes::{build_recent as build_inbox_recent, normalize_symbol_query},
    },
    app::AppState,
    toxicity::{
        toxic_signal_report_service::{
            toxic_signal_daily_report, toxic_signal_report_status, toxic_signal_rolling_report,
        },
        toxic_weight_recommendation_service::toxic_weight_recommendation_summary,
    },
};

#[derive(Debug, Deserialize)]
pub struct ToxicSignalReportQuery {
    symbol: Option<String>,
    window: Option<String>,
}

pub async fn toxic_signal_report_status_route(
    State(state): State<AppState>,
    Query(query): Query<ToxicSignalReportQuery>,
) -> Json<serde_json::Value> {
    let requested_symbol = normalize_symbol_query(query.symbol, &state.config().symbol);
    let report_date = current_report_date();
    let inbox_recent = build_inbox_recent(&state, &requested_symbol);
    let group_recent = build_group_recent(&state, &requested_symbol);

    Json(serde_json::json!(toxic_signal_report_status(
        &requested_symbol,
        &report_date,
        &inbox_recent,
        &group_recent,
    )))
}

pub async fn toxic_signal_report_daily_route(
    State(state): State<AppState>,
    Query(query): Query<ToxicSignalReportQuery>,
) -> Json<serde_json::Value> {
    let requested_symbol = normalize_symbol_query(query.symbol, &state.config().symbol);
    Json(serde_json::json!(build_daily_report(
        &state,
        &requested_symbol
    )))
}

pub async fn toxic_signal_report_rolling_route(
    State(state): State<AppState>,
    Query(query): Query<ToxicSignalReportQuery>,
) -> Json<serde_json::Value> {
    let requested_symbol = normalize_symbol_query(query.symbol, &state.config().symbol);
    let normalized_window = normalize_window(query.window);
    Json(serde_json::json!(build_rolling_report(
        &state,
        &requested_symbol,
        &normalized_window,
    )))
}

pub(crate) fn build_daily_report(
    state: &AppState,
    requested_symbol: &str,
) -> crate::types::toxic_signal_report::ToxicSignalReportDailyResponse {
    let report_date = current_report_date();
    let inbox_recent = build_inbox_recent(state, requested_symbol);
    let group_recent = build_group_recent(state, requested_symbol);
    let quality_summary = build_quality_summary(state, requested_symbol);
    let fusion_recent =
        crate::api::toxic_quality_scorecard_routes::build_fusion_recent(state, requested_symbol);
    let recommendation_summary = toxic_weight_recommendation_summary(
        requested_symbol,
        &fusion_recent,
        |ts| state.price_snapshot_at_or_before_for_symbol(ts, requested_symbol),
        |ts| state.price_snapshots_since_for_symbol(ts, requested_symbol),
    );

    toxic_signal_daily_report(
        requested_symbol,
        &report_date,
        &inbox_recent,
        &group_recent,
        &quality_summary,
        &recommendation_summary,
    )
}

pub(crate) fn build_rolling_report(
    state: &AppState,
    requested_symbol: &str,
    window: &str,
) -> crate::types::toxic_signal_report::ToxicSignalReportRollingResponse {
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

    let signal_history = state.signal_history_service().recent(requested_symbol);
    let alert_history = state
        .signal_history_service()
        .recent_alerts(requested_symbol);
    let now_ms = crate::normalizers::trade::now_ms().max(0) as u64;
    let window_start_ms = rolling_window_start_ms(window, now_ms);
    let filtered_signals = signal_history
        .items
        .into_iter()
        .filter(|item| item.history_recorded_at_ms >= window_start_ms)
        .collect::<Vec<_>>();
    let filtered_alerts = alert_history
        .items
        .into_iter()
        .filter(|item| item.history_recorded_at_ms >= window_start_ms)
        .collect::<Vec<_>>();

    toxic_signal_rolling_report(
        requested_symbol,
        window,
        &filtered_signals,
        &filtered_alerts,
    )
}

pub(crate) fn current_report_date() -> String {
    Local::now().format("%Y-%m-%d").to_string()
}

fn normalize_window(window: Option<String>) -> String {
    match window.as_deref().map(str::trim) {
        Some("7d") | None => "7d".to_string(),
        Some("") => "7d".to_string(),
        Some(other) => other.to_ascii_lowercase(),
    }
}

fn rolling_window_start_ms(window: &str, now_ms: u64) -> u64 {
    match window {
        "7d" => now_ms.saturating_sub(7 * 24 * 60 * 60 * 1000),
        _ => 0,
    }
}
