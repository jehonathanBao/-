use std::time::Duration;

use chrono::{FixedOffset, TimeZone};
use serde_json::Value;
use url::Url;

use crate::normalizers::trade::now_ms;

use super::{
    config::HourlyDeltaAlertConfig,
    types::{HourlyDeltaAlertRecord, HourlyDeltaDirection, HourlyDeltaResult},
    LOG_EVENTS_PREFIX,
};
use crate::contract_whale_monitor::{LOG_PREFIX, LOG_TARGET};

const DEFAULT_TIMEOUT_MS: u64 = 5_000;

#[derive(Debug, Clone)]
pub struct HourlyDeltaDiscordSettings {
    pub enabled: bool,
    pub dry_run: bool,
    pub webhook_url: Option<String>,
    pub timeout_ms: u64,
    pub max_attempts: usize,
}

impl HourlyDeltaDiscordSettings {
    pub fn from_config(config: &HourlyDeltaAlertConfig, parent_dry_run: bool) -> Self {
        Self {
            enabled: config.discord_enabled,
            dry_run: config.effective_dry_run(parent_dry_run),
            webhook_url: std::env::var("HOURLY_DELTA_DISCORD_WEBHOOK_URL")
                .ok()
                .filter(|value| !value.trim().is_empty())
                .or_else(|| {
                    std::env::var("CONTRACT_WHALE_DISCORD_WEBHOOK_URL")
                        .ok()
                        .filter(|value| !value.trim().is_empty())
                })
                .or_else(|| {
                    std::env::var("DISCORD_WEBHOOK_URL")
                        .ok()
                        .filter(|value| !value.trim().is_empty())
                }),
            timeout_ms: std::env::var("HOURLY_DELTA_DISCORD_TIMEOUT_MS")
                .ok()
                .and_then(|value| value.parse().ok())
                .unwrap_or(DEFAULT_TIMEOUT_MS),
            max_attempts: config.outbox_max_attempts.clamp(1, 6),
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct HourlyDeltaDiscordOutcome {
    pub eligible: bool,
    pub sent: bool,
    pub dry_run: bool,
    pub reason: String,
    pub sent_at_ms: Option<i64>,
    pub payload: Option<Value>,
    pub retryable: bool,
}

pub fn build_hourly_delta_discord_content(result: &HourlyDeltaResult) -> String {
    let summary = presentation_summary(result);
    let (buy_share, sell_share) = flow_shares(result);
    let period = format_utc8_period(result.kline_open_time_ms, result.kline_close_time_ms);
    let delta_label = format_signed_btc(result.delta_btc);
    format!(
        "{} {} | BTC 1H {}\n\n\
{}：{} BTC\n\
卖出占比：{}  |  买入占比：{}\n\n\
方向强度\n\
SELL  {} {}\n\
BUY   {} {}\n\n\
主动卖出：{} BTC\n\
主动买入：{} BTC\n\
净差 Delta：{} BTC\n\
总成交量：{} BTC\n\
周期：{period}（UTC+8）\n\
触发阈值：|Delta| > {} BTC\n\
数据源：Binance {} 永续\n\
状态：1H 已收线",
        summary.emoji,
        summary.bias,
        summary.title,
        summary.hero_label,
        format_btc(summary.hero_amount),
        format_share(sell_share),
        format_share(buy_share),
        direction_bar(sell_share),
        format_share(sell_share),
        direction_bar(buy_share),
        format_share(buy_share),
        format_btc(result.taker_sell_btc),
        format_btc(result.taker_buy_btc),
        delta_label,
        format_btc(result.volume_btc),
        format_btc(result.threshold_btc),
        result.symbol
    )
}

pub fn build_hourly_delta_discord_payload(result: &HourlyDeltaResult) -> Value {
    let content = build_hourly_delta_discord_content(result);
    let summary = presentation_summary(result);
    let (buy_share, sell_share) = flow_shares(result);
    let color = match (result.above_threshold, result.direction) {
        (true, HourlyDeltaDirection::NetBuy) => 0x22_C5_5E,
        (true, HourlyDeltaDirection::NetSell) => 0xEF_44_44,
        _ => 0x9C_A3_AF,
    };
    serde_json::json!({
        "content": content,
        "embeds": [{
            "title": format!("{} {} · BTC 1H 主动成交", summary.emoji, summary.bias),
            "description": format!("{}：{} BTC\n主动成交净差报警（非资金净流入/非持仓变化/非主力买卖判定）", summary.hero_label, format_btc(summary.hero_amount)),
            "color": color,
            "fields": [
                {"name": "方向", "value": format!("{} / {}", summary.bias, summary.code), "inline": true},
                {"name": "净差 Delta", "value": format!("{} BTC", format_signed_btc(result.delta_btc)), "inline": true},
                {"name": summary.hero_label, "value": format!("{} BTC", format_btc(summary.hero_amount)), "inline": true},
                {"name": "卖出占比", "value": format_share(sell_share), "inline": true},
                {"name": "买入占比", "value": format_share(buy_share), "inline": true},
                {"name": "方向强度", "value": format!("SELL  {} {}\nBUY   {} {}", direction_bar(sell_share), format_share(sell_share), direction_bar(buy_share), format_share(buy_share)), "inline": false},
                {"name": "主动卖出", "value": format!("{} BTC", format_btc(result.taker_sell_btc)), "inline": true},
                {"name": "主动买入", "value": format!("{} BTC", format_btc(result.taker_buy_btc)), "inline": true},
                {"name": "总成交量", "value": format!("{} BTC", format_btc(result.volume_btc)), "inline": true},
                {"name": "周期 (UTC+8)", "value": format_utc8_period(result.kline_open_time_ms, result.kline_close_time_ms), "inline": false},
                {"name": "阈值", "value": format!("|Delta| > {} BTC", format_btc(result.threshold_btc)), "inline": true},
                {"name": "数据源", "value": format!("Binance {} 永续", result.symbol), "inline": true},
                {"name": "状态", "value": "1H 已收线", "inline": true}
            ],
            "footer": {"text": format!("record={}", result.record_key)}
        }]
    })
}

#[derive(Debug, Clone, Copy)]
struct PresentationSummary {
    emoji: &'static str,
    bias: &'static str,
    title: &'static str,
    code: &'static str,
    hero_label: &'static str,
    hero_amount: f64,
}

fn presentation_summary(result: &HourlyDeltaResult) -> PresentationSummary {
    if !result.above_threshold {
        return PresentationSummary {
            emoji: "⚪",
            bias: "观察",
            title: "主动成交净差",
            code: "FLAT",
            hero_label: "净差",
            hero_amount: result.delta_btc.abs(),
        };
    }
    match result.direction {
        HourlyDeltaDirection::NetBuy => PresentationSummary {
            emoji: "🟢",
            bias: "偏多",
            title: "主动成交净买入",
            code: "BUY",
            hero_label: "净买入",
            hero_amount: result.delta_btc.abs(),
        },
        HourlyDeltaDirection::NetSell => PresentationSummary {
            emoji: "🔴",
            bias: "偏空",
            title: "主动成交净卖出",
            code: "SELL",
            hero_label: "净卖出",
            hero_amount: result.delta_btc.abs(),
        },
        HourlyDeltaDirection::Flat => PresentationSummary {
            emoji: "⚪",
            bias: "观察",
            title: "主动成交净差",
            code: "FLAT",
            hero_label: "净差",
            hero_amount: result.delta_btc.abs(),
        },
    }
}

fn flow_shares(result: &HourlyDeltaResult) -> (f64, f64) {
    let total = if result.volume_btc.is_finite() && result.volume_btc > 0.0 {
        result.volume_btc
    } else {
        result.taker_buy_btc.max(0.0) + result.taker_sell_btc.max(0.0)
    };
    if !total.is_finite() || total <= f64::EPSILON {
        return (0.0, 0.0);
    }
    (
        safe_share(result.taker_buy_btc, total),
        safe_share(result.taker_sell_btc, total),
    )
}

fn safe_share(value: f64, total: f64) -> f64 {
    if value.is_finite() {
        (value.max(0.0) / total * 100.0).clamp(0.0, 100.0)
    } else {
        0.0
    }
}

fn format_share(value: f64) -> String {
    format!("{value:.1}%")
}

fn direction_bar(share: f64) -> String {
    const WIDTH: usize = 20;
    let filled = ((share.clamp(0.0, 100.0) / 100.0) * WIDTH as f64).round() as usize;
    format!("{}{}", "█".repeat(filled), "░".repeat(WIDTH - filled))
}

pub fn result_from_record(record: &HourlyDeltaAlertRecord) -> HourlyDeltaResult {
    HourlyDeltaResult {
        record_key: record.record_key.clone(),
        exchange: record.exchange.clone(),
        symbol: record.symbol.clone(),
        interval: record.interval.clone(),
        kline_open_time_ms: record.kline_open_time_ms,
        kline_close_time_ms: record.kline_close_time_ms,
        taker_buy_btc: record.taker_buy_btc,
        taker_sell_btc: record.taker_sell_btc,
        delta_btc: record.delta_btc,
        volume_btc: record.volume_btc,
        direction: record.direction,
        above_threshold: record.above_threshold,
        threshold_btc: 1000.0,
        data_status: record.data_status,
    }
}

pub async fn notify_hourly_delta_discord(
    settings: &HourlyDeltaDiscordSettings,
    result: &HourlyDeltaResult,
) -> HourlyDeltaDiscordOutcome {
    let payload = build_hourly_delta_discord_payload(result);
    if !settings.enabled {
        return outcome(false, false, false, "disabled", None, payload, false);
    }
    if !result.above_threshold {
        return outcome(false, false, false, "below_threshold", None, payload, false);
    }
    if settings.dry_run {
        tracing::info!(
            target: LOG_TARGET,
            event = format!("{LOG_EVENTS_PREFIX}.discord.would_send"),
            record_key = result.record_key.as_str(),
            delta_btc = result.delta_btc,
            message = %build_hourly_delta_discord_content(result),
            "{} hourly_delta discord would_send",
            LOG_PREFIX
        );
        return outcome(true, false, true, "dry_run", None, payload, false);
    }

    let Some(webhook_url) = settings.webhook_url.as_deref() else {
        return outcome(true, false, false, "webhook_missing", None, payload, true);
    };
    if let Err(reason) = validate_discord_webhook_url(webhook_url) {
        return outcome(true, false, false, &reason, None, payload, false);
    }

    let client = match reqwest::Client::builder()
        .timeout(Duration::from_millis(settings.timeout_ms))
        .build()
    {
        Ok(client) => client,
        Err(_) => {
            return outcome(
                true,
                false,
                false,
                "client_build_failed",
                None,
                payload,
                true,
            )
        }
    };

    for attempt in 1..=settings.max_attempts {
        match client.post(webhook_url).json(&payload).send().await {
            Ok(response) if response.status().is_success() => {
                let sent_at_ms = now_ms();
                tracing::info!(
                    target: LOG_TARGET,
                    event = format!("{LOG_EVENTS_PREFIX}.discord.sent"),
                    record_key = result.record_key.as_str(),
                    status = response.status().as_u16(),
                    "{} hourly_delta discord sent",
                    LOG_PREFIX
                );
                return outcome(true, true, false, "sent", Some(sent_at_ms), payload, false);
            }
            Ok(response) => {
                let status = response.status();
                let retryable =
                    status.as_u16() == 429 || status.as_u16() == 408 || status.is_server_error();
                if !retryable || attempt >= settings.max_attempts {
                    return outcome(
                        true,
                        false,
                        false,
                        &format!("http_{}", status.as_u16()),
                        None,
                        payload,
                        retryable,
                    );
                }
                tokio::time::sleep(Duration::from_millis(500 * attempt as u64)).await;
            }
            Err(error) => {
                if attempt >= settings.max_attempts {
                    return outcome(
                        true,
                        false,
                        false,
                        &format!("transport:{error}"),
                        None,
                        payload,
                        true,
                    );
                }
                tokio::time::sleep(Duration::from_millis(500 * attempt as u64)).await;
            }
        }
    }

    outcome(true, false, false, "max_attempts", None, payload, true)
}

pub fn validate_discord_webhook_url(raw: &str) -> Result<(), String> {
    let url = Url::parse(raw).map_err(|_| "webhook_invalid".to_string())?;
    if url.scheme() != "https" {
        return Err("webhook_invalid".to_string());
    }
    let host = url.host_str().unwrap_or_default();
    if !(host == "discord.com" || host == "discordapp.com" || host.ends_with(".discord.com")) {
        return Err("webhook_invalid".to_string());
    }
    if !url.path().contains("/api/webhooks/") {
        return Err("webhook_invalid".to_string());
    }
    Ok(())
}

pub fn format_utc8_period(open_ms: i64, close_ms: i64) -> String {
    let offset = FixedOffset::east_opt(8 * 3600).expect("utc+8");
    let open = offset.timestamp_millis_opt(open_ms).single();
    // close_time from Binance is inclusive end of candle; display as next hour start.
    let end_ms = close_ms.saturating_add(1);
    let close = offset.timestamp_millis_opt(end_ms).single();
    match (open, close) {
        (Some(o), Some(c)) => format!("{}–{}", o.format("%H:%M"), c.format("%H:%M")),
        _ => format!("{open_ms}–{close_ms}"),
    }
}

fn format_btc(value: f64) -> String {
    let rounded = (value * 100.0).round() / 100.0;
    if (rounded - rounded.round()).abs() < 1e-9 {
        format_with_commas(rounded.round() as i64)
    } else {
        let whole = rounded.trunc() as i64;
        let frac = ((rounded.fract().abs() * 100.0).round() as i64).clamp(0, 99);
        format!("{}.{:02}", format_with_commas(whole), frac)
    }
}

fn format_signed_btc(value: f64) -> String {
    if value > 0.0 {
        format!("+{}", format_btc(value))
    } else if value < 0.0 {
        format!("-{}", format_btc(value.abs()))
    } else {
        "0".to_string()
    }
}

fn format_with_commas(value: i64) -> String {
    let sign = if value < 0 { "-" } else { "" };
    let digits = value.unsigned_abs().to_string();
    let mut out = String::new();
    for (idx, ch) in digits.chars().rev().enumerate() {
        if idx > 0 && idx % 3 == 0 {
            out.push(',');
        }
        out.push(ch);
    }
    let body: String = out.chars().rev().collect();
    format!("{sign}{body}")
}

fn outcome(
    eligible: bool,
    sent: bool,
    dry_run: bool,
    reason: &str,
    sent_at_ms: Option<i64>,
    payload: Value,
    retryable: bool,
) -> HourlyDeltaDiscordOutcome {
    HourlyDeltaDiscordOutcome {
        eligible,
        sent,
        dry_run,
        reason: reason.to_string(),
        sent_at_ms,
        payload: Some(payload),
        retryable,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::contract_whale_monitor::hourly_delta_alert::types::HourlyDeltaDataStatus;

    fn sample_result(delta: f64) -> HourlyDeltaResult {
        let buy = (7_700.0 + delta) / 2.0;
        HourlyDeltaResult {
            record_key: "binance:BTCUSDT:1h:1700000000000".into(),
            exchange: "binance".into(),
            symbol: "BTCUSDT".into(),
            interval: "1h".into(),
            kline_open_time_ms: 1_704_067_200_000, // 2024-01-01 08:00 UTC+8
            kline_close_time_ms: 1_704_070_799_999,
            taker_buy_btc: buy,
            taker_sell_btc: 7_700.0 - buy,
            delta_btc: delta,
            volume_btc: 7_700.0,
            direction: if delta > 0.0 {
                HourlyDeltaDirection::NetBuy
            } else if delta < 0.0 {
                HourlyDeltaDirection::NetSell
            } else {
                HourlyDeltaDirection::Flat
            },
            above_threshold: delta.abs() > 1000.0,
            threshold_btc: 1000.0,
            data_status: HourlyDeltaDataStatus::Closed,
        }
    }

    #[test]
    fn message_uses_active_trade_wording() {
        let content = build_hourly_delta_discord_content(&sample_result(-2800.0));
        assert!(content.contains("主动成交净卖出"));
        assert!(content.contains("净差：-2,800 BTC"));
        assert!(!content.contains("资金净流入"));
        assert!(!content.contains("主力买卖"));
        assert!(!content.contains("持仓增加"));
    }

    #[test]
    fn dry_run_does_not_require_webhook() {
        let settings = HourlyDeltaDiscordSettings {
            enabled: true,
            dry_run: true,
            webhook_url: None,
            timeout_ms: 1000,
            max_attempts: 1,
        };
        let runtime = tokio::runtime::Runtime::new().unwrap();
        let outcome = runtime.block_on(notify_hourly_delta_discord(
            &settings,
            &sample_result(-2800.0),
        ));
        assert!(outcome.eligible);
        assert!(outcome.dry_run);
        assert!(!outcome.sent);
        assert_eq!(outcome.reason, "dry_run");
    }
}
