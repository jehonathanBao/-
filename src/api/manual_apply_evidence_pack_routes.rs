use axum::{
    extract::{Path, State},
    http::{header, HeaderValue, StatusCode},
    response::IntoResponse,
    Json,
};

use crate::{app::AppState, calibration::manual_apply_evidence_pack::ManualApplyEvidencePackStore};

pub async fn latest_manual_apply_evidence_pack(State(state): State<AppState>) -> impl IntoResponse {
    let store = evidence_pack_store(&state);
    match store.latest_pack() {
        Ok(Some(pack)) => Json(serde_json::json!(pack)).into_response(),
        Ok(None) => json_reason(StatusCode::NOT_FOUND, "no_exports_found"),
        Err(err) if err.to_string() == "current_config_unavailable" => json_reason(
            StatusCode::SERVICE_UNAVAILABLE,
            "current_config_unavailable",
        ),
        Err(err) => json_error(StatusCode::INTERNAL_SERVER_ERROR, err.to_string()),
    }
}

pub async fn manual_apply_evidence_pack_by_id(
    State(state): State<AppState>,
    Path(export_id): Path<String>,
) -> impl IntoResponse {
    let store = evidence_pack_store(&state);
    match store.pack_by_id(&export_id) {
        Ok(Some(pack)) => Json(serde_json::json!(pack)).into_response(),
        Ok(None) => json_reason(StatusCode::NOT_FOUND, "export_not_found"),
        Err(err) if err.to_string() == "current_config_unavailable" => json_reason(
            StatusCode::SERVICE_UNAVAILABLE,
            "current_config_unavailable",
        ),
        Err(err) => json_error(StatusCode::INTERNAL_SERVER_ERROR, err.to_string()),
    }
}

pub async fn latest_manual_apply_evidence_pack_markdown(
    State(state): State<AppState>,
) -> impl IntoResponse {
    let store = evidence_pack_store(&state);
    match store.latest_markdown() {
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

pub async fn manual_apply_evidence_pack_markdown_by_id(
    State(state): State<AppState>,
    Path(export_id): Path<String>,
) -> impl IntoResponse {
    let store = evidence_pack_store(&state);
    match store.markdown_by_id(&export_id) {
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

fn evidence_pack_store(state: &AppState) -> ManualApplyEvidencePackStore {
    ManualApplyEvidencePackStore::new(
        state.config().replay_report_dir.clone(),
        Some(state.config().clone()),
    )
}

fn json_reason(status: StatusCode, reason: &str) -> axum::response::Response {
    (
        status,
        Json(serde_json::json!({
            "reason": reason,
            "applyMode": "manual_signoff_only",
            "runtimeModified": false,
            "signoffAllowed": false
        })),
    )
        .into_response()
}

fn json_error(status: StatusCode, error: String) -> axum::response::Response {
    (
        status,
        Json(serde_json::json!({
            "error": error,
            "applyMode": "manual_signoff_only",
            "runtimeModified": false,
            "signoffAllowed": false
        })),
    )
        .into_response()
}
