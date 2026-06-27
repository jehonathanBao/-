use std::collections::BTreeMap;

use axum::{
    extract::{Query, State},
    http::{header, HeaderMap, StatusCode},
    response::IntoResponse,
    Json,
};
use base64::{engine::general_purpose::STANDARD as BASE64_STANDARD, Engine as _};
use serde::Deserialize;

use crate::{
    api::contract_event_routes::{contract_event_page_for_query, final_events_v2_for_query},
    app::AppState,
    config::AppConfig,
    contract_whale_monitor::{
        aggregator::{
            compute_percentile_threshold, dynamic_multiple_for_volume,
            historical_window_average_btc_with_min_samples, liquidation_context_for_window,
            market_context_from_snapshots, percentile_level_for_volume,
            rolling_window_stats_with_config, RollingWindowStatsOptions,
        },
        cluster::apply_contract_whale_signal_clusters,
        config::contract_whale_runtime_config,
        detector::{inspect_contract_whale_signal_with_config, ContractWhaleDetectorRejectReason},
        event_lifecycle::apply_contract_whale_event_lifecycle,
        event_quality::{
            apply_contract_whale_event_quality_filter, decorate_contract_whale_event_quality,
        },
        intelligence,
        log_events,
        merge::merge_contract_whale_signals,
        trading,
        trajectory::apply_contract_whale_trajectories,
        types::{
            ContractFlowBucket, ContractWhaleDirection, ContractWhaleDiscordDryRunStats,
            ContractWhaleExchangeStatus, ContractWhaleLatestResponse,
            ContractWhaleIntelligenceResponse,
            ContractWhaleLiquidationContext, ContractWhaleMarketCapability,
            ContractWhaleMarketContext, ContractWhaleMarketStructureLite, ContractWhaleMarketType,
            ContractWhaleNoiseSuppressionSummary, ContractWhalePercentileThreshold,
            ContractWhalePlatformCapability, ContractWhaleResponseMeta, ContractWhaleSeverity,
            ContractWhaleSignal, ContractWhaleSignalType, ContractWhaleSpotConfirmationContext,
            ContractWhaleSummary, ContractWhaleTradeOpportunity,
            ContractWhaleTradingDecisionResponse, ContractWhaleTrend60s, ContractWhaleWindowStats,
            ExchangeFlowContribution,
        },
        LOG_PREFIX as CWM_LOG_PREFIX, LOG_TARGET as CWM_LOG_TARGET,
    },
    normalizers::trade::now_ms,
    spot_whale_monitor::types::{
        SpotWhaleDirection, SpotWhaleSeverity, SpotWhaleSignal, SpotWhaleSignalType,
    },
    storage::contract_whale_repo::{ContractWhaleRepo, ContractWhaleSignalQuery},
    types::{
        flow::{FlowState, FlowWindow, VenueFlowBreakdown},
        market::{VenueConnectionStatus, VenueHealth},
        status::VenueHealthMap,
    },
};

#[derive(Debug, Deserialize, Default)]
pub struct ContractWhaleQuery {
    pub limit: Option<String>,
    pub symbol: Option<String>,
    pub severity: Option<String>,
    pub signal_type: Option<String>,
    pub direction: Option<String>,
    pub discord_sent: Option<String>,
    pub window_sec: Option<String>,
    pub exchange: Option<String>,
    pub net_direction: Option<String>,
    pub status: Option<String>,
    pub range: Option<String>,
    pub cursor: Option<String>,
    pub from: Option<String>,
    pub to: Option<String>,
    pub offset: Option<String>,
    pub include_hidden: Option<String>,
    pub hide_stale: Option<String>,
}

type ApiJsonResult = Result<Json<serde_json::Value>, (StatusCode, Json<serde_json::Value>)>;

#[derive(Debug, Clone, Default)]
pub struct ContractWhaleQualityBaseline {
    pub dynamic_multiple: Option<f64>,
    pub dynamic_baseline_btc: Option<f64>,
    pub dynamic_threshold_level: String,
    pub percentile_level: Option<f64>,
}

#[derive(Debug, Clone, serde::Serialize, Default)]
#[serde(rename_all = "camelCase")]
struct ContractWhalePipelineDebugResponse {
    symbol: String,
    range: String,
    raw_flow: PipelineRawFlowDebug,
    rolling_windows: BTreeMap<String, PipelineRollingWindowDebug>,
    detector: PipelineDetectorDebug,
    persistence: PipelinePersistenceDebug,
    history: PipelineHistoryDebug,
    latest: PipelineLatestDebug,
    error: Option<String>,
}

#[derive(Debug, Clone, serde::Serialize, Default)]
#[serde(rename_all = "camelCase")]
struct PipelineRawFlowDebug {
    flow_1s_rows: usize,
    oldest_ts: Option<i64>,
    newest_ts: Option<i64>,
    buy_volume_btc: f64,
    sell_volume_btc: f64,
    total_volume_btc: f64,
}

#[derive(Debug, Clone, serde::Serialize, Default)]
#[serde(rename_all = "camelCase")]
struct PipelineRollingWindowDebug {
    windows: usize,
    max_total_volume_btc: f64,
}

#[derive(Debug, Clone, serde::Serialize, Default)]
#[serde(rename_all = "camelCase")]
struct PipelineDetectorDebug {
    input_windows: usize,
    candidates: usize,
    accepted: usize,
    rejected: usize,
    reject_reasons: BTreeMap<String, usize>,
}

#[derive(Debug, Clone, serde::Serialize, Default)]
#[serde(rename_all = "camelCase")]
struct PipelinePersistenceDebug {
    persist_attempts: usize,
    persist_success: usize,
    persist_skipped: usize,
    skip_reasons: BTreeMap<String, usize>,
    persist_errors: usize,
}

#[derive(Debug, Clone, serde::Serialize, Default)]
#[serde(rename_all = "camelCase")]
struct PipelineHistoryDebug {
    contract_whale_signals_rows: usize,
}

#[derive(Debug, Clone, serde::Serialize, Default)]
#[serde(rename_all = "camelCase")]
struct PipelineLatestDebug {
    latest_count: usize,
    stale_count: usize,
    max_age_sec: i64,
    items: Vec<PipelineLatestItemDebug>,
}

#[derive(Debug, Clone, serde::Serialize, Default)]
#[serde(rename_all = "camelCase")]
struct PipelineLatestItemDebug {
    event_id: String,
    ts: i64,
    age_sec: i64,
    is_stale: bool,
    stale_reason: Option<String>,
}

#[derive(Debug, Clone, serde::Serialize, Default)]
#[serde(rename_all = "camelCase")]
struct ContractWhaleLatencyDebugResponse {
    symbol: String,
    range: String,
    server_time: i64,
    latest: ContractWhaleLatencyLatestDebug,
    contract_events: ContractWhaleLatencyLayerDebug,
    final_events_v2: ContractWhaleLatencyProjectionDebug,
    flow: ContractWhaleLatencyFlowDebug,
    diagnosis: ContractWhaleLatencyDiagnosis,
}

#[derive(Debug, Clone, serde::Serialize, Default)]
#[serde(rename_all = "camelCase")]
struct ContractWhaleLatencyLatestDebug {
    count: usize,
    max_ts: Option<i64>,
    age_sec: i64,
    stale_count: usize,
}

#[derive(Debug, Clone, serde::Serialize, Default)]
#[serde(rename_all = "camelCase")]
struct ContractWhaleLatencyLayerDebug {
    count: usize,
    max_event_ts: Option<i64>,
    lag_sec: i64,
    lag_vs_latest_sec: i64,
    cache_age_sec: i64,
    cache_ttl_sec: i64,
}

#[derive(Debug, Clone, serde::Serialize, Default)]
#[serde(rename_all = "camelCase")]
struct ContractWhaleLatencyProjectionDebug {
    active_count: usize,
    closed_count: usize,
    max_event_ts: Option<i64>,
    projection_lag_sec: i64,
    cache_age_sec: i64,
    cache_ttl_sec: i64,
    generated_at: Option<i64>,
}

#[derive(Debug, Clone, serde::Serialize, Default)]
#[serde(rename_all = "camelCase")]
struct ContractWhaleLatencyFlowDebug {
    updated_at: Option<i64>,
    flow_lag_sec: i64,
}

#[derive(Debug, Clone, serde::Serialize, Default)]
#[serde(rename_all = "camelCase")]
struct ContractWhaleLatencyDiagnosis {
    layer: String,
    reason: String,
}

#[derive(Debug, Clone, serde::Serialize, Default)]
#[serde(rename_all = "camelCase")]
struct ContractWhaleRawFlowDebugResponse {
    symbol: String,
    range: String,
    config: RawFlowConfigDebug,
    raw_trade_ingest: RawTradeIngestDebug,
    normalizer: RawFlowNormalizerDebug,
    aggregator: RawFlowAggregatorDebug,
    contract_flow_1s: RawFlowPersistenceDebug,
    diagnosis: RawFlowDiagnosisDebug,
    error: Option<String>,
}

#[derive(Debug, Clone, serde::Serialize, Default)]
#[serde(rename_all = "camelCase")]
struct RawFlowConfigDebug {
    app_requested_symbol: String,
    app_requested_symbol_base: String,
    query_symbol: String,
    query_symbol_enabled: bool,
    runtime_enabled: bool,
    runtime_dry_run: bool,
    runtime_enabled_symbols: Vec<String>,
    windows_sec: Vec<u64>,
}

#[derive(Debug, Clone, serde::Serialize, Default)]
#[serde(rename_all = "camelCase")]
struct RawTradeIngestDebug {
    venue_count: usize,
    ws_connected_count: usize,
    trade_active_count: usize,
    total_trade_messages: u64,
    exchanges: Vec<RawTradeIngestVenueDebug>,
}

#[derive(Debug, Clone, serde::Serialize, Default)]
#[serde(rename_all = "camelCase")]
struct RawTradeIngestVenueDebug {
    venue: String,
    requested_symbol: String,
    venue_symbol: Option<String>,
    symbol_mapping_status: String,
    symbol_mapping_error: Option<String>,
    ws_connected: bool,
    trade_subscribe_acked: bool,
    trade_message_count: u64,
    last_trade_ts: Option<i64>,
    trade_active: bool,
    activity_status: String,
}

#[derive(Debug, Clone, serde::Serialize, Default)]
#[serde(rename_all = "camelCase")]
struct RawFlowNormalizerDebug {
    query_symbol: String,
    normalized_query_symbol: String,
    app_requested_symbol: String,
    app_requested_symbol_base: String,
    app_requested_symbol_matches_query: bool,
    connector_symbol_mismatch: bool,
    query_venue_symbols: BTreeMap<String, String>,
}

#[derive(Debug, Clone, serde::Serialize, Default)]
#[serde(rename_all = "camelCase")]
struct RawFlowAggregatorDebug {
    flow_state_symbol: String,
    updated_at: i64,
    windows: BTreeMap<String, RawFlowAggregatorWindowDebug>,
}

#[derive(Debug, Clone, serde::Serialize, Default)]
#[serde(rename_all = "camelCase")]
struct RawFlowAggregatorWindowDebug {
    trade_count: u64,
    buy_volume_btc: f64,
    sell_volume_btc: f64,
    total_volume_btc: f64,
    active_venues: Vec<String>,
    stale_venues: Vec<String>,
    has_trades: bool,
}

#[derive(Debug, Clone, serde::Serialize, Default)]
#[serde(rename_all = "camelCase")]
struct RawFlowPersistenceDebug {
    exact_symbol_rows: usize,
    sibling_symbol_rows: usize,
    oldest_ts: Option<i64>,
    newest_ts: Option<i64>,
    buy_volume_btc: f64,
    sell_volume_btc: f64,
    total_volume_btc: f64,
    distinct_symbols: Vec<String>,
    distinct_product_ids: Vec<String>,
}

#[derive(Debug, Clone, serde::Serialize, Default)]
#[serde(rename_all = "camelCase")]
struct RawFlowDiagnosisDebug {
    status: String,
    primary_reason: String,
    details: Vec<String>,
}

#[derive(Debug, Clone, Copy, Default)]
struct ContractWhaleWarmupState {
    active: bool,
    until_ms: Option<i64>,
    remaining_ms: Option<i64>,
}

pub struct ContractWhaleResponseRuntime<'a> {
    pub venue_health: Option<&'a VenueHealthMap>,
    pub baselines: &'a BTreeMap<u64, ContractWhaleQualityBaseline>,
    pub liquidations: &'a BTreeMap<u64, ContractWhaleLiquidationContext>,
    pub market_context: &'a ContractWhaleMarketContext,
    pub booted_at_ms: Option<i64>,
}

pub async fn contract_whale_summary_route(
    State(state): State<AppState>,
    Query(query): Query<ContractWhaleQuery>,
    headers: HeaderMap,
) -> ApiJsonResult {
    log_summary_access(&headers);
    let symbol = parse_symbol_for_latest(query.symbol.as_deref())?;
    let exchange_filter = parse_exchange_filter(query.exchange.as_deref())?;
    let config = state.config().contract_whale_monitor;
    let runtime_config = contract_whale_runtime_config();
    let symbol_enabled = config.enabled && contract_whale_runtime_config().symbol_enabled(&symbol);
    let flow_state = state.flow_state_for_symbol(&symbol);
    let venue_health = state.venue_health();
    let baselines = state
        .contract_whale_store()
        .map(|store| load_quality_baselines(&store, &flow_state, &symbol))
        .unwrap_or_default();
    let liquidations = state
        .contract_whale_store()
        .map(|store| load_liquidation_contexts(&store, &flow_state, &symbol))
        .unwrap_or_default();
    let market_context = state
        .contract_whale_store()
        .map(|store| load_market_context(&store, &flow_state, &symbol))
        .unwrap_or_default();
    let mut response = build_contract_whale_response_with_runtime_and_baselines(
        &flow_state,
        &symbol,
        50,
        None,
        symbol_enabled,
        config.dry_run,
        ContractWhaleResponseRuntime {
            venue_health: Some(&venue_health),
            baselines: &baselines,
            liquidations: &liquidations,
            market_context: &market_context,
            booted_at_ms: Some(state.booted_at_ms()),
        },
    );
    enrich_contract_whale_response_with_state(&mut response, &state, &symbol);
    let mut summary = response.summary;
    summary.meta = contract_market_mismatch_meta(&runtime_config, exchange_filter.as_deref());
    Ok(Json(serde_json::json!(summary)))
}

fn log_summary_access(headers: &HeaderMap) {
    let user_agent = headers
        .get(header::USER_AGENT)
        .and_then(|value| value.to_str().ok())
        .map(sanitize_user_agent)
        .unwrap_or_else(|| "unknown".to_string());
    tracing::info!(
        target: CWM_LOG_TARGET,
        "{CWM_LOG_PREFIX} summary requested user_agent={user_agent}"
    );
}

fn sanitize_user_agent(value: &str) -> String {
    value
        .chars()
        .filter(|character| !character.is_control())
        .take(180)
        .collect()
}

pub async fn contract_whale_latest_route(
    State(state): State<AppState>,
    Query(query): Query<ContractWhaleQuery>,
) -> ApiJsonResult {
    let symbol = parse_symbol_for_latest(query.symbol.as_deref())?;
    let limit = parse_limit(query.limit.as_deref(), 50, 200)?;
    let hide_stale =
        parse_optional_bool(query.hide_stale.as_deref(), "hide_stale")?.unwrap_or(false);
    let range = query.range.clone().or_else(|| Some("24h".to_string()));
    let stale_after_ts = parse_range_start_ms(range.as_deref())?;
    let exchange_filter = parse_exchange_filter(query.exchange.as_deref())?;
    let flow_state = state.flow_state_for_symbol(&symbol);
    let venue_health = state.venue_health();
    let config = state.config().contract_whale_monitor;
    let store = state.contract_whale_store();
    let cwm_runtime_config = contract_whale_runtime_config();
    if let Some(meta) =
        contract_market_mismatch_meta(&cwm_runtime_config, exchange_filter.as_deref())
    {
        let mut response = build_contract_whale_history_response(
            Vec::new(),
            &symbol,
            limit,
            None,
            config.enabled,
            config.dry_run,
            Some(meta),
        );
        enrich_contract_whale_response_with_state(&mut response, &state, &symbol);
        return Ok(Json(with_latest_stale_annotations(
            response,
            stale_after_ts,
            range.as_deref(),
            hide_stale,
        )));
    }
    if !config.enabled || !cwm_runtime_config.symbol_enabled(&symbol) {
        let mut response = build_contract_whale_response_with_runtime(
            &flow_state,
            &symbol,
            limit,
            None,
            false,
            config.dry_run,
            Some(&venue_health),
        );
        enrich_contract_whale_response_with_state(&mut response, &state, &symbol);
        return Ok(Json(with_latest_stale_annotations(
            response,
            stale_after_ts,
            range.as_deref(),
            hide_stale,
        )));
    }
    if let Some(store) = store.as_ref() {
        match store.query_contract_whale_signals(&ContractWhaleSignalQuery {
            symbol: Some(symbol.clone()),
            exchange: exchange_filter.clone(),
            limit,
            ..ContractWhaleSignalQuery::default()
        }) {
            Ok(items) if !items.is_empty() => {
                let now = if flow_state.updated_at > 0 {
                    flow_state.updated_at
                } else {
                    now_ms()
                };
                let mut response = filter_latest_response_by_exchange(
                    build_contract_whale_items_response(
                        items,
                        &symbol,
                        limit,
                        config.enabled,
                        config.dry_run,
                        contract_exchange_statuses(
                            &flow_state,
                            Some(&venue_health),
                            config.enabled,
                            now,
                        ),
                        trend_60s_from_flow_state(&flow_state, &symbol, now),
                    ),
                    exchange_filter.as_deref(),
                );
                enrich_contract_whale_response_with_state(&mut response, &state, &symbol);
                return Ok(Json(with_latest_stale_annotations(
                    response,
                    stale_after_ts,
                    range.as_deref(),
                    hide_stale,
                )));
            }
            Ok(_) => {}
            Err(error) => {
                tracing::warn!(
                    target: CWM_LOG_TARGET,
                    error = %error,
                    "{} latest query failed",
                    CWM_LOG_PREFIX
                );
            }
        }
    }
    let baselines = store
        .as_ref()
        .map(|store| load_quality_baselines(store, &flow_state, &symbol))
        .unwrap_or_default();
    let liquidations = store
        .as_ref()
        .map(|store| load_liquidation_contexts(store, &flow_state, &symbol))
        .unwrap_or_default();
    let market_context = store
        .as_ref()
        .map(|store| load_market_context(store, &flow_state, &symbol))
        .unwrap_or_default();
    let mut response = filter_latest_response_by_exchange(
        build_contract_whale_response_with_runtime_and_baselines(
            &flow_state,
            &symbol,
            limit,
            None,
            config.enabled,
            config.dry_run,
            ContractWhaleResponseRuntime {
                venue_health: Some(&venue_health),
                baselines: &baselines,
                liquidations: &liquidations,
                market_context: &market_context,
                booted_at_ms: Some(state.booted_at_ms()),
            },
        ),
        exchange_filter.as_deref(),
    );
    enrich_contract_whale_response_with_state(&mut response, &state, &symbol);
    Ok(Json(with_latest_stale_annotations(
        response,
        stale_after_ts,
        range.as_deref(),
        hide_stale,
    )))
}

