use std::collections::{BTreeMap, BTreeSet};

use axum::{
    extract::{Query, State},
    http::{header, HeaderValue, StatusCode},
    response::{IntoResponse, Response},
    Json,
};
use serde::Serialize;

use crate::{
    api::contract_event_projection_runtime::{
        ContractEventProjectionValue, ProjectionFailure, ProjectionKey, ProjectionOutcome,
        ProjectionUnavailable, ProjectionUnavailableReason,
    },
    api::contract_retention_runtime::ContractRetentionSnapshotOutcome,
    api::contract_timeline_routes::{build_canonical_timeline_meta, CanonicalTimelineMeta},
    api::contract_whale_routes::{
        build_contract_whale_items_response, decorate_contract_whale_oi_contexts,
        decorate_price_deviation_signals, encode_contract_history_cursor, parse_history_query,
        ContractWhaleQuery,
    },
    app::AppState,
    contract_whale_monitor::{
        cluster::apply_contract_whale_signal_clusters,
        config::contract_whale_runtime_config,
        discord::{
            contract_whale_min_display_total_volume_btc, meets_contract_whale_display_total_volume,
        },
        event_lifecycle::{
            apply_contract_whale_event_lifecycle, enrich_lifecycle_unique_turnover,
            lifecycle_raw_start_ts, ContractWhaleLifecycleClock,
        },
        event_quality::decorate_contract_whale_event_quality,
        merge::merge_contract_whale_signals,
        trajectory::apply_contract_whale_trajectories,
        types::{
            ContractWhaleDirection, ContractWhaleSeverity, ContractWhaleSignal,
            ContractWhaleTrend60s,
        },
    },
    core_event::final_store::final_event_store::{
        build_final_events_from_contract_whale_signals, FinalEvent, VolumeDisplayContext,
    },
    normalizers::trade::now_ms,
    storage::{
        contract_whale_repo::{
            ContractWhaleRepo, ContractWhaleSignalQuery,
            CONTRACT_WHALE_PERMANENT_NET_DIRECTION_THRESHOLD_BTC,
        },
        SqliteStore,
    },
};

