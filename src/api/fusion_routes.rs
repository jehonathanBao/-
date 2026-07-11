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
    liquidation_cascade_predictor::{analyze_liquidation_cascade, input_from_runtime_state},
    market_domain::classify_market_domain,
    market_regime_engine::normalize_market_symbol,
    multi_timeframe_orderflow_fusion::{
        analyze_mtf_orderflow_fusion, MtfofeBreakdownResponse, MtfofeDecisionResponse, MtfofeInput,
        MtfofeLayerInput, MtfofeStateResponse,
    },
    normalizers::trade::now_ms,
    storage::contract_whale_repo::{ContractWhaleRepo, ContractWhaleSignalQuery},
};

#[derive(Debug, Deserialize, Default)]
pub struct FusionQuery {
    pub symbol: Option<String>,
}

pub async fn fusion_state_route(
    State(state): State<AppState>,
    Query(query): Query<FusionQuery>,
) -> Json<MtfofeStateResponse> {
    let input = build_latest_fusion_input(&state, query.symbol.as_deref());
    Json(analyze_mtf_orderflow_fusion(&input))
}

pub async fn fusion_decision_route(
    State(state): State<AppState>,
    Query(query): Query<FusionQuery>,
) -> Json<MtfofeDecisionResponse> {
    let input = build_latest_fusion_input(&state, query.symbol.as_deref());
    Json(analyze_mtf_orderflow_fusion(&input).into())
}

pub async fn fusion_tf_breakdown_route(
    State(state): State<AppState>,
    Query(query): Query<FusionQuery>,
) -> Json<MtfofeBreakdownResponse> {
    let input = build_latest_fusion_input(&state, query.symbol.as_deref());
    Json(analyze_mtf_orderflow_fusion(&input).into())
}

fn build_latest_fusion_input(state: &AppState, requested_symbol: Option<&str>) -> MtfofeInput {
    let symbol = requested_symbol
        .map(normalize_market_symbol)
        .unwrap_or_else(|| normalize_market_symbol(&state.config().symbol));
    let market_domain = classify_market_domain(&symbol);
    let now = now_ms();
    let latest_signal = latest_contract_signal(state, &symbol);
    let lcp_input = input_from_runtime_state(
        &symbol,
        &state.flow_state_for_symbol(&symbol),
        &state.liquidation_state(),
        latest_signal
            .as_ref()
            .and_then(|signal| signal.oi_change_pct),
        latest_signal
            .as_ref()
            .and_then(|signal| signal.funding_rate),
    );
    let cascade = analyze_liquidation_cascade(&lcp_input);

    MtfofeInput {
        symbol: symbol.clone(),
        market_domain,
        micro_5s: build_layer(
            state,
            &symbol,
            now.saturating_sub(5_000),
            now,
            "5s",
            latest_signal.as_ref(),
        ),
        flow_60s: build_layer(
            state,
            &symbol,
            now.saturating_sub(60_000),
            now,
            "60s",
            latest_signal.as_ref(),
        ),
        structure_5m: build_layer(
            state,
            &symbol,
            now.saturating_sub(5 * 60_000),
            now,
            "5m",
            latest_signal.as_ref(),
        ),
        regime_1h: build_layer(
            state,
            &symbol,
            now.saturating_sub(60 * 60_000),
            now,
            "1h",
            latest_signal.as_ref(),
        ),
        liquidation_cascade_probability: cascade.cascade_probability,
        liquidation_cascade_direction: cascade.direction.as_key().to_string(),
    }
}

fn latest_contract_signal(state: &AppState, symbol: &str) -> Option<ContractWhaleSignal> {
    state.contract_whale_store().and_then(|store| {
        store
            .query_contract_whale_signals(&ContractWhaleSignalQuery {
                symbol: Some(symbol.to_string()),
                limit: 1,
                ..ContractWhaleSignalQuery::default()
            })
            .ok()
            .and_then(|mut signals| signals.drain(..).next())
    })
}

