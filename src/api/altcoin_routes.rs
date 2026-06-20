use axum::{
    extract::{Query, State},
    Json,
};
use serde::Deserialize;

use crate::{
    altcoin_manipulation_engine::{
        analyze_altcoin_manipulation, AltcoinManipulationInput, AltcoinManipulationState,
    },
    app::AppState,
    contract_whale_monitor::{
        aggregator::market_context_from_snapshots,
        types::{ContractFlowBucket, ContractWhaleDirection},
    },
    market_domain::{classify_market_domain, MarketDomain},
    market_regime_engine::normalize_market_symbol,
    normalizers::trade::now_ms,
    storage::contract_whale_repo::{ContractWhaleRepo, ContractWhaleSignalQuery},
};

#[derive(Debug, Deserialize, Default)]
pub struct AltcoinQuery {
    pub symbol: Option<String>,
}

pub async fn altcoin_manipulation_route(
    State(state): State<AppState>,
    Query(query): Query<AltcoinQuery>,
) -> Json<AltcoinManipulationState> {
    Json(build_altcoin_state(&state, query.symbol.as_deref()))
}

pub async fn altcoin_regime_route(
    State(state): State<AppState>,
    Query(query): Query<AltcoinQuery>,
) -> Json<AltcoinManipulationState> {
    Json(build_altcoin_state(&state, query.symbol.as_deref()))
}

pub async fn altcoin_fusion_route(
    State(state): State<AppState>,
    Query(query): Query<AltcoinQuery>,
) -> Json<AltcoinManipulationState> {
    Json(build_altcoin_state(&state, query.symbol.as_deref()))
}

pub async fn altcoin_signals_route(
    State(state): State<AppState>,
    Query(query): Query<AltcoinQuery>,
) -> Json<AltcoinManipulationState> {
    Json(build_altcoin_state(&state, query.symbol.as_deref()))
}

fn build_altcoin_state(
    state: &AppState,
    requested_symbol: Option<&str>,
) -> AltcoinManipulationState {
    let symbol = requested_symbol
        .map(normalize_market_symbol)
        .unwrap_or_else(|| "ETH".to_string());
    if classify_market_domain(&symbol) == MarketDomain::BtcStructure {
        return AltcoinManipulationState {
            symbol: "BTC".to_string(),
            regime: "ACCUMULATION".to_string(),
            bias: "NEUTRAL".to_string(),
            confidence: 0.0,
            manipulation_score: 0.0,
            oi_signal_score: 0.0,
            volume_signal_score: 0.0,
            funding_signal_score: 0.0,
            price_signal_score: 0.0,
            pump_dump_score: 0.0,
            signals: vec!["BTC_ROUTE_ISOLATED_FROM_ALTCOIN_ENGINE".to_string()],
            risk_tags: Vec::new(),
            metrics: Default::default(),
            read_only: true,
            runtime_modified: false,
        };
    }

    let now = now_ms();
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
    let (flow_bias_score, buy_volume, sell_volume, price_change_from_flow) = state
        .contract_whale_store()
        .map_or((0.0, 0.0, 0.0, None), |store| {
            let buckets = store
                .list_contract_flow_buckets_between(&symbol, now.saturating_sub(5 * 60_000), now)
                .unwrap_or_default();
            let (buy, sell) = summarize_flow(&buckets);
            let total = buy + sell;
            let bias = if total > f64::EPSILON {
                (buy - sell) / total
            } else {
                0.0
            };
            (bias, buy, sell, price_change_pct(&buckets))
        });
    let (oi_change_pct, funding_rate) = state.contract_whale_store().map_or((0.0, 0.0), |store| {
        let from = now.saturating_sub(10 * 60_000);
        let oi_snapshots = store
            .list_contract_oi_snapshots_between(&symbol, from, now)
            .unwrap_or_default();
        let funding_snapshots = store
            .list_contract_funding_snapshots_between(&symbol, from, now)
            .unwrap_or_default();
        let context =
            market_context_from_snapshots(&oi_snapshots, &funding_snapshots, &symbol, now);
        (
            context.oi_change_pct.unwrap_or(0.0),
            context.funding_rate.unwrap_or(0.0),
        )
    });
    let price_change_pct = price_change_from_flow
        .or_else(|| {
            latest_signal.as_ref().and_then(|signal| {
                signal
                    .price_move_pct
                    .or(signal.price_move_30s_pct)
                    .or(signal.price_move_15s_pct)
                    .or(signal.price_move_5s_pct)
            })
        })
        .unwrap_or(0.0);
    let total_volume = buy_volume + sell_volume;
    let volume_spike_multiple = (total_volume / 500.0).clamp(0.0, 5.0).max(
        latest_signal
            .as_ref()
            .and_then(|signal| signal.dynamic_multiple)
            .unwrap_or(0.0),
    );
    let total_notional_m = latest_signal
        .as_ref()
        .map(|signal| (signal.total_notional_usd / 1_000_000.0).max(0.001))
        .unwrap_or(0.001);
    let price_impact_efficiency = (price_change_pct.abs() / total_notional_m).clamp(0.0, 1.0);
    let liquidation_ratio = latest_signal
        .as_ref()
        .and_then(|signal| signal.liquidation_ratio)
        .unwrap_or(0.0);
    let data_quality = latest_signal
        .as_ref()
        .map(|signal| signal.data_quality as f64 / 100.0)
        .unwrap_or(0.70);
    let flow_bias = latest_signal
        .as_ref()
        .map(|signal| match signal.direction {
            ContractWhaleDirection::Buy | ContractWhaleDirection::Absorption => 1.0,
            ContractWhaleDirection::Sell | ContractWhaleDirection::Suppression => -1.0,
        })
        .unwrap_or(flow_bias_score);

    analyze_altcoin_manipulation(&AltcoinManipulationInput {
        symbol,
        price_change_pct,
        oi_change_pct,
        volume_spike_multiple,
        funding_rate,
        liquidation_ratio,
        price_impact_efficiency,
        flow_bias_score: flow_bias,
        data_quality,
    })
}

fn summarize_flow(buckets: &[ContractFlowBucket]) -> (f64, f64) {
    buckets.iter().fold((0.0, 0.0), |(buy, sell), bucket| {
        (
            buy + bucket.buy_volume_btc.max(0.0),
            sell + bucket.sell_volume_btc.max(0.0),
        )
    })
}

fn price_change_pct(buckets: &[ContractFlowBucket]) -> Option<f64> {
    let first = buckets.iter().find_map(|bucket| bucket.vwap)?;
    let last = buckets.iter().rev().find_map(|bucket| bucket.vwap)?;
    (first.abs() > f64::EPSILON).then_some(((last - first) / first) * 100.0)
}
