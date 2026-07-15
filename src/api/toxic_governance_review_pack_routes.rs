use axum::{
    extract::{Path, Query, State},
    Json,
};
use serde::Deserialize;

use crate::{
    api::toxic_quality_scorecard_routes::build_fusion_recent,
    app::AppState,
    toxicity::toxic_governance_review_pack_service::{
        toxic_governance_review_pack_export, toxic_governance_review_pack_status,
        toxic_governance_review_pack_summary,
    },
};

#[derive(Debug, Deserialize)]
pub struct ToxicGovernanceReviewPackQuery {
    symbol: Option<String>,
}

pub async fn toxic_governance_review_pack_status_route(
    State(state): State<AppState>,
    Query(query): Query<ToxicGovernanceReviewPackQuery>,
) -> Json<serde_json::Value> {
    let requested_symbol = query
        .symbol
        .unwrap_or_else(|| state.config().symbol.clone());
    let fusion_recent = build_fusion_recent(&state, &requested_symbol);
    Json(serde_json::json!(toxic_governance_review_pack_status(
        &requested_symbol,
        &fusion_recent,
        |ts| state.price_snapshot_at_or_before_for_symbol(ts, &requested_symbol),
        |ts| state.price_snapshots_since_for_symbol(ts, &requested_symbol),
    )))
}

pub async fn toxic_governance_review_pack_summary_route(
    State(state): State<AppState>,
    Query(query): Query<ToxicGovernanceReviewPackQuery>,
) -> Json<serde_json::Value> {
    let requested_symbol = query
        .symbol
        .unwrap_or_else(|| state.config().symbol.clone());
    Json(serde_json::json!(build_summary(&state, &requested_symbol)))
}

pub async fn toxic_governance_review_pack_for_symbol(
    State(state): State<AppState>,
    Path(symbol): Path<String>,
) -> Json<serde_json::Value> {
    Json(serde_json::json!(build_summary(&state, &symbol)))
}

pub async fn toxic_governance_review_pack_export_route(
    State(state): State<AppState>,
    Query(query): Query<ToxicGovernanceReviewPackQuery>,
) -> Json<serde_json::Value> {
    let requested_symbol = query
        .symbol
        .unwrap_or_else(|| state.config().symbol.clone());
    let fusion_recent = build_fusion_recent(&state, &requested_symbol);
    Json(serde_json::json!(toxic_governance_review_pack_export(
        &requested_symbol,
        &fusion_recent,
        |ts| state.price_snapshot_at_or_before_for_symbol(ts, &requested_symbol),
        |ts| state.price_snapshots_since_for_symbol(ts, &requested_symbol),
    )))
}

fn build_summary(
    state: &AppState,
    requested_symbol: &str,
) -> crate::types::toxic_governance_review_pack::ToxicGovernanceReviewPackSummaryResponse {
    let fusion_recent = build_fusion_recent(state, requested_symbol);
    toxic_governance_review_pack_summary(
        requested_symbol,
        &fusion_recent,
        |ts| state.price_snapshot_at_or_before_for_symbol(ts, requested_symbol),
        |ts| state.price_snapshots_since_for_symbol(ts, requested_symbol),
    )
}