type ApiJsonResult<T = serde_json::Value> = Result<Json<T>, (StatusCode, Json<serde_json::Value>)>;
const FINAL_EVENTS_V2_CACHE_TTL_SEC: i64 = 30;

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ContractEventItem {
    pub event_id: String,
    pub source_signal_id: Option<String>,
    pub symbol: String,
    pub price: Option<f64>,
    pub ts: i64,
    pub status: String,
    pub signal_type: String,
    pub severity: String,
    pub window_sec: u64,
    pub volume_btc: f64,
    pub notional_usd: f64,
    pub net_volume_btc: f64,
    pub direction: String,
    pub net_direction: String,
    pub main_force_score: Option<u8>,
    pub exchange_spot_count: usize,
    pub exchange_contract_count: usize,
    pub source: String,
    pub is_retention_protected: bool,
    pub retention_reason: Option<String>,
    pub is_visible: bool,
    pub hidden_reason: Option<String>,
    pub hidden_detail: Option<String>,
    #[serde(flatten)]
    pub final_event: FinalEvent,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ContractEventPage {
    pub items: Vec<ContractEventItem>,
    pub data_state: String,
    pub degraded: bool,
    pub error_code: Option<String>,
    pub last_known_data_available: bool,
    pub next_cursor: Option<String>,
    pub has_more: bool,
    pub limit: usize,
    pub range: String,
    pub server_time: i64,
    pub last_event_ts: Option<i64>,
    pub max_event_ts: Option<i64>,
    pub max_persisted_at: Option<i64>,
    pub history_lag_sec: i64,
    pub latest_lag_sec: i64,
    pub cache_age_sec: i64,
    pub cache_ttl_sec: i64,
    pub timeline: CanonicalTimelineMeta,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct FinalEventsV2Response {
    pub active: Vec<FinalEvent>,
    pub closed: Vec<FinalEvent>,
    pub data_state: String,
    pub degraded: bool,
    pub error_code: Option<String>,
    pub last_known_data_available: bool,
    pub next_cursor: Option<String>,
    pub has_more: bool,
    pub limit: usize,
    pub range: String,
    pub server_time: i64,
    pub last_event_ts: Option<i64>,
    pub max_event_ts: Option<i64>,
    pub generated_at: i64,
    pub cache_age_sec: i64,
    pub cache_ttl_sec: i64,
    pub projection_lag_sec: i64,
    pub timeline: CanonicalTimelineMeta,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct RetentionTableStats {
    pub oldest_ts: Option<i64>,
    pub newest_ts: Option<i64>,
    pub row_count: Option<i64>,
    pub rows_older_than_retention: Option<i64>,
    pub protected_s_count: Option<i64>,
    pub protected_net_volume_count: Option<i64>,
    pub has_retention_cleanup: Option<bool>,
    pub reason: Option<String>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ContractRetentionStatusResponse {
    pub flow_retention_days: i64,
    pub signal_retention_days: i64,
    pub impact_b_retention_days: i64,
    pub signal_protect_severity_s: bool,
    pub signal_protect_impact_a_s: bool,
    pub signal_protect_net_volume_btc: f64,
    pub cleanup_interval_hours: i64,
    pub tables: ContractRetentionTables,
    pub data_state: String,
    pub degraded: bool,
    pub last_known_data_available: bool,
    pub generated_at: Option<i64>,
    pub cache_age_sec: Option<u64>,
    pub retry_after_ms: Option<u64>,
    pub error_code: Option<String>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ContractRetentionTables {
    pub contract_flow_1s: RetentionTableStats,
    pub contract_whale_signals: RetentionTableStats,
    pub main_force_events: RetentionTableStats,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ContractEventDebugCountsResponse {
    pub symbol: String,
    pub range: String,
    pub generated_at: String,
    pub db: DebugDbCounts,
    pub api_query: ApiQueryDebugCounts,
    pub visibility: VisibilityDebugCounts,
    pub latest: LatestDebugCounts,
    pub final_events_v2: FinalEventsDebugCounts,
    pub latest_vs_history: Vec<LatestVsHistoryEntry>,
    pub final_events_projection: FinalEventsProjectionDebug,
    pub error: Option<String>,
}

#[derive(Debug, Clone, Serialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct DebugDbCounts {
    pub contract_whale_signals_total_24h: i64,
    pub contract_whale_signals_btc_24h: i64,
    pub oldest_ts: Option<i64>,
    pub newest_ts: Option<i64>,
}

#[derive(Debug, Clone, Serialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct ApiQueryDebugCounts {
    pub matched_before_filter: usize,
    pub matched_after_symbol_filter: usize,
    pub matched_after_range_filter: usize,
    pub matched_after_severity_filter: Option<usize>,
    pub matched_after_window_filter: Option<usize>,
    pub matched_after_direction_filter: Option<usize>,
    pub returned_items: usize,
    pub limit: usize,
}

#[derive(Debug, Clone, Serialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct VisibilityDebugCounts {
    pub visible_count: usize,
    pub hidden_count: usize,
    pub hidden_reasons: HiddenReasonCounts,
}

#[derive(Debug, Clone, Serialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct HiddenReasonCounts {
    pub price_deviation_gt_5pct: usize,
    pub missing_price: usize,
    pub bad_quality: usize,
    pub disabled_monitor: usize,
    pub unknown: usize,
}

#[derive(Debug, Clone, Serialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct LatestDebugCounts {
    pub latest_count: usize,
    pub latest_symbols: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct FinalEventsDebugCounts {
    pub active_count: usize,
    pub closed_count: usize,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct LatestVsHistoryEntry {
    pub latest_event_id: String,
    pub symbol: String,
    pub ts: i64,
    pub exists_in_history: bool,
    pub history_event_id: Option<String>,
    pub not_in_history_reason: Option<String>,
}

#[derive(Debug, Clone, Serialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct FinalEventsProjectionDebug {
    pub source: String,
    pub raw_signals: usize,
    pub after_filter: usize,
    pub merged_events: usize,
    pub active: usize,
    pub closed: usize,
    pub range: String,
}

#[derive(Debug, Clone)]
struct ContractEventCandidate {
    event: FinalEvent,
    is_visible: bool,
    hidden_reason: Option<String>,
    hidden_detail: Option<String>,
}

pub async fn contract_events_route(
    State(state): State<AppState>,
    Query(query): Query<ContractWhaleQuery>,
) -> Result<Response, Response> {
    let requested_limit =
        validate_contract_events_query(&query).map_err(IntoResponse::into_response)?;
    let include_hidden = parse_include_hidden(query.include_hidden.as_deref())
        .map_err(IntoResponse::into_response)?;
    let include_source_signal = parse_include_source_signal(query.include_source_signal.as_deref())
        .map_err(IntoResponse::into_response)?;
    let key = contract_projection_key("contract_events", &query, requested_limit, include_hidden);
    let runtime = state.contract_event_projection_runtime();
    let projection_state = state.clone();
    let projection_query = query.clone();
    let outcome = runtime
        .get_or_spawn(key, move || {
            contract_event_page_for_query(projection_state, projection_query)
                .map(ContractEventProjectionValue::ContractEvents)
                .map_err(|error| {
                    tracing::warn!(
                        status = %error.0,
                        "contract_event_projection_failed"
                    );
                    ProjectionFailure::new("contract_event_projection_failed")
                })
        })
        .await
        .map_err(projection_unavailable_response)?;
    let page =
        contract_event_page_from_outcome(outcome).map_err(projection_unavailable_response)?;
    Ok(contract_events_wire_response(page, include_source_signal))
}

pub async fn contract_events_debug_counts_route(
    State(state): State<AppState>,
    Query(query): Query<ContractWhaleQuery>,
) -> ApiJsonResult<ContractEventDebugCountsResponse> {
    let response =
        tokio::task::spawn_blocking(move || contract_event_debug_counts_for_query(state, query))
            .await
            .map_err(|error| {
                internal_error(anyhow::anyhow!("debug projection join failed: {error}"))
            })?;
    Ok(Json(response))
}

pub async fn final_events_v2_route(
    State(state): State<AppState>,
    Query(query): Query<ContractWhaleQuery>,
) -> Result<Json<FinalEventsV2Response>, Response> {
    let requested_limit =
        validate_final_events_query(&query).map_err(IntoResponse::into_response)?;
    let key = contract_projection_key("final_events_v2", &query, requested_limit, false);
    let runtime = state.contract_event_projection_runtime();
    let projection_state = state.clone();
    let projection_query = query.clone();
    let outcome = runtime
        .get_or_spawn(key, move || {
            final_events_v2_for_query(projection_state, projection_query)
                .map(ContractEventProjectionValue::FinalEventsV2)
                .map_err(|error| {
                    tracing::warn!(
                        status = %error.0,
                        "final_events_v2_projection_failed"
                    );
                    ProjectionFailure::new("final_events_v2_projection_failed")
                })
        })
        .await
        .map_err(projection_unavailable_response)?;
    let page =
        final_events_v2_page_from_outcome(outcome).map_err(projection_unavailable_response)?;
    Ok(Json(page))
}

fn validate_contract_events_query(
    query: &ContractWhaleQuery,
) -> Result<usize, (StatusCode, Json<serde_json::Value>)> {
    let requested_limit = parse_requested_limit(query.limit.as_deref(), 100, 500)?;
    parse_include_hidden(query.include_hidden.as_deref())?;
    let mut history_query = query.clone();
    history_query.limit = Some((requested_limit + 1).to_string());
    parse_history_query(&history_query)?;
    Ok(requested_limit)
}

fn validate_final_events_query(
    query: &ContractWhaleQuery,
) -> Result<usize, (StatusCode, Json<serde_json::Value>)> {
    let requested_limit = parse_requested_limit(query.limit.as_deref(), 100, 500)?;
    let mut history_query = query.clone();
    history_query.limit = Some((requested_limit + 1).to_string());
    parse_history_query(&history_query)?;
    Ok(requested_limit)
}

fn contract_projection_key(
    view: &str,
    query: &ContractWhaleQuery,
    requested_limit: usize,
    include_hidden: bool,
) -> ProjectionKey {
    let normalize = |value: Option<&String>| {
        value
            .map(|value| value.trim().to_ascii_lowercase())
            .unwrap_or_default()
    };
    let symbol = query
        .symbol
        .as_deref()
        .unwrap_or("BTC")
        .trim()
        .to_ascii_uppercase();
    let range = query
        .range
        .as_deref()
        .unwrap_or("24h")
        .trim()
        .to_ascii_lowercase();
    let min_notional_bits = query
        .min_notional_usd
        .as_deref()
        .and_then(|value| value.trim().parse::<f64>().ok())
        .filter(|value| value.is_finite() && *value > 0.0)
        .map(f64::to_bits)
        .unwrap_or_default();
    ProjectionKey::new(
        serde_json::json!({
            "view": view,
            "symbol": symbol,
            "severity": normalize(query.severity.as_ref()),
            "signalType": normalize(query.signal_type.as_ref()),
            "direction": normalize(query.direction.as_ref()),
            "discordSent": normalize(query.discord_sent.as_ref()),
            "windowSec": normalize(query.window_sec.as_ref()),
            "exchange": normalize(query.exchange.as_ref()),
            "netDirection": normalize(query.net_direction.as_ref()),
            "impactLevel": normalize(query.impact_level.as_ref()),
            "status": normalize_status_filter(query.status.as_deref()).unwrap_or_else(|| "all".to_string()),
            "range": range,
            "cursor": query.cursor.as_deref().map(str::trim).unwrap_or_default(),
            "from": normalize(query.from.as_ref()),
            "to": normalize(query.to.as_ref()),
            "offset": normalize(query.offset.as_ref()),
            "minNotionalBits": min_notional_bits,
            "includeHidden": include_hidden,
            "limit": requested_limit,
        })
        .to_string(),
    )
}

fn contract_event_page_from_outcome(
    outcome: ProjectionOutcome<ContractEventProjectionValue>,
) -> Result<ContractEventPage, ProjectionUnavailable> {
    match outcome {
        ProjectionOutcome::Fresh {
            value,
            cache_age,
            completed_at_ms,
        } => {
            match value {
                ContractEventProjectionValue::ContractEvents(page) => Ok(
                    serve_contract_event_page(page, cache_age, completed_at_ms, None),
                ),
                ContractEventProjectionValue::FinalEventsV2(_) => Err(projection_type_mismatch()),
            }
        }
        ProjectionOutcome::Stale {
            value,
            cache_age,
            completed_at_ms,
            reason,
        } => {
            match value {
                ContractEventProjectionValue::ContractEvents(page) => Ok(
                    serve_contract_event_page(page, cache_age, completed_at_ms, Some(reason)),
                ),
                ContractEventProjectionValue::FinalEventsV2(_) => Err(projection_type_mismatch()),
            }
        }
    }
}

fn final_events_v2_page_from_outcome(
    outcome: ProjectionOutcome<ContractEventProjectionValue>,
) -> Result<FinalEventsV2Response, ProjectionUnavailable> {
    match outcome {
        ProjectionOutcome::Fresh {
            value,
            cache_age,
            completed_at_ms,
        } => {
            match value {
                ContractEventProjectionValue::FinalEventsV2(page) => Ok(
                    serve_final_events_v2_page(page, cache_age, completed_at_ms, None),
                ),
                ContractEventProjectionValue::ContractEvents(_) => Err(projection_type_mismatch()),
            }
        }
        ProjectionOutcome::Stale {
            value,
            cache_age,
            completed_at_ms,
            reason,
        } => {
            match value {
                ContractEventProjectionValue::FinalEventsV2(page) => Ok(
                    serve_final_events_v2_page(page, cache_age, completed_at_ms, Some(reason)),
                ),
                ContractEventProjectionValue::ContractEvents(_) => Err(projection_type_mismatch()),
            }
        }
    }
}

fn serve_contract_event_page(
    mut page: ContractEventPage,
    cache_age: std::time::Duration,
    _completed_at_ms: i64,
    stale_reason: Option<ProjectionUnavailableReason>,
) -> ContractEventPage {
    let served_at = now_ms();
    page.server_time = served_at;
    page.cache_age_sec = duration_secs_i64(cache_age);
    page.cache_ttl_sec = FINAL_EVENTS_V2_CACHE_TTL_SEC;
    page.timeline.served_ts = served_at;
    if let Some(reason) = stale_reason {
        page.data_state = "stale".to_string();
        page.degraded = true;
        page.error_code = Some(reason.error_code().to_string());
        page.last_known_data_available = !page.items.is_empty();
    } else {
        page.degraded = false;
        page.error_code = None;
    }
    page
}

fn contract_events_wire_response(page: ContractEventPage, include_source_signal: bool) -> Response {
    let Ok(mut value) = serde_json::to_value(page) else {
        return (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(serde_json::json!({
                "dataState": "unavailable",
                "degraded": true,
                "errorCode": "contract_events_serialize_failed",
                "lastKnownDataAvailable": false,
                "readOnly": true,
                "executionEnabled": false,
            })),
        )
            .into_response();
    };
    if let Some(items) = value.get_mut("items").and_then(|items| items.as_array_mut()) {
        for item in items {
            promote_contract_event_tape_fields(item);
            if !include_source_signal {
                if let Some(object) = item.as_object_mut() {
                    object.remove("sourceSignal");
                }
            }
        }
    }
    Json(value).into_response()
}

fn promote_contract_event_tape_fields(item: &mut serde_json::Value) {
    let Some(object) = item.as_object_mut() else {
        return;
    };
    let Some(source_signal) = object.get("sourceSignal").cloned() else {
        return;
    };
    let Some(signal) = source_signal.as_object() else {
        return;
    };
    copy_json_field(object, signal, "mainExchange");
    copy_json_field(object, signal, "discordSent");
    copy_json_field(object, signal, "discordEligible");
    copy_json_field(object, signal, "discordReason");
    copy_json_field(object, signal, "discordWouldSend");
    copy_json_field(object, signal, "liquidationSuspected");
    copy_json_field(object, signal, "liquidationLongBtc");
    copy_json_field(object, signal, "liquidationShortBtc");
    copy_json_field(object, signal, "liquidationRatio");
    copy_json_field(object, signal, "fundingRate");
    copy_json_field(object, signal, "fundingBias");
    copy_json_field(object, signal, "oiChange5mBtc");
    copy_json_field(object, signal, "oiChange1mBtc");
    copy_json_field(object, signal, "oiChangePct");
    copy_json_field(object, signal, "oiBias");
    copy_json_field(object, signal, "score");
    copy_json_field(object, signal, "dynamicMultiple");
    copy_json_field(object, signal, "percentileLevel");
    copy_json_field(object, signal, "triggerPriceUsd");
    copy_json_field(object, signal, "orderPriceUsd");
    // Classification v2 is flattened on the nested signal; promote before sourceSignal is stripped
    // so the tape can render flow / price-response / OI semantics without the nested payload.
    copy_json_field(object, signal, "flowDirection");
    copy_json_field(object, signal, "priceResponseType");
    copy_json_field(object, signal, "priceResponseTypeV2");
    copy_json_field(object, signal, "displaySignalType");
    copy_json_field(object, signal, "structureInterpretation");
    copy_json_field(object, signal, "classificationReasons");
    copy_json_field(object, signal, "oiConsistentSources");
    copy_json_field(object, signal, "oiExcludedSources");
    copy_json_field(object, signal, "oiSourceCoverageChanged");
    copy_json_field(object, signal, "oiCrossExchangeConsensus");
    copy_json_field(object, signal, "oiEvidenceDegraded");
    copy_json_field(object, signal, "oiEvidenceReason");

    if !object.contains_key("eventLifecycle") {
        if let Some(lifecycle) = signal.get("eventLifecycle") {
            object.insert("eventLifecycle".to_string(), lifecycle.clone());
        }
    }
    if !object.contains_key("eventQuality") {
        if let Some(quality) = signal.get("eventQuality") {
            object.insert("eventQuality".to_string(), quality.clone());
        }
    }
}

fn copy_json_field(
    target: &mut serde_json::Map<String, serde_json::Value>,
    source: &serde_json::Map<String, serde_json::Value>,
    key: &str,
) {
    if target.contains_key(key) {
        return;
    }
    if let Some(value) = source.get(key) {
        target.insert(key.to_string(), value.clone());
    }
}

fn serve_final_events_v2_page(
    mut page: FinalEventsV2Response,
    cache_age: std::time::Duration,
    _completed_at_ms: i64,
    stale_reason: Option<ProjectionUnavailableReason>,
) -> FinalEventsV2Response {
    let served_at = now_ms();
    page.server_time = served_at;
    page.cache_age_sec = duration_secs_i64(cache_age);
    page.cache_ttl_sec = FINAL_EVENTS_V2_CACHE_TTL_SEC;
    page.projection_lag_sec = page
        .max_event_ts
        .map(|ts| served_at.saturating_sub(ts).max(0).saturating_div(1000))
        .unwrap_or(0);
    page.timeline.served_ts = served_at;
    if let Some(reason) = stale_reason {
        page.data_state = "stale".to_string();
        page.degraded = true;
        page.error_code = Some(reason.error_code().to_string());
        page.last_known_data_available = !page.active.is_empty() || !page.closed.is_empty();
    } else {
        page.degraded = false;
        page.error_code = None;
    }
    page
}

fn duration_secs_i64(duration: std::time::Duration) -> i64 {
    duration.as_secs().min(i64::MAX as u64) as i64
}

fn projection_type_mismatch() -> ProjectionUnavailable {
    ProjectionUnavailable {
        reason: ProjectionUnavailableReason::Failed,
        retry_after_ms: 2_000,
    }
}

fn projection_unavailable_response(error: ProjectionUnavailable) -> Response {
    let mut response = (
        StatusCode::SERVICE_UNAVAILABLE,
        Json(serde_json::json!({
            "dataState": "degraded",
            "degraded": true,
            "errorCode": error.error_code(),
            "lastKnownDataAvailable": false,
            "retryAfterMs": error.retry_after_ms,
            "readOnly": true,
            "executionEnabled": false,
        })),
    )
        .into_response();
    response
        .headers_mut()
        .insert(header::RETRY_AFTER, HeaderValue::from_static("2"));
    response
}

pub(crate) async fn contract_event_page_for_query_nonblocking(
    state: AppState,
    query: ContractWhaleQuery,
) -> Result<ContractEventPage, (StatusCode, Json<serde_json::Value>)> {
    let requested_limit = validate_contract_events_query(&query)?;
    let include_hidden = parse_include_hidden(query.include_hidden.as_deref())?;
    let key = contract_projection_key("contract_events", &query, requested_limit, include_hidden);
    let runtime = state.contract_event_projection_runtime();
    let projection_state = state.clone();
    let projection_query = query.clone();
    let outcome = runtime
        .get_or_spawn(key, move || {
            contract_event_page_for_query(projection_state, projection_query)
                .map(ContractEventProjectionValue::ContractEvents)
                .map_err(|_| ProjectionFailure::new("contract_event_projection_failed"))
        })
        .await
        .map_err(projection_unavailable_api_error)?;
    contract_event_page_from_outcome(outcome).map_err(projection_unavailable_api_error)
}

pub(crate) async fn final_events_v2_for_query_nonblocking(
    state: AppState,
    query: ContractWhaleQuery,
) -> Result<FinalEventsV2Response, (StatusCode, Json<serde_json::Value>)> {
    let requested_limit = validate_final_events_query(&query)?;
    let key = contract_projection_key("final_events_v2", &query, requested_limit, false);
    let runtime = state.contract_event_projection_runtime();
    let projection_state = state.clone();
    let projection_query = query.clone();
    let outcome = runtime
        .get_or_spawn(key, move || {
            final_events_v2_for_query(projection_state, projection_query)
                .map(ContractEventProjectionValue::FinalEventsV2)
                .map_err(|_| ProjectionFailure::new("final_events_v2_projection_failed"))
        })
        .await
        .map_err(projection_unavailable_api_error)?;
    final_events_v2_page_from_outcome(outcome).map_err(projection_unavailable_api_error)
}

fn projection_unavailable_api_error(
    error: ProjectionUnavailable,
) -> (StatusCode, Json<serde_json::Value>) {
    (
        StatusCode::SERVICE_UNAVAILABLE,
        Json(serde_json::json!({
            "dataState": "degraded",
            "degraded": true,
            "errorCode": error.error_code(),
            "lastKnownDataAvailable": false,
            "retryAfterMs": error.retry_after_ms,
            "readOnly": true,
            "executionEnabled": false,
        })),
    )
}

pub async fn contract_retention_status_route(
    State(state): State<AppState>,
) -> ApiJsonResult<ContractRetentionStatusResponse> {
    let retention = contract_whale_runtime_config().retention;
    let (
        tables,
        data_state,
        degraded,
        last_known_data_available,
        generated_at,
        cache_age_sec,
        retry_after_ms,
        error_code,
    ) = match state.contract_whale_store() {
        Some(store) => {
            let flow_days = retention.flow_1s_days;
            let signal_days = retention.signals_days;
            match state
                .contract_retention_runtime()
                .get_or_spawn(move || retention_tables(store, flow_days, signal_days))
                .await
            {
                ContractRetentionSnapshotOutcome::Fresh {
                    value,
                    cache_age,
                    generated_at_ms,
                } => (
                    value,
                    "fresh",
                    false,
                    true,
                    Some(generated_at_ms),
                    Some(cache_age.as_secs()),
                    None,
                    None,
                ),
                ContractRetentionSnapshotOutcome::Stale {
                    value,
                    cache_age,
                    generated_at_ms,
                } => (
                    value,
                    "stale",
                    true,
                    true,
                    Some(generated_at_ms),
                    Some(cache_age.as_secs()),
                    Some(2_000),
                    Some("contract_retention_refresh_in_progress".to_string()),
                ),
                ContractRetentionSnapshotOutcome::Refreshing => (
                    unavailable_retention_tables("refresh_in_progress"),
                    "degraded",
                    true,
                    false,
                    None,
                    None,
                    Some(2_000),
                    Some("contract_retention_refresh_in_progress".to_string()),
                ),
                ContractRetentionSnapshotOutcome::RefreshFailed => (
                    unavailable_retention_tables("refresh_failed"),
                    "degraded",
                    true,
                    false,
                    None,
                    None,
                    Some(2_000),
                    Some("contract_retention_refresh_failed".to_string()),
                ),
            }
        }
        None => (
            unavailable_retention_tables("query_failed"),
            "degraded",
            true,
            false,
            None,
            None,
            None,
            Some("contract_retention_store_unavailable".to_string()),
        ),
    };

    Ok(Json(ContractRetentionStatusResponse {
        flow_retention_days: retention.flow_1s_days,
        signal_retention_days: retention.signals_days,
        impact_b_retention_days: retention.impact_b_days,
        signal_protect_severity_s: true,
        signal_protect_impact_a_s: true,
        signal_protect_net_volume_btc: CONTRACT_WHALE_PERMANENT_NET_DIRECTION_THRESHOLD_BTC,
        cleanup_interval_hours: 1,
        tables,
        data_state: data_state.to_string(),
        degraded,
        last_known_data_available,
        generated_at,
        cache_age_sec,
        retry_after_ms,
        error_code,
    }))
}

pub(crate) fn contract_event_page_for_query(
    state: AppState,
    mut query: ContractWhaleQuery,
) -> Result<ContractEventPage, (StatusCode, Json<serde_json::Value>)> {
    let requested_limit = parse_requested_limit(query.limit.as_deref(), 100, 500)?;
    let include_hidden = parse_include_hidden(query.include_hidden.as_deref())?;
    let range = query.range.clone().unwrap_or_else(|| "7d".to_string());
    query.limit = Some((requested_limit + 1).to_string());
    let history_query = parse_history_query(&query)?;
    let store = state
        .contract_whale_store()
        .ok_or_else(|| internal_error(anyhow::anyhow!("contract whale store unavailable")))?;
    let raw_items = store
        .query_contract_whale_signals(&history_query)
        .map_err(internal_error)?;
    let raw_count = raw_items.len();
    let has_more = raw_items.len() > requested_limit;
    let sliced_items = raw_items
        .into_iter()
        .take(requested_limit)
        .collect::<Vec<_>>();
    let next_cursor = has_more
        .then(|| {
            sliced_items
                .last()
                .map(|signal| encode_contract_history_cursor(signal.ts, &signal.id))
        })
        .flatten();
    let now = now_ms();
    let last_event_ts = sliced_items.last().map(|signal| signal.ts);
    let max_event_ts = sliced_items.first().map(|signal| signal.ts);
    let max_persisted_at = state.contract_whale_store().and_then(|store| {
        let Some(symbol) = history_query.symbol.clone() else {
            return None;
        };
        store
            .with_connection(|conn| {
                let mut stmt = conn.prepare(
                    "SELECT MAX(created_at) FROM contract_whale_signals WHERE symbol = ?1",
                )?;
                let value =
                    stmt.query_row([symbol.as_str()], |row| row.get::<_, Option<i64>>(0))?;
                Ok(value)
            })
            .ok()
            .flatten()
    });
    let history_lag_sec = max_event_ts
        .map(|ts| now.saturating_sub(ts).max(0).saturating_div(1000))
        .unwrap_or(0);
    let cache_age_sec = max_persisted_at
        .map(|ts| now.saturating_sub(ts).max(0).saturating_div(1000))
        .unwrap_or(history_lag_sec);
    let latest_lag_sec = state
        .contract_whale_store()
        .and_then(|store| {
            let Some(symbol) = history_query.symbol.clone() else {
                return None;
            };
            store
                .query_contract_whale_signals(&ContractWhaleSignalQuery {
                    symbol: Some(symbol),
                    limit: 1,
                    ..ContractWhaleSignalQuery::default()
                })
                .ok()
        })
        .and_then(|rows| rows.first().map(|signal| signal.ts))
        .map(|latest_ts| {
            latest_ts
                .saturating_sub(max_event_ts.unwrap_or(latest_ts))
                .max(0)
                .saturating_div(1000)
        })
        .unwrap_or(0);
    let requested_status = normalize_status_filter(query.status.as_deref());
    let store = state.contract_whale_store();
    let items = project_contract_event_candidates(
        sliced_items,
        VolumeDisplayContext::ContractEventStream,
        store.as_ref(),
    )
    .into_iter()
    .filter(|candidate| status_matches(requested_status.as_deref(), &candidate.event.status))
    .filter(|candidate| include_hidden || candidate.is_visible)
    .map(contract_event_from_candidate)
    .collect::<Vec<_>>();

    Ok(ContractEventPage {
        items,
        data_state: if raw_count == 0 {
            "empty".to_string()
        } else {
            "fresh".to_string()
        },
        degraded: false,
        error_code: None,
        last_known_data_available: raw_count > 0,
        next_cursor,
        has_more,
        limit: requested_limit,
        range,
        server_time: now,
        last_event_ts,
        max_event_ts,
        max_persisted_at,
        history_lag_sec,
        latest_lag_sec,
        cache_age_sec,
        cache_ttl_sec: 5,
        timeline: build_canonical_timeline_meta(
            "contract_whale_signals",
            max_event_ts,
            max_persisted_at,
            max_persisted_at.or(max_event_ts),
            now,
        ),
    })
}

pub(crate) fn final_events_v2_for_query(
    state: AppState,
    mut query: ContractWhaleQuery,
) -> Result<FinalEventsV2Response, (StatusCode, Json<serde_json::Value>)> {
    let requested_limit = parse_requested_limit(query.limit.as_deref(), 100, 500)?;
    let range = query.range.clone().unwrap_or_else(|| "24h".to_string());
    let requested_status = normalize_status_filter(query.status.as_deref());
    query.limit = Some((requested_limit + 1).to_string());
    let history_query = parse_history_query(&query)?;
    let now = now_ms();
    let store = state
        .contract_whale_store()
        .ok_or_else(|| internal_error(anyhow::anyhow!("contract whale store unavailable")))?;
    let raw_items = store
        .query_contract_whale_signals(&history_query)
        .map_err(internal_error)?;
    let has_more = raw_items.len() > requested_limit;
    let sliced_items = raw_items
        .into_iter()
        .take(requested_limit)
        .collect::<Vec<_>>();
    let next_cursor = has_more
        .then(|| {
            sliced_items
                .last()
                .map(|signal| encode_contract_history_cursor(signal.ts, &signal.id))
        })
        .flatten();
    let last_event_ts = sliced_items.last().map(|signal| signal.ts);
    let max_event_ts = sliced_items.first().map(|signal| signal.ts);
    let mut active = Vec::new();
    let mut closed = Vec::new();
    let store = state.contract_whale_store();
    for candidate in project_contract_event_candidates(
        sliced_items,
        VolumeDisplayContext::FinalLifecycleEvent,
        store.as_ref(),
    ) {
        if !candidate.is_visible
            || !status_matches(requested_status.as_deref(), &candidate.event.status)
        {
            continue;
        }
        if candidate.event.status.eq_ignore_ascii_case("closed") {
            closed.push(candidate.event);
        } else {
            active.push(candidate.event);
        }
    }

    Ok(FinalEventsV2Response {
        active,
        closed,
        data_state: if max_event_ts.is_some() {
            "fresh".to_string()
        } else {
            "empty".to_string()
        },
        degraded: false,
        error_code: None,
        last_known_data_available: max_event_ts.is_some(),
        next_cursor,
        has_more,
        limit: requested_limit,
        range,
        server_time: now,
        last_event_ts,
        max_event_ts,
        generated_at: now,
        cache_age_sec: 0,
        cache_ttl_sec: FINAL_EVENTS_V2_CACHE_TTL_SEC,
        projection_lag_sec: max_event_ts
            .map(|ts| now.saturating_sub(ts).max(0).saturating_div(1000))
            .unwrap_or(0),
        timeline: build_canonical_timeline_meta(
            "contract_whale_signals",
            max_event_ts,
            max_event_ts,
            Some(now),
            now,
        ),
    })
}

fn contract_event_debug_counts_for_query(
    state: AppState,
    mut query: ContractWhaleQuery,
) -> ContractEventDebugCountsResponse {
    let generated_at_ms = now_ms();
    let requested_limit = parse_requested_limit(query.limit.as_deref(), 500, 500).unwrap_or(500);
    let range = query.range.clone().unwrap_or_else(|| "24h".to_string());
    let symbol = query
        .symbol
        .clone()
        .unwrap_or_else(|| "BTC".to_string())
        .to_ascii_uppercase();
    query.limit = Some(requested_limit.to_string());
    let history_query = match parse_history_query(&query) {
        Ok(history_query) => history_query,
        Err(_) => {
            return ContractEventDebugCountsResponse {
                symbol,
                range,
                generated_at: generated_at_ms.to_string(),
                db: DebugDbCounts::default(),
                api_query: ApiQueryDebugCounts {
                    limit: requested_limit,
                    ..ApiQueryDebugCounts::default()
                },
                visibility: VisibilityDebugCounts::default(),
                latest: LatestDebugCounts::default(),
                final_events_v2: FinalEventsDebugCounts::default(),
                latest_vs_history: Vec::new(),
                final_events_projection: FinalEventsProjectionDebug::default(),
                error: Some("bad_request".to_string()),
            };
        }
    };
    let Some(store) = state.contract_whale_store() else {
        return ContractEventDebugCountsResponse {
            symbol,
            range,
            generated_at: generated_at_ms.to_string(),
            db: DebugDbCounts::default(),
            api_query: ApiQueryDebugCounts {
                limit: requested_limit,
                ..ApiQueryDebugCounts::default()
            },
            visibility: VisibilityDebugCounts::default(),
            latest: LatestDebugCounts::default(),
            final_events_v2: FinalEventsDebugCounts::default(),
            latest_vs_history: Vec::new(),
            final_events_projection: FinalEventsProjectionDebug::default(),
            error: Some("query_failed".to_string()),
        };
    };

    let db = db_debug_counts(&store, &symbol, history_query.from_ts, history_query.to_ts)
        .unwrap_or_default();
    let db_contract_whale_signals_btc_24h = db.contract_whale_signals_btc_24h.max(0) as usize;
    let raw_items = store
        .query_contract_whale_signals(&history_query)
        .unwrap_or_default();
    let projected = project_contract_event_candidates(
        raw_items.clone(),
        VolumeDisplayContext::ContractEventStream,
        Some(&store),
    );
    let visibility = visibility_debug_counts(&projected);
    let visible_events = projected
        .iter()
        .filter(|candidate| candidate.is_visible)
        .map(|candidate| candidate.event.clone())
        .collect::<Vec<_>>();
    let final_counts = FinalEventsDebugCounts {
        active_count: visible_events
            .iter()
            .filter(|event| event.status.eq_ignore_ascii_case("active"))
            .count(),
        closed_count: visible_events
            .iter()
            .filter(|event| event.status.eq_ignore_ascii_case("closed"))
            .count(),
    };
    let final_active_count = final_counts.active_count;
    let final_closed_count = final_counts.closed_count;

    let latest_raw = store
        .query_contract_whale_signals(&ContractWhaleSignalQuery {
            symbol: Some(symbol.clone()),
            limit: 50,
            ..ContractWhaleSignalQuery::default()
        })
        .unwrap_or_default();
    let latest_response = build_contract_whale_items_response(
        latest_raw,
        &symbol,
        50,
        state.config().contract_whale_monitor.enabled,
        state.config().contract_whale_monitor.dry_run,
        BTreeMap::new(),
        ContractWhaleTrend60s::default(),
    );

    ContractEventDebugCountsResponse {
        symbol: symbol.clone(),
        range: range.clone(),
        generated_at: generated_at_ms.to_string(),
        db,
        api_query: ApiQueryDebugCounts {
            matched_before_filter: db_contract_whale_signals_btc_24h,
            matched_after_symbol_filter: db_contract_whale_signals_btc_24h,
            matched_after_range_filter: raw_items.len(),
            matched_after_severity_filter: history_query.severity.map(|_| raw_items.len()),
            matched_after_window_filter: history_query.window_sec.map(|_| raw_items.len()),
            matched_after_direction_filter: history_query.direction.map(|_| raw_items.len()),
            returned_items: projected
                .iter()
                .filter(|candidate| candidate.is_visible)
                .count(),
            limit: requested_limit,
        },
        visibility,
        latest: LatestDebugCounts {
            latest_count: latest_response.items.len(),
            latest_symbols: latest_response
                .items
                .iter()
                .map(|item| item.symbol.clone())
                .collect(),
        },
        final_events_v2: final_counts,
        latest_vs_history: latest_vs_history(
            &latest_response.items,
            &projected,
            history_query.from_ts,
            history_query.to_ts,
        ),
        final_events_projection: FinalEventsProjectionDebug {
            source: "contract_whale_signals".to_string(),
            raw_signals: raw_items.len(),
            after_filter: projected
                .iter()
                .filter(|candidate| candidate.is_visible)
                .count(),
            merged_events: visible_events.len(),
            active: final_active_count,
            closed: final_closed_count,
            range,
        },
        error: None,
    }
}

fn project_contract_event_candidates(
    raw_items: Vec<ContractWhaleSignal>,
    context: VolumeDisplayContext,
    store: Option<&SqliteStore>,
) -> Vec<ContractEventCandidate> {
    if raw_items.is_empty() {
        return Vec::new();
    }
    let mut items = merge_contract_whale_signals(raw_items);
    decorate_price_deviation_signals(
        &mut items,
        None,
        contract_whale_runtime_config()
            .toxic_order
            .max_price_deviation_pct,
    );
    items = apply_contract_whale_event_lifecycle(
        items,
        ContractWhaleLifecycleClock::Live { now_ms: now_ms() },
    );
    if let Some(store) = store {
        let mut ranges_by_symbol = BTreeMap::<String, Vec<(i64, i64)>>::new();
        for signal in &items {
            let start_ts = lifecycle_raw_start_ts(signal);
            let end_ts = signal.event_lifecycle.last_update_time.max(signal.ts);
            ranges_by_symbol
                .entry(signal.symbol.to_ascii_uppercase())
                .or_default()
                .push((start_ts, end_ts));
        }
        let mut buckets = Vec::new();
        let mut failed_symbols = BTreeSet::new();
        for (symbol, ranges) in coalesce_lifecycle_flow_ranges(ranges_by_symbol) {
            for (from_ts, to_ts) in ranges {
                match store.list_contract_flow_buckets_between(&symbol, from_ts, to_ts) {
                    Ok(mut symbol_buckets) => buckets.append(&mut symbol_buckets),
                    Err(error) => {
                        tracing::warn!(
                            symbol = %symbol,
                            from_ts,
                            to_ts,
                            error = %error,
                            "contract lifecycle raw-flow range query failed"
                        );
                        failed_symbols.insert(symbol.clone());
                        break;
                    }
                }
            }
        }
        enrich_lifecycle_unique_turnover(&mut items, &buckets, &failed_symbols);
    }
    items = decorate_contract_whale_event_quality(items);
    if let Some(store) = store {
        decorate_contract_whale_oi_contexts(store, &mut items);
    }
    apply_contract_whale_signal_clusters(&mut items);
    apply_contract_whale_trajectories(&mut items);
    items.sort_by(|left, right| {
        right
            .ts
            .cmp(&left.ts)
            .then_with(|| right.severity.rank().cmp(&left.severity.rank()))
            .then_with(|| right.score.cmp(&left.score))
    });
    let final_events = build_final_events_from_contract_whale_signals(&items, context);
    items
        .into_iter()
        .zip(final_events)
        .map(|(signal, event)| {
            let (is_visible, hidden_reason, hidden_detail) =
                visibility_metadata(&signal, event.display_volume_btc);
            ContractEventCandidate {
                event,
                is_visible,
                hidden_reason,
                hidden_detail,
            }
        })
        .collect()
}

fn coalesce_lifecycle_flow_ranges(
    mut ranges_by_symbol: BTreeMap<String, Vec<(i64, i64)>>,
) -> BTreeMap<String, Vec<(i64, i64)>> {
    for ranges in ranges_by_symbol.values_mut() {
        for range in ranges.iter_mut() {
            if range.0 > range.1 {
                std::mem::swap(&mut range.0, &mut range.1);
            }
        }
        ranges.sort_unstable_by_key(|range| (range.0, range.1));

        let mut merged = Vec::<(i64, i64)>::with_capacity(ranges.len());
        for (start_ts, end_ts) in ranges.drain(..) {
            match merged.last_mut() {
                Some(current) if start_ts <= current.1.saturating_add(1_000) => {
                    current.1 = current.1.max(end_ts);
                }
                _ => merged.push((start_ts, end_ts)),
            }
        }
        *ranges = merged;
    }
    ranges_by_symbol
}

fn visibility_metadata(
    signal: &ContractWhaleSignal,
    display_volume_btc: f64,
) -> (bool, Option<String>, Option<String>) {
    if signal.price_deviation_filtered {
        let detail = signal
            .price_deviation_pct
            .map(|value| format!("price deviation {:.2}% > max 5%", value))
            .or_else(|| Some("price deviation exceeded configured max".to_string()));
        return (false, Some("price_deviation_gt_5pct".to_string()), detail);
    }
    if !signal.event_quality.valid {
        let detail = if signal.event_quality.false_event_flags.is_empty() {
            format!(
                "quality score {:.2} <= publish threshold",
                signal.event_quality.quality_score
            )
        } else {
            format!(
                "quality rejected: {}",
                signal.event_quality.false_event_flags.join(", ")
            )
        };
        return (false, Some("bad_quality".to_string()), Some(detail));
    }
    if !meets_contract_whale_display_total_volume(&signal.symbol, display_volume_btc) {
        let unit = if signal.quantity_unit.trim().is_empty() {
            signal.symbol.as_str()
        } else {
            signal.quantity_unit.as_str()
        };
        let detail = contract_whale_min_display_total_volume_btc(&signal.symbol).map(|threshold| {
            format!(
                "total volume {:.2} {unit} < display threshold {:.2} {unit}",
                display_volume_btc, threshold
            )
        });
        return (
            false,
            Some("below_display_volume_threshold".to_string()),
            detail,
        );
    }
    (true, None, None)
}

fn visibility_debug_counts(candidates: &[ContractEventCandidate]) -> VisibilityDebugCounts {
    let mut counts = VisibilityDebugCounts::default();
    for candidate in candidates {
        if candidate.is_visible {
            counts.visible_count += 1;
            continue;
        }
        counts.hidden_count += 1;
        match candidate.hidden_reason.as_deref() {
            Some("price_deviation_gt_5pct") => counts.hidden_reasons.price_deviation_gt_5pct += 1,
            Some("missing_price") => counts.hidden_reasons.missing_price += 1,
            Some("bad_quality") => counts.hidden_reasons.bad_quality += 1,
            Some("disabled_monitor") => counts.hidden_reasons.disabled_monitor += 1,
            _ => counts.hidden_reasons.unknown += 1,
        }
    }
    counts
}

fn latest_vs_history(
    latest_items: &[ContractWhaleSignal],
    projected: &[ContractEventCandidate],
    from_ts: Option<i64>,
    to_ts: Option<i64>,
) -> Vec<LatestVsHistoryEntry> {
    latest_items
        .iter()
        .map(|item| {
            let latest_event_id = if item.event_lifecycle.event_id.is_empty() {
                item.id.clone()
            } else {
                item.event_lifecycle.event_id.clone()
            };
            let matching_candidate = projected.iter().find(|candidate| {
                candidate.event.event_id == latest_event_id
                    || candidate.event.source_signal.id == item.id
                    || candidate
                        .event
                        .source_signal_ids
                        .iter()
                        .any(|id| id == &item.id)
            });
            if let Some(candidate) = matching_candidate {
                return LatestVsHistoryEntry {
                    latest_event_id,
                    symbol: item.symbol.clone(),
                    ts: item.ts,
                    exists_in_history: candidate.is_visible,
                    history_event_id: Some(candidate.event.event_id.clone()),
                    not_in_history_reason: if candidate.is_visible {
                        None
                    } else {
                        candidate.hidden_reason.clone()
                    },
                };
            }

            let not_in_history_reason = if from_ts.is_some_and(|from_ts| item.ts < from_ts) {
                Some("outside_requested_range".to_string())
            } else if to_ts.is_some_and(|to_ts| item.ts > to_ts) {
                Some("outside_requested_range".to_string())
            } else {
                Some("latest_snapshot_not_persisted_yet".to_string())
            };
            LatestVsHistoryEntry {
                latest_event_id,
                symbol: item.symbol.clone(),
                ts: item.ts,
                exists_in_history: false,
                history_event_id: None,
                not_in_history_reason,
            }
        })
        .collect()
}

fn db_debug_counts(
    store: &SqliteStore,
    symbol: &str,
    from_ts: Option<i64>,
    to_ts: Option<i64>,
) -> anyhow::Result<DebugDbCounts> {
    let start_ts = from_ts.unwrap_or_else(|| now_ms() - 24 * 60 * 60 * 1000);
    let end_ts = to_ts.unwrap_or_else(now_ms);
    store.with_connection(|conn| {
        let total_24h = conn.query_row(
            "SELECT COUNT(*) FROM contract_whale_signals WHERE market_type = 'perp' AND ts >= ?1 AND ts <= ?2",
            [start_ts, end_ts],
            |row| row.get(0),
        )?;
        let symbol_24h = conn.query_row(
            "SELECT COUNT(*) FROM contract_whale_signals WHERE market_type = 'perp' AND symbol = ?1 AND ts >= ?2 AND ts <= ?3",
            rusqlite::params![symbol, start_ts, end_ts],
            |row| row.get(0),
        )?;
        let oldest_ts = conn.query_row(
            "SELECT MIN(ts) FROM contract_whale_signals WHERE market_type = 'perp' AND symbol = ?1 AND ts >= ?2 AND ts <= ?3",
            rusqlite::params![symbol, start_ts, end_ts],
            |row| row.get(0),
        )?;
        let newest_ts = conn.query_row(
            "SELECT MAX(ts) FROM contract_whale_signals WHERE market_type = 'perp' AND symbol = ?1 AND ts >= ?2 AND ts <= ?3",
            rusqlite::params![symbol, start_ts, end_ts],
            |row| row.get(0),
        )?;
        Ok(DebugDbCounts {
            contract_whale_signals_total_24h: total_24h,
            contract_whale_signals_btc_24h: symbol_24h,
            oldest_ts,
            newest_ts,
        })
    })
}

fn contract_event_from_candidate(candidate: ContractEventCandidate) -> ContractEventItem {
    let event = candidate.event;
    let source_signal = &event.source_signal;
    let exchange_spot_count = source_signal.active_sources.spot.len();
    let exchange_contract_count = source_signal.active_sources.contract.len();
    let severity_key = severity_key(source_signal.severity);
    let direction_key = direction_key(source_signal.direction);
    let impact_level = source_signal
        .impact_level
        .as_deref()
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(|value| value.to_ascii_uppercase());
    let impact_permanent = matches!(impact_level.as_deref(), Some("A") | Some("S"));
    let is_retention_protected = severity_key == "s"
        || impact_permanent
        || source_signal.net_volume_btc.abs()
            >= CONTRACT_WHALE_PERMANENT_NET_DIRECTION_THRESHOLD_BTC;
    let retention_reason = if severity_key == "s" {
        Some("severity_s".to_string())
    } else if impact_level.as_deref() == Some("S") {
        Some("impact_s".to_string())
    } else if impact_level.as_deref() == Some("A") {
        Some("impact_a".to_string())
    } else if source_signal.net_volume_btc.abs()
        >= CONTRACT_WHALE_PERMANENT_NET_DIRECTION_THRESHOLD_BTC
    {
        Some("net_volume_ge_500_btc".to_string())
    } else {
        None
    };
    ContractEventItem {
        event_id: event.event_id.clone(),
        source_signal_id: Some(source_signal.id.clone()),
        symbol: event.symbol.clone(),
        price: event.price,
        ts: event.end_time,
        status: event.status.clone(),
        signal_type: event.event_type.clone(),
        severity: severity_key.to_string(),
        window_sec: event.window_sec,
        volume_btc: event.volume,
        notional_usd: event.notional,
        net_volume_btc: event.net_volume,
        direction: direction_key.to_string(),
        net_direction: event.direction_bias.clone(),
        main_force_score: source_signal.main_force_score,
        exchange_spot_count,
        exchange_contract_count,
        source: "contract_whale_signals".to_string(),
        is_retention_protected,
        retention_reason,
        is_visible: candidate.is_visible,
        hidden_reason: candidate.hidden_reason,
        hidden_detail: candidate.hidden_detail,
        final_event: event,
    }
}

fn severity_key(severity: ContractWhaleSeverity) -> &'static str {
    match severity {
        ContractWhaleSeverity::Calm => "calm",
        ContractWhaleSeverity::Medium => "medium",
        ContractWhaleSeverity::High => "high",
        ContractWhaleSeverity::Critical => "critical",
        ContractWhaleSeverity::S => "s",
    }
}

fn direction_key(direction: ContractWhaleDirection) -> &'static str {
    match direction {
        ContractWhaleDirection::Buy => "buy",
        ContractWhaleDirection::Sell => "sell",
        ContractWhaleDirection::Absorption => "absorption",
        ContractWhaleDirection::Suppression => "suppression",
    }
}