pub async fn contract_whale_trading_decisions_route(
    State(state): State<AppState>,
    Query(query): Query<ContractWhaleQuery>,
) -> ApiJsonResult {
    let symbol = parse_symbol_for_latest(query.symbol.as_deref())?;
    let limit = parse_limit(query.limit.as_deref(), 50, 200)?;
    let range = query.range.clone().or_else(|| Some("24h".to_string()));
    let stale_after_ts = parse_range_start_ms(range.as_deref())?;
    let exchange_filter = parse_exchange_filter(query.exchange.as_deref())?;
    let flow_state = state.flow_state_for_symbol(&symbol);
    let venue_health = state.venue_health();
    let config = state.config().contract_whale_monitor;
    let store = state.contract_whale_store();
    let cwm_runtime_config = contract_whale_runtime_config();

    let response = if let Some(meta) =
        contract_market_mismatch_meta(&cwm_runtime_config, exchange_filter.as_deref())
    {
        let mut response = build_contract_whale_history_response(
            Vec::new(),
            &symbol,
            limit,
            None,
            config.enabled,
            config.dry_run,
            Some(meta),
        );
        enrich_contract_whale_response_with_state(&mut response, &state, &symbol);
        response
    } else if !config.enabled || !cwm_runtime_config.symbol_enabled(&symbol) {
        let mut response = build_contract_whale_response_with_runtime(
            &flow_state,
            &symbol,
            limit,
            None,
            false,
            config.dry_run,
            Some(&venue_health),
        );
        enrich_contract_whale_response_with_state(&mut response, &state, &symbol);
        response
    } else if let Some(store) = store.as_ref() {
        match store.query_contract_whale_signals(&ContractWhaleSignalQuery {
            symbol: Some(symbol.clone()),
            exchange: exchange_filter.clone(),
            limit,
            ..ContractWhaleSignalQuery::default()
        }) {
            Ok(items) if !items.is_empty() => {
                let now = if flow_state.updated_at > 0 {
                    flow_state.updated_at
                } else {
                    now_ms()
                };
                let mut response = filter_latest_response_by_exchange(
                    build_contract_whale_items_response(
                        items,
                        &symbol,
                        limit,
                        config.enabled,
                        config.dry_run,
                        contract_exchange_statuses(
                            &flow_state,
                            Some(&venue_health),
                            config.enabled,
                            now,
                        ),
                        trend_60s_from_flow_state(&flow_state, &symbol, now),
                    ),
                    exchange_filter.as_deref(),
                );
                enrich_contract_whale_response_with_state(&mut response, &state, &symbol);
                response
            }
            _ => {
                let baselines = load_quality_baselines(store, &flow_state, &symbol);
                let liquidations = load_liquidation_contexts(store, &flow_state, &symbol);
                let market_context = load_market_context(store, &flow_state, &symbol);
                let mut response = filter_latest_response_by_exchange(
                    build_contract_whale_response_with_runtime_and_baselines(
                        &flow_state,
                        &symbol,
                        limit,
                        None,
                        config.enabled,
                        config.dry_run,
                        ContractWhaleResponseRuntime {
                            venue_health: Some(&venue_health),
                            baselines: &baselines,
                            liquidations: &liquidations,
                            market_context: &market_context,
                            booted_at_ms: Some(state.booted_at_ms()),
                        },
                    ),
                    exchange_filter.as_deref(),
                );
                enrich_contract_whale_response_with_state(&mut response, &state, &symbol);
                response
            }
        }
    } else {
        let mut response = filter_latest_response_by_exchange(
            build_contract_whale_response_with_runtime_and_baselines(
                &flow_state,
                &symbol,
                limit,
                None,
                config.enabled,
                config.dry_run,
                ContractWhaleResponseRuntime {
                    venue_health: Some(&venue_health),
                    baselines: &BTreeMap::new(),
                    liquidations: &BTreeMap::new(),
                    market_context: &ContractWhaleMarketContext::default(),
                    booted_at_ms: Some(state.booted_at_ms()),
                },
            ),
            exchange_filter.as_deref(),
        );
        enrich_contract_whale_response_with_state(&mut response, &state, &symbol);
        response
    };

    let now = now_ms();
    let latest_debug = build_pipeline_latest_debug(&response.items, stale_after_ts, range.as_deref(), now);
    let fresh_items = response
        .items
        .into_iter()
        .zip(latest_debug.items.iter())
        .filter_map(|(item, debug)| (!debug.is_stale).then_some(item))
        .collect::<Vec<_>>();
    let mut decision = build_trading_decision_response(
        &symbol,
        &fresh_items,
        &response.summary.market_structure_lite,
        response.summary.noise_suppression.clone(),
        now,
    );
    if fresh_items.is_empty() && latest_debug.stale_count > 0 {
        decision.market_bias = "NEUTRAL".to_string();
        decision.bias_confidence = 0;
        decision.bias_reason =
            format!("{symbol} latest 为旧快照，最近 {} 没有新的 {} 主力历史信号。", range.unwrap_or_else(|| "24h".to_string()), symbol);
        decision.no_trade_zones.push(crate::contract_whale_monitor::types::ContractWhaleNoTradeZone {
            reason: decision.bias_reason.clone(),
            range_label: "stale_latest_only".to_string(),
            low_price: 0.0,
            high_price: 0.0,
        });
    }

    Ok(Json(serde_json::to_value(decision).unwrap_or_else(|_| serde_json::json!({
        "symbol": symbol,
        "timestamp": now,
        "marketBias": "NEUTRAL",
        "biasConfidence": 0,
        "biasReason": "serialize_failed",
        "noiseSuppression": ContractWhaleNoiseSuppressionSummary::default(),
        "topSetups": [],
        "noTradeZones": [],
    }))))
}

pub async fn contract_whale_intelligence_terminal_route(
    State(state): State<AppState>,
    Query(query): Query<ContractWhaleQuery>,
) -> ApiJsonResult {
    let symbol = parse_symbol_for_latest(query.symbol.as_deref())?;
    let limit = parse_limit(query.limit.as_deref(), 50, 200)?;
    let range = query.range.clone().or_else(|| Some("24h".to_string()));
    let stale_after_ts = parse_range_start_ms(range.as_deref())?;
    let exchange_filter = parse_exchange_filter(query.exchange.as_deref())?;
    let flow_state = state.flow_state_for_symbol(&symbol);
    let venue_health = state.venue_health();
    let config = state.config().contract_whale_monitor;
    let store = state.contract_whale_store();
    let cwm_runtime_config = contract_whale_runtime_config();

    let response = if let Some(meta) =
        contract_market_mismatch_meta(&cwm_runtime_config, exchange_filter.as_deref())
    {
        let mut response = build_contract_whale_history_response(
            Vec::new(),
            &symbol,
            limit,
            None,
            config.enabled,
            config.dry_run,
            Some(meta),
        );
        enrich_contract_whale_response_with_state(&mut response, &state, &symbol);
        response
    } else if !config.enabled || !cwm_runtime_config.symbol_enabled(&symbol) {
        let mut response = build_contract_whale_response_with_runtime(
            &flow_state,
            &symbol,
            limit,
            None,
            false,
            config.dry_run,
            Some(&venue_health),
        );
        enrich_contract_whale_response_with_state(&mut response, &state, &symbol);
        response
    } else if let Some(store) = store.as_ref() {
        match store.query_contract_whale_signals(&ContractWhaleSignalQuery {
            symbol: Some(symbol.clone()),
            exchange: exchange_filter.clone(),
            limit,
            ..ContractWhaleSignalQuery::default()
        }) {
            Ok(items) if !items.is_empty() => {
                let now = if flow_state.updated_at > 0 {
                    flow_state.updated_at
                } else {
                    now_ms()
                };
                let mut response = filter_latest_response_by_exchange(
                    build_contract_whale_items_response(
                        items,
                        &symbol,
                        limit,
                        config.enabled,
                        config.dry_run,
                        contract_exchange_statuses(
                            &flow_state,
                            Some(&venue_health),
                            config.enabled,
                            now,
                        ),
                        trend_60s_from_flow_state(&flow_state, &symbol, now),
                    ),
                    exchange_filter.as_deref(),
                );
                enrich_contract_whale_response_with_state(&mut response, &state, &symbol);
                response
            }
            _ => {
                let baselines = load_quality_baselines(store, &flow_state, &symbol);
                let liquidations = load_liquidation_contexts(store, &flow_state, &symbol);
                let market_context = load_market_context(store, &flow_state, &symbol);
                let mut response = filter_latest_response_by_exchange(
                    build_contract_whale_response_with_runtime_and_baselines(
                        &flow_state,
                        &symbol,
                        limit,
                        None,
                        config.enabled,
                        config.dry_run,
                        ContractWhaleResponseRuntime {
                            venue_health: Some(&venue_health),
                            baselines: &baselines,
                            liquidations: &liquidations,
                            market_context: &market_context,
                            booted_at_ms: Some(state.booted_at_ms()),
                        },
                    ),
                    exchange_filter.as_deref(),
                );
                enrich_contract_whale_response_with_state(&mut response, &state, &symbol);
                response
            }
        }
    } else {
        let mut response = filter_latest_response_by_exchange(
            build_contract_whale_response_with_runtime_and_baselines(
                &flow_state,
                &symbol,
                limit,
                None,
                config.enabled,
                config.dry_run,
                ContractWhaleResponseRuntime {
                    venue_health: Some(&venue_health),
                    baselines: &BTreeMap::new(),
                    liquidations: &BTreeMap::new(),
                    market_context: &ContractWhaleMarketContext::default(),
                    booted_at_ms: Some(state.booted_at_ms()),
                },
            ),
            exchange_filter.as_deref(),
        );
        enrich_contract_whale_response_with_state(&mut response, &state, &symbol);
        response
    };

    let now = now_ms();
    let latest_debug =
        build_pipeline_latest_debug(&response.items, stale_after_ts, range.as_deref(), now);
    let fresh_items = response
        .items
        .into_iter()
        .zip(latest_debug.items.iter())
        .filter_map(|(item, debug)| (!debug.is_stale).then_some(item))
        .collect::<Vec<_>>();
    let mut intelligence = build_contract_whale_intelligence_response(
        &symbol,
        &fresh_items,
        &response.summary.market_structure_lite,
        response.summary.noise_suppression.clone(),
        now,
    );

    if fresh_items.is_empty() && latest_debug.stale_count > 0 {
        intelligence.market_regime.regime = "RANGING".to_string();
        intelligence.market_regime.confidence = 0;
        intelligence.market_regime.reason = format!(
            "{symbol} latest 为旧快照，最近 {} 没有新的 {} 主力历史信号。",
            range.clone().unwrap_or_else(|| "24h".to_string()),
            symbol
        );
    }

    Ok(Json(serde_json::to_value(intelligence).unwrap_or_else(
        |_| serde_json::json!({
            "symbol": symbol,
            "timestamp": now,
            "marketRegime": {
                "regime": "RANGING",
                "confidence": 0,
                "reason": "serialize_failed"
            },
            "liquidityBehaviors": [],
            "rankedEvents": [],
            "opportunityMap": [],
            "noiseSuppression": ContractWhaleNoiseSuppressionSummary::default(),
        }),
    )))
}

pub fn build_contract_whale_intelligence_response(
    symbol: &str,
    items: &[ContractWhaleSignal],
    market_structure_lite: &ContractWhaleMarketStructureLite,
    noise_suppression: ContractWhaleNoiseSuppressionSummary,
    timestamp: i64,
) -> ContractWhaleIntelligenceResponse {
    intelligence::build_intelligence_response(
        symbol,
        items,
        market_structure_lite,
        noise_suppression,
        timestamp,
    )
}

pub fn build_contract_whale_response(
    flow_state: &FlowState,
    symbol: &str,
    limit: usize,
    severity: Option<&str>,
    enabled: bool,
    dry_run: bool,
) -> ContractWhaleLatestResponse {
    build_contract_whale_response_with_runtime(
        flow_state, symbol, limit, severity, enabled, dry_run, None,
    )
}

pub async fn contract_whale_history_route(
    State(state): State<AppState>,
    Query(query): Query<ContractWhaleQuery>,
) -> ApiJsonResult {
    let history_query = parse_history_query(&query)?;
    let symbol_for_filter = history_query.symbol.as_deref().unwrap_or("all").to_string();
    let config = state.config().contract_whale_monitor;
    let runtime_config = contract_whale_runtime_config();
    if let Some(meta) =
        contract_market_mismatch_meta(&runtime_config, history_query.exchange.as_deref())
    {
        let mut response = build_contract_whale_history_response(
            Vec::new(),
            &symbol_for_filter,
            history_query.limit,
            None,
            config.enabled,
            config.dry_run,
            Some(meta),
        );
        enrich_contract_whale_response_with_state(&mut response, &state, &symbol_for_filter);
        return Ok(Json(serde_json::json!(response)));
    }
    if !config.enabled {
        let mut response = build_contract_whale_history_response(
            Vec::new(),
            &symbol_for_filter,
            history_query.limit,
            None,
            false,
            config.dry_run,
            None,
        );
        enrich_contract_whale_response_with_state(&mut response, &state, &symbol_for_filter);
        return Ok(Json(serde_json::json!(response)));
    }
    if let Some(store) = state.contract_whale_store() {
        match store.query_contract_whale_signals(&history_query) {
            Ok(items) => {
                let mut response = build_contract_whale_history_response(
                    items,
                    &symbol_for_filter,
                    history_query.limit,
                    None,
                    config.enabled,
                    config.dry_run,
                    None,
                );
                enrich_contract_whale_response_with_state(
                    &mut response,
                    &state,
                    &symbol_for_filter,
                );
                return Ok(Json(serde_json::json!(response)));
            }
            Err(error) => {
                tracing::warn!(
                    target: CWM_LOG_TARGET,
                    error = %error,
                    "{} history query failed",
                    CWM_LOG_PREFIX
                );
            }
        }
    }
    let mut response = build_contract_whale_history_response(
        Vec::new(),
        &symbol_for_filter,
        history_query.limit,
        None,
        config.enabled,
        config.dry_run,
        None,
    );
    enrich_contract_whale_response_with_state(&mut response, &state, &symbol_for_filter);
    Ok(Json(serde_json::json!(response)))
}

fn enrich_contract_whale_response_with_state(
    response: &mut ContractWhaleLatestResponse,
    state: &AppState,
    symbol: &str,
) {
    let flow_state = state.flow_state_for_symbol(symbol);
    let current_market_price = current_market_price_from_flow_state(&flow_state, symbol);
    decorate_and_filter_price_deviated_signals(
        &mut response.items,
        current_market_price,
        contract_whale_runtime_config()
            .toxic_order
            .max_price_deviation_pct,
    );
    let spot_context = spot_confirmation_context_from_state(state, symbol);
    enrich_contract_whale_response(response, &spot_context);
}

fn enrich_contract_whale_response(
    response: &mut ContractWhaleLatestResponse,
    spot_context: &ContractWhaleSpotConfirmationContext,
) {
    for signal in &mut response.items {
        signal.spot_confirmation = spot_confirmation_for_signal(signal, spot_context);
        decorate_market_structure_scores(signal, response.summary.overall_data_quality);
    }
    refresh_response_summary_from_items(response, spot_context);
}

fn decorate_market_structure_scores(signal: &mut ContractWhaleSignal, _data_quality: u8) {
    let runtime_config = contract_whale_runtime_config();
    let spot_score = if runtime_config.toxic_order.enable_spot_score
        && signal.spot_confirmation.status != "disabled"
        && signal.spot_confirmation.status != "no_spot_sample"
    {
        signal.spot_confirmation.score
    } else {
        0
    };
    let contract_score = if runtime_config.toxic_order.enable_contract_score {
        signal.score
    } else {
        0
    };
    let cross_confirm_score = cross_confirm_score(&signal.spot_confirmation.confirmation_type);
    let structure_raw =
        0.4 * spot_score as f64 + 0.4 * contract_score as f64 + 0.2 * cross_confirm_score as f64;
    signal.spot_score = Some(spot_score);
    signal.contract_score = Some(contract_score);
    signal.main_force_score = Some(structure_raw.round().clamp(0.0, 100.0) as u8);
}

fn refresh_response_summary_from_items(
    response: &mut ContractWhaleLatestResponse,
    spot_context: &ContractWhaleSpotConfirmationContext,
) {
    let latest = response.items.first();
    response.summary.signal_count = response.items.len();
    response.summary.latest_signal_at = latest.map(|signal| signal.ts);
    response.summary.latest_severity = latest
        .map(|signal| signal.severity)
        .unwrap_or(ContractWhaleSeverity::Calm);
    let latest_direction = latest
        .map(|signal| direction_key(signal.direction).to_string())
        .unwrap_or_else(|| "neutral".to_string());
    if !response.summary.warmup && response.summary.enabled {
        response.summary.status = latest
            .map(|signal| status_code(signal.severity).to_string())
            .unwrap_or_else(|| "calm".to_string());
    }
    response.summary.direction = latest_direction.clone();
    response.summary.latest_direction = latest_direction;
    let last_discord_sent_at = response
        .items
        .iter()
        .filter(|signal| signal.discord_sent)
        .filter_map(|signal| signal.discord_sent_at.or(Some(signal.ts)))
        .max();
    response.summary.latest_pushed_at_ms = last_discord_sent_at;
    response.summary.last_discord_sent_at = last_discord_sent_at;
    response.summary.discord_dry_run_stats =
        discord_dry_run_stats(&response.items, response.summary.updated_at_ms);
    response.summary.market_structure_lite = market_structure_lite_from_items(
        &response.items,
        spot_context,
        response.summary.overall_data_quality,
    );
}

pub fn decorate_price_deviation_signals(
    items: &mut [ContractWhaleSignal],
    current_market_price_usd: Option<f64>,
    max_deviation_pct: f64,
) {
    let max_deviation_pct = if max_deviation_pct.is_finite() && max_deviation_pct > 0.0 {
        max_deviation_pct
    } else {
        5.0
    };
    for signal in items.iter_mut() {
        let order_price = signal
            .order_price_usd
            .filter(|price| price.is_finite() && *price > 0.0)
            .or_else(|| signal_average_price_usd(signal));
        let current_price = current_market_price_usd
            .filter(|price| price.is_finite() && *price > 0.0)
            .or(signal.current_market_price_usd);
        signal.order_price_usd = order_price.map(|price| round_for_route(price, 2));
        signal.current_market_price_usd = current_price.map(|price| round_for_route(price, 2));
        signal.price_deviation_pct = order_price
            .zip(current_price)
            .and_then(|(order_price, current_price)| {
                price_deviation_pct(order_price, current_price)
            })
            .map(|value| round_for_route(value, 4));
        signal.price_deviation_filtered = signal
            .price_deviation_pct
            .is_some_and(|value| value > max_deviation_pct);
    }
}

fn decorate_and_filter_price_deviated_signals(
    items: &mut Vec<ContractWhaleSignal>,
    current_market_price_usd: Option<f64>,
    max_deviation_pct: f64,
) {
    decorate_price_deviation_signals(items, current_market_price_usd, max_deviation_pct);
    items.retain(|signal| !signal.price_deviation_filtered);
}

fn signal_average_price_usd(signal: &ContractWhaleSignal) -> Option<f64> {
    (signal.total_volume_btc > f64::EPSILON && signal.total_notional_usd > 0.0)
        .then(|| signal.total_notional_usd / signal.total_volume_btc)
        .filter(|price| price.is_finite() && *price > 0.0)
}

fn price_deviation_pct(order_price: f64, current_price: f64) -> Option<f64> {
    (order_price.is_finite()
        && current_price.is_finite()
        && order_price > 0.0
        && current_price > 0.0)
        .then(|| (order_price - current_price).abs() / current_price * 100.0)
}

fn current_market_price_from_flow_state(flow_state: &FlowState, symbol: &str) -> Option<f64> {
    [5_u64, 15, 60]
        .into_iter()
        .filter_map(|window_sec| flow_window_for_seconds(flow_state, window_sec))
        .filter(|window| symbol_matches_window(&window.symbol, symbol))
        .find_map(current_market_price_from_window)
}

fn current_market_price_from_window(window: &FlowWindow) -> Option<f64> {
    let total_volume = window.aggressive_buy_btc + window.aggressive_sell_btc;
    let total_notional = window.aggressive_buy_usd + window.aggressive_sell_usd;
    if total_volume > f64::EPSILON && total_notional > 0.0 {
        return Some(total_notional / total_volume)
            .filter(|price| price.is_finite() && *price > 0.0);
    }
    window
        .mid_end
        .or(window.mid_start)
        .filter(|price| price.is_finite() && *price > 0.0)
}

fn current_market_price_from_trend(_trend: &ContractWhaleTrend60s) -> Option<f64> {
    None
}

fn spot_confirmation_context_from_state(
    state: &AppState,
    symbol: &str,
) -> ContractWhaleSpotConfirmationContext {
    let response = state.spot_whale_service().latest(symbol, 1);
    if !response.summary.enabled {
        return ContractWhaleSpotConfirmationContext {
            status: "disabled".to_string(),
            confirmation_type: "spot_monitor_disabled".to_string(),
            direction: "neutral".to_string(),
            ..Default::default()
        };
    }
    let Some(signal) = response.items.first() else {
        return ContractWhaleSpotConfirmationContext {
            status: "no_spot_sample".to_string(),
            confirmation_type: "unavailable".to_string(),
            direction: "neutral".to_string(),
            ..Default::default()
        };
    };
    spot_confirmation_context_from_signal(signal)
}

fn spot_confirmation_context_from_signal(
    signal: &SpotWhaleSignal,
) -> ContractWhaleSpotConfirmationContext {
    ContractWhaleSpotConfirmationContext {
        status: "available".to_string(),
        confirmation_type: "spot_context_only".to_string(),
        direction: spot_direction_key(signal.direction).to_string(),
        score: signal.score,
        latest_signal_id: Some(signal.id.clone()),
        latest_signal_at: Some(signal.ts),
        signal_type: Some(spot_signal_type_key(signal.signal_type).to_string()),
        severity: Some(spot_severity_key(signal.severity).to_string()),
        total_volume_btc: Some(round_for_route(signal.total_volume_base, 3)),
        net_volume_btc: Some(round_for_route(signal.net_volume_base, 3)),
        dominance: Some(round_for_route(signal.dominance, 4)),
        coinbase_premium_pct: signal
            .coinbase_premium_pct
            .map(|value| round_for_route(value, 4)),
        final_result: Some(signal.final_result.clone()),
    }
}

