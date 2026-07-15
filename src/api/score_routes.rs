use std::collections::BTreeMap;

use axum::{
    extract::{Query, State},
    Json,
};
use serde::{Deserialize, Serialize};

use crate::{
    api::{
        main_force_event_routes,
        toxic_signal_history_routes::ensure_signal_history_snapshot,
        toxic_signal_inbox_routes::{
            build_recent, latest_cwm_signal_for_state, normalize_symbol_query,
            observed_tof_snapshot_for_state,
        },
        toxic_signal_ws_routes::{
            build_ws_snapshot_with_authoritative_state, ToxicSignalWsItem, ToxicSignalWsSnapshot,
        },
    },
    app::AppState,
    runtime::cwm_risk_fusion::{MarketStructureReason, ToxicReason},
    types::toxic_signal_history::ToxicSignalHistorySignalItem,
};

#[derive(Debug, Deserialize, Default)]
pub struct ScoreQuery {
    pub symbol: Option<String>,
    pub limit: Option<String>,
}

#[derive(Debug, Clone, Serialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct ToxicShortComponents {
    pub toxic_order_cluster: Option<f64>,
    pub aggressive_sweep: Option<f64>,
    pub orderbook_deformation: Option<f64>,
    pub spoof_cancel: Option<f64>,
    pub adverse_move: Option<f64>,
    pub liquidity_gap: Option<f64>,
    pub micro_volatility_shock: Option<f64>,
}