fn parse_requested_limit(
    value: Option<&str>,
    default: usize,
    max: usize,
) -> Result<usize, (StatusCode, Json<serde_json::Value>)> {
    let Some(value) = value.map(str::trim).filter(|value| !value.is_empty()) else {
        return Ok(default);
    };
    let parsed = value
        .parse::<usize>()
        .map_err(|_| bad_request("limit_invalid"))?;
    Ok(parsed.clamp(1, max))
}

fn parse_include_hidden(
    value: Option<&str>,
) -> Result<bool, (StatusCode, Json<serde_json::Value>)> {
    match value.map(str::trim).filter(|value| !value.is_empty()) {
        None => Ok(false),
        Some("true") | Some("1") => Ok(true),
        Some("false") | Some("0") => Ok(false),
        Some(_) => Err(bad_request("include_hidden_invalid")),
    }
}

fn parse_include_source_signal(
    value: Option<&str>,
) -> Result<bool, (StatusCode, Json<serde_json::Value>)> {
    match value.map(str::trim).filter(|value| !value.is_empty()) {
        None => Ok(false),
        Some("true") | Some("1") => Ok(true),
        Some("false") | Some("0") => Ok(false),
        Some(_) => Err(bad_request("include_source_signal_invalid")),
    }
}

fn normalize_status_filter(value: Option<&str>) -> Option<String> {
    value
        .map(str::trim)
        .filter(|value| !value.is_empty() && !value.eq_ignore_ascii_case("all"))
        .map(|value| value.to_ascii_lowercase())
}

