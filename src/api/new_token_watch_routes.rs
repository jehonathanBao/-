use std::sync::OnceLock;
use std::time::Duration;

use axum::{
    extract::{
        ws::{Message, WebSocket, WebSocketUpgrade},
        Json, Query, State,
    },
    http::{HeaderMap, StatusCode},
    response::IntoResponse,
};
use futures_util::{SinkExt, StreamExt};
use serde::Deserialize;

use crate::{
    app::AppState,
    storage::new_token_l2_repo::NewTokenL2Repo,
    toxic_v3::new_token_watch::{
        fetch_market_price_snapshot, normalize_symbol,
        runtime::{validate_binance_usdm_symbol, BinanceL2Runtime},
        session::L2SessionSnapshot,
        NewTokenWatchMutationResponse, NewTokenWatchRequest, TokenWatchError, TokenWatchItem,
        TokenWatchManager, MAX_ACTIVE_TOKENS,
    },
};

static NEW_TOKEN_WATCH_MANAGER: OnceLock<TokenWatchManager> = OnceLock::new();
static NEW_TOKEN_L2_RUNTIME: OnceLock<BinanceL2Runtime> = OnceLock::new();

pub fn global_new_token_watch_manager() -> &'static TokenWatchManager {
    NEW_TOKEN_WATCH_MANAGER.get_or_init(TokenWatchManager::persistent_default)
}

pub fn global_new_token_l2_runtime() -> &'static BinanceL2Runtime {
    NEW_TOKEN_L2_RUNTIME.get_or_init(BinanceL2Runtime::from_env)
}

/// Starts one public L2 session for every persisted watchlist item. This is
/// intentionally idempotent and keeps the legacy flow-only view available
/// until the individual book reaches a contiguous ready state.
pub fn restore_new_token_l2_sessions(state: &AppState) {
    global_new_token_l2_runtime().configure_store(state.contract_whale_store());
    let symbols = global_new_token_watch_manager()
        .list_active_tokens()
        .items
        .into_iter()
        .map(|item| (item.symbol, item.added_at_ms))
        .collect::<Vec<_>>();
    global_new_token_l2_runtime().restore_symbols_at(symbols);
}

pub async fn new_token_watch_list_route(State(state): State<AppState>) -> impl IntoResponse {
    restore_new_token_l2_sessions(&state);
    Json(decorate_watchlist(
        global_new_token_watch_manager().list_active_tokens(),
    ))
}

#[derive(Debug, Deserialize)]
pub struct NewTokenReconstructionQuery {
    pub symbol: String,
    pub tf: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct NewTokenStreamQuery {
    pub symbol: String,
}

#[derive(Debug, Deserialize)]
pub struct NewTokenOutcomesQuery {
    pub symbol: String,
    pub limit: Option<usize>,
}

pub async fn new_token_watch_reconstruction_route(
    Query(query): Query<NewTokenReconstructionQuery>,
) -> impl IntoResponse {
    let timeframe = query.tf.as_deref().unwrap_or("15m");
    let market_price = fetch_market_price_snapshot(&query.symbol).await;
    match global_new_token_watch_manager().get_reconstruction_with_market(
        &query.symbol,
        timeframe,
        market_price,
    ) {
        Ok(mut response) => {
            apply_session_to_reconstruction(&query.symbol, &mut response);
            Json(response).into_response()
        }
        Err(error) => token_error_response(error),
    }
}

pub async fn new_token_watch_chart_route(
    Query(query): Query<NewTokenReconstructionQuery>,
) -> impl IntoResponse {
    let timeframe = query.tf.as_deref().unwrap_or("15m");
    let market_price = fetch_market_price_snapshot(&query.symbol).await;
    match global_new_token_watch_manager().get_chart_with_market(
        &query.symbol,
        timeframe,
        market_price,
    ) {
        Ok(response) => Json(response).into_response(),
        Err(error) => token_error_response(error),
    }
}

/// Read-only shadow calibration evidence. These records never enable trading
/// or Discord delivery and only contain compact delayed markouts.
pub async fn new_token_watch_outcomes_route(
    State(state): State<AppState>,
    Query(query): Query<NewTokenOutcomesQuery>,
) -> impl IntoResponse {
    let symbol = match normalize_symbol(&query.symbol) {
        Ok(symbol) => symbol,
        Err(error) => return token_error_response(error),
    };
    let Some(store) = state.contract_whale_store() else {
        return (
            StatusCode::SERVICE_UNAVAILABLE,
            Json(serde_json::json!({
                "ok": false,
                "symbol": symbol,
                "reason": "new_token_outcome_store_unavailable",
                "shadowOnly": true,
                "executionEnabled": false,
                "discordEligible": false
            })),
        )
            .into_response();
    };
    match store.list_new_token_l2_shadow_outcomes(&symbol, query.limit.unwrap_or(100)) {
        Ok(items) => Json(serde_json::json!({
            "ok": true,
            "symbol": symbol,
            "items": items,
            "shadowOnly": true,
            "executionEnabled": false,
            "discordEligible": false,
            "outcomeVersion": "new_token_l2_shadow_v1"
        }))
        .into_response(),
        Err(error) => {
            tracing::warn!(%error, "new-token L2 shadow outcome query failed");
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(serde_json::json!({
                    "ok": false,
                    "symbol": symbol,
                    "reason": "new_token_outcome_query_failed",
                    "shadowOnly": true,
                    "executionEnabled": false,
                    "discordEligible": false
                })),
            )
                .into_response()
        }
    }
}

