use std::{env, sync::Arc};

use parking_lot::RwLock;

use crate::{
    config::{thresholds::AlertGateConfig, AppConfig},
    normalizers::trade::now_ms,
    toxicity::toxic_service::ToxicService,
    types::{
        market::Venue,
        toxic::{ToxicDirection, ToxicEvent, ToxicSeverity, ToxicState},
    },
};

use super::{
    alert_types::AlertState, deduper::AlertDeduper, formatter::format_alert_message,
    sidecar::ToxicFlowSidecarWriter, telegram::TelegramClient,
};

pub trait ToxicStateSource: Send + Sync {
    fn toxic_state(&self) -> ToxicState;
}

impl ToxicStateSource for ToxicService {
    fn toxic_state(&self) -> ToxicState {
        self.get_state()
    }
}

#[derive(Clone)]
pub struct AlertService {
    source: Arc<dyn ToxicStateSource>,
    client: TelegramClient,
    sidecar_writer: ToxicFlowSidecarWriter,
    gate: AlertGateConfig,
    deduper: Arc<RwLock<AlertDeduper>>,
    state: Arc<RwLock<AlertState>>,
    compute_interval_ms: u64,
    task: Arc<RwLock<Option<tokio::task::JoinHandle<()>>>>,
}

#[derive(Debug, Clone)]
pub struct DevTestSidecarAlertInput {
    pub severity: ToxicSeverity,
    pub venue: Venue,
    pub symbol: String,
    pub dedupe_suffix: String,
}

#[derive(Debug, Clone)]
pub struct DevTestSidecarAlertResult {
    pub dedupe_key: String,
    pub deduped: bool,
    pub sidecar_written: bool,
}

impl AlertService {
    pub fn new(source: Arc<dyn ToxicStateSource>, config: &AppConfig) -> Self {
        let gate = AlertGateConfig {
            dedup_window_ms: config.alert_dedup_window_ms,
            min_severity: config.alert_min_severity,
            require_cross_venue: config.alert_require_cross_venue,
            require_markout: config.alert_require_markout,
            require_liquidity_drain: config.alert_require_liquidity_drain,
        };
        Self {
            source,
            client: TelegramClient::new(
                config.telegram_enabled,
                non_empty(config.telegram_bot_token.clone()),
                non_empty(config.telegram_chat_id.clone()),
            ),
            sidecar_writer: ToxicFlowSidecarWriter::new(
                parse_sidecar_enabled(),
                env::var("TOXIC_FLOW_SIDECAR_EVENTS_PATH").ok(),
            ),
            gate,
            deduper: Arc::new(RwLock::new(AlertDeduper::new(config.alert_dedup_window_ms))),
            state: Arc::new(RwLock::new(AlertState::new(config.telegram_enabled))),
            compute_interval_ms: config.toxic_compute_interval_ms,
            task: Arc::new(RwLock::new(None)),
        }
    }

    pub fn with_client(
        source: Arc<dyn ToxicStateSource>,
        client: TelegramClient,
        gate: AlertGateConfig,
        compute_interval_ms: u64,
    ) -> Self {
        Self {
            source,
            state: Arc::new(RwLock::new(AlertState::new(client.enabled()))),
            deduper: Arc::new(RwLock::new(AlertDeduper::new(gate.dedup_window_ms))),
            client,
            sidecar_writer: ToxicFlowSidecarWriter::disabled(),
            gate,
            compute_interval_ms,
            task: Arc::new(RwLock::new(None)),
        }
    }

    pub fn with_client_and_sidecar(
        source: Arc<dyn ToxicStateSource>,
        client: TelegramClient,
        sidecar_writer: ToxicFlowSidecarWriter,
        gate: AlertGateConfig,
        compute_interval_ms: u64,
    ) -> Self {
        Self {
            source,
            state: Arc::new(RwLock::new(AlertState::new(client.enabled()))),
            deduper: Arc::new(RwLock::new(AlertDeduper::new(gate.dedup_window_ms))),
            client,
            sidecar_writer,
            gate,
            compute_interval_ms,
            task: Arc::new(RwLock::new(None)),
        }
    }

    pub fn start(&self) {
        if self.task.read().is_some() {
            return;
        }

        let service = self.clone();
        let handle = tokio::spawn(async move {
            let mut interval = tokio::time::interval(std::time::Duration::from_millis(
                service.compute_interval_ms,
            ));
            loop {
                interval.tick().await;
                let _ = service.process_once(now_ms()).await;
            }
        });
        *self.task.write() = Some(handle);
    }

    pub fn stop(&self) {
        if let Some(handle) = self.task.write().take() {
            handle.abort();
        }
    }

    pub fn get_state(&self) -> AlertState {
        self.state.read().clone()
    }

    pub fn emit_runtime_acceptance_test_alert(
        &self,
        now_ts: i64,
        input: &DevTestSidecarAlertInput,
    ) -> anyhow::Result<DevTestSidecarAlertResult> {
        let mut state = self.state.read().clone();
        state.last_checked_ts = Some(now_ts);

        let dedupe_key = format!(
            "runtime_acceptance_test:{}:{}:{}:{}",
            input.venue.as_key(),
            input.symbol.trim(),
            input.severity.label().to_lowercase(),
            input.dedupe_suffix.trim()
        );
        let can_send = {
            let mut deduper = self.deduper.write();
            deduper.should_send(&dedupe_key, now_ts)
        };

        if !can_send {
            state.suppressed_count = state.suppressed_count.saturating_add(1);
            state.last_suppressed_ts = Some(now_ts);
            *self.state.write() = state;
            return Ok(DevTestSidecarAlertResult {
                dedupe_key,
                deduped: true,
                sidecar_written: false,
            });
        }

        if !self.sidecar_writer.enabled() {
            state.last_error = Some("sidecar_disabled_or_path_missing".to_string());
            *self.state.write() = state;
            anyhow::bail!("sidecar_disabled_or_path_missing");
        }

        self.sidecar_writer.write_runtime_acceptance_test(
            now_ts,
            input.severity,
            input.venue,
            &input.symbol,
            &dedupe_key,
        )?;
        self.deduper.write().mark_sent(&dedupe_key, now_ts);

        state.sent_count = state.sent_count.saturating_add(1);
        state.last_sent_ts = Some(now_ts);
        state.last_error = None;
        *self.state.write() = state;

        Ok(DevTestSidecarAlertResult {
            dedupe_key,
            deduped: false,
            sidecar_written: true,
        })
    }

