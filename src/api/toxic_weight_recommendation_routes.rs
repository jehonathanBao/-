use axum::{
    extract::{Path, Query, State},
    Json,
};
use serde::Deserialize;

use crate::{
    api::toxic_quality_scorecard_routes::build_fusion_recent,
    app::AppState,
    toxicity::toxic_weight_recommendation_service::{
        toxic_weight_recommendation_status, toxic_weight_recommendation_summary,
    },
};

#[derive(Debug, Deserialize)]
pub struct ToxicWeightRecommendationQuery {
    symbol: Option<String>,
}

pub async fn toxic_weight_recommendation_status_route(
    State(state): State<AppState>,
    Query(query): Query<ToxicWeightRecommendationQuery>,
) -> Json<serde_json::Value> {
    let requested_symbol = query
        .symbol
        .unwrap_or_else(|| state.config().symbol.clone());
    let fusion_recent = build_fusion_recent(&state, &requested_symbol);
    Json(serde_json::json!(toxic_weight_recommendation_status(
        &requested_symbol,
        &fusion_recent,
        |ts| state.price_snapshot_at_or_before(ts),
        |ts| state.price_snapshots_since(ts),
    )))
}

pub async fn toxic_weight_recommendation_summary_route(
    State(state): State<AppState>,
    Query(query): Query<ToxicWeightRecommendationQuery>,
) -> Json<serde_json::Value> {
    let requested_symbol = query
        .symbol
        .unwrap_or_else(|| state.config().symbol.clone());
    Json(serde_json::json!(build_summary(&state, &requested_symbol)))
}

pub async fn toxic_weight_recommendation_for_symbol(
    State(state): State<AppState>,
    Path(symbol): Path<String>,
) -> Json<serde_json::Value> {
    Json(serde_json::json!(build_summary(&state, &symbol)))
}

fn build_summary(
    state: &AppState,
    requested_symbol: &str,
) -> crate::types::toxic_weight_recommendation::ToxicWeightRecommendationSummaryResponse {
    let fusion_recent = build_fusion_recent(state, requested_symbol);
    toxic_weight_recommendation_summary(
        requested_symbol,
        &fusion_recent,
        |ts| state.price_snapshot_at_or_before(ts),
        |ts| state.price_snapshots_since(ts),
    )
}
