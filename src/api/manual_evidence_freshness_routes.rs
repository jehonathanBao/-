use axum::{extract::State, http::StatusCode, response::IntoResponse, Json};

use crate::{app::AppState, calibration::manual_evidence_freshness::ManualEvidenceFreshnessStore};

pub async fn manual_evidence_freshness(State(state): State<AppState>) -> impl IntoResponse {
    let store = ManualEvidenceFreshnessStore::new(
        state.config().replay_report_dir.clone(),
        state.config().clone(),
    );
    match store.freshness() {
        Ok(response) => Json(serde_json::json!(response)).into_response(),
        Err(err) => (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(serde_json::json!({
                "readOnly": true,
                "status": "MISSING_EVIDENCE",
                "error": err.to_string(),
            })),
        )
            .into_response(),
    }
}
