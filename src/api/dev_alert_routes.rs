use axum::{extract::State, http::StatusCode, response::IntoResponse, Json};
use serde::Deserialize;

use crate::{
    app::AppState,
    types::{market::Venue, toxic::ToxicSeverity},
};

#[derive(Debug, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum DevTestAlertSeverity {
    Info,
    Warning,
    Critical,
}

#[derive(Debug, Deserialize)]
pub struct DevTestSidecarAlertRequest {
    pub severity: DevTestAlertSeverity,
    pub venue: Venue,
    pub symbol: String,
    pub dedupe_suffix: String,
}

pub async fn test_sidecar_alert(
    State(state): State<AppState>,
    Json(body): Json<DevTestSidecarAlertRequest>,
) -> impl IntoResponse {
    if !dev_test_alerts_enabled() {
        return json_reason(StatusCode::NOT_FOUND, "dev_test_alerts_disabled");
    }

    if body.symbol.trim().is_empty() || body.dedupe_suffix.trim().is_empty() {
        return json_reason(
            StatusCode::BAD_REQUEST,
            "symbol_and_dedupe_suffix_are_required",
        );
    }

    match state.emit_runtime_acceptance_test_sidecar_alert(
        map_dev_test_severity(body.severity),
        body.venue,
        body.symbol.trim().to_string(),
        body.dedupe_suffix.trim().to_string(),
    ) {
        Ok(result) => Json(serde_json::json!({
            "ok": true,
            "readOnly": false,
            "analysisOnly": true,
            "action": "emit_runtime_acceptance_test_sidecar_alert",
            "kind": "runtime_acceptance_test",
            "title": "Runtime acceptance test alert",
            "message": "This is a monitor-generated sidecar test alert.",
            "sidecarWritten": result.sidecar_written,
            "deduped": result.deduped,
            "dedupeKey": result.dedupe_key,
            "telegramTriggered": false,
            "safetyBoundary": [
                "Monitoring only",
                "No order placement",
                "No cancel/amend",
                "No wallet/signing",
                "No live trading",
                "No config mutation",
                "No Discord credential access"
            ]
        }))
        .into_response(),
        Err(err) => json_error(StatusCode::CONFLICT, err.to_string()),
    }
}

fn map_dev_test_severity(severity: DevTestAlertSeverity) -> ToxicSeverity {
    match severity {
        DevTestAlertSeverity::Info => ToxicSeverity::Watch,
        DevTestAlertSeverity::Warning => ToxicSeverity::Warning,
        DevTestAlertSeverity::Critical => ToxicSeverity::Alert,
    }
}

fn dev_test_alerts_enabled() -> bool {
    std::env::var("ENABLE_DEV_TEST_ALERTS")
        .ok()
        .is_some_and(|value| value.eq_ignore_ascii_case("true"))
}

fn json_reason(status: StatusCode, reason: &str) -> axum::response::Response {
    (
        status,
        Json(serde_json::json!({
            "ok": false,
            "readOnly": false,
            "analysisOnly": true,
            "reason": reason,
        })),
    )
        .into_response()
}

fn json_error(status: StatusCode, error: String) -> axum::response::Response {
    (
        status,
        Json(serde_json::json!({
            "ok": false,
            "readOnly": false,
            "analysisOnly": true,
            "error": error,
        })),
    )
        .into_response()
}