fn spot_confirmation_for_signal(
    signal: &ContractWhaleSignal,
    base: &ContractWhaleSpotConfirmationContext,
) -> ContractWhaleSpotConfirmationContext {
    if base.status != "available" {
        return base.clone();
    }
    let contract_direction = direction_key(signal.direction);
    let spot_direction = base.direction.as_str();
    let confirmation_type = match (contract_direction, spot_direction) {
        ("buy", "buy") | ("sell", "sell") => "confirms_contract_direction",
        ("sell", "absorption") => "spot_absorption_against_contract_sell",
        ("buy", "suppression") => "spot_resistance_against_contract_buy",
        ("buy", "sell") | ("sell", "buy") => "spot_divergence",
        _ => "spot_context_only",
    };
    let status = match confirmation_type {
        "confirms_contract_direction"
        | "spot_absorption_against_contract_sell"
        | "spot_resistance_against_contract_buy" => "confirmed",
        "spot_divergence" => "divergent",
        _ => "context",
    };
    let mut context = base.clone();
    context.status = status.to_string();
    context.confirmation_type = confirmation_type.to_string();
    context
}

fn market_structure_lite_from_items(
    items: &[ContractWhaleSignal],
    spot_context: &ContractWhaleSpotConfirmationContext,
    data_quality: u8,
) -> ContractWhaleMarketStructureLite {
    let Some(signal) = items.iter().max_by_key(|signal| signal.score) else {
        return ContractWhaleMarketStructureLite {
            status: "calm".to_string(),
            regime_type: "unclear".to_string(),
            data_quality,
            reason: "暂无 CWM 主力合约信号，结构评分保持观察。".to_string(),
            ..Default::default()
        };
    };
    let runtime_config = contract_whale_runtime_config();
    let contract_score = if runtime_config.toxic_order.enable_contract_score {
        signal.score
    } else {
        0
    };
    let spot_score = if !runtime_config.toxic_order.enable_spot_score
        || spot_context.status == "disabled"
        || spot_context.status == "no_spot_sample"
    {
        0
    } else {
        spot_context.score
    };
    let cross_confirm_score = cross_confirm_score(&spot_context.confirmation_type);
    let structure_raw =
        0.4 * spot_score as f64 + 0.4 * contract_score as f64 + 0.2 * cross_confirm_score as f64;
    let main_force_score = structure_raw.round().clamp(0.0, 100.0) as u8;
    let extreme_impact_score = extreme_impact_score(signal);
    let structure_bias = structure_bias(signal, &spot_context.confirmation_type);
    let main_force_confirmed =
        main_force_score >= 75 && data_quality >= 70 && cross_confirm_score >= 60;
    let extreme_impact_confirmed = extreme_impact_score >= 85 && data_quality >= 70;
    let regime_type = market_regime_type(signal, &spot_context.confirmation_type);
    let status = if main_force_confirmed || extreme_impact_confirmed {
        "confirmed"
    } else if main_force_score >= 55 || extreme_impact_score >= 65 {
        "watch"
    } else {
        "calm"
    };
    ContractWhaleMarketStructureLite {
        status: status.to_string(),
        regime_type: regime_type.to_string(),
        main_force_score,
        extreme_impact_score,
        structure_bias,
        confidence: market_structure_confidence(data_quality, cross_confirm_score, signal),
        data_quality,
        spot_score,
        contract_score,
        cross_confirm_score,
        main_force_confirmed,
        extreme_impact_confirmed,
        reason: market_structure_reason(regime_type, &spot_context.confirmation_type),
    }
}

fn discord_dry_run_stats(
    items: &[ContractWhaleSignal],
    now: i64,
) -> ContractWhaleDiscordDryRunStats {
    let from = now.saturating_sub(60 * 60 * 1000);
    let mut stats = ContractWhaleDiscordDryRunStats::default();
    for signal in items.iter().filter(|signal| signal.ts >= from) {
        stats.signals_1h += 1;
        match signal.severity {
            ContractWhaleSeverity::High => stats.high_1h += 1,
            ContractWhaleSeverity::Critical => stats.critical_1h += 1,
            ContractWhaleSeverity::S => stats.s_1h += 1,
            ContractWhaleSeverity::Calm | ContractWhaleSeverity::Medium => {}
        }
        if signal.discord_would_send || signal.discord_eligible {
            stats.would_send_1h += 1;
            continue;
        }
        let reason = signal.discord_reason.to_ascii_lowercase();
        if reason.contains("cooldown") || reason.contains("duplicate") {
            stats.skipped_cooldown_1h += 1;
        } else if reason.contains("data_quality") {
            stats.skipped_data_quality_1h += 1;
        } else if reason.contains("warmup") {
            stats.skipped_warmup_1h += 1;
        } else if reason.contains("medium") || reason.contains("display") {
            stats.skipped_display_only_1h += 1;
        } else {
            stats.skipped_low_score_1h += 1;
        }
    }
    stats
}

fn cross_confirm_score(confirmation_type: &str) -> u8 {
    match confirmation_type {
        "confirms_contract_direction" => 75,
        "spot_absorption_against_contract_sell" | "spot_resistance_against_contract_buy" => 65,
        "spot_divergence" => 25,
        "spot_context_only" => 45,
        _ => 0,
    }
}

fn extreme_impact_score(signal: &ContractWhaleSignal) -> u8 {
    let base = match signal.severity {
        ContractWhaleSeverity::S => signal.score,
        ContractWhaleSeverity::Critical => signal.score.saturating_sub(5),
        ContractWhaleSeverity::High => signal.score.saturating_sub(15),
        ContractWhaleSeverity::Medium => signal.score.saturating_sub(25),
        ContractWhaleSeverity::Calm => 0,
    };
    if signal.liquidation_suspected {
        base.saturating_add(8).min(100)
    } else {
        base
    }
}

fn structure_bias(signal: &ContractWhaleSignal, confirmation_type: &str) -> i16 {
    let base = match signal.direction {
        ContractWhaleDirection::Buy => signal.score as i16,
        ContractWhaleDirection::Sell => -(signal.score as i16),
        ContractWhaleDirection::Absorption => 25,
        ContractWhaleDirection::Suppression => -25,
    };
    let adjusted = match confirmation_type {
        "confirms_contract_direction" => base,
        "spot_absorption_against_contract_sell" => 20,
        "spot_resistance_against_contract_buy" => -20,
        "spot_divergence" => base / 2,
        _ => base * 2 / 3,
    };
    adjusted.clamp(-100, 100)
}

fn market_regime_type(signal: &ContractWhaleSignal, confirmation_type: &str) -> &'static str {
    if signal.liquidation_suspected {
        return match signal.direction {
            ContractWhaleDirection::Sell => "long_liquidation_cascade",
            ContractWhaleDirection::Buy => "contract_short_squeeze",
            _ => "extreme_contract_flow",
        };
    }
    match confirmation_type {
        "spot_absorption_against_contract_sell" => "downside_absorption",
        "spot_resistance_against_contract_buy" => "upside_resistance",
        "confirms_contract_direction" => match signal.direction {
            ContractWhaleDirection::Buy => "main_force_long_build",
            ContractWhaleDirection::Sell => "main_force_short_build",
            _ => "range_rotation",
        },
        "spot_divergence" => "range_rotation",
        _ => match signal.signal_type {
            ContractWhaleSignalType::DownsideAbsorption => "downside_absorption",
            ContractWhaleSignalType::UpsideSuppression => "upside_resistance",
            ContractWhaleSignalType::AggressiveBuy | ContractWhaleSignalType::AggressiveSell => {
                "contract_flow_shock"
            }
        },
    }
}

fn market_structure_confidence(
    data_quality: u8,
    cross_confirm_score: u8,
    signal: &ContractWhaleSignal,
) -> u8 {
    let multi_source = if signal.multi_exchange_confirmed {
        80
    } else {
        45
    };
    (0.35 * data_quality as f64
        + 0.30 * cross_confirm_score as f64
        + 0.20 * multi_source as f64
        + 0.15 * signal.score as f64)
        .round()
        .clamp(0.0, 100.0) as u8
}

fn market_structure_reason(regime_type: &str, confirmation_type: &str) -> String {
    match regime_type {
        "main_force_long_build" => "合约主动买入与现货方向确认，主力建多概率提高。".to_string(),
        "main_force_short_build" => "合约主动卖出与现货方向确认，主力建空概率提高。".to_string(),
        "downside_absorption" => "合约卖压出现时现货呈承接/吸收，暂按下方吸收观察。".to_string(),
        "upside_resistance" => "合约买盘出现时现货呈压制/派发，暂按上方压制观察。".to_string(),
        "long_liquidation_cascade" | "contract_short_squeeze" => {
            "疑似清算驱动，极端冲击升高，但不直接确认主力建仓。".to_string()
        }
        _ if confirmation_type == "spot_divergence" => {
            "现货与合约方向分歧，主力结构确认度降低。".to_string()
        }
        _ => "当前主要是合约成交流冲击，现货确认不足，保持观察。".to_string(),
    }
}

fn spot_direction_key(direction: SpotWhaleDirection) -> &'static str {
    match direction {
        SpotWhaleDirection::Buy => "buy",
        SpotWhaleDirection::Sell => "sell",
        SpotWhaleDirection::Absorption => "absorption",
        SpotWhaleDirection::Suppression => "suppression",
        SpotWhaleDirection::Dislocation => "dislocation",
    }
}

fn spot_signal_type_key(signal_type: SpotWhaleSignalType) -> &'static str {
    match signal_type {
        SpotWhaleSignalType::SpotAggressiveBuy => "spot_aggressive_buy",
        SpotWhaleSignalType::SpotAggressiveSell => "spot_aggressive_sell",
        SpotWhaleSignalType::SpotDownsideAbsorption => "spot_downside_absorption",
        SpotWhaleSignalType::SpotUpsideSuppression => "spot_upside_suppression",
        SpotWhaleSignalType::SpotExchangeDislocation => "spot_exchange_dislocation",
    }
}

fn spot_severity_key(severity: SpotWhaleSeverity) -> &'static str {
    match severity {
        SpotWhaleSeverity::Calm => "calm",
        SpotWhaleSeverity::Medium => "medium",
        SpotWhaleSeverity::High => "high",
        SpotWhaleSeverity::Critical => "critical",
        SpotWhaleSeverity::S => "s",
    }
}

fn round_for_route(value: f64, decimals: u32) -> f64 {
    let factor = 10_f64.powi(decimals as i32);
    (value * factor).round() / factor
}

pub async fn contract_whale_metrics_route(State(state): State<AppState>) -> impl IntoResponse {
    let config = state.config().contract_whale_monitor;
    let flow_state = state.flow_state_for_symbol("BTC");
    let venue_health = state.venue_health();
    let now = if flow_state.updated_at > 0 {
        flow_state.updated_at
    } else {
        now_ms()
    };
    let exchanges =
        contract_exchange_statuses(&flow_state, Some(&venue_health), config.enabled, now);
    let trend_symbols = ["BTC", "ETH"];
    let trend_60s_list = trend_symbols
        .iter()
        .map(|symbol| {
            let symbol_flow_state = state.flow_state_for_symbol(symbol);
            let symbol_now = if symbol_flow_state.updated_at > 0 {
                symbol_flow_state.updated_at
            } else {
                now
            };
            trend_60s_from_flow_state(&symbol_flow_state, symbol, symbol_now)
        })
        .collect::<Vec<_>>();
    let items = state
        .contract_whale_store()
        .and_then(|store| {
            store
                .query_contract_whale_signals(&ContractWhaleSignalQuery {
                    limit: 200,
                    ..ContractWhaleSignalQuery::default()
                })
                .ok()
        })
        .unwrap_or_else(|| {
            build_contract_whale_response_with_runtime(
                &flow_state,
                "BTC",
                200,
                None,
                config.enabled,
                config.dry_run,
                Some(&venue_health),
            )
            .items
        });
    let body = build_contract_whale_metrics_text_with_trends(
        config.enabled,
        &exchanges,
        &trend_60s_list,
        &items,
    );
    (
        [(
            header::CONTENT_TYPE,
            "text/plain; version=0.0.4; charset=utf-8",
        )],
        body,
    )
}

pub async fn contract_whale_pipeline_debug_route(
    State(state): State<AppState>,
    Query(query): Query<ContractWhaleQuery>,
) -> ApiJsonResult {
    let response = contract_whale_pipeline_debug_for_query(state, query);
    Ok(Json(serde_json::to_value(response).unwrap_or_else(|_| {
        serde_json::json!({
            "symbol": "BTC",
            "range": "24h",
            "error": "serialize_failed"
        })
    })))
}

pub async fn contract_whale_raw_flow_debug_route(
    State(state): State<AppState>,
    Query(query): Query<ContractWhaleQuery>,
) -> ApiJsonResult {
    let response = contract_whale_raw_flow_debug_for_query(state, query);
    Ok(Json(serde_json::to_value(response).unwrap_or_else(|_| {
        serde_json::json!({
            "symbol": "BTC",
            "range": "24h",
            "error": "serialize_failed"
        })
    })))
}

pub async fn contract_whale_latency_debug_route(
    State(state): State<AppState>,
    Query(query): Query<ContractWhaleQuery>,
) -> ApiJsonResult {
    let symbol = parse_symbol_for_latest(query.symbol.as_deref())?;
    let range = query.range.clone().unwrap_or_else(|| "24h".to_string());
    let stale_after_ts = parse_range_start_ms(Some(&range))?;
    let now = now_ms();

    let latest_rows = state
        .contract_whale_store()
        .and_then(|store| {
            store
                .query_contract_whale_signals(&ContractWhaleSignalQuery {
                    symbol: Some(symbol.clone()),
                    limit: 50,
                    ..ContractWhaleSignalQuery::default()
                })
                .ok()
        })
        .unwrap_or_default();
    let latest_debug = build_pipeline_latest_debug(&latest_rows, stale_after_ts, Some(&range), now);
    let latest_max_ts = latest_debug.items.iter().map(|item| item.ts).max();
    let latest_age_sec = latest_max_ts
        .map(|ts| now.saturating_sub(ts).max(0).saturating_div(1000))
        .unwrap_or(0);

    let contract_events = contract_event_page_for_query(
        state.clone(),
        ContractWhaleQuery {
            symbol: Some(symbol.clone()),
            range: Some(range.clone()),
            limit: Some("100".to_string()),
            include_hidden: Some("true".to_string()),
            ..ContractWhaleQuery::default()
        },
    )?;
    let final_events = final_events_v2_for_query(
        state.clone(),
        ContractWhaleQuery {
            symbol: Some(symbol.clone()),
            range: Some(range.clone()),
            limit: Some("100".to_string()),
            ..ContractWhaleQuery::default()
        },
    )?;
    let flow_state = state.flow_state_for_symbol(&symbol);
    let flow_updated_at = (flow_state.updated_at > 0).then_some(flow_state.updated_at);
    let flow_lag_sec = flow_updated_at
        .map(|ts| now.saturating_sub(ts).max(0).saturating_div(1000))
        .unwrap_or(0);
    let diagnosis = diagnose_contract_whale_latency(
        latest_max_ts,
        contract_events.max_event_ts,
        final_events.max_event_ts,
        flow_updated_at,
        now,
    );

    Ok(Json(
        serde_json::to_value(ContractWhaleLatencyDebugResponse {
            symbol: symbol.clone(),
            range: range.clone(),
            server_time: now,
            latest: ContractWhaleLatencyLatestDebug {
                count: latest_debug.latest_count,
                max_ts: latest_max_ts,
                age_sec: latest_age_sec,
                stale_count: latest_debug.stale_count,
            },
            contract_events: ContractWhaleLatencyLayerDebug {
                count: contract_events.items.len(),
                max_event_ts: contract_events.max_event_ts,
                lag_sec: contract_events.history_lag_sec,
                lag_vs_latest_sec: contract_events.latest_lag_sec,
                cache_age_sec: contract_events.cache_age_sec,
                cache_ttl_sec: contract_events.cache_ttl_sec,
            },
            final_events_v2: ContractWhaleLatencyProjectionDebug {
                active_count: final_events.active.len(),
                closed_count: final_events.closed.len(),
                max_event_ts: final_events.max_event_ts,
                projection_lag_sec: latest_max_ts
                    .zip(final_events.max_event_ts)
                    .map(|(latest, projection)| {
                        latest
                            .saturating_sub(projection)
                            .max(0)
                            .saturating_div(1000)
                    })
                    .unwrap_or(final_events.projection_lag_sec),
                cache_age_sec: final_events.cache_age_sec,
                cache_ttl_sec: final_events.cache_ttl_sec,
                generated_at: Some(final_events.generated_at),
            },
            flow: ContractWhaleLatencyFlowDebug {
                updated_at: flow_updated_at,
                flow_lag_sec,
            },
            diagnosis,
        })
        .unwrap_or_else(|_| {
            serde_json::json!({
                "symbol": symbol,
                "range": range,
                "serverTime": now,
                "error": "serialize_failed"
            })
        }),
    ))
}

fn contract_whale_pipeline_debug_for_query(
    state: AppState,
    query: ContractWhaleQuery,
) -> ContractWhalePipelineDebugResponse {
    let symbol = match parse_symbol_for_latest(query.symbol.as_deref()) {
        Ok(value) => value,
        Err(_) => {
            return ContractWhalePipelineDebugResponse {
                symbol: "BTC".to_string(),
                range: query.range.unwrap_or_else(|| "24h".to_string()),
                error: Some("bad_request".to_string()),
                ..ContractWhalePipelineDebugResponse::default()
            };
        }
    };
    let history_query = match parse_history_query(&query) {
        Ok(value) => value,
        Err(_) => {
            return ContractWhalePipelineDebugResponse {
                symbol,
                range: query.range.unwrap_or_else(|| "24h".to_string()),
                error: Some("bad_request".to_string()),
                ..ContractWhalePipelineDebugResponse::default()
            };
        }
    };
    let range = query.range.unwrap_or_else(|| "24h".to_string());
    let now = history_query.to_ts.unwrap_or_else(now_ms);
    let from_ts = history_query
        .from_ts
        .unwrap_or_else(|| now.saturating_sub(24 * 60 * 60 * 1000));
    let Some(store) = state.contract_whale_store() else {
        return ContractWhalePipelineDebugResponse {
            symbol,
            range,
            error: Some("query_failed".to_string()),
            ..ContractWhalePipelineDebugResponse::default()
        };
    };

    let raw_flow_buckets = store
        .list_contract_flow_buckets_between(&symbol, from_ts, now)
        .unwrap_or_default();
    let liquidation_buckets = store
        .list_contract_liquidation_buckets_between(&symbol, from_ts, now)
        .unwrap_or_default();
    let oi_snapshots = store
        .list_contract_oi_snapshots_between(&symbol, from_ts, now)
        .unwrap_or_default();
    let funding_snapshots = store
        .list_contract_funding_snapshots_between(&symbol, from_ts, now)
        .unwrap_or_default();
    let history_rows = store
        .query_contract_whale_signals(&history_query)
        .unwrap_or_default();
    let latest_rows = store
        .query_contract_whale_signals(&ContractWhaleSignalQuery {
            symbol: Some(symbol.clone()),
            limit: 10,
            ..ContractWhaleSignalQuery::default()
        })
        .unwrap_or_default();

    let raw_flow = summarize_pipeline_raw_flow(&raw_flow_buckets);
    let latest = build_pipeline_latest_debug(&latest_rows, Some(from_ts), Some(&range), now);
    let (rolling_windows, detector, accepted_signals) = replay_pipeline_detector_debug(
        &store,
        &symbol,
        from_ts,
        now,
        &raw_flow_buckets,
        &liquidation_buckets,
        &oi_snapshots,
        &funding_snapshots,
    );
    let persistence = project_pipeline_persistence_debug(accepted_signals, history_rows.len());

    tracing::info!(
        target: CWM_LOG_TARGET,
        event = log_events::SIGNAL_GENERATED,
        symbol = symbol.as_str(),
        range = range.as_str(),
        raw_flow_rows = raw_flow.flow_1s_rows,
        detector_input_windows = detector.input_windows,
        detector_accepted = detector.accepted,
        detector_rejected = detector.rejected,
        history_rows = history_rows.len(),
        stale_latest_count = latest.stale_count,
        latest_max_age_sec = latest.max_age_sec,
        "{} pipeline debug snapshot",
        CWM_LOG_PREFIX
    );

    ContractWhalePipelineDebugResponse {
        symbol,
        range,
        raw_flow,
        rolling_windows,
        detector,
        persistence,
        history: PipelineHistoryDebug {
            contract_whale_signals_rows: history_rows.len(),
        },
        latest,
        error: None,
    }
}