pub async fn new_token_watch_add_route(
    State(state): State<AppState>,
    headers: HeaderMap,
    Json(request): Json<NewTokenWatchRequest>,
) -> impl IntoResponse {
    if let Some(response) = require_new_token_operator(&state, &headers) {
        return response;
    }
    global_new_token_l2_runtime().configure_store(state.contract_whale_store());
    let symbol = match normalize_symbol(&request.symbol) {
        Ok(symbol) => symbol,
        Err(error) => return error_response(error),
    };
    if let Err(reason) = validate_binance_usdm_symbol(&symbol).await {
        return (
            StatusCode::BAD_REQUEST,
            Json(serde_json::json!({
                "ok": false,
                "error": reason,
                "readOnly": true,
                "runtimeModified": false
            })),
        )
            .into_response();
    }
    let result =
        tokio::task::spawn_blocking(move || global_new_token_watch_manager().add_token(&symbol))
            .await
            .unwrap_or(Err(TokenWatchError::PersistenceFailed));
    match result {
        Ok(item) => {
            global_new_token_l2_runtime().start_symbol_at(&item.symbol, item.added_at_ms);
            let market_price = fetch_market_price_snapshot(&item.symbol).await;
            let mut anchored_item = global_new_token_watch_manager()
                .refresh_token_with_market(&item.symbol, market_price)
                .unwrap_or(item);
            decorate_item(&mut anchored_item);
            Json(NewTokenWatchMutationResponse {
                ok: true,
                item: Some(anchored_item),
                items: decorate_watchlist(global_new_token_watch_manager().list_active_tokens())
                    .items,
                error: None,
                max_active_tokens: MAX_ACTIVE_TOKENS,
                read_only: true,
            })
            .into_response()
        }
        Err(error) => error_response(error),
    }
}

pub async fn new_token_watch_remove_route(
    State(state): State<AppState>,
    headers: HeaderMap,
    Json(request): Json<NewTokenWatchRequest>,
) -> impl IntoResponse {
    if let Some(response) = require_new_token_operator(&state, &headers) {
        return response;
    }
    let symbol = request.symbol;
    let runtime_symbol = symbol.clone();
    let result =
        tokio::task::spawn_blocking(move || global_new_token_watch_manager().remove_token(&symbol))
            .await
            .unwrap_or(Err(TokenWatchError::PersistenceFailed));
    match result {
        Ok(item) => Json(NewTokenWatchMutationResponse {
            ok: true,
            item: Some(item),
            items: {
                global_new_token_l2_runtime().stop_symbol(&runtime_symbol);
                decorate_watchlist(global_new_token_watch_manager().list_active_tokens()).items
            },
            error: None,
            max_active_tokens: MAX_ACTIVE_TOKENS,
            read_only: true,
        })
        .into_response(),
        Err(error) => error_response(error),
    }
}

pub async fn new_token_watch_restart_route(
    State(state): State<AppState>,
    headers: HeaderMap,
    Json(request): Json<NewTokenWatchRequest>,
) -> impl IntoResponse {
    if let Some(response) = require_new_token_operator(&state, &headers) {
        return response;
    }
    let symbol = match normalize_symbol(&request.symbol) {
        Ok(symbol) => symbol,
        Err(error) => return error_response(error),
    };
    if !global_new_token_watch_manager()
        .list_active_tokens()
        .items
        .iter()
        .any(|item| item.symbol == symbol)
    {
        return error_response(TokenWatchError::TokenNotFound);
    }
    global_new_token_l2_runtime().configure_store(state.contract_whale_store());
    global_new_token_l2_runtime().stop_symbol(&symbol);
    let activated_at_ms = global_new_token_watch_manager()
        .list_active_tokens()
        .items
        .into_iter()
        .find(|item| item.symbol == symbol)
        .map(|item| item.added_at_ms)
        .unwrap_or_else(crate::normalizers::trade::now_ms);
    let session = global_new_token_l2_runtime().start_symbol_at(&symbol, activated_at_ms);
    Json(serde_json::json!({
        "ok": true,
        "symbol": symbol,
        "status": session.status,
        "contractValidated": true,
        "bookSynced": session.orderbook_evidence_available,
        "readOnly": true,
        "executionEnabled": false
    }))
    .into_response()
}

