use axum::{
    extract::{Query, State},
    Json,
};
use serde::Deserialize;

use crate::{
    app::AppState,
    contract_whale_monitor::{
        aggregator::market_context_from_snapshots,
        types::{
            ContractFlowBucket, ContractLiquidationBucket, ContractWhaleDirection,
            ContractWhaleSignal,
        },
    },
    market_regime_engine::{
        analyze_market_regime, normalize_market_symbol, DirectionBias, ManipulationAssessment,
        MarketFeatureSet, MarketRegimeAssessment, MarketSignalAssessment,
    },
    normalizers::trade::now_ms,
    storage::contract_whale_repo::{ContractWhaleRepo, ContractWhaleSignalQuery},
};

#[derive(Debug, Deserialize, Default)]
pub struct MarketRegimeQuery {
    pub symbol: Option<String>,
}

pub async fn market_regime_latest_route(
    State(state): State<AppState>,
    Query(query): Query<MarketRegimeQuery>,
) -> Json<MarketRegimeAssessment> {
    let features = build_latest_market_features(&state, query.symbol.as_deref());
    Json(analyze_market_regime(&features).regime)
}

pub async fn manipulation_latest_route(
    State(state): State<AppState>,
    Query(query): Query<MarketRegimeQuery>,
) -> Json<ManipulationAssessment> {
    let features = build_latest_market_features(&state, query.symbol.as_deref());
    Json(analyze_market_regime(&features).manipulation)
}

pub async fn market_signal_latest_route(
    State(state): State<AppState>,
    Query(query): Query<MarketRegimeQuery>,
) -> Json<MarketSignalAssessment> {
    let features = build_latest_market_features(&state, query.symbol.as_deref());
    Json(analyze_market_regime(&features).signal)
}

fn build_latest_market_features(
    state: &AppState,
    requested_symbol: Option<&str>,
) -> MarketFeatureSet {
    let symbol = requested_symbol
        .map(normalize_market_symbol)
        .unwrap_or_else(|| normalize_market_symbol(&state.config().symbol));
    let now = now_ms();
    let Some(store) = state.contract_whale_store() else {
        return MarketFeatureSet {
            symbol,
            data_quality: Some(0.0),
            ..MarketFeatureSet::default()
        };
    };

    let latest_signal = store
        .query_contract_whale_signals(&ContractWhaleSignalQuery {
            symbol: Some(symbol.clone()),
            limit: 1,
            ..ContractWhaleSignalQuery::default()
        })
        .ok()
        .and_then(|mut items| items.drain(..).next());

    let from = now.saturating_sub(10 * 60_000);
    let oi_snapshots = store
        .list_contract_oi_snapshots_between(&symbol, from, now)
        .unwrap_or_default();
    let funding_snapshots = store
        .list_contract_funding_snapshots_between(&symbol, from, now)
        .unwrap_or_default();
    let market_context =
        market_context_from_snapshots(&oi_snapshots, &funding_snapshots, &symbol, now);
    let flow_buckets = store
        .list_contract_flow_buckets_between(&symbol, now.saturating_sub(60_000), now)
        .unwrap_or_default();
    let liquidation_buckets = store
        .list_contract_liquidation_buckets_between(&symbol, now.saturating_sub(60_000), now)
        .unwrap_or_default();

    features_from_latest_inputs(
        &symbol,
        latest_signal.as_ref(),
        &flow_buckets,
        &liquidation_buckets,
        market_context.oi_change_pct,
        market_context.funding_rate,
    )
}

fn features_from_latest_inputs(
    symbol: &str,
    signal: Option<&ContractWhaleSignal>,
    flow_buckets: &[ContractFlowBucket],
    liquidation_buckets: &[ContractLiquidationBucket],
    oi_change_pct: Option<f64>,
    funding_rate: Option<f64>,
) -> MarketFeatureSet {
    let flow_volume_btc = flow_buckets
        .iter()
        .map(|bucket| bucket.buy_volume_btc + bucket.sell_volume_btc)
        .sum::<f64>();
    let liquidation_btc = liquidation_buckets
        .iter()
        .map(|bucket| bucket.long_liq_btc + bucket.short_liq_btc)
        .sum::<f64>();
    let liquidation_ratio_from_buckets = (flow_volume_btc > f64::EPSILON)
        .then_some((liquidation_btc / flow_volume_btc).clamp(0.0, 1.0));

    let price_change = signal.and_then(|signal| {
        signal
            .price_move_pct
            .or(signal.price_move_30s_pct)
            .or(signal.price_move_15s_pct)
            .or(signal.price_move_5s_pct)
    });
    let total_notional_m = signal
        .map(|signal| (signal.total_notional_usd / 1_000_000.0).max(0.001))
        .unwrap_or(0.001);
    let price_impact_efficiency = price_change.map(|price| price.abs() / total_notional_m);

    MarketFeatureSet {
        symbol: symbol.to_string(),
        price_change_5m_pct: price_change,
        oi_change_pct: signal
            .and_then(|signal| signal.oi_change_pct)
            .or(oi_change_pct),
        volume_spike_multiple: signal
            .and_then(|signal| signal.dynamic_multiple)
            .or_else(|| signal.map(|_| 1.0)),
        funding_rate: signal
            .and_then(|signal| signal.funding_rate)
            .or(funding_rate),
        spot_futures_divergence_pct: signal.and_then(|signal| signal.price_deviation_pct),
        liquidation_ratio: signal
            .and_then(|signal| signal.liquidation_ratio)
            .or(liquidation_ratio_from_buckets),
        price_impact_efficiency,
        flow_direction: signal.map(signal_direction_bias),
        data_quality: signal.map(|signal| signal.data_quality as f64 / 100.0),
    }
}

fn signal_direction_bias(signal: &ContractWhaleSignal) -> DirectionBias {
    match signal.direction {
        ContractWhaleDirection::Buy => DirectionBias::Long,
        ContractWhaleDirection::Sell => DirectionBias::Short,
        ContractWhaleDirection::Absorption | ContractWhaleDirection::Suppression => {
            DirectionBias::Neutral
        }
    }
}
