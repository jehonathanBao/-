use axum::{
    extract::{Path, Query, State},
    Json,
};
use serde::Deserialize;

use crate::{
    api::toxic_quality_scorecard_routes::build_fusion_recent,
    app::AppState,
    runtime::tof_metrics::{enhance_signal_summary, TofSummaryInput},
    toxicity::{
        toxic_governance_ledger_service::toxic_governance_ledger_summary,
        toxic_markout_service::toxic_markout_recent,
        toxic_quality_scorecard_service::toxic_quality_scorecard_summary,
        toxic_replay_service::replay_recent,
        toxic_signal_inbox_service::{
            toxic_signal_inbox_by_signal_id, toxic_signal_inbox_recent, toxic_signal_inbox_status,
        },
        toxic_weight_recommendation_service::toxic_weight_recommendation_summary,
    },
    types::toxic_signal_inbox::ToxicSignalInboxRecentResponse,
};

#[derive(Debug, Deserialize)]
pub struct ToxicSignalInboxQuery {
    symbol: Option<String>,
}

pub async fn toxic_signal_inbox_status_route(
    State(state): State<AppState>,
    Query(query): Query<ToxicSignalInboxQuery>,
) -> Json<serde_json::Value> {
    let requested_symbol = normalize_symbol_query(query.symbol, &state.config().symbol);
    let recent = build_recent(&state, &requested_symbol);
    Json(with_filter_contract(
        serde_json::json!(toxic_signal_inbox_status(&recent)),
        &requested_symbol,
    ))
}

pub async fn toxic_signal_inbox_recent_route(
    State(state): State<AppState>,
    Query(query): Query<ToxicSignalInboxQuery>,
) -> Json<serde_json::Value> {
    let requested_symbol = normalize_symbol_query(query.symbol, &state.config().symbol);
    Json(with_filter_contract(
        with_tof_metrics_contract(serde_json::json!(build_recent(&state, &requested_symbol))),
        &requested_symbol,
    ))
}

pub async fn toxic_signal_inbox_for_symbol(
    State(state): State<AppState>,
    Path(symbol): Path<String>,
) -> Json<serde_json::Value> {
    let requested_symbol = normalize_symbol_text(&symbol, &state.config().symbol);
    Json(with_filter_contract(
        with_tof_metrics_contract(serde_json::json!(build_recent(&state, &requested_symbol))),
        &requested_symbol,
    ))
}

pub async fn toxic_signal_inbox_for_signal(
    State(state): State<AppState>,
    Path(signal_id): Path<String>,
    Query(query): Query<ToxicSignalInboxQuery>,
) -> Json<serde_json::Value> {
    let requested_symbol = normalize_symbol_query(query.symbol, &state.config().symbol);
    let recent = build_recent(&state, &requested_symbol);
    Json(with_tof_metrics_contract(serde_json::json!(
        toxic_signal_inbox_by_signal_id(&requested_symbol, &signal_id, &recent,)
    )))
}

pub(crate) fn build_recent(
    state: &AppState,
    requested_symbol: &str,
) -> ToxicSignalInboxRecentResponse {
    let fusion_recent = build_fusion_recent(state, requested_symbol);
    let replay_recent = replay_recent(requested_symbol, &fusion_recent);
    let markout_recent = toxic_markout_recent(
        requested_symbol,
        &fusion_recent,
        |ts| state.price_snapshot_at_or_before(ts),
        |ts| state.price_snapshots_since(ts),
    );
    let quality_summary = toxic_quality_scorecard_summary(
        requested_symbol,
        &fusion_recent,
        |ts| state.price_snapshot_at_or_before(ts),
        |ts| state.price_snapshots_since(ts),
    );
    let recommendation_summary = toxic_weight_recommendation_summary(
        requested_symbol,
        &fusion_recent,
        |ts| state.price_snapshot_at_or_before(ts),
        |ts| state.price_snapshots_since(ts),
    );
    let governance_summary = toxic_governance_ledger_summary(Some(requested_symbol));

    toxic_signal_inbox_recent(
        requested_symbol,
        &fusion_recent,
        &replay_recent,
        &markout_recent,
        &quality_summary,
        &recommendation_summary,
        &governance_summary,
    )
}

pub(crate) fn normalize_symbol_query(symbol: Option<String>, default_symbol: &str) -> String {
    match symbol {
        Some(symbol) => normalize_symbol_text(&symbol, default_symbol),
        None => normalize_symbol_text(default_symbol, default_symbol),
    }
}