pub async fn new_token_watch_ws_route(ws: WebSocketUpgrade) -> impl IntoResponse {
    ws.on_upgrade(stream_new_token_watch)
}

pub async fn new_token_reconstruction_ws_route(
    Query(query): Query<NewTokenStreamQuery>,
    ws: WebSocketUpgrade,
) -> impl IntoResponse {
    let symbol = match normalize_symbol(&query.symbol) {
        Ok(symbol) => symbol,
        Err(error) => return token_error_response(error),
    };
    ws.on_upgrade(move |socket| stream_new_token_reconstruction(socket, Some(symbol)))
}

async fn stream_new_token_watch(socket: WebSocket) {
    let (mut sender, mut receiver) = socket.split();
    let mut interval = tokio::time::interval(Duration::from_secs(2));

    loop {
        tokio::select! {
            _ = interval.tick() => {
                let snapshot = global_new_token_watch_manager().list_active_tokens();
                let Ok(payload) = serde_json::to_string(&snapshot) else {
                    break;
                };
                if sender.send(Message::Text(payload)).await.is_err() {
                    break;
                }
            }
            message = receiver.next() => {
                if message.is_none() {
                    break;
                }
            }
        }
    }
}

async fn stream_new_token_reconstruction(socket: WebSocket, symbol: Option<String>) {
    let (mut sender, mut receiver) = socket.split();
    let mut interval = tokio::time::interval(Duration::from_secs(10));
    let mut last_snapshot = None;
    let mut sequence = 0_u64;

    loop {
        tokio::select! {
            _ = interval.tick() => {
                let snapshot = symbol.as_deref().and_then(stream_payload_for_symbol)
                    .unwrap_or_else(|| serde_json::json!({
                        "type": "new_token_reconstruction",
                        "evidenceMode": "flow_only",
                        "intentAssessmentAvailable": false,
                        "reason": "symbol_required"
                    }));
                let Ok(snapshot_key) = serde_json::to_string(&snapshot) else {
                    break;
                };
                if last_snapshot.as_deref() == Some(snapshot_key.as_str()) {
                    if sender.send(Message::Ping(Vec::new().into())).await.is_err() {
                        break;
                    }
                    continue;
                }
                last_snapshot = Some(snapshot_key);
                sequence = sequence.saturating_add(1);
                let payload = serde_json::json!({
                    "type": "new_token_reconstruction",
                    "symbol": snapshot.get("symbol").and_then(|value| value.as_str()).unwrap_or_default(),
                    "sequence": sequence,
                    "serverTs": crate::normalizers::trade::now_ms(),
                    "dataAgeMs": snapshot
                        .get("l2")
                        .and_then(|value| value.get("orderbook"))
                        .and_then(|value| value.get("lastEventTimeMs"))
                        .and_then(|value| value.as_i64())
                        .map(|ts| crate::normalizers::trade::now_ms().saturating_sub(ts).max(0))
                        .unwrap_or_default(),
                    "bookSynced": snapshot
                        .get("orderbookEvidenceAvailable")
                        .and_then(|value| value.as_bool())
                        .unwrap_or(false),
                    "snapshot": snapshot,
                });
                if sender.send(Message::Text(payload.to_string().into())).await.is_err() {
                    break;
                }
            }
            message = receiver.next() => {
                if message.is_none() {
                    break;
                }
            }
        }
    }
}

pub async fn new_token_watch_runtime_debug_route(
    State(state): State<AppState>,
    headers: HeaderMap,
) -> impl IntoResponse {
    if !state.operator_token_configured() {
        return (
            StatusCode::FORBIDDEN,
            Json(serde_json::json!({ "ok": false, "reason": "operator_token_missing" })),
        )
            .into_response();
    }
    if !state.operator_token_authorized(&headers) {
        return (
            StatusCode::UNAUTHORIZED,
            Json(serde_json::json!({ "ok": false, "reason": "operator_token_required" })),
        )
            .into_response();
    }
    let sessions = global_new_token_l2_runtime().sessions();
    Json(serde_json::json!({
        "readOnly": true,
        "executionEnabled": false,
        "sessionCount": sessions.len(),
        "readyCount": sessions.iter().filter(|item| item.orderbook_evidence_available).count(),
        "sessions": sessions,
    }))
    .into_response()
}

