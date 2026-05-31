use axum::{
    extract::{Path, Query, State},
    Json,
};
use serde::Deserialize;

use crate::{
    api::{
        toxic_quality_scorecard_routes::build_fusion_recent,
        toxic_signal_group_routes::build_recent as build_group_recent,
        toxic_signal_inbox_routes::{build_recent as build_inbox_recent, normalize_symbol_query},
    },
    app::AppState,
    toxicity::{
        toxic_governance_ledger_service::toxic_governance_ledger_summary,
        toxic_markout_service::toxic_markout_recent,
        toxic_quality_scorecard_service::toxic_quality_scorecard_summary,
        toxic_replay_service::replay_recent,
        toxic_signal_detail::ToxicSignalDetailContext,
        toxic_signal_detail_service::{
            toxic_signal_detail_by_group_id, toxic_signal_detail_by_signal_id,
            toxic_signal_detail_status,
        },
        toxic_weight_recommendation_service::toxic_weight_recommendation_summary,
    },
};

#[derive(Debug, Deserialize)]
pub struct ToxicSignalDetailQuery {
    symbol: Option<String>,
}

struct ToxicSignalDetailOwnedContext {
    fusion_recent: crate::types::toxic_signal::ToxicSignalRecentResponse,
    replay_recent: crate::types::toxic_replay::ToxicReplayRecentResponse,
    markout_recent: crate::types::toxic_markout::ToxicMarkoutRecentResponse,
    quality_summary: crate::types::toxic_quality_scorecard::ToxicQualityScorecardSummaryResponse,
    recommendation_summary:
        crate::types::toxic_weight_recommendation::ToxicWeightRecommendationSummaryResponse,
    governance_summary: crate::types::toxic_governance_ledger::ToxicGovernanceLedgerSummaryResponse,
    inbox_recent: crate::types::toxic_signal_inbox::ToxicSignalInboxRecentResponse,
    group_recent: crate::types::toxic_signal_group::ToxicSignalGroupRecentResponse,
}

impl ToxicSignalDetailOwnedContext {
    fn as_context(&self) -> ToxicSignalDetailContext<'_> {
        ToxicSignalDetailContext {
            fusion_recent: &self.fusion_recent,
            replay_recent: &self.replay_recent,
            markout_recent: &self.markout_recent,
            quality_summary: &self.quality_summary,
            recommendation_summary: &self.recommendation_summary,
            governance_summary: &self.governance_summary,
            inbox_recent: &self.inbox_recent,
            group_recent: &self.group_recent,
        }
    }
}

pub async fn toxic_signal_detail_status_route(
    State(state): State<AppState>,
    Query(query): Query<ToxicSignalDetailQuery>,
) -> Json<serde_json::Value> {
    let requested_symbol = normalize_symbol_query(query.symbol, &state.config().symbol);
    let context = build_context(&state, &requested_symbol);
    Json(serde_json::json!(toxic_signal_detail_status(
        &requested_symbol,
        &context.as_context(),
    )))
}

pub async fn toxic_signal_detail_for_signal(
    State(state): State<AppState>,
    Path(signal_id): Path<String>,
    Query(query): Query<ToxicSignalDetailQuery>,
) -> Json<serde_json::Value> {
    let requested_symbol = normalize_symbol_query(query.symbol, &state.config().symbol);
    let context = build_context(&state, &requested_symbol);
    Json(serde_json::json!(toxic_signal_detail_by_signal_id(
        &requested_symbol,
        &signal_id,
        &context.as_context(),
    )))
}

pub async fn toxic_signal_detail_for_group(
    State(state): State<AppState>,
    Path(group_id): Path<String>,
    Query(query): Query<ToxicSignalDetailQuery>,
) -> Json<serde_json::Value> {
    let requested_symbol = normalize_symbol_query(query.symbol, &state.config().symbol);
    let context = build_context(&state, &requested_symbol);
    Json(serde_json::json!(toxic_signal_detail_by_group_id(
        &requested_symbol,
        &group_id,
        &context.as_context(),
    )))
}

fn build_context(state: &AppState, requested_symbol: &str) -> ToxicSignalDetailOwnedContext {
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
    let inbox_recent = build_inbox_recent(state, requested_symbol);
    let group_recent = build_group_recent(state, requested_symbol);

    ToxicSignalDetailOwnedContext {
        fusion_recent,
        replay_recent,
        markout_recent,
        quality_summary,
        recommendation_summary,
        governance_summary,
        inbox_recent,
        group_recent,
    }
}
