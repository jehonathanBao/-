use axum::{
    body::Bytes,
    extract::{Path, Query, State},
    Json,
};
use serde::Deserialize;

use crate::{
    api::{
        toxic_signal_alert_preview_routes::build_preview as build_alert_preview,
        toxic_signal_group_routes::build_recent as build_group_recent,
        toxic_signal_history_routes::record_current_snapshot,
        toxic_signal_inbox_routes::{build_recent as build_inbox_recent, normalize_symbol_query},
        toxic_signal_report_routes::build_daily_report,
    },
    app::AppState,
    toxicity::durable_archive_dryrun_service::{
        durable_archive_dryrun_review_pack_by_id, durable_archive_dryrun_review_pack_latest,
        durable_archive_dryrun_validate_payload, durable_archive_dryrun_write,
    },
};

#[derive(Debug, Deserialize)]
pub struct DurableArchiveDryRunQuery {
    symbol: Option<String>,
}

pub async fn durable_archive_dryrun_write_route(
    State(state): State<AppState>,
    Query(query): Query<DurableArchiveDryRunQuery>,
    body: Bytes,
) -> Json<serde_json::Value> {
    let requested_symbol = normalize_symbol_query(query.symbol, &state.config().symbol);

    if !body.is_empty() {
        let payload = serde_json::from_slice::<serde_json::Value>(&body)
            .unwrap_or_else(|_| serde_json::json!({ "records": [] }));
        return Json(serde_json::json!(durable_archive_dryrun_validate_payload(
            &requested_symbol,
            &payload,
        )));
    }

    Json(serde_json::json!(build_latest_dry_run_response(
        &state,
        requested_symbol.as_str(),
    )))
}

pub async fn durable_archive_dryrun_review_pack_latest_route(
    State(state): State<AppState>,
    Query(query): Query<DurableArchiveDryRunQuery>,
) -> Json<serde_json::Value> {
    let requested_symbol = normalize_symbol_query(query.symbol, &state.config().symbol);
    let payload = build_latest_dry_run_response(&state, requested_symbol.as_str());
    Json(serde_json::json!(
        durable_archive_dryrun_review_pack_latest(&payload,)
    ))
}

pub async fn durable_archive_dryrun_review_pack_by_id_route(
    State(state): State<AppState>,
    Query(query): Query<DurableArchiveDryRunQuery>,
    Path(dry_run_id): Path<String>,
) -> Json<serde_json::Value> {
    let requested_symbol = normalize_symbol_query(query.symbol, &state.config().symbol);
    let payload = build_latest_dry_run_response(&state, requested_symbol.as_str());
    Json(serde_json::json!(durable_archive_dryrun_review_pack_by_id(
        requested_symbol.as_str(),
        &payload,
        &dry_run_id,
    )))
}

fn build_latest_dry_run_response(
    state: &AppState,
    requested_symbol: &str,
) -> crate::types::durable_archive_dryrun::DurableArchiveDryRunResponse {
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

    let history_recent = state.signal_history_service().recent(requested_symbol);
    let alert_recent = state
        .signal_history_service()
        .recent_alerts(requested_symbol);
    let report_recent = state
        .signal_history_service()
        .recent_reports(requested_symbol);

    durable_archive_dryrun_write(
        requested_symbol,
        &history_recent,
        &alert_recent,
        &report_recent,
    )
}
