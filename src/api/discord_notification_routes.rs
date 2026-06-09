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
use crate::runtime::score_config::score_runtime_config;
use crate::runtime::tof_metrics::TofMetrics;
use axum::{extract::State, http::StatusCode, response::IntoResponse, Json};
use reqwest::Url;
use serde::{Deserialize, Serialize};

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DiscordNotificationRequest {
    pub alert_family: Option<String>,
    pub signal_id: Option<String>,
    pub id: Option<String>,
    pub dedupe_key: Option<String>,
    pub exchange: Option<String>,
    pub symbol: Option<String>,
    pub signal_type: Option<String>,
    pub level: Option<String>,
    pub side: Option<String>,
    pub score: Option<u8>,
    pub confidence: Option<f64>,
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
    pub main_force_score: Option<u8>,
    pub extreme_impact_score: Option<u8>,
    pub structure_bias: Option<i16>,
    pub market_structure_confidence: Option<f64>,
    pub market_structure_data_quality: Option<f64>,
    pub market_structure_severity: Option<String>,
    pub regime_type: Option<String>,
    pub spot_score: Option<u8>,
    pub contract_score: Option<u8>,
    pub cross_confirm_score: Option<u8>,
    pub main_force_confirmed: Option<bool>,
    pub signal_agreement: Option<u8>,
    pub source_coverage: Option<u8>,
    pub oi_score: Option<u8>,
    pub liquidation_score: Option<u8>,
    pub test: Option<bool>,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DiscordNotificationResponse {
    pub ok: bool,
    pub configured: bool,
    pub reason: &'static str,
    pub min_score: u8,
    pub min_confidence: f64,
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
    pub confidence: f64,
    pub data_quality: f64,
    pub min_score: u8,
    pub min_confidence: f64,
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
    let gate = AlertGate::from_env(alert_family(&body));
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
                min_confidence: gate.min_confidence,
                min_data_quality: gate.min_data_quality,
                sent: false,
                read_only: true,
                execution_enabled: false,
            }),
        )
            .into_response();
    }

    let Some(webhook_url) = discord_webhook_url(&body) else {
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
                min_confidence: gate.min_confidence,
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
                min_confidence: gate.min_confidence,
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
            .reserve(
                key,
                cooldown_key.as_deref(),
                Some(cooldown_duration_for_signal(&body)),
            )
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
                    min_confidence: gate.min_confidence,
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
                    min_confidence: gate.min_confidence,
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
                    min_confidence: gate.min_confidence,
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
                    min_confidence: gate.min_confidence,
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
            "score_below_threshold"
                | "confidence_below_threshold"
                | "data_quality_below_threshold"
                | "non_high_risk"
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

    let Some(webhook_url) = discord_webhook_url(&body) else {
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
            .reserve(
                key,
                cooldown_key.as_deref(),
                Some(cooldown_duration_for_signal(&body)),
            )
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
    min_extreme_score: u8,
    min_confidence: f64,
    min_data_quality: f64,
}

impl AlertGate {
    fn from_env(family: &str) -> Self {
        let defaults = Self::defaults_for_family(family);
        Self {
            min_score: read_u8_env(
                family_specific_key(family, "ALERT_MIN_SCORE"),
                "ALERT_MIN_SCORE",
            )
            .unwrap_or(defaults.min_score),
            min_extreme_score: read_u8_env(
                family_specific_key(family, "EXTREME_MIN_SCORE"),
                "ALERT_MIN_SCORE",
            )
            .unwrap_or(defaults.min_extreme_score),
            min_confidence: read_f64_env(
                family_specific_key(family, "ALERT_MIN_CONFIDENCE"),
                "ALERT_MIN_CONFIDENCE",
            )
            .unwrap_or(defaults.min_confidence),
            min_data_quality: read_f64_env(
                family_specific_key(family, "ALERT_MIN_DATA_QUALITY"),
                "ALERT_MIN_DATA_QUALITY",
            )
            .unwrap_or(defaults.min_data_quality),
        }
    }

    fn defaults_for_family(family: &str) -> Self {
        let config = score_runtime_config();
        match family {
            "MARKET_STRUCTURE" => Self {
                min_score: config.market_structure.discord.min_main_force_score,
                min_extreme_score: config.market_structure.discord.min_extreme_impact_score,
                min_confidence: config.market_structure.discord.min_confidence,
                min_data_quality: config.market_structure.discord.min_data_quality,
            },
            _ => Self {
                min_score: config.toxic_short.discord.min_score,
                min_extreme_score: config.toxic_short.discord.min_score,
                min_confidence: config.toxic_short.discord.min_confidence,
                min_data_quality: config.toxic_short.discord.min_data_quality,
            },
        }
    }
}

fn read_u8_env(primary: String, fallback: &str) -> Option<u8> {
    std::env::var(&primary)
        .ok()
        .and_then(|value| value.parse::<u8>().ok())
        .or_else(|| {
            std::env::var(fallback)
                .ok()
                .and_then(|value| value.parse::<u8>().ok())
        })
}

fn read_f64_env(primary: String, fallback: &str) -> Option<f64> {
    std::env::var(&primary)
        .ok()
        .and_then(|value| value.parse::<f64>().ok())
        .or_else(|| {
            std::env::var(fallback)
                .ok()
                .and_then(|value| value.parse::<f64>().ok())
        })
}

pub fn evaluate_discord_alert_gate(
    signal: &DiscordNotificationRequest,
    mode: DiscordAlertMode,
) -> AlertGateDecision {
    let family = alert_family(signal);
    let gate = AlertGate::from_env(family);
    let auto_push_enabled = discord_auto_push_enabled(family);
    let dry_run = discord_dry_run();
    let configured = discord_webhook_url(signal).is_some();
    match family {
        "MARKET_STRUCTURE" => evaluate_market_structure_gate(
            signal,
            &gate,
            mode,
            auto_push_enabled,
            dry_run,
            configured,
        ),
        _ => evaluate_short_toxic_gate(signal, &gate, mode, auto_push_enabled, dry_run, configured),
    }
}

fn severity_allows(level: Option<&str>) -> bool {
    matches!(
        level.unwrap_or("").trim().to_ascii_lowercase().as_str(),
        "high" | "critical" | "a" | "s"
    )
}

fn discord_webhook_url(signal: &DiscordNotificationRequest) -> Option<String> {
    let family = alert_family(signal);
    std::env::var(family_specific_key(family, "DISCORD_WEBHOOK_URL"))
        .ok()
        .filter(|value| !value.trim().is_empty())
        .or_else(|| {
            std::env::var("DISCORD_WEBHOOK_URL")
                .ok()
                .filter(|value| !value.trim().is_empty())
        })
}

fn alert_family(signal: &DiscordNotificationRequest) -> &str {
    match signal
        .alert_family
        .as_deref()
        .unwrap_or("short_toxic_order")
        .trim()
        .to_ascii_lowercase()
        .as_str()
    {
        "short_toxic_order" | "short_toxic" => "SHORT_TOXIC",
        "market_structure" => "MARKET_STRUCTURE",
        other if !other.is_empty() => "SHORT_TOXIC",
        _ => "SHORT_TOXIC",
    }
}

fn family_specific_key(family: &str, suffix: &str) -> String {
    format!("{family}_{suffix}")
}

fn alert_family_label(signal: &DiscordNotificationRequest) -> &'static str {
    match alert_family(signal) {
        "SHORT_TOXIC" => "short_toxic_order",
        "MARKET_STRUCTURE" => "market_structure",
        _ => "short_toxic_order",
    }
}

