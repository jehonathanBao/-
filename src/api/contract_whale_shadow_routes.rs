use std::collections::BTreeMap;

use axum::{
    extract::{Query, State},
    Json,
};
use serde::{Deserialize, Serialize};

use crate::{
    app::AppState,
    contract_whale_monitor::{
        config::contract_whale_runtime_config,
        shadow::{ShadowCandidate, ShadowObservation, ShadowState, ShadowTracker},
        types::{ContractFlowBucket, ContractLiquidationBucket},
    },
    market_regime_engine::normalize_market_symbol,
    normalizers::trade::now_ms,
    storage::contract_whale_repo::ContractWhaleRepo,
};

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ShadowQuery {
    pub symbol: Option<String>,
    pub lookback_min: Option<u64>,
    pub limit: Option<usize>,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ShadowResponse {
    pub lane: &'static str,
    pub read_only: bool,
    pub analysis_only: bool,
    pub execution_enabled: bool,
    pub data_state: &'static str,
    pub lookback_min: u64,
    pub items: Vec<ShadowCandidate>,
}

pub async fn contract_whale_shadows_route(
    State(state): State<AppState>,
    Query(query): Query<ShadowQuery>,
) -> Json<ShadowResponse> {
    let symbol = query
        .symbol
        .as_deref()
        .map(normalize_market_symbol)
        .unwrap_or_else(|| normalize_market_symbol(&state.config().symbol));
    let lookback_min = query.lookback_min.unwrap_or(60).clamp(15, 60);
    let limit = query.limit.unwrap_or(50).clamp(1, 100);
    let now = now_ms();
    let from = now.saturating_sub(lookback_min as i64 * 60_000);
    let items = state
        .contract_whale_store()
        .and_then(|store| {
            let flow = store
                .list_contract_flow_buckets_between(&symbol, from, now)
                .ok()?;
            let liquidations = store
                .list_contract_liquidation_buckets_between(&symbol, from, now)
                .unwrap_or_default();
            let oi = store
                .list_contract_oi_snapshots_between(&symbol, from, now)
                .unwrap_or_default();
            Some(build_shadow_candidates(
                &symbol,
                flow,
                liquidations,
                oi,
                limit,
            ))
        })
        .unwrap_or_default();
    Json(ShadowResponse {
        lane: "shadow",
        read_only: true,
        analysis_only: true,
        execution_enabled: false,
        data_state: if items.is_empty() {
            "empty"
        } else {
            "available"
        },
        lookback_min,
        items,
    })
}

#[derive(Default)]
struct WindowAccumulator {
    symbol: String,
    ts: i64,
    total_volume_btc: f64,
    net_volume_btc: f64,
    trade_count: u64,
    exchanges: std::collections::BTreeSet<String>,
    first_vwap: Option<f64>,
    last_vwap: Option<f64>,
    liquidation_btc: f64,
}

pub fn build_shadow_candidates(
    symbol: &str,
    flow: Vec<ContractFlowBucket>,
    liquidations: Vec<ContractLiquidationBucket>,
    oi: Vec<crate::contract_whale_monitor::types::ContractOiSnapshot>,
    limit: usize,
) -> Vec<ShadowCandidate> {
    let mut windows: BTreeMap<i64, WindowAccumulator> = BTreeMap::new();
    for bucket in flow {
        let slot = bucket.ts_bucket.div_euclid(60_000);
        let window = windows.entry(slot).or_insert_with(|| WindowAccumulator {
            symbol: symbol.to_string(),
            ts: slot * 60_000,
            ..WindowAccumulator::default()
        });
        let volume = bucket.buy_volume_btc.max(0.0) + bucket.sell_volume_btc.max(0.0);
        window.total_volume_btc += volume;
        window.net_volume_btc += bucket.buy_volume_btc - bucket.sell_volume_btc;
        window.trade_count = window.trade_count.saturating_add(bucket.trade_count);
        window.exchanges.insert(bucket.exchange);
        if bucket.vwap.is_some() && window.first_vwap.is_none() {
            window.first_vwap = bucket.vwap;
        }
        if bucket.vwap.is_some() {
            window.last_vwap = bucket.vwap;
        }
    }
    for bucket in liquidations {
        let slot = bucket.ts_bucket.div_euclid(60_000);
        if let Some(window) = windows.get_mut(&slot) {
            window.liquidation_btc += bucket.long_liq_btc.max(0.0) + bucket.short_liq_btc.max(0.0);
        }
    }
    let oi_change_pct = oi.first().and_then(|first| {
        oi.last().and_then(|last| {
            if first.oi_btc.abs() > f64::EPSILON {
                Some((last.oi_btc - first.oi_btc) / first.oi_btc * 100.0)
            } else {
                None
            }
        })
    });
    let high_threshold = contract_whale_runtime_config()
        .thresholds_for_symbol_window(symbol, 60)
        .high_btc;
    let mut tracker = ShadowTracker::default();
    let mut candidates = Vec::new();
    for window in windows.into_values() {
        if window.total_volume_btc <= 0.0 {
            continue;
        }
        let price_move_pct = match (window.first_vwap, window.last_vwap) {
            (Some(first), Some(last)) if first.abs() > f64::EPSILON => {
                Some((last - first) / first * 100.0)
            }
            _ => None,
        };
        let candidate = tracker.observe(ShadowObservation {
            symbol: window.symbol,
            ts: window.ts,
            total_volume_btc: window.total_volume_btc,
            net_volume_btc: window.net_volume_btc,
            high_threshold_btc: high_threshold,
            price_move_pct,
            oi_change_pct,
            data_quality: 85,
            multi_exchange_confirmed: window.exchanges.len() >= 2,
            live_liquidation_btc: window.liquidation_btc,
            trade_count: window.trade_count,
        });
        if candidate.state != ShadowState::Invalidated {
            candidates.push(candidate);
        }
    }
    candidates.sort_by_key(|candidate| std::cmp::Reverse(candidate.last_seen_ts));
    candidates.truncate(limit);
    candidates
}
