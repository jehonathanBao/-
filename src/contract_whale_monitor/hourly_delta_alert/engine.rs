use std::{
    collections::HashSet,
    sync::{
        atomic::{AtomicBool, Ordering},
        Arc,
    },
    time::Duration,
};

use parking_lot::RwLock;
use tokio::sync::mpsc;

use crate::{
    contract_whale_monitor::{LOG_PREFIX, LOG_TARGET},
    normalizers::trade::now_ms,
    storage::{hourly_delta_repo::HourlyDeltaRepo, SqliteStore},
};

use super::{
    calc::compute_hourly_delta,
    collector::{
        fetch_closed_hourly_klines, fetch_closed_kline_by_open_time,
        run_binance_hourly_kline_collector,
    },
    config::HourlyDeltaAlertConfig,
    discord::{notify_hourly_delta_discord, result_from_record, HourlyDeltaDiscordSettings},
    types::{
        ClosedHourlyKline, HourlyDeltaDiscordStatus, HourlyDeltaResult,
        HourlyDeltaRuntimeDiagnostics,
    },
    LOG_EVENTS_PREFIX,
};

#[derive(Clone)]
pub struct HourlyDeltaAlertRuntime {
    config: HourlyDeltaAlertConfig,
    parent_dry_run: bool,
    store: Option<SqliteStore>,
    diagnostics: Arc<RwLock<HourlyDeltaRuntimeDiagnostics>>,
    stop: Arc<AtomicBool>,
}

impl HourlyDeltaAlertRuntime {
    pub fn new(
        config: HourlyDeltaAlertConfig,
        parent_dry_run: bool,
        store: Option<SqliteStore>,
    ) -> Self {
        Self {
            config,
            parent_dry_run,
            store,
            diagnostics: Arc::new(RwLock::new(HourlyDeltaRuntimeDiagnostics::default())),
            stop: Arc::new(AtomicBool::new(false)),
        }
    }

    pub fn diagnostics(&self) -> HourlyDeltaRuntimeDiagnostics {
        self.diagnostics.read().clone()
    }

    pub fn spawn(self) -> Vec<tokio::task::JoinHandle<()>> {
        if !self.config.enabled {
            tracing::info!(
                target: LOG_TARGET,
                event = format!("{LOG_EVENTS_PREFIX}.disabled"),
                "{} hourly_delta_alert disabled",
                LOG_PREFIX
            );
            return Vec::new();
        }

        let (tx, mut rx) = mpsc::channel::<ClosedHourlyKline>(64);
        let mut handles = Vec::new();

        let collector_config = self.config.clone();
        let stop = self.stop.clone();
        handles.push(tokio::spawn(async move {
            tokio::select! {
                _ = run_binance_hourly_kline_collector(collector_config, tx) => {}
                _ = wait_until_stopped(stop) => {}
            }
        }));

        let engine = self.clone();
        handles.push(tokio::spawn(async move {
            engine.run_processor(&mut rx).await;
        }));

        let outbox = self.clone();
        handles.push(tokio::spawn(async move {
            outbox.run_outbox_loop().await;
        }));

        let reconciliation = self.clone();
        handles.push(tokio::spawn(async move {
            reconciliation.run_rest_reconciliation_loop().await;
        }));

        handles
    }

    pub fn stop(&self) {
        self.stop.store(true, Ordering::SeqCst);
    }

    async fn run_processor(&self, rx: &mut mpsc::Receiver<ClosedHourlyKline>) {
        let client = reqwest::Client::new();
        if let Err(error) = self.startup_backfill(&client).await {
            self.record_error(format!("startup_backfill:{error}"));
            tracing::warn!(
                target: LOG_TARGET,
                event = format!("{LOG_EVENTS_PREFIX}.backfill.failed"),
                error = %error,
                "{} hourly_delta startup backfill failed",
                LOG_PREFIX
            );
        }

        let pending_close = Arc::new(tokio::sync::Mutex::new(HashSet::<i64>::new()));
        while !self.stop.load(Ordering::SeqCst) {
            tokio::select! {
                maybe = rx.recv() => {
                    let Some(kline) = maybe else { break; };
                    {
                        let mut diag = self.diagnostics.write();
                        diag.ws_connected = true;
                        diag.last_ws_event_at_ms = Some(now_ms());
                    }
                    if !self.config.matches_stream(&kline.exchange, &kline.symbol, &kline.interval) {
                        continue;
                    }
                    if !kline.is_closed {
                        continue;
                    }
                    {
                        let mut pending = pending_close.lock().await;
                        if !pending.insert(kline.open_time_ms) {
                            continue;
                        }
                    }
                    let open_time = kline.open_time_ms;
                    let grace = Duration::from_secs(self.config.close_grace_seconds);
                    let engine = self.clone();
                    let client = client.clone();
                    let pending_close = pending_close.clone();
                    tokio::spawn(async move {
                        tokio::time::sleep(grace).await;
                        if let Err(error) = engine.finalize_closed_hour(&client, open_time).await {
                            engine.record_error(format!("finalize:{error}"));
                            tracing::warn!(
                                target: LOG_TARGET,
                                event = format!("{LOG_EVENTS_PREFIX}.finalize.failed"),
                                open_time_ms = open_time,
                                error = %error,
                                "{} hourly_delta finalize failed",
                                LOG_PREFIX
                            );
                        }
                        pending_close.lock().await.remove(&open_time);
                    });
                }
                _ = tokio::time::sleep(Duration::from_millis(500)) => {
                    if self.stop.load(Ordering::SeqCst) {
                        break;
                    }
                }
            }
        }
    }