fn decorate_watchlist(
    mut list: crate::toxic_v3::new_token_watch::TokenWatchListResponse,
) -> crate::toxic_v3::new_token_watch::TokenWatchListResponse {
    for item in &mut list.items {
        decorate_item(item);
    }
    list
}

fn decorate_item(item: &mut TokenWatchItem) {
    let Some(session) = global_new_token_l2_runtime().session(&item.symbol) else {
        item.evidence_mode = "flow_only".to_string();
        item.orderbook_evidence_available = false;
        item.intent_assessment_available = false;
        item.intent_reason = "l2_session_not_started".to_string();
        return;
    };
    item.evidence_mode = session.evidence_mode;
    item.orderbook_evidence_available = session.orderbook_evidence_available;
    item.intent_assessment_available = session.intent_assessment_available;
    item.intent_reason = session.intent.reason;
    item.stream_status = format!("l2_{:?}", session.status).to_ascii_lowercase();
}

fn apply_session_to_reconstruction(
    raw_symbol: &str,
    response: &mut crate::toxic_v3::new_token_watch::SmartMoneyReconstructionResponse,
) {
    let session = global_new_token_l2_runtime().session(raw_symbol);
    let Some(session) = session else {
        return;
    };
    response.evidence_mode = session.evidence_mode;
    response.orderbook_evidence_available = session.orderbook_evidence_available;
    response.intent_assessment_available = session.intent_assessment_available;
    response.intent_reason = session.intent.reason.clone();
    response.l2_orderbook = Some(session.orderbook);
    response.l2_intent = Some(session.intent);
    response.l2_wall_evidence = session.wall_evidence;
    response.l2_trade_flow = Some(session.trade_flow);
    response.l2_open_interest = Some(session.open_interest);
    response.l2_listing_phase = session.listing_phase;
}

fn stream_payload_for_symbol(raw_symbol: &str) -> Option<serde_json::Value> {
    let item = global_new_token_watch_manager()
        .list_active_tokens()
        .items
        .into_iter()
        .find(|item| item.symbol.eq_ignore_ascii_case(raw_symbol))?;
    let mut item = item;
    decorate_item(&mut item);
    let session: Option<L2SessionSnapshot> = global_new_token_l2_runtime().session(&item.symbol);
    Some(serde_json::json!({
        "symbol": item.symbol,
        "item": item,
        "l2": session,
        "evidenceMode": item.evidence_mode,
        "orderbookEvidenceAvailable": item.orderbook_evidence_available,
        "intentAssessmentAvailable": item.intent_assessment_available,
    }))
}

fn token_error_response(error: TokenWatchError) -> axum::response::Response {
    let status = match error {
        TokenWatchError::InvalidSymbol => StatusCode::BAD_REQUEST,
        TokenWatchError::MaxActiveTokensReached => StatusCode::CONFLICT,
        TokenWatchError::TokenNotFound => StatusCode::NOT_FOUND,
        TokenWatchError::PersistenceFailed => StatusCode::INTERNAL_SERVER_ERROR,
    };
    (
        status,
        Json(serde_json::json!({
            "ok": false,
            "error": error.to_string(),
            "readOnly": true
        })),
    )
        .into_response()
}

fn require_new_token_operator(
    state: &AppState,
    headers: &HeaderMap,
) -> Option<axum::response::Response> {
    if !state.operator_token_configured() {
        return Some(
            (
                StatusCode::FORBIDDEN,
                Json(serde_json::json!({
                    "ok": false,
                    "reason": "operator_token_missing",
                    "readOnly": true,
                    "runtimeModified": false
                })),
            )
                .into_response(),
        );
    }
    if !state.operator_token_authorized(headers) {
        return Some(
            (
                StatusCode::UNAUTHORIZED,
                Json(serde_json::json!({
                    "ok": false,
                    "reason": "operator_token_required",
                    "readOnly": true,
                    "runtimeModified": false
                })),
            )
                .into_response(),
        );
    }
    None
}

fn error_response(error: TokenWatchError) -> axum::response::Response {
    let status = match error {
        TokenWatchError::InvalidSymbol => StatusCode::BAD_REQUEST,
        TokenWatchError::MaxActiveTokensReached => StatusCode::CONFLICT,
        TokenWatchError::TokenNotFound => StatusCode::NOT_FOUND,
        TokenWatchError::PersistenceFailed => StatusCode::INTERNAL_SERVER_ERROR,
    };
    (
        status,
        Json(NewTokenWatchMutationResponse {
            ok: false,
            item: None,
            items: global_new_token_watch_manager().list_active_tokens().items,
            error: Some(error.to_string()),
            max_active_tokens: MAX_ACTIVE_TOKENS,
            read_only: true,
        }),
    )
        .into_response()
}