fn discord_channel_name(signal: &DiscordNotificationRequest) -> Option<String> {
    let family = alert_family(signal);
    std::env::var(family_specific_key(family, "DISCORD_CHANNEL_NAME"))
        .ok()
        .filter(|value| !value.trim().is_empty())
}

fn discord_dry_run() -> bool {
    parse_bool_env("DRY_RUN", true)
}

fn discord_auto_push_enabled(family: &str) -> bool {
    let config = score_runtime_config();
    let default = match family {
        "MARKET_STRUCTURE" => config.market_structure.discord.enabled,
        _ => config.toxic_short.discord.enabled,
    };
    parse_bool_env_with_fallback(
        &family_specific_key(family, "DISCORD_AUTO_PUSH_ENABLED"),
        "DISCORD_AUTO_PUSH_ENABLED",
        default,
    )
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

fn parse_bool_env_with_fallback(primary: &str, fallback: &str, default: bool) -> bool {
    std::env::var(primary)
        .ok()
        .map(|value| {
            matches!(
                value.trim().to_ascii_lowercase().as_str(),
                "1" | "true" | "yes" | "on"
            )
        })
        .or_else(|| {
            std::env::var(fallback).ok().map(|value| {
                matches!(
                    value.trim().to_ascii_lowercase().as_str(),
                    "1" | "true" | "yes" | "on"
                )
            })
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
        "{}:{}:{}:{}",
        alert_family_label(signal),
        symbol.to_ascii_uppercase(),
        signal_type.to_ascii_lowercase(),
        side.to_ascii_lowercase()
    ))
}

fn cooldown_duration_for_signal(signal: &DiscordNotificationRequest) -> Duration {
    Duration::from_secs(default_cooldown_secs_for_family(alert_family(signal)))
}

fn default_cooldown_secs_for_family(family: &str) -> u64 {
    let config = score_runtime_config();
    let default = match family {
        "MARKET_STRUCTURE" => config.market_structure.discord.cooldown_sec,
        _ => config.toxic_short.discord.cooldown_sec,
    };
    std::env::var(family_specific_key(family, "DISCORD_COOLDOWN_SECONDS"))
        .ok()
        .and_then(|value| value.parse::<u64>().ok())
        .or_else(|| {
            std::env::var("DISCORD_PUSH_COOLDOWN_SECONDS")
                .ok()
                .and_then(|value| value.parse::<u64>().ok())
        })
        .filter(|value| *value > 0)
        .unwrap_or(default)
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
    let family = alert_family_label(body);
    state.record_scan_log(
        "info",
        "alert_gate_evaluated",
        format!(
            "{} {} alert gate evaluated: severity={} toxicScore={} confidence={:.0} quality={:.0} autoPush={} reason={}",
            body.symbol.as_deref().unwrap_or("unknown"),
            family,
            body.level.as_deref().unwrap_or("unknown"),
            decision.score,
            decision.confidence,
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
            "{prefix}: toxicScore={} below {}",
            decision.score, decision.min_score
        ),
        "confidence_below_threshold" => format!(
            "{prefix}: confidence={:.0} below {:.0}",
            decision.confidence, decision.min_confidence
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

    match alert_family(signal) {
        "MARKET_STRUCTURE" => market_structure_payload(signal),
        _ => short_toxic_payload(signal),
    }
}

fn short_toxic_payload(signal: &DiscordNotificationRequest) -> DiscordWebhookPayload {
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
    let level = signal.level.as_deref().unwrap_or("High");
    let family = alert_family_label(signal);
    let final_result = signal
        .reason
        .as_deref()
        .filter(|value| !value.trim().is_empty())
        .unwrap_or("无法判断方向");

    let mut fields = vec![
        DiscordEmbedField {
            name: "类型".to_string(),
            value: short_toxic_type_label(signal, direction).to_string(),
            inline: true,
        },
        DiscordEmbedField {
            name: "短线压力".to_string(),
            value: short_pressure_label(direction).to_string(),
            inline: true,
        },
        DiscordEmbedField {
            name: "窗口".to_string(),
            value: "1s / 5s / 15s / 60s".to_string(),
            inline: true,
        },
        DiscordEmbedField {
            name: "毒性评分".to_string(),
            value: signal
                .score
                .map(|value| format!("{value}/100"))
                .unwrap_or_else(|| "N/A".to_string()),
            inline: true,
        },
        DiscordEmbedField {
            name: "置信度".to_string(),
            value: signal
                .confidence
                .or(signal.direction_confidence)
                .map(|value| format!("{value:.0}/100"))
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
        DiscordEmbedField {
            name: "通知链路".to_string(),
            value: "短线有毒订单".to_string(),
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
    fields.push(DiscordEmbedField {
        name: "原因".to_string(),
        value: short_toxic_reasons(signal, direction),
        inline: false,
    });
    fields.push(DiscordEmbedField {
        name: "判断".to_string(),
        value: format!(
            "{}；短线扫盘 / 插针风险升高，不代表中长线趋势。",
            final_result.trim_end_matches('。')
        ),
        inline: false,
    });
    fields.push(DiscordEmbedField {
        name: "说明".to_string(),
        value: "短线有毒订单 Candidate only，基于公开盘口 / L2 / 成交数据推断；只做提醒，不代表中长线趋势，不执行下单、拦截、封禁或资金操作。".to_string(),
        inline: false,
    });

    DiscordWebhookPayload {
        content: None,
        embeds: vec![DiscordEmbed {
            title: format!(
                "{} {symbol} 短线有毒订单 {}",
                discord_direction_emoji(direction),
                level
            ),
            description: format!("{exchange} / {symbol} · {event_type} · {side} · {family}"),
            color: discord_embed_color_from_direction(direction),
            fields,
            footer: Some(DiscordEmbedFooter {
                text: format!(
                    "Candidate only | Family: {}{} | Signal: {}",
                    family,
                    discord_channel_name(signal)
                        .map(|name| format!(" | Channel: {name}"))
                        .unwrap_or_default(),
                    signal.signal_id.as_deref().unwrap_or("N/A")
                ),
            }),
            timestamp: signal.time.clone(),
        }],
    }
}

fn market_structure_payload(signal: &DiscordNotificationRequest) -> DiscordWebhookPayload {
    let symbol = signal
        .symbol
        .clone()
        .unwrap_or_else(|| "Unknown".to_string());
    let event_type = signal
        .signal_type
        .clone()
        .unwrap_or_else(|| "market_structure".to_string());
    let exchange = signal.exchange.as_deref().unwrap_or("Runtime");
    let family = alert_family_label(signal);
    let main_force_score = market_structure_main_force_score(signal);
    let extreme_impact_score = market_structure_extreme_score(signal);
    let confidence = market_structure_confidence(signal);
    let data_quality = market_structure_data_quality(signal);
    let structure_bias = signal.structure_bias.unwrap_or(0);
    let regime_type = signal.regime_type.as_deref().unwrap_or("unclear");
    let regime_label = market_structure_regime_label(regime_type);
    let severity = market_structure_severity_label(signal);
    let direction = market_structure_direction_from_bias(structure_bias);
    let extreme_template = matches!(
        market_structure_trigger(signal, &AlertGate::from_env("MARKET_STRUCTURE")),
        Some(MarketStructureTrigger::ExtremeImpact)
    ) && (main_force_score
        < AlertGate::from_env("MARKET_STRUCTURE").min_score
        || signal.main_force_confirmed == Some(false));
    let final_result = signal
        .reason
        .as_deref()
        .filter(|value| !value.trim().is_empty())
        .unwrap_or("暂无进一步判断");

    let mut fields = vec![
        DiscordEmbedField {
            name: "类型".to_string(),
            value: regime_label.to_string(),
            inline: true,
        },
        DiscordEmbedField {
            name: if extreme_template {
                "极端冲击".to_string()
            } else {
                "主力评分".to_string()
            },
            value: if extreme_template {
                format!("{extreme_impact_score}/100")
            } else {
                format!("{main_force_score}/100")
            },
            inline: true,
        },
        DiscordEmbedField {
            name: "结构方向".to_string(),
            value: format!(
                "{} {:+}",
                structure_bias_label(structure_bias),
                structure_bias
            ),
            inline: true,
        },
        DiscordEmbedField {
            name: "极端冲击".to_string(),
            value: format!("{extreme_impact_score}/100"),
            inline: true,
        },
        DiscordEmbedField {
            name: "置信度".to_string(),
            value: format!("{confidence:.0}/100"),
            inline: true,
        },
        DiscordEmbedField {
            name: "数据质量".to_string(),
            value: format!("{data_quality:.0}/100"),
            inline: true,
        },
        DiscordEmbedField {
            name: "现货评分".to_string(),
            value: signal
                .spot_score
                .map(|value| format!("{value}/100"))
                .unwrap_or_else(|| "N/A".to_string()),
            inline: true,
        },
        DiscordEmbedField {
            name: "合约评分".to_string(),
            value: signal
                .contract_score
                .map(|value| format!("{value}/100"))
                .unwrap_or_else(|| "N/A".to_string()),
            inline: true,
        },
        DiscordEmbedField {
            name: "现货合约确认".to_string(),
            value: signal
                .cross_confirm_score
                .map(|value| format!("{value}/100"))
                .unwrap_or_else(|| "N/A".to_string()),
            inline: true,
        },
        DiscordEmbedField {
            name: "通知链路".to_string(),
            value: "主力结构异动".to_string(),
            inline: true,
        },
    ];
    fields.push(DiscordEmbedField {
        name: "主要原因".to_string(),
        value: market_structure_reasons(signal, direction, regime_type, extreme_template),
        inline: false,
    });
    fields.push(DiscordEmbedField {
        name: "判断".to_string(),
        value: market_structure_judgment(regime_type, final_result, extreme_template),
        inline: false,
    });
    fields.push(DiscordEmbedField {
        name: "说明".to_string(),
        value: "主力结构 Candidate only，基于现货、合约、OI、价格响应和公开成交上下文推断；只做提醒，不执行下单、拦截、封禁或资金操作。".to_string(),
        inline: false,
    });

    DiscordWebhookPayload {
        content: None,
        embeds: vec![DiscordEmbed {
            title: if extreme_template {
                format!("⚠️ {symbol} 极端行情冲击")
            } else {
                format!("🚨 {symbol} 主力结构异动 {severity}")
            },
            description: format!(
                "{exchange} / {symbol} · {event_type} · {} · {family}",
                regime_label
            ),
            color: discord_embed_color_from_direction(direction),
            fields,
            footer: Some(DiscordEmbedFooter {
                text: format!(
                    "Candidate only | Family: {}{} | Signal: {}",
                    family,
                    discord_channel_name(signal)
                        .map(|name| format!(" | Channel: {name}"))
                        .unwrap_or_default(),
                    signal.signal_id.as_deref().unwrap_or("N/A")
                ),
            }),
            timestamp: signal.time.clone(),
        }],
    }
}

fn short_toxic_type_label(
    signal: &DiscordNotificationRequest,
    direction: NormalizedDiscordDirection,
) -> &'static str {
    let value = signal
        .candidate_type
        .as_deref()
        .or(signal.signal_type.as_deref())
        .unwrap_or("")
        .to_ascii_lowercase();
    if value.contains("spoof") {
        "虚假挂单 / 撤单诱导"
    } else if value.contains("liquidity") && value.contains("pull") {
        "短线流动性撤离"
    } else if value.contains("thin") || value.contains("gap") {
        "短线流动性缺口"
    } else if value.contains("stop") {
        "扫损 / Stop Hunt"
    } else if value.contains("breakout") {
        "假突破"
    } else {
        match direction {
            NormalizedDiscordDirection::Bullish => "主动买入扫盘",
            NormalizedDiscordDirection::Bearish => "主动卖出扫盘",
            NormalizedDiscordDirection::Neutral => "短线有毒流",
        }
    }
}

fn short_pressure_label(direction: NormalizedDiscordDirection) -> &'static str {
    match direction {
        NormalizedDiscordDirection::Bullish => "偏多",
        NormalizedDiscordDirection::Bearish => "偏空",
        NormalizedDiscordDirection::Neutral => "中性 / 不明确",
    }
}

fn short_toxic_reasons(
    signal: &DiscordNotificationRequest,
    direction: NormalizedDiscordDirection,
) -> String {
    let mut reasons = Vec::new();
    match direction {
        NormalizedDiscordDirection::Bullish => reasons.push("主动买入扫穿近端卖盘".to_string()),
        NormalizedDiscordDirection::Bearish => reasons.push("主动卖出扫穿近端买盘".to_string()),
        NormalizedDiscordDirection::Neutral => reasons.push("短线成交流与盘口结构异常".to_string()),
    }
    if let Some(metrics) = signal.tof_metrics.as_ref() {
        if metrics.depth_withdrawal_score >= 60.0 {
            reasons.push("近端盘口深度快速消失".to_string());
        }
        if metrics.spread_widening_score >= 55.0 || metrics.spread_bps >= 5.0 {
            reasons.push(format!("价差短线扩大至 {:.1}bps", metrics.spread_bps));
        }
        if metrics.trade_imbalance.abs() >= 0.35 {
            reasons.push(format!("主动成交方向不平衡 {:.2}", metrics.trade_imbalance));
        }
    }
    if let Some(price_impact_bps) = signal.price_impact_bps {
        reasons.push(format!("价格短线冲击 {:.2}bps", price_impact_bps));
    }
    if let Some(tags) = signal.explain_tags.as_ref() {
        for tag in tags.iter().take(2) {
            reasons.push(format!("解释标签：{tag}"));
        }
    }
    reasons
        .into_iter()
        .take(5)
        .map(|reason| format!("- {reason}"))
        .collect::<Vec<_>>()
        .join("\n")
}

fn market_structure_regime_label(regime_type: &str) -> &'static str {
    match regime_type {
        "main_force_long_build" => "主力建多",
        "main_force_short_build" => "主力建空",
        "contract_flow_shock" => "合约冲击",
        "spot_accumulation" => "现货吸筹",
        "spot_distribution" => "现货派发",
        "contract_short_squeeze" => "空头挤压",
        "long_liquidation_cascade" => "多头清算瀑布",
        "downside_absorption" => "下方吸收",
        "upside_resistance" => "上方压制",
        "range_rotation" => "高换手震荡",
        _ => "结构未明",
    }
}

fn structure_bias_label(structure_bias: i16) -> &'static str {
    if structure_bias >= 15 {
        "偏多"
    } else if structure_bias <= -15 {
        "偏空"
    } else {
        "中性"
    }
}

fn market_structure_severity_label(signal: &DiscordNotificationRequest) -> String {
    if let Some(value) = signal
        .market_structure_severity
        .as_deref()
        .filter(|value| !value.trim().is_empty())
    {
        return value.to_string();
    }
    match market_structure_main_force_score(signal) {
        90..=u8::MAX => "Extreme".to_string(),
        75..=89 => "Major".to_string(),
        60..=74 => "Confirmed".to_string(),
        40..=59 => "Watch".to_string(),
        _ => "Calm".to_string(),
    }
}

fn market_structure_direction_from_bias(structure_bias: i16) -> NormalizedDiscordDirection {
    if structure_bias >= 15 {
        NormalizedDiscordDirection::Bullish
    } else if structure_bias <= -15 {
        NormalizedDiscordDirection::Bearish
    } else {
        NormalizedDiscordDirection::Neutral
    }
}

fn market_structure_reasons(
    signal: &DiscordNotificationRequest,
    direction: NormalizedDiscordDirection,
    regime_type: &str,
    extreme_template: bool,
) -> String {
    let mut reasons = Vec::new();
    if signal.contract_score.unwrap_or(0) >= 70 {
        reasons.push(match direction {
            NormalizedDiscordDirection::Bullish => "合约主动买入显著放大".to_string(),
            NormalizedDiscordDirection::Bearish => "合约主动卖出爆发".to_string(),
            NormalizedDiscordDirection::Neutral => "合约主动成交流异常放大".to_string(),
        });
    }
    if signal.oi_score.unwrap_or(0) >= 70 {
        reasons.push(match direction {
            NormalizedDiscordDirection::Bullish => "OI 同步上升，偏新多开仓".to_string(),
            NormalizedDiscordDirection::Bearish => "OI 同步上升，偏新空开仓".to_string(),
            NormalizedDiscordDirection::Neutral => "OI 变化明显，结构进入再定价".to_string(),
        });
    } else if extreme_template {
        reasons.push("OI 快速下降".to_string());
    }
    if signal.spot_score.unwrap_or(0) >= 60 {
        reasons.push(match regime_type {
            "downside_absorption" => "现货买盘承接明显".to_string(),
            "upside_resistance" => "现货卖盘压制明显".to_string(),
            _ => match direction {
                NormalizedDiscordDirection::Bullish => "现货主动买入跟随".to_string(),
                NormalizedDiscordDirection::Bearish => "现货主动卖出跟随".to_string(),
                NormalizedDiscordDirection::Neutral => "现货成交方向开始配合".to_string(),
            },
        });
    } else if extreme_template {
        reasons.push("现货卖出确认不足".to_string());
    }
    if signal.price_impact_bps.unwrap_or_default().abs() > 0.0 {
        reasons.push("价格出现明显短线冲击".to_string());
    } else if regime_type == "downside_absorption" {
        reasons.push("价格回调未破，出现下方承接".to_string());
    } else if regime_type == "upside_resistance" {
        reasons.push("价格上冲未成，出现上方压制".to_string());
    }
    if signal.liquidation_score.unwrap_or(0) >= 70 && extreme_template {
        reasons.push("多头清算显著增加".to_string());
    }
    reasons
        .into_iter()
        .take(4)
        .map(|reason| format!("- {reason}"))
        .collect::<Vec<_>>()
        .join("\n")
}

fn market_structure_judgment(
    regime_type: &str,
    final_result: &str,
    extreme_template: bool,
) -> String {
    if extreme_template {
        let bearish = matches!(
            regime_type,
            "long_liquidation_cascade" | "main_force_short_build"
        );
        return if bearish {
            "这是极端下跌冲击，但暂不确认是主力建空。".to_string()
        } else {
            format!(
                "{}；这是极端行情冲击，但暂不确认是持续主力结构异动。",
                final_result.trim_end_matches('。')
            )
        };
    }
    match regime_type {
        "main_force_long_build" => "高概率主力建多，不是单纯清算推动。".to_string(),
        "main_force_short_build" => "高概率主力建空，不是单纯反弹回落。".to_string(),
        "spot_accumulation" => "现货吸筹迹象增强，偏向中长线承接。".to_string(),
        "spot_distribution" => "现货派发迹象增强，追多质量下降。".to_string(),
        "contract_flow_shock" => "当前更像合约侧冲击，现货/OI/价格尚未形成主力确认。".to_string(),
        "downside_absorption" => "下方承接明确，但方向还没有完全展开。".to_string(),
        "upside_resistance" => "上方压制明确，但趋势尚未完全下破。".to_string(),
        _ => format!(
            "{}；主力结构异动增强，建议继续观察现货和合约是否继续同向确认。",
            final_result.trim_end_matches('。')
        ),
    }
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

pub fn preferred_discord_alert_family(signal: &DiscordNotificationRequest) -> &'static str {
    if market_structure_trigger(signal, &AlertGate::from_env("MARKET_STRUCTURE")).is_some() {
        "market_structure"
    } else {
        "short_toxic_order"
    }
}

fn evaluate_short_toxic_gate(
    signal: &DiscordNotificationRequest,
    gate: &AlertGate,
    mode: DiscordAlertMode,
    auto_push_enabled: bool,
    dry_run: bool,
    configured: bool,
) -> AlertGateDecision {
    let severity_allowed = severity_allows(signal.level.as_deref());
    let score = signal.score.unwrap_or(0);
    let confidence = signal
        .confidence
        .or(signal.direction_confidence)
        .unwrap_or(0.0);
    let data_quality = signal.data_quality.unwrap_or(0.0);
    let mut decision = AlertGateDecision {
        allowed: true,
        reason: "passed",
        severity_allowed,
        score,
        confidence,
        data_quality,
        min_score: gate.min_score,
        min_confidence: gate.min_confidence,
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
    } else if confidence < gate.min_confidence {
        decision.allowed = false;
        decision.reason = "confidence_below_threshold";
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

fn evaluate_market_structure_gate(
    signal: &DiscordNotificationRequest,
    gate: &AlertGate,
    mode: DiscordAlertMode,
    auto_push_enabled: bool,
    dry_run: bool,
    configured: bool,
) -> AlertGateDecision {
    let confidence = market_structure_confidence(signal);
    let data_quality = market_structure_data_quality(signal);
    let trigger = market_structure_trigger(signal, gate);
    let score = match trigger {
        Some(MarketStructureTrigger::MainForce) => market_structure_main_force_score(signal),
        Some(MarketStructureTrigger::ExtremeImpact) => market_structure_extreme_score(signal),
        None => {
            market_structure_main_force_score(signal).max(market_structure_extreme_score(signal))
        }
    };
    let mut decision = AlertGateDecision {
        allowed: true,
        reason: "passed",
        severity_allowed: true,
        score,
        confidence,
        data_quality,
        min_score: gate.min_score,
        min_confidence: gate.min_confidence,
        min_data_quality: gate.min_data_quality,
        auto_push_enabled,
        dry_run,
        configured,
    };

    if matches!(mode, DiscordAlertMode::Auto) && !auto_push_enabled {
        decision.allowed = false;
        decision.reason = "auto_disabled";
    } else if trigger.is_none() {
        decision.allowed = false;
        decision.reason = market_structure_rejection_reason(signal, gate);
    } else if dry_run {
        decision.allowed = false;
        decision.reason = "dry_run";
    } else if !configured {
        decision.allowed = false;
        decision.reason = "webhook_missing";
    }

    decision
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum MarketStructureTrigger {
    MainForce,
    ExtremeImpact,
}

fn market_structure_trigger(
    signal: &DiscordNotificationRequest,
    gate: &AlertGate,
) -> Option<MarketStructureTrigger> {
    let data_quality = market_structure_data_quality(signal);
    if data_quality < gate.min_data_quality {
        return None;
    }
    if market_structure_main_force_score(signal) >= gate.min_score
        && market_structure_confidence(signal) >= gate.min_confidence
    {
        return Some(MarketStructureTrigger::MainForce);
    }
    if market_structure_extreme_score(signal) >= gate.min_extreme_score {
        return Some(MarketStructureTrigger::ExtremeImpact);
    }
    None
}

fn market_structure_rejection_reason(
    signal: &DiscordNotificationRequest,
    gate: &AlertGate,
) -> &'static str {
    let data_quality = market_structure_data_quality(signal);
    if data_quality < gate.min_data_quality {
        return "data_quality_below_threshold";
    }
    let main_force_score = market_structure_main_force_score(signal);
    let confidence = market_structure_confidence(signal);
    let extreme = market_structure_extreme_score(signal);
    if main_force_score >= gate.min_score && confidence < gate.min_confidence {
        return "confidence_below_threshold";
    }
    if main_force_score < gate.min_score && extreme < gate.min_extreme_score {
        return "score_below_threshold";
    }
    "score_below_threshold"
}

fn market_structure_main_force_score(signal: &DiscordNotificationRequest) -> u8 {
    signal.main_force_score.unwrap_or(0)
}

fn market_structure_extreme_score(signal: &DiscordNotificationRequest) -> u8 {
    signal.extreme_impact_score.unwrap_or(0)
}

fn market_structure_confidence(signal: &DiscordNotificationRequest) -> f64 {
    signal
        .market_structure_confidence
        .or(signal.confidence)
        .unwrap_or(0.0)
}

fn market_structure_data_quality(signal: &DiscordNotificationRequest) -> f64 {
    signal
        .market_structure_data_quality
        .or(signal.data_quality)
        .unwrap_or(0.0)
}

#[derive(Debug)]
struct DiscordPushLimiter {
    by_key: HashMap<String, Instant>,
    cooldown_by_key: HashMap<String, CooldownEntry>,
    burst: VecDeque<Instant>,
}

#[derive(Debug, Clone, Copy)]
struct CooldownEntry {
    at: Instant,
    duration: Duration,
}

impl DiscordPushLimiter {
    fn new() -> Self {
        Self {
            by_key: HashMap::new(),
            cooldown_by_key: HashMap::new(),
            burst: VecDeque::new(),
        }
    }

    fn reserve(
        &mut self,
        key: &str,
        cooldown_key: Option<&str>,
        cooldown_duration: Option<Duration>,
    ) -> Option<&'static str> {
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
            self.cooldown_by_key.insert(
                cooldown_key.to_string(),
                CooldownEntry {
                    at: now,
                    duration: cooldown_duration.unwrap_or_else(|| Duration::from_secs(60)),
                },
            );
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
        self.cooldown_by_key
            .retain(|_, entry| now.duration_since(entry.at) < entry.duration);
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
        .reserve(key, None, None)
}

pub fn reserve_discord_push_for_tests_with_cooldown(
    key: &str,
    cooldown_key: &str,
    cooldown_secs: u64,
) -> Option<&'static str> {
    discord_push_limiter()
        .lock()
        .expect("discord limiter")
        .reserve(
            key,
            Some(cooldown_key),
            Some(Duration::from_secs(cooldown_secs)),
        )
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
        "score_below_threshold"
            | "confidence_below_threshold"
            | "data_quality_below_threshold"
            | "non_high_risk"
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
        std::env::set_var("ALERT_MIN_SCORE", "85");
        std::env::set_var("ALERT_MIN_CONFIDENCE", "70");
        std::env::set_var("ALERT_MIN_DATA_QUALITY", "70");
        std::env::set_var("DRY_RUN", "false");
        std::env::set_var(
            "DISCORD_WEBHOOK_URL",
            "https://discord.com/api/webhooks/test-id/test-token",
        );

        assert!(
            evaluate_discord_alert_gate(&request(Some(85), Some(70.0)), DiscordAlertMode::Manual)
                .allowed
        );
        assert_eq!(
            evaluate_discord_alert_gate(&request(Some(84), Some(90.0)), DiscordAlertMode::Manual)
                .reason,
            "score_below_threshold"
        );
        let mut low_confidence = request(Some(90), Some(90.0));
        low_confidence.confidence = Some(69.0);
        assert_eq!(
            evaluate_discord_alert_gate(&low_confidence, DiscordAlertMode::Manual).reason,
            "confidence_below_threshold"
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
        std::env::remove_var("ALERT_MIN_CONFIDENCE");
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
        assert!(text.contains("短线有毒订单"));
        assert!(text.contains("毒性评分"));
        assert!(text.contains("置信度"));
        assert!(text.contains("数据质量"));
        assert!(text.contains("不代表中长线趋势"));
        assert!(text.contains("卖方流动性移除，潜在下行压力"));
        assert!(!text.contains("主力介入"));
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
            limiter.reserve(
                "sig_001",
                Some("BTC-PERP:spoofing:short"),
                Some(std::time::Duration::from_secs(60)),
            ),
            None
        );
        assert_eq!(
            limiter.reserve(
                "sig_002",
                Some("BTC-PERP:spoofing:short"),
                Some(std::time::Duration::from_secs(60)),
            ),
            Some("COOLDOWN_SUPPRESSED")
        );
    }

    fn request(score: Option<u8>, data_quality: Option<f64>) -> DiscordNotificationRequest {
        DiscordNotificationRequest {
            alert_family: Some("short_toxic_order".to_string()),
            signal_id: Some("sig_001".to_string()),
            id: None,
            dedupe_key: None,
            exchange: None,
            symbol: None,
            signal_type: None,
            level: Some("high".to_string()),
            side: None,
            score,
            confidence: Some(90.0),
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
            main_force_score: None,
            extreme_impact_score: None,
            structure_bias: None,
            market_structure_confidence: None,
            market_structure_data_quality: None,
            market_structure_severity: None,
            regime_type: None,
            spot_score: None,
            contract_score: None,
            cross_confirm_score: None,
            main_force_confirmed: None,
            signal_agreement: None,
            source_coverage: None,
            oi_score: None,
            liquidation_score: None,
            test: None,
        }
    }
}
