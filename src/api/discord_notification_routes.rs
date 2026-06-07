use std::{
    collections::{HashMap, HashSet, VecDeque},
    sync::{Mutex, OnceLock},
    time::{Duration, Instant},
};

use crate::alerts::discord_message_builder::{
    DiscordEmbed, DiscordEmbedField, DiscordEmbedFooter, DiscordWebhookPayload,
};
use crate::app::AppState;
use crate::runtime::advanced_tof_metrics::AdvancedTofMetrics;
use crate::runtime::perp_tof_metrics::PerpTofMetrics;
use crate::runtime::tof_metrics::TofMetrics;
use axum::{extract::State, http::StatusCode, response::IntoResponse, Json};
use reqwest::Url;
use serde::{Deserialize, Serialize};

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DiscordNotificationRequest {
    pub signal_id: Option<String>,
    pub id: Option<String>,
    pub dedupe_key: Option<String>,
    pub exchange: Option<String>,
    pub symbol: Option<String>,
    pub signal_type: Option<String>,
    pub level: Option<String>,
    pub side: Option<String>,
    pub score: Option<u8>,
    pub data_quality: Option<f64>,
    pub reason: Option<String>,
    pub impact: Option<String>,
    pub time: Option<String>,
    pub price_range: Option<String>,
    pub add_qty: Option<f64>,
    pub cancel_qty: Option<f64>,
    pub fill_qty: Option<f64>,
    pub cancel_to_trade_ratio: Option<f64>,
    pub depth_before: Option<f64>,
    pub depth_after: Option<f64>,
    pub depth_impact: Option<f64>,
    pub price_impact_bps: Option<f64>,
    pub markout_1s_bps: Option<f64>,
    pub markout_5s_bps: Option<f64>,
    pub markout_30s_bps: Option<f64>,
    pub tof_metrics: Option<TofMetrics>,
    pub tof_score: Option<f64>,
    pub candidate_type: Option<String>,
    pub explain_tags: Option<Vec<String>>,
    pub direction_confidence: Option<f64>,
    pub perp_tof_metrics: Option<PerpTofMetrics>,
    pub perp_score: Option<u8>,
    pub perp_candidate_type: Option<String>,
    pub final_candidate_type: Option<String>,
    pub metrics_direction: Option<String>,
    pub advanced_tof_metrics: Option<AdvancedTofMetrics>,
    pub advanced_score: Option<u8>,
    pub advanced_candidate_type: Option<String>,
    pub test: Option<bool>,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DiscordNotificationResponse {
    pub ok: bool,
    pub configured: bool,
    pub reason: &'static str,
    pub min_score: u8,
    pub min_data_quality: f64,
    pub sent: bool,
    pub read_only: bool,
    pub execution_enabled: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DiscordAlertMode {
    Manual,
    Auto,
}

#[derive(Debug, Clone, PartialEq)]
pub struct AlertGateDecision {
    pub allowed: bool,
    pub reason: &'static str,
    pub severity_allowed: bool,
    pub score: u8,
    pub data_quality: f64,
    pub min_score: u8,
    pub min_data_quality: f64,
    pub auto_push_enabled: bool,
    pub dry_run: bool,
    pub configured: bool,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DiscordAlertPublicStatus {
    pub auto_eligible: bool,
    pub auto_sent: bool,
    pub last_decision: String,
    pub reason: String,
    pub sent_at: Option<String>,
    pub manual_sent_at: Option<String>,
}

pub async fn discord_notification_proxy(
    State(state): State<AppState>,
    Json(body): Json<DiscordNotificationRequest>,
) -> impl IntoResponse {
    let gate = AlertGate::from_env();
    let is_test = body.test.unwrap_or(false);
    let decision = evaluate_discord_alert_gate(&body, DiscordAlertMode::Manual);
    if !is_test {
        record_alert_gate_log(&state, &body, &decision, false);
    }
    if !is_test && !decision.allowed {
        record_discord_alert_status(&body, &decision, false, None);
        record_discord_log(
            &state,
            "warn",
            "discord_manual_push_skipped",
            alert_gate_message("Discord manual push skipped", &decision),
            &body,
        );
        return (
            StatusCode::OK,
            Json(DiscordNotificationResponse {
                ok: false,
                configured: decision.configured,
                reason: decision.reason,
                min_score: gate.min_score,
                min_data_quality: gate.min_data_quality,
                sent: false,
                read_only: true,
                execution_enabled: false,
            }),
        )
            .into_response();
    }

    let Some(webhook_url) = discord_webhook_url() else {
        record_discord_log(
            &state,
            "warn",
            "discord_config_missing",
            "Discord push skipped: Discord is not configured",
            &body,
        );
        return (
            StatusCode::OK,
            Json(DiscordNotificationResponse {
                ok: false,
                configured: false,
                reason: "DISCORD_NOT_CONFIGURED",
                min_score: gate.min_score,
                min_data_quality: gate.min_data_quality,
                sent: false,
                read_only: true,
                execution_enabled: false,
            }),
        )
            .into_response();
    };

    if validate_discord_webhook_url(&webhook_url).is_err() {
        record_discord_log(
            &state,
            "error",
            "discord_push_failed",
            "Discord push failed: configuration is invalid",
            &body,
        );
        return (
            StatusCode::BAD_REQUEST,
            Json(DiscordNotificationResponse {
                ok: false,
                configured: true,
                reason: "DISCORD_WEBHOOK_URL_INVALID",
                min_score: gate.min_score,
                min_data_quality: gate.min_data_quality,
                sent: false,
                read_only: true,
                execution_enabled: false,
            }),
        )
            .into_response();
    }

    let push_key = if is_test {
        None
    } else {
        push_dedupe_key(&body)
    };
    let cooldown_key = if is_test {
        None
    } else {
        push_cooldown_key(&body)
    };
    if let Some(key) = push_key.as_deref() {
        if let Some(reason) = discord_push_limiter()
            .lock()
            .expect("discord limiter")
            .reserve(key, cooldown_key.as_deref())
        {
            let limited_decision = AlertGateDecision {
                allowed: false,
                reason,
                ..decision.clone()
            };
            record_discord_alert_status(&body, &limited_decision, false, None);
            record_discord_log(
                &state,
                "warn",
                "discord_push_skipped",
                format!("Discord push skipped: {reason}"),
                &body,
            );
            return (
                StatusCode::OK,
                Json(DiscordNotificationResponse {
                    ok: false,
                    configured: true,
                    reason,
                    min_score: gate.min_score,
                    min_data_quality: gate.min_data_quality,
                    sent: false,
                    read_only: true,
                    execution_enabled: false,
                }),
            )
                .into_response();
        }
    }

    let client = match reqwest::Client::builder()
        .timeout(Duration::from_secs(alert_http_timeout_secs()))
        .build()
    {
        Ok(client) => client,
        Err(_) => {
            if let Some(key) = push_key.as_deref() {
                discord_push_limiter()
                    .lock()
                    .expect("discord limiter")
                    .release(key, cooldown_key.as_deref());
            }
            record_discord_log(
                &state,
                "error",
                "discord_push_failed",
                "Discord push failed: HTTP client unavailable",
                &body,
            );
            return (
                StatusCode::BAD_GATEWAY,
                Json(DiscordNotificationResponse {
                    ok: false,
                    configured: true,
                    reason: "DISCORD_HTTP_CLIENT_BUILD_FAILED",
                    min_score: gate.min_score,
                    min_data_quality: gate.min_data_quality,
                    sent: false,
                    read_only: true,
                    execution_enabled: false,
                }),
            )
                .into_response();
        }
    };
    let payload = discord_payload(&body);
    record_discord_log(
        &state,
        "info",
        "discord_manual_push_queued",
        "Discord manual push queued for alert-only delivery",
        &body,
    );
    let result = client
        .post(webhook_url)
        .json(&payload)
        .send()
        .await
        .and_then(|response| response.error_for_status());

    match result {
        Ok(_) => {
            record_discord_alert_status(&body, &decision, false, Some(now_rfc3339()));
            record_discord_log(
                &state,
                "info",
                "discord_manual_push_sent",
                "Discord manual push sent successfully",
                &body,
            );
            (
                StatusCode::OK,
                Json(DiscordNotificationResponse {
                    ok: true,
                    configured: true,
                    reason: "DISCORD_WEBHOOK_SENT",
                    min_score: gate.min_score,
                    min_data_quality: gate.min_data_quality,
                    sent: true,
                    read_only: true,
                    execution_enabled: false,
                }),
            )
                .into_response()
        }
        Err(err) => {
            if let Some(key) = push_key.as_deref() {
                discord_push_limiter()
                    .lock()
                    .expect("discord limiter")
                    .release(key, cooldown_key.as_deref());
            }
            let reason = if err.is_timeout() {
                "DISCORD_WEBHOOK_TIMEOUT"
            } else {
                "DISCORD_WEBHOOK_SEND_FAILED"
            };
            let failed_decision = AlertGateDecision {
                allowed: false,
                reason,
                ..decision.clone()
            };
            record_discord_alert_status(&body, &failed_decision, false, None);
            record_discord_log(
                &state,
                "error",
                "discord_push_failed",
                format!("Discord push failed: {reason}"),
                &body,
            );
            (
                StatusCode::BAD_GATEWAY,
                Json(DiscordNotificationResponse {
                    ok: false,
                    configured: true,
                    reason,
                    min_score: gate.min_score,
                    min_data_quality: gate.min_data_quality,
                    sent: false,
                    read_only: true,
                    execution_enabled: false,
                }),
            )
                .into_response()
        }
    }
}

pub async fn maybe_auto_push_discord(
    state: &AppState,
    body: DiscordNotificationRequest,
    created_at_ms: u64,
) -> AlertGateDecision {
    let mut decision = evaluate_discord_alert_gate(&body, DiscordAlertMode::Auto);
    let candidate_key = push_dedupe_key(&body)
        .or(body.id.clone())
        .unwrap_or_else(|| "unknown_candidate".to_string());

    if !discord_auto_push_cached_on_boot() && created_at_ms < state.booted_at_ms().max(0) as u64 {
        decision.allowed = false;
        decision.reason = "cached_on_boot";
        if discord_auto_push_tracker()
            .lock()
            .expect("discord auto tracker")
            .mark_once(&candidate_key)
        {
            set_discord_alert_status(&candidate_key, status_from_decision(&decision, false, None));
            record_alert_gate_log(state, &body, &decision, true);
            record_discord_log(
                state,
                "warn",
                "discord_auto_push_skipped",
                "Discord auto push skipped: cached candidate from before backend boot",
                &body,
            );
        }
        return decision;
    }

    if !discord_auto_push_tracker()
        .lock()
        .expect("discord auto tracker")
        .mark_once(&candidate_key)
    {
        decision.allowed = false;
        decision.reason = "duplicate_candidate";
        set_discord_alert_status(&candidate_key, status_from_decision(&decision, false, None));
        return decision;
    }

    record_alert_gate_log(state, &body, &decision, true);
    if !decision.allowed {
        set_discord_alert_status(&candidate_key, status_from_decision(&decision, false, None));
        let kind = if matches!(
            decision.reason,
            "score_below_threshold" | "data_quality_below_threshold" | "non_high_risk"
        ) {
            "discord_auto_push_rejected"
        } else {
            "discord_auto_push_skipped"
        };
        record_discord_log(
            state,
            "warn",
            kind,
            alert_gate_message("Discord auto push skipped", &decision),
            &body,
        );
        return decision;
    }

    let Some(webhook_url) = discord_webhook_url() else {
        decision.allowed = false;
        decision.reason = "webhook_missing";
        set_discord_alert_status(&candidate_key, status_from_decision(&decision, false, None));
        record_discord_log(
            state,
            "warn",
            "discord_auto_push_skipped",
            "Discord auto push skipped: Discord is not configured",
            &body,
        );
        return decision;
    };
    if validate_discord_webhook_url(&webhook_url).is_err() {
        decision.allowed = false;
        decision.reason = "webhook_invalid";
        set_discord_alert_status(&candidate_key, status_from_decision(&decision, false, None));
        record_discord_log(
            state,
            "error",
            "discord_auto_push_failed",
            "Discord auto push failed: configuration is invalid",
            &body,
        );
        return decision;
    }

    let push_key = push_dedupe_key(&body);
    let cooldown_key = push_cooldown_key(&body);
    if let Some(key) = push_key.as_deref() {
        if let Some(reason) = discord_push_limiter()
            .lock()
            .expect("discord limiter")
            .reserve(key, cooldown_key.as_deref())
        {
            decision.allowed = false;
            decision.reason = match reason {
                "DUPLICATE_PUSH_SUPPRESSED" => "duplicate",
                "COOLDOWN_SUPPRESSED" => "cooldown",
                "RATE_LIMITED" => "rate_limited",
                _ => "push_limited",
            };
            set_discord_alert_status(&candidate_key, status_from_decision(&decision, false, None));
            record_discord_log(
                state,
                "warn",
                "discord_auto_push_skipped",
                alert_gate_message("Discord auto push skipped", &decision),
                &body,
            );
            return decision;
        }
    }

    let client = match reqwest::Client::builder()
        .timeout(Duration::from_secs(alert_http_timeout_secs()))
        .build()
    {
        Ok(client) => client,
        Err(_) => {
            release_reserved_push(&body, push_key.as_deref(), cooldown_key.as_deref());
            decision.allowed = false;
            decision.reason = "http_client_unavailable";
            set_discord_alert_status(&candidate_key, status_from_decision(&decision, false, None));
            record_discord_log(
                state,
                "error",
                "discord_auto_push_failed",
                "Discord auto push failed: HTTP client unavailable",
                &body,
            );
            return decision;
        }
    };

    record_discord_log(
        state,
        "info",
        "discord_auto_push_queued",
        "Discord auto push queued for alert-only delivery",
        &body,
    );
    let payload = discord_payload(&body);
    let result = client
        .post(webhook_url)
        .json(&payload)
        .send()
        .await
        .and_then(|response| response.error_for_status());

    match result {
        Ok(_) => {
            decision.reason = "sent";
            set_discord_alert_status(
                &candidate_key,
                status_from_decision(&decision, true, Some(now_rfc3339())),
            );
            record_discord_log(
                state,
                "info",
                "discord_auto_push_sent",
                "Discord auto push sent successfully",
                &body,
            );
        }
        Err(err) => {
            release_reserved_push(&body, push_key.as_deref(), cooldown_key.as_deref());
            decision.allowed = false;
            decision.reason = if err.is_timeout() {
                "timeout"
            } else {
                "send_failed"
            };
            set_discord_alert_status(&candidate_key, status_from_decision(&decision, false, None));
            record_discord_log(
                state,
                "error",
                "discord_auto_push_failed",
                alert_gate_message("Discord auto push failed", &decision),
                &body,
            );
        }
    }
    decision
}

#[derive(Debug, Clone, Copy)]
struct AlertGate {
    min_score: u8,
    min_data_quality: f64,
}

impl AlertGate {
    fn from_env() -> Self {
        Self {
            min_score: std::env::var("ALERT_MIN_SCORE")
                .ok()
                .and_then(|value| value.parse::<u8>().ok())
                .unwrap_or(80),
            min_data_quality: std::env::var("ALERT_MIN_DATA_QUALITY")
                .ok()
                .and_then(|value| value.parse::<f64>().ok())
                .unwrap_or(70.0),
        }
    }
}

pub fn evaluate_discord_alert_gate(
    signal: &DiscordNotificationRequest,
    mode: DiscordAlertMode,
) -> AlertGateDecision {
    let gate = AlertGate::from_env();
    let severity_allowed = severity_allows(signal.level.as_deref());
    let score = signal.score.unwrap_or(0);
    let data_quality = signal.data_quality.unwrap_or(0.0);
    let auto_push_enabled = discord_auto_push_enabled();
    let dry_run = discord_dry_run();
    let configured = discord_webhook_url().is_some();
    let mut decision = AlertGateDecision {
        allowed: true,
        reason: "passed",
        severity_allowed,
        score,
        data_quality,
        min_score: gate.min_score,
        min_data_quality: gate.min_data_quality,
        auto_push_enabled,
        dry_run,
        configured,
    };

    if matches!(mode, DiscordAlertMode::Auto) && !auto_push_enabled {
        decision.allowed = false;
        decision.reason = "auto_disabled";
    } else if !severity_allowed {
        decision.allowed = false;
        decision.reason = "non_high_risk";
    } else if score < gate.min_score {
        decision.allowed = false;
        decision.reason = "score_below_threshold";
    } else if data_quality < gate.min_data_quality {
        decision.allowed = false;
        decision.reason = "data_quality_below_threshold";
    } else if dry_run {
        decision.allowed = false;
        decision.reason = "dry_run";
    } else if !configured {
        decision.allowed = false;
        decision.reason = "webhook_missing";
    }

    decision
}

fn severity_allows(level: Option<&str>) -> bool {
    matches!(
        level.unwrap_or("").trim().to_ascii_lowercase().as_str(),
        "high" | "critical" | "a" | "s"
    )
}

fn discord_webhook_url() -> Option<String> {
    std::env::var("DISCORD_WEBHOOK_URL")
        .ok()
        .filter(|value| !value.trim().is_empty())
}

fn discord_dry_run() -> bool {
    parse_bool_env("DRY_RUN", true)
}

fn discord_auto_push_enabled() -> bool {
    parse_bool_env("DISCORD_AUTO_PUSH_ENABLED", true)
}

fn discord_auto_push_cached_on_boot() -> bool {
    parse_bool_env("DISCORD_AUTO_PUSH_CACHED_ON_BOOT", false)
}

fn parse_bool_env(key: &str, default: bool) -> bool {
    std::env::var(key)
        .ok()
        .map(|value| {
            matches!(
                value.trim().to_ascii_lowercase().as_str(),
                "1" | "true" | "yes" | "on"
            )
        })
        .unwrap_or(default)
}

fn alert_http_timeout_secs() -> u64 {
    std::env::var("ALERT_HTTP_TIMEOUT_SECS")
        .ok()
        .and_then(|value| value.parse::<u64>().ok())
        .filter(|value| *value > 0)
        .unwrap_or(5)
}

fn push_dedupe_key(signal: &DiscordNotificationRequest) -> Option<String> {
    signal
        .dedupe_key
        .as_deref()
        .or(signal.signal_id.as_deref())
        .or(signal.id.as_deref())
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(str::to_string)
}

fn push_cooldown_key(signal: &DiscordNotificationRequest) -> Option<String> {
    let symbol = signal.symbol.as_deref()?.trim();
    if symbol.is_empty() {
        return None;
    }
    let side = signal.side.as_deref().unwrap_or("unknown").trim();
    let signal_type = signal.signal_type.as_deref().unwrap_or("unknown").trim();
    Some(format!(
        "{}:{}:{}",
        symbol.to_ascii_uppercase(),
        signal_type.to_ascii_lowercase(),
        side.to_ascii_lowercase()
    ))
}

fn discord_push_cooldown_secs() -> u64 {
    std::env::var("DISCORD_PUSH_COOLDOWN_SECONDS")
        .ok()
        .and_then(|value| value.parse::<u64>().ok())
        .filter(|value| *value > 0)
        .unwrap_or(60)
}

fn record_discord_log(
    state: &AppState,
    level: &'static str,
    kind: &'static str,
    message: impl AsRef<str>,
    body: &DiscordNotificationRequest,
) {
    state.record_scan_log(
        level,
        kind,
        message,
        body.symbol.clone(),
        push_dedupe_key(body).or(body.id.clone()),
    );
}

fn record_alert_gate_log(
    state: &AppState,
    body: &DiscordNotificationRequest,
    decision: &AlertGateDecision,
    auto_push: bool,
) {
    state.record_scan_log(
        "info",
        "alert_gate_evaluated",
        format!(
            "{} alert gate evaluated: severity={} score={} quality={:.0} autoPush={} reason={}",
            body.symbol.as_deref().unwrap_or("unknown"),
            body.level.as_deref().unwrap_or("unknown"),
            decision.score,
            decision.data_quality,
            auto_push,
            decision.reason
        ),
        body.symbol.clone(),
        push_dedupe_key(body).or(body.id.clone()),
    );
}

fn alert_gate_message(prefix: &str, decision: &AlertGateDecision) -> String {
    match decision.reason {
        "score_below_threshold" => format!(
            "{prefix}: score={} below {}",
            decision.score, decision.min_score
        ),
        "data_quality_below_threshold" => format!(
            "{prefix}: dataQuality={:.0} below {:.0}",
            decision.data_quality, decision.min_data_quality
        ),
        "non_high_risk" => format!("{prefix}: Medium/Low candidate is display-only"),
        "auto_disabled" => format!("{prefix}: auto push disabled"),
        "dry_run" => format!("{prefix}: dry run enabled"),
        "webhook_missing" => format!("{prefix}: Discord is not configured"),
        "cached_on_boot" => format!("{prefix}: cached candidate from before backend boot"),
        "duplicate" | "duplicate_candidate" => format!("{prefix}: duplicate candidate"),
        "cooldown" => format!("{prefix}: cooldown active"),
        "rate_limited" => format!("{prefix}: rate limit active"),
        "timeout" => format!("{prefix}: timeout"),
        "send_failed" => format!("{prefix}: send failed"),
        other => format!("{prefix}: {other}"),
    }
}

fn release_reserved_push(
    _body: &DiscordNotificationRequest,
    push_key: Option<&str>,
    cooldown_key: Option<&str>,
) {
    if let Some(key) = push_key {
        discord_push_limiter()
            .lock()
            .expect("discord limiter")
            .release(key, cooldown_key);
    }
}

fn validate_discord_webhook_url(raw: &str) -> anyhow::Result<()> {
    let url = Url::parse(raw)?;
    anyhow::ensure!(url.scheme() == "https", "discord webhook must use https");
    anyhow::ensure!(
        matches!(url.host_str(), Some("discord.com" | "discordapp.com")),
        "discord webhook host is not allowed"
    );
    anyhow::ensure!(
        url.path().starts_with("/api/webhooks/"),
        "discord webhook path is not allowed"
    );
    Ok(())
}

fn discord_payload(signal: &DiscordNotificationRequest) -> DiscordWebhookPayload {
    if signal.test.unwrap_or(false) {
        return DiscordWebhookPayload {
            content: Some(
                "TEST MESSAGE / 测试消息: toxic-order-monitor Discord proxy is reachable."
                    .to_string(),
            ),
            embeds: Vec::new(),
        };
    }

    let symbol = signal
        .symbol
        .clone()
        .unwrap_or_else(|| "Unknown".to_string());
    let event_type = signal
        .signal_type
        .clone()
        .unwrap_or_else(|| "Unknown".to_string());
    let side = signal.side.as_deref().unwrap_or("N/A");
    let exchange = signal.exchange.as_deref().unwrap_or("N/A");
    let direction = normalize_signal_direction(signal.side.as_deref());
    let direction_label = discord_direction_label(direction);
    let final_result = signal
        .reason
        .as_deref()
        .filter(|value| !value.trim().is_empty())
        .unwrap_or("无法判断方向");

    let mut fields = vec![
        DiscordEmbedField {
            name: "方向".to_string(),
            value: direction_field_value(direction_label, signal.direction_confidence).to_string(),
            inline: true,
        },
        DiscordEmbedField {
            name: "最终结果".to_string(),
            value: final_result.to_string(),
            inline: false,
        },
        DiscordEmbedField {
            name: "风险评分".to_string(),
            value: signal
                .score
                .map(|value| format!("{value}/100"))
                .unwrap_or_else(|| "N/A".to_string()),
            inline: true,
        },
        DiscordEmbedField {
            name: "数据质量".to_string(),
            value: signal
                .data_quality
                .map(|value| format!("{value:.0}/100"))
                .unwrap_or_else(|| "N/A".to_string()),
            inline: true,
        },
    ];
    if let Some(candidate_type) = signal
        .candidate_type
        .as_deref()
        .filter(|value| !value.trim().is_empty())
    {
        fields.push(DiscordEmbedField {
            name: "候选类型".to_string(),
            value: candidate_type.to_string(),
            inline: true,
        });
    }
    if let Some(tof_metrics) = signal.tof_metrics.as_ref() {
        fields.push(DiscordEmbedField {
            name: "TOF 指标".to_string(),
            value: format!(
                "TOF {:.0} / VPIN {:.0} / Imbalance {:.2} / Spread {:.1}bps / Depth {:.0}",
                signal.tof_score.unwrap_or(tof_metrics.tof_score),
                tof_metrics.vpin_proxy,
                tof_metrics.trade_imbalance,
                tof_metrics.spread_bps,
                tof_metrics.depth_withdrawal_score
            ),
            inline: false,
        });
    }
    if let Some(tags) = signal.explain_tags.as_ref().filter(|tags| !tags.is_empty()) {
        fields.push(DiscordEmbedField {
            name: "核心解释标签".to_string(),
            value: tags
                .iter()
                .take(6)
                .map(String::as_str)
                .collect::<Vec<_>>()
                .join(" + "),
            inline: false,
        });
    }
    if let Some(final_candidate_type) = signal
        .final_candidate_type
        .as_deref()
        .filter(|value| !value.trim().is_empty())
    {
        fields.push(DiscordEmbedField {
            name: "最终候选".to_string(),
            value: final_candidate_type.to_string(),
            inline: true,
        });
    }
    if let Some(perp_metrics) = signal.perp_tof_metrics.as_ref() {
        fields.push(DiscordEmbedField {
            name: "合约 TOF 指标".to_string(),
            value: format!(
                "Type {} / OI {:.0} ({}) / Funding {:.4}% {} / Liq {:.0} {} / AGF buy {:.0} sell {:.0}",
                signal
                    .perp_candidate_type
                    .as_deref()
                    .unwrap_or(perp_metrics.candidate_type.as_str()),
                perp_metrics.oi_change,
                perp_metrics.oi_direction,
                perp_metrics.funding_rate,
                perp_metrics.funding_side,
                perp_metrics.liquidation_pressure,
                perp_metrics.squeeze_side,
                perp_metrics.agg_buy_volume,
                perp_metrics.agg_sell_volume,
            ),
            inline: false,
        });
    }
    if let Some(advanced_metrics) = signal.advanced_tof_metrics.as_ref() {
        fields.push(DiscordEmbedField {
            name: "高级指标".to_string(),
            value: format!(
                "Type {} / VPIN+ {:.0} / FlowCluster {:.0} / FundingOI {:.0} / Heatmap {:.0} / Final {}",
                signal
                    .advanced_candidate_type
                    .as_deref()
                    .unwrap_or(advanced_metrics.candidate_type.as_str()),
                advanced_metrics.vpin_enhanced,
                advanced_metrics.large_order_flow_cluster,
                advanced_metrics.historical_funding_oi_trend,
                advanced_metrics.market_pressure_heatmap,
                signal
                    .advanced_score
                    .unwrap_or(advanced_metrics.final_risk_score),
            ),
            inline: false,
        });
    }
    fields.push(DiscordEmbedField {
        name: "说明".to_string(),
        value: "该信号基于公开盘口 / L2 数据推断，为 Candidate，不是执法或定性结论。".to_string(),
        inline: false,
    });

    DiscordWebhookPayload {
        content: None,
        embeds: vec![DiscordEmbed {
            title: format!(
                "{} 疑似有毒订单候选信号：{symbol}",
                discord_direction_emoji(direction)
            ),
            description: format!("{exchange} / {symbol} · {event_type} · {side}"),
            color: discord_embed_color_from_direction(direction),
            fields,
            footer: Some(DiscordEmbedFooter {
                text: format!(
                    "Candidate only. Signal: {}",
                    signal.signal_id.as_deref().unwrap_or("N/A")
                ),
            }),
            timestamp: signal.time.clone(),
        }],
    }
}

fn direction_field_value(direction_label: &str, confidence: Option<f64>) -> String {
    confidence.map_or_else(
        || direction_label.to_string(),
        |confidence| format!("{direction_label}，置信度 {:.0}", confidence),
    )
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum NormalizedDiscordDirection {
    Bullish,
    Bearish,
    Neutral,
}

pub fn normalize_signal_direction(raw: Option<&str>) -> NormalizedDiscordDirection {
    let value = raw.unwrap_or("").to_ascii_lowercase();
    if value.contains("bid")
        || value.contains("buy")
        || value.contains("long")
        || value.contains("bull")
    {
        NormalizedDiscordDirection::Bullish
    } else if value.contains("ask")
        || value.contains("sell")
        || value.contains("short")
        || value.contains("bear")
    {
        NormalizedDiscordDirection::Bearish
    } else {
        NormalizedDiscordDirection::Neutral
    }
}

pub fn discord_embed_color_from_direction(direction: NormalizedDiscordDirection) -> u32 {
    match direction {
        NormalizedDiscordDirection::Bullish => 5_763_719,
        NormalizedDiscordDirection::Bearish => 15_548_997,
        NormalizedDiscordDirection::Neutral => 9_807_270,
    }
}

fn discord_direction_label(direction: NormalizedDiscordDirection) -> &'static str {
    match direction {
        NormalizedDiscordDirection::Bullish => "🟢 看涨 / Bid-Buy",
        NormalizedDiscordDirection::Bearish => "🔴 看跌 / Ask-Sell",
        NormalizedDiscordDirection::Neutral => "🟡 中性 / 未知",
    }
}

fn discord_direction_emoji(direction: NormalizedDiscordDirection) -> &'static str {
    match direction {
        NormalizedDiscordDirection::Bullish => "🟢",
        NormalizedDiscordDirection::Bearish => "🔴",
        NormalizedDiscordDirection::Neutral => "🟡",
    }
}

pub fn discord_payload_for_tests(signal: &DiscordNotificationRequest) -> DiscordWebhookPayload {
    discord_payload(signal)
}

#[derive(Debug)]
struct DiscordPushLimiter {
    by_key: HashMap<String, Instant>,
    cooldown_by_key: HashMap<String, Instant>,
    burst: VecDeque<Instant>,
}

impl DiscordPushLimiter {
    fn new() -> Self {
        Self {
            by_key: HashMap::new(),
            cooldown_by_key: HashMap::new(),
            burst: VecDeque::new(),
        }
    }

    fn reserve(&mut self, key: &str, cooldown_key: Option<&str>) -> Option<&'static str> {
        let now = Instant::now();
        self.prune(now);
        if self.by_key.contains_key(key) {
            return Some("DUPLICATE_PUSH_SUPPRESSED");
        }
        if let Some(cooldown_key) = cooldown_key {
            if self.cooldown_by_key.contains_key(cooldown_key) {
                return Some("COOLDOWN_SUPPRESSED");
            }
        }
        if self.burst.len() >= 5 {
            return Some("RATE_LIMITED");
        }
        self.by_key.insert(key.to_string(), now);
        if let Some(cooldown_key) = cooldown_key {
            self.cooldown_by_key.insert(cooldown_key.to_string(), now);
        }
        self.burst.push_back(now);
        None
    }

    fn release(&mut self, key: &str, cooldown_key: Option<&str>) {
        self.by_key.remove(key);
        if let Some(cooldown_key) = cooldown_key {
            self.cooldown_by_key.remove(cooldown_key);
        }
    }

    fn prune(&mut self, now: Instant) {
        self.by_key
            .retain(|_, at| now.duration_since(*at) < Duration::from_secs(60));
        let cooldown = Duration::from_secs(discord_push_cooldown_secs());
        self.cooldown_by_key
            .retain(|_, at| now.duration_since(*at) < cooldown);
        while self
            .burst
            .front()
            .is_some_and(|at| now.duration_since(*at) >= Duration::from_secs(10))
        {
            self.burst.pop_front();
        }
    }
}

fn discord_push_limiter() -> &'static Mutex<DiscordPushLimiter> {
    static LIMITER: OnceLock<Mutex<DiscordPushLimiter>> = OnceLock::new();
    LIMITER.get_or_init(|| Mutex::new(DiscordPushLimiter::new()))
}