pub(crate) fn normalize_symbol_text(symbol: &str, default_symbol: &str) -> String {
    let trimmed = symbol.trim();
    if trimmed.is_empty() {
        default_symbol.trim().to_ascii_uppercase()
    } else {
        trimmed.to_ascii_uppercase()
    }
}

pub(crate) fn with_filter_contract(
    mut payload: serde_json::Value,
    requested_symbol: &str,
) -> serde_json::Value {
    if let Some(object) = payload.as_object_mut() {
        object.insert(
            "filter".to_string(),
            serde_json::json!({
                "symbol": requested_symbol,
                "viewOnly": true,
                "persistentWatchlistEnabled": false,
                "runtimeMonitorModified": false,
            }),
        );
    }
    payload
}

pub(crate) fn with_tof_metrics_contract(mut payload: serde_json::Value) -> serde_json::Value {
    if let Some(items) = payload
        .get_mut("items")
        .and_then(|value| value.as_array_mut())
    {
        for item in items {
            decorate_item_with_tof(item);
        }
    }
    if let Some(item) = payload.get_mut("item") {
        decorate_item_with_tof(item);
    }
    payload
}

fn decorate_item_with_tof(item: &mut serde_json::Value) {
    let Some(object) = item.as_object_mut() else {
        return;
    };
    let signal_kind = object
        .get("signalKind")
        .and_then(|value| value.as_str())
        .unwrap_or("unknown");
    let direction_bias = object
        .get("directionBias")
        .and_then(|value| value.as_str())
        .unwrap_or("neutral");
    let severity = object
        .get("severity")
        .and_then(|value| value.as_str())
        .unwrap_or("low");
    let confidence = object
        .get("confidence")
        .and_then(|value| value.as_f64())
        .unwrap_or(0.35);
    let quality_bucket = object
        .get("quality")
        .and_then(|value| value.get("qualityBucket"))
        .and_then(|value| value.as_str())
        .unwrap_or("not_enough_data");
    let summary = object
        .get("fusion")
        .and_then(|value| value.get("summary"))
        .and_then(|value| value.as_str())
        .unwrap_or("candidate signal");
    let existing_risk_score = risk_score_for_severity(severity);
    let existing_data_quality = data_quality_for_bucket(quality_bucket);
    let enhancement = enhance_signal_summary(&TofSummaryInput {
        signal_kind,
        direction_bias,
        severity,
        confidence,
        quality_bucket,
        summary,
        existing_risk_score,
        existing_data_quality,
    });
    object.insert(
        "tofMetrics".to_string(),
        serde_json::to_value(&enhancement.tof_metrics).unwrap_or(serde_json::Value::Null),
    );
    object.insert(
        "candidateType".to_string(),
        serde_json::json!(enhancement.candidate_type),
    );
    object.insert(
        "explainTags".to_string(),
        serde_json::json!(enhancement.explain_tags),
    );
    object.insert(
        "direction".to_string(),
        serde_json::json!(enhancement.direction),
    );
    object.insert(
        "directionLabel".to_string(),
        serde_json::json!(enhancement.direction_label),
    );
    object.insert(
        "directionConfidence".to_string(),
        serde_json::json!(enhancement.direction_confidence),
    );
    object.insert(
        "directionSource".to_string(),
        serde_json::json!(enhancement.direction_source),
    );
    object.insert(
        "tofScore".to_string(),
        serde_json::json!(enhancement.tof_score),
    );
    object.insert(
        "finalRiskScore".to_string(),
        serde_json::json!(enhancement.final_risk_score),
    );
    object.insert(
        "riskScore".to_string(),
        serde_json::json!(enhancement.final_risk_score),
    );
    object.insert(
        "dataQuality".to_string(),
        serde_json::json!(existing_data_quality),
    );
}

fn risk_score_for_severity(severity: &str) -> u8 {
    match severity.to_ascii_lowercase().as_str() {
        "critical" => 92,
        "high" => 85,
        "medium" => 72,
        _ => 45,
    }
}

fn data_quality_for_bucket(bucket: &str) -> f64 {
    match bucket.to_ascii_lowercase().as_str() {
        "excellent" => 92.0,
        "good" => 82.0,
        "mixed" => 74.0,
        "weak" => 62.0,
        "bad" => 45.0,
        _ => 70.0,
    }
}