fn status_matches(filter: Option<&str>, actual: &str) -> bool {
    match filter {
        Some("active") => actual.eq_ignore_ascii_case("active"),
        Some("closed") => actual.eq_ignore_ascii_case("closed"),
        _ => true,
    }
}

fn retention_tables(
    store: SqliteStore,
    flow_days: i64,
    signal_days: i64,
) -> anyhow::Result<ContractRetentionTables> {
    let now_ms = now_ms();
    let flow_cutoff = now_ms.saturating_sub(flow_days.max(1) * 24 * 60 * 60 * 1000);
    let signal_cutoff = now_ms.saturating_sub(signal_days.max(1) * 24 * 60 * 60 * 1000);
    store.with_connection(|conn| {
        let contract_flow_1s = RetentionTableStats {
            oldest_ts: query_min_ts(conn, "contract_flow_1s", "ts_bucket")?,
            newest_ts: query_max_ts(conn, "contract_flow_1s", "ts_bucket")?,
            row_count: Some(query_count(conn, "contract_flow_1s")?),
            rows_older_than_retention: Some(query_older_than(
                conn,
                "contract_flow_1s",
                "ts_bucket",
                flow_cutoff,
            )?),
            protected_s_count: None,
            protected_net_volume_count: None,
            has_retention_cleanup: None,
            reason: None,
        };
        let contract_whale_signals = RetentionTableStats {
            oldest_ts: query_min_ts(conn, "contract_whale_signals", "ts")?,
            newest_ts: query_max_ts(conn, "contract_whale_signals", "ts")?,
            row_count: Some(query_count(conn, "contract_whale_signals")?),
            rows_older_than_retention: Some(query_older_than(
                conn,
                "contract_whale_signals",
                "ts",
                signal_cutoff,
            )?),
            protected_s_count: Some(conn.query_row(
                "SELECT COUNT(*) FROM contract_whale_signals WHERE severity = 's'",
                [],
                |row| row.get(0),
            )?),
            protected_net_volume_count: Some(conn.query_row(
                "SELECT COUNT(*) FROM contract_whale_signals WHERE ABS(COALESCE(net_volume_btc, 0.0)) >= ?1",
                [CONTRACT_WHALE_PERMANENT_NET_DIRECTION_THRESHOLD_BTC],
                |row| row.get(0),
            )?),
            has_retention_cleanup: None,
            reason: None,
        };
        let main_force_events = RetentionTableStats {
            oldest_ts: query_min_ts(conn, "main_force_events", "started_at")?,
            newest_ts: query_max_ts(conn, "main_force_events", "started_at")?,
            row_count: Some(query_count(conn, "main_force_events")?),
            rows_older_than_retention: None,
            protected_s_count: None,
            protected_net_volume_count: None,
            has_retention_cleanup: Some(false),
            reason: None,
        };
        Ok(ContractRetentionTables {
            contract_flow_1s,
            contract_whale_signals,
            main_force_events,
        })
    })
}