fn contract_whale_raw_flow_debug_for_query(
    state: AppState,
    query: ContractWhaleQuery,
) -> ContractWhaleRawFlowDebugResponse {
    let symbol = match parse_symbol_for_latest(query.symbol.as_deref()) {
        Ok(value) => value,
        Err(_) => {
            return ContractWhaleRawFlowDebugResponse {
                symbol: "BTC".to_string(),
                range: query.range.unwrap_or_else(|| "24h".to_string()),
                error: Some("bad_request".to_string()),
                ..ContractWhaleRawFlowDebugResponse::default()
            };
        }
    };
    let history_query = match parse_history_query(&query) {
        Ok(value) => value,
        Err(_) => {
            return ContractWhaleRawFlowDebugResponse {
                symbol,
                range: query.range.unwrap_or_else(|| "24h".to_string()),
                error: Some("bad_request".to_string()),
                ..ContractWhaleRawFlowDebugResponse::default()
            };
        }
    };
    let range = query.range.unwrap_or_else(|| "24h".to_string());
    let now = history_query.to_ts.unwrap_or_else(now_ms);
    let from_ts = history_query
        .from_ts
        .unwrap_or_else(|| now.saturating_sub(24 * 60 * 60 * 1000));
    let Some(store) = state.contract_whale_store() else {
        return ContractWhaleRawFlowDebugResponse {
            symbol,
            range,
            error: Some("query_failed".to_string()),
            ..ContractWhaleRawFlowDebugResponse::default()
        };
    };

    let runtime = contract_whale_runtime_config();
    let app_requested_symbol = state.config().symbol.clone();
    let config =
        build_raw_flow_config_debug(&symbol, &app_requested_symbol, state.config(), &runtime);
    let venue_health = state.venue_health();
    let raw_trade_ingest = build_raw_trade_ingest_debug(&venue_health);
    let normalizer = build_raw_flow_normalizer_debug(&symbol, &app_requested_symbol);
    let aggregator = build_raw_flow_aggregator_debug(&state.flow_state_for_symbol(&symbol));
    let contract_flow_1s =
        query_raw_flow_persistence_debug(&store, &symbol, from_ts, now).unwrap_or_default();
    let diagnosis = build_raw_flow_diagnosis(
        &config,
        &raw_trade_ingest,
        &normalizer,
        &aggregator,
        &contract_flow_1s,
    );

    tracing::info!(
        target: CWM_LOG_TARGET,
        event = log_events::BUCKET_FLUSHED,
        symbol = symbol.as_str(),
        range = range.as_str(),
        app_requested_symbol = config.app_requested_symbol.as_str(),
        connector_symbol_mismatch = normalizer.connector_symbol_mismatch,
        total_trade_messages = raw_trade_ingest.total_trade_messages,
        flow_state_symbol = aggregator.flow_state_symbol.as_str(),
        exact_symbol_rows = contract_flow_1s.exact_symbol_rows,
        sibling_symbol_rows = contract_flow_1s.sibling_symbol_rows,
        diagnosis = diagnosis.primary_reason.as_str(),
        "{} raw flow debug snapshot",
        CWM_LOG_PREFIX
    );

    ContractWhaleRawFlowDebugResponse {
        symbol,
        range,
        config,
        raw_trade_ingest,
        normalizer,
        aggregator,
        contract_flow_1s,
        diagnosis,
        error: None,
    }
}

fn summarize_pipeline_raw_flow(buckets: &[ContractFlowBucket]) -> PipelineRawFlowDebug {
    let buy_volume_btc = buckets
        .iter()
        .map(|bucket| bucket.buy_volume_btc)
        .sum::<f64>();
    let sell_volume_btc = buckets
        .iter()
        .map(|bucket| bucket.sell_volume_btc)
        .sum::<f64>();
    PipelineRawFlowDebug {
        flow_1s_rows: buckets.len(),
        oldest_ts: buckets.iter().map(|bucket| bucket.ts_bucket).min(),
        newest_ts: buckets.iter().map(|bucket| bucket.ts_bucket).max(),
        buy_volume_btc: round_for_route(buy_volume_btc, 3),
        sell_volume_btc: round_for_route(sell_volume_btc, 3),
        total_volume_btc: round_for_route(buy_volume_btc + sell_volume_btc, 3),
    }
}

fn build_raw_flow_config_debug(
    query_symbol: &str,
    app_requested_symbol: &str,
    app_config: &AppConfig,
    runtime: &crate::contract_whale_monitor::config::ContractWhaleRuntimeConfig,
) -> RawFlowConfigDebug {
    let mut runtime_enabled_symbols = runtime
        .symbols
        .iter()
        .filter(|(_, symbol_config)| symbol_config.enabled)
        .map(|(symbol, _)| symbol.clone())
        .collect::<Vec<_>>();
    runtime_enabled_symbols.sort();
    RawFlowConfigDebug {
        app_requested_symbol: app_requested_symbol.to_string(),
        app_requested_symbol_base: symbol_base_prefix(app_requested_symbol),
        query_symbol: query_symbol.to_string(),
        query_symbol_enabled: runtime.symbol_enabled(query_symbol),
        runtime_enabled: app_config.contract_whale_monitor.enabled,
        runtime_dry_run: app_config.contract_whale_monitor.dry_run,
        runtime_enabled_symbols,
        windows_sec: app_config
            .windows_ms
            .iter()
            .copied()
            .filter(|window_ms| matches!(*window_ms, 5_000 | 15_000 | 60_000))
            .map(|window_ms| window_ms / 1000)
            .collect(),
    }
}

fn build_raw_trade_ingest_debug(venue_health: &VenueHealthMap) -> RawTradeIngestDebug {
    let mut exchanges = venue_health
        .values()
        .map(|health| RawTradeIngestVenueDebug {
            venue: health.venue.as_key().to_string(),
            requested_symbol: health.requested_symbol.clone(),
            venue_symbol: health.venue_symbol.clone(),
            symbol_mapping_status: health.symbol_mapping_status.clone(),
            symbol_mapping_error: health.symbol_mapping_error.clone(),
            ws_connected: health.ws_connected,
            trade_subscribe_acked: health.trade_subscribe_acked,
            trade_message_count: health.trade_message_count,
            last_trade_ts: health.last_trade_ts,
            trade_active: health.trade_active,
            activity_status: health.activity_status.clone(),
        })
        .collect::<Vec<_>>();
    exchanges.sort_by(|left, right| left.venue.cmp(&right.venue));
    RawTradeIngestDebug {
        venue_count: exchanges.len(),
        ws_connected_count: exchanges.iter().filter(|item| item.ws_connected).count(),
        trade_active_count: exchanges.iter().filter(|item| item.trade_active).count(),
        total_trade_messages: exchanges.iter().map(|item| item.trade_message_count).sum(),
        exchanges,
    }
}

fn build_raw_flow_normalizer_debug(
    query_symbol: &str,
    app_requested_symbol: &str,
) -> RawFlowNormalizerDebug {
    let app_requested_symbol_base = symbol_base_prefix(app_requested_symbol);
    let normalized_query_symbol = symbol_base_prefix(query_symbol);
    let query_venue_symbols = [
        ("binance", "BTC"),
        ("bybit", "BTC"),
        ("okx", "BTC"),
        ("bitfinex", "BTC"),
    ]
    .into_iter()
    .map(|(venue_key, _)| {
        let venue = match venue_key {
            "binance" => crate::types::market::Venue::Binance,
            "bybit" => crate::types::market::Venue::Bybit,
            "okx" => crate::types::market::Venue::Okx,
            _ => crate::types::market::Venue::Bitfinex,
        };
        (
            venue_key.to_string(),
            crate::types::market::venue_symbol_mapping(venue, query_symbol)
                .venue_symbol
                .unwrap_or_else(|| "unmapped".to_string()),
        )
    })
    .collect::<BTreeMap<_, _>>();
    RawFlowNormalizerDebug {
        query_symbol: query_symbol.to_string(),
        normalized_query_symbol: normalized_query_symbol.clone(),
        app_requested_symbol: app_requested_symbol.to_string(),
        app_requested_symbol_base: app_requested_symbol_base.clone(),
        app_requested_symbol_matches_query: app_requested_symbol_base == normalized_query_symbol,
        connector_symbol_mismatch: app_requested_symbol_base != normalized_query_symbol,
        query_venue_symbols,
    }
}

fn build_raw_flow_aggregator_debug(flow_state: &FlowState) -> RawFlowAggregatorDebug {
    let windows = flow_state
        .windows
        .iter()
        .filter_map(|(window_key, window)| {
            let seconds = window_key.parse::<u64>().ok()?.saturating_div(1000);
            if !matches!(seconds, 5 | 15 | 60) {
                return None;
            }
            Some((
                format!("{seconds}s"),
                RawFlowAggregatorWindowDebug {
                    trade_count: window.trade_count,
                    buy_volume_btc: round_for_route(window.aggressive_buy_btc, 3),
                    sell_volume_btc: round_for_route(window.aggressive_sell_btc, 3),
                    total_volume_btc: round_for_route(window.abs_aggressive_btc, 3),
                    active_venues: window.data_quality.active_venues.clone(),
                    stale_venues: window.data_quality.stale_venues.clone(),
                    has_trades: window.trade_count > 0,
                },
            ))
        })
        .collect::<BTreeMap<_, _>>();
    RawFlowAggregatorDebug {
        flow_state_symbol: flow_state.symbol.clone(),
        updated_at: flow_state.updated_at,
        windows,
    }
}

fn query_raw_flow_persistence_debug(
    store: &crate::storage::SqliteStore,
    query_symbol: &str,
    from_ts: i64,
    to_ts: i64,
) -> anyhow::Result<RawFlowPersistenceDebug> {
    let query_prefix = format!("{}%", symbol_base_prefix(query_symbol));
    store.with_connection(|conn| {
        let exact_symbol_rows = conn.query_row(
            r#"
            SELECT COUNT(*)
            FROM contract_flow_1s
            WHERE market_type = 'perp'
              AND symbol = ?1
              AND ts_bucket BETWEEN ?2 AND ?3
            "#,
            rusqlite::params![query_symbol, from_ts, to_ts],
            |row| row.get::<_, i64>(0),
        )? as usize;
        let sibling_symbol_rows = conn.query_row(
            r#"
            SELECT COUNT(*)
            FROM contract_flow_1s
            WHERE market_type = 'perp'
              AND ts_bucket BETWEEN ?2 AND ?3
              AND (
                    symbol LIKE ?1
                 OR product_id LIKE ?1
              )
              AND symbol <> ?4
            "#,
            rusqlite::params![query_prefix, from_ts, to_ts, query_symbol],
            |row| row.get::<_, i64>(0),
        )? as usize;
        let (oldest_ts, newest_ts, buy_volume_btc, sell_volume_btc): (
            Option<i64>,
            Option<i64>,
            Option<f64>,
            Option<f64>,
        ) = conn.query_row(
            r#"
            SELECT MIN(ts_bucket), MAX(ts_bucket), SUM(buy_volume_btc), SUM(sell_volume_btc)
            FROM contract_flow_1s
            WHERE market_type = 'perp'
              AND symbol = ?1
              AND ts_bucket BETWEEN ?2 AND ?3
            "#,
            rusqlite::params![query_symbol, from_ts, to_ts],
            |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?, row.get(3)?)),
        )?;

        let mut distinct_symbols_stmt = conn.prepare(
            r#"
            SELECT DISTINCT symbol
            FROM contract_flow_1s
            WHERE market_type = 'perp'
              AND ts_bucket BETWEEN ?2 AND ?3
              AND (
                    symbol LIKE ?1
                 OR product_id LIKE ?1
              )
            ORDER BY symbol ASC
            "#,
        )?;
        let distinct_symbols = distinct_symbols_stmt
            .query_map(rusqlite::params![query_prefix, from_ts, to_ts], |row| {
                row.get::<_, String>(0)
            })?
            .filter_map(Result::ok)
            .collect::<Vec<_>>();

        let mut distinct_product_ids_stmt = conn.prepare(
            r#"
            SELECT DISTINCT product_id
            FROM contract_flow_1s
            WHERE market_type = 'perp'
              AND ts_bucket BETWEEN ?2 AND ?3
              AND product_id IS NOT NULL
              AND (
                    symbol LIKE ?1
                 OR product_id LIKE ?1
              )
            ORDER BY product_id ASC
            "#,
        )?;
        let distinct_product_ids = distinct_product_ids_stmt
            .query_map(rusqlite::params![query_prefix, from_ts, to_ts], |row| {
                row.get::<_, String>(0)
            })?
            .filter_map(Result::ok)
            .collect::<Vec<_>>();

        let buy_volume_btc = buy_volume_btc.unwrap_or(0.0);
        let sell_volume_btc = sell_volume_btc.unwrap_or(0.0);
        Ok(RawFlowPersistenceDebug {
            exact_symbol_rows,
            sibling_symbol_rows,
            oldest_ts,
            newest_ts,
            buy_volume_btc: round_for_route(buy_volume_btc, 3),
            sell_volume_btc: round_for_route(sell_volume_btc, 3),
            total_volume_btc: round_for_route(buy_volume_btc + sell_volume_btc, 3),
            distinct_symbols,
            distinct_product_ids,
        })
    })
}

fn build_raw_flow_diagnosis(
    config: &RawFlowConfigDebug,
    raw_trade_ingest: &RawTradeIngestDebug,
    normalizer: &RawFlowNormalizerDebug,
    aggregator: &RawFlowAggregatorDebug,
    contract_flow_1s: &RawFlowPersistenceDebug,
) -> RawFlowDiagnosisDebug {
    let window_has_trades = aggregator
        .windows
        .values()
        .any(|window| window.trade_count > 0);
    let mut details = Vec::new();

    if normalizer.connector_symbol_mismatch && contract_flow_1s.exact_symbol_rows == 0 {
        details.push(format!(
            "connector requested {} while query symbol is {}",
            config.app_requested_symbol, config.query_symbol
        ));
        if contract_flow_1s.sibling_symbol_rows > 0 {
            details.push(format!(
                "contract_flow_1s has {} sibling rows under {}-like symbols/products",
                contract_flow_1s.sibling_symbol_rows, config.query_symbol
            ));
        }
        return RawFlowDiagnosisDebug {
            status: "upstream_no_raw_flow".to_string(),
            primary_reason: "connector_requested_symbol_mismatch".to_string(),
            details,
        };
    }

    if raw_trade_ingest.total_trade_messages == 0 && contract_flow_1s.exact_symbol_rows == 0 {
        details.push("venue health reports zero parsed trade messages".to_string());
        return RawFlowDiagnosisDebug {
            status: "upstream_no_raw_flow".to_string(),
            primary_reason: "no_trade_ingest_activity".to_string(),
            details,
        };
    }

    if raw_trade_ingest.total_trade_messages > 0
        && !window_has_trades
        && contract_flow_1s.exact_symbol_rows == 0
    {
        details.push("trade messages exist but 5s/15s/60s flow windows are empty".to_string());
        return RawFlowDiagnosisDebug {
            status: "upstream_no_raw_flow".to_string(),
            primary_reason: "aggregator_not_producing_symbol_flow".to_string(),
            details,
        };
    }

    if window_has_trades && contract_flow_1s.exact_symbol_rows == 0 {
        details.push(
            "rolling windows have trades but contract_flow_1s has no matching persisted rows"
                .to_string(),
        );
        return RawFlowDiagnosisDebug {
            status: "upstream_no_raw_flow".to_string(),
            primary_reason: "contract_flow_not_persisted".to_string(),
            details,
        };
    }

    if contract_flow_1s.exact_symbol_rows > 0 {
        details.push(format!(
            "contract_flow_1s contains {} exact {} rows in requested range",
            contract_flow_1s.exact_symbol_rows, config.query_symbol
        ));
        return RawFlowDiagnosisDebug {
            status: "raw_flow_available".to_string(),
            primary_reason: "raw_flow_present".to_string(),
            details,
        };
    }

    RawFlowDiagnosisDebug {
        status: "upstream_no_raw_flow".to_string(),
        primary_reason: "unknown".to_string(),
        details: vec!["raw flow unavailable but no dominant reason inferred".to_string()],
    }
}

fn symbol_base_prefix(symbol: &str) -> String {
    let upper = symbol.trim().to_ascii_uppercase();
    let head = upper
        .split([':', '/', '_'])
        .next()
        .unwrap_or(upper.as_str())
        .split('-')
        .next()
        .unwrap_or(upper.as_str())
        .to_string();
    ["PERP", "USDT", "USD", "F0"]
        .into_iter()
        .fold(head, |current, suffix| {
            current
                .strip_suffix(suffix)
                .map(str::to_string)
                .unwrap_or(current)
        })
}

fn replay_pipeline_detector_debug(
    store: &impl ContractWhaleRepo,
    symbol: &str,
    from_ts: i64,
    to_ts: i64,
    raw_flow_buckets: &[ContractFlowBucket],
    liquidation_buckets: &[crate::contract_whale_monitor::types::ContractLiquidationBucket],
    oi_snapshots: &[crate::contract_whale_monitor::types::ContractOiSnapshot],
    funding_snapshots: &[crate::contract_whale_monitor::types::ContractFundingSnapshot],
) -> (
    BTreeMap<String, PipelineRollingWindowDebug>,
    PipelineDetectorDebug,
    Vec<ContractWhaleSignal>,
) {
    let config = contract_whale_runtime_config();
    let min_dynamic_samples = config.data_quality.min_dynamic_samples;
    let lookback_from = to_ts.saturating_sub(7 * 24 * 60 * 60 * 1000);
    let baseline_buckets = store
        .list_contract_flow_buckets_between(symbol, lookback_from, to_ts)
        .unwrap_or_else(|_| raw_flow_buckets.to_vec());
    let mut rolling_windows = BTreeMap::new();
    let mut detector = PipelineDetectorDebug::default();
    let mut accepted_signals = Vec::new();

    for window_sec in [5_u64, 15, 60] {
        let threshold = compute_percentile_threshold(
            &baseline_buckets,
            symbol,
            "all",
            window_sec,
            lookback_from,
            to_ts,
            to_ts,
        );
        let timestamps = detector_window_timestamps(raw_flow_buckets, symbol, from_ts, to_ts);
        let mut window_debug = PipelineRollingWindowDebug::default();

        for now_ts in timestamps.iter().copied() {
            let current_price_move_pct =
                price_move_pct_for_window(raw_flow_buckets, symbol, window_sec, now_ts);
            let dynamic_to = now_ts.saturating_sub((window_sec as i64).saturating_mul(1000));
            let dynamic_from = dynamic_to.saturating_sub(60 * 60 * 1000);
            let dynamic_baseline_btc = historical_window_average_btc_with_min_samples(
                &baseline_buckets,
                symbol,
                window_sec,
                dynamic_from,
                dynamic_to,
                min_dynamic_samples,
            );
            let stats = rolling_window_stats_with_config(
                raw_flow_buckets,
                symbol,
                window_sec,
                now_ts,
                RollingWindowStatsOptions {
                    price_move_pct: current_price_move_pct,
                    dynamic_multiple: None,
                    dynamic_baseline_btc,
                    dynamic_threshold_level: String::new(),
                    data_quality: 0,
                    config: &config,
                },
            );
            let Some(mut stats) = stats else {
                continue;
            };
            let dynamic_multiple =
                dynamic_multiple_for_volume(stats.total_volume_btc, dynamic_baseline_btc);
            let percentile_level =
                percentile_level_for_volume(stats.total_volume_btc, threshold.as_ref());
            stats.dynamic_multiple = dynamic_multiple;
            stats.dynamic_baseline_btc = dynamic_baseline_btc;
            stats.dynamic_threshold_level =
                dynamic_threshold_level(dynamic_multiple, percentile_level);
            stats.percentile_level = percentile_level;
            stats.data_quality = detector_data_quality(stats.exchange_count);
            stats.liquidation_context = liquidation_context_for_window(
                liquidation_buckets,
                symbol,
                window_sec,
                now_ts,
                stats.total_volume_btc,
            );
            stats.market_context =
                market_context_from_snapshots(oi_snapshots, funding_snapshots, symbol, now_ts);
            detector.input_windows += 1;
            detector.candidates += 1;
            window_debug.windows += 1;
            window_debug.max_total_volume_btc = window_debug
                .max_total_volume_btc
                .max(stats.total_volume_btc);

            let decision = inspect_contract_whale_signal_with_config(&stats, &config);
            if let Some(signal) = decision.signal {
                detector.accepted += 1;
                accepted_signals.push(signal);
            } else {
                detector.rejected += 1;
                let key = detector_reject_reason_key(decision.reject_reason);
                *detector.reject_reasons.entry(key).or_default() += 1;
            }
        }

        window_debug.max_total_volume_btc = round_for_route(window_debug.max_total_volume_btc, 3);
        rolling_windows.insert(format!("{window_sec}s"), window_debug);
    }

    (rolling_windows, detector, accepted_signals)
}

fn detector_window_timestamps(
    buckets: &[ContractFlowBucket],
    symbol: &str,
    from_ts: i64,
    to_ts: i64,
) -> Vec<i64> {
    let mut timestamps = buckets
        .iter()
        .filter(|bucket| bucket.symbol.eq_ignore_ascii_case(symbol))
        .map(|bucket| bucket.ts_bucket)
        .filter(|ts| *ts >= from_ts && *ts <= to_ts)
        .collect::<Vec<_>>();
    timestamps.sort_unstable();
    timestamps.dedup();
    timestamps
}

fn detector_data_quality(exchange_count: usize) -> u8 {
    match exchange_count {
        0 => 40,
        1 => 70,
        _ => 85,
    }
}

