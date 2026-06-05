use axum::{
    extract::{
        ws::{Message, WebSocket, WebSocketUpgrade},
        Query, State,
    },
    response::IntoResponse,
};
use futures_util::{SinkExt, StreamExt};
use serde::Serialize;
use std::time::Duration;

use crate::{
    api::toxic_signal_inbox_routes::{build_recent, normalize_symbol_query},
    app::AppState,
    normalizers::trade::now_ms,
    types::toxic_signal_inbox::{ToxicSignalInboxItem, ToxicSignalInboxRecentResponse},
};

#[derive(Debug, serde::Deserialize)]
pub struct ToxicSignalWsQuery {
    symbol: Option<String>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ToxicSignalWsSnapshot {
    #[serde(rename = "type")]
    pub message_type: &'static str,
    pub read_only: bool,
    pub runtime_modified: bool,
    pub analysis_only: bool,
    pub execution_enabled: bool,
    pub selected_symbol: String,
    pub generated_at: String,
    pub signals: Vec<ToxicSignalWsItem>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ToxicSignalWsItem {
    pub id: String,
    pub symbol: String,
    pub detector: String,
    pub direction: String,
    pub severity: String,
    pub confidence: f64,
    pub created_at: String,
    pub final_result: String,
    pub core_reason: String,
    pub risk_score: u8,
    pub data_quality: f64,
    pub read_only: bool,
    pub runtime_modified: bool,
    pub analysis_only: bool,
    pub execution_enabled: bool,
}

pub async fn toxic_signal_ws_route(
    State(state): State<AppState>,
    Query(query): Query<ToxicSignalWsQuery>,
    ws: WebSocketUpgrade,
) -> impl IntoResponse {
    let selected_symbol = normalize_symbol_query(query.symbol, &state.config().symbol);
    ws.on_upgrade(move |socket| stream_signal_snapshots(socket, state, selected_symbol))
}

async fn stream_signal_snapshots(socket: WebSocket, state: AppState, selected_symbol: String) {
    let (mut sender, mut receiver) = socket.split();
    let mut interval = tokio::time::interval(ws_signal_interval());
    tracing::info!(target: "toxic_signal_ws", symbol = %selected_symbol, "ws client connected");
    loop {
        tokio::select! {
            _ = interval.tick() => {
                let recent = build_recent(&state, &selected_symbol);
                let snapshot = build_ws_snapshot(&recent);
                let Ok(payload) = serde_json::to_string(&snapshot) else {
                    tracing::warn!(target: "toxic_signal_ws", "ws snapshot skipped because serialization failed");
                    break;
                };
                if sender.send(Message::Text(payload)).await.is_err() {
                    break;
                }
                tracing::debug!(target: "toxic_signal_ws", signal_count = snapshot.signals.len(), "ws snapshot sent");
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
    tracing::info!(target: "toxic_signal_ws", symbol = %selected_symbol, "ws client disconnected");
}

pub fn build_ws_snapshot(recent: &ToxicSignalInboxRecentResponse) -> ToxicSignalWsSnapshot {
    ToxicSignalWsSnapshot {
        message_type: "signal_snapshot",
        read_only: true,
        runtime_modified: false,
        analysis_only: true,
        execution_enabled: false,
        selected_symbol: recent.selected_symbol.clone(),
        generated_at: rfc3339_from_ms(now_ms()),
        signals: recent.items.iter().map(redact_signal_item).collect(),
    }
}

fn redact_signal_item(item: &ToxicSignalInboxItem) -> ToxicSignalWsItem {
    ToxicSignalWsItem {
        id: item.signal_id.clone(),
        symbol: item.symbol.clone(),
        detector: item.signal_kind.clone(),
        direction: direction_value(&item.direction_bias).to_string(),
        severity: item.severity.clone(),
        confidence: item.confidence,
        created_at: rfc3339_from_ms(item.created_at_ms as i64),
        final_result: format!(
            "{} · {}",
            direction_label(&item.direction_bias),
            item.fusion.summary
        ),
        core_reason: item.fusion.summary.clone(),
        risk_score: risk_score_for(&item.severity),
        data_quality: data_quality_for(&item.quality.quality_bucket),
        read_only: true,
        runtime_modified: false,
        analysis_only: true,
        execution_enabled: false,
    }
}

fn ws_signal_interval() -> Duration {
    let ms = std::env::var("WS_SIGNAL_INTERVAL_MS")
        .ok()
        .and_then(|value| value.parse::<u64>().ok())
        .filter(|value| (250..=60_000).contains(value))
        .unwrap_or(1000);
    tracing::debug!(target: "toxic_signal_ws", interval_ms = ms, "ws interval configured");
    Duration::from_millis(ms)
}

fn direction_value(direction_bias: &str) -> &'static str {
    let value = direction_bias.to_ascii_lowercase();
    if value.contains("short") {
        "short"
    } else if value.contains("long") {
        "long"
    } else {
        "unknown"
    }
}

fn direction_label(direction_bias: &str) -> &'static str {
    let value = direction_bias.to_ascii_lowercase();
    if value.contains("short") {
        "Ask/Sell"
    } else if value.contains("long") {
        "Bid/Buy"
    } else if value.contains("trap") {
        "Trap Risk"
    } else {
        "Neutral"
    }
}

fn rfc3339_from_ms(ms: i64) -> String {
    chrono::DateTime::<chrono::Utc>::from_timestamp_millis(ms)
        .unwrap_or_else(chrono::Utc::now)
        .to_rfc3339_opts(chrono::SecondsFormat::Millis, true)
}

fn risk_score_for(severity: &str) -> u8 {
    match severity.to_ascii_lowercase().as_str() {
        "critical" => 92,
        "high" => 85,
        "medium" => 72,
        _ => 45,
    }
}

fn data_quality_for(bucket: &str) -> f64 {
    match bucket.to_ascii_lowercase().as_str() {
        "excellent" => 92.0,
        "good" => 82.0,
        "mixed" => 74.0,
        "weak" => 62.0,
        "bad" => 45.0,
        _ => 70.0,
    }
}

#[cfg(test)]
mod tests {
    use super::build_ws_snapshot;
    use crate::types::toxic_signal_inbox::{
        ToxicSignalInboxFusionSummary, ToxicSignalInboxGovernanceSummary, ToxicSignalInboxItem,
        ToxicSignalInboxMarkoutSummary, ToxicSignalInboxOperatorAction,
        ToxicSignalInboxQualitySummary, ToxicSignalInboxRecentResponse,
        ToxicSignalInboxRecommendationSummary, ToxicSignalInboxReplaySummary,
    };

    #[test]
    fn ws_snapshot_redacts_technical_fields() {
        let snapshot = build_ws_snapshot(&recent());
        let json = serde_json::to_string(&snapshot).expect("snapshot json");

        assert!(json.contains("signal_snapshot"));
        assert!(json.contains("finalResult"));
        assert!(json.contains("riskScore"));
        assert!(json.contains("dataQuality"));
        for forbidden in [
            "markout",
            "evidence",
            "stale",
            "token",
            "webhook",
            "rawPayload",
            "debug",
            "secret",
            "authorization",
            "operator",
            "apiKey",
        ] {
            assert!(
                !json.contains(forbidden),
                "forbidden field leaked: {forbidden}"
            );
        }
    }

    fn recent() -> ToxicSignalInboxRecentResponse {
        ToxicSignalInboxRecentResponse {
            read_only: true,
            runtime_modified: false,
            analysis_only: true,
            execution_enabled: false,
            manual_review_required: true,
            runtime_weight_modified: false,
            config_modified: false,
            mode: "analysis_only".to_string(),
            selected_symbol: "BTC-PERP".to_string(),
            status: "signal_inbox_ready".to_string(),
            warnings: vec![],
            items: vec![ToxicSignalInboxItem {
                signal_id: "sig_ws".to_string(),
                symbol: "BTC-PERP".to_string(),
                signal_kind: "spoofing_candidate".to_string(),
                direction_bias: "short_bias".to_string(),
                severity: "high".to_string(),
                confidence: 0.82,
                created_at_ms: 1_700_000_000_000,
                fusion: ToxicSignalInboxFusionSummary {
                    available: true,
                    summary: "large ask wall removed".to_string(),
                },
                replay: ToxicSignalInboxReplaySummary {
                    available: true,
                    evidence_count: 3,
                },
                markout: ToxicSignalInboxMarkoutSummary {
                    available: true,
                    one_minute: "adverse".to_string(),
                    five_minute: "adverse".to_string(),
                    fifteen_minute: "not_enough_data".to_string(),
                    one_hour: "not_enough_data".to_string(),
                },
                quality: ToxicSignalInboxQualitySummary {
                    available: true,
                    quality_bucket: "good".to_string(),
                    aligned_ratio: 0.8,
                    adverse_ratio: 0.2,
                },
                recommendation: ToxicSignalInboxRecommendationSummary {
                    available: true,
                    action: "review_evidence".to_string(),
                    no_trade_only: false,
                    manual_review_required: true,
                },
                governance: ToxicSignalInboxGovernanceSummary {
                    ledger_available: false,
                    latest_decision: "missing_ledger_evidence".to_string(),
                },
                operator_action: ToxicSignalInboxOperatorAction::ReviewEvidence,
                read_only: true,
                runtime_modified: false,
                analysis_only: true,
                execution_enabled: false,
            }],
        }
    }
}
