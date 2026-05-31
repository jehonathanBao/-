use axum::{extract::State, Json};

use crate::{
    app::AppState,
    types::status::{RuntimeStartResult, RuntimeStopResult},
};

pub async fn ensure_monitoring_started(State(state): State<AppState>) -> Json<serde_json::Value> {
    let outcome = state.ensure_monitoring_started().await;
    let runtime_control = state.runtime_control_summary();

    Json(serde_json::json!({
        "ok": outcome.result != RuntimeStartResult::Failed,
        "readOnly": false,
        "runtimeModified": outcome.runtime_modified,
        "analysisOnly": true,
        "monitoringStarted": runtime_control.monitoring_started,
        "action": "start_monitoring",
        "result": outcome.result,
        "startState": outcome.start_state,
        "message": match outcome.result {
            RuntimeStartResult::Started => "Monitoring services started.",
            RuntimeStartResult::AlreadyStarted => "Monitoring services are already started.",
            RuntimeStartResult::Failed => "Monitoring start failed.",
            RuntimeStartResult::None => "No monitoring start action was performed.",
        },
        "error": runtime_control.last_start_error,
        "safetyBoundary": [
            "Monitoring only",
            "No order placement",
            "No cancel/amend",
            "No wallet/signing",
            "No live trading",
            "No config mutation",
            "No apply/reload"
        ]
    }))
}

pub async fn ensure_monitoring_stopped(State(state): State<AppState>) -> Json<serde_json::Value> {
    let outcome = state.ensure_monitoring_stopped().await;
    let runtime_control = state.runtime_control_summary();

    Json(serde_json::json!({
        "ok": outcome.result != RuntimeStopResult::Failed,
        "readOnly": false,
        "runtimeModified": outcome.runtime_modified,
        "analysisOnly": true,
        "monitoringStarted": runtime_control.monitoring_started,
        "action": "stop_monitoring",
        "result": outcome.result,
        "stopState": outcome.stop_state,
        "message": match outcome.result {
            RuntimeStopResult::Stopped => "Monitoring services stopped.",
            RuntimeStopResult::AlreadyStopped => "Monitoring services are already stopped.",
            RuntimeStopResult::Failed => "Monitoring stop failed.",
            RuntimeStopResult::None => "No monitoring stop action was performed.",
        },
        "error": runtime_control.last_stop_error,
        "safetyBoundary": [
            "Monitoring only",
            "No order placement",
            "No cancel/amend",
            "No wallet/signing",
            "No live trading",
            "No config mutation",
            "No apply/reload"
        ]
    }))
}
