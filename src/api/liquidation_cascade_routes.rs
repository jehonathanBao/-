use axum::{
    extract::{Query, State},
    Json,
};
use serde::Deserialize;

use crate::{
    app::AppState,
    contract_whale_monitor::aggregator::market_context_from_snapshots,
    liquidation_cascade_predictor::{
        analyze_liquidation_cascade, input_from_runtime_state, leverage_map_from_liquidation_state,
        liquidity_gap_from_input, LcpCascadeResponse, LcpInput, LcpLeverageMapResponse,
        LcpLiquidityGapResponse,
    },
    market_regime_engine::normalize_market_symbol,
    normalizers::trade::now_ms,
    storage::contract_whale_repo::ContractWhaleRepo,
};

#[derive(Debug, Deserialize, Default)]
pub struct LiquidationCascadeQuery {
    pub symbol: Option<String>,
}

pub async fn liquidation_cascade_route(
    State(state): State<AppState>,
    Query(query): Query<LiquidationCascadeQuery>,
) -> Json<LcpCascadeResponse> {
    let input = build_latest_lcp_input(&state, query.symbol.as_deref());
    Json(analyze_liquidation_cascade(&input))
}

pub async fn liquidation_leverage_map_route(
    State(state): State<AppState>,
    Query(query): Query<LiquidationCascadeQuery>,
) -> Json<LcpLeverageMapResponse> {
    let symbol = requested_symbol(&state, query.symbol.as_deref());
    Json(leverage_map_from_liquidation_state(
        &symbol,
        &state.liquidation_state(),
    ))
}

pub async fn liquidation_liquidity_gap_route(
    State(state): State<AppState>,
    Query(query): Query<LiquidationCascadeQuery>,
) -> Json<LcpLiquidityGapResponse> {
    let input = build_latest_lcp_input(&state, query.symbol.as_deref());
    Json(liquidity_gap_from_input(&input))
}

fn build_latest_lcp_input(state: &AppState, requested: Option<&str>) -> LcpInput {
    let symbol = requested_symbol(state, requested);
    let now = now_ms();
    let (oi_change_pct, funding_rate) = state
        .contract_whale_store()
        .map(|store| {
            let from = now.saturating_sub(10 * 60_000);
            let oi_snapshots = store
                .list_contract_oi_snapshots_between(&symbol, from, now)
                .unwrap_or_default();
            let funding_snapshots = store
                .list_contract_funding_snapshots_between(&symbol, from, now)
                .unwrap_or_default();
            let context =
                market_context_from_snapshots(&oi_snapshots, &funding_snapshots, &symbol, now);
            (context.oi_change_pct, context.funding_rate)
        })
        .unwrap_or((None, None));

    input_from_runtime_state(
        &symbol,
        &state.flow_state(),
        &state.liquidation_state(),
        oi_change_pct,
        funding_rate,
    )
}

fn requested_symbol(state: &AppState, requested: Option<&str>) -> String {
    requested
        .map(normalize_market_symbol)
        .unwrap_or_else(|| normalize_market_symbol(&state.config().symbol))
}
