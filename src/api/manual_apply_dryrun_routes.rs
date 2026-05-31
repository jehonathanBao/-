use axum::{
    extract::{Path, State},
    http::{header, HeaderValue, StatusCode},
    response::IntoResponse,
    Json,
};

use crate::{
    app::AppState, calibration::manual_apply_dryrun_validator::ManualApplyDryRunValidator,
};

pub async fn latest_manual_apply_dry_run(State(state): State<AppState>) -> impl IntoResponse {
    let validator = validator(&state);
    match validator.latest_report() {
        Ok(Some(report)) => Json(serde_json::json!(report)).into_response(),
        Ok(None) => json_reason(StatusCode::NOT_FOUND, "no_exports_found"),
        Err(err) if err.to_string() == "current_config_unavailable" => json_reason(
            StatusCode::SERVICE_UNAVAILABLE,
            "current_config_unavailable",
        ),
        Err(err) => json_error(StatusCode::INTERNAL_SERVER_ERROR, err.to_string()),
    }
}

pub async fn manual_apply_dry_run_by_id(
    State(state): State<AppState>,
    Path(export_id): Path<String>,
) -> impl IntoResponse {
    let validator = validator(&state);
    match validator.report_by_id(&export_id) {
        Ok(Some(report)) => Json(serde_json::json!(report)).into_response(),
        Ok(None) => json_reason(StatusCode::NOT_FOUND, "export_not_found"),
        Err(err) if err.to_string() == "current_config_unavailable" => json_reason(
            StatusCode::SERVICE_UNAVAILABLE,
            "current_config_unavailable",
        ),
        Err(err) => json_error(StatusCode::INTERNAL_SERVER_ERROR, err.to_string()),
    }
}

pub async fn latest_manual_apply_dry_run_markdown(
    State(state): State<AppState>,
) -> impl IntoResponse {
    let validator = validator(&state);
    match validator.latest_markdown() {
        Ok(Some(markdown)) => (
            [(
                header::CONTENT_TYPE,
                HeaderValue::from_static("text/markdown; charset=utf-8"),
            )],
            markdown,
        )
            .into_response(),
        Ok(None) => (StatusCode::NOT_FOUND, "no_exports_found".to_string()).into_response(),
        Err(err) if err.to_string() == "current_config_unavailable" => (
            StatusCode::SERVICE_UNAVAILABLE,
            "current_config_unavailable".to_string(),
        )
            .into_response(),
        Err(err) => (StatusCode::INTERNAL_SERVER_ERROR, err.to_string()).into_response(),
    }
}

pub async fn manual_apply_dry_run_markdown_by_id(
    State(state): State<AppState>,
    Path(export_id): Path<String>,
) -> impl IntoResponse {
    let validator = validator(&state);
    match validator.markdown_by_id(&export_id) {
        Ok(Some(markdown)) => (
            [(
                header::CONTENT_TYPE,
                HeaderValue::from_static("text/markdown; charset=utf-8"),
            )],
            markdown,
        )
            .into_response(),
        Ok(None) => (StatusCode::NOT_FOUND, "export_not_found".to_string()).into_response(),
        Err(err) if err.to_string() == "current_config_unavailable" => (
            StatusCode::SERVICE_UNAVAILABLE,
            "current_config_unavailable".to_string(),
        )
            .into_response(),
        Err(err) => (StatusCode::INTERNAL_SERVER_ERROR, err.to_string()).into_response(),
    }
}

fn validator(state: &AppState) -> ManualApplyDryRunValidator {
    ManualApplyDryRunValidator::new(
        state.config().replay_report_dir.clone(),
        Some(state.config().clone()),
    )
}

fn json_reason(status: StatusCode, reason: &str) -> axum::response::Response {
    (
        status,
        Json(serde_json::json!({
            "reason": reason,
            "applyMode": "dry_run_only",
            "runtimeModified": false,
            "canApplyManually": false
        })),
    )
        .into_response()
}

fn json_error(status: StatusCode, error: String) -> axum::response::Response {
    (
        status,
        Json(serde_json::json!({
            "error": error,
            "applyMode": "dry_run_only",
            "runtimeModified": false,
            "canApplyManually": false
        })),
    )
        .into_response()
}