fn unavailable_retention_tables(reason: &str) -> ContractRetentionTables {
    let empty = RetentionTableStats {
        oldest_ts: None,
        newest_ts: None,
        row_count: None,
        rows_older_than_retention: None,
        protected_s_count: None,
        protected_net_volume_count: None,
        has_retention_cleanup: None,
        reason: Some(reason.to_string()),
    };
    ContractRetentionTables {
        contract_flow_1s: empty.clone(),
        contract_whale_signals: empty.clone(),
        main_force_events: empty,
    }
}

fn query_count(conn: &rusqlite::Connection, table: &str) -> rusqlite::Result<i64> {
    conn.query_row(&format!("SELECT COUNT(*) FROM {table}"), [], |row| {
        row.get(0)
    })
}

fn query_min_ts(
    conn: &rusqlite::Connection,
    table: &str,
    column: &str,
) -> rusqlite::Result<Option<i64>> {
    conn.query_row(&format!("SELECT MIN({column}) FROM {table}"), [], |row| {
        row.get(0)
    })
}

fn query_max_ts(
    conn: &rusqlite::Connection,
    table: &str,
    column: &str,
) -> rusqlite::Result<Option<i64>> {
    conn.query_row(&format!("SELECT MAX({column}) FROM {table}"), [], |row| {
        row.get(0)
    })
}