fn price_move_pct_for_window(
    buckets: &[ContractFlowBucket],
    symbol: &str,
    window_sec: u64,
    now_ts: i64,
) -> Option<f64> {
    let start_ts = now_ts.saturating_sub((window_sec as i64).saturating_mul(1000));
    let mut prices = buckets
        .iter()
        .filter(|bucket| bucket.symbol.eq_ignore_ascii_case(symbol))
        .filter(|bucket| bucket.ts_bucket >= start_ts && bucket.ts_bucket <= now_ts)
        .filter_map(|bucket| bucket.vwap.map(|price| (bucket.ts_bucket, price)))
        .collect::<Vec<_>>();
    prices.sort_by_key(|(ts, _)| *ts);
    let (_, first) = prices.first().copied()?;
    let (_, last) = prices.last().copied()?;
    if first <= f64::EPSILON || !first.is_finite() || !last.is_finite() {
        return None;
    }
    Some(((last - first) / first) * 100.0)
}

fn detector_reject_reason_key(reason: Option<ContractWhaleDetectorRejectReason>) -> String {
    match reason.unwrap_or(ContractWhaleDetectorRejectReason::Unknown) {
        ContractWhaleDetectorRejectReason::BelowVolumeThreshold
        | ContractWhaleDetectorRejectReason::ZeroVolume => "below_volume_threshold".to_string(),
        ContractWhaleDetectorRejectReason::BelowNotionalThreshold => {
            "below_notional_threshold".to_string()
        }
        ContractWhaleDetectorRejectReason::DynamicMultipleTooLow => {
            "dynamic_multiple_too_low".to_string()
        }
        ContractWhaleDetectorRejectReason::PercentileTooLow => "percentile_too_low".to_string(),
        ContractWhaleDetectorRejectReason::DominanceTooLow => "dominance_too_low".to_string(),
        ContractWhaleDetectorRejectReason::DataQualityTooLow => "data_quality_too_low".to_string(),
        ContractWhaleDetectorRejectReason::Warmup => "warmup".to_string(),
        ContractWhaleDetectorRejectReason::MultiExchangeNotConfirmed => {
            "multi_exchange_not_confirmed".to_string()
        }
        ContractWhaleDetectorRejectReason::SameDirectionPriceMoveTooLow => {
            "same_direction_price_move_too_low".to_string()
        }
        _ => "unknown".to_string(),
    }
}

fn project_pipeline_persistence_debug(
    accepted_signals: Vec<ContractWhaleSignal>,
    persisted_history_rows: usize,
) -> PipelinePersistenceDebug {
    if accepted_signals.is_empty() {
        return PipelinePersistenceDebug::default();
    }
    let mut merged = merge_contract_whale_signals(accepted_signals);
    let reference_now = merged
        .iter()
        .map(|signal| signal.ts)
        .max()
        .unwrap_or_else(now_ms);
    decorate_and_filter_price_deviated_signals(
        &mut merged,
        None,
        contract_whale_runtime_config()
            .toxic_order
            .max_price_deviation_pct,
    );
    let price_filtered = merged
        .iter()
        .filter(|signal| signal.price_deviation_filtered)
        .count();
    merged = apply_contract_whale_event_lifecycle(merged, reference_now);
    let quality_decorated = decorate_contract_whale_event_quality(merged);
    let quality_filtered = quality_decorated
        .iter()
        .filter(|signal| !signal.event_quality.valid)
        .count();
    let persist_attempts = quality_decorated
        .iter()
        .filter(|signal| !signal.price_deviation_filtered && signal.event_quality.valid)
        .count();
    let persist_success = persisted_history_rows.min(persist_attempts);
    let mut skip_reasons = BTreeMap::new();
    if price_filtered > 0 {
        skip_reasons.insert("price_deviation_filtered".to_string(), price_filtered);
    }
    if quality_filtered > 0 {
        skip_reasons.insert("bad_quality".to_string(), quality_filtered);
    }
    PipelinePersistenceDebug {
        persist_attempts,
        persist_success,
        persist_skipped: price_filtered + quality_filtered,
        skip_reasons,
        persist_errors: 0,
    }
}

fn build_pipeline_latest_debug(
    latest_rows: &[ContractWhaleSignal],
    stale_after_ts: Option<i64>,
    range_label: Option<&str>,
    now_ts: i64,
) -> PipelineLatestDebug {
    let items = latest_rows
        .iter()
        .map(|signal| stale_debug_item(signal, stale_after_ts, range_label, now_ts))
        .collect::<Vec<_>>();
    let stale_count = items.iter().filter(|item| item.is_stale).count();
    let max_age_sec = items.iter().map(|item| item.age_sec).max().unwrap_or(0);
    PipelineLatestDebug {
        latest_count: items.len(),
        stale_count,
        max_age_sec,
        items,
    }
}

fn stale_debug_item(
    signal: &ContractWhaleSignal,
    stale_after_ts: Option<i64>,
    range_label: Option<&str>,
    now_ts: i64,
) -> PipelineLatestItemDebug {
    let age_sec = now_ts.saturating_sub(signal.ts).max(0).saturating_div(1000);
    let is_stale = stale_after_ts.is_some_and(|cutoff| signal.ts < cutoff);
    PipelineLatestItemDebug {
        event_id: if signal.event_lifecycle.event_id.is_empty() {
            signal.id.clone()
        } else {
            signal.event_lifecycle.event_id.clone()
        },
        ts: signal.ts,
        age_sec,
        is_stale,
        stale_reason: is_stale.then(|| stale_reason_for_range(range_label)),
    }
}

fn stale_reason_for_range(range_label: Option<&str>) -> String {
    match range_label
        .unwrap_or("24h")
        .trim()
        .to_ascii_lowercase()
        .as_str()
    {
        "15m" => "older_than_15m".to_string(),
        "1h" => "older_than_1h".to_string(),
        "4h" => "older_than_4h".to_string(),
        "24h" => "older_than_24h".to_string(),
        "7d" => "older_than_7d".to_string(),
        _ => "older_than_requested_range".to_string(),
    }
}

fn diagnose_contract_whale_latency(
    latest_max_ts: Option<i64>,
    history_max_ts: Option<i64>,
    final_max_ts: Option<i64>,
    flow_updated_at: Option<i64>,
    now: i64,
) -> ContractWhaleLatencyDiagnosis {
    let Some(latest_ts) = latest_max_ts else {
        return ContractWhaleLatencyDiagnosis {
            layer: "ok".to_string(),
            reason: "no_recent_signal".to_string(),
        };
    };
    if history_max_ts.is_none() {
        return ContractWhaleLatencyDiagnosis {
            layer: "history".to_string(),
            reason: "latest_ahead_of_history".to_string(),
        };
    }
    let history_ts = history_max_ts.unwrap_or(latest_ts);
    if latest_ts.saturating_sub(history_ts) > 15_000 {
        return ContractWhaleLatencyDiagnosis {
            layer: "history".to_string(),
            reason: "history_persist_lagging_latest".to_string(),
        };
    }
    if let Some(final_ts) = final_max_ts {
        if history_ts.saturating_sub(final_ts) > 15_000 {
            return ContractWhaleLatencyDiagnosis {
                layer: "final_events_v2".to_string(),
                reason: "projection_lagging_history".to_string(),
            };
        }
    }
    if let Some(flow_ts) = flow_updated_at {
        if now.saturating_sub(flow_ts) > 30_000 {
            return ContractWhaleLatencyDiagnosis {
                layer: "flow".to_string(),
                reason: "flow_state_stale".to_string(),
            };
        }
    }
    ContractWhaleLatencyDiagnosis {
        layer: "ok".to_string(),
        reason: "in_sync".to_string(),
    }
}

fn with_latest_stale_annotations(
    response: ContractWhaleLatestResponse,
    stale_after_ts: Option<i64>,
    range_label: Option<&str>,
    hide_stale: bool,
) -> serde_json::Value {
    let now = now_ms();
    let latest_debug =
        build_pipeline_latest_debug(&response.items, stale_after_ts, range_label, now);
    tracing::info!(
        target: CWM_LOG_TARGET,
        event = log_events::SIGNAL_GENERATED,
        latest_count = latest_debug.latest_count,
        stale_count = latest_debug.stale_count,
        max_age_sec = latest_debug.max_age_sec,
        "{} latest stale summary",
        CWM_LOG_PREFIX
    );
    let mut value = serde_json::to_value(response).unwrap_or_else(|_| serde_json::json!({}));
    let annotated_items = value
        .get_mut("items")
        .and_then(|items| items.as_array_mut())
        .map(|items| {
            let mut annotated = Vec::with_capacity(items.len());
            for (item, stale) in items.drain(..).zip(latest_debug.items.iter()) {
                if hide_stale && stale.is_stale {
                    continue;
                }
                let mut object = item.as_object().cloned().unwrap_or_default();
                object.insert("ageSec".to_string(), serde_json::json!(stale.age_sec));
                object.insert("isStale".to_string(), serde_json::json!(stale.is_stale));
                object.insert(
                    "staleReason".to_string(),
                    stale
                        .stale_reason
                        .as_ref()
                        .map(|reason| serde_json::json!(reason))
                        .unwrap_or(serde_json::Value::Null),
                );
                annotated.push(serde_json::Value::Object(object));
            }
            annotated
        })
        .unwrap_or_default();
    if let Some(items) = value
        .get_mut("items")
        .and_then(|items| items.as_array_mut())
    {
        *items = annotated_items;
    }
    let max_ts = latest_debug.items.iter().map(|item| item.ts).max();
    value["serverTime"] = serde_json::json!(now);
    value["maxTs"] = max_ts
        .map(serde_json::Value::from)
        .unwrap_or(serde_json::Value::Null);
    value["maxAgeSec"] = serde_json::json!(latest_debug.max_age_sec);
    value["staleCount"] = serde_json::json!(latest_debug.stale_count);
    value
}

pub fn build_contract_whale_metrics_text(
    enabled: bool,
    exchanges: &BTreeMap<String, ContractWhaleExchangeStatus>,
    trend_60s: &ContractWhaleTrend60s,
    signals: &[ContractWhaleSignal],
) -> String {
    build_contract_whale_metrics_text_with_trends(
        enabled,
        exchanges,
        std::slice::from_ref(trend_60s),
        signals,
    )
}

pub fn build_contract_whale_metrics_text_with_trends(
    enabled: bool,
    exchanges: &BTreeMap<String, ContractWhaleExchangeStatus>,
    trend_60s_list: &[ContractWhaleTrend60s],
    signals: &[ContractWhaleSignal],
) -> String {
    let fallback_trend = ContractWhaleTrend60s::default();
    let trend_60s = trend_60s_list.first().unwrap_or(&fallback_trend);
    let mut output = String::new();
    output.push_str("# HELP cwm_enabled Contract whale monitor enabled flag.\n");
    output.push_str("# TYPE cwm_enabled gauge\n");
    output.push_str(&format!("cwm_enabled {}\n", i32::from(enabled)));
    output.push_str("# HELP cwm_ws_connected WebSocket connection status by exchange.\n");
    output.push_str("# TYPE cwm_ws_connected gauge\n");
    for (exchange, status) in exchanges {
        output.push_str(&format!(
            "cwm_ws_connected{{exchange=\"{}\"}} {}\n",
            metric_label(exchange),
            i32::from(enabled && status.connected)
        ));
        output.push_str(&format!(
            "cwm_ws_reconnect_total{{exchange=\"{}\"}} {}\n",
            metric_label(exchange),
            status.reconnect_count
        ));
        output.push_str(&format!(
            "cwm_trades_received_total{{exchange=\"{}\"}} {}\n",
            metric_label(exchange),
            status.last_trade_at.map(|_| 1).unwrap_or(0)
        ));
        output.push_str(&format!(
            "cwm_trade_normalize_errors_total{{exchange=\"{}\"}} 0\n",
            metric_label(exchange)
        ));
    }
    output.push_str(&format!(
        "cwm_bucket_flush_total {}\n",
        i32::from(trend_60s.total_volume_btc > 0.0)
    ));
    output.push_str("cwm_bucket_flush_errors_total 0\n");

    let mut generated: BTreeMap<(String, String), usize> = BTreeMap::new();
    let mut skipped: BTreeMap<String, usize> = BTreeMap::new();
    let mut discord_sent_total = 0_usize;
    let mut data_quality_total = 0_u64;
    for signal in signals {
        *generated
            .entry((
                metric_label(&format!("{:?}", signal.severity).to_ascii_lowercase()),
                metric_label(&format!("{:?}", signal.signal_type).to_ascii_lowercase()),
            ))
            .or_default() += 1;
        if signal.discord_sent {
            discord_sent_total += 1;
        } else {
            *skipped
                .entry(metric_label(&signal.discord_reason))
                .or_default() += 1;
        }
        data_quality_total += signal.data_quality as u64;
    }
    for ((severity, signal_type), count) in generated {
        output.push_str(&format!(
            "cwm_signals_generated_total{{severity=\"{severity}\",type=\"{signal_type}\"}} {count}\n"
        ));
    }
    output.push_str(&format!("cwm_discord_sent_total {discord_sent_total}\n"));
    for (reason, count) in skipped {
        output.push_str(&format!(
            "cwm_discord_skipped_total{{reason=\"{reason}\"}} {count}\n"
        ));
    }
    let data_quality = if signals.is_empty() {
        0.0
    } else {
        data_quality_total as f64 / signals.len() as f64
    };
    output.push_str(&format!("cwm_data_quality {data_quality:.1}\n"));
    for trend_60s in trend_60s_list {
        append_trend_metrics(&mut output, trend_60s);
    }
    output.push_str(&format!(
        "cwm_trend_60s_buy_volume_btc {:.6}\n",
        trend_60s.buy_volume_btc
    ));
    output.push_str(&format!(
        "cwm_trend_60s_sell_volume_btc {:.6}\n",
        trend_60s.sell_volume_btc
    ));
    output
}

fn append_trend_metrics(output: &mut String, trend_60s: &ContractWhaleTrend60s) {
    let trend_symbol = metric_label(&trend_summary_unit(trend_60s));
    let trend_unit = metric_label(&trend_summary_unit(trend_60s));
    output.push_str(&format!(
        "cwm_trend_60s_buy_volume{{symbol=\"{trend_symbol}\",quantity_unit=\"{trend_unit}\"}} {:.6}\n",
        trend_60s.buy_volume
    ));
    output.push_str(&format!(
        "cwm_trend_60s_sell_volume{{symbol=\"{trend_symbol}\",quantity_unit=\"{trend_unit}\"}} {:.6}\n",
        trend_60s.sell_volume
    ));
    output.push_str(&format!(
        "cwm_trend_60s_total_volume{{symbol=\"{trend_symbol}\",quantity_unit=\"{trend_unit}\"}} {:.6}\n",
        trend_60s.total_volume
    ));
    output.push_str(&format!(
        "cwm_trend_60s_net_volume{{symbol=\"{trend_symbol}\",quantity_unit=\"{trend_unit}\"}} {:.6}\n",
        trend_60s.net_volume
    ));
}

pub fn build_contract_whale_response_with_runtime(
    flow_state: &FlowState,
    symbol: &str,
    limit: usize,
    severity: Option<&str>,
    enabled: bool,
    dry_run: bool,
    venue_health: Option<&VenueHealthMap>,
) -> ContractWhaleLatestResponse {
    build_contract_whale_response_with_runtime_and_baselines(
        flow_state,
        symbol,
        limit,
        severity,
        enabled,
        dry_run,
        ContractWhaleResponseRuntime {
            venue_health,
            baselines: &BTreeMap::new(),
            liquidations: &BTreeMap::new(),
            market_context: &ContractWhaleMarketContext::default(),
            booted_at_ms: None,
        },
    )
}

pub fn build_contract_whale_response_with_runtime_and_baselines(
    flow_state: &FlowState,
    symbol: &str,
    limit: usize,
    severity: Option<&str>,
    enabled: bool,
    dry_run: bool,
    runtime: ContractWhaleResponseRuntime<'_>,
) -> ContractWhaleLatestResponse {
    let now = if flow_state.updated_at > 0 {
        flow_state.updated_at
    } else {
        now_ms()
    };
    let filter = response_filter(symbol, enabled, dry_run);
    let exchanges = contract_exchange_statuses(flow_state, runtime.venue_health, enabled, now);
    let warmup = warmup_state(now, enabled, runtime.booted_at_ms);
    let trend_60s = trend_60s_from_flow_state(flow_state, symbol, now);
    if !enabled {
        return ContractWhaleLatestResponse {
            summary: disabled_summary(now, dry_run, exchanges, trend_60s),
            items: Vec::new(),
            filter,
            meta: None,
        };
    }

    let mut detector_reject_reasons: BTreeMap<&'static str, usize> = BTreeMap::new();
    let mut detector_input_windows = 0usize;
    let mut detector_accepted = 0usize;
    let mut items: Vec<ContractWhaleSignal> = Vec::new();
    for window_sec in [5_u64, 15, 60] {
        let Some(window) = flow_window_for_seconds(flow_state, window_sec) else {
            continue;
        };
        let Some(stats) = stats_from_flow_window(
            window,
            symbol,
            now,
            runtime.baselines,
            runtime.liquidations,
            runtime.market_context,
            runtime.booted_at_ms,
        ) else {
            continue;
        };
        detector_input_windows += 1;
        let decision =
            inspect_contract_whale_signal_with_config(&stats, &contract_whale_runtime_config());
        if let Some(signal) = decision.signal {
            detector_accepted += 1;
            if severity_matches(signal.severity, severity) {
                items.push(signal);
            }
        } else if let Some(reason) = decision.reject_reason {
            *detector_reject_reasons.entry(reason.as_key()).or_default() += 1;
        }
    }
    tracing::info!(
        target: CWM_LOG_TARGET,
        event = log_events::SIGNAL_GENERATED,
        symbol = symbol,
        input_windows = detector_input_windows,
        candidates = detector_input_windows,
        accepted = detector_accepted,
        rejected = detector_input_windows.saturating_sub(detector_accepted),
        reject_reasons = ?detector_reject_reasons,
        "{} detector round summary",
        CWM_LOG_PREFIX
    );
    let raw_candidates = items.len();
    items = merge_contract_whale_signals(items);
    let merged_events = items.len();
    decorate_and_filter_price_deviated_signals(
        &mut items,
        current_market_price_from_flow_state(flow_state, symbol),
        contract_whale_runtime_config()
            .toxic_order
            .max_price_deviation_pct,
    );
    items = apply_contract_whale_event_lifecycle(items, now);
    let lifecycle_events = items.len();
    items = apply_contract_whale_event_quality_filter(items);
    let filtered_events = items.len();
    apply_contract_whale_signal_clusters(&mut items);
    apply_contract_whale_trajectories(&mut items);
    items.sort_by(|left, right| {
        right
            .severity
            .rank()
            .cmp(&left.severity.rank())
            .then_with(|| right.score.cmp(&left.score))
            .then_with(|| right.ts.cmp(&left.ts))
    });
    items.truncate(limit);
    let summary = build_summary(
        &items,
        now,
        enabled,
        dry_run,
        exchanges,
        warmup,
        trend_60s,
        ContractWhaleSummaryBuildStats {
            raw_candidates,
            merged_events,
            lifecycle_events,
            filtered_events,
        },
    );
    ContractWhaleLatestResponse {
        summary,
        items,
        filter,
        meta: None,
    }
}

fn response_filter(symbol: &str, enabled: bool, dry_run: bool) -> BTreeMap<String, String> {
    let mut filter = BTreeMap::new();
    filter.insert("symbol".to_string(), symbol.to_string());
    filter.insert("market".to_string(), "perp".to_string());
    filter.insert("marketType".to_string(), "perp".to_string());
    filter.insert("readOnly".to_string(), "true".to_string());
    filter.insert("enabled".to_string(), enabled.to_string());
    filter.insert("dryRun".to_string(), dry_run.to_string());
    filter
}

fn metric_label(value: &str) -> String {
    value
        .chars()
        .filter_map(|ch| match ch {
            'a'..='z' | 'A'..='Z' | '0'..='9' | '_' | '-' | '.' => Some(ch),
            ' ' => Some('_'),
            _ => None,
        })
        .collect::<String>()
        .to_ascii_lowercase()
}