pub fn reset_discord_push_limits_for_tests() {
    *discord_push_limiter().lock().expect("discord limiter") = DiscordPushLimiter::new();
}

pub fn reserve_discord_push_for_tests(key: &str) -> Option<&'static str> {
    discord_push_limiter()
        .lock()
        .expect("discord limiter")
        .reserve(key, None)
}

pub fn reserve_discord_push_for_tests_with_cooldown(
    key: &str,
    cooldown_key: &str,
) -> Option<&'static str> {
    discord_push_limiter()
        .lock()
        .expect("discord limiter")
        .reserve(key, Some(cooldown_key))
}

#[derive(Debug, Default)]
struct DiscordAutoPushTracker {
    seen_keys: HashSet<String>,
    statuses: HashMap<String, DiscordAlertPublicStatus>,
}

impl DiscordAutoPushTracker {
    fn mark_once(&mut self, key: &str) -> bool {
        self.seen_keys.insert(key.to_string())
    }

    fn set_status(&mut self, key: &str, status: DiscordAlertPublicStatus) {
        self.statuses.insert(key.to_string(), status);
    }

    fn status(&self, key: &str) -> Option<DiscordAlertPublicStatus> {
        self.statuses.get(key).cloned()
    }
}

fn discord_auto_push_tracker() -> &'static Mutex<DiscordAutoPushTracker> {
    static TRACKER: OnceLock<Mutex<DiscordAutoPushTracker>> = OnceLock::new();
    TRACKER.get_or_init(|| Mutex::new(DiscordAutoPushTracker::default()))
}

