use std::collections::BTreeMap;

use axum::{
    extract::{Query, State},
    http::StatusCode,
    Json,
};
use serde::Serialize;

use crate::{
    api::contract_timeline_routes::{build_canonical_timeline_meta, CanonicalTimelineMeta},
    api::contract_whale_routes::{
        build_contract_whale_items_response, decorate_price_deviation_signals,
        encode_contract_history_cursor, parse_history_query, ContractWhaleQuery,
    },
    app::AppState,
    contract_whale_monitor::{
        cluster::apply_contract_whale_signal_clusters,
        config::contract_whale_runtime_config,
        discord::{
            contract_whale_min_display_total_volume_btc, meets_contract_whale_display_total_volume,
        },
        event_lifecycle::apply_contract_whale_event_lifecycle,
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
const FINAL_EVENTS_V2_CACHE_TTL_SEC: i64 = 10;

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
    pub signal_protect_severity_s: bool,
    pub signal_protect_net_volume_btc: f64,
    pub cleanup_interval_hours: i64,
    pub tables: ContractRetentionTables,
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
) -> ApiJsonResult<ContractEventPage> {
    let page = contract_event_page_for_query(state, query)?;
    Ok(Json(page))
}

pub async fn contract_events_debug_counts_route(
    State(state): State<AppState>,
    Query(query): Query<ContractWhaleQuery>,
) -> ApiJsonResult<ContractEventDebugCountsResponse> {
    Ok(Json(contract_event_debug_counts_for_query(state, query)))
}

pub async fn final_events_v2_route(
    State(state): State<AppState>,
    Query(query): Query<ContractWhaleQuery>,
) -> ApiJsonResult<FinalEventsV2Response> {
    let page = final_events_v2_for_query(state, query)?;
    Ok(Json(page))
}

pub async fn contract_retention_status_route(
    State(state): State<AppState>,
) -> ApiJsonResult<ContractRetentionStatusResponse> {
    let retention = contract_whale_runtime_config().retention;
    let tables = match state.contract_whale_store() {
        Some(store) => retention_tables(store, retention.flow_1s_days, retention.signals_days)
            .map_err(internal_error)?,
        None => unavailable_retention_tables("query_failed"),
    };

    Ok(Json(ContractRetentionStatusResponse {
        flow_retention_days: retention.flow_1s_days,
        signal_retention_days: retention.signals_days,
        signal_protect_severity_s: true,
        signal_protect_net_volume_btc: CONTRACT_WHALE_PERMANENT_NET_DIRECTION_THRESHOLD_BTC,
        cleanup_interval_hours: 1,
        tables,
    }))
}

pub(crate) fn contract_event_page_for_query(
    state: AppState,
    mut query: ContractWhaleQuery,
) -> Result<ContractEventPage, (StatusCode, Json<serde_json::Value>)> {
    let requested_limit = parse_requested_limit(query.limit.as_deref(), 100, 500)?;
    let include_hidden = parse_include_hidden(query.include_hidden.as_deref())?;
    let range = query.range.clone().unwrap_or_else(|| "24h".to_string());
    query.limit = Some((requested_limit + 1).to_string());
    let history_query = parse_history_query(&query)?;
    let raw_items = state
        .contract_whale_store()
        .and_then(|store| store.query_contract_whale_signals(&history_query).ok())
        .unwrap_or_default();
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
    let items =
        project_contract_event_candidates(sliced_items, VolumeDisplayContext::ContractEventStream)
            .into_iter()
            .filter(|candidate| {
                status_matches(requested_status.as_deref(), &candidate.event.status)
            })
            .filter(|candidate| include_hidden || candidate.is_visible)
            .map(contract_event_from_candidate)
            .collect::<Vec<_>>();

    Ok(ContractEventPage {
        items,
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
    let cache_key =
        final_events_v2_cache_key(&query, requested_status.as_deref(), &range, requested_limit);
    let history_query = parse_history_query(&query)?;
    let now = now_ms();
    if let Some((cached_at_ms, mut response)) = state.cached_final_events_v2(&cache_key) {
        let cache_age_sec = now.saturating_sub(cached_at_ms).max(0).saturating_div(1000);
        if cache_age_sec <= FINAL_EVENTS_V2_CACHE_TTL_SEC {
            response.server_time = now;
            response.cache_age_sec = cache_age_sec;
            response.cache_ttl_sec = FINAL_EVENTS_V2_CACHE_TTL_SEC;
            response.projection_lag_sec = response
                .max_event_ts
                .map(|ts| now.saturating_sub(ts).max(0).saturating_div(1000))
                .unwrap_or(0);
            response.timeline = build_canonical_timeline_meta(
                "contract_whale_signals",
                response.max_event_ts,
                response.timeline.persisted_ts.or(response.max_event_ts),
                Some(response.generated_at),
                now,
            );
            return Ok(response);
        }
    }
    let raw_items = state
        .contract_whale_store()
        .and_then(|store| store.query_contract_whale_signals(&history_query).ok())
        .unwrap_or_default();
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
    for candidate in
        project_contract_event_candidates(sliced_items, VolumeDisplayContext::FinalLifecycleEvent)
    {
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

    let response = FinalEventsV2Response {
        active,
        closed,
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
    };
    state.store_final_events_v2_cache(cache_key, now, response.clone());
    Ok(response)
}

fn final_events_v2_cache_key(
    query: &ContractWhaleQuery,
    requested_status: Option<&str>,
    range: &str,
    requested_limit: usize,
) -> String {
    format!(
        "symbol={:?}|severity={:?}|signal_type={:?}|direction={:?}|discord_sent={:?}|window_sec={:?}|exchange={:?}|net_direction={:?}|min_notional_usd={:?}|cursor={:?}|from={:?}|to={:?}|offset={:?}|status={}|range={}|limit={}",
        query.symbol,
        query.severity,
        query.signal_type,
        query.direction,
        query.discord_sent,
        query.window_sec,
        query.exchange,
        query.net_direction,
        query.min_notional_usd,
        query.cursor,
        query.from,
        query.to,
        query.offset,
        requested_status.unwrap_or("all"),
        range,
        requested_limit,
    )
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
    let lifecycle_reference_now = items
        .iter()
        .map(|item| item.ts)
        .max()
        .unwrap_or_else(now_ms);
    items = apply_contract_whale_event_lifecycle(items, lifecycle_reference_now);
    items = decorate_contract_whale_event_quality(items);
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
    let is_retention_protected = severity_key == "s"
        || source_signal.net_volume_btc.abs()
            >= CONTRACT_WHALE_PERMANENT_NET_DIRECTION_THRESHOLD_BTC;
    let retention_reason = if severity_key == "s" {
        Some("severity_s".to_string())
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

fn internal_error(error: anyhow::Error) -> (StatusCode, Json<serde_json::Value>) {
    tracing::warn!(error = %error, "contract_event_route_failed");
    (
        StatusCode::INTERNAL_SERVER_ERROR,
        Json(serde_json::json!({
            "error": "internal_error",
            "reason": "contract_event_route_failed",
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