pub fn build_contract_whale_history_response(
    mut items: Vec<ContractWhaleSignal>,
    symbol: &str,
    limit: usize,
    severity: Option<&str>,
    enabled: bool,
    dry_run: bool,
    meta: Option<ContractWhaleResponseMeta>,
) -> ContractWhaleLatestResponse {
    let now = now_ms();
    let filter = response_filter(symbol, enabled, dry_run);
    if !enabled {
        return ContractWhaleLatestResponse {
            summary: disabled_summary(
                now,
                dry_run,
                default_exchange_statuses(),
                empty_trend_60s(symbol),
            ),
            items: Vec::new(),
            filter,
            meta,
        };
    }
    items.retain(|signal| is_perp_signal(signal) && severity_matches(signal.severity, severity));
    let raw_candidates = items.len();
    items = merge_contract_whale_signals(items);
    let merged_events = items.len();
    decorate_and_filter_price_deviated_signals(
        &mut items,
        None,
        contract_whale_runtime_config()
            .toxic_order
            .max_price_deviation_pct,
    );
    let lifecycle_reference_now = items.iter().map(|item| item.ts).max().unwrap_or(now);
    items = apply_contract_whale_event_lifecycle(items, lifecycle_reference_now);
    let lifecycle_events = items.len();
    items = apply_contract_whale_event_quality_filter(items);
    let filtered_events = items.len();
    apply_contract_whale_signal_clusters(&mut items);
    apply_contract_whale_trajectories(&mut items);
    items.sort_by(|left, right| {
        right
            .ts
            .cmp(&left.ts)
            .then_with(|| right.severity.rank().cmp(&left.severity.rank()))
            .then_with(|| right.score.cmp(&left.score))
    });
    items.truncate(limit);
    let summary = build_summary(
        &items,
        now,
        enabled,
        dry_run,
        default_exchange_statuses(),
        warmup_state(now, enabled, None),
        empty_trend_60s(symbol),
        ContractWhaleSummaryBuildStats {
            raw_candidates,
            merged_events,
            lifecycle_events,
            filtered_events,
        },
    );
    ContractWhaleLatestResponse {
        summary,
        items,
        filter,
        meta,
    }
}

pub fn build_contract_whale_items_response(
    mut items: Vec<ContractWhaleSignal>,
    symbol: &str,
    limit: usize,
    enabled: bool,
    dry_run: bool,
    exchanges: BTreeMap<String, ContractWhaleExchangeStatus>,
    trend_60s: ContractWhaleTrend60s,
) -> ContractWhaleLatestResponse {
    let now = now_ms();
    let filter = response_filter(symbol, enabled, dry_run);
    if !enabled {
        return ContractWhaleLatestResponse {
            summary: disabled_summary(now, dry_run, exchanges, empty_trend_60s(symbol)),
            items: Vec::new(),
            filter,
            meta: None,
        };
    }
    items.retain(is_perp_signal);
    decorate_signal_units(&mut items, symbol);
    let raw_candidates = items.len();
    items = merge_contract_whale_signals(items);
    let merged_events = items.len();
    decorate_and_filter_price_deviated_signals(
        &mut items,
        current_market_price_from_trend(&trend_60s),
        contract_whale_runtime_config()
            .toxic_order
            .max_price_deviation_pct,
    );
    items = apply_contract_whale_event_lifecycle(items, now);
    let lifecycle_events = items.len();
    items = apply_contract_whale_event_quality_filter(items);
    let filtered_events = items.len();
    apply_contract_whale_signal_clusters(&mut items);
    apply_contract_whale_trajectories(&mut items);
    items.sort_by(|left, right| {
        right
            .ts
            .cmp(&left.ts)
            .then_with(|| right.severity.rank().cmp(&left.severity.rank()))
            .then_with(|| right.score.cmp(&left.score))
    });
    items.truncate(limit);
    let summary = build_summary(
        &items,
        now,
        enabled,
        dry_run,
        exchanges,
        warmup_state(now, enabled, None),
        trend_60s,
        ContractWhaleSummaryBuildStats {
            raw_candidates,
            merged_events,
            lifecycle_events,
            filtered_events,
        },
    );
    ContractWhaleLatestResponse {
        summary,
        items,
        filter,
        meta: None,
    }
}

fn decorate_signal_units(items: &mut [ContractWhaleSignal], fallback_symbol: &str) {
    for signal in items {
        let unit_source = if signal.symbol.trim().is_empty() {
            fallback_symbol
        } else {
            &signal.symbol
        };
        let base_asset = contract_base_asset(unit_source);
        if signal.symbol.trim().is_empty() {
            signal.symbol = base_asset.clone();
        }
        signal.base_asset = base_asset.clone();
        signal.quantity_unit = base_asset;
        signal.total_volume = signal.total_volume_btc;
        signal.net_volume = signal.net_volume_btc;
    }
}

fn filter_latest_response_by_exchange(
    mut response: ContractWhaleLatestResponse,
    exchange: Option<&str>,
) -> ContractWhaleLatestResponse {
    let Some(exchange) = exchange else {
        return response;
    };
    response.items.retain(|signal| {
        signal
            .exchanges
            .iter()
            .any(|item| item.exchange.eq_ignore_ascii_case(exchange))
    });
    response
        .filter
        .insert("exchange".to_string(), exchange.to_ascii_lowercase());
    response.summary = build_summary(
        &response.items,
        response.summary.updated_at_ms,
        response.summary.enabled,
        response.summary.dry_run,
        response.summary.exchanges.clone(),
        ContractWhaleWarmupState {
            active: response.summary.warmup,
            until_ms: response.summary.warmup_until_ms,
            remaining_ms: response.summary.warmup_remaining_ms,
        },
        response.summary.trend_60s.clone(),
        ContractWhaleSummaryBuildStats::from_visible_items(&response.items),
    );
    response
}

fn flow_window_for_seconds(flow_state: &FlowState, window_sec: u64) -> Option<&FlowWindow> {
    let key_ms = (window_sec * 1000).to_string();
    let key_sec = window_sec.to_string();
    flow_state
        .windows
        .get(&key_ms)
        .or_else(|| flow_state.windows.get(&key_sec))
}

fn stats_from_flow_window(
    window: &FlowWindow,
    symbol: &str,
    now: i64,
    baselines: &BTreeMap<u64, ContractWhaleQualityBaseline>,
    liquidations: &BTreeMap<u64, ContractWhaleLiquidationContext>,
    market_context: &ContractWhaleMarketContext,
    booted_at_ms: Option<i64>,
) -> Option<ContractWhaleWindowStats> {
    if !symbol_matches_window(&window.symbol, symbol) {
        return None;
    }
    let exchanges = exchange_contributions(&window.venue_breakdown);
    if exchanges.is_empty() {
        return None;
    }
    let buy_volume_btc = exchanges
        .iter()
        .map(|item| item.buy_volume_btc)
        .sum::<f64>();
    let sell_volume_btc = exchanges
        .iter()
        .map(|item| item.sell_volume_btc)
        .sum::<f64>();
    let buy_notional_usd = exchanges
        .iter()
        .map(|item| item.buy_notional_usd)
        .sum::<f64>();
    let sell_notional_usd = exchanges
        .iter()
        .map(|item| item.sell_notional_usd)
        .sum::<f64>();
    let exchange_count = exchanges
        .iter()
        .filter(|item| item.total_volume_btc > 0.0)
        .count();
    let main_exchange = exchanges.first().map(|item| item.exchange.clone());
    let total_volume_btc = buy_volume_btc + sell_volume_btc;
    if total_volume_btc <= 0.0 {
        return None;
    }
    let net_volume_btc = buy_volume_btc - sell_volume_btc;
    let data_quality = market_context_quality_score(data_quality_score(window), market_context);
    let window_sec = window.window_ms / 1000;
    let baseline = baselines.get(&window_sec);
    let liquidation_context = liquidations.get(&window_sec).cloned().unwrap_or_default();
    let liquidation_driven = liquidation_context
        .liq_to_volume_ratio
        .is_some_and(|ratio| ratio >= 0.25 && liquidation_context.total_liq_btc >= 50.0);
    Some(ContractWhaleWindowStats {
        symbol: symbol.to_string(),
        window_sec,
        ts: now,
        buy_volume_btc,
        sell_volume_btc,
        total_volume_btc,
        net_volume_btc,
        dominance: net_volume_btc.abs() / total_volume_btc,
        buy_notional_usd,
        sell_notional_usd,
        total_notional_usd: buy_notional_usd + sell_notional_usd,
        price_move_pct: window.price_move_bps.map(|bps| bps / 100.0),
        exchange_count,
        main_exchange,
        dominant_venue_net_contribution_share: dominant_venue_net_contribution_share(&exchanges),
        exchanges,
        dynamic_multiple: baseline.and_then(|item| item.dynamic_multiple),
        dynamic_baseline_btc: baseline.and_then(|item| item.dynamic_baseline_btc),
        dynamic_threshold_level: baseline
            .map(|item| item.dynamic_threshold_level.clone())
            .unwrap_or_default(),
        percentile_level: baseline.and_then(|item| item.percentile_level),
        multi_exchange_confirmed: false,
        liquidation_context,
        market_context: market_context.clone(),
        price_reversal_ratio: None,
        data_quality,
        ws_latency_ms: None,
        startup_age_ms: booted_at_ms.map(|booted_at_ms| now.saturating_sub(booted_at_ms)),
        liquidation_driven,
        price_jump_anomaly: false,
    })
}

fn exchange_contributions(
    breakdown: &BTreeMap<String, VenueFlowBreakdown>,
) -> Vec<ExchangeFlowContribution> {
    let runtime_config = contract_whale_runtime_config();
    let total_net_volume_btc = breakdown
        .iter()
        .filter(|(exchange, _)| runtime_config.exchange_enabled(exchange))
        .map(|(_, item)| item.aggressive_buy_btc - item.aggressive_sell_btc)
        .sum::<f64>();
    let mut contributions: Vec<ExchangeFlowContribution> = breakdown
        .iter()
        .filter(|(exchange, _)| runtime_config.exchange_enabled(exchange))
        .map(|(exchange, item)| {
            let total_volume_btc = item.aggressive_buy_btc + item.aggressive_sell_btc;
            let net_volume_btc = item.aggressive_buy_btc - item.aggressive_sell_btc;
            ExchangeFlowContribution {
                exchange: exchange.clone(),
                buy_volume_btc: item.aggressive_buy_btc,
                sell_volume_btc: item.aggressive_sell_btc,
                total_volume_btc,
                buy_share: share(item.aggressive_buy_btc, total_volume_btc),
                sell_share: share(item.aggressive_sell_btc, total_volume_btc),
                buy_notional_usd: item.aggressive_buy_usd,
                sell_notional_usd: item.aggressive_sell_usd,
                total_notional_usd: item.aggressive_buy_usd + item.aggressive_sell_usd,
                net_volume_btc,
                dominance: dominance(net_volume_btc.abs(), total_volume_btc),
                net_contribution_share: 0.0,
                trade_count: item.trade_count,
            }
        })
        .filter(|item| item.total_volume_btc > 0.0)
        .collect();
    apply_net_contribution_shares(&mut contributions, total_net_volume_btc);
    contributions.sort_by(|left, right| {
        right
            .total_volume_btc
            .partial_cmp(&left.total_volume_btc)
            .unwrap_or(std::cmp::Ordering::Equal)
    });
    contributions
}

pub(crate) fn load_quality_baselines(
    store: &impl ContractWhaleRepo,
    flow_state: &FlowState,
    symbol: &str,
) -> BTreeMap<u64, ContractWhaleQualityBaseline> {
    let now = if flow_state.updated_at > 0 {
        flow_state.updated_at
    } else {
        now_ms()
    };
    let min_dynamic_samples = contract_whale_runtime_config()
        .data_quality
        .min_dynamic_samples;
    [5_u64, 15, 60]
        .into_iter()
        .filter_map(|window_sec| {
            let window = flow_window_for_seconds(flow_state, window_sec)?;
            let current_total = window.aggressive_buy_btc + window.aggressive_sell_btc;
            let window_ms = (window_sec as i64).saturating_mul(1000);
            let dynamic_to = now.saturating_sub(window_ms);
            let dynamic_from = dynamic_to.saturating_sub(60 * 60 * 1000);
            let dynamic_baseline_btc =
                match store.list_contract_flow_buckets_between(symbol, dynamic_from, dynamic_to) {
                    Ok(buckets) => historical_window_average_btc_with_min_samples(
                        &buckets,
                        symbol,
                        window_sec,
                        dynamic_from,
                        dynamic_to,
                        min_dynamic_samples,
                    ),
                    Err(error) => {
                        tracing::warn!(
                            target: CWM_LOG_TARGET,
                            error = %error,
                            window_sec,
                            "{} dynamic baseline query failed",
                            CWM_LOG_PREFIX
                        );
                        None
                    }
                };
            let dynamic_multiple = dynamic_multiple_for_volume(current_total, dynamic_baseline_btc);
            let percentile_level =
                latest_percentile_level(store, symbol, window_sec, current_total, now);
            let dynamic_threshold_level =
                dynamic_threshold_level(dynamic_multiple, percentile_level);
            Some((
                window_sec,
                ContractWhaleQualityBaseline {
                    dynamic_multiple,
                    dynamic_baseline_btc,
                    dynamic_threshold_level,
                    percentile_level,
                },
            ))
        })
        .collect()
}

fn latest_percentile_level(
    store: &impl ContractWhaleRepo,
    symbol: &str,
    window_sec: u64,
    current_total: f64,
    now: i64,
) -> Option<f64> {
    let threshold = match store.latest_contract_whale_percentile(symbol, "all", window_sec) {
        Ok(Some(threshold)) if threshold.computed_at >= now.saturating_sub(60 * 60 * 1000) => {
            Some(threshold)
        }
        Ok(_) => refresh_percentile_threshold(store, symbol, window_sec, now),
        Err(error) => {
            tracing::warn!(
                target: CWM_LOG_TARGET,
                error = %error,
                window_sec,
                "{} percentile threshold query failed",
                CWM_LOG_PREFIX
            );
            None
        }
    };
    percentile_level_for_volume(current_total, threshold.as_ref())
}

fn dynamic_threshold_level(dynamic_multiple: Option<f64>, percentile_level: Option<f64>) -> String {
    if dynamic_multiple.is_some_and(|value| value >= 10.0)
        || percentile_level.is_some_and(|value| value >= 99.9)
    {
        "s".to_string()
    } else if dynamic_multiple.is_some_and(|value| value >= 7.0)
        || percentile_level.is_some_and(|value| value >= 99.5)
    {
        "critical".to_string()
    } else if dynamic_multiple.is_some_and(|value| value >= 5.0)
        || percentile_level.is_some_and(|value| value >= 99.0)
    {
        "high".to_string()
    } else if dynamic_multiple.is_some_and(|value| value >= 4.0) {
        "watch".to_string()
    } else {
        "normal".to_string()
    }
}

fn refresh_percentile_threshold(
    store: &impl ContractWhaleRepo,
    symbol: &str,
    window_sec: u64,
    now: i64,
) -> Option<ContractWhalePercentileThreshold> {
    let from = now.saturating_sub(7 * 24 * 60 * 60 * 1000);
    let buckets = match store.list_contract_flow_buckets_between(symbol, from, now) {
        Ok(buckets) => buckets,
        Err(error) => {
            tracing::warn!(
                target: CWM_LOG_TARGET,
                error = %error,
                window_sec,
                "{} percentile bucket query failed",
                CWM_LOG_PREFIX
            );
            return None;
        }
    };
    let threshold =
        compute_percentile_threshold(&buckets, symbol, "all", window_sec, from, now, now)?;
    if let Err(error) = store.upsert_contract_whale_percentiles(std::slice::from_ref(&threshold)) {
        tracing::warn!(
            target: CWM_LOG_TARGET,
            error = %error,
            window_sec,
            "{} percentile threshold upsert failed",
            CWM_LOG_PREFIX
        );
    }
    Some(threshold)
}

pub(crate) fn load_liquidation_contexts(
    store: &impl ContractWhaleRepo,
    flow_state: &FlowState,
    symbol: &str,
) -> BTreeMap<u64, ContractWhaleLiquidationContext> {
    let now = if flow_state.updated_at > 0 {
        flow_state.updated_at
    } else {
        now_ms()
    };
    let from = now.saturating_sub(60_000);
    let buckets = match store.list_contract_liquidation_buckets_between(symbol, from, now) {
        Ok(buckets) => buckets,
        Err(error) => {
            tracing::warn!(
                target: CWM_LOG_TARGET,
                error = %error,
                "{} liquidation bucket query failed",
                CWM_LOG_PREFIX
            );
            return BTreeMap::new();
        }
    };
    [5_u64, 15, 60]
        .into_iter()
        .filter_map(|window_sec| {
            let window = flow_window_for_seconds(flow_state, window_sec)?;
            let total_volume_btc = window.aggressive_buy_btc + window.aggressive_sell_btc;
            Some((
                window_sec,
                liquidation_context_for_window(&buckets, symbol, window_sec, now, total_volume_btc),
            ))
        })
        .collect()
}

pub(crate) fn load_market_context(
    store: &impl ContractWhaleRepo,
    flow_state: &FlowState,
    symbol: &str,
) -> ContractWhaleMarketContext {
    let now = if flow_state.updated_at > 0 {
        flow_state.updated_at
    } else {
        now_ms()
    };
    let from = now.saturating_sub(6 * 60_000);
    let oi_snapshots = match store.list_contract_oi_snapshots_between(symbol, from, now) {
        Ok(rows) => rows,
        Err(error) => {
            tracing::warn!(
                target: CWM_LOG_TARGET,
                error = %error,
                "{} oi snapshot query failed",
                CWM_LOG_PREFIX
            );
            Vec::new()
        }
    };
    let funding_snapshots = match store.list_contract_funding_snapshots_between(symbol, from, now) {
        Ok(rows) => rows,
        Err(error) => {
            tracing::warn!(
                target: CWM_LOG_TARGET,
                error = %error,
                "{} funding snapshot query failed",
                CWM_LOG_PREFIX
            );
            Vec::new()
        }
    };
    market_context_from_snapshots(&oi_snapshots, &funding_snapshots, symbol, now)
}

fn data_quality_score(window: &FlowWindow) -> u8 {
    let runtime_config = contract_whale_runtime_config();
    let active_exchange_count = window
        .venue_breakdown
        .iter()
        .filter(|(exchange, breakdown)| {
            runtime_config.exchange_enabled(exchange)
                && breakdown.aggressive_buy_btc + breakdown.aggressive_sell_btc > 0.0
        })
        .count();
    let has_active_trades = active_exchange_count > 0;
    if has_active_trades && active_exchange_count >= 2 {
        85
    } else if has_active_trades {
        70
    } else {
        40
    }
}

fn market_context_quality_score(base: u8, context: &ContractWhaleMarketContext) -> u8 {
    if !context.context_expected {
        return base;
    }
    let mut score = base;
    if !context.oi_available {
        score = score.saturating_sub(3);
    }
    if !context.funding_available {
        score = score.saturating_sub(2);
    }
    score
}

fn dominance(abs_net_volume: f64, total_volume: f64) -> f64 {
    if total_volume <= f64::EPSILON {
        0.0
    } else {
        abs_net_volume / total_volume
    }
}

fn share(part: f64, total: f64) -> f64 {
    if total <= f64::EPSILON {
        0.0
    } else {
        part.max(0.0) / total
    }
}

fn apply_net_contribution_shares(
    contributions: &mut [ExchangeFlowContribution],
    total_net_volume_btc: f64,
) {
    let net_positive = total_net_volume_btc > 0.0;
    let same_direction_net_sum = contributions
        .iter()
        .filter(|item| item.net_volume_btc.abs() > f64::EPSILON)
        .filter(|item| (item.net_volume_btc > 0.0) == net_positive)
        .map(|item| item.net_volume_btc.abs())
        .sum::<f64>();
    for item in contributions {
        item.net_contribution_share = if same_direction_net_sum > f64::EPSILON
            && item.net_volume_btc.abs() > f64::EPSILON
            && (item.net_volume_btc > 0.0) == net_positive
        {
            item.net_volume_btc.abs() / same_direction_net_sum
        } else {
            0.0
        };
    }
}

fn dominant_venue_net_contribution_share(
    contributions: &[ExchangeFlowContribution],
) -> Option<f64> {
    contributions
        .iter()
        .map(|item| item.net_contribution_share)
        .filter(|value| value.is_finite() && *value > 0.0)
        .max_by(|left, right| left.total_cmp(right))
}

fn trend_60s_from_flow_state(
    flow_state: &FlowState,
    symbol: &str,
    now: i64,
) -> ContractWhaleTrend60s {
    let Some(window) = flow_window_for_seconds(flow_state, 60) else {
        return empty_trend_60s(symbol);
    };
    if !symbol_matches_window(&window.symbol, symbol) {
        return empty_trend_60s(symbol);
    }
    let contributions = exchange_contributions(&window.venue_breakdown);
    if contributions.is_empty() {
        return empty_trend_60s(symbol);
    }
    let buy_volume_btc = contributions
        .iter()
        .map(|item| item.buy_volume_btc)
        .sum::<f64>()
        .max(0.0);
    let sell_volume_btc = contributions
        .iter()
        .map(|item| item.sell_volume_btc)
        .sum::<f64>()
        .max(0.0);
    let total_volume_btc = buy_volume_btc + sell_volume_btc;
    let net_volume_btc = buy_volume_btc - sell_volume_btc;
    let dominance = dominance(net_volume_btc.abs(), total_volume_btc);
    let buy_ratio = if total_volume_btc > f64::EPSILON {
        buy_volume_btc / total_volume_btc
    } else {
        0.0
    };
    ContractWhaleTrend60s {
        symbol: contract_base_asset(symbol),
        base_asset: contract_base_asset(symbol),
        quantity_unit: contract_base_asset(symbol),
        buy_volume: buy_volume_btc,
        sell_volume: sell_volume_btc,
        total_volume: total_volume_btc,
        net_volume: net_volume_btc,
        buy_volume_btc,
        sell_volume_btc,
        total_volume_btc,
        net_volume_btc,
        dominance,
        buy_ratio,
        sell_ratio: 1.0_f64.min(1.0 - buy_ratio).max(0.0),
        updated_at_ms: Some(window.now_ts).filter(|ts| *ts > 0).or(Some(now)),
    }
}

