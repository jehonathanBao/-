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
    normalizers::symbol::{canonical_base_asset, canonical_perp_symbol},
    normalizers::trade::now_ms,
    storage::contract_whale_repo::ContractWhaleRepo,
    types::liquidation::{empty_liquidation_state, LiquidationState},
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
    let liquidation_state =
        liquidation_state_for_symbol(&symbol, &state.liquidation_state(), now_ms());
    Json(leverage_map_from_liquidation_state(
        &symbol,
        &liquidation_state,
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

    let flow_state = state.flow_state_for_symbol(&symbol);
    let liquidation_state = liquidation_state_for_symbol(&symbol, &state.liquidation_state(), now);

    input_from_runtime_state(
        &symbol,
        &flow_state,
        &liquidation_state,
        oi_change_pct,
        funding_rate,
    )
}

fn requested_symbol(state: &AppState, requested: Option<&str>) -> String {
    requested
        .map(normalize_market_symbol)
        .unwrap_or_else(|| normalize_market_symbol(&state.config().symbol))
}

fn liquidation_state_for_symbol(
    requested_symbol: &str,
    observed_state: &LiquidationState,
    now: i64,
) -> LiquidationState {
    let requested_base = canonical_base_asset(requested_symbol);
    let observed_base = canonical_base_asset(&observed_state.symbol);
    if requested_base.is_some() && requested_base == observed_base {
        return observed_state.clone();
    }

    let mut unavailable = empty_liquidation_state(now);
    unavailable.symbol = canonical_perp_symbol(requested_symbol)
        .unwrap_or_else(|| requested_symbol.trim().to_ascii_uppercase());
    unavailable.metrics.reason_codes = vec!["liquidation_symbol_unavailable".to_string()];
    unavailable
}

#[cfg(test)]
mod tests {
    use super::liquidation_state_for_symbol;
    use crate::types::{
        liquidation::{
            empty_liquidation_state, EstimatedLiquidationCluster, LiquidationClusterSide,
        },
        toxic::ToxicDirection,
    };

    #[test]
    fn mismatched_global_liquidation_state_becomes_symbol_scoped_unavailable_state() {
        let mut btc_state = empty_liquidation_state(100);
        btc_state.symbol = "BTC-PERP".to_string();
        btc_state.metrics.enabled = true;
        btc_state.metrics.current_mid = Some(100_000.0);
        btc_state.metrics.dominant_direction = ToxicDirection::Buy;
        btc_state.recent_clusters = vec![EstimatedLiquidationCluster {
            side: LiquidationClusterSide::ShortAbove,
            price: 100_500.0,
            distance_bps: 50.0,
            cluster_notional_usd: 25_000_000.0,
            cluster_density: 0.9,
            touched_snapshots: 4,
            first_seen_ts: 10,
            last_seen_ts: 90,
            reason_codes: vec!["price_cluster_detected".to_string()],
        }];

        let scoped = liquidation_state_for_symbol("ETH", &btc_state, 200);

        assert_eq!(scoped.symbol, "ETH-PERP");
        assert!(!scoped.metrics.enabled);
        assert_eq!(scoped.metrics.current_mid, None);
        assert!(scoped.recent_clusters.is_empty());
        assert_eq!(
            scoped.metrics.reason_codes,
            vec!["liquidation_symbol_unavailable".to_string()]
        );
    }

    #[test]
    fn canonical_matching_liquidation_state_preserves_observed_clusters() {
        let mut btc_state = empty_liquidation_state(100);
        btc_state.symbol = "BTC-PERP".to_string();
        btc_state.metrics.enabled = true;
        btc_state.metrics.current_mid = Some(100_000.0);

        let scoped = liquidation_state_for_symbol("BTCUSDT", &btc_state, 200);

        assert_eq!(scoped.symbol, "BTC-PERP");
        assert!(scoped.metrics.enabled);
        assert_eq!(scoped.metrics.current_mid, Some(100_000.0));
    }
}
