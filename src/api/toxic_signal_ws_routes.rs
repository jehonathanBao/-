use axum::{
    extract::{
        ws::{Message, WebSocket, WebSocketUpgrade},
        Query, State,
    },
    response::IntoResponse,
};
use futures_util::{SinkExt, StreamExt};
use serde::Serialize;
use std::time::{Duration, Instant};

use crate::{
    api::discord_notification_routes::{
        discord_alert_status_for_key, evaluate_discord_alert_gate, DiscordAlertMode,
        DiscordNotificationRequest,
    },
    api::toxic_signal_inbox_routes::{build_recent, normalize_symbol_query},
    app::AppState,
    normalizers::trade::now_ms,
    runtime::{
        advanced_tof_metrics::{build_advanced_tof_metrics, AdvancedTofInput, AdvancedTofMetrics},
        perp_tof_metrics::{build_perp_tof_metrics, PerpTofInput, PerpTofMetrics},
        tof_metrics::{enhance_signal_summary, TofMetrics, TofSummaryInput},
    },
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
    pub tof_metrics: TofMetrics,
    pub tof_score: f64,
    pub perp_tof_metrics: PerpTofMetrics,
    pub perp_score: u8,
    pub perp_candidate_type: String,
    pub final_candidate_type: String,
    pub metrics_direction: crate::runtime::tof_metrics::TofDirection,
    pub merged_confidence: f64,
    pub advanced_tof_metrics: AdvancedTofMetrics,
    pub advanced_score: u8,
    pub advanced_candidate_type: String,
    pub final_risk_score: u8,
    pub candidate_type: String,
    pub explain_tags: Vec<String>,
    pub direction_label: String,
    pub direction_confidence: f64,
    pub direction_source: String,
    pub alert_status: String,
    pub alert_reason: String,
    pub discord_alert: DiscordAlertSummary,
    pub read_only: bool,
    pub runtime_modified: bool,
    pub analysis_only: bool,
    pub execution_enabled: bool,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DiscordAlertSummary {
    pub auto_eligible: bool,
    pub auto_sent: bool,
    pub last_decision: String,
    pub reason: String,
    pub sent_at: Option<String>,
    pub manual_sent_at: Option<String>,
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
    let tof_log_interval = tof_scan_log_interval();
    let mut last_tof_log = Instant::now();
    let mut tof_log_ready = true;
    tracing::info!(target: "toxic_signal_ws", symbol = %selected_symbol, "ws client connected");
    state.record_scan_log(
        "info",
        "signal_ws_connected",
        "Dashboard signal stream connected",
        Some(selected_symbol.clone()),
        None,
    );
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
                if !snapshot.signals.is_empty() {
                    state.record_scan_log(
                        "info",
                        "scan_candidates_detected",
                        format!("Signal scan snapshot contains {} candidate(s)", snapshot.signals.len()),
                        Some(selected_symbol.clone()),
                        None,
                    );
                    if tof_log_ready || last_tof_log.elapsed() >= tof_log_interval {
                        if let Some(signal) = snapshot.signals.first() {
                            state.record_scan_log(
                                "info",
                                "metrics_computed",
                                format!(
                                    "{} metrics computed: vpin={:.0} imbalance={:.2} spread={:.1}bps",
                                    signal.symbol,
                                    signal.tof_metrics.vpin_proxy,
                                    signal.tof_metrics.trade_imbalance,
                                    signal.tof_metrics.spread_bps
                                ),
                                Some(signal.symbol.clone()),
                                Some(signal.id.clone()),
                            );
                            state.record_scan_log(
                                "info",
                                "direction_resolved",
                                format!(
                                    "{} direction resolved: {} confidence={:.0}",
                                    signal.symbol, signal.direction_label, signal.direction_confidence
                                ),
                                Some(signal.symbol.clone()),
                                Some(signal.id.clone()),
                            );
                            state.record_scan_log(
                                "info",
                                "perp_metrics_computed",
                                format!(
                                    "{} perp metrics computed: oi={:.0} funding={:.4} liq={:.0} agfBuy={:.0} agfSell={:.0}",
                                    signal.symbol,
                                    signal.perp_tof_metrics.oi_change,
                                    signal.perp_tof_metrics.funding_rate,
                                    signal.perp_tof_metrics.liquidation_pressure,
                                    signal.perp_tof_metrics.agg_buy_volume,
                                    signal.perp_tof_metrics.agg_sell_volume
                                ),
                                Some(signal.symbol.clone()),
                                Some(signal.id.clone()),
                            );
                            state.record_scan_log(
                                "info",
                                "perp_candidate_generated",
                                format!(
                                    "{} perp candidate generated: type={} direction={:?} score={}",
                                    signal.symbol,
                                    signal.perp_candidate_type,
                                    signal.metrics_direction,
                                    signal.perp_score
                                ),
                                Some(signal.symbol.clone()),
                                Some(signal.id.clone()),
                            );
                            state.record_scan_log(
                                "info",
                                "advanced_metrics_computed",
                                format!(
                                    "{} advanced metrics computed: vpinEnhanced={:.0} flowCluster={:.0} fundingOiTrend={:.0} heatmap={:.0} finalScore={}",
                                    signal.symbol,
                                    signal.advanced_tof_metrics.vpin_enhanced,
                                    signal.advanced_tof_metrics.large_order_flow_cluster,
                                    signal.advanced_tof_metrics.historical_funding_oi_trend,
                                    signal.advanced_tof_metrics.market_pressure_heatmap,
                                    signal.final_risk_score
                                ),
                                Some(signal.symbol.clone()),
                                Some(signal.id.clone()),
                            );
                            last_tof_log = Instant::now();
                            tof_log_ready = false;
                        }
                    }
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
    let enhancement = enhancement_for_item(item);
    let existing_risk_score = risk_score_for(&item.severity);
    let existing_data_quality = data_quality_for(&item.quality.quality_bucket);
    let perp_metrics = build_perp_tof_metrics(&PerpTofInput {
        symbol: &item.symbol,
        spot_candidate_type: &enhancement.candidate_type,
        spot_direction: enhancement.direction,
        spot_risk_score: enhancement.final_risk_score,
        spot_data_quality: existing_data_quality,
        spot_confidence: item.confidence,
        summary: &item.fusion.summary,
    });
    let advanced_metrics = build_advanced_tof_metrics(&AdvancedTofInput {
        symbol: &item.symbol,
        spot_candidate_type: &enhancement.candidate_type,
        spot_direction: enhancement.direction,
        spot_risk_score: existing_risk_score,
        spot_data_quality: existing_data_quality,
        spot_confidence: item.confidence,
        tof_metrics: &enhancement.tof_metrics,
        spot_tags: &enhancement.explain_tags,
        perp_metrics: &perp_metrics,
        summary: &item.fusion.summary,
    });
    let final_risk_score = advanced_metrics.final_risk_score;
    let final_data_quality = advanced_metrics.data_quality;
    let alert_request = DiscordNotificationRequest {
        signal_id: Some(item.signal_id.clone()),
        id: Some(item.signal_id.clone()),
        dedupe_key: Some(item.signal_id.clone()),
        exchange: Some("Runtime".to_string()),
        symbol: Some(item.symbol.clone()),
        signal_type: Some(item.signal_kind.clone()),
        level: Some(item.severity.clone()),
        side: Some(enhancement.direction_label.clone()),
        score: Some(final_risk_score),
        data_quality: Some(final_data_quality),
        reason: Some(item.fusion.summary.clone()),
        impact: None,
        time: None,
        price_range: None,
        add_qty: None,
        cancel_qty: None,
        fill_qty: None,
        cancel_to_trade_ratio: None,
        depth_before: None,
        depth_after: None,
        depth_impact: None,
        price_impact_bps: None,
        markout_1s_bps: None,
        markout_5s_bps: None,
        markout_30s_bps: None,
        tof_metrics: Some(enhancement.tof_metrics.clone()),
        tof_score: Some(enhancement.tof_score),
        candidate_type: Some(advanced_metrics.candidate_type.clone()),
        explain_tags: Some(advanced_metrics.explain_tags.clone()),
        direction_confidence: Some(enhancement.direction_confidence),
        perp_tof_metrics: Some(perp_metrics.clone()),
        perp_score: Some(perp_metrics.risk_score),
        perp_candidate_type: Some(perp_metrics.candidate_type.clone()),
        final_candidate_type: Some(advanced_metrics.final_candidate_type.clone()),
        metrics_direction: serde_json::to_value(advanced_metrics.metrics_direction)
            .ok()
            .and_then(|value| value.as_str().map(str::to_string)),
        advanced_tof_metrics: Some(advanced_metrics.clone()),
        advanced_score: Some(advanced_metrics.final_risk_score),
        advanced_candidate_type: Some(advanced_metrics.candidate_type.clone()),
        test: None,
    };
    let alert_decision = evaluate_discord_alert_gate(&alert_request, DiscordAlertMode::Auto);
    let stored_alert = discord_alert_status_for_key(&item.signal_id);
    let alert_status = stored_alert
        .as_ref()
        .map(|status| status.last_decision.clone())
        .unwrap_or_else(|| {
            alert_status_from_reason(alert_decision.allowed, alert_decision.reason).to_string()
        });
    let alert_reason = stored_alert
        .as_ref()
        .map(|status| status.reason.clone())
        .unwrap_or_else(|| alert_decision.reason.to_string());
    let discord_alert = stored_alert
        .map(|status| DiscordAlertSummary {
            auto_eligible: status.auto_eligible,
            auto_sent: status.auto_sent,
            last_decision: status.last_decision,
            reason: status.reason,
            sent_at: status.sent_at,
            manual_sent_at: status.manual_sent_at,
        })
        .unwrap_or_else(|| DiscordAlertSummary {
            auto_eligible: alert_decision.allowed,
            auto_sent: false,
            last_decision: alert_status.clone(),
            reason: alert_reason.clone(),
            sent_at: None,
            manual_sent_at: None,
        });
    ToxicSignalWsItem {
        id: item.signal_id.clone(),
        symbol: item.symbol.clone(),
        detector: item.signal_kind.clone(),
        direction: serde_json::to_value(enhancement.direction)
            .ok()
            .and_then(|value| value.as_str().map(ToOwned::to_owned))
            .unwrap_or_else(|| direction_value(&item.direction_bias).to_string()),
        severity: item.severity.clone(),
        confidence: item.confidence,
        created_at: rfc3339_from_ms(item.created_at_ms as i64),
        final_result: format!(
            "{} · {}",
            direction_label(&item.direction_bias),
            item.fusion.summary
        ),
        core_reason: item.fusion.summary.clone(),
        risk_score: final_risk_score,
        data_quality: final_data_quality,
        tof_metrics: enhancement.tof_metrics,
        tof_score: enhancement.tof_score,
        perp_tof_metrics: perp_metrics.clone(),
        perp_score: perp_metrics.risk_score,
        perp_candidate_type: perp_metrics.candidate_type,
        final_candidate_type: advanced_metrics.final_candidate_type.clone(),
        metrics_direction: advanced_metrics.metrics_direction,
        merged_confidence: advanced_metrics.confidence,
        advanced_tof_metrics: advanced_metrics.clone(),
        advanced_score: advanced_metrics.final_risk_score,
        advanced_candidate_type: advanced_metrics.candidate_type.clone(),
        final_risk_score,
        candidate_type: advanced_metrics.candidate_type,
        explain_tags: advanced_metrics.explain_tags,
        direction_label: enhancement.direction_label,
        direction_confidence: enhancement.direction_confidence,
        direction_source: enhancement.direction_source,
        alert_status,
        alert_reason,
        discord_alert,
        read_only: true,
        runtime_modified: false,
        analysis_only: true,
        execution_enabled: false,
    }
}

fn alert_status_from_reason(allowed: bool, reason: &str) -> &'static str {
    if allowed {
        "eligible"
    } else if matches!(
        reason,
        "score_below_threshold" | "data_quality_below_threshold" | "non_high_risk"
    ) {
        "rejected"
    } else {
        "skipped"
    }
}

fn enhancement_for_item(
    item: &ToxicSignalInboxItem,
) -> crate::runtime::tof_metrics::TofSignalEnhancement {
    enhance_signal_summary(&TofSummaryInput {
        signal_kind: &item.signal_kind,
        direction_bias: &item.direction_bias,
        severity: &item.severity,
        confidence: item.confidence,
        quality_bucket: &item.quality.quality_bucket,
        summary: &item.fusion.summary,
        existing_risk_score: risk_score_for(&item.severity),
        existing_data_quality: data_quality_for(&item.quality.quality_bucket),
    })
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

fn tof_scan_log_interval() -> Duration {
    let seconds = std::env::var("TOF_SCAN_LOG_INTERVAL_SECONDS")
        .ok()
        .and_then(|value| value.parse::<u64>().ok())
        .filter(|value| (1..=300).contains(value))
        .unwrap_or(5);
    Duration::from_secs(seconds)
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
        assert!(json.contains("tofMetrics"));
        assert!(json.contains("perpTofMetrics"));
        assert!(json.contains("perpCandidateType"));
        assert!(json.contains("finalCandidateType"));
        assert!(json.contains("metricsDirection"));
        assert!(json.contains("advancedTofMetrics"));
        assert!(json.contains("advancedCandidateType"));
        assert!(json.contains("advancedScore"));
        assert!(json.contains("candidateType"));
        assert!(json.contains("explainTags"));
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