fn empty_trend_60s(symbol: &str) -> ContractWhaleTrend60s {
    let base_asset = contract_base_asset(symbol);
    ContractWhaleTrend60s {
        symbol: base_asset.clone(),
        base_asset: base_asset.clone(),
        quantity_unit: base_asset,
        ..Default::default()
    }
}

fn contract_base_asset(symbol: &str) -> String {
    symbol
        .trim()
        .split(['-', '_', '/', ':'])
        .next()
        .unwrap_or(symbol)
        .trim_end_matches("USDT")
        .trim_end_matches("USD")
        .to_ascii_uppercase()
}

fn trend_summary_unit(trend_60s: &ContractWhaleTrend60s) -> String {
    if !trend_60s.quantity_unit.trim().is_empty() {
        return trend_60s.quantity_unit.clone();
    }
    if !trend_60s.base_asset.trim().is_empty() {
        return trend_60s.base_asset.clone();
    }
    if !trend_60s.symbol.trim().is_empty() {
        return contract_base_asset(&trend_60s.symbol);
    }
    "BTC".to_string()
}

fn symbol_matches_window(window_symbol: &str, requested_symbol: &str) -> bool {
    let normalize = |value: &str| {
        value
            .trim()
            .split(['-', '_', '/', ':'])
            .next()
            .unwrap_or(value)
            .to_ascii_uppercase()
    };
    normalize(window_symbol) == normalize(requested_symbol)
}

fn contract_whale_health_status(
    enabled: bool,
    warmup: ContractWhaleWarmupState,
    exchanges: &BTreeMap<String, ContractWhaleExchangeStatus>,
    now: i64,
) -> (String, String) {
    if !enabled {
        return (
            "disabled".to_string(),
            "contract_whale_monitor_disabled".to_string(),
        );
    }
    if warmup.active {
        return ("warming_up".to_string(), "warmup_collect_only".to_string());
    }

    let runtime_config = contract_whale_runtime_config();
    let enabled_exchanges = runtime_config.enabled_exchanges();
    if enabled_exchanges.is_empty() {
        return (
            "unhealthy".to_string(),
            "no_enabled_contract_exchanges".to_string(),
        );
    }
    let recent_exchange_count = enabled_exchanges
        .iter()
        .filter(|exchange| {
            let silence_ms = if exchange.as_str() == "bitfinex" {
                60_000
            } else {
                30_000
            };
            exchange_recent(exchanges.get(exchange.as_str()), now, silence_ms)
        })
        .count();
    let primary_recent = runtime_config
        .primary_contract_exchanges()
        .iter()
        .any(|exchange| exchange_recent(exchanges.get(exchange.as_str()), now, 30_000));
    let all_sources_stale = enabled_exchanges.iter().all(|exchange| {
        exchanges
            .get(exchange.as_str())
            .and_then(|status| status.last_trade_at)
            .is_none_or(|last_trade_at| now.saturating_sub(last_trade_at) > 60_000)
    });

    if recent_exchange_count == enabled_exchanges.len() && primary_recent {
        ("healthy".to_string(), "enabled_sources_recent".to_string())
    } else if recent_exchange_count > 0 {
        (
            "degraded".to_string(),
            if primary_recent {
                "partial_sources_recent".to_string()
            } else {
                "primary_source_missing".to_string()
            },
        )
    } else if all_sources_stale {
        (
            "unhealthy".to_string(),
            "all_sources_no_recent_trades".to_string(),
        )
    } else {
        (
            "unhealthy".to_string(),
            "primary_sources_disconnected".to_string(),
        )
    }
}

fn exchange_recent(
    status: Option<&ContractWhaleExchangeStatus>,
    now: i64,
    max_silence_ms: i64,
) -> bool {
    status
        .and_then(|status| status.last_trade_at)
        .is_some_and(|last_trade_at| now.saturating_sub(last_trade_at) <= max_silence_ms)
}

fn disabled_summary(
    now: i64,
    dry_run: bool,
    exchanges: BTreeMap<String, ContractWhaleExchangeStatus>,
    trend_60s: ContractWhaleTrend60s,
) -> ContractWhaleSummary {
    let runtime_config = contract_whale_runtime_config();
    let resolution = runtime_config.threshold_profile_resolution_with_statuses(&exchanges, now);
    let quantity_unit = trend_summary_unit(&trend_60s);
    ContractWhaleSummary {
        status: "disabled".to_string(),
        health_status: "disabled".to_string(),
        health_reason: "contract_whale_monitor_disabled".to_string(),
        symbol: quantity_unit.clone(),
        base_asset: quantity_unit.clone(),
        quantity_unit: quantity_unit.clone(),
        market_type: "perp".to_string(),
        meta: None,
        threshold_profile: resolution.profile_name.clone(),
        threshold_profile_reason: resolution.reason.clone(),
        configured_contract_sources: resolution.configured_keys(),
        eligible_contract_sources: resolution.eligible_keys(),
        active_exchange_count: resolution.active_contract_sources.len(),
        enabled_exchanges: resolution.active_keys(),
        disabled_exchanges: runtime_config.disabled_exchanges(),
        active_contract_exchanges: resolution.active_keys(),
        direction: "disabled".to_string(),
        latest_direction: "disabled".to_string(),
        latest_severity: ContractWhaleSeverity::Calm,
        latest_signal_at: None,
        latest_pushed_at_ms: None,
        last_discord_sent_at: None,
        updated_at_ms: now,
        signal_count: 0,
        read_only: true,
        enabled: false,
        dry_run,
        contract_data_quality: 0,
        spot_data_quality: 0,
        overall_data_quality: 0,
        warmup: false,
        warmup_until_ms: None,
        warmup_remaining_ms: None,
        trend_60s,
        discord_dry_run_stats: ContractWhaleDiscordDryRunStats::default(),
        market_structure_lite: ContractWhaleMarketStructureLite {
            status: "disabled".to_string(),
            regime_type: "unclear".to_string(),
            data_quality: 0,
            reason: "CWM 未启用，Market Structure Lite 不计算。".to_string(),
            ..Default::default()
        },
        noise_suppression: ContractWhaleNoiseSuppressionSummary::default(),
        trade_opportunities: Vec::new(),
        exchanges,
        platforms: build_platform_capabilities(&runtime_config),
    }
}

#[derive(Debug, Clone, Copy, Default)]
struct ContractWhaleSummaryBuildStats {
    raw_candidates: usize,
    merged_events: usize,
    lifecycle_events: usize,
    filtered_events: usize,
}

impl ContractWhaleSummaryBuildStats {
    fn from_visible_items(items: &[ContractWhaleSignal]) -> Self {
        Self {
            raw_candidates: items.len(),
            merged_events: items.len(),
            lifecycle_events: items.len(),
            filtered_events: items.len(),
        }
    }
}

fn build_summary(
    items: &[ContractWhaleSignal],
    now: i64,
    enabled: bool,
    dry_run: bool,
    exchanges: BTreeMap<String, ContractWhaleExchangeStatus>,
    warmup: ContractWhaleWarmupState,
    trend_60s: ContractWhaleTrend60s,
    build_stats: ContractWhaleSummaryBuildStats,
) -> ContractWhaleSummary {
    let quantity_unit = trend_summary_unit(&trend_60s);
    let latest = items.first();
    let latest_direction = latest
        .map(|signal| direction_key(signal.direction).to_string())
        .unwrap_or_else(|| "neutral".to_string());
    let last_discord_sent_at = items
        .iter()
        .filter(|signal| signal.discord_sent)
        .filter_map(|signal| signal.discord_sent_at.or(Some(signal.ts)))
        .max();
    let (health_status, health_reason) =
        contract_whale_health_status(enabled, warmup, &exchanges, now);
    tracing::debug!(
        target: CWM_LOG_TARGET,
        event = log_events::HEALTH_EVALUATED,
        health_status = health_status.as_str(),
        health_reason = health_reason.as_str(),
        "{} health evaluated",
        CWM_LOG_PREFIX
    );
    let runtime_config = contract_whale_runtime_config();
    let resolution = runtime_config.threshold_profile_resolution_with_statuses(&exchanges, now);
    let (contract_data_quality, spot_data_quality, overall_data_quality) =
        summary_data_quality_scores(
            &runtime_config,
            &resolution.active_keys(),
            &exchanges,
            now,
            warmup.active,
        );
    let market_structure_lite = market_structure_lite_from_items(
        items,
        &ContractWhaleSpotConfirmationContext::default(),
        overall_data_quality,
    );
    let trade_opportunities = build_trade_opportunities(items, &market_structure_lite);
    let noise_suppression = build_noise_suppression_summary(build_stats, trade_opportunities.len());
    ContractWhaleSummary {
        status: if warmup.active {
            "warmup".to_string()
        } else {
            latest
                .map(|signal| status_code(signal.severity))
                .unwrap_or("calm")
                .to_string()
        },
        health_status,
        health_reason,
        symbol: quantity_unit.clone(),
        base_asset: quantity_unit.clone(),
        quantity_unit: quantity_unit.clone(),
        market_type: "perp".to_string(),
        meta: None,
        threshold_profile: resolution.profile_name.clone(),
        threshold_profile_reason: resolution.reason.clone(),
        configured_contract_sources: resolution.configured_keys(),
        eligible_contract_sources: resolution.eligible_keys(),
        active_exchange_count: resolution.active_contract_sources.len(),
        enabled_exchanges: resolution.active_keys(),
        disabled_exchanges: runtime_config.disabled_exchanges(),
        active_contract_exchanges: resolution.active_keys(),
        direction: latest_direction.clone(),
        latest_direction,
        latest_severity: latest
            .map(|signal| signal.severity)
            .unwrap_or(ContractWhaleSeverity::Calm),
        latest_signal_at: latest.map(|signal| signal.ts),
        latest_pushed_at_ms: last_discord_sent_at,
        last_discord_sent_at,
        updated_at_ms: now,
        signal_count: items.len(),
        read_only: true,
        enabled,
        dry_run,
        contract_data_quality,
        spot_data_quality,
        overall_data_quality,
        warmup: warmup.active,
        warmup_until_ms: warmup.until_ms,
        warmup_remaining_ms: warmup.remaining_ms,
        trend_60s,
        discord_dry_run_stats: discord_dry_run_stats(items, now),
        market_structure_lite,
        noise_suppression,
        trade_opportunities,
        exchanges,
        platforms: build_platform_capabilities(&runtime_config),
    }
}

fn build_noise_suppression_summary(
    build_stats: ContractWhaleSummaryBuildStats,
    tradeable_setups: usize,
) -> ContractWhaleNoiseSuppressionSummary {
    let noise_reduction_pct = if build_stats.raw_candidates == 0 {
        0
    } else {
        (((build_stats
            .raw_candidates
            .saturating_sub(build_stats.filtered_events)) as f64
            / build_stats.raw_candidates as f64)
            * 100.0)
            .round()
            .clamp(0.0, 100.0) as u8
    };
    ContractWhaleNoiseSuppressionSummary {
        raw_candidates: build_stats.raw_candidates,
        merged_events: build_stats.merged_events,
        lifecycle_events: build_stats.lifecycle_events,
        filtered_events: build_stats.filtered_events,
        tradeable_setups,
        suppressed_duplicates: build_stats
            .raw_candidates
            .saturating_sub(build_stats.lifecycle_events),
        noise_reduction_pct,
    }
}

pub fn build_trading_decision_response(
    symbol: &str,
    items: &[ContractWhaleSignal],
    market_structure_lite: &ContractWhaleMarketStructureLite,
    noise_suppression: ContractWhaleNoiseSuppressionSummary,
    timestamp: i64,
) -> ContractWhaleTradingDecisionResponse {
    trading::build_trading_decision_response(
        symbol,
        items,
        market_structure_lite,
        noise_suppression,
        timestamp,
    )
}

fn build_trade_opportunities(
    items: &[ContractWhaleSignal],
    market_structure_lite: &ContractWhaleMarketStructureLite,
) -> Vec<ContractWhaleTradeOpportunity> {
    let mut ranked: Vec<ContractWhaleTradeOpportunity> = items
        .iter()
        .map(|signal| {
            let trade_score = trading::scoring::score_signal(signal);
            let confidence = trading::scoring::confidence_from_score(signal, trade_score);
            ContractWhaleTradeOpportunity {
                signal_id: signal.id.clone(),
                rank: 0,
                setup_type: trading::classifier::setup_type_label(signal.signal_type).to_string(),
                action: summary_trade_action(signal, trade_score).to_string(),
                direction_bias: direction_key(signal.direction).to_string(),
                trade_score,
                confidence,
                severity: signal.severity,
                window_sec: signal.window_sec,
                regime_context: trade_regime_context(market_structure_lite),
                rationale: trade_rationale(signal, market_structure_lite),
            }
        })
        .filter(|opportunity| opportunity.trade_score >= 65)
        .collect();

    ranked.sort_by(|left, right| {
        right
            .trade_score
            .cmp(&left.trade_score)
            .then_with(|| right.confidence.cmp(&left.confidence))
            .then_with(|| right.severity.rank().cmp(&left.severity.rank()))
            .then_with(|| right.window_sec.cmp(&left.window_sec))
    });
    ranked.truncate(3);
    for (index, opportunity) in ranked.iter_mut().enumerate() {
        opportunity.rank = index + 1;
    }
    ranked
}

fn summary_trade_action(signal: &ContractWhaleSignal, trade_score: u8) -> &'static str {
    match trading::classifier::classify_direction(signal, trade_score) {
        trading::classifier::TradingDirection::Long => "LONG",
        trading::classifier::TradingDirection::Short => "SHORT",
        trading::classifier::TradingDirection::NoTrade => "WATCH",
    }
}

fn trade_regime_context(market_structure_lite: &ContractWhaleMarketStructureLite) -> String {
    if !market_structure_lite.regime_type.trim().is_empty() {
        market_structure_lite.regime_type.clone()
    } else if !market_structure_lite.status.trim().is_empty() {
        market_structure_lite.status.clone()
    } else {
        "unclear".to_string()
    }
}

fn trade_rationale(
    signal: &ContractWhaleSignal,
    market_structure_lite: &ContractWhaleMarketStructureLite,
) -> String {
    let mut evidence = Vec::new();
    if !signal.merged_from.is_empty() {
        evidence.push("多窗口一致".to_string());
    }
    if signal.multi_exchange_confirmed {
        evidence.push("双交易所确认".to_string());
    }
    if signal.dominance >= 0.60 {
        evidence.push(format!("方向占比 {:.1}%", signal.dominance * 100.0));
    }
    if let Some(price_move_pct) = signal.price_move_pct {
        if price_move_pct.abs() >= 0.10 {
            evidence.push(format!("价格响应 {:.2}%", price_move_pct));
        }
    }
    if evidence.is_empty() {
        evidence.push("结构强度满足交易观察阈值".to_string());
    }
    format!(
        "{}，当前结构上下文为 {}。",
        evidence.join("，"),
        trade_regime_context(market_structure_lite)
    )
}

fn build_platform_capabilities(
    runtime_config: &crate::contract_whale_monitor::config::ContractWhaleRuntimeConfig,
) -> BTreeMap<String, ContractWhalePlatformCapability> {
    runtime_config
        .platform_keys()
        .into_iter()
        .map(|exchange| {
            let capability = runtime_config
                .exchange_platform(&exchange)
                .map(platform_capability_from_config)
                .unwrap_or_default();
            (exchange, capability)
        })
        .collect()
}

fn platform_capability_from_config(
    platform: &crate::contract_whale_monitor::config::ContractWhalePlatformConfig,
) -> ContractWhalePlatformCapability {
    let status = if !platform.enabled {
        "disabled"
    } else if platform.perp.enabled
        && platform.perp.requires_auth
        && !platform.perp.auth_configured()
    {
        "degraded"
    } else if platform.contract_markets_enabled() {
        "active"
    } else if platform.any_market_enabled() {
        "spot_only"
    } else {
        "disabled"
    };
    ContractWhalePlatformCapability {
        platform_enabled: platform.enabled,
        status: status.to_string(),
        markets: [
            market_capability_entry(
                "spot",
                platform.spot.enabled,
                platform.spot.role.as_key(),
                &platform.spot,
            ),
            market_capability_entry(
                "perp",
                platform.perp.enabled,
                platform.perp.role.as_key(),
                &platform.perp,
            ),
            market_capability_entry(
                "level2",
                platform.level2.enabled,
                platform.level2.role.as_key(),
                &platform.level2,
            ),
            market_capability_entry(
                "funding",
                platform.funding.enabled,
                platform.funding.role.as_key(),
                &platform.funding,
            ),
            market_capability_entry(
                "oi",
                platform.oi.enabled,
                platform.oi.role.as_key(),
                &platform.oi,
            ),
            market_capability_entry(
                "liquidation",
                platform.liquidation.enabled,
                platform.liquidation.role.as_key(),
                &platform.liquidation,
            ),
        ]
        .into_iter()
        .collect(),
    }
}

fn market_capability_entry(
    market: &str,
    enabled: bool,
    role: &str,
    source: &crate::contract_whale_monitor::config::ContractWhaleSourceConfig,
) -> (String, ContractWhaleMarketCapability) {
    (
        market.to_string(),
        ContractWhaleMarketCapability {
            enabled,
            status: if !enabled {
                "disabled".to_string()
            } else if market == "perp" && source.requires_auth && !source.auth_configured() {
                "auth_missing".to_string()
            } else if market == "perp" && source.requires_auth {
                "ready".to_string()
            } else if market == "perp" {
                "active".to_string()
            } else {
                "enabled".to_string()
            },
            role: role.to_string(),
            product: source.product.clone(),
            source: source.source.clone(),
            requires_auth: source.requires_auth,
            market_data_only: source.market_data_only,
            auth_configured: source.auth_configured(),
        },
    )
}

fn summary_data_quality_scores(
    runtime_config: &crate::contract_whale_monitor::config::ContractWhaleRuntimeConfig,
    active_contract_exchanges: &[String],
    exchanges: &BTreeMap<String, ContractWhaleExchangeStatus>,
    now: i64,
    warmup_active: bool,
) -> (u8, u8, u8) {
    let contract_data_quality = contract_data_quality_score(
        runtime_config,
        active_contract_exchanges,
        exchanges,
        now,
        warmup_active,
    );
    let spot_data_quality = spot_data_quality_score(runtime_config, exchanges, now, warmup_active);
    let overall_data_quality =
        ((contract_data_quality as f64 * 0.60) + (spot_data_quality as f64 * 0.40)).round() as u8;
    (
        contract_data_quality,
        spot_data_quality,
        overall_data_quality.min(100),
    )
}

fn contract_data_quality_score(
    runtime_config: &crate::contract_whale_monitor::config::ContractWhaleRuntimeConfig,
    active_contract_exchanges: &[String],
    exchanges: &BTreeMap<String, ContractWhaleExchangeStatus>,
    now: i64,
    warmup_active: bool,
) -> u8 {
    let enabled_exchanges = active_contract_exchanges;
    if enabled_exchanges.is_empty() {
        return 0;
    }
    let recent_count = enabled_exchanges
        .iter()
        .filter(|exchange| exchange_recent(exchanges.get(*exchange), now, 30_000))
        .count();
    let primary_exchanges = runtime_config
        .primary_contract_exchanges()
        .into_iter()
        .filter(|exchange| enabled_exchanges.iter().any(|active| active == exchange))
        .collect::<Vec<_>>();
    let primary_recent = primary_exchanges
        .iter()
        .all(|exchange| exchange_recent(exchanges.get(exchange), now, 30_000));
    let mut score: u8 = if recent_count == 0 {
        20
    } else if recent_count == enabled_exchanges.len() && primary_recent {
        95
    } else if primary_recent && recent_count + 1 >= enabled_exchanges.len() {
        78
    } else if primary_recent {
        72
    } else if recent_count > 0 {
        58
    } else {
        20
    };
    if warmup_active {
        score = score.saturating_sub(20);
    }
    score
}

fn spot_data_quality_score(
    runtime_config: &crate::contract_whale_monitor::config::ContractWhaleRuntimeConfig,
    exchanges: &BTreeMap<String, ContractWhaleExchangeStatus>,
    now: i64,
    warmup_active: bool,
) -> u8 {
    let enabled_spot_sources: Vec<String> = runtime_config
        .platform_keys()
        .into_iter()
        .filter(|exchange| {
            runtime_config.market_enabled(
                exchange,
                crate::contract_whale_monitor::types::ContractWhaleMarketType::Spot,
            )
        })
        .collect();
    if enabled_spot_sources.is_empty() {
        return 0;
    }
    let recent_count = enabled_spot_sources
        .iter()
        .filter(|exchange| exchange_recent(exchanges.get(*exchange), now, 30_000))
        .count();
    let mut score: u8 = if recent_count == 0 {
        25
    } else if recent_count == enabled_spot_sources.len() {
        92
    } else if recent_count + 1 >= enabled_spot_sources.len() {
        78
    } else {
        60
    };
    if warmup_active {
        score = score.saturating_sub(10);
    }
    score
}

