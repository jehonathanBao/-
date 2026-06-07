use axum::{
    extract::{Path, Query, State},
    Json,
};
use serde::Deserialize;

use crate::{
    api::{
        discord_notification_routes::{
            discord_alert_status_for_key, evaluate_discord_alert_gate, DiscordAlertMode,
            DiscordNotificationRequest,
        },
        toxic_quality_scorecard_routes::build_fusion_recent,
    },
    app::AppState,
    runtime::{
        advanced_tof_metrics::{build_advanced_tof_metrics, AdvancedTofInput},
        perp_tof_metrics::{build_perp_tof_metrics, PerpTofInput},
        tof_metrics::{enhance_signal_summary, TofSummaryInput},
    },
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
        .unwrap_or("unknown")
        .to_string();
    let direction_bias = object
        .get("directionBias")
        .and_then(|value| value.as_str())
        .unwrap_or("neutral")
        .to_string();
    let severity = object
        .get("severity")
        .and_then(|value| value.as_str())
        .unwrap_or("low")
        .to_string();
    let confidence = object
        .get("confidence")
        .and_then(|value| value.as_f64())
        .unwrap_or(0.35);
    let quality_bucket = object
        .get("quality")
        .and_then(|value| value.get("qualityBucket"))
        .and_then(|value| value.as_str())
        .unwrap_or("not_enough_data")
        .to_string();
    let summary = object
        .get("fusion")
        .and_then(|value| value.get("summary"))
        .and_then(|value| value.as_str())
        .unwrap_or("candidate signal")
        .to_string();
    let symbol = object
        .get("symbol")
        .and_then(|value| value.as_str())
        .unwrap_or("UNKNOWN")
        .to_string();
    let existing_risk_score = risk_score_for_severity(&severity);
    let existing_data_quality = data_quality_for_bucket(&quality_bucket);
    let enhancement = enhance_signal_summary(&TofSummaryInput {
        signal_kind: &signal_kind,
        direction_bias: &direction_bias,
        severity: &severity,
        confidence,
        quality_bucket: &quality_bucket,
        summary: &summary,
        existing_risk_score,
        existing_data_quality,
    });
    let candidate_type = enhancement.candidate_type.clone();
    let explain_tags = enhancement.explain_tags.clone();
    let direction_label = enhancement.direction_label.clone();
    let direction_source = enhancement.direction_source.clone();
    let perp_metrics = build_perp_tof_metrics(&PerpTofInput {
        symbol: &symbol,
        spot_candidate_type: &candidate_type,
        spot_direction: enhancement.direction,
        spot_risk_score: enhancement.final_risk_score,
        spot_data_quality: existing_data_quality,
        spot_confidence: confidence,
        summary: &summary,
    });
    let advanced_metrics = build_advanced_tof_metrics(&AdvancedTofInput {
        symbol: &symbol,
        spot_candidate_type: &candidate_type,
        spot_direction: enhancement.direction,
        spot_risk_score: existing_risk_score,
        spot_data_quality: existing_data_quality,
        spot_confidence: confidence,
        tof_metrics: &enhancement.tof_metrics,
        spot_tags: &explain_tags,
        perp_metrics: &perp_metrics,
        summary: &summary,
    });
    let merged_tags = advanced_metrics.explain_tags.clone();
    let advanced_score = advanced_metrics.final_risk_score;
    let advanced_data_quality = advanced_metrics.data_quality;
    let advanced_candidate_type = advanced_metrics.candidate_type.clone();
    let perp_score = perp_metrics.risk_score;
    let perp_candidate_type = perp_metrics.candidate_type.clone();
    let final_candidate_type = advanced_metrics.final_candidate_type.clone();
    let metrics_direction = serde_json::to_value(advanced_metrics.metrics_direction)
        .ok()
        .and_then(|value| value.as_str().map(str::to_string));
    let final_risk_score = advanced_score;
    object.insert(
        "tofMetrics".to_string(),
        serde_json::to_value(&enhancement.tof_metrics).unwrap_or(serde_json::Value::Null),
    );
    object.insert(
        "candidateType".to_string(),
        serde_json::json!(advanced_candidate_type.clone()),
    );
    object.insert(
        "explainTags".to_string(),
        serde_json::json!(merged_tags.clone()),
    );
    object.insert(
        "direction".to_string(),
        serde_json::json!(enhancement.direction),
    );
    object.insert(
        "directionLabel".to_string(),
        serde_json::json!(direction_label),
    );
    object.insert(
        "directionConfidence".to_string(),
        serde_json::json!(enhancement.direction_confidence),
    );
    object.insert(
        "directionSource".to_string(),
        serde_json::json!(direction_source),
    );
    object.insert(
        "tofScore".to_string(),
        serde_json::json!(enhancement.tof_score),
    );
    object.insert(
        "perpTofMetrics".to_string(),
        serde_json::to_value(&perp_metrics).unwrap_or(serde_json::Value::Null),
    );
    object.insert("perpScore".to_string(), serde_json::json!(perp_score));
    object.insert(
        "perpCandidateType".to_string(),
        serde_json::json!(perp_candidate_type.clone()),
    );
    object.insert(
        "finalCandidateType".to_string(),
        serde_json::json!(final_candidate_type.clone()),
    );
    object.insert(
        "metricsDirection".to_string(),
        serde_json::json!(advanced_metrics.metrics_direction),
    );
    object.insert(
        "mergedConfidence".to_string(),
        serde_json::json!(advanced_metrics.confidence),
    );
    object.insert(
        "advancedTofMetrics".to_string(),
        serde_json::to_value(&advanced_metrics).unwrap_or(serde_json::Value::Null),
    );
    object.insert(
        "advancedScore".to_string(),
        serde_json::json!(advanced_score),
    );
    object.insert(
        "advancedCandidateType".to_string(),
        serde_json::json!(advanced_candidate_type.clone()),
    );
    object.insert(
        "finalRiskScore".to_string(),
        serde_json::json!(final_risk_score),
    );
    object.insert("riskScore".to_string(), serde_json::json!(final_risk_score));
    object.insert(
        "dataQuality".to_string(),
        serde_json::json!(advanced_data_quality),
    );
    let signal_id = object
        .get("signalId")
        .and_then(|value| value.as_str())
        .map(str::to_string);
    let alert_request = DiscordNotificationRequest {
        signal_id: signal_id.clone(),
        id: signal_id.clone(),
        dedupe_key: signal_id.clone(),
        exchange: Some("Runtime".to_string()),
        symbol: Some(symbol),
        signal_type: Some(signal_kind),
        level: Some(severity),
        side: Some(enhancement.direction_label.clone()),
        score: Some(final_risk_score),
        data_quality: Some(advanced_data_quality),
        reason: Some(summary),
        impact: None,
        time: None,
        price_range: None,
        add_qty: None,
        cancel_qty: None,
        fill_qty: None,
        cancel_to_trade_ratio: None,
        depth_before: None,
        depth_after: None,
        depth_impact: None,
        price_impact_bps: None,
        markout_1s_bps: None,
        markout_5s_bps: None,
        markout_30s_bps: None,
        tof_metrics: Some(enhancement.tof_metrics),
        tof_score: Some(enhancement.tof_score),
        candidate_type: Some(advanced_candidate_type.clone()),
        explain_tags: Some(merged_tags),
        direction_confidence: Some(enhancement.direction_confidence),
        perp_tof_metrics: Some(perp_metrics),
        perp_score: Some(perp_score),
        perp_candidate_type: Some(perp_candidate_type),
        final_candidate_type: Some(final_candidate_type),
        metrics_direction,
        advanced_tof_metrics: Some(advanced_metrics),
        advanced_score: Some(advanced_score),
        advanced_candidate_type: Some(advanced_candidate_type),
        test: None,
    };
    let alert_decision = evaluate_discord_alert_gate(&alert_request, DiscordAlertMode::Auto);
    let stored_alert = signal_id.as_deref().and_then(discord_alert_status_for_key);
    let alert_status = stored_alert
        .as_ref()
        .map(|status| status.last_decision.clone())
        .unwrap_or_else(|| {
            alert_status_from_reason(alert_decision.allowed, alert_decision.reason).to_string()
        });
    let alert_reason = stored_alert
        .as_ref()
        .map(|status| status.reason.clone())
        .unwrap_or_else(|| alert_decision.reason.to_string());
    object.insert(
        "alertStatus".to_string(),
        serde_json::json!(alert_status.clone()),
    );
    object.insert(
        "alertReason".to_string(),
        serde_json::json!(alert_reason.clone()),
    );
    let discord_alert = stored_alert
        .map(serde_json::to_value)
        .and_then(Result::ok)
        .unwrap_or_else(|| {
            serde_json::json!({
            "autoEligible": alert_decision.allowed,
            "autoSent": false,
            "lastDecision": alert_status,
            "reason": alert_reason,
            "sentAt": null,
            "manualSentAt": null,
            })
        });
    object.insert("discordAlert".to_string(), discord_alert);
}

fn alert_status_from_reason(allowed: bool, reason: &str) -> &'static str {
    if allowed {
        "eligible"
    } else if matches!(
        reason,
        "score_below_threshold" | "data_quality_below_threshold" | "non_high_risk"
    ) {
        "rejected"
    } else {
        "skipped"
    }
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
