use axum::{
    extract::{Query, State},
    http::StatusCode,
    Json,
};
use serde::Serialize;

use crate::{
    api::contract_whale_routes::{
        build_contract_whale_history_response, encode_contract_history_cursor, parse_history_query,
        ContractWhaleQuery,
    },
    app::AppState,
    contract_whale_monitor::{
        config::contract_whale_runtime_config,
        types::{ContractWhaleDirection, ContractWhaleSeverity},
    },
    core_event::final_store::final_event_store::{
        build_final_events_from_contract_whale_signals, FinalEvent, VolumeDisplayContext,
    },
    storage::{
        contract_whale_repo::{
            ContractWhaleRepo, CONTRACT_WHALE_PERMANENT_NET_DIRECTION_THRESHOLD_BTC,
        },
        SqliteStore,
    },
};

type ApiJsonResult<T = serde_json::Value> = Result<Json<T>, (StatusCode, Json<serde_json::Value>)>;

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

pub async fn contract_events_route(
    State(state): State<AppState>,
    Query(query): Query<ContractWhaleQuery>,
) -> ApiJsonResult<ContractEventPage> {
    let page = contract_event_page_for_query(state, query)?;
    Ok(Json(page))
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

fn contract_event_page_for_query(
    state: AppState,
    mut query: ContractWhaleQuery,
) -> Result<ContractEventPage, (StatusCode, Json<serde_json::Value>)> {
    let requested_limit = parse_requested_limit(query.limit.as_deref(), 100, 500)?;
    let range = query.range.clone().unwrap_or_else(|| "24h".to_string());
    query.limit = Some((requested_limit + 1).to_string());
    let history_query = parse_history_query(&query)?;
    let symbol = history_query.symbol.as_deref().unwrap_or("BTC").to_string();
    let config = state.config().contract_whale_monitor;
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
    let response = build_contract_whale_history_response(
        sliced_items,
        &symbol,
        requested_limit,
        None,
        config.enabled,
        config.dry_run,
        None,
    );

    let requested_status = normalize_status_filter(query.status.as_deref());
    let items = response
        .items
        .iter()
        .map(FinalEvent::from_contract_signal)
        .filter(|event| status_matches(requested_status.as_deref(), &event.status))
        .map(contract_event_from_final_event)
        .collect::<Vec<_>>();

    Ok(ContractEventPage {
        items,
        next_cursor,
        has_more,
        limit: requested_limit,
        range,
        server_time: crate::normalizers::trade::now_ms(),
        last_event_ts,
    })
}

fn final_events_v2_for_query(
    state: AppState,
    mut query: ContractWhaleQuery,
) -> Result<FinalEventsV2Response, (StatusCode, Json<serde_json::Value>)> {
    let requested_limit = parse_requested_limit(query.limit.as_deref(), 100, 500)?;
    let range = query.range.clone().unwrap_or_else(|| "24h".to_string());
    query.limit = Some((requested_limit + 1).to_string());
    let history_query = parse_history_query(&query)?;
    let symbol = history_query.symbol.as_deref().unwrap_or("BTC").to_string();
    let config = state.config().contract_whale_monitor;
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
    let response = build_contract_whale_history_response(
        sliced_items,
        &symbol,
        requested_limit,
        None,
        config.enabled,
        config.dry_run,
        None,
    );

    let requested_status = normalize_status_filter(query.status.as_deref());
    let mut active = Vec::new();
    let mut closed = Vec::new();
    for event in build_final_events_from_contract_whale_signals(
        &response.items,
        VolumeDisplayContext::FinalLifecycleEvent,
    ) {
        if !status_matches(requested_status.as_deref(), &event.status) {
            continue;
        }
        if event.status.eq_ignore_ascii_case("closed") {
            closed.push(event);
        } else {
            active.push(event);
        }
    }

    Ok(FinalEventsV2Response {
        active,
        closed,
        next_cursor,
        has_more,
        limit: requested_limit,
        range,
        server_time: crate::normalizers::trade::now_ms(),
        last_event_ts,
    })
}

fn contract_event_from_final_event(event: FinalEvent) -> ContractEventItem {
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
    let now_ms = crate::normalizers::trade::now_ms();
    let flow_cutoff = now_ms.saturating_sub(flow_days.max(1) * 24 * 60 * 60 * 1000);
    let signal_cutoff = now_ms.saturating_sub(signal_days.max(1) * 24 * 60 * 60 * 1000);
    store.with_connection(|conn| {
        let contract_flow_1s = RetentionTableStats {
            oldest_ts: query_min_ts(conn, "contract_flow_1s", "ts_bucket")?,
            newest_ts: query_max_ts(conn, "contract_flow_1s", "ts_bucket")?,
            row_count: Some(query_count(conn, "contract_flow_1s")?),
            rows_older_than_retention: Some(query_older_than(conn, "contract_flow_1s", "ts_bucket", flow_cutoff)?),
            protected_s_count: None,
            protected_net_volume_count: None,
            has_retention_cleanup: None,
            reason: None,
        };
        let contract_whale_signals = RetentionTableStats {
            oldest_ts: query_min_ts(conn, "contract_whale_signals", "ts")?,
            newest_ts: query_max_ts(conn, "contract_whale_signals", "ts")?,
            row_count: Some(query_count(conn, "contract_whale_signals")?),
            rows_older_than_retention: Some(query_older_than(conn, "contract_whale_signals", "ts", signal_cutoff)?),
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