fn build_layer(
    state: &AppState,
    symbol: &str,
    from_ts: i64,
    to_ts: i64,
    timeframe: &str,
    latest_signal: Option<&ContractWhaleSignal>,
) -> MtfofeLayerInput {
    let Some(store) = state.contract_whale_store() else {
        return fallback_layer(timeframe, latest_signal);
    };
    let flow_buckets = store
        .list_contract_flow_buckets_between(symbol, from_ts, to_ts)
        .unwrap_or_default();
    let liquidation_buckets = store
        .list_contract_liquidation_buckets_between(symbol, from_ts, to_ts)
        .unwrap_or_default();
    let oi_snapshots = store
        .list_contract_oi_snapshots_between(symbol, from_ts, to_ts)
        .unwrap_or_default();
    let funding_snapshots = store
        .list_contract_funding_snapshots_between(symbol, from_ts, to_ts)
        .unwrap_or_default();
    let context = market_context_from_snapshots(&oi_snapshots, &funding_snapshots, symbol, to_ts);
    let flow = summarize_flow(&flow_buckets);
    let liquidation_ratio = liquidation_ratio(&flow_buckets, &liquidation_buckets)
        .or_else(|| latest_signal.and_then(|signal| signal.liquidation_ratio))
        .unwrap_or(0.0);
    let price_change_pct = price_change_pct(&flow_buckets)
        .or_else(|| latest_signal.and_then(signal_price_change_pct))
        .unwrap_or(0.0);
    let oi_change_pct = context
        .oi_change_pct
        .or_else(|| latest_signal.and_then(|signal| signal.oi_change_pct))
        .unwrap_or(0.0);
    let funding_rate = context
        .funding_rate
        .or_else(|| latest_signal.and_then(|signal| signal.funding_rate))
        .unwrap_or(0.0);
    let volume_spike_multiple = volume_spike_multiple(
        flow.buy_volume + flow.sell_volume,
        (to_ts.saturating_sub(from_ts) as f64 / 1000.0).max(1.0),
    )
    .or_else(|| latest_signal.and_then(|signal| signal.dynamic_multiple))
    .unwrap_or(0.0);
    let data_quality = data_quality(
        &flow_buckets,
        context.oi_available,
        context.funding_available,
        latest_signal,
    );

    MtfofeLayerInput {
        timeframe: timeframe.to_string(),
        buy_volume: flow.buy_volume,
        sell_volume: flow.sell_volume,
        price_change_pct,
        oi_change_pct,
        funding_rate,
        liquidation_ratio,
        volume_spike_multiple,
        data_quality,
    }
}

fn fallback_layer(
    timeframe: &str,
    latest_signal: Option<&ContractWhaleSignal>,
) -> MtfofeLayerInput {
    let mut layer = MtfofeLayerInput::new(timeframe);
    if let Some(signal) = latest_signal {
        match signal.direction {
            ContractWhaleDirection::Buy | ContractWhaleDirection::Absorption => {
                layer.buy_volume = signal.total_volume_btc.max(0.0);
            }
            ContractWhaleDirection::Sell | ContractWhaleDirection::Suppression => {
                layer.sell_volume = signal.total_volume_btc.max(0.0);
            }
        }
        layer.price_change_pct = signal_price_change_pct(signal).unwrap_or(0.0);
        layer.oi_change_pct = signal.oi_change_pct.unwrap_or(0.0);
        layer.funding_rate = signal.funding_rate.unwrap_or(0.0);
        layer.liquidation_ratio = signal.liquidation_ratio.unwrap_or(0.0);
        layer.volume_spike_multiple = signal.dynamic_multiple.unwrap_or(0.0);
        layer.data_quality = signal.data_quality as f64 / 100.0;
    }
    layer
}

#[derive(Debug, Clone, Copy, Default)]
struct FlowSummary {
    buy_volume: f64,
    sell_volume: f64,
}

fn summarize_flow(buckets: &[ContractFlowBucket]) -> FlowSummary {
    buckets
        .iter()
        .fold(FlowSummary::default(), |mut acc, bucket| {
            acc.buy_volume += bucket.buy_volume_btc.max(0.0);
            acc.sell_volume += bucket.sell_volume_btc.max(0.0);
            acc
        })
}

fn liquidation_ratio(
    flow_buckets: &[ContractFlowBucket],
    liquidation_buckets: &[ContractLiquidationBucket],
) -> Option<f64> {
    let total_flow = flow_buckets
        .iter()
        .map(|bucket| bucket.buy_volume_btc + bucket.sell_volume_btc)
        .sum::<f64>();
    if total_flow <= f64::EPSILON {
        return None;
    }
    let total_liquidation = liquidation_buckets
        .iter()
        .map(|bucket| bucket.long_liq_btc + bucket.short_liq_btc)
        .sum::<f64>();
    Some((total_liquidation / total_flow).clamp(0.0, 1.0))
}

fn price_change_pct(buckets: &[ContractFlowBucket]) -> Option<f64> {
    let first = buckets.iter().find_map(|bucket| bucket.vwap)?;
    let last = buckets.iter().rev().find_map(|bucket| bucket.vwap)?;
    (first.abs() > f64::EPSILON).then_some(((last - first) / first) * 100.0)
}

fn signal_price_change_pct(signal: &ContractWhaleSignal) -> Option<f64> {
    signal
        .price_move_pct
        .or(signal.price_move_30s_pct)
        .or(signal.price_move_15s_pct)
        .or(signal.price_move_5s_pct)
}

fn volume_spike_multiple(total_volume_btc: f64, duration_sec: f64) -> Option<f64> {
    if total_volume_btc <= f64::EPSILON {
        return None;
    }
    let volume_per_min = total_volume_btc / duration_sec.max(1.0) * 60.0;
    Some((volume_per_min / 500.0).clamp(0.0, 5.0))
}

fn data_quality(
    flow_buckets: &[ContractFlowBucket],
    oi_available: bool,
    funding_available: bool,
    latest_signal: Option<&ContractWhaleSignal>,
) -> f64 {
    let mut quality: f64 = 0.35;
    if !flow_buckets.is_empty() {
        quality += 0.35;
    }
    if oi_available {
        quality += 0.15;
    }
    if funding_available {
        quality += 0.10;
    }
    if let Some(signal) = latest_signal {
        quality = quality.max(signal.data_quality as f64 / 100.0);
    }
    quality.clamp(0.0, 1.0)
}
