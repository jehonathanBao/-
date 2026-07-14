use axum::{
    extract::{Query, State},
    http::StatusCode,
    Json,
};
use serde::Serialize;

use crate::{
    api::{
        contract_event_routes::{
            contract_event_page_for_query_nonblocking, final_events_v2_for_query_nonblocking,
        },
        contract_whale_routes::{parse_history_query, parse_symbol_for_latest, ContractWhaleQuery},
    },
    app::AppState,
    normalizers::trade::now_ms,
    storage::contract_whale_repo::ContractWhaleRepo,
};

type ApiJsonResult<T = serde_json::Value> = Result<Json<T>, (StatusCode, Json<serde_json::Value>)>;

#[derive(Debug, Clone, Serialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct CanonicalTimelineMeta {
    pub source: String,
    pub event_ts: Option<i64>,
    pub processed_ts: Option<i64>,
    pub persisted_ts: Option<i64>,
    pub served_ts: i64,
    pub timeline_lag_sec: i64,
}

#[derive(Debug, Clone, Serialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct ContractWhaleTimelineViewMeta {
    pub count: usize,
    pub max_event_ts: Option<i64>,
    pub drift_vs_canonical_sec: i64,
    pub cache_age_sec: Option<i64>,
    pub cache_ttl_sec: Option<i64>,
    pub generated_at: Option<i64>,
}

#[derive(Debug, Clone, Serialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct ContractWhaleTimelineFlowViewMeta {
    pub updated_at: Option<i64>,
    pub drift_vs_canonical_sec: i64,
}

#[derive(Debug, Clone, Serialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct ContractWhaleTimelineViews {
    pub latest: ContractWhaleTimelineViewMeta,
    pub history: ContractWhaleTimelineViewMeta,
    pub final_events_v2: ContractWhaleTimelineViewMeta,
    pub flow: ContractWhaleTimelineFlowViewMeta,
}

#[derive(Debug, Clone, Serialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct ContractWhaleTimelineResponse {
    pub symbol: String,
    pub range: String,
    #[serde(flatten)]
    pub timeline: CanonicalTimelineMeta,
    pub views: ContractWhaleTimelineViews,
}

pub async fn contract_whale_timeline_route(
    State(state): State<AppState>,
    Query(query): Query<ContractWhaleQuery>,
) -> ApiJsonResult<ContractWhaleTimelineResponse> {
    let symbol = parse_symbol_for_latest(query.symbol.as_deref())?;
    let range = query.range.clone().unwrap_or_else(|| "24h".to_string());
    let timeline = build_contract_whale_timeline_response(
        &state,
        &symbol,
        &range,
        query
            .limit
            .as_deref()
            .and_then(|value| value.parse::<usize>().ok())
            .unwrap_or(100),
    )
    .await?;
    Ok(Json(timeline))
}

pub fn build_canonical_timeline_meta(
    source: impl Into<String>,
    event_ts: Option<i64>,
    persisted_ts: Option<i64>,
    processed_ts: Option<i64>,
    served_ts: i64,
) -> CanonicalTimelineMeta {
    CanonicalTimelineMeta {
        source: source.into(),
        event_ts,
        processed_ts,
        persisted_ts,
        served_ts,
        timeline_lag_sec: event_ts
            .map(|ts| served_ts.saturating_sub(ts).max(0).saturating_div(1000))
            .unwrap_or(0),
    }
}

pub fn canonical_timeline_meta_for_signal_range(
    state: &AppState,
    symbol: &str,
    range: &str,
    served_ts: i64,
    processed_ts: Option<i64>,
) -> CanonicalTimelineMeta {
    let history_query = parse_history_query(&ContractWhaleQuery {
        symbol: Some(symbol.to_string()),
        range: Some(range.to_string()),
        limit: Some("1".to_string()),
        ..ContractWhaleQuery::default()
    });

    let Ok(history_query) = history_query else {
        return build_canonical_timeline_meta("none", None, None, processed_ts, served_ts);
    };

    let Some(store) = state.contract_whale_store() else {
        return build_canonical_timeline_meta("none", None, None, processed_ts, served_ts);
    };

    let max_event_ts = store
        .query_contract_whale_signals(&history_query)
        .ok()
        .and_then(|rows| rows.first().map(|signal| signal.ts));

    let persisted_ts = store
        .with_connection(|conn| {
            let mut sql = String::from(
                "SELECT MAX(created_at) FROM contract_whale_signals WHERE symbol = ?1",
            );
            let mut params: Vec<Box<dyn rusqlite::ToSql>> = vec![Box::new(symbol.to_string())];
            if let Some(from_ts) = history_query.from_ts {
                sql.push_str(" AND ts >= ?2");
                params.push(Box::new(from_ts));
            }
            if let Some(to_ts) = history_query.to_ts {
                let placeholder = format!(" ?{}", params.len() + 1);
                sql.push_str(" AND ts <= ");
                sql.push_str(&placeholder);
                params.push(Box::new(to_ts));
            }
            let mut stmt = conn.prepare(&sql)?;
            let param_refs = params
                .iter()
                .map(|value| value.as_ref() as &dyn rusqlite::ToSql)
                .collect::<Vec<_>>();
            let value = stmt.query_row(rusqlite::params_from_iter(param_refs), |row| {
                row.get::<_, Option<i64>>(0)
            })?;
            Ok(value)
        })
        .ok()
        .flatten();

    let source = if max_event_ts.is_some() {
        "contract_whale_signals"
    } else {
        "none"
    };

    build_canonical_timeline_meta(
        source,
        max_event_ts,
        persisted_ts,
        processed_ts.or(persisted_ts).or(max_event_ts),
        served_ts,
    )
}

