use axum::{
    extract::State,
    http::{header, HeaderValue, StatusCode},
    response::IntoResponse,
    Json,
};

use crate::{app::AppState, calibration::manual_audit_story::ManualAuditStoryStore};

pub async fn manual_audit_story(State(state): State<AppState>) -> impl IntoResponse {
    let store = audit_story_store(&state);
    match store.build_story() {
        Ok(story) => Json(serde_json::json!(story)).into_response(),
        Err(err) => json_error(StatusCode::INTERNAL_SERVER_ERROR, err.to_string()),
    }
}

pub async fn manual_audit_story_markdown(State(state): State<AppState>) -> impl IntoResponse {
    let store = audit_story_store(&state);
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

fn audit_story_store(state: &AppState) -> ManualAuditStoryStore {
    ManualAuditStoryStore::new(
        state.config().replay_report_dir.clone(),
        state.config().clone(),
    )
}

fn json_error(status: StatusCode, error: String) -> axum::response::Response {
    (
        status,
        Json(serde_json::json!({
            "readOnly": true,
            "applyMode": "read_only_audit_story",
            "runtimeModified": false,
            "error": error,
        })),
    )
        .into_response()
}
