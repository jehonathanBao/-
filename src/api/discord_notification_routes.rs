use std::{
    collections::{HashMap, VecDeque},
    sync::{Mutex, OnceLock},
    time::{Duration, Instant},
};

use crate::alerts::discord_message_builder::{
    DiscordEmbed, DiscordEmbedField, DiscordEmbedFooter, DiscordWebhookPayload,
};
use crate::app::AppState;
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

pub async fn discord_notification_proxy(
    State(state): State<AppState>,
    Json(body): Json<DiscordNotificationRequest>,
) -> impl IntoResponse {
    let gate = AlertGate::from_env();
    let is_test = body.test.unwrap_or(false);
    if !is_test && !gate.allows(&body) {
        record_discord_log(
            &state,
            "warn",
            "discord_push_skipped",
            "Discord push skipped: alert gate rejected",
            &body,
        );
        return (
            StatusCode::OK,
            Json(DiscordNotificationResponse {
                ok: false,
                configured: discord_webhook_url().is_some(),
                reason: "ALERT_GATE_REJECTED",
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
        "discord_push_queued",
        "Discord push queued for alert-only delivery",
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
            record_discord_log(
                &state,
                "info",
                "discord_push_sent",
                "Discord push sent successfully",
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

    fn allows(self, signal: &DiscordNotificationRequest) -> bool {
        severity_allows(signal.level.as_deref())
            && signal.score.unwrap_or(0) >= self.min_score
            && signal.data_quality.unwrap_or(0.0) >= self.min_data_quality
    }
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

#[cfg(test)]
mod tests {
    use std::sync::{Mutex, OnceLock};

    use super::{
        discord_payload, validate_discord_webhook_url, AlertGate, DiscordNotificationRequest,
        DiscordPushLimiter,
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
        let gate = AlertGate::from_env();

        assert!(gate.allows(&request(Some(80), Some(70.0))));
        assert!(!gate.allows(&request(Some(79), Some(90.0))));
        assert!(!gate.allows(&request(Some(90), Some(69.0))));
        assert!(!gate.allows(&request(None, Some(90.0))));

        let mut medium = request(Some(90), Some(90.0));
        medium.level = Some("medium".to_string());
        assert!(!gate.allows(&medium));

        std::env::remove_var("ALERT_MIN_SCORE");
        std::env::remove_var("ALERT_MIN_DATA_QUALITY");
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
            test: None,
        }
    }
}