pub fn reset_discord_auto_push_for_tests() {
    *discord_auto_push_tracker()
        .lock()
        .expect("discord auto tracker") = DiscordAutoPushTracker::default();
}

pub fn discord_alert_status_for_key(key: &str) -> Option<DiscordAlertPublicStatus> {
    discord_auto_push_tracker()
        .lock()
        .expect("discord auto tracker")
        .status(key)
}

fn set_discord_alert_status(key: &str, status: DiscordAlertPublicStatus) {
    discord_auto_push_tracker()
        .lock()
        .expect("discord auto tracker")
        .set_status(key, status);
}

fn record_discord_alert_status(
    body: &DiscordNotificationRequest,
    decision: &AlertGateDecision,
    auto_sent: bool,
    sent_at: Option<String>,
) {
    let Some(key) = push_dedupe_key(body).or(body.id.clone()) else {
        return;
    };
    let mut status = status_from_decision(decision, auto_sent, sent_at);
    if !auto_sent && status.sent_at.is_some() {
        status.manual_sent_at = status.sent_at.take();
        status.last_decision = "sent".to_string();
        status.reason = "manual_sent".to_string();
    }
    set_discord_alert_status(&key, status);
}

fn status_from_decision(
    decision: &AlertGateDecision,
    auto_sent: bool,
    sent_at: Option<String>,
) -> DiscordAlertPublicStatus {
    DiscordAlertPublicStatus {
        auto_eligible: decision.allowed,
        auto_sent,
        last_decision: if auto_sent {
            "sent".to_string()
        } else {
            alert_status_from_reason(decision.allowed, decision.reason).to_string()
        },
        reason: decision.reason.to_string(),
        sent_at,
        manual_sent_at: None,
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

fn now_rfc3339() -> String {
    chrono::Utc::now().to_rfc3339_opts(chrono::SecondsFormat::Secs, true)
}

#[cfg(test)]
mod tests {
    use std::sync::{Mutex, OnceLock};

    use super::{
        discord_payload, evaluate_discord_alert_gate, validate_discord_webhook_url,
        DiscordAlertMode, DiscordNotificationRequest, DiscordPushLimiter,
    };

    fn env_lock() -> &'static Mutex<()> {
        static LOCK: OnceLock<Mutex<()>> = OnceLock::new();
        LOCK.get_or_init(|| Mutex::new(()))
    }

    #[test]
    fn discord_webhook_validation_only_allows_discord_https_webhooks() {
        let discord_path = "api/webhooks";
        assert!(validate_discord_webhook_url(&format!(
            "https://discord.com/{discord_path}/1/token"
        ))
        .is_ok());
        assert!(validate_discord_webhook_url(&format!(
            "https://discordapp.com/{discord_path}/1/token"
        ))
        .is_ok());
        assert!(validate_discord_webhook_url(&format!(
            "http://discord.com/{discord_path}/1/token"
        ))
        .is_err());
        assert!(
            validate_discord_webhook_url(&format!("https://127.0.0.1/{discord_path}/1/token"))
                .is_err()
        );
        assert!(validate_discord_webhook_url(&format!(
            "https://example.com/{discord_path}/1/token"
        ))
        .is_err());
        assert!(validate_discord_webhook_url("https://discord.com/not-webhooks/1/token").is_err());
    }

    #[test]
    fn alert_gate_rejects_low_score_or_low_data_quality() {
        let _guard = env_lock().lock().expect("env lock");
        std::env::set_var("ALERT_MIN_SCORE", "80");
        std::env::set_var("ALERT_MIN_DATA_QUALITY", "70");
        std::env::set_var("DRY_RUN", "false");
        std::env::set_var(
            "DISCORD_WEBHOOK_URL",
            "https://discord.com/api/webhooks/test-id/test-token",
        );

        assert!(
            evaluate_discord_alert_gate(&request(Some(80), Some(70.0)), DiscordAlertMode::Manual)
                .allowed
        );
        assert_eq!(
            evaluate_discord_alert_gate(&request(Some(79), Some(90.0)), DiscordAlertMode::Manual)
                .reason,
            "score_below_threshold"
        );
        assert_eq!(
            evaluate_discord_alert_gate(&request(Some(90), Some(69.0)), DiscordAlertMode::Manual)
                .reason,
            "data_quality_below_threshold"
        );
        assert_eq!(
            evaluate_discord_alert_gate(&request(None, Some(90.0)), DiscordAlertMode::Manual)
                .reason,
            "score_below_threshold"
        );

        let mut medium = request(Some(90), Some(90.0));
        medium.level = Some("medium".to_string());
        assert_eq!(
            evaluate_discord_alert_gate(&medium, DiscordAlertMode::Manual).reason,
            "non_high_risk"
        );

        std::env::remove_var("ALERT_MIN_SCORE");
        std::env::remove_var("ALERT_MIN_DATA_QUALITY");
        std::env::remove_var("DRY_RUN");
        std::env::remove_var("DISCORD_WEBHOOK_URL");
    }

    #[test]
    fn discord_proxy_payload_uses_final_result_with_score_and_data_quality() {
        let mut request = request(Some(92), Some(90.0));
        request.signal_id = Some("sig_final".to_string());
        request.exchange = Some("Binance".to_string());
        request.symbol = Some("BTC-PERP".to_string());
        request.signal_type = Some("SpoofingCandidate".to_string());
        request.side = Some("Ask/Sell".to_string());
        request.reason = Some("卖方流动性移除，潜在下行压力".to_string());
        request.add_qty = Some(1_000.0);
        request.cancel_qty = Some(980.0);

        let payload = discord_payload(&request);
        let text = serde_json::to_string(&payload).expect("payload json");

        assert!(text.contains("最终结果"));
        assert!(text.contains("风险评分"));
        assert!(text.contains("数据质量"));
        assert!(text.contains("卖方流动性移除，潜在下行压力"));
        assert!(!text.contains("盘口证据"));
        assert!(!text.contains("撤后 Markout"));
        assert!(!text.contains("1,000"));
    }

    #[test]
    fn discord_test_payload_is_marked_and_does_not_include_signal_id() {
        let mut request = request(Some(92), Some(90.0));
        request.test = Some(true);

        let payload = discord_payload(&request);
        let text = serde_json::to_string(&payload).expect("payload json");

        assert!(text.contains("TEST MESSAGE"));
        assert!(text.contains("测试消息"));
        assert!(!text.contains("sig_001"));
    }

    #[test]
    fn alert_http_timeout_uses_env_or_safe_default() {
        let _guard = env_lock().lock().expect("env lock");
        std::env::remove_var("ALERT_HTTP_TIMEOUT_SECS");
        assert_eq!(super::alert_http_timeout_secs(), 5);

        std::env::set_var("ALERT_HTTP_TIMEOUT_SECS", "7");
        assert_eq!(super::alert_http_timeout_secs(), 7);

        std::env::remove_var("ALERT_HTTP_TIMEOUT_SECS");
    }

    #[test]
    fn discord_limiter_suppresses_same_symbol_direction_cooldown() {
        let mut limiter = DiscordPushLimiter::new();

        assert_eq!(
            limiter.reserve("sig_001", Some("BTC-PERP:spoofing:short")),
            None
        );
        assert_eq!(
            limiter.reserve("sig_002", Some("BTC-PERP:spoofing:short")),
            Some("COOLDOWN_SUPPRESSED")
        );
    }

    fn request(score: Option<u8>, data_quality: Option<f64>) -> DiscordNotificationRequest {
        DiscordNotificationRequest {
            signal_id: Some("sig_001".to_string()),
            id: None,
            dedupe_key: None,
            exchange: None,
            symbol: None,
            signal_type: None,
            level: Some("high".to_string()),
            side: None,
            score,
            data_quality,
            reason: None,
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
            tof_metrics: None,
            tof_score: None,
            candidate_type: None,
            explain_tags: None,
            direction_confidence: None,
            perp_tof_metrics: None,
            perp_score: None,
            perp_candidate_type: None,
            final_candidate_type: None,
            metrics_direction: None,
            advanced_tof_metrics: None,
            advanced_score: None,
            advanced_candidate_type: None,
            test: None,
        }
    }
}
