use axum::{
    extract::{Query, State},
    Json,
};
use serde::Deserialize;

use crate::{
    altcoin_manipulation_engine::{
        analyze_altcoin_manipulation, AltcoinManipulationInput, AltcoinManipulationState,
    },
    api::new_token_watch_routes::global_new_token_watch_manager,
    app::AppState,
    binance_alt_contract_monitor::config::enable_binance_alt_contract_symbol_for_watch,
    contract_whale_monitor::{
        aggregator::market_context_from_snapshots,
        types::{ContractFlowBucket, ContractWhaleDirection},
    },
    market_domain::{classify_market_domain, MarketDomain},
    market_regime_engine::normalize_market_symbol,
    normalizers::trade::now_ms,
    storage::contract_whale_repo::{ContractWhaleRepo, ContractWhaleSignalQuery},
    toxic_v3::new_token_watch::{
        normalize_symbol as normalize_watch_symbol, TokenFlowRegime, TokenWatchItem,
    },
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
    let raw_symbol = requested_symbol.unwrap_or("ETHUSDT");
    let symbol = normalize_market_symbol(raw_symbol);
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
    let _ = enable_binance_alt_contract_symbol_for_watch(&symbol);

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

    let has_contract_context = latest_signal.is_some()
        || (buy_volume + sell_volume) > f64::EPSILON
        || oi_change_pct.abs() > f64::EPSILON
        || funding_rate.abs() > f64::EPSILON;
    if !has_contract_context {
        if let Some(item) = token_watch_item_for(raw_symbol) {
            return altcoin_state_from_token_watch_item(&item);
        }
    }

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

fn token_watch_item_for(raw_symbol: &str) -> Option<TokenWatchItem> {
    let symbol = normalize_watch_symbol(raw_symbol).ok()?;
    let manager = global_new_token_watch_manager();
    if let Some(item) = manager
        .list_active_tokens()
        .items
        .into_iter()
        .find(|item| item.symbol.eq_ignore_ascii_case(&symbol))
    {
        return Some(item);
    }
    manager.add_token(&symbol).ok()
}

fn altcoin_state_from_token_watch_item(item: &TokenWatchItem) -> AltcoinManipulationState {
    let input = altcoin_input_from_token_watch_item(item);
    let mut state = analyze_altcoin_manipulation(&input);
    let signal = &item.last_signal;
    let compression = &signal.signal_compression;
    let capital = &signal.capital_structure;
    let impact = &signal.impact_response;
    let actor = &signal.actor_decomposition;

    state
        .metrics
        .insert("token_watch_flow_source".to_string(), 1.0);
    state
        .metrics
        .insert("token_flow_strength".to_string(), round4(signal.strength));
    state.metrics.insert(
        "token_flow_confidence".to_string(),
        round4(signal.confidence),
    );
    state.metrics.insert(
        "smart_money_probability".to_string(),
        round4(actor.smart_money_probability),
    );
    state.metrics.insert(
        "absorption_score".to_string(),
        round4(impact.absorption_score),
    );
    state.metrics.insert(
        "flow_persistence".to_string(),
        round4(signal.flow_persistence),
    );
    state.metrics.insert(
        "capital_phase_confidence".to_string(),
        round4(capital.phase_confidence),
    );

    match signal.regime {
        TokenFlowRegime::Accumulation => {
            state.signals.push("TOKEN_ACCUMULATION_FLOW".to_string());
            state
                .risk_tags
                .push("ACCUMULATION_CONTROL_WATCH".to_string());
        }
        TokenFlowRegime::Building => {
            state.signals.push("TOKEN_POSITION_BUILDING".to_string());
            state.risk_tags.push("POSITION_BUILDING_WATCH".to_string());
        }
        TokenFlowRegime::Distribution => {
            state.signals.push("TOKEN_DISTRIBUTION_FLOW".to_string());
            state.risk_tags.push("DISTRIBUTION_RISK".to_string());
        }
        TokenFlowRegime::Neutral => {
            state.signals.push("TOKEN_FLOW_NEUTRAL".to_string());
        }
    }
    if impact.absorption_score >= 0.45 {
        state.signals.push("TOKEN_ABSORPTION_EVIDENCE".to_string());
    }
    if signal.flow_persistence >= 0.35 {
        state.signals.push("TOKEN_FLOW_PERSISTENCE".to_string());
    }
    if compression.liquidity_stress_manipulation >= 0.35 {
        state.risk_tags.push("LIQUIDITY_STRESS_RISK".to_string());
    }
    state.signals.sort();
    state.signals.dedup();
    state.risk_tags.sort();
    state.risk_tags.dedup();
    state
}

fn altcoin_input_from_token_watch_item(item: &TokenWatchItem) -> AltcoinManipulationInput {
    let signal = &item.last_signal;
    let compression = &signal.signal_compression;
    let impact = &signal.impact_response;
    let capital = &signal.capital_structure;

    let mut flow_bias = compression.smart_money_pressure;
    if flow_bias.abs() < 0.05 {
        flow_bias = match signal.regime {
            TokenFlowRegime::Accumulation | TokenFlowRegime::Building => 0.35,
            TokenFlowRegime::Distribution => -0.35,
            TokenFlowRegime::Neutral => 0.0,
        };
    }
    let price_change_pct = impact.price_move_pct * 100.0;
    let oi_change_pct =
        (flow_bias * 1.15 + signal.flow_persistence * flow_bias.signum() * 0.35).clamp(-2.0, 2.0);
    let volume_spike_multiple = (impact.total_volume / 35.0)
        .clamp(0.0, 5.0)
        .max(signal.strength * 4.0)
        .max(0.1);
    let funding_rate = (compression.liquidity_stress_manipulation * 0.001).clamp(-0.001, 0.001);
    let price_impact_efficiency = impact
        .thin_liquidity_score
        .max(impact.impact_per_volume)
        .clamp(0.0, 1.0);
    let data_quality = (0.30
        + signal.confidence * 0.35
        + capital.phase_confidence * 0.20
        + signal.actor_decomposition.confidence * 0.15)
        .clamp(0.20, 0.95);

    AltcoinManipulationInput {
        symbol: normalize_market_symbol(&item.symbol),
        price_change_pct,
        oi_change_pct,
        volume_spike_multiple,
        funding_rate,
        liquidation_ratio: capital.distribution_risk.score,
        price_impact_efficiency,
        flow_bias_score: flow_bias.clamp(-1.0, 1.0),
        data_quality,
    }
}

fn round4(value: f64) -> f64 {
    (value * 10_000.0).round() / 10_000.0
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::toxic_v3::new_token_watch::TokenWatchManager;

    #[test]
    fn token_watch_fallback_populates_altcoin_component_scores() {
        let manager = TokenWatchManager::default();
        let item = manager.add_token("BTWUSDT").expect("valid watch item");

        let state = altcoin_state_from_token_watch_item(&item);

        assert_eq!(state.symbol, "BTW");
        assert!(
            state.oi_signal_score > 0.0
                || state.volume_signal_score > 0.0
                || state.price_signal_score > 0.0
        );
        assert_eq!(state.metrics.get("token_watch_flow_source"), Some(&1.0));
        assert!(state
            .signals
            .iter()
            .any(|signal| signal.starts_with("TOKEN_")));
    }

    #[test]
    fn token_watch_fallback_keeps_btc_out_of_altcoin_manipulation() {
        let manager = TokenWatchManager::default();
        let item = manager.add_token("ASTERUSDT").expect("valid watch item");

        let input = altcoin_input_from_token_watch_item(&item);

        assert_eq!(input.symbol, "ASTER");
        assert!(input.data_quality > 0.0);
    }
}
