use axum::{
    extract::{Path, State},
    http::StatusCode,
    response::IntoResponse,
    Json,
};

use crate::{app::AppState, calibration::parameter_patch_diff::ParameterPatchDiffStore};

pub async fn latest_parameter_export_diff(State(state): State<AppState>) -> impl IntoResponse {
    let store = diff_store(&state);
    match store.latest_diff() {
        Ok(Some(diff)) => Json(serde_json::json!({
            "applyMode": "manual_only",
            "runtimeModified": false,
            "diff": diff
        }))
        .into_response(),
        Ok(None) => json_reason(StatusCode::NOT_FOUND, "no_exports_found"),
        Err(err) if err.to_string() == "current_config_unavailable" => json_reason(
            StatusCode::SERVICE_UNAVAILABLE,
            "current_config_unavailable",
        ),
        Err(err) => json_error(StatusCode::INTERNAL_SERVER_ERROR, err.to_string()),
    }
}

pub async fn parameter_export_diff_by_id(
    State(state): State<AppState>,
    Path(export_id): Path<String>,
) -> impl IntoResponse {
    let store = diff_store(&state);
    match store.diff_by_id(&export_id) {
        Ok(Some(diff)) => Json(serde_json::json!({
            "applyMode": "manual_only",
            "runtimeModified": false,
            "diff": diff
        }))
        .into_response(),
        Ok(None) => json_reason(StatusCode::NOT_FOUND, "export_not_found"),
        Err(err) if err.to_string() == "current_config_unavailable" => json_reason(
            StatusCode::SERVICE_UNAVAILABLE,
            "current_config_unavailable",
        ),
        Err(err) => json_error(StatusCode::INTERNAL_SERVER_ERROR, err.to_string()),
    }
}

pub async fn latest_parameter_export_audit(State(state): State<AppState>) -> impl IntoResponse {
    let store = diff_store(&state);
    match store.latest_audit() {
        Ok(Some(audit)) => Json(serde_json::json!({
            "applyMode": "manual_only",
            "runtimeModified": false,
            "audit": audit
        }))
        .into_response(),
        Ok(None) => json_reason(StatusCode::NOT_FOUND, "no_exports_found"),
        Err(err) if err.to_string() == "current_config_unavailable" => json_reason(
            StatusCode::SERVICE_UNAVAILABLE,
            "current_config_unavailable",
        ),
        Err(err) => json_error(StatusCode::INTERNAL_SERVER_ERROR, err.to_string()),
    }
}

pub async fn parameter_export_audit_by_id(
    State(state): State<AppState>,
    Path(export_id): Path<String>,
) -> impl IntoResponse {
    let store = diff_store(&state);
    match store.audit_by_id(&export_id) {
        Ok(Some(audit)) => Json(serde_json::json!({
            "applyMode": "manual_only",
            "runtimeModified": false,
            "audit": audit
        }))
        .into_response(),
        Ok(None) => json_reason(StatusCode::NOT_FOUND, "export_not_found"),
        Err(err) if err.to_string() == "current_config_unavailable" => json_reason(
            StatusCode::SERVICE_UNAVAILABLE,
            "current_config_unavailable",
        ),
        Err(err) => json_error(StatusCode::INTERNAL_SERVER_ERROR, err.to_string()),
    }
}

fn diff_store(state: &AppState) -> ParameterPatchDiffStore {
    ParameterPatchDiffStore::new(
        state.config().replay_report_dir.clone(),
        Some(state.config().clone()),
    )
}

fn json_reason(status: StatusCode, reason: &str) -> axum::response::Response {
    (
        status,
        Json(serde_json::json!({
            "reason": reason,
            "applyMode": "manual_only",
            "runtimeModified": false
        })),
    )
        .into_response()
}

fn json_error(status: StatusCode, error: String) -> axum::response::Response {
    (
        status,
        Json(serde_json::json!({
            "error": error,
            "applyMode": "manual_only",
            "runtimeModified": false
        })),
    )
        .into_response()
}