    async fn startup_backfill(&self, client: &reqwest::Client) -> anyhow::Result<()> {
        let ok = self.reconcile_recent_closed_hours(client).await?;
        self.diagnostics.write().backfill_ok += ok;
        tracing::info!(
            target: LOG_TARGET,
            event = format!("{LOG_EVENTS_PREFIX}.backfill.ok"),
            count = ok,
            "{} hourly_delta startup backfill ok",
            LOG_PREFIX
        );
        Ok(())
    }

    async fn reconcile_recent_closed_hours(&self, client: &reqwest::Client) -> anyhow::Result<u64> {
        let limit = self.config.startup_backfill_hours.saturating_add(1);
        let klines = fetch_closed_hourly_klines(client, &self.config, limit).await?;
        let mut ok = 0_u64;
        for kline in klines
            .into_iter()
            .rev()
            .take(self.config.startup_backfill_hours as usize)
        {
            match self.process_closed_kline(kline).await {
                Ok(true) => ok += 1,
                Ok(false) => {}
                Err(error) => {
                    return Err(error);
                }
            }
        }
        Ok(ok)
    }

    async fn run_rest_reconciliation_loop(&self) {
        let client = reqwest::Client::new();
        let initial_grace = Duration::from_secs(self.config.close_grace_seconds);
        if !initial_grace.is_zero() {
            tokio::time::sleep(initial_grace).await;
        }

        let mut interval = tokio::time::interval(Duration::from_millis(
            self.config.rest_reconcile_interval_ms,
        ));
        interval.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Skip);
        while !self.stop.load(Ordering::SeqCst) {
            interval.tick().await;
            if self.stop.load(Ordering::SeqCst) {
                break;
            }
            match self.reconcile_recent_closed_hours(&client).await {
                Ok(processed) if processed > 0 => {
                    tracing::info!(
                        target: LOG_TARGET,
                        event = format!("{LOG_EVENTS_PREFIX}.reconcile.ok"),
                        count = processed,
                        "{} hourly_delta REST reconciliation recovered closed hour(s)",
                        LOG_PREFIX
                    );
                }
                Ok(_) => {}
                Err(error) => {
                    self.diagnostics.write().backfill_fail += 1;
                    self.record_error(format!("reconcile:{error}"));
                    tracing::warn!(
                        target: LOG_TARGET,
                        event = format!("{LOG_EVENTS_PREFIX}.reconcile.failed"),
                        error = %error,
                        "{} hourly_delta REST reconciliation failed",
                        LOG_PREFIX
                    );
                }
            }
        }
    }

    async fn finalize_closed_hour(
        &self,
        client: &reqwest::Client,
        open_time_ms: i64,
    ) -> anyhow::Result<()> {
        let Some(kline) =
            fetch_closed_kline_by_open_time(client, &self.config, open_time_ms).await?
        else {
            anyhow::bail!("closed kline not available yet for open_time={open_time_ms}");
        };
        self.process_closed_kline(kline).await?;
        Ok(())
    }

    async fn process_closed_kline(&self, kline: ClosedHourlyKline) -> anyhow::Result<bool> {
        let Some(result) = compute_hourly_delta(&kline, self.config.threshold_btc) else {
            return Ok(false);
        };
        self.persist_and_maybe_enqueue(result).await
    }

    async fn persist_and_maybe_enqueue(&self, result: HourlyDeltaResult) -> anyhow::Result<bool> {
        let Some(store) = self.store.clone() else {
            tracing::warn!(
                target: LOG_TARGET,
                event = format!("{LOG_EVENTS_PREFIX}.persist.skipped"),
                record_key = result.record_key.as_str(),
                "{} hourly_delta store missing; skip persist",
                LOG_PREFIX
            );
            return Ok(false);
        };

        let now = now_ms();
        let inserted = tokio::task::spawn_blocking({
            let store = store.clone();
            let result = result.clone();
            move || store.upsert_hourly_delta_closed_result(&result, now)
        })
        .await??;

        {
            let mut diag = self.diagnostics.write();
            diag.closed_processed = diag.closed_processed.saturating_add(1);
            diag.last_closed_open_time_ms = Some(result.kline_open_time_ms);
        }

        if !inserted {
            tracing::debug!(
                target: LOG_TARGET,
                event = format!("{LOG_EVENTS_PREFIX}.persist.duplicate"),
                record_key = result.record_key.as_str(),
                "{} hourly_delta already processed",
                LOG_PREFIX
            );
            return Ok(false);
        }

        tracing::info!(
            target: LOG_TARGET,
            event = format!("{LOG_EVENTS_PREFIX}.closed"),
            record_key = result.record_key.as_str(),
            delta_btc = result.delta_btc,
            above_threshold = result.above_threshold,
            direction = result.direction.as_str(),
            "{} hourly_delta closed hour processed",
            LOG_PREFIX
        );

        if result.above_threshold && self.config.discord_enabled {
            let enqueued = tokio::task::spawn_blocking({
                let store = store.clone();
                let key = result.record_key.clone();
                move || store.enqueue_hourly_delta_discord_outbox(&key, now)
            })
            .await??;
            if enqueued {
                self.diagnostics.write().alerts_enqueued += 1;
            }
        }
        Ok(true)
    }

    async fn run_outbox_loop(&self) {
        let mut interval =
            tokio::time::interval(Duration::from_millis(self.config.outbox_poll_interval_ms));
        interval.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Skip);
        while !self.stop.load(Ordering::SeqCst) {
            interval.tick().await;
            if let Err(error) = self.process_outbox_once().await {
                self.record_error(format!("outbox:{error}"));
            }
        }
    }

    async fn process_outbox_once(&self) -> anyhow::Result<()> {
        let Some(store) = self.store.clone() else {
            return Ok(());
        };
        let now = now_ms();
        let batch = self.config.outbox_batch_size;
        let claimed = tokio::task::spawn_blocking({
            let store = store.clone();
            move || store.claim_hourly_delta_discord_outbox(batch, now)
        })
        .await??;

        let settings = HourlyDeltaDiscordSettings::from_config(&self.config, self.parent_dry_run);
        for item in claimed {
            let mut result = result_from_record(&item.record);
            result.threshold_btc = self.config.threshold_btc;
            let outcome = notify_hourly_delta_discord(&settings, &result).await;
            let (status, next_attempt_at, sent_at, last_error) = if outcome.dry_run {
                self.diagnostics.write().discord_dry_run += 1;
                (HourlyDeltaDiscordStatus::DryRun, None, None, None)
            } else if outcome.sent {
                self.diagnostics.write().discord_sent += 1;
                (
                    HourlyDeltaDiscordStatus::Sent,
                    None,
                    outcome.sent_at_ms,
                    None,
                )
            } else if outcome.retryable && item.attempts < self.config.outbox_max_attempts {
                let delay_sec = retry_delay_seconds(
                    item.attempts,
                    self.config.outbox_base_retry_seconds,
                    self.config.outbox_max_retry_seconds,
                );
                (
                    HourlyDeltaDiscordStatus::Retry,
                    Some(now.saturating_add(delay_sec.saturating_mul(1000))),
                    None,
                    Some(outcome.reason.clone()),
                )
            } else {
                (
                    HourlyDeltaDiscordStatus::Dead,
                    None,
                    None,
                    Some(outcome.reason.clone()),
                )
            };

            let record_key = item.record_key.clone();
            tokio::task::spawn_blocking({
                let store = store.clone();
                let last_error = last_error.clone();
                move || {
                    store.finish_hourly_delta_discord_outbox(
                        &record_key,
                        status,
                        next_attempt_at,
                        sent_at,
                        last_error.as_deref(),
                    )
                }
            })
            .await??;
        }
        Ok(())
    }

    fn record_error(&self, error: String) {
        self.diagnostics.write().last_error = Some(error);
    }
}

async fn wait_until_stopped(stop: Arc<AtomicBool>) {
    while !stop.load(Ordering::SeqCst) {
        tokio::time::sleep(Duration::from_millis(200)).await;
    }
}

fn retry_delay_seconds(attempts: usize, base: i64, max: i64) -> i64 {
    let shift = attempts.saturating_sub(1).min(8) as u32;
    base.saturating_mul(2_i64.saturating_pow(shift))
        .min(max)
        .max(1)
}
