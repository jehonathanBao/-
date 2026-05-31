use axum::{
    extract::State,
    http::{header, HeaderValue, StatusCode},
    response::IntoResponse,
    Json,
};

use crate::{
    app::AppState, calibration::manual_apply_governance_index::ManualApplyGovernanceIndexStore,
};

pub async fn manual_apply_governance(State(state): State<AppState>) -> impl IntoResponse {
    let store = governance_store(&state);
    match store.build_index() {
        Ok(index) => Json(serde_json::json!(index)).into_response(),
        Err(err) => json_error(StatusCode::INTERNAL_SERVER_ERROR, err.to_string()),
    }
}

pub async fn latest_manual_apply_governance(State(state): State<AppState>) -> impl IntoResponse {
    manual_apply_governance(State(state)).await
}

pub async fn manual_apply_governance_markdown(State(state): State<AppState>) -> impl IntoResponse {
    let store = governance_store(&state);
    match store.markdown() {
        Ok(markdown) => (
            [(
                header::CONTENT_TYPE,
                HeaderValue::from_static("text/markdown; charset=utf-8"),
            )],
            markdown,
        )
            .into_response(),
        Err(err) => (StatusCode::INTERNAL_SERVER_ERROR, err.to_string()).into_response(),
    }
}

fn governance_store(state: &AppState) -> ManualApplyGovernanceIndexStore {
    ManualApplyGovernanceIndexStore::new(
        state.config().replay_report_dir.clone(),
        state.config().clone(),
    )
}

fn json_error(status: StatusCode, error: String) -> axum::response::Response {
    (
        status,
        Json(serde_json::json!({
            "readOnly": true,
            "applyMode": "governance_index_only",
            "runtimeModified": false,
            "error": error,
        })),
    )
        .into_response()
}