fn query_older_than(
    conn: &rusqlite::Connection,
    table: &str,
    column: &str,
    cutoff: i64,
) -> rusqlite::Result<i64> {
    conn.query_row(
        &format!("SELECT COUNT(*) FROM {table} WHERE {column} < ?1"),
        [cutoff],
        |row| row.get(0),
    )
}

pub(crate) fn internal_error(error: anyhow::Error) -> (StatusCode, Json<serde_json::Value>) {
    tracing::warn!(error = %error, "contract_event_route_failed");
    (
        StatusCode::INTERNAL_SERVER_ERROR,
        Json(serde_json::json!({
            "error": "internal_error",
            "reason": "contract_event_route_failed",
            "dataState": "degraded",
            "degraded": true,
            "errorCode": "contract_history_query_failed",
            "errorMessage": "历史事件查询暂时不可用",
            "servedAt": crate::normalizers::trade::now_ms(),
            "lastKnownDataAvailable": true,
            "readOnly": true,
            "executionEnabled": false,
        })),
    )
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn lifecycle_flow_ranges_keep_separated_events_disjoint() {
        let ranges = BTreeMap::from([(
            "BTC".to_string(),
            vec![(1_000, 5_000), (4_000, 7_000), (60_000, 65_000)],
        )]);

        let merged = coalesce_lifecycle_flow_ranges(ranges);

        assert_eq!(
            merged.get("BTC"),
            Some(&vec![(1_000, 7_000), (60_000, 65_000)])
        );
    }
}
