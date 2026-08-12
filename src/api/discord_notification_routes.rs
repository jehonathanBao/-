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
use serde::{de::DeserializeOwned, Deserialize, Deserializer, Serialize};

fn deserialize_available_metric<'de, D, T>(deserializer: D) -> Result<Option<T>, D::Error>
where
    D: Deserializer<'de>,
    T: DeserializeOwned,
{
    let Some(value) = Option::<serde_json::Value>::deserialize(deserializer)? else {
        return Ok(None);
    };
    if value
        .get("lineage")
        .and_then(|lineage| lineage.get("available"))
        .and_then(serde_json::Value::as_bool)
        == Some(false)
    {
        return Ok(None);
    }
    // Metrics in this request are display hints only and are always cleared
    // before authoritative hydration. Wire DTOs may legitimately contain
    // group-available metrics with unavailable (null) sub-fields, while the
    // internal calculation model uses concrete numbers. Treat any shape that
    // cannot round-trip into the internal model as absent instead of rejecting
    // the entire canonical-signal request with a 422.
    Ok(serde_json::from_value(value).ok())
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DiscordNotificationRequest {
    #[serde(default, skip_deserializing)]
    pub server_evidence_verified: bool,
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
    pub impact_level: Option<String>,
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
    #[serde(default, deserialize_with = "deserialize_available_metric")]
    pub tof_metrics: Option<TofMetrics>,
    pub tof_score: Option<f64>,
    pub candidate_type: Option<String>,
    pub explain_tags: Option<Vec<String>>,
    pub direction_confidence: Option<f64>,
    #[serde(default, deserialize_with = "deserialize_available_metric")]
    pub perp_tof_metrics: Option<PerpTofMetrics>,
    pub perp_score: Option<u8>,
    pub perp_candidate_type: Option<String>,
    pub final_candidate_type: Option<String>,
    pub metrics_direction: Option<String>,
    #[serde(default, deserialize_with = "deserialize_available_metric")]
    pub advanced_tof_metrics: Option<AdvancedTofMetrics>,
    pub advanced_score: Option<u8>,
    pub advanced_candidate_type: Option<String>,
    pub main_force_score: Option<u8>,
    pub extreme_impact_score: Option<u8>,
    pub structure_bias: Option<i16>,
    pub market_structure_confidence: Option<f64>,
    pub market_structure_data_quality: Option<f64>,
    pub market_structure_severity: Option<String>,
    /// Evidence-first behavior lane. Runtime-generated requests populate this
    /// so ordinary impact/volume observations cannot masquerade as main-force
    /// behavior notifications.
    pub behavior_type: Option<String>,
    pub behavior_state: Option<String>,
    pub behavior_confidence: Option<u8>,
    pub behavior_main_force_confirmed: Option<bool>,
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
    Preview,
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

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
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
    Json(mut body): Json<DiscordNotificationRequest>,
) -> impl IntoResponse {
    hydrate_authoritative_short_toxic_request(&state, &mut body);
    let gate = AlertGate::from_env(alert_family(&body));
    let mode = if body.test == Some(true) {
        DiscordAlertMode::Preview
    } else {
        DiscordAlertMode::Manual
    };
    let decision = evaluate_discord_alert_gate(&body, mode);
    let webhook_delivery_permitted = webhook_delivery_permitted(&body, &decision);
    record_alert_gate_log(&state, &body, &decision, false);
    if !decision.allowed {
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

    let webhook_url = if webhook_delivery_permitted {
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
        Some(webhook_url)
    } else {
        None
    };

    let push_key = canonical_manual_push_key(&body);
    let cooldown_key = push_cooldown_key(&body);
    if push_key.is_none() {
        let failed_decision = AlertGateDecision {
            allowed: false,
            reason: "authoritative_signal_id_unavailable",
            ..decision.clone()
        };
        record_discord_alert_status(&body, &failed_decision, false, None);
        record_discord_log(
            &state,
            "warn",
            "discord_manual_push_skipped",
            "Discord manual push skipped: canonical signal id is unavailable",
            &body,
        );
        return (
            StatusCode::OK,
            Json(DiscordNotificationResponse {
                ok: false,
                configured: decision.configured,
                reason: failed_decision.reason,
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
    if let Some(key) = push_key.as_deref() {
        let limiter = if webhook_delivery_permitted {
            discord_push_limiter()
        } else {
            discord_test_preview_limiter()
        };
        if let Some(reason) = limiter.lock().expect("discord limiter").reserve(
            key,
            cooldown_key.as_deref(),
            Some(cooldown_duration_for_signal(&body)),
        ) {
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
                    configured: decision.configured,
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

    // `test=true` is a payload preview that still requires authoritative
    // hydration, runtime safety, score thresholds and isolated rate/cooldown
    // limits. Delivery-only DRY_RUN and webhook configuration do not block a
    // preview, because no HTTP client is built and no webhook is contacted.
    if !webhook_delivery_permitted {
        let _validated_payload = discord_candidate_payload(&body);
        record_discord_log(
            &state,
            "info",
            "discord_test_preview_ready",
            "Discord test preview validated; webhook delivery was not attempted",
            &body,
        );
        return (
            StatusCode::OK,
            Json(DiscordNotificationResponse {
                ok: true,
                configured: decision.configured,
                reason: "TEST_PREVIEW_ONLY",
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

    let webhook_url = webhook_url.expect("permitted Discord delivery must have a validated URL");
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

fn webhook_delivery_permitted(
    request: &DiscordNotificationRequest,
    decision: &AlertGateDecision,
) -> bool {
    decision.allowed && request.test != Some(true)
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
        .filter(|value| *value <= 100)
        .or_else(|| {
            std::env::var(fallback)
                .ok()
                .and_then(|value| value.parse::<u8>().ok())
                .filter(|value| *value <= 100)
        })
}

fn read_f64_env(primary: String, fallback: &str) -> Option<f64> {
    std::env::var(&primary)
        .ok()
        .and_then(|value| value.parse::<f64>().ok())
        .filter(|value| value.is_finite() && (0.0..=100.0).contains(value))
        .or_else(|| {
            std::env::var(fallback)
                .ok()
                .and_then(|value| value.parse::<f64>().ok())
                .filter(|value| value.is_finite() && (0.0..=100.0).contains(value))
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
    if family == "MARKET_STRUCTURE"
        && !matches!(mode, DiscordAlertMode::Preview)
        && behavior_lane_is_present(signal)
        && !behavior_lane_allows_push(signal)
    {
        return behavior_rejection_decision(signal, gate, auto_push_enabled, dry_run, configured);
    }
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

fn behavior_lane_is_present(signal: &DiscordNotificationRequest) -> bool {
    signal.behavior_type.is_some()
        || signal.behavior_state.is_some()
        || signal.behavior_main_force_confirmed.is_some()
}

fn behavior_lane_allows_push(signal: &DiscordNotificationRequest) -> bool {
    let behavior_type = signal
        .behavior_type
        .as_deref()
        .unwrap_or_default()
        .trim()
        .to_ascii_lowercase();
    if behavior_type == "liquidation_sweep" {
        // Liquidation is an impact lane; it must not be mislabeled as a
        // confirmed main-force behavior, but it may still pass the existing
        // market-impact gate.
        return true;
    }
    signal
        .behavior_state
        .as_deref()
        .is_some_and(|state| state.eq_ignore_ascii_case("confirmed"))
        && signal.behavior_main_force_confirmed == Some(true)
        && signal.behavior_confidence.unwrap_or(0) >= 80
        && behavior_type != "insufficient_evidence"
}

fn behavior_rejection_decision(
    signal: &DiscordNotificationRequest,
    gate: AlertGate,
    auto_push_enabled: bool,
    dry_run: bool,
    configured: bool,
) -> AlertGateDecision {
    AlertGateDecision {
        allowed: false,
        reason: "behavior_not_confirmed",
        severity_allowed: severity_allows(signal.level.as_deref()),
        score: signal.main_force_score.unwrap_or(signal.score.unwrap_or(0)),
        confidence: signal
            .behavior_confidence
            .map(f64::from)
            .unwrap_or_else(|| signal.confidence.unwrap_or(0.0)),
        data_quality: signal.data_quality.unwrap_or(0.0),
        min_score: gate.min_score,
        min_confidence: gate.min_confidence,
        min_data_quality: gate.min_data_quality,
        auto_push_enabled,
        dry_run,
        configured,
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
    match std::env::var("DRY_RUN") {
        Err(_) => true,
        Ok(value) => match value.trim().to_ascii_lowercase().as_str() {
            "1" | "true" | "yes" | "on" => true,
            "0" | "false" | "no" | "off" => false,
            // An empty or malformed safety flag must not enable delivery.
            _ => true,
        },
    }
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

fn canonical_manual_push_key(signal: &DiscordNotificationRequest) -> Option<String> {
    if !signal.server_evidence_verified {
        return None;
    }
    let signal_id = signal
        .signal_id
        .as_deref()
        .map(str::trim)
        .filter(|value| !value.is_empty())?;
    Some(format!(
        "manual:{}:{}",
        alert_family_label(signal),
        signal_id
    ))
}

fn push_cooldown_key(signal: &DiscordNotificationRequest) -> Option<String> {
    let symbol = signal.symbol.as_deref()?.trim();
    if symbol.is_empty() {
        return None;
    }
    let canonical_symbol = crate::normalizers::symbol::canonical_base_asset(symbol)
        .unwrap_or_else(|| symbol.to_ascii_uppercase());
    let side = signal.side.as_deref().unwrap_or("unknown").trim();
    let signal_type = signal.signal_type.as_deref().unwrap_or("unknown").trim();
    Some(format!(
        "{}:{}:{}:{}",
        alert_family_label(signal),
        canonical_symbol,
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

    discord_candidate_payload(signal)
}

fn discord_candidate_payload(signal: &DiscordNotificationRequest) -> DiscordWebhookPayload {
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
    if let Some(tof_metrics) = authoritative_tof_metrics(signal) {
        let vpin = authoritative_tof_metric(tof_metrics, "vpin", tof_metrics.vpin_proxy)
            .map(|value| format!("{value:.0}"))
            .unwrap_or_else(|| "N/A".to_string());
        let imbalance =
            authoritative_tof_metric(tof_metrics, "tradeImbalance", tof_metrics.trade_imbalance)
                .map(|value| format!("{value:.2}"))
                .unwrap_or_else(|| "N/A".to_string());
        let spread = authoritative_tof_metric(tof_metrics, "spread", tof_metrics.spread_bps)
            .map(|value| format!("{value:.1}bps"))
            .unwrap_or_else(|| "N/A".to_string());
        let depth =
            authoritative_tof_metric(tof_metrics, "depth", tof_metrics.depth_withdrawal_score)
                .map(|value| format!("{value:.0}"))
                .unwrap_or_else(|| "N/A".to_string());
        fields.push(DiscordEmbedField {
            name: "TOF 指标".to_string(),
            value: format!(
                "TOF {:.0} / VPIN {} / Imbalance {} / Spread {} / Depth {}",
                signal.tof_score.unwrap_or(tof_metrics.tof_score),
                vpin,
                imbalance,
                spread,
                depth,
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
        name: "最终结果".to_string(),
        value: format!(
            "{}；短线毒性风险升高，不代表中长线趋势。",
            final_result.trim_end_matches('。')
        ),
        inline: false,
    });
    fields.push(DiscordEmbedField {
        name: "说明".to_string(),
        value: if authoritative_tof_metrics(signal).is_some() {
            "短线有毒订单 Candidate only，基于已验证的公开盘口 / L2 / 成交数据推断；只做提醒，不代表中长线趋势，不执行下单、拦截、封禁或资金操作。"
        } else {
            "短线有毒订单 Candidate only，基于服务端已验证的可用候选证据；只做提醒，不代表中长线趋势，不执行下单、拦截、封禁或资金操作。"
        }
        .to_string(),
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
    let authoritative_spot = authoritative_tof_metrics(signal).is_some();
    let displayed_spot_score = if authoritative_spot {
        signal.spot_score
    } else {
        None
    };
    let structure_bias = signal.structure_bias.unwrap_or(0);
    let regime_type = signal.regime_type.as_deref().unwrap_or("unclear");
    let regime_label = market_structure_regime_label(regime_type);
    let severity = market_structure_severity_label(signal);
    let direction = market_structure_direction_from_bias(structure_bias);
    let behavior_type = signal
        .behavior_type
        .as_deref()
        .unwrap_or("insufficient_evidence");
    let behavior_state = signal.behavior_state.as_deref().unwrap_or("insufficient");
    let behavior_label = match behavior_type {
        "new_long_build" => "新多建仓",
        "new_short_build" => "新空建仓",
        "short_covering" => "空头回补",
        "long_unwind" => "多头平仓",
        "downside_absorption" => "下方吸收",
        "upside_suppression" => "上方压制",
        "liquidation_sweep" => "清算驱动",
        _ => "普通成交流",
    };
    let behavior_state_label = match behavior_state {
        "confirmed" => "已确认",
        "provisional" => "候选",
        "invalidated" => "已失效",
        _ => "证据不足",
    };
    let extreme_template = matches!(
        market_structure_trigger(signal, &AlertGate::from_env("MARKET_STRUCTURE")),
        Some(MarketStructureTrigger::ExtremeImpact | MarketStructureTrigger::ImpactLevel)
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
            name: "冲击等级".to_string(),
            value: market_structure_impact_level(signal)
                .unwrap_or("N/A")
                .to_string(),
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
            value: displayed_spot_score
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
        DiscordEmbedField {
            name: "主力行为".to_string(),
            value: format!(
                "{behavior_label} · {behavior_state_label} · {}/100",
                signal.behavior_confidence.unwrap_or(0)
            ),
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
        value: "主力结构 Candidate only，仅使用已通过来源验证的可用现货、合约、OI、价格响应和公开成交上下文；只做提醒，不执行下单、拦截、封禁或资金操作。".to_string(),
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
    _direction: NormalizedDiscordDirection,
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
        "短线有毒流"
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
    _direction: NormalizedDiscordDirection,
) -> String {
    let mut reasons = vec!["检测器识别到短线毒性候选".to_string()];
    if let Some(metrics) = authoritative_tof_metrics(signal) {
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
    if signal.server_evidence_verified {
        if let Some(price_impact_bps) = signal.price_impact_bps {
            reasons.push(format!("价格短线冲击 {:.2}bps", price_impact_bps));
        }
        if let Some(tags) = signal.explain_tags.as_ref() {
            for tag in tags.iter().take(2) {
                reasons.push(format!("解释标签：{tag}"));
            }
        }
    }
    reasons
        .into_iter()
        .take(5)
        .map(|reason| format!("- {reason}"))
        .collect::<Vec<_>>()
        .join("\n")
}

fn authoritative_tof_metrics(signal: &DiscordNotificationRequest) -> Option<&TofMetrics> {
    if !signal.server_evidence_verified {
        return None;
    }
    signal
        .tof_metrics
        .as_ref()
        .filter(|metrics| metrics.lineage.alert_eligible)
}

fn authoritative_tof_metric(metrics: &TofMetrics, key: &str, value: f64) -> Option<f64> {
    metrics
        .metric_lineage
        .get(key)
        .unwrap_or(&metrics.lineage)
        .alert_eligible
        .then_some(value)
        .filter(|value| value.is_finite())
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
    let authoritative_perp = signal.server_evidence_verified
        && signal
            .perp_tof_metrics
            .as_ref()
            .is_some_and(|metrics| metrics.lineage.alert_eligible);
    let authoritative_liquidation = signal.server_evidence_verified
        && signal
            .perp_tof_metrics
            .as_ref()
            .is_some_and(|metrics| metrics.liquidation_lineage.alert_eligible);
    let authoritative_spot = authoritative_tof_metrics(signal).is_some();
    if authoritative_perp {
        if signal.oi_score.unwrap_or(0) >= 70 {
            reasons.push(match direction {
                NormalizedDiscordDirection::Bullish => "OI 同步上升，偏新多开仓".to_string(),
                NormalizedDiscordDirection::Bearish => "OI 同步上升，偏新空开仓".to_string(),
                NormalizedDiscordDirection::Neutral => "OI 变化明显，结构进入再定价".to_string(),
            });
        } else if extreme_template {
            reasons.push("OI 快速下降".to_string());
        }
    }
    if authoritative_spot {
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
    }
    if signal.server_evidence_verified
        && signal
            .price_impact_bps
            .is_some_and(|value| value.is_finite() && value.abs() > 0.0)
    {
        reasons.push("价格出现明显短线冲击".to_string());
    }
    if authoritative_liquidation && signal.liquidation_score.unwrap_or(0) >= 70 && extreme_template
    {
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
    if behavior_lane_is_present(signal) {
        return "market_structure";
    }
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

    if !signal.server_evidence_verified {
        decision.allowed = false;
        decision.reason = "authoritative_evidence_unavailable";
    } else if !short_toxic_evidence_is_valid(signal) {
        decision.allowed = false;
        decision.reason = "invalid_authoritative_evidence";
    } else if matches!(mode, DiscordAlertMode::Auto) && !auto_push_enabled {
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
    } else if dry_run && !matches!(mode, DiscordAlertMode::Preview) {
        decision.allowed = false;
        decision.reason = "dry_run";
    } else if !configured && !matches!(mode, DiscordAlertMode::Preview) {
        decision.allowed = false;
        decision.reason = "webhook_missing";
    }

    decision
}

fn short_toxic_evidence_is_valid(signal: &DiscordNotificationRequest) -> bool {
    signal.score.is_none_or(|score| score <= 100)
        && signal
            .confidence
            .or(signal.direction_confidence)
            .unwrap_or(0.0)
            .is_finite()
        && (0.0..=100.0).contains(
            &signal
                .confidence
                .or(signal.direction_confidence)
                .unwrap_or(0.0),
        )
        && signal.data_quality.unwrap_or(0.0).is_finite()
        && (0.0..=100.0).contains(&signal.data_quality.unwrap_or(0.0))
}

fn hydrate_authoritative_short_toxic_request(
    state: &AppState,
    request: &mut DiscordNotificationRequest,
) {
    let family = alert_family(request).to_string();
    if family != "SHORT_TOXIC" && family != "MARKET_STRUCTURE" {
        return;
    }
    let requested_signal_id = request
        .signal_id
        .as_deref()
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(str::to_string);
    let requested_symbol = request
        .symbol
        .as_deref()
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .unwrap_or(&state.config().symbol)
        .to_ascii_uppercase();

    // The request body supplies only the canonical signal lookup key and the
    // desired family. Every value that can affect a gate or Discord wording is
    // cleared before the server reconstructs it from monitor state.
    clear_client_controlled_alert_content(request);
    request.alert_family = Some(
        match family.as_str() {
            "MARKET_STRUCTURE" => "market_structure",
            _ => "short_toxic_order",
        }
        .to_string(),
    );
    let Some(signal_id) = requested_signal_id else {
        return;
    };
    let recent =
        crate::api::toxic_quality_scorecard_routes::build_fusion_recent(state, &requested_symbol);
    if !state.runtime_started()
        || !recent.read_only
        || recent.runtime_modified
        || !recent.analysis_only
        || recent.execution_enabled
    {
        return;
    }
    let Some(signal) = recent.signals.iter().find(|signal| {
        signal.signal_id == signal_id && signal.symbol.eq_ignore_ascii_case(&requested_symbol)
    }) else {
        return;
    };
    let Some(data_quality) = signal
        .data_quality
        .filter(|value| value.is_finite() && *value >= 0.0 && *value <= 100.0)
    else {
        return;
    };
    if family == "MARKET_STRUCTURE" {
        let cwm_signal = crate::api::toxic_signal_inbox_routes::latest_cwm_signal_for_state(
            state,
            &requested_symbol,
        );
        let tof_snapshot = crate::api::toxic_signal_inbox_routes::observed_tof_snapshot_for_state(
            state,
            &requested_symbol,
        );
        let inbox_recent =
            crate::api::toxic_signal_inbox_routes::build_recent(state, &requested_symbol);
        let snapshot =
            crate::api::toxic_signal_ws_routes::build_ws_snapshot_with_authoritative_state(
                &inbox_recent,
                cwm_signal.as_ref(),
                tof_snapshot.as_ref(),
                state.runtime_started(),
            );
        let Some(authoritative) = snapshot
            .signals
            .iter()
            .find(|candidate| candidate.id == signal_id)
        else {
            return;
        };
        if !authoritative.alert_eligible || !authoritative.cwm_contribution.available {
            return;
        }
        request.alert_family = Some("market_structure".to_string());
        request.signal_id = Some(authoritative.id.clone());
        request.id = Some(authoritative.id.clone());
        request.exchange = authoritative.cwm_contribution.main_exchange.clone();
        request.symbol = Some(authoritative.symbol.clone());
        request.signal_type = Some(authoritative.detector.clone());
        request.level = authoritative.market_structure_severity.clone();
        request.side = Some(authoritative.direction_label.clone());
        request.score = Some(authoritative.risk_score);
        request.confidence = Some(authoritative.confidence * 100.0);
        request.data_quality = Some(data_quality);
        request.reason = Some(authoritative.core_reason.clone());
        request.time = Some(authoritative.created_at.clone());
        if authoritative.tof_metrics.lineage.alert_eligible {
            request.tof_score = authoritative.tof_score;
            request.tof_metrics = Some(authoritative.tof_metrics.clone());
        }
        if authoritative.perp_tof_metrics.lineage.alert_eligible {
            request.perp_score = authoritative.perp_score;
            request.perp_tof_metrics = Some(authoritative.perp_tof_metrics.clone());
        }
        request.main_force_score = authoritative.main_force_score;
        request.extreme_impact_score = authoritative.extreme_impact_score;
        request.structure_bias = authoritative.structure_bias;
        request.market_structure_confidence = authoritative.market_structure_confidence;
        request.market_structure_data_quality = authoritative.market_structure_data_quality;
        request.market_structure_severity = authoritative.market_structure_severity.clone();
        request.behavior_type = authoritative.behavior_type.clone();
        request.behavior_state = authoritative.behavior_state.clone();
        request.behavior_confidence = authoritative.behavior_confidence;
        request.behavior_main_force_confirmed = authoritative.behavior_main_force_confirmed;
        request.regime_type = authoritative.regime_type.clone();
        request.spot_score = authoritative.spot_score;
        request.contract_score = authoritative.contract_score;
        request.cross_confirm_score = authoritative.cross_confirm_score;
        request.main_force_confirmed = authoritative.main_force_confirmed;
        request.signal_agreement = authoritative.signal_agreement;
        request.source_coverage = authoritative.source_coverage;
        request.oi_score = authoritative.oi_score;
        request.liquidation_score = authoritative
            .perp_tof_metrics
            .liquidation_lineage
            .alert_eligible
            .then_some(authoritative.liquidation_score)
            .flatten();
        if !market_structure_evidence_is_valid(request) {
            clear_client_controlled_alert_content(request);
            request.alert_family = Some("market_structure".to_string());
            return;
        }
        request.server_evidence_verified = true;
        return;
    }
    request.alert_family = Some("short_toxic_order".to_string());
    request.signal_id = Some(signal.signal_id.clone());
    request.id = Some(signal.signal_id.clone());
    request.score = Some(signal.toxicity_score);
    request.data_quality = Some(data_quality);
    request.confidence = Some(canonical_toxic_confidence_percent(signal.confidence));
    request.level = Some(
        if signal.toxicity_score >= 90 {
            "critical"
        } else if signal.toxicity_score >= 80 {
            "high"
        } else if signal.toxicity_score >= 65 {
            "medium"
        } else {
            "low"
        }
        .to_string(),
    );
    request.symbol = Some(signal.symbol.clone());
    request.time = chrono::DateTime::<chrono::Utc>::from_timestamp_millis(signal.ts_ms as i64)
        .map(|value| value.to_rfc3339_opts(chrono::SecondsFormat::Millis, true));
    request.signal_type = serde_json::to_value(signal.signal_type)
        .ok()
        .and_then(|value| value.as_str().map(str::to_string));
    request.side = serde_json::to_value(signal.direction)
        .ok()
        .and_then(|value| value.as_str().map(str::to_string));
    request.exchange = signal
        .evidence
        .as_ref()
        .map(|evidence| evidence.venue.clone())
        .or_else(|| Some("Runtime".to_string()));
    request.candidate_type = request.signal_type.clone();
    request.explain_tags = Some(signal.reason.clone());
    request.direction_confidence = request.confidence;
    if let Some(evidence) = signal.evidence.as_ref() {
        request.add_qty = evidence.add_qty.is_finite().then_some(evidence.add_qty);
        request.cancel_qty = evidence
            .cancel_qty
            .is_finite()
            .then_some(evidence.cancel_qty);
        request.fill_qty = evidence.fill_qty.is_finite().then_some(evidence.fill_qty);
        request.cancel_to_trade_ratio = finite_metric(evidence.cancel_to_trade_ratio);
        request.depth_before = finite_metric(evidence.depth_before);
        request.depth_after = finite_metric(evidence.depth_after);
        request.depth_impact = finite_metric(evidence.depth_impact);
        request.price_impact_bps = finite_metric(evidence.price_impact_bps);
        request.markout_1s_bps = finite_metric(evidence.markout_1s_bps);
        request.markout_5s_bps = finite_metric(evidence.markout_5s_bps);
        request.markout_30s_bps = finite_metric(evidence.markout_30s_bps);
    }
    request.reason = Some(signal.primary_reason.clone());
    request.server_evidence_verified = true;
}

fn finite_metric(value: Option<f64>) -> Option<f64> {
    value.filter(|metric| metric.is_finite())
}

fn canonical_toxic_confidence_percent(
    confidence: crate::types::toxic_flow::ToxicConfidence,
) -> f64 {
    crate::toxicity::toxic_signal_inbox::toxic_confidence_score(confidence) * 100.0
}

fn clear_client_controlled_alert_content(request: &mut DiscordNotificationRequest) {
    request.server_evidence_verified = false;
    request.signal_id = None;
    request.id = None;
    request.dedupe_key = None;
    request.exchange = None;
    request.symbol = None;
    request.signal_type = None;
    request.level = None;
    request.side = None;
    request.score = None;
    request.confidence = None;
    request.data_quality = None;
    request.reason = None;
    request.impact = None;
    request.impact_level = None;
    request.time = None;
    request.price_range = None;
    request.add_qty = None;
    request.cancel_qty = None;
    request.fill_qty = None;
    request.cancel_to_trade_ratio = None;
    request.depth_before = None;
    request.depth_after = None;
    request.depth_impact = None;
    request.price_impact_bps = None;
    request.markout_1s_bps = None;
    request.markout_5s_bps = None;
    request.markout_30s_bps = None;
    request.tof_metrics = None;
    request.tof_score = None;
    request.candidate_type = None;
    request.explain_tags = None;
    request.direction_confidence = None;
    request.perp_tof_metrics = None;
    request.perp_score = None;
    request.perp_candidate_type = None;
    request.final_candidate_type = None;
    request.metrics_direction = None;
    request.advanced_tof_metrics = None;
    request.advanced_score = None;
    request.advanced_candidate_type = None;
    request.main_force_score = None;
    request.extreme_impact_score = None;
    request.structure_bias = None;
    request.market_structure_confidence = None;
    request.market_structure_data_quality = None;
    request.market_structure_severity = None;
    request.behavior_type = None;
    request.behavior_state = None;
    request.behavior_confidence = None;
    request.behavior_main_force_confirmed = None;
    request.regime_type = None;
    request.spot_score = None;
    request.contract_score = None;
    request.cross_confirm_score = None;
    request.main_force_confirmed = None;
    request.signal_agreement = None;
    request.source_coverage = None;
    request.oi_score = None;
    request.liquidation_score = None;
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
    let evidence_valid = market_structure_evidence_is_valid(signal);
    let trigger = if evidence_valid {
        market_structure_trigger(signal, gate)
    } else {
        None
    };
    let score = match trigger {
        Some(MarketStructureTrigger::MainForce) => market_structure_main_force_score(signal),
        Some(MarketStructureTrigger::ExtremeImpact) => market_structure_extreme_score(signal),
        Some(MarketStructureTrigger::ImpactLevel) => {
            market_structure_impact_level_score(signal).unwrap_or(0)
        }
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

    if !signal.server_evidence_verified {
        decision.allowed = false;
        decision.reason = "authoritative_evidence_unavailable";
    } else if !evidence_valid {
        decision.allowed = false;
        decision.reason = "invalid_authoritative_evidence";
    } else if matches!(mode, DiscordAlertMode::Auto) && !auto_push_enabled {
        decision.allowed = false;
        decision.reason = "auto_disabled";
    } else if trigger.is_none() {
        decision.allowed = false;
        decision.reason = market_structure_rejection_reason(signal, gate);
    } else if dry_run && !matches!(mode, DiscordAlertMode::Preview) {
        decision.allowed = false;
        decision.reason = "dry_run";
    } else if !configured && !matches!(mode, DiscordAlertMode::Preview) {
        decision.allowed = false;
        decision.reason = "webhook_missing";
    }

    decision
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum MarketStructureTrigger {
    MainForce,
    ExtremeImpact,
    ImpactLevel,
}

fn market_structure_trigger(
    signal: &DiscordNotificationRequest,
    gate: &AlertGate,
) -> Option<MarketStructureTrigger> {
    if !market_structure_evidence_is_valid(signal) {
        return None;
    }
    let data_quality = market_structure_data_quality(signal);
    if data_quality < gate.min_data_quality {
        return None;
    }
    if market_structure_main_force_score(signal) >= gate.min_score
        && market_structure_confidence(signal) >= gate.min_confidence
        && signal.main_force_confirmed == Some(true)
    {
        return Some(MarketStructureTrigger::MainForce);
    }
    if market_structure_extreme_score(signal) >= gate.min_extreme_score {
        return Some(MarketStructureTrigger::ExtremeImpact);
    }
    if market_structure_impact_level_score(signal).is_some() {
        return Some(MarketStructureTrigger::ImpactLevel);
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
    if main_force_score >= gate.min_score
        && confidence >= gate.min_confidence
        && signal.main_force_confirmed != Some(true)
    {
        return "main_force_not_confirmed";
    }
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

fn market_structure_impact_level(signal: &DiscordNotificationRequest) -> Option<&'static str> {
    let level = signal
        .impact_level
        .as_deref()
        .or(signal.level.as_deref())
        .unwrap_or("")
        .trim()
        .to_ascii_uppercase();
    match level.as_str() {
        "S" => Some("S"),
        "A" => Some("A"),
        "B" => Some("B"),
        _ => None,
    }
}

fn market_structure_impact_level_score(signal: &DiscordNotificationRequest) -> Option<u8> {
    match market_structure_impact_level(signal)? {
        "S" => Some(95),
        "A" => Some(85),
        "B" => Some(80),
        _ => None,
    }
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

fn market_structure_evidence_is_valid(signal: &DiscordNotificationRequest) -> bool {
    let Some(main_force_score) = signal.main_force_score else {
        return false;
    };
    let Some(extreme_impact_score) = signal.extreme_impact_score else {
        return false;
    };
    let Some(structure_bias) = signal.structure_bias else {
        return false;
    };
    let Some(confidence) = signal.market_structure_confidence else {
        return false;
    };
    let Some(data_quality) = signal.market_structure_data_quality else {
        return false;
    };
    if signal.main_force_confirmed.is_none()
        || main_force_score > 100
        || extreme_impact_score > 100
        || !(-100..=100).contains(&structure_bias)
        || !confidence.is_finite()
        || !(0.0..=100.0).contains(&confidence)
        || !data_quality.is_finite()
        || !(0.0..=100.0).contains(&data_quality)
    {
        return false;
    }

    [
        signal.score,
        signal.spot_score,
        signal.contract_score,
        signal.cross_confirm_score,
        signal.signal_agreement,
        signal.source_coverage,
        signal.oi_score,
        signal.liquidation_score,
    ]
    .into_iter()
    .flatten()
    .all(|score| score <= 100)
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

fn discord_test_preview_limiter() -> &'static Mutex<DiscordPushLimiter> {
    static LIMITER: OnceLock<Mutex<DiscordPushLimiter>> = OnceLock::new();
    LIMITER.get_or_init(|| Mutex::new(DiscordPushLimiter::new()))
}

pub fn reset_discord_push_limits_for_tests() {
    *discord_push_limiter().lock().expect("discord limiter") = DiscordPushLimiter::new();
    *discord_test_preview_limiter()
        .lock()
        .expect("discord preview limiter") = DiscordPushLimiter::new();
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

const DISCORD_AUTO_TRACKER_MAX_ENTRIES: usize = 4_096;

#[derive(Debug, Default)]
struct DiscordAutoPushTracker {
    seen_keys: HashSet<String>,
    seen_order: VecDeque<String>,
    statuses: HashMap<String, DiscordAlertPublicStatus>,
    status_order: VecDeque<String>,
}

impl DiscordAutoPushTracker {
    fn mark_once(&mut self, key: &str) -> bool {
        if !self.seen_keys.insert(key.to_string()) {
            return false;
        }
        self.seen_order.push_back(key.to_string());
        while self.seen_order.len() > DISCORD_AUTO_TRACKER_MAX_ENTRIES {
            if let Some(oldest) = self.seen_order.pop_front() {
                self.seen_keys.remove(&oldest);
            }
        }
        true
    }

    fn set_status(&mut self, key: &str, status: DiscordAlertPublicStatus) {
        // A successfully delivered alert is terminal for this candidate. A
        // later polling pass may observe it as a duplicate, but must not erase
        // the sent timestamp or downgrade the public state.
        if let Some(existing) = self.statuses.get_mut(key) {
            if existing.auto_sent && !status.auto_sent {
                if status.manual_sent_at.is_some() {
                    existing.manual_sent_at = status.manual_sent_at;
                }
                return;
            }
        }
        if !self.statuses.contains_key(key) {
            self.status_order.push_back(key.to_string());
        }
        self.statuses.insert(key.to_string(), status);
        while self.status_order.len() > DISCORD_AUTO_TRACKER_MAX_ENTRIES {
            if let Some(oldest) = self.status_order.pop_front() {
                self.statuses.remove(&oldest);
            }
        }
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

    use crate::test_support;

    use super::{
        discord_payload, evaluate_discord_alert_gate, validate_discord_webhook_url,
        DiscordAlertMode, DiscordNotificationRequest, DiscordPushLimiter,
    };

    fn env_lock() -> &'static Mutex<()> {
        static LOCK: OnceLock<Mutex<()>> = OnceLock::new();
        LOCK.get_or_init(|| Mutex::new(()))
    }

    #[test]
    fn manual_and_auto_share_the_canonical_confidence_scale() {
        use crate::types::toxic_flow::ToxicConfidence;

        for (confidence, expected) in [
            (ToxicConfidence::Low, 35.0),
            (ToxicConfidence::Medium, 62.0),
            (ToxicConfidence::High, 82.0),
        ] {
            assert_eq!(
                super::canonical_toxic_confidence_percent(confidence),
                expected
            );
            assert_eq!(
                crate::toxicity::toxic_signal_inbox::toxic_confidence_score(confidence) * 100.0,
                expected
            );
        }
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
        let mut invalid_confidence = request(Some(90), Some(90.0));
        invalid_confidence.confidence = Some(f64::NAN);
        assert_eq!(
            evaluate_discord_alert_gate(&invalid_confidence, DiscordAlertMode::Manual).reason,
            "invalid_authoritative_evidence"
        );
        let invalid_quality = request(Some(90), Some(f64::INFINITY));
        assert_eq!(
            evaluate_discord_alert_gate(&invalid_quality, DiscordAlertMode::Manual).reason,
            "invalid_authoritative_evidence"
        );
        let invalid_score = request(Some(101), Some(90.0));
        assert_eq!(
            evaluate_discord_alert_gate(&invalid_score, DiscordAlertMode::Manual).reason,
            "invalid_authoritative_evidence"
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
        let _guard = test_support::env_lock().lock().expect("env lock");
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

    #[test]
    fn cooldown_key_canonicalizes_symbol_aliases() {
        let mut binance = request(Some(90), Some(90.0));
        binance.symbol = Some("BTCUSDT".to_string());
        binance.signal_type = Some("spoofing".to_string());
        binance.side = Some("Ask/Sell".to_string());

        let mut canonical = request(Some(90), Some(90.0));
        canonical.symbol = Some("BTC-PERP".to_string());
        canonical.signal_type = Some("spoofing".to_string());
        canonical.side = Some("Ask/Sell".to_string());

        assert_eq!(
            super::push_cooldown_key(&binance),
            super::push_cooldown_key(&canonical)
        );
        assert!(super::push_cooldown_key(&binance)
            .expect("cooldown key")
            .contains(":BTC:"));
    }

    #[test]
    fn malformed_or_empty_dry_run_value_fails_closed() {
        let _guard = env_lock().lock().expect("env lock");
        for value in ["", "garbage", "2", "tru"] {
            std::env::set_var("DRY_RUN", value);
            assert!(super::discord_dry_run(), "DRY_RUN={value:?} must be safe");
        }
        std::env::set_var("DRY_RUN", "false");
        assert!(!super::discord_dry_run());
        std::env::remove_var("DRY_RUN");
        assert!(super::discord_dry_run());
    }

    #[test]
    fn non_finite_or_out_of_range_threshold_env_is_ignored() {
        let _guard = env_lock().lock().expect("env lock");
        std::env::set_var("TEST_PRIMARY_THRESHOLD", "NaN");
        std::env::set_var("TEST_FALLBACK_THRESHOLD", "77");
        assert_eq!(
            super::read_f64_env(
                "TEST_PRIMARY_THRESHOLD".to_string(),
                "TEST_FALLBACK_THRESHOLD"
            ),
            Some(77.0)
        );

        for invalid in ["inf", "-inf", "101", "-1"] {
            std::env::set_var("TEST_PRIMARY_THRESHOLD", invalid);
            std::env::remove_var("TEST_FALLBACK_THRESHOLD");
            assert_eq!(
                super::read_f64_env(
                    "TEST_PRIMARY_THRESHOLD".to_string(),
                    "TEST_FALLBACK_THRESHOLD"
                ),
                None,
                "threshold {invalid:?} must be rejected"
            );
        }
        std::env::remove_var("TEST_PRIMARY_THRESHOLD");
        std::env::remove_var("TEST_FALLBACK_THRESHOLD");
    }

    #[test]
    fn test_flag_is_preview_only_even_after_a_gate_passes() {
        let mut request = request(Some(95), Some(95.0));
        request.test = Some(true);
        let decision = super::AlertGateDecision {
            allowed: true,
            reason: "passed",
            severity_allowed: true,
            score: 95,
            confidence: 95.0,
            data_quality: 95.0,
            min_score: 80,
            min_confidence: 70.0,
            min_data_quality: 70.0,
            auto_push_enabled: true,
            dry_run: false,
            configured: true,
        };

        assert!(!super::webhook_delivery_permitted(&request, &decision));
        request.test = Some(false);
        assert!(super::webhook_delivery_permitted(&request, &decision));
    }

    #[test]
    fn preview_validates_candidate_without_requiring_delivery_configuration() {
        let _guard = env_lock().lock().expect("env lock");
        std::env::set_var("SHORT_TOXIC_ALERT_MIN_SCORE", "85");
        std::env::set_var("SHORT_TOXIC_ALERT_MIN_CONFIDENCE", "70");
        std::env::set_var("SHORT_TOXIC_ALERT_MIN_DATA_QUALITY", "70");
        std::env::set_var("DRY_RUN", "true");
        std::env::remove_var("SHORT_TOXIC_DISCORD_WEBHOOK_URL");
        std::env::remove_var("DISCORD_WEBHOOK_URL");

        let mut candidate = request(Some(92), Some(88.0));
        candidate.test = Some(true);
        let preview = evaluate_discord_alert_gate(&candidate, DiscordAlertMode::Preview);
        assert!(preview.allowed);
        assert!(preview.dry_run);
        assert!(!preview.configured);
        assert!(!super::webhook_delivery_permitted(&candidate, &preview));
        let _payload = super::discord_candidate_payload(&candidate);

        let mut below_threshold = request(Some(84), Some(88.0));
        below_threshold.test = Some(true);
        assert_eq!(
            evaluate_discord_alert_gate(&below_threshold, DiscordAlertMode::Preview).reason,
            "score_below_threshold"
        );

        candidate.test = Some(false);
        let manual = evaluate_discord_alert_gate(&candidate, DiscordAlertMode::Manual);
        assert!(!manual.allowed);
        assert_eq!(manual.reason, "dry_run");

        for key in [
            "SHORT_TOXIC_ALERT_MIN_SCORE",
            "SHORT_TOXIC_ALERT_MIN_CONFIDENCE",
            "SHORT_TOXIC_ALERT_MIN_DATA_QUALITY",
            "DRY_RUN",
        ] {
            std::env::remove_var(key);
        }
    }

    #[test]
    fn test_preview_uses_limits_without_consuming_real_delivery_capacity() {
        let mut preview = super::DiscordPushLimiter::new();
        let mut delivery = super::DiscordPushLimiter::new();
        let key = "manual:short_toxic_order:canonical-signal-42";

        assert_eq!(preview.reserve(key, None, None), None);
        assert_eq!(
            preview.reserve(key, None, None),
            Some("DUPLICATE_PUSH_SUPPRESSED")
        );
        assert_eq!(delivery.reserve(key, None, None), None);
    }

    #[test]
    fn manual_push_key_ignores_client_dedupe_and_is_always_limited() {
        let mut request = request(Some(95), Some(95.0));
        request.signal_id = Some("canonical-signal-42".to_string());
        request.dedupe_key = Some("client-controlled-bypass".to_string());
        let key = super::canonical_manual_push_key(&request).expect("canonical push key");
        assert_eq!(key, "manual:short_toxic_order:canonical-signal-42");

        let mut limiter = DiscordPushLimiter::new();
        assert_eq!(limiter.reserve(&key, None, None), None);
        request.dedupe_key = Some("different-client-value".to_string());
        let same_key = super::canonical_manual_push_key(&request).expect("same canonical key");
        assert_eq!(same_key, key);
        assert_eq!(
            limiter.reserve(&same_key, None, None),
            Some("DUPLICATE_PUSH_SUPPRESSED")
        );

        request.signal_id = None;
        request.id = Some("legacy-client-id".to_string());
        assert!(super::canonical_manual_push_key(&request).is_none());
    }

    #[test]
    fn clearing_untrusted_request_removes_content_and_evidence_for_both_families() {
        for family in ["short_toxic_order", "market_structure"] {
            let mut request = request(Some(99), Some(99.0));
            request.alert_family = Some(family.to_string());
            request.id = Some("forged-id".to_string());
            request.dedupe_key = Some("forged-key".to_string());
            request.reason = Some("forged judgment".to_string());
            request.impact_level = Some("S".to_string());
            request.price_impact_bps = Some(999.0);
            request.main_force_score = Some(100);
            request.market_structure_confidence = Some(100.0);
            request.market_structure_data_quality = Some(100.0);
            request.main_force_confirmed = Some(true);
            request.regime_type = Some("main_force_long_build".to_string());
            request.test = Some(true);

            super::clear_client_controlled_alert_content(&mut request);

            assert!(!request.server_evidence_verified);
            assert!(request.signal_id.is_none());
            assert!(request.id.is_none());
            assert!(request.dedupe_key.is_none());
            assert!(request.symbol.is_none());
            assert!(request.reason.is_none());
            assert!(request.impact_level.is_none());
            assert!(request.price_impact_bps.is_none());
            assert!(request.main_force_score.is_none());
            assert!(request.market_structure_confidence.is_none());
            assert!(request.market_structure_data_quality.is_none());
            assert!(request.main_force_confirmed.is_none());
            assert!(request.regime_type.is_none());
            assert_eq!(request.test, Some(true));
        }
    }

    #[test]
    fn market_structure_gate_requires_valid_evidence_and_main_force_confirmation() {
        let _guard = env_lock().lock().expect("env lock");
        std::env::set_var("MARKET_STRUCTURE_ALERT_MIN_SCORE", "80");
        std::env::set_var("MARKET_STRUCTURE_EXTREME_MIN_SCORE", "85");
        std::env::set_var("MARKET_STRUCTURE_ALERT_MIN_CONFIDENCE", "70");
        std::env::set_var("MARKET_STRUCTURE_ALERT_MIN_DATA_QUALITY", "70");
        std::env::set_var("MARKET_STRUCTURE_DISCORD_AUTO_PUSH_ENABLED", "true");
        std::env::set_var("DRY_RUN", "false");
        std::env::set_var(
            "MARKET_STRUCTURE_DISCORD_WEBHOOK_URL",
            "https://discord.com/api/webhooks/test-id/test-token",
        );

        let mut market = request(Some(90), Some(90.0));
        market.alert_family = Some("market_structure".to_string());
        market.main_force_score = Some(90);
        market.extreme_impact_score = Some(40);
        market.structure_bias = Some(55);
        market.market_structure_confidence = Some(90.0);
        market.market_structure_data_quality = Some(90.0);
        market.main_force_confirmed = Some(false);
        assert_eq!(
            evaluate_discord_alert_gate(&market, DiscordAlertMode::Auto).reason,
            "main_force_not_confirmed"
        );

        market.main_force_confirmed = Some(true);
        assert!(evaluate_discord_alert_gate(&market, DiscordAlertMode::Auto).allowed);

        market.market_structure_confidence = Some(f64::NAN);
        assert_eq!(
            evaluate_discord_alert_gate(&market, DiscordAlertMode::Auto).reason,
            "invalid_authoritative_evidence"
        );

        for key in [
            "MARKET_STRUCTURE_ALERT_MIN_SCORE",
            "MARKET_STRUCTURE_EXTREME_MIN_SCORE",
            "MARKET_STRUCTURE_ALERT_MIN_CONFIDENCE",
            "MARKET_STRUCTURE_ALERT_MIN_DATA_QUALITY",
            "MARKET_STRUCTURE_DISCORD_AUTO_PUSH_ENABLED",
            "MARKET_STRUCTURE_DISCORD_WEBHOOK_URL",
            "DRY_RUN",
        ] {
            std::env::remove_var(key);
        }
    }

    #[test]
    fn behavior_lane_blocks_ordinary_volume_even_when_impact_scores_are_high() {
        let _guard = env_lock().lock().expect("env lock");
        std::env::set_var("DRY_RUN", "false");
        std::env::set_var(
            "SHORT_TOXIC_ORDER_DISCORD_WEBHOOK_URL",
            "https://discord.com/api/webhooks/test-id/test-token",
        );
        let mut ordinary = request(Some(99), Some(99.0));
        ordinary.main_force_score = Some(99);
        ordinary.behavior_type = Some("insufficient_evidence".to_string());
        ordinary.behavior_state = Some("insufficient".to_string());
        ordinary.behavior_confidence = Some(0);
        ordinary.behavior_main_force_confirmed = Some(false);
        let decision = evaluate_discord_alert_gate(&ordinary, DiscordAlertMode::Auto);
        assert!(!decision.allowed);
        assert_eq!(decision.reason, "behavior_not_confirmed");
        std::env::remove_var("SHORT_TOXIC_ORDER_DISCORD_WEBHOOK_URL");
        std::env::remove_var("DRY_RUN");
    }

    #[test]
    fn behavior_lane_allows_only_confirmed_main_force_behavior() {
        let _guard = env_lock().lock().expect("env lock");
        std::env::set_var("DRY_RUN", "false");
        std::env::set_var(
            "SHORT_TOXIC_ORDER_DISCORD_WEBHOOK_URL",
            "https://discord.com/api/webhooks/test-id/test-token",
        );
        let mut confirmed = request(Some(99), Some(99.0));
        confirmed.main_force_score = Some(99);
        confirmed.behavior_type = Some("new_long_build".to_string());
        confirmed.behavior_state = Some("confirmed".to_string());
        confirmed.behavior_confidence = Some(86);
        confirmed.behavior_main_force_confirmed = Some(true);
        let decision = evaluate_discord_alert_gate(&confirmed, DiscordAlertMode::Auto);
        assert!(decision.allowed);
        std::env::remove_var("SHORT_TOXIC_ORDER_DISCORD_WEBHOOK_URL");
        std::env::remove_var("DRY_RUN");
    }

    #[test]
    fn unavailable_null_metric_dtos_deserialize_as_absent() {
        let null_metrics: DiscordNotificationRequest = serde_json::from_value(serde_json::json!({
            "tofMetrics": null,
            "perpTofMetrics": null,
            "advancedTofMetrics": null
        }))
        .expect("top-level null metrics must not reject the request");
        assert!(null_metrics.tof_metrics.is_none());
        assert!(null_metrics.perp_tof_metrics.is_none());
        assert!(null_metrics.advanced_tof_metrics.is_none());

        let partial_tof: DiscordNotificationRequest = serde_json::from_value(serde_json::json!({
            "tofMetrics": {
                "lineage": { "available": true, "alertEligible": true },
                "tradeImbalance": 0.42,
                "depthWithdrawalScore": null,
                "spreadBps": null
            }
        }))
        .expect("partial observed TOF with unavailable L2 must not reject the request");
        assert!(partial_tof.tof_metrics.is_none());

        let request: DiscordNotificationRequest = serde_json::from_value(serde_json::json!({
            "tofMetrics": {
                "lineage": { "available": false },
                "tofScore": null,
                "vpinProxy": null
            },
            "perpTofMetrics": {
                "lineage": { "available": false },
                "riskScore": null
            },
            "advancedTofMetrics": {
                "lineage": { "available": false },
                "advancedScore": null
            }
        }))
        .expect("unavailable metrics must not reject the request");

        assert!(request.tof_metrics.is_none());
        assert!(request.perp_tof_metrics.is_none());
        assert!(request.advanced_tof_metrics.is_none());
    }

    #[test]
    fn sent_auto_status_is_not_downgraded_by_duplicate_poll() {
        let mut tracker = super::DiscordAutoPushTracker::default();
        let sent = super::DiscordAlertPublicStatus {
            auto_eligible: true,
            auto_sent: true,
            last_decision: "sent".to_string(),
            reason: "sent".to_string(),
            sent_at: Some("2026-07-15T00:00:00Z".to_string()),
            manual_sent_at: None,
        };
        tracker.set_status("signal-1", sent.clone());
        tracker.set_status(
            "signal-1",
            super::DiscordAlertPublicStatus {
                auto_eligible: false,
                auto_sent: false,
                last_decision: "skipped".to_string(),
                reason: "duplicate_candidate".to_string(),
                sent_at: None,
                manual_sent_at: None,
            },
        );

        assert_eq!(tracker.status("signal-1"), Some(sent));
    }

    #[test]
    fn auto_tracker_seen_and_status_maps_are_capacity_bounded() {
        let mut tracker = super::DiscordAutoPushTracker::default();
        for index in 0..(super::DISCORD_AUTO_TRACKER_MAX_ENTRIES + 32) {
            let key = format!("signal-{index}");
            assert!(tracker.mark_once(&key));
            tracker.set_status(
                &key,
                super::DiscordAlertPublicStatus {
                    auto_eligible: false,
                    auto_sent: false,
                    last_decision: "skipped".to_string(),
                    reason: "test".to_string(),
                    sent_at: None,
                    manual_sent_at: None,
                },
            );
        }

        assert!(tracker.seen_keys.len() <= super::DISCORD_AUTO_TRACKER_MAX_ENTRIES);
        assert!(tracker.statuses.len() <= super::DISCORD_AUTO_TRACKER_MAX_ENTRIES);
        assert!(!tracker.seen_keys.contains("signal-0"));
        assert!(!tracker.statuses.contains_key("signal-0"));
    }

    fn request(score: Option<u8>, data_quality: Option<f64>) -> DiscordNotificationRequest {
        DiscordNotificationRequest {
            server_evidence_verified: true,
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
            impact_level: None,
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
            behavior_type: None,
            behavior_state: None,
            behavior_confidence: None,
            behavior_main_force_confirmed: None,
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
