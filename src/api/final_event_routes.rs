use axum::{
    extract::{Path, Query, State},
    http::StatusCode,
    Json,
};

use crate::{
    api::contract_whale_routes::{
        build_contract_whale_history_response, decorate_contract_whale_oi_contexts,
        parse_history_query, ContractWhaleQuery,
    },
    app::AppState,
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
    let history_query = parse_history_query(&query)?;
    let symbol_for_filter = history_query.symbol.as_deref().unwrap_or("all").to_string();
    let config = state.config().contract_whale_monitor;
    let items = state
        .contract_whale_store()
        .and_then(|store| store.query_contract_whale_signals(&history_query).ok())
        .unwrap_or_default();
    let mut contract_response = build_contract_whale_history_response(
        items,
        &symbol_for_filter,
        history_query.limit,
        None,
        config.enabled,
        config.dry_run,
        None,
    );
    if let Some(store) = state.contract_whale_store() {
        decorate_contract_whale_oi_contexts(&store, &mut contract_response.items);
    }
    Ok(build_final_event_store_response_from_contract_whale_response(&contract_response))
}
