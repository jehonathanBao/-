use axum::{
    extract::{Path, State},
    http::StatusCode,
    response::IntoResponse,
    Json,
};

use crate::{app::AppState, calibration::calibration_report_store::CalibrationReportStore};

pub async fn calibration_reports(State(state): State<AppState>) -> impl IntoResponse {
    let store = CalibrationReportStore::new(state.config().replay_report_dir.clone());
    match store.list_reports() {
        Ok(reports) => Json(serde_json::json!({ "reports": reports })).into_response(),
        Err(err) => (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(serde_json::json!({ "error": err.to_string() })),
        )
            .into_response(),
    }
}

pub async fn latest_calibration_report(State(state): State<AppState>) -> impl IntoResponse {
    let store = CalibrationReportStore::new(state.config().replay_report_dir.clone());
    match store.latest_report() {
        Ok(Some(report)) => Json(serde_json::json!({ "report": report })).into_response(),
        Ok(None) => Json(serde_json::json!({ "report": null })).into_response(),
        Err(err) => (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(serde_json::json!({ "error": err.to_string() })),
        )
            .into_response(),
    }
}

pub async fn calibration_report_by_id(
    State(state): State<AppState>,
    Path(report_id): Path<String>,
) -> impl IntoResponse {
    if !valid_report_id(&report_id) {
        return (
            StatusCode::BAD_REQUEST,
            Json(serde_json::json!({ "error": "invalid calibration report id" })),
        )
            .into_response();
    }

    let store = CalibrationReportStore::new(state.config().replay_report_dir.clone());
    match store.get_report(&report_id) {
        Ok(Some(report)) => Json(serde_json::json!({ "report": report })).into_response(),
        Ok(None) => (
            StatusCode::NOT_FOUND,
            Json(serde_json::json!({ "error": "calibration report not found" })),
        )
            .into_response(),
        Err(err) => (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(serde_json::json!({ "error": err.to_string() })),
        )
            .into_response(),
    }
}

fn valid_report_id(report_id: &str) -> bool {
    report_id.starts_with("calibration-")
        && !report_id.contains("..")
        && !report_id.contains('\\')
        && !report_id.contains('/')
}
