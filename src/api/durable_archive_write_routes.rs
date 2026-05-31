use axum::{extract::State, Json};

use crate::{
    app::AppState,
    toxicity::durable_archive_write_service::{
        durable_archive_write_reject, durable_archive_write_status,
    },
    types::durable_archive_write::DurableArchiveWriteRequest,
};

pub async fn durable_archive_write_status_route(
    State(_state): State<AppState>,
) -> Json<serde_json::Value> {
    Json(serde_json::json!(durable_archive_write_status()))
}

pub async fn durable_archive_write_route(
    State(_state): State<AppState>,
    request_contract: Option<Json<DurableArchiveWriteRequest>>,
) -> Json<serde_json::Value> {
    Json(serde_json::json!(durable_archive_write_reject(
        request_contract.map(|Json(payload)| payload),
    )))
}
