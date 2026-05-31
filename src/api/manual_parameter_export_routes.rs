use axum::{
    extract::{Path, State},
    http::StatusCode,
    response::IntoResponse,
    Json,
};

use crate::{
    app::AppState,
    calibration::manual_parameter_export::{
        ManualParameterExportRequest, ManualParameterExportStore,
    },
};

pub async fn parameter_exports(State(state): State<AppState>) -> impl IntoResponse {
    let store = export_store(&state);
    match store.list_exports() {
        Ok(exports) => Json(serde_json::json!({ "exports": exports })).into_response(),
        Err(err) => json_error(StatusCode::INTERNAL_SERVER_ERROR, err.to_string()),
    }
}

pub async fn latest_parameter_export(State(state): State<AppState>) -> impl IntoResponse {
    let store = export_store(&state);
    match store.latest_export() {
        Ok(Some(export)) => Json(serde_json::json!({ "export": export })).into_response(),
        Ok(None) => Json(serde_json::json!({ "export": null })).into_response(),
        Err(err) => json_error(StatusCode::INTERNAL_SERVER_ERROR, err.to_string()),
    }
}

pub async fn parameter_export_by_id(
    State(state): State<AppState>,
    Path(export_id): Path<String>,
) -> impl IntoResponse {
    let store = export_store(&state);
    match store.get_export(&export_id) {
        Ok(Some(export)) => Json(serde_json::json!({ "export": export })).into_response(),
        Ok(None) => json_error(StatusCode::NOT_FOUND, "export not found".to_string()),
        Err(err) => json_error(StatusCode::INTERNAL_SERVER_ERROR, err.to_string()),
    }
}

pub async fn create_parameter_export(
    State(state): State<AppState>,
    Json(request): Json<ManualParameterExportRequest>,
) -> impl IntoResponse {
    let store = export_store(&state);
    match store.create_export(request) {
        Ok(response) => Json(serde_json::json!(response)).into_response(),
        Err(err) => json_error(StatusCode::INTERNAL_SERVER_ERROR, err.to_string()),
    }
}

fn export_store(state: &AppState) -> ManualParameterExportStore {
    ManualParameterExportStore::new(state.config().replay_report_dir.clone())
}

fn json_error(status: StatusCode, error: String) -> axum::response::Response {
    (status, Json(serde_json::json!({ "error": error }))).into_response()
}
