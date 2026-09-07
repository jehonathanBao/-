use axum::{
    extract::{Path, Query, State},
    http::StatusCode,
    Json,
};

use crate::{
    api::contract_event_routes::decorate_v3_final_events,
    api::contract_whale_routes::{
        build_contract_whale_history_response_with_clock, decorate_contract_whale_oi_contexts,
        parse_history_query, ContractWhaleQuery,
    },
    app::AppState,
    contract_whale_monitor::config::contract_whale_runtime_config,
    core_event::final_store::final_event_store::build_final_event_store_response_from_contract_whale_response,
    storage::contract_whale_repo::ContractWhaleRepo,
};

type ApiJsonResult<T = serde_json::Value> = Result<Json<T>, (StatusCode, Json<serde_json::Value>)>;

pub async fn final_events_route(
    State(state): State<AppState>,
    Query(query): Query<ContractWhaleQuery>,
) -> ApiJsonResult<crate::core_event::final_store::final_event_store::FinalEventStoreResponse> {
    let response = final_event_response_for_query(state, query)?;
    Ok(Json(response))
}

pub async fn final_event_by_id_route(
    State(state): State<AppState>,
    Path(event_id): Path<String>,
    Query(query): Query<ContractWhaleQuery>,
) -> ApiJsonResult<crate::core_event::final_store::final_event_store::FinalEventStoreResponse> {
    let mut response = final_event_response_for_query(state, query)?;
    response.items.retain(|event| event.event_id == event_id);
    response.count = response.items.len();
    Ok(Json(response))
}

fn final_event_response_for_query(
    state: AppState,
    query: ContractWhaleQuery,
) -> Result<
    crate::core_event::final_store::final_event_store::FinalEventStoreResponse,
    (StatusCode, Json<serde_json::Value>),
> {
    let mut history_query = parse_history_query(&query)?;
    let runtime_config = contract_whale_runtime_config();
    if runtime_config.impact_grade_v3.enabled && history_query.impact_level.is_some() {
        history_query.impact_grade_version =
            Some(runtime_config.impact_grade_v3.grade_version.clone());
    }
    let symbol_for_filter = history_query.symbol.as_deref().unwrap_or("all").to_string();
    let config = state.config().contract_whale_monitor;
    let store = state.contract_whale_store().ok_or_else(|| {
        crate::api::contract_event_routes::internal_error(anyhow::anyhow!(
            "contract whale store unavailable"
        ))
    })?;
    let mut items = store
        .query_contract_whale_signals(&history_query)
        .map_err(crate::api::contract_event_routes::internal_error)?;
    crate::api::contract_event_routes::decorate_v3_signal_grades(Some(&store), &mut items);
    let mut contract_response = build_contract_whale_history_response_with_clock(
        items,
        &symbol_for_filter,
        history_query.limit,
        None,
        config.enabled,
        config.dry_run,
        None,
        crate::contract_whale_monitor::event_lifecycle::ContractWhaleLifecycleClock::Live {
            now_ms: crate::normalizers::trade::now_ms(),
        },
    );
    let oi_diagnostics = decorate_contract_whale_oi_contexts(&store, &mut contract_response.items);
    state.record_contract_whale_oi_resolver_diagnostics(oi_diagnostics);
    let mut response =
        build_final_event_store_response_from_contract_whale_response(&contract_response);
    decorate_v3_final_events(Some(&store), &mut response.items);
    Ok(response)
}
