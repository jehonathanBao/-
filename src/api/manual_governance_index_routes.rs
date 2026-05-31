use axum::{extract::State, http::StatusCode, response::IntoResponse, Json};

use crate::{app::AppState, calibration::manual_governance_index::ManualApplyGovernanceIndexStore};

pub async fn manual_governance_index(State(state): State<AppState>) -> impl IntoResponse {
    let store = governance_store(&state);
    match store.build_index() {
        Ok(index) => Json(serde_json::json!(index)).into_response(),
        Err(err) => json_error(StatusCode::INTERNAL_SERVER_ERROR, err.to_string()),
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