    pub async fn process_once_for_tests(&self, now_ts: i64) -> AlertState {
        self.process_once(now_ts).await
    }

    async fn process_once(&self, now_ts: i64) -> AlertState {
        let toxic_state = self.source.toxic_state();
        let mut state = self.state.read().clone();
        state.telegram_enabled = self.client.enabled();
        state.last_checked_ts = Some(now_ts);

        let Some(event) = toxic_state.latest_event.clone() else {
            *self.state.write() = state.clone();
            return state;
        };

        if !self.should_attempt_send(&event, &toxic_state) {
            state.suppressed_count = state.suppressed_count.saturating_add(1);
            state.last_suppressed_ts = Some(now_ts);
            *self.state.write() = state.clone();
            return state;
        }

        let key = alert_key(&event);
        let can_send = {
            let mut deduper = self.deduper.write();
            deduper.should_send(&key, now_ts)
        };

        if !can_send {
            state.suppressed_count = state.suppressed_count.saturating_add(1);
            state.last_suppressed_ts = Some(now_ts);
            *self.state.write() = state.clone();
            return state;
        }

        if !self.client.enabled() && !self.sidecar_writer.enabled() {
            *self.state.write() = state.clone();
            return state;
        }

        let message = format_alert_message(&event, &toxic_state);
        let sidecar_result = self
            .sidecar_writer
            .write_alert(&event, &toxic_state, &key, &message);
        let telegram_result = if self.client.enabled() {
            Some(self.client.send_message(&message).await)
        } else {
            None
        };
        let sidecar_delivered = self.sidecar_writer.enabled() && sidecar_result.is_ok();
        let telegram_delivered = telegram_result
            .as_ref()
            .is_some_and(|result| result.as_ref().is_ok());
        let delivered = sidecar_delivered || telegram_delivered;

        // A channel being configured is not the same as a successful delivery.
        // Keep the dedupe key retryable after a transient transport failure.
        if delivered {
            self.deduper.write().mark_sent(&key, now_ts);
        }

        let sidecar_error = sidecar_result.err().map(|err| err.to_string());
        let telegram_error =
            telegram_result.and_then(|result| result.err().map(|err| err.to_string()));

        if delivered {
            state.sent_count = state.sent_count.saturating_add(1);
            state.last_sent_ts = Some(now_ts);
            state.last_error = telegram_error
                .map(|err| format!("telegram: {err}"))
                .or_else(|| sidecar_error.map(|err| format!("sidecar: {err}")));
        } else if let Some(err) = telegram_error {
            state.last_error = Some(err);
        } else if let Some(err) = sidecar_error {
            state.last_error = Some(format!("sidecar: {err}"));
        }

        *self.state.write() = state.clone();
        state
    }

    fn should_attempt_send(&self, event: &ToxicEvent, state: &ToxicState) -> bool {
        // External alerts are fail-closed while the authoritative state is stale
        // or the flow window has not been rebuilt after a data interruption.
        if !state.quality.has_flow || !state.quality.stale_venues.is_empty() {
            return false;
        }
        if event.direction == ToxicDirection::Neutral {
            return false;
        }
        if !event.severity.is_at_least(self.gate.min_severity) {
            return false;
        }
        if event.toxic_volume_btc < event.threshold_btc {
            return false;
        }

        if self.gate.require_cross_venue
            && event.severity != ToxicSeverity::Extreme
            && !event.cross_venue_confirmed
        {
            return false;
        }

        if self.gate.require_markout && !markout_confirmed(event) {
            return false;
        }

        if self.gate.require_liquidity_drain && !event.liquidity_thin {
            return false;
        }

        true
    }
}

pub fn alert_key(event: &ToxicEvent) -> String {
    let direction = match event.direction {
        ToxicDirection::Buy => "buy",
        ToxicDirection::Sell => "sell",
        ToxicDirection::Neutral => "neutral",
    };
    format!(
        "{}:{}:{}:{}:{}",
        event.symbol,
        direction,
        event.window_ms,
        event
            .leader_venue
            .map(|venue| venue.as_key())
            .unwrap_or("unknown"),
        event.severity.label()
    )
}

fn markout_confirmed(event: &ToxicEvent) -> bool {
    event.markout_1s_bps.is_some_and(|bps| bps > 1.0)
        || event.markout_5s_bps.is_some_and(|bps| bps > 3.0)
}

fn non_empty(value: String) -> Option<String> {
    let trimmed = value.trim().to_string();
    (!trimmed.is_empty()).then_some(trimmed)
}

fn parse_sidecar_enabled() -> bool {
    env::var("TOXIC_FLOW_SIDECAR_ENABLED")
        .ok()
        .is_some_and(|value| value.eq_ignore_ascii_case("true"))
}
