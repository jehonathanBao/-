use axum::{extract::State, http::StatusCode, response::IntoResponse, Json};

use crate::{app::AppState, calibration::manual_startup_check::ManualStartupCheckStore};

pub async fn manual_startup_check(State(state): State<AppState>) -> impl IntoResponse {
    let store = ManualStartupCheckStore::new(
        state.config().replay_report_dir.clone(),
        state.config().clone(),
    );
    match store.run_check() {
        Ok(report) => Json(serde_json::json!(report)).into_response(),
        Err(err) => (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(serde_json::json!({
                "ok": false,
                "status": "BLOCKED",
                "readOnly": true,
                "error": err.to_string(),
                "nextAction": "Inspect startup check internals before manual apply."
            })),
        )
            .into_response(),
    }
}