pub async fn build_contract_whale_timeline_response(
    state: &AppState,
    symbol: &str,
    range: &str,
    limit: usize,
) -> Result<ContractWhaleTimelineResponse, (StatusCode, Json<serde_json::Value>)> {
    let now = now_ms();
    let store = state.contract_whale_store().ok_or_else(|| {
        crate::api::contract_event_routes::internal_error(anyhow::anyhow!(
            "contract whale store unavailable"
        ))
    })?;
    let latest_symbol = symbol.to_string();
    let latest_rows = tokio::task::spawn_blocking(move || {
        store.query_contract_whale_signals(
            &crate::storage::contract_whale_repo::ContractWhaleSignalQuery {
                symbol: Some(latest_symbol),
                limit: limit.max(1),
                ..crate::storage::contract_whale_repo::ContractWhaleSignalQuery::default()
            },
        )
    })
    .await
    .map_err(|error| {
        crate::api::contract_event_routes::internal_error(anyhow::anyhow!(
            "timeline latest join failed: {error}"
        ))
    })?
    .map_err(crate::api::contract_event_routes::internal_error)?;
    let latest_max_ts = latest_rows.iter().map(|signal| signal.ts).max();

    let history_future = contract_event_page_for_query_nonblocking(
        state.clone(),
        ContractWhaleQuery {
            symbol: Some(symbol.to_string()),
            range: Some(range.to_string()),
            limit: Some(limit.max(1).to_string()),
            include_hidden: Some("true".to_string()),
            ..ContractWhaleQuery::default()
        },
    );
    let final_events_future = final_events_v2_for_query_nonblocking(
        state.clone(),
        ContractWhaleQuery {
            symbol: Some(symbol.to_string()),
            range: Some(range.to_string()),
            limit: Some(limit.max(1).to_string()),
            ..ContractWhaleQuery::default()
        },
    );
    let (history, final_events) = tokio::join!(history_future, final_events_future);
    let history = history?;
    let final_events = final_events?;
    let flow_state = state.flow_state_for_symbol(symbol);
    let flow_updated_at = (flow_state.updated_at > 0).then_some(flow_state.updated_at);

    let source = if history.max_event_ts.is_some() {
        "contract_whale_signals"
    } else if final_events.max_event_ts.is_some() {
        "final_events_v2"
    } else if latest_max_ts.is_some() {
        "latest_snapshot"
    } else if flow_updated_at.is_some() {
        "flow_state"
    } else {
        "none"
    };
    let event_ts = history
        .max_event_ts
        .or(final_events.max_event_ts)
        .or(latest_max_ts)
        .or(flow_updated_at);
    let persisted_ts = history.max_persisted_at.or(event_ts);
    let processed_ts = Some(final_events.generated_at)
        .or(persisted_ts)
        .or(event_ts);
    let timeline = build_canonical_timeline_meta(source, event_ts, persisted_ts, processed_ts, now);

    Ok(ContractWhaleTimelineResponse {
        symbol: symbol.to_string(),
        range: range.to_string(),
        timeline: timeline.clone(),
        views: ContractWhaleTimelineViews {
            latest: ContractWhaleTimelineViewMeta {
                count: latest_rows.len(),
                max_event_ts: latest_max_ts,
                drift_vs_canonical_sec: drift_vs_canonical_sec(timeline.event_ts, latest_max_ts),
                ..ContractWhaleTimelineViewMeta::default()
            },
            history: ContractWhaleTimelineViewMeta {
                count: history.items.len(),
                max_event_ts: history.max_event_ts,
                drift_vs_canonical_sec: drift_vs_canonical_sec(
                    timeline.event_ts,
                    history.max_event_ts,
                ),
                cache_age_sec: Some(history.cache_age_sec),
                cache_ttl_sec: Some(history.cache_ttl_sec),
                generated_at: Some(history.server_time),
            },
            final_events_v2: ContractWhaleTimelineViewMeta {
                count: final_events.active.len() + final_events.closed.len(),
                max_event_ts: final_events.max_event_ts,
                drift_vs_canonical_sec: drift_vs_canonical_sec(
                    timeline.event_ts,
                    final_events.max_event_ts,
                ),
                cache_age_sec: Some(final_events.cache_age_sec),
                cache_ttl_sec: Some(final_events.cache_ttl_sec),
                generated_at: Some(final_events.generated_at),
            },
            flow: ContractWhaleTimelineFlowViewMeta {
                updated_at: flow_updated_at,
                drift_vs_canonical_sec: drift_vs_canonical_sec(timeline.event_ts, flow_updated_at),
            },
        },
    })
}

fn drift_vs_canonical_sec(canonical_ts: Option<i64>, view_ts: Option<i64>) -> i64 {
    canonical_ts
        .zip(view_ts)
        .map(|(canonical, view)| canonical.saturating_sub(view).abs().saturating_div(1000))
        .unwrap_or(0)
}
