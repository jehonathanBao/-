use axum::{
    extract::{Query, State},
    Json,
};
use serde::Deserialize;

use crate::{
    app::AppState,
    btc_structure_engine::{analyze_btc_structure, BtcStructureInput, BtcStructureState},
    contract_whale_monitor::{
        aggregator::market_context_from_snapshots, types::ContractWhaleDirection,
    },
    liquidation_cascade_predictor::{
        analyze_liquidation_cascade, input_from_runtime_state, LcpCascadeResponse,
    },
    market_regime_engine::normalize_market_symbol,
    normalizers::trade::now_ms,
    storage::contract_whale_repo::{ContractWhaleRepo, ContractWhaleSignalQuery},
};

#[derive(Debug, Deserialize, Default)]
pub struct BtcStructureQuery {
    pub symbol: Option<String>,
}

pub async fn btc_structure_route(
    State(state): State<AppState>,
    Query(query): Query<BtcStructureQuery>,
) -> Json<BtcStructureState> {
    Json(analyze_btc_structure(&build_btc_structure_input(
        &state,
        query.symbol.as_deref(),
    )))
}

pub async fn btc_regime_route(
    State(state): State<AppState>,
    Query(query): Query<BtcStructureQuery>,
) -> Json<BtcStructureState> {
    Json(analyze_btc_structure(&build_btc_structure_input(
        &state,
        query.symbol.as_deref(),
    )))
}

pub async fn btc_liquidation_route(
    State(state): State<AppState>,
    Query(query): Query<BtcStructureQuery>,
) -> Json<LcpCascadeResponse> {
    let input = build_btc_lcp_input(&state, query.symbol.as_deref());
    Json(analyze_liquidation_cascade(&input))
}

fn build_btc_structure_input(
    state: &AppState,
    requested_symbol: Option<&str>,
) -> BtcStructureInput {
    let symbol = btc_symbol(requested_symbol);
    let latest_signal = state.contract_whale_store().and_then(|store| {
        store
            .query_contract_whale_signals(&ContractWhaleSignalQuery {
                symbol: Some(symbol.clone()),
                limit: 1,
                ..ContractWhaleSignalQuery::default()
            })
            .ok()
            .and_then(|mut signals| signals.drain(..).next())
    });
    let lcp_input = build_btc_lcp_input(state, requested_symbol);
    let cascade = analyze_liquidation_cascade(&lcp_input);
    let (oi_change_pct, funding_rate) = btc_market_context(state, &symbol);
    let flow_bias_score = latest_signal
        .as_ref()
        .map(|signal| match signal.direction {
            ContractWhaleDirection::Buy | ContractWhaleDirection::Absorption => 1.0,
            ContractWhaleDirection::Sell | ContractWhaleDirection::Suppression => -1.0,
        })
        .unwrap_or(0.0);
    let gamma_pressure = (cascade.cascade_probability * 0.55
        + (funding_rate.abs() / 0.001).clamp(0.0, 1.0) * 0.45)
        .clamp(0.0, 1.0);

    BtcStructureInput {
        symbol,
        flow_bias_score,
        oi_change_pct,
        funding_rate,
        liquidation_cascade_probability: cascade.cascade_probability,
        liquidation_direction: cascade.direction.as_key().to_string(),
        gamma_pressure,
        data_quality: latest_signal
            .as_ref()
            .map(|signal| signal.data_quality as f64 / 100.0)
            .unwrap_or(0.75),
    }
}

fn build_btc_lcp_input(
    state: &AppState,
    requested_symbol: Option<&str>,
) -> crate::liquidation_cascade_predictor::LcpInput {
    let symbol = btc_symbol(requested_symbol);
    let (oi_change_pct, funding_rate) = btc_market_context(state, &symbol);
    input_from_runtime_state(
        &symbol,
        &state.flow_state_for_symbol(&symbol),
        &state.liquidation_state(),
        Some(oi_change_pct),
        Some(funding_rate),
    )
}

fn btc_market_context(state: &AppState, symbol: &str) -> (f64, f64) {
    let now = now_ms();
    state
        .contract_whale_store()
        .map(|store| {
            let from = now.saturating_sub(10 * 60_000);
            let oi_snapshots = store
                .list_contract_oi_snapshots_between(symbol, from, now)
                .unwrap_or_default();
            let funding_snapshots = store
                .list_contract_funding_snapshots_between(symbol, from, now)
                .unwrap_or_default();
            let context =
                market_context_from_snapshots(&oi_snapshots, &funding_snapshots, symbol, now);
            (
                context.oi_change_pct.unwrap_or(0.0),
                context.funding_rate.unwrap_or(0.0),
            )
        })
        .unwrap_or((0.0, 0.0))
}

fn btc_symbol(requested_symbol: Option<&str>) -> String {
    let normalized = requested_symbol
        .map(normalize_market_symbol)
        .unwrap_or_else(|| "BTC".to_string());
    if normalized == "BTC" {
        normalized
    } else {
        "BTC".to_string()
    }
}
