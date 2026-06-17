use std::sync::OnceLock;
use std::time::Duration;

use axum::{
    extract::{
        ws::{Message, WebSocket, WebSocketUpgrade},
        Json, Query,
    },
    http::StatusCode,
    response::IntoResponse,
};
use futures_util::{SinkExt, StreamExt};
use serde::Deserialize;

use crate::toxic_v3::new_token_watch::{
    NewTokenWatchMutationResponse, NewTokenWatchRequest, TokenWatchError, TokenWatchManager,
    MAX_ACTIVE_TOKENS,
};

static NEW_TOKEN_WATCH_MANAGER: OnceLock<TokenWatchManager> = OnceLock::new();

pub fn global_new_token_watch_manager() -> &'static TokenWatchManager {
    NEW_TOKEN_WATCH_MANAGER.get_or_init(TokenWatchManager::default)
}

pub async fn new_token_watch_list_route() -> impl IntoResponse {
    Json(global_new_token_watch_manager().list_active_tokens())
}

#[derive(Debug, Deserialize)]
pub struct NewTokenReconstructionQuery {
    pub symbol: String,
    pub tf: Option<String>,
}

pub async fn new_token_watch_reconstruction_route(
    Query(query): Query<NewTokenReconstructionQuery>,
) -> impl IntoResponse {
    let timeframe = query.tf.as_deref().unwrap_or("15m");
    match global_new_token_watch_manager().get_reconstruction(&query.symbol, timeframe) {
        Ok(response) => Json(response).into_response(),
        Err(error) => token_error_response(error),
    }
}

pub async fn new_token_watch_chart_route(
    Query(query): Query<NewTokenReconstructionQuery>,
) -> impl IntoResponse {
    let timeframe = query.tf.as_deref().unwrap_or("15m");
    match global_new_token_watch_manager().get_chart(&query.symbol, timeframe) {
        Ok(response) => Json(response).into_response(),
        Err(error) => token_error_response(error),
    }
}

pub async fn new_token_watch_add_route(
    Json(request): Json<NewTokenWatchRequest>,
) -> impl IntoResponse {
    match global_new_token_watch_manager().add_token(&request.symbol) {
        Ok(item) => Json(NewTokenWatchMutationResponse {
            ok: true,
            item: Some(item),
            items: global_new_token_watch_manager().list_active_tokens().items,
            error: None,
            max_active_tokens: MAX_ACTIVE_TOKENS,
            read_only: true,
        })
        .into_response(),
        Err(error) => error_response(error),
    }
}

pub async fn new_token_watch_remove_route(
    Json(request): Json<NewTokenWatchRequest>,
) -> impl IntoResponse {
    match global_new_token_watch_manager().remove_token(&request.symbol) {
        Ok(item) => Json(NewTokenWatchMutationResponse {
            ok: true,
            item: Some(item),
            items: global_new_token_watch_manager().list_active_tokens().items,
            error: None,
            max_active_tokens: MAX_ACTIVE_TOKENS,
            read_only: true,
        })
        .into_response(),
        Err(error) => error_response(error),
    }
}

pub async fn new_token_watch_ws_route(ws: WebSocketUpgrade) -> impl IntoResponse {
    ws.on_upgrade(stream_new_token_watch)
}

pub async fn new_token_reconstruction_ws_route(ws: WebSocketUpgrade) -> impl IntoResponse {
    ws.on_upgrade(stream_new_token_reconstruction)
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

async fn stream_new_token_reconstruction(socket: WebSocket) {
    let (mut sender, mut receiver) = socket.split();
    let mut interval = tokio::time::interval(Duration::from_secs(10));

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

fn token_error_response(error: TokenWatchError) -> axum::response::Response {
    let status = match error {
        TokenWatchError::InvalidSymbol => StatusCode::BAD_REQUEST,
        TokenWatchError::MaxActiveTokensReached => StatusCode::CONFLICT,
        TokenWatchError::TokenNotFound => StatusCode::NOT_FOUND,
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

fn error_response(error: TokenWatchError) -> axum::response::Response {
    let status = match error {
        TokenWatchError::InvalidSymbol => StatusCode::BAD_REQUEST,
        TokenWatchError::MaxActiveTokensReached => StatusCode::CONFLICT,
        TokenWatchError::TokenNotFound => StatusCode::NOT_FOUND,
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