fn contract_market_mismatch_meta(
    runtime_config: &crate::contract_whale_monitor::config::ContractWhaleRuntimeConfig,
    exchange: Option<&str>,
) -> Option<ContractWhaleResponseMeta> {
    let exchange = exchange?;
    let platform = runtime_config.exchange_platform(exchange)?;
    if platform.market_enabled(ContractWhaleMarketType::Perp) {
        return None;
    }
    let spot_enabled = platform.market_enabled(ContractWhaleMarketType::Spot);
    let reason = if exchange.eq_ignore_ascii_case("coinbase") && platform.enabled && spot_enabled {
        "coinbase_perp_disabled"
    } else if platform.enabled && spot_enabled {
        "perp_market_disabled"
    } else if platform.enabled {
        "contract_market_disabled"
    } else {
        "exchange_disabled"
    };
    Some(ContractWhaleResponseMeta {
        exchange: Some(exchange.to_string()),
        market_type: Some("perp".to_string()),
        exchange_status: Some(
            if platform.enabled && spot_enabled && !platform.contract_markets_enabled() {
                "spot_only".to_string()
            } else {
                "disabled".to_string()
            },
        ),
        reason: Some(reason.to_string()),
    })
}

fn is_perp_signal(signal: &ContractWhaleSignal) -> bool {
    signal.market_type == ContractWhaleMarketType::Perp
}

fn warmup_state(now: i64, enabled: bool, booted_at_ms: Option<i64>) -> ContractWhaleWarmupState {
    if !enabled {
        return ContractWhaleWarmupState::default();
    }
    let warmup_ms = contract_whale_runtime_config().data_quality.warmup_ms;
    if warmup_ms <= 0 {
        return ContractWhaleWarmupState::default();
    }
    let Some(booted_at_ms) = booted_at_ms else {
        return ContractWhaleWarmupState::default();
    };
    let until_ms = booted_at_ms.saturating_add(warmup_ms);
    let remaining_ms = until_ms.saturating_sub(now);
    ContractWhaleWarmupState {
        active: remaining_ms > 0,
        until_ms: Some(until_ms),
        remaining_ms: (remaining_ms > 0).then_some(remaining_ms),
    }
}

fn contract_exchange_statuses(
    flow_state: &FlowState,
    venue_health: Option<&VenueHealthMap>,
    enabled: bool,
    now: i64,
) -> BTreeMap<String, ContractWhaleExchangeStatus> {
    let mut statuses = default_exchange_statuses();
    let flow_last_trades = flow_last_trades(flow_state);
    let runtime_config = contract_whale_runtime_config();
    for exchange in runtime_config.platform_keys() {
        let platform = runtime_config
            .exchange_platform(&exchange)
            .expect("known contract whale platform");
        let platform_enabled = enabled && platform.any_market_enabled();
        let contract_enabled = enabled && runtime_config.exchange_enabled(&exchange);
        let last_trade_at = if contract_enabled {
            max_option(
                flow_last_trades.get(&exchange).copied().flatten(),
                health_for_exchange(venue_health, &exchange).and_then(health_last_trade_at),
            )
        } else if platform_enabled
            && platform
                .market_enabled(crate::contract_whale_monitor::types::ContractWhaleMarketType::Spot)
        {
            health_for_exchange(venue_health, &exchange).and_then(health_last_trade_at)
        } else {
            None
        };
        let reconnect_count = health_for_exchange(venue_health, &exchange)
            .map(|health| health.ws_reconnect_count.max(health.reconnect_count))
            .unwrap_or(0);
        let health_connected = health_for_exchange(venue_health, &exchange)
            .map(health_connected)
            .unwrap_or(false);
        let flow_connected = last_trade_at.is_some_and(|ts| now.saturating_sub(ts) <= 30_000);
        let connected = contract_enabled && (health_connected || flow_connected);
        let status = exchange_status_label(
            platform_enabled,
            contract_enabled,
            connected,
            health_for_exchange(venue_health, &exchange),
        );
        let latency_ms = last_trade_at
            .map(|ts| now.saturating_sub(ts).max(0))
            .filter(|latency| *latency <= 24 * 60 * 60 * 1000);
        statuses.insert(
            exchange.clone(),
            ContractWhaleExchangeStatus {
                connected,
                status: status.to_string(),
                last_trade_at,
                latency_ms,
                reconnect_count,
                platform_enabled,
                contract_enabled,
                enabled_markets: platform.enabled_markets(),
                market_roles: platform.enabled_market_roles(),
            },
        );
    }
    statuses
}

fn default_exchange_statuses() -> BTreeMap<String, ContractWhaleExchangeStatus> {
    contract_whale_runtime_config()
        .platform_keys()
        .into_iter()
        .map(|exchange| {
            (
                exchange,
                ContractWhaleExchangeStatus {
                    connected: false,
                    status: "disabled".to_string(),
                    last_trade_at: None,
                    latency_ms: None,
                    reconnect_count: 0,
                    platform_enabled: false,
                    contract_enabled: false,
                    enabled_markets: Vec::new(),
                    market_roles: BTreeMap::new(),
                },
            )
        })
        .collect()
}

fn flow_last_trades(flow_state: &FlowState) -> BTreeMap<String, Option<i64>> {
    let mut last_trades: BTreeMap<String, Option<i64>> = BTreeMap::new();
    let runtime_config = contract_whale_runtime_config();
    for window in flow_state.windows.values() {
        for (exchange, breakdown) in &window.venue_breakdown {
            if runtime_config.exchange_platform(exchange).is_none() {
                continue;
            }
            let candidate = breakdown.last_trade_ts.or_else(|| {
                let total_volume = breakdown.aggressive_buy_btc + breakdown.aggressive_sell_btc;
                (total_volume > 0.0).then_some(window.now_ts)
            });
            let current = last_trades.entry(exchange.clone()).or_insert(None);
            *current = max_option(*current, candidate);
        }
    }
    last_trades
}

fn health_for_exchange<'a>(
    venue_health: Option<&'a VenueHealthMap>,
    exchange: &str,
) -> Option<&'a VenueHealth> {
    venue_health?.get(exchange)
}

fn health_last_trade_at(health: &VenueHealth) -> Option<i64> {
    [
        health.last_trade_ts,
        health.last_trade_message_at_ms,
        health.last_parsed_trade_at_ms,
    ]
    .into_iter()
    .flatten()
    .max()
}

fn health_connected(health: &VenueHealth) -> bool {
    health.ws_connected
        || health.trade_active
        || matches!(health.status, VenueConnectionStatus::Connected)
}

fn exchange_status_label(
    platform_enabled: bool,
    contract_enabled: bool,
    connected: bool,
    health: Option<&VenueHealth>,
) -> &'static str {
    if !platform_enabled {
        return "disabled";
    }
    if !contract_enabled {
        return "spot_only";
    }
    if connected {
        return "connected";
    }
    match health.map(|item| item.status) {
        Some(VenueConnectionStatus::Reconnecting) | Some(VenueConnectionStatus::Connecting) => {
            "reconnecting"
        }
        Some(VenueConnectionStatus::Disabled) => "disabled",
        _ => "disconnected",
    }
}

fn max_option(left: Option<i64>, right: Option<i64>) -> Option<i64> {
    match (left, right) {
        (Some(left), Some(right)) => Some(left.max(right)),
        (Some(value), None) | (None, Some(value)) => Some(value),
        (None, None) => None,
    }
}

fn status_code(severity: ContractWhaleSeverity) -> &'static str {
    match severity {
        ContractWhaleSeverity::S | ContractWhaleSeverity::Critical => "strong",
        ContractWhaleSeverity::High | ContractWhaleSeverity::Medium => "active",
        ContractWhaleSeverity::Calm => "calm",
    }
}

fn direction_key(
    direction: crate::contract_whale_monitor::types::ContractWhaleDirection,
) -> &'static str {
    match direction {
        crate::contract_whale_monitor::types::ContractWhaleDirection::Buy => "buy",
        crate::contract_whale_monitor::types::ContractWhaleDirection::Sell => "sell",
        crate::contract_whale_monitor::types::ContractWhaleDirection::Absorption => "absorption",
        crate::contract_whale_monitor::types::ContractWhaleDirection::Suppression => "suppression",
    }
}

fn severity_matches(severity: ContractWhaleSeverity, filter: Option<&str>) -> bool {
    let Some(filter) = filter else {
        return true;
    };
    match filter.to_ascii_lowercase().as_str() {
        "s" => severity == ContractWhaleSeverity::S,
        "critical" => severity == ContractWhaleSeverity::Critical,
        "high" => severity == ContractWhaleSeverity::High,
        "medium" => severity == ContractWhaleSeverity::Medium,
        _ => true,
    }
}

pub fn parse_history_query(
    query: &ContractWhaleQuery,
) -> Result<ContractWhaleSignalQuery, (StatusCode, Json<serde_json::Value>)> {
    let mut from_ts = parse_optional_i64(query.from.as_deref(), "from")?;
    let to_ts = parse_optional_i64(query.to.as_deref(), "to")?;
    if from_ts.is_none() {
        from_ts = parse_range_start_ms(query.range.as_deref())?;
    }
    if let (Some(from_ts), Some(to_ts)) = (from_ts, to_ts) {
        if from_ts > to_ts {
            return Err(bad_request("from_must_be_before_to"));
        }
    }
    let parsed_cursor = parse_cursor(query.cursor.as_deref())?;
    let offset = match parsed_cursor.as_ref().and_then(cursor_offset) {
        Some(cursor) => cursor,
        None => parse_offset(query.offset.as_deref())?,
    };
    Ok(ContractWhaleSignalQuery {
        symbol: parse_symbol_filter(query.symbol.as_deref())?,
        severity: parse_severity_filter(query.severity.as_deref())?,
        signal_type: parse_signal_type_filter(query.signal_type.as_deref())?,
        direction: parse_direction_filter(query.direction.as_deref())?,
        discord_sent: parse_optional_bool(query.discord_sent.as_deref(), "discord_sent")?,
        window_sec: parse_window_sec_filter(query.window_sec.as_deref())?,
        exchange: parse_exchange_filter(query.exchange.as_deref())?,
        min_abs_net_volume_btc: parse_net_direction_filter(query.net_direction.as_deref())?,
        from_ts,
        to_ts,
        cursor_ts: parsed_cursor.as_ref().and_then(cursor_ts),
        cursor_signal_id: parsed_cursor.as_ref().and_then(cursor_signal_id),
        limit: parse_limit(query.limit.as_deref(), 50, 500)?,
        offset,
    })
}

fn parse_symbol_for_latest(
    symbol: Option<&str>,
) -> Result<String, (StatusCode, Json<serde_json::Value>)> {
    Ok(parse_symbol_filter(symbol)?.unwrap_or_else(|| "BTC".to_string()))
}

fn parse_symbol_filter(
    symbol: Option<&str>,
) -> Result<Option<String>, (StatusCode, Json<serde_json::Value>)> {
    let Some(symbol) = symbol.map(str::trim).filter(|value| !value.is_empty()) else {
        return Ok(None);
    };
    if !symbol
        .chars()
        .all(|ch| ch.is_ascii_alphanumeric() || matches!(ch, '-' | '_' | '.' | ':' | '/'))
    {
        return Err(bad_request("symbol_invalid"));
    }
    Ok(Some(symbol.to_ascii_uppercase()))
}

fn parse_severity_filter(
    filter: Option<&str>,
) -> Result<Option<ContractWhaleSeverity>, (StatusCode, Json<serde_json::Value>)> {
    let Some(filter) = filter.map(str::trim).filter(|value| !value.is_empty()) else {
        return Ok(None);
    };
    match filter.to_ascii_lowercase().as_str() {
        "s" => Ok(Some(ContractWhaleSeverity::S)),
        "critical" => Ok(Some(ContractWhaleSeverity::Critical)),
        "high" => Ok(Some(ContractWhaleSeverity::High)),
        "medium" => Ok(Some(ContractWhaleSeverity::Medium)),
        "calm" => Ok(Some(ContractWhaleSeverity::Calm)),
        _ => Err(bad_request("severity_invalid")),
    }
}

fn parse_signal_type_filter(
    filter: Option<&str>,
) -> Result<Option<ContractWhaleSignalType>, (StatusCode, Json<serde_json::Value>)> {
    let Some(filter) = filter.map(str::trim).filter(|value| !value.is_empty()) else {
        return Ok(None);
    };
    match normalize_token(filter).as_str() {
        "aggressivebuy" | "aggressive_buy" => Ok(Some(ContractWhaleSignalType::AggressiveBuy)),
        "aggressivesell" | "aggressive_sell" => Ok(Some(ContractWhaleSignalType::AggressiveSell)),
        "downsideabsorption" | "downside_absorption" => {
            Ok(Some(ContractWhaleSignalType::DownsideAbsorption))
        }
        "upsidesuppression" | "upside_suppression" => {
            Ok(Some(ContractWhaleSignalType::UpsideSuppression))
        }
        _ => Err(bad_request("signal_type_invalid")),
    }
}

fn parse_direction_filter(
    filter: Option<&str>,
) -> Result<Option<ContractWhaleDirection>, (StatusCode, Json<serde_json::Value>)> {
    let Some(filter) = filter.map(str::trim).filter(|value| !value.is_empty()) else {
        return Ok(None);
    };
    match normalize_token(filter).as_str() {
        "buy" | "long" | "activebuy" | "active_buy" | "aggressivebuy" | "aggressive_buy"
        | "主动买入" => Ok(Some(ContractWhaleDirection::Buy)),
        "sell" | "short" | "activesell" | "active_sell" | "aggressivesell" | "aggressive_sell"
        | "主动卖出" => Ok(Some(ContractWhaleDirection::Sell)),
        "absorption" | "downsideabsorption" | "downside_absorption" => {
            Ok(Some(ContractWhaleDirection::Absorption))
        }
        "suppression" | "upsidesuppression" | "upside_suppression" => {
            Ok(Some(ContractWhaleDirection::Suppression))
        }
        _ => Err(bad_request("direction_invalid")),
    }
}

fn parse_exchange_filter(
    filter: Option<&str>,
) -> Result<Option<String>, (StatusCode, Json<serde_json::Value>)> {
    let Some(filter) = filter.map(str::trim).filter(|value| !value.is_empty()) else {
        return Ok(None);
    };
    match filter.to_ascii_lowercase().as_str() {
        "binance" | "okx" | "bitfinex" | "coinbase" => Ok(Some(filter.to_ascii_lowercase())),
        _ => Err(bad_request("exchange_invalid")),
    }
}

fn parse_window_sec_filter(
    filter: Option<&str>,
) -> Result<Option<u64>, (StatusCode, Json<serde_json::Value>)> {
    let Some(filter) = filter.map(str::trim).filter(|value| !value.is_empty()) else {
        return Ok(None);
    };
    match filter.parse::<u64>() {
        Ok(window_sec @ (5 | 15 | 60)) => Ok(Some(window_sec)),
        _ => Err(bad_request("window_sec_invalid")),
    }
}

fn parse_net_direction_filter(
    filter: Option<&str>,
) -> Result<Option<f64>, (StatusCode, Json<serde_json::Value>)> {
    let Some(filter) = filter.map(str::trim).filter(|value| !value.is_empty()) else {
        return Ok(None);
    };
    if filter.eq_ignore_ascii_case("all") {
        return Ok(None);
    }
    match normalize_token(filter).as_str() {
        "abs500" | "gte500" | "min500" | "500" => Ok(Some(500.0)),
        "abs1000" | "gte1000" | "min1000" | "1000" => Ok(Some(1000.0)),
        _ => Err(bad_request("net_direction_invalid")),
    }
}

fn normalize_token(value: &str) -> String {
    value
        .trim()
        .to_ascii_lowercase()
        .replace(['-', ' ', '/'], "_")
}

fn parse_limit(
    value: Option<&str>,
    default: usize,
    max: usize,
) -> Result<usize, (StatusCode, Json<serde_json::Value>)> {
    let Some(value) = value.map(str::trim).filter(|value| !value.is_empty()) else {
        return Ok(default);
    };
    let limit = value
        .parse::<usize>()
        .map_err(|_| bad_request("limit_invalid"))?;
    Ok(limit.clamp(1, max))
}

fn parse_offset(value: Option<&str>) -> Result<usize, (StatusCode, Json<serde_json::Value>)> {
    let Some(value) = value.map(str::trim).filter(|value| !value.is_empty()) else {
        return Ok(0);
    };
    value
        .parse::<usize>()
        .map_err(|_| bad_request("offset_invalid"))
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ContractHistoryCursor {
    Offset(usize),
    Positioned { ts: i64, signal_id: String },
}

pub fn encode_contract_history_cursor(ts: i64, signal_id: &str) -> String {
    BASE64_STANDARD.encode(format!("{ts}|{signal_id}"))
}

fn parse_cursor(
    value: Option<&str>,
) -> Result<Option<ContractHistoryCursor>, (StatusCode, Json<serde_json::Value>)> {
    let Some(value) = value.map(str::trim).filter(|value| !value.is_empty()) else {
        return Ok(None);
    };
    if let Ok(offset) = value.parse::<usize>() {
        return Ok(Some(ContractHistoryCursor::Offset(offset)));
    }

    let decoded = BASE64_STANDARD
        .decode(value)
        .map_err(|_| bad_request("cursor_invalid"))?;
    let decoded = String::from_utf8(decoded).map_err(|_| bad_request("cursor_invalid"))?;
    let Some((ts_raw, signal_id_raw)) = decoded.split_once('|') else {
        return Err(bad_request("cursor_invalid"));
    };
    let ts = ts_raw
        .trim()
        .parse::<i64>()
        .map_err(|_| bad_request("cursor_invalid"))?;
    let signal_id = signal_id_raw.trim();
    if signal_id.is_empty() {
        return Err(bad_request("cursor_invalid"));
    }
    Ok(Some(ContractHistoryCursor::Positioned {
        ts,
        signal_id: signal_id.to_string(),
    }))
}

fn cursor_offset(cursor: &ContractHistoryCursor) -> Option<usize> {
    match cursor {
        ContractHistoryCursor::Offset(offset) => Some(*offset),
        ContractHistoryCursor::Positioned { .. } => None,
    }
}

fn cursor_ts(cursor: &ContractHistoryCursor) -> Option<i64> {
    match cursor {
        ContractHistoryCursor::Offset(_) => None,
        ContractHistoryCursor::Positioned { ts, .. } => Some(*ts),
    }
}

fn cursor_signal_id(cursor: &ContractHistoryCursor) -> Option<String> {
    match cursor {
        ContractHistoryCursor::Offset(_) => None,
        ContractHistoryCursor::Positioned { signal_id, .. } => Some(signal_id.clone()),
    }
}

fn parse_range_start_ms(
    value: Option<&str>,
) -> Result<Option<i64>, (StatusCode, Json<serde_json::Value>)> {
    let Some(value) = value.map(str::trim).filter(|value| !value.is_empty()) else {
        return Ok(None);
    };
    if value.eq_ignore_ascii_case("all") {
        return Ok(None);
    }
    let now = now_ms();
    let delta_ms = match value.to_ascii_lowercase().as_str() {
        "15m" => 15 * 60 * 1000,
        "1h" => 60 * 60 * 1000,
        "4h" => 4 * 60 * 60 * 1000,
        "24h" => 24 * 60 * 60 * 1000,
        "7d" => 7 * 24 * 60 * 60 * 1000,
        _ => return Err(bad_request("range_invalid")),
    };
    Ok(Some(now.saturating_sub(delta_ms)))
}

fn parse_optional_bool(
    value: Option<&str>,
    field: &str,
) -> Result<Option<bool>, (StatusCode, Json<serde_json::Value>)> {
    let Some(value) = value.map(str::trim).filter(|value| !value.is_empty()) else {
        return Ok(None);
    };
    match value.to_ascii_lowercase().as_str() {
        "true" | "1" | "yes" => Ok(Some(true)),
        "false" | "0" | "no" => Ok(Some(false)),
        _ => Err(bad_request(&format!("{field}_invalid"))),
    }
}

fn parse_optional_i64(
    value: Option<&str>,
    field: &str,
) -> Result<Option<i64>, (StatusCode, Json<serde_json::Value>)> {
    let Some(value) = value.map(str::trim).filter(|value| !value.is_empty()) else {
        return Ok(None);
    };
    value
        .parse::<i64>()
        .map(Some)
        .map_err(|_| bad_request(&format!("{field}_invalid")))
}

fn bad_request(reason: &str) -> (StatusCode, Json<serde_json::Value>) {
    (
        StatusCode::BAD_REQUEST,
        Json(serde_json::json!({
            "error": "bad_request",
            "reason": reason,
            "readOnly": true,
            "executionEnabled": false
        })),
    )
}
