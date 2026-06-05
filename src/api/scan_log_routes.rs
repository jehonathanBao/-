use axum::{
    extract::{
        ws::{Message, WebSocket, WebSocketUpgrade},
        Query, State,
    },
    response::IntoResponse,
    Json,
};
use futures_util::{SinkExt, StreamExt};
use serde::{Deserialize, Serialize};

use crate::{app::AppState, runtime::scan_log::ScanLogItem};

#[derive(Debug, Deserialize)]
pub struct ScanLogRecentQuery {
    limit: Option<usize>,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ScanLogRecentResponse {
    pub read_only: bool,
    pub runtime_modified: bool,
    pub items: Vec<ScanLogItem>,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct ScanLogWsEvent {
    #[serde(rename = "type")]
    message_type: &'static str,
    read_only: bool,
    runtime_modified: bool,
    item: ScanLogItem,
}

pub async fn scan_log_recent_route(
    State(state): State<AppState>,
    Query(query): Query<ScanLogRecentQuery>,
) -> Json<ScanLogRecentResponse> {
    Json(ScanLogRecentResponse {
        read_only: true,
        runtime_modified: false,
        items: state.recent_scan_logs(query.limit.unwrap_or(100).clamp(1, 500)),
    })
}

pub async fn scan_log_ws_route(
    State(state): State<AppState>,
    ws: WebSocketUpgrade,
) -> impl IntoResponse {
    ws.on_upgrade(move |socket| stream_scan_logs(socket, state))
}

async fn stream_scan_logs(socket: WebSocket, state: AppState) {
    let (mut sender, mut receiver) = socket.split();
    let mut logs = state.subscribe_scan_logs();
    state.record_scan_log(
        "info",
        "scan_log_ws_connected",
        "Dashboard scan log stream connected",
        Some(state.config().symbol.clone()),
        None,
    );

    loop {
        tokio::select! {
            event = logs.recv() => {
                let item = match event {
                    Ok(item) => item,
                    Err(tokio::sync::broadcast::error::RecvError::Lagged(_)) => continue,
                    Err(tokio::sync::broadcast::error::RecvError::Closed) => break,
                };
                let event = ScanLogWsEvent {
                    message_type: "scan_log_event",
                    read_only: true,
                    runtime_modified: false,
                    item,
                };
                let Ok(payload) = serde_json::to_string(&event) else {
                    break;
                };
                if sender.send(Message::Text(payload)).await.is_err() {
                    break;
                }
            }
            incoming = receiver.next() => {
                match incoming {
                    Some(Ok(Message::Ping(payload))) => {
                        let _ = sender.send(Message::Pong(payload)).await;
                    }
                    Some(Ok(Message::Close(_))) | None => break,
                    Some(Ok(_)) => {}
                    Some(Err(_)) => break,
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::ScanLogRecentResponse;
    use crate::runtime::scan_log::ScanLogStore;

    #[test]
    fn scan_log_recent_response_is_read_only() {
        let store = ScanLogStore::new(50);
        store.push(
            "info",
            "tick",
            "scan tick",
            Some("BTC-PERP".to_string()),
            None,
        );
        let response = ScanLogRecentResponse {
            read_only: true,
            runtime_modified: false,
            items: store.recent(10),
        };

        let json = serde_json::to_string(&response).expect("scan log json");

        assert!(json.contains("\"readOnly\":true"));
        assert!(json.contains("\"runtimeModified\":false"));
        assert!(json.contains("scan tick"));
        assert!(!json.contains("authorization"));
        assert!(!json.contains("rawPayload"));
        assert!(!json.contains("markout"));
        assert!(!json.contains("evidence"));
        assert!(!json.contains("webhook"));
        assert!(!json.contains("token"));
    }
}