#[derive(Debug, Clone, Serialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct MarketStructureComponents {
    pub spot_score: u8,
    pub contract_score: u8,
    pub cross_confirm_score: u8,
    pub cwm_score: u8,
    pub oi_score: u8,
    pub liquidation_score: u8,
    pub funding_crowding_score: u8,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ToxicShortScoreItem {
    pub signal_id: String,
    pub symbol: String,
    pub detector: String,
    pub direction: String,
    pub created_at: String,
    pub final_result: String,
    pub core_reason: String,
    pub severity: String,
    pub toxic_score: u8,
    pub short_pressure: i16,
    pub confidence: f64,
    pub data_quality: f64,
    pub toxic_type: String,
    pub ttl_sec: u64,
    pub expires_at: i64,
    pub half_life_sec: u64,
    pub max_ttl_sec: u64,
    pub decayed_score: f64,
    pub formula: String,
    pub discord_gate: String,
    pub components: ToxicShortComponents,
    pub reasons: Vec<ToxicReason>,
    pub alert_status: String,
    pub alert_reason: String,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ToxicShortSummaryResponse {
    pub read_only: bool,
    pub runtime_modified: bool,
    pub analysis_only: bool,
    pub execution_enabled: bool,
    pub symbol: String,
    pub available: bool,
    pub toxic_score: Option<u8>,
    pub short_pressure: Option<i16>,
    pub severity: Option<String>,
    pub toxic_type: Option<String>,
    pub confidence: Option<f64>,
    pub data_quality: Option<f64>,
    pub ttl_sec: Option<u64>,
    pub expires_at: Option<i64>,
    pub half_life_sec: Option<u64>,
    pub max_ttl_sec: Option<u64>,
    pub decayed_score: Option<f64>,
    pub formula: Option<String>,
    pub discord_gate: Option<String>,
    pub final_result: Option<String>,
    pub core_reason: Option<String>,
    pub components: ToxicShortComponents,
    pub reasons: Vec<ToxicReason>,
    pub filter: BTreeMap<String, String>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ToxicShortListResponse {
    pub read_only: bool,
    pub runtime_modified: bool,
    pub analysis_only: bool,
    pub execution_enabled: bool,
    pub selected_symbol: String,
    pub items: Vec<ToxicShortScoreItem>,
    pub filter: BTreeMap<String, String>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ToxicShortHistoryItem {
    pub signal_id: String,
    pub symbol: String,
    pub detector: String,
    pub direction: String,
    pub severity: String,
    pub confidence: f64,
    pub created_at_ms: u64,
    pub history_recorded_at_ms: u64,
    pub source: String,
    pub operator_action: String,
    pub toxic_score: Option<u8>,
    pub short_pressure: Option<i16>,
    pub data_quality: Option<f64>,
    pub toxic_type: Option<String>,
    pub ttl_sec: Option<u64>,
    pub expires_at: Option<i64>,
    pub decayed_score: Option<f64>,
    pub components: ToxicShortComponents,
    pub reasons: Vec<ToxicReason>,
    pub current_snapshot_available: bool,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ToxicShortHistoryResponse {
    pub read_only: bool,
    pub runtime_modified: bool,
    pub analysis_only: bool,
    pub execution_enabled: bool,
    pub selected_symbol: String,
    pub items: Vec<ToxicShortHistoryItem>,
    pub filter: BTreeMap<String, String>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct MarketStructureScoreItem {
    pub signal_id: String,
    pub symbol: String,
    pub detector: String,
    pub direction: String,
    pub created_at: String,
    pub final_result: String,
    pub core_reason: String,
    pub severity: String,
    pub regime_type: String,
    pub main_force_score: u8,
    pub extreme_impact_score: u8,
    pub structure_bias: i16,
    pub confidence: f64,
    pub data_quality: f64,
    pub main_force_confirmed: bool,
    pub extreme_impact_confirmed: bool,
    pub liquidation_driven: bool,
    pub components: MarketStructureComponents,
    pub reasons: Vec<MarketStructureReason>,
    pub alert_status: String,
    pub alert_reason: String,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct MarketStructureSummaryResponse {
    pub read_only: bool,
    pub runtime_modified: bool,
    pub analysis_only: bool,
    pub execution_enabled: bool,
    pub symbol: String,
    pub available: bool,
    pub main_force_score: Option<u8>,
    pub extreme_impact_score: Option<u8>,
    pub structure_bias: Option<i16>,
    pub severity: Option<String>,
    pub regime_type: Option<String>,
    pub confidence: Option<f64>,
    pub data_quality: Option<f64>,
    pub main_force_confirmed: Option<bool>,
    pub extreme_impact_confirmed: Option<bool>,
    pub liquidation_driven: Option<bool>,
    pub components: Option<MarketStructureComponents>,
    pub reasons: Vec<MarketStructureReason>,
    pub final_result: Option<String>,
    pub core_reason: Option<String>,
    pub filter: BTreeMap<String, String>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct MarketStructureListResponse {
    pub read_only: bool,
    pub runtime_modified: bool,
    pub analysis_only: bool,
    pub execution_enabled: bool,
    pub selected_symbol: String,
    pub items: Vec<MarketStructureScoreItem>,
    pub filter: BTreeMap<String, String>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct MarketStructureHistoryItem {
    pub signal_id: String,
    pub symbol: String,
    pub detector: String,
    pub direction: String,
    pub severity: String,
    pub confidence: f64,
    pub created_at_ms: u64,
    pub history_recorded_at_ms: u64,
    pub source: String,
    pub operator_action: String,
    pub regime_type: Option<String>,
    pub main_force_score: Option<u8>,
    pub extreme_impact_score: Option<u8>,
    pub structure_bias: Option<i16>,
    pub data_quality: Option<f64>,
    pub main_force_confirmed: Option<bool>,
    pub extreme_impact_confirmed: Option<bool>,
    pub liquidation_driven: Option<bool>,
    pub components: Option<MarketStructureComponents>,
    pub reasons: Vec<MarketStructureReason>,
    pub current_snapshot_available: bool,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct MarketStructureHistoryResponse {
    pub read_only: bool,
    pub runtime_modified: bool,
    pub analysis_only: bool,
    pub execution_enabled: bool,
    pub selected_symbol: String,
    pub items: Vec<MarketStructureHistoryItem>,
    pub filter: BTreeMap<String, String>,
}

pub async fn toxic_short_summary_route(
    State(state): State<AppState>,
    Query(query): Query<ScoreQuery>,
) -> Json<serde_json::Value> {
    let symbol = normalize_symbol_query(query.symbol, &state.config().symbol);
    let snapshot = build_score_snapshot(&state, &symbol);
    Json(serde_json::json!(toxic_short_summary_from_snapshot(
        &symbol, &snapshot
    )))
}

pub async fn toxic_short_latest_route(
    State(state): State<AppState>,
    Query(query): Query<ScoreQuery>,
) -> Json<serde_json::Value> {
    let symbol = normalize_symbol_query(query.symbol, &state.config().symbol);
    let limit = parse_limit(query.limit.as_deref(), 50, 200);
    let snapshot = build_score_snapshot(&state, &symbol);
    let items = snapshot
        .signals
        .iter()
        .take(limit)
        .map(project_toxic_short_item)
        .collect();
    Json(serde_json::json!(ToxicShortListResponse {
        read_only: true,
        runtime_modified: false,
        analysis_only: true,
        execution_enabled: false,
        selected_symbol: symbol.clone(),
        items,
        filter: score_filter(&symbol, limit),
    }))
}

pub async fn toxic_short_history_route(
    State(state): State<AppState>,
    Query(query): Query<ScoreQuery>,
) -> Json<serde_json::Value> {
    let symbol = normalize_symbol_query(query.symbol, &state.config().symbol);
    let limit = parse_limit(query.limit.as_deref(), 50, 200);
    ensure_signal_history_snapshot(&state, &symbol);
    let snapshot = build_score_snapshot(&state, &symbol);
    let lookup = signal_lookup(&snapshot);
    let history = state.signal_history_service().recent(&symbol);
    let items = history
        .items
        .iter()
        .take(limit)
        .map(|item| project_toxic_short_history_item(item, lookup.get(&item.signal_id)))
        .collect();
    Json(serde_json::json!(ToxicShortHistoryResponse {
        read_only: true,
        runtime_modified: false,
        analysis_only: true,
        execution_enabled: false,
        selected_symbol: symbol.clone(),
        items,
        filter: score_filter(&symbol, limit),
    }))
}

pub async fn market_structure_summary_route(
    State(state): State<AppState>,
    Query(query): Query<ScoreQuery>,
) -> Json<serde_json::Value> {
    let symbol = normalize_symbol_query(query.symbol, &state.config().symbol);
    let snapshot = build_score_snapshot(&state, &symbol);
    Json(serde_json::json!(market_structure_summary_from_snapshot(
        &symbol, &snapshot
    )))
}

pub async fn market_structure_latest_route(
    State(state): State<AppState>,
    Query(query): Query<ScoreQuery>,
) -> Json<serde_json::Value> {
    let symbol = normalize_symbol_query(query.symbol, &state.config().symbol);
    let limit = parse_limit(query.limit.as_deref(), 50, 200);
    let snapshot = build_score_snapshot(&state, &symbol);
    let items = snapshot
        .signals
        .iter()
        .filter_map(project_market_structure_item)
        .take(limit)
        .collect();
    Json(serde_json::json!(MarketStructureListResponse {
        read_only: true,
        runtime_modified: false,
        analysis_only: true,
        execution_enabled: false,
        selected_symbol: symbol.clone(),
        items,
        filter: score_filter(&symbol, limit),
    }))
}

pub async fn market_structure_history_route(
    State(state): State<AppState>,
    Query(query): Query<ScoreQuery>,
) -> Json<serde_json::Value> {
    let symbol = normalize_symbol_query(query.symbol, &state.config().symbol);
    let limit = parse_limit(query.limit.as_deref(), 50, 200);
    ensure_signal_history_snapshot(&state, &symbol);
    let snapshot = build_score_snapshot(&state, &symbol);
    let lookup = signal_lookup(&snapshot);
    let history = state.signal_history_service().recent(&symbol);
    let items = history
        .items
        .iter()
        .take(limit)
        .map(|item| project_market_structure_history_item(item, lookup.get(&item.signal_id)))
        .collect();
    Json(serde_json::json!(MarketStructureHistoryResponse {
        read_only: true,
        runtime_modified: false,
        analysis_only: true,
        execution_enabled: false,
        selected_symbol: symbol.clone(),
        items,
        filter: score_filter(&symbol, limit),
    }))
}

pub use main_force_event_routes::main_force_events_route as market_structure_events_route;

fn build_score_snapshot(state: &AppState, symbol: &str) -> ToxicSignalWsSnapshot {
    let recent = build_recent(state, symbol);
    let cwm_signal = latest_cwm_signal_for_state(state, symbol);
    let tof_snapshot = observed_tof_snapshot_for_state(state, symbol);
    build_ws_snapshot_with_authoritative_state(
        &recent,
        cwm_signal.as_ref(),
        tof_snapshot.as_ref(),
        state.runtime_started(),
    )
}

fn best_toxic_short_signal(snapshot: &ToxicSignalWsSnapshot) -> Option<&ToxicSignalWsItem> {
    snapshot.signals.iter().max_by(|left, right| {
        left.toxic_score
            .cmp(&right.toxic_score)
            .then_with(|| {
                left.toxic_decayed_score
                    .partial_cmp(&right.toxic_decayed_score)
                    .unwrap_or(std::cmp::Ordering::Equal)
            })
            .then_with(|| left.confidence.total_cmp(&right.confidence))
            .then_with(|| {
                left.data_quality
                    .unwrap_or(f64::NEG_INFINITY)
                    .total_cmp(&right.data_quality.unwrap_or(f64::NEG_INFINITY))
            })
            .then_with(|| left.created_at.cmp(&right.created_at))
    })
}

fn best_market_structure_signal(snapshot: &ToxicSignalWsSnapshot) -> Option<&ToxicSignalWsItem> {
    snapshot
        .signals
        .iter()
        .filter(|signal| signal.market_structure_score.is_some())
        .max_by(|left, right| {
            left.main_force_score
                .cmp(&right.main_force_score)
                .then_with(|| left.extreme_impact_score.cmp(&right.extreme_impact_score))
                .then_with(|| {
                    left.market_structure_confidence
                        .unwrap_or(f64::NEG_INFINITY)
                        .total_cmp(
                            &right
                                .market_structure_confidence
                                .unwrap_or(f64::NEG_INFINITY),
                        )
                })
                .then_with(|| left.created_at.cmp(&right.created_at))
        })
}

fn toxic_short_summary_from_snapshot(
    symbol: &str,
    snapshot: &ToxicSignalWsSnapshot,
) -> ToxicShortSummaryResponse {
    let item = best_toxic_short_signal(snapshot).map(project_toxic_short_item);
    ToxicShortSummaryResponse {
        read_only: true,
        runtime_modified: false,
        analysis_only: true,
        execution_enabled: false,
        symbol: symbol.to_string(),
        available: item.is_some(),
        toxic_score: item.as_ref().map(|value| value.toxic_score),
        short_pressure: item.as_ref().map(|value| value.short_pressure),
        severity: item.as_ref().map(|value| value.severity.clone()),
        toxic_type: item.as_ref().map(|value| value.toxic_type.clone()),
        confidence: item.as_ref().map(|value| value.confidence),
        data_quality: item.as_ref().map(|value| value.data_quality),
        ttl_sec: item.as_ref().map(|value| value.ttl_sec),
        expires_at: item.as_ref().map(|value| value.expires_at),
        half_life_sec: item.as_ref().map(|value| value.half_life_sec),
        max_ttl_sec: item.as_ref().map(|value| value.max_ttl_sec),
        decayed_score: item.as_ref().map(|value| value.decayed_score),
        formula: item.as_ref().map(|value| value.formula.clone()),
        discord_gate: item.as_ref().map(|value| value.discord_gate.clone()),
        final_result: item.as_ref().map(|value| value.final_result.clone()),
        core_reason: item.as_ref().map(|value| value.core_reason.clone()),
        components: item
            .as_ref()
            .map(|value| value.components.clone())
            .unwrap_or_default(),
        reasons: item.map(|value| value.reasons).unwrap_or_default(),
        filter: score_filter(symbol, snapshot.signals.len().max(1)),
    }
}

fn market_structure_summary_from_snapshot(
    symbol: &str,
    snapshot: &ToxicSignalWsSnapshot,
) -> MarketStructureSummaryResponse {
    let item = best_market_structure_signal(snapshot).and_then(project_market_structure_item);
    MarketStructureSummaryResponse {
        read_only: true,
        runtime_modified: false,
        analysis_only: true,
        execution_enabled: false,
        symbol: symbol.to_string(),
        available: item.is_some(),
        main_force_score: item.as_ref().map(|value| value.main_force_score),
        extreme_impact_score: item.as_ref().map(|value| value.extreme_impact_score),
        structure_bias: item.as_ref().map(|value| value.structure_bias),
        severity: item.as_ref().map(|value| value.severity.clone()),
        regime_type: item.as_ref().map(|value| value.regime_type.clone()),
        confidence: item.as_ref().map(|value| value.confidence),
        data_quality: item.as_ref().map(|value| value.data_quality),
        main_force_confirmed: item.as_ref().map(|value| value.main_force_confirmed),
        extreme_impact_confirmed: item.as_ref().map(|value| value.extreme_impact_confirmed),
        liquidation_driven: item.as_ref().map(|value| value.liquidation_driven),
        components: item.as_ref().map(|value| value.components.clone()),
        reasons: item.map(|value| value.reasons).unwrap_or_default(),
        final_result: best_market_structure_signal(snapshot)
            .map(|value| value.final_result.clone()),
        core_reason: best_market_structure_signal(snapshot).map(|value| value.core_reason.clone()),
        filter: score_filter(symbol, snapshot.signals.len().max(1)),
    }
}

fn project_toxic_short_item(signal: &ToxicSignalWsItem) -> ToxicShortScoreItem {
    ToxicShortScoreItem {
        signal_id: signal.id.clone(),
        symbol: signal.symbol.clone(),
        detector: signal.detector.clone(),
        direction: signal.direction.clone(),
        created_at: signal.created_at.clone(),
        final_result: signal.final_result.clone(),
        core_reason: signal.core_reason.clone(),
        severity: signal.toxic_severity.clone(),
        toxic_score: signal.toxic_score,
        short_pressure: signal.short_pressure,
        confidence: signal.toxic_short_score.confidence,
        data_quality: signal.toxic_short_score.data_quality,
        toxic_type: signal.toxic_type.clone(),
        ttl_sec: signal.toxic_ttl_sec,
        expires_at: signal.toxic_expires_at,
        half_life_sec: signal.toxic_half_life_sec,
        max_ttl_sec: signal.toxic_max_ttl_sec,
        decayed_score: signal.toxic_decayed_score,
        formula: signal.toxic_decay_formula.clone(),
        discord_gate: signal.toxic_short_score.discord_gate.clone(),
        components: toxic_short_components(signal),
        reasons: signal.toxic_reasons.clone(),
        alert_status: signal.alert_status.clone(),
        alert_reason: signal.alert_reason.clone(),
    }
}

fn project_market_structure_item(signal: &ToxicSignalWsItem) -> Option<MarketStructureScoreItem> {
    let score = signal.market_structure_score.as_ref()?;
    Some(MarketStructureScoreItem {
        signal_id: signal.id.clone(),
        symbol: signal.symbol.clone(),
        detector: signal.detector.clone(),
        direction: signal.direction.clone(),
        created_at: signal.created_at.clone(),
        final_result: signal.final_result.clone(),
        core_reason: signal.core_reason.clone(),
        severity: score.severity.clone(),
        regime_type: score.regime_type.clone(),
        main_force_score: score.main_force_score,
        extreme_impact_score: score.extreme_impact_score,
        structure_bias: score.structure_bias,
        confidence: score.confidence,
        data_quality: score.data_quality,
        main_force_confirmed: score.main_force_confirmed,
        extreme_impact_confirmed: score.extreme_impact_confirmed,
        liquidation_driven: signal.cwm_contribution.liquidation_suspected == Some(true),
        components: market_structure_components_from_score(score),
        reasons: score.reasons.clone(),
        alert_status: signal.alert_status.clone(),
        alert_reason: signal.alert_reason.clone(),
    })
}

fn project_toxic_short_history_item(
    item: &ToxicSignalHistorySignalItem,
    signal: Option<&&ToxicSignalWsItem>,
) -> ToxicShortHistoryItem {
    let current = signal.copied();
    ToxicShortHistoryItem {
        signal_id: item.signal_id.clone(),
        symbol: item.symbol.clone(),
        detector: item.signal_kind.clone(),
        direction: item.direction_bias.clone(),
        severity: item.severity.clone(),
        confidence: item.confidence,
        created_at_ms: item.created_at_ms,
        history_recorded_at_ms: item.history_recorded_at_ms,
        source: item.source.clone(),
        operator_action: item.operator_action.clone(),
        toxic_score: current.map(|value| value.toxic_score),
        short_pressure: current.map(|value| value.short_pressure),
        data_quality: current.map(|value| value.toxic_short_score.data_quality),
        toxic_type: current.map(|value| value.toxic_type.clone()),
        ttl_sec: current.map(|value| value.toxic_ttl_sec),
        expires_at: current.map(|value| value.toxic_expires_at),
        decayed_score: current.map(|value| value.toxic_decayed_score),
        components: current.map(toxic_short_components).unwrap_or_default(),
        reasons: current
            .map(|value| value.toxic_reasons.clone())
            .unwrap_or_default(),
        current_snapshot_available: current.is_some(),
    }
}

fn project_market_structure_history_item(
    item: &ToxicSignalHistorySignalItem,
    signal: Option<&&ToxicSignalWsItem>,
) -> MarketStructureHistoryItem {
    let current = signal
        .copied()
        .filter(|value| value.market_structure_score.is_some());
    MarketStructureHistoryItem {
        signal_id: item.signal_id.clone(),
        symbol: item.symbol.clone(),
        detector: item.signal_kind.clone(),
        direction: item.direction_bias.clone(),
        severity: item.severity.clone(),
        confidence: item.confidence,
        created_at_ms: item.created_at_ms,
        history_recorded_at_ms: item.history_recorded_at_ms,
        source: item.source.clone(),
        operator_action: item.operator_action.clone(),
        regime_type: current.and_then(|value| value.regime_type.clone()),
        main_force_score: current.and_then(|value| value.main_force_score),
        extreme_impact_score: current.and_then(|value| value.extreme_impact_score),
        structure_bias: current.and_then(|value| value.structure_bias),
        data_quality: current.and_then(|value| value.market_structure_data_quality),
        main_force_confirmed: current.and_then(|value| value.main_force_confirmed),
        extreme_impact_confirmed: current.and_then(|value| value.extreme_impact_confirmed),
        liquidation_driven: current
            .map(|value| value.cwm_contribution.liquidation_suspected == Some(true)),
        components: current.and_then(market_structure_components),
        reasons: current
            .and_then(|value| value.market_structure_score.as_ref())
            .map(|score| score.reasons.clone())
            .unwrap_or_default(),
        current_snapshot_available: current.is_some(),
    }
}

fn signal_lookup(snapshot: &ToxicSignalWsSnapshot) -> BTreeMap<String, &ToxicSignalWsItem> {
    snapshot
        .signals
        .iter()
        .map(|signal| (signal.id.clone(), signal))
        .collect()
}

fn toxic_short_components(signal: &ToxicSignalWsItem) -> ToxicShortComponents {
    ToxicShortComponents {
        toxic_order_cluster: toxic_reason_score(signal, "ToxicOrderCluster"),
        aggressive_sweep: toxic_reason_score(signal, "AggressiveSweep"),
        orderbook_deformation: toxic_reason_score(signal, "OrderbookDeformation"),
        spoof_cancel: toxic_reason_score(signal, "SpoofCancel"),
        adverse_move: toxic_reason_score(signal, "AdverseMove"),
        liquidity_gap: toxic_reason_score(signal, "LiquidityGap"),
        micro_volatility_shock: toxic_reason_score(signal, "MicroVolatilityShock"),
    }
}

fn market_structure_components(signal: &ToxicSignalWsItem) -> Option<MarketStructureComponents> {
    signal
        .market_structure_score
        .as_ref()
        .map(market_structure_components_from_score)
}

fn market_structure_components_from_score(
    score: &crate::runtime::cwm_risk_fusion::MainForceStructureRisk,
) -> MarketStructureComponents {
    MarketStructureComponents {
        spot_score: score.spot_score,
        contract_score: score.contract_score,
        cross_confirm_score: score.cross_confirm_score,
        cwm_score: score.cwm_score,
        oi_score: score.oi_score,
        liquidation_score: score.liquidation_score,
        funding_crowding_score: score.funding_crowding_score,
    }
}

fn toxic_reason_score(signal: &ToxicSignalWsItem, reason_type: &str) -> Option<f64> {
    signal
        .toxic_reasons
        .iter()
        .find(|reason| reason.reason_type == reason_type)
        .map(|reason| reason.score)
}

fn score_filter(symbol: &str, limit: usize) -> BTreeMap<String, String> {
    let mut filter = BTreeMap::new();
    filter.insert("symbol".to_string(), symbol.to_string());
    filter.insert("limit".to_string(), limit.to_string());
    filter.insert("viewOnly".to_string(), "true".to_string());
    filter
}

fn parse_limit(value: Option<&str>, default: usize, max: usize) -> usize {
    value
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .and_then(|value| value.parse::<usize>().ok())
        .map(|value| value.clamp(1, max))
        .unwrap_or(default)
}

#[cfg(test)]
mod tests {
    use super::{
        best_market_structure_signal, best_toxic_short_signal,
        market_structure_summary_from_snapshot, project_market_structure_history_item,
        project_toxic_short_history_item, toxic_short_summary_from_snapshot,
    };
    use crate::{
        api::toxic_signal_ws_routes::build_ws_snapshot,
        types::toxic_signal_history::ToxicSignalHistorySignalItem,
        types::toxic_signal_inbox::{
            ToxicSignalInboxFusionSummary, ToxicSignalInboxGovernanceSummary, ToxicSignalInboxItem,
            ToxicSignalInboxMarkoutSummary, ToxicSignalInboxOperatorAction,
            ToxicSignalInboxQualitySummary, ToxicSignalInboxRecentResponse,
            ToxicSignalInboxRecommendationSummary, ToxicSignalInboxReplaySummary,
        },
    };

    #[test]
    fn toxic_short_summary_exposes_split_short_score_fields() {
        let snapshot = build_ws_snapshot(&sample_recent());
        let summary = toxic_short_summary_from_snapshot("BTC", &snapshot);

        assert!(summary.available);
        assert_eq!(summary.symbol, "BTC");
        assert!(summary.toxic_score.unwrap_or_default() >= 60);
        assert!(summary.short_pressure.unwrap_or_default() < 0);
        assert!(summary.severity.is_some());
        assert!(summary.components.aggressive_sweep.is_none());
    }

    #[test]
    fn market_structure_summary_is_unavailable_without_authoritative_evidence() {
        let snapshot = build_ws_snapshot(&sample_recent());
        let summary = market_structure_summary_from_snapshot("BTC", &snapshot);

        assert!(!summary.available);
        assert!(summary.main_force_score.is_none());
        assert!(summary.extreme_impact_score.is_none());
        assert!(summary.structure_bias.is_none());
        assert!(summary.regime_type.is_none());
        assert!(summary.components.is_none());
    }

    #[test]
    fn score_summary_selectors_choose_highest_signal_for_each_system() {
        let snapshot = build_ws_snapshot(&sample_recent());
        let short = best_toxic_short_signal(&snapshot).expect("short signal");

        assert_eq!(short.id, "sig_scores_high");
        assert!(best_market_structure_signal(&snapshot).is_none());
    }

    #[test]
    fn history_keeps_detector_current_but_filters_unavailable_market_structure() {
        let snapshot = build_ws_snapshot(&sample_recent());
        let signal = snapshot.signals.first().expect("snapshot signal");
        let history = sample_history(&signal.id);

        let toxic = project_toxic_short_history_item(&history, Some(&signal));
        assert!(toxic.current_snapshot_available);
        assert_eq!(toxic.toxic_score, Some(signal.toxic_score));
        assert_eq!(toxic.short_pressure, Some(signal.short_pressure));

        let market = project_market_structure_history_item(&history, Some(&signal));
        assert!(!market.current_snapshot_available);
        assert!(market.main_force_score.is_none());
        assert!(market.components.is_none());
        assert!(market.liquidation_driven.is_none());
    }

    fn sample_recent() -> ToxicSignalInboxRecentResponse {
        ToxicSignalInboxRecentResponse {
            read_only: true,
            runtime_modified: false,
            analysis_only: true,
            execution_enabled: false,
            manual_review_required: true,
            runtime_weight_modified: false,
            config_modified: false,
            mode: "analysis_only".to_string(),
            selected_symbol: "BTC".to_string(),
            status: "signal_inbox_ready".to_string(),
            warnings: vec![],
            items: vec![
                sample_item(
                    "sig_scores_high",
                    "high",
                    0.92,
                    "ask/sell",
                    "large ask wall removed",
                ),
                sample_item(
                    "sig_scores_medium",
                    "medium",
                    0.68,
                    "bid/buy",
                    "market bounced after thin book",
                ),
            ],
        }
    }

    fn sample_item(
        signal_id: &str,
        severity: &str,
        confidence: f64,
        direction_bias: &str,
        summary: &str,
    ) -> ToxicSignalInboxItem {
        ToxicSignalInboxItem {
            signal_id: signal_id.to_string(),
            symbol: "BTC".to_string(),
            signal_kind: "spoofing_candidate".to_string(),
            direction_bias: direction_bias.to_string(),
            severity: severity.to_string(),
            risk_score: 82,
            data_quality_score: Some(82.0),
            confidence,
            created_at_ms: 1_700_000_000_000,
            fusion: ToxicSignalInboxFusionSummary {
                available: true,
                summary: summary.to_string(),
            },
            replay: ToxicSignalInboxReplaySummary {
                available: true,
                evidence_count: 3,
            },
            markout: ToxicSignalInboxMarkoutSummary {
                available: true,
                one_minute: "adverse".to_string(),
                five_minute: "adverse".to_string(),
                fifteen_minute: "aligned".to_string(),
                one_hour: "aligned".to_string(),
            },
            quality: ToxicSignalInboxQualitySummary {
                available: true,
                quality_bucket: "excellent".to_string(),
                aligned_ratio: 0.82,
                adverse_ratio: 0.18,
            },
            recommendation: ToxicSignalInboxRecommendationSummary {
                available: true,
                action: "review_evidence".to_string(),
                no_trade_only: false,
                manual_review_required: true,
            },
            governance: ToxicSignalInboxGovernanceSummary {
                ledger_available: false,
                latest_decision: "missing_ledger_evidence".to_string(),
            },
            operator_action: ToxicSignalInboxOperatorAction::ReviewEvidence,
            read_only: true,
            runtime_modified: false,
            analysis_only: true,
            execution_enabled: false,
        }
    }

    fn sample_history(signal_id: &str) -> ToxicSignalHistorySignalItem {
        ToxicSignalHistorySignalItem {
            signal_id: signal_id.to_string(),
            symbol: "BTC".to_string(),
            signal_kind: "spoofing_candidate".to_string(),
            direction_bias: "ask/sell".to_string(),
            severity: "high".to_string(),
            confidence: 0.92,
            created_at_ms: 1_700_000_000_000,
            markout_one_minute: "adverse".to_string(),
            markout_five_minute: "adverse".to_string(),
            markout_fifteen_minute: "aligned".to_string(),
            markout_one_hour: "aligned".to_string(),
            quality_bucket: "excellent".to_string(),
            recommendation_action: "review_evidence".to_string(),
            no_trade_only: false,
            source: "test".to_string(),
            history_recorded_at_ms: 1_700_000_000_100,
            operator_action: "review_evidence".to_string(),
        }
    }
}
