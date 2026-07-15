use axum::{
    extract::{Path, Query, State},
    Json,
};
use serde::Deserialize;

use crate::{
    app::AppState,
    toxicity::{
        active_trade_toxicity_service::build_active_trade_toxicity_recent,
        liquidation_toxicity_service::build_liquidation_toxicity_recent,
        orderbook_wall_interpretation_service::build_orderbook_wall_interpretation_recent,
        orderbook_wall_lifecycle::build_orderbook_wall_lifecycle_report,
        structural_toxicity_service::build_structural_toxicity_recent,
        toxic_markout_service::{
            toxic_markout_by_signal_id, toxic_markout_recent, toxic_markout_status,
        },
        toxic_signal_fusion_service::build_toxic_signal_fusion_recent,
    },
    types::{
        orderbook_wall::{OrderbookWallInterpretationReport, OrderbookWallLifecycleReport},
        toxic_markout::{ToxicMarkoutDetailResponse, ToxicMarkoutRecentResponse},
        toxic_signal::ToxicSignalRecentResponse,
    },
};

#[derive(Debug, Deserialize)]
pub struct ToxicMarkoutQuery {
    symbol: Option<String>,
}

pub async fn toxic_markout_status_route(
    State(state): State<AppState>,
    Query(query): Query<ToxicMarkoutQuery>,
) -> Json<serde_json::Value> {
    let requested_symbol = query
        .symbol
        .unwrap_or_else(|| state.config().symbol.clone());
    let fusion_recent = build_fusion_recent(&state, &requested_symbol);
    Json(serde_json::json!(toxic_markout_status(
        &requested_symbol,
        &fusion_recent,
        |ts| state.price_snapshot_at_or_before_for_symbol(ts, &requested_symbol),
        |ts| state.price_snapshots_since_for_symbol(ts, &requested_symbol),
    )))
}

pub async fn toxic_markout_recent_route(
    State(state): State<AppState>,
    Query(query): Query<ToxicMarkoutQuery>,
) -> Json<serde_json::Value> {
    let requested_symbol = query
        .symbol
        .unwrap_or_else(|| state.config().symbol.clone());
    Json(serde_json::json!(build_recent(&state, &requested_symbol)))
}

pub async fn toxic_markout_for_symbol(
    State(state): State<AppState>,
    Path(symbol): Path<String>,
) -> Json<serde_json::Value> {
    Json(serde_json::json!(build_recent(&state, &symbol)))
}

pub async fn toxic_markout_for_signal(
    State(state): State<AppState>,
    Path(signal_id): Path<String>,
    Query(query): Query<ToxicMarkoutQuery>,
) -> Json<serde_json::Value> {
    let requested_symbol = query
        .symbol
        .unwrap_or_else(|| state.config().symbol.clone());
    Json(serde_json::json!(build_signal(
        &state,
        &requested_symbol,
        &signal_id
    )))
}

pub(crate) fn build_recent(state: &AppState, requested_symbol: &str) -> ToxicMarkoutRecentResponse {
    let fusion_recent = build_fusion_recent(state, requested_symbol);
    toxic_markout_recent(
        requested_symbol,
        &fusion_recent,
        |ts| state.price_snapshot_at_or_before_for_symbol(ts, requested_symbol),
        |ts| state.price_snapshots_since_for_symbol(ts, requested_symbol),
    )
}

fn build_signal(
    state: &AppState,
    requested_symbol: &str,
    signal_id: &str,
) -> ToxicMarkoutDetailResponse {
    let fusion_recent = build_fusion_recent(state, requested_symbol);
    toxic_markout_by_signal_id(
        requested_symbol,
        signal_id,
        &fusion_recent,
        |ts| state.price_snapshot_at_or_before_for_symbol(ts, requested_symbol),
        |ts| state.price_snapshots_since_for_symbol(ts, requested_symbol),
    )
}

fn build_fusion_recent(state: &AppState, requested_symbol: &str) -> ToxicSignalRecentResponse {
    let active_trade_recent = build_active_trade_toxicity_recent(
        requested_symbol,
        &state.flow_state(),
        &state.sweep_state(),
        &state.markout_state(),
    );
    let liquidation_recent = build_liquidation_toxicity_recent(
        requested_symbol,
        &state.liquidation_state(),
        &state.flow_state(),
        &state.sweep_state(),
        &state.markout_state(),
    );
    let wall_lifecycle_report = build_lifecycle_report(state, requested_symbol);
    let wall_interpretation_report = build_interpretation_report(
        requested_symbol,
        &wall_lifecycle_report,
        &active_trade_recent,
        &liquidation_recent,
    );
    let structural_recent = build_structural_toxicity_recent(
        requested_symbol,
        &active_trade_recent,
        &liquidation_recent,
        &wall_lifecycle_report,
        &wall_interpretation_report,
    );
    build_toxic_signal_fusion_recent(
        requested_symbol,
        &active_trade_recent,
        &liquidation_recent,
        &wall_lifecycle_report,
        &wall_interpretation_report,
        &structural_recent,
    )
}

fn build_lifecycle_report(
    state: &AppState,
    requested_symbol: &str,
) -> OrderbookWallLifecycleReport {
    let wall_state = state.orderbook_wall_lifecycle_state();
    if !wall_state.symbol.eq_ignore_ascii_case(requested_symbol) {
        return OrderbookWallLifecycleReport {
            read_only: true,
            runtime_modified: false,
            analysis_mode: "analysis_only".to_string(),
            symbol: requested_symbol.to_string(),
            generated_at_ms: wall_state.generated_at_ms,
            status: "insufficient_data".to_string(),
            tracked_walls: Vec::new(),
            recent_events: Vec::new(),
            toxicity_candidates: Vec::new(),
            warnings: vec![format!(
                "runtime wall tracker is currently scoped to {}",
                wall_state.symbol
            )],
            no_trade_reasons: vec![
                "selected symbol is not the runtime wall-tracking symbol".to_string()
            ],
        };
    }
    let active_trade_recent = build_active_trade_toxicity_recent(
        requested_symbol,
        &state.flow_state(),
        &state.sweep_state(),
        &state.markout_state(),
    );
    let liquidation_recent = build_liquidation_toxicity_recent(
        requested_symbol,
        &state.liquidation_state(),
        &state.flow_state(),
        &state.sweep_state(),
        &state.markout_state(),
    );
    build_orderbook_wall_lifecycle_report(&wall_state, &active_trade_recent, &liquidation_recent)
}

fn build_interpretation_report(
    requested_symbol: &str,
    wall_lifecycle_report: &OrderbookWallLifecycleReport,
    active_trade_recent: &crate::types::toxic_flow::ActiveTradeToxicityRecentResponse,
    liquidation_recent: &crate::types::liquidation::LiquidationToxicityRecentResponse,
) -> OrderbookWallInterpretationReport {
    build_orderbook_wall_interpretation_recent(
        requested_symbol,
        wall_lifecycle_report,
        active_trade_recent,
        liquidation_recent,
    )
}
