use axum::{extract::State, http::StatusCode, response::IntoResponse, Json};

use crate::{
    app::AppState,
    calibration::manual_signoff_store::{ManualSignoffInput, ManualSignoffStore},
};

pub async fn manual_signoff_status(State(state): State<AppState>) -> impl IntoResponse {
    let store = signoff_store(&state);
    match store.status() {
        Ok(status) => Json(serde_json::json!(status)).into_response(),
        Err(err) => json_error(StatusCode::INTERNAL_SERVER_ERROR, err.to_string()),
    }
}

pub async fn create_manual_signoff(
    State(state): State<AppState>,
    Json(input): Json<ManualSignoffInput>,
) -> impl IntoResponse {
    let store = signoff_store(&state);
    match store.create_signoff(input) {
        Ok(response) => Json(serde_json::json!(response)).into_response(),
        Err(err) if err.to_string() == "readiness_not_ready" => (
            StatusCode::CONFLICT,
            Json(serde_json::json!({
                "ok": false,
                "readOnly": true,
                "reason": "readiness_not_ready",
                "applyMode": "manual_signoff_only",
                "runtimeModified": false,
            })),
        )
            .into_response(),
        Err(err) => json_error(StatusCode::INTERNAL_SERVER_ERROR, err.to_string()),
    }
}

pub async fn manual_signoff_history(State(state): State<AppState>) -> impl IntoResponse {
    let store = signoff_store(&state);
    match store.history() {
        Ok(records) => Json(serde_json::json!({
            "readOnly": true,
            "records": records,
        }))
        .into_response(),
        Err(err) => json_error(StatusCode::INTERNAL_SERVER_ERROR, err.to_string()),
    }
}

fn signoff_store(state: &AppState) -> ManualSignoffStore {
    ManualSignoffStore::new(
        state.config().replay_report_dir.clone(),
        state.config().clone(),
    )
}

fn json_error(status: StatusCode, error: String) -> axum::response::Response {
    (
        status,
        Json(serde_json::json!({
            "error": error,
            "readOnly": true,
            "applyMode": "manual_signoff_only",
            "runtimeModified": false,
        })),
    )
        .into_response()
}
