use std::time::Duration;

use futures_util::StreamExt;
use serde_json::Value;
use tokio::sync::mpsc;
use tokio_tungstenite::connect_async;

use super::{
    calc::{parse_binance_kline_ws_message, parse_binance_rest_klines},
    config::HourlyDeltaAlertConfig,
    types::ClosedHourlyKline,
    LOG_EVENTS_PREFIX,
};
use crate::contract_whale_monitor::{LOG_PREFIX, LOG_TARGET};

pub const BINANCE_BTC_USDT_PERP_KLINE_1H_STREAM: &str =
    "wss://fstream.binance.com/ws/btcusdt@kline_1h";
pub const BINANCE_FUTURES_KLINES_URL: &str = "https://fapi.binance.com/fapi/v1/klines";
const RECONNECT_MAX_DELAY_MS: u64 = 30_000;

pub async fn run_binance_hourly_kline_collector(
    config: HourlyDeltaAlertConfig,
    sender: mpsc::Sender<ClosedHourlyKline>,
) {
    let mut reconnect_attempt = 0_u32;
    loop {
        tracing::info!(
            target: LOG_TARGET,
            event = format!("{LOG_EVENTS_PREFIX}.ws.connecting"),
            "{} connecting binance 1h kline stream",
            LOG_PREFIX
        );
        match connect_async(BINANCE_BTC_USDT_PERP_KLINE_1H_STREAM).await {
            Ok((ws, _)) => {
                reconnect_attempt = 0;
                tracing::info!(
                    target: LOG_TARGET,
                    event = format!("{LOG_EVENTS_PREFIX}.ws.connected"),
                    "{} binance 1h kline stream connected",
                    LOG_PREFIX
                );
                let (_, mut read) = ws.split();
                while let Some(message) = read.next().await {
                    match message {
                        Ok(message) => {
                            if let Ok(text) = message.to_text() {
                                match parse_binance_kline_ws_message(text, &config.exchange) {
                                    Ok(Some(kline))
                                        if config.matches_stream(
                                            &kline.exchange,
                                            &kline.symbol,
                                            &kline.interval,
                                        ) =>
                                    {
                                        if sender.send(kline).await.is_err() {
                                            tracing::warn!(
                                                target: LOG_TARGET,
                                                event = format!("{LOG_EVENTS_PREFIX}.ws.disconnected"),
                                                "{} hourly kline receiver dropped",
                                                LOG_PREFIX
                                            );
                                            return;
                                        }
                                    }
                                    Ok(_) => {}
                                    Err(error) => {
                                        tracing::warn!(
                                            target: LOG_TARGET,
                                            event = format!("{LOG_EVENTS_PREFIX}.parse_error"),
                                            error = %error,
                                            "{} failed to parse hourly kline ws message",
                                            LOG_PREFIX
                                        );
                                    }
                                }
                            }
                        }
                        Err(error) => {
                            tracing::warn!(
                                target: LOG_TARGET,
                                event = format!("{LOG_EVENTS_PREFIX}.ws.disconnected"),
                                error = %error,
                                "{} binance 1h kline stream disconnected",
                                LOG_PREFIX
                            );
                            break;
                        }
                    }
                }
            }
            Err(error) => {
                tracing::warn!(
                    target: LOG_TARGET,
                    event = format!("{LOG_EVENTS_PREFIX}.ws.connect_failed"),
                    error = %error,
                    "{} binance 1h kline connect failed",
                    LOG_PREFIX
                );
            }
        }

        reconnect_attempt = reconnect_attempt.saturating_add(1);
        let delay = reconnect_delay_ms(reconnect_attempt);
        tokio::time::sleep(Duration::from_millis(delay)).await;
    }
}

pub async fn fetch_closed_hourly_klines(
    client: &reqwest::Client,
    config: &HourlyDeltaAlertConfig,
    limit: u32,
) -> anyhow::Result<Vec<ClosedHourlyKline>> {
    let limit = limit.clamp(1, 24);
    let url = format!(
        "{BINANCE_FUTURES_KLINES_URL}?symbol={}&interval={}&limit={}",
        config.symbol, config.interval, limit
    );
    let mut last_error = None;
    for attempt in 1..=config.rest_retry_max {
        match client.get(&url).send().await {
            Ok(response) if response.status().is_success() => {
                let body: Value = response.json().await?;
                let mut klines = parse_binance_rest_klines(
                    &body,
                    &config.exchange,
                    &config.symbol,
                    &config.interval,
                )?;
                // REST returns completed candles only for past hours; drop the still-open last candle
                // by comparing close_time against now when possible.
                let now_ms = crate::normalizers::trade::now_ms();
                klines.retain(|k| k.close_time_ms < now_ms);
                return Ok(klines);
            }
            Ok(response) => {
                last_error = Some(anyhow::anyhow!("http {}", response.status()));
            }
            Err(error) => {
                last_error = Some(anyhow::anyhow!(error));
            }
        }
        let backoff = config
            .rest_retry_base_ms
            .saturating_mul(attempt as u64)
            .min(10_000);
        tokio::time::sleep(Duration::from_millis(backoff)).await;
    }
    Err(last_error.unwrap_or_else(|| anyhow::anyhow!("rest klines failed")))
}

pub async fn fetch_closed_kline_by_open_time(
    client: &reqwest::Client,
    config: &HourlyDeltaAlertConfig,
    open_time_ms: i64,
) -> anyhow::Result<Option<ClosedHourlyKline>> {
    let url = format!(
        "{BINANCE_FUTURES_KLINES_URL}?symbol={}&interval={}&startTime={}&limit=1",
        config.symbol, config.interval, open_time_ms
    );
    let mut last_error = None;
    for attempt in 1..=config.rest_retry_max {
        match client.get(&url).send().await {
            Ok(response) if response.status().is_success() => {
                let body: Value = response.json().await?;
                let klines = parse_binance_rest_klines(
                    &body,
                    &config.exchange,
                    &config.symbol,
                    &config.interval,
                )?;
                let now_ms = crate::normalizers::trade::now_ms();
                return Ok(klines
                    .into_iter()
                    .find(|k| k.open_time_ms == open_time_ms && k.close_time_ms < now_ms));
            }
            Ok(response) => {
                last_error = Some(anyhow::anyhow!("http {}", response.status()));
            }
            Err(error) => {
                last_error = Some(anyhow::anyhow!(error));
            }
        }
        let backoff = config
            .rest_retry_base_ms
            .saturating_mul(attempt as u64)
            .min(10_000);
        tokio::time::sleep(Duration::from_millis(backoff)).await;
    }
    Err(last_error.unwrap_or_else(|| anyhow::anyhow!("rest kline by open_time failed")))
}

fn reconnect_delay_ms(attempt: u32) -> u64 {
    let base = 1_000_u64.saturating_mul(2_u64.saturating_pow(attempt.saturating_sub(1).min(5)));
    base.min(RECONNECT_MAX_DELAY_MS)
}
