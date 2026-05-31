use axum::{extract::State, Json};

use crate::{
    app::AppState,
    toxicity::durable_archive_write_audit_service::{
        durable_archive_write_audit_latest, durable_archive_write_audit_recent,
        durable_archive_write_audit_status,
    },
};

pub async fn durable_archive_write_audit_status_route(
    State(_state): State<AppState>,
) -> Json<serde_json::Value> {
    Json(serde_json::json!(durable_archive_write_audit_status()))
}

pub async fn durable_archive_write_audit_recent_route(
    State(_state): State<AppState>,
) -> Json<serde_json::Value> {
    Json(serde_json::json!(durable_archive_write_audit_recent()))
}

pub async fn durable_archive_write_audit_latest_route(
    State(_state): State<AppState>,
) -> Json<serde_json::Value> {
    Json(serde_json::json!(durable_archive_write_audit_latest()))
}
