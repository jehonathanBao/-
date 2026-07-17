use std::{collections::BTreeMap, sync::Arc};

use parking_lot::RwLock;

use crate::{
    config::{thresholds::ToxicVolumeParams, AppConfig},
    market_data::flow_window_service::FlowWindowService,
    normalizers::trade::now_ms,
    regime_thresholds::{RegimeAwareProvider, RegimeThresholdManager},
    storage::{sqlite::SqliteStore, toxic_events_repo::ToxicEventsRepo},
    toxicity::{
        liquidation_service::LiquidationService, markout_service::MarkoutService,
        sweep_service::SweepService, toxic_volume_engine::ToxicVolumeEngine,
        vpin_service::VpinService,
    },
    types::{
        flow::{empty_venue_breakdown, DataQuality, FlowWindow},
        toxic::{ToxicDirection, ToxicEvent, ToxicQuality, ToxicState, ToxicWindowMs},
    },
};

#[derive(Clone)]
pub struct ToxicService {
    flow_service: FlowWindowService,
    markout_service: MarkoutService,
    sweep_service: SweepService,
    vpin_service: VpinService,
    liquidation_service: LiquidationService,
    base_params: ToxicVolumeParams,
    regime_manager: Arc<RegimeThresholdManager>,
    windows_ms: Vec<ToxicWindowMs>,
    compute_interval_ms: u64,
    recent_event_limit: usize,
    latest_state: Arc<RwLock<ToxicState>>,
    recent_events: Arc<RwLock<Vec<ToxicEvent>>>,
    store: Option<SqliteStore>,
    seen_event_ids: Arc<RwLock<std::collections::BTreeSet<String>>>,
    task: Arc<RwLock<Option<tokio::task::JoinHandle<()>>>>,
}

impl ToxicService {
    pub fn new(
        flow_service: FlowWindowService,
        markout_service: MarkoutService,
        sweep_service: SweepService,
        vpin_service: VpinService,
        liquidation_service: LiquidationService,
        config: &AppConfig,
    ) -> Self {
        Self::new_with_regime(
            flow_service,
            markout_service,
            sweep_service,
            vpin_service,
            liquidation_service,
            config,
            Arc::new(RegimeThresholdManager::from_runtime_config()),
        )
    }

    pub fn new_with_regime(
        flow_service: FlowWindowService,
        markout_service: MarkoutService,
        sweep_service: SweepService,
        vpin_service: VpinService,
        liquidation_service: LiquidationService,
        config: &AppConfig,
        regime_manager: Arc<RegimeThresholdManager>,
    ) -> Self {
        let mut params = ToxicVolumeParams {
            threshold_btc: config.toxic_volume_alert_btc,
            ..ToxicVolumeParams::default()
        };
        if params.threshold_btc <= 0.0 {
            params.threshold_btc = ToxicVolumeParams::default().threshold_btc;
        }

        let windows_ms = config.windows_ms.clone();
        let latest_state = Arc::new(RwLock::new(empty_state(
            &windows_ms,
            now_ms(),
            params.threshold_btc,
        )));
        let store = if config.sqlite_enabled {
            SqliteStore::open(&config.sqlite_path)
                .and_then(|store| {
                    store.migrate()?;
                    Ok(store)
                })
                .ok()
        } else {
            None
        };

        Self {
            flow_service,
            markout_service,
            sweep_service,
            vpin_service,
            liquidation_service,
            base_params: params.clone(),
            regime_manager,
            windows_ms,
            compute_interval_ms: config.toxic_compute_interval_ms,
            recent_event_limit: params.recent_event_limit,
            latest_state,
            recent_events: Arc::new(RwLock::new(Vec::new())),
            store,
            seen_event_ids: Arc::new(RwLock::new(std::collections::BTreeSet::new())),
            task: Arc::new(RwLock::new(None)),
        }
    }

    pub fn regime_context(&self) -> crate::types::regime::RegimeContext {
        self.regime_manager.current()
    }

    pub fn get_current_params(&self) -> ToxicVolumeParams {
        self.regime_manager
            .adjusted_toxic_volume_params(&self.base_params)
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
                service.compute_once(now_ms());
            }
        });
        *self.task.write() = Some(handle);
    }

    pub fn stop(&self) {
        if let Some(handle) = self.task.write().take() {
            handle.abort();
        }
    }

    pub fn get_state(&self) -> ToxicState {
        self.latest_state.read().clone()
    }

    pub fn compute_once_for_tests(&self, now_ts: i64) -> ToxicState {
        self.compute_once(now_ts)
    }

    pub fn list_recent_events(&self, limit: usize) -> anyhow::Result<Vec<ToxicEvent>> {
        if let Some(store) = &self.store {
            return store.list_recent_events(limit);
        }
        Ok(self
            .recent_events
            .read()
            .iter()
            .rev()
            .take(limit)
            .cloned()
            .collect())
    }

    pub fn get_latest_event(&self) -> anyhow::Result<Option<ToxicEvent>> {
        if let Some(store) = &self.store {
            return store.get_latest_event();
        }
        Ok(self.recent_events.read().last().cloned())
    }

    pub fn store(&self) -> Option<SqliteStore> {
        self.store.clone()
    }

    fn compute_once(&self, now_ts: i64) -> ToxicState {
        let adjusted_params = self.get_current_params();
        let engine = ToxicVolumeEngine::new(adjusted_params.clone());
        let flow_state = self.flow_service.get_latest_flow_state();
        let markout_state = self.markout_service.get_state();
        let sweep_state = self.sweep_service.get_state();
        let vpin_state = self.vpin_service.get_state();
        let liquidation_state = self.liquidation_service.get_state();
        let mut results = BTreeMap::new();
        let confidence_ok = self.regime_manager.passes_confidence_gate();

        for window_ms in &self.windows_ms {
            let flow_window = flow_state
                .windows
                .get(&window_ms.to_string())
                .cloned()
                .unwrap_or_else(|| empty_flow_window(*window_ms, now_ts));
            let result = engine.compute_window(
                &flow_window,
                &markout_state,
                &sweep_state,
                &vpin_state,
                &liquidation_state,
            );
            if confidence_ok {
                if let Some(event) = engine.build_event_if_triggered(&result) {
                    self.push_event(event);
                }
            }
            results.insert(window_ms.to_string(), result);
        }

        let recent_events = self.recent_events.read().clone();
        let latest_event = recent_events.last().cloned();
        let state = ToxicState {
            symbol: flow_state.symbol.clone(),
            updated_at: now_ts,
            threshold_btc: adjusted_params.threshold_btc,
            windows_ms: self.windows_ms.clone(),
            results,
            latest_event,
            recent_events,
            quality: ToxicQuality {
                has_flow: flow_state
                    .windows
                    .values()
                    .any(|window| window.trade_count > 0),
                has_markout: markout_state.quality.pending_samples > 0
                    || markout_state.quality.resolved_samples > 0,
                has_sweep: sweep_state
                    .results
                    .values()
                    .any(|result| result.sweep_detected),
                has_liquidation: liquidation_state.metrics.current_mid.is_some(),
                liquidation: Some(liquidation_state.metrics.clone()),
                active_venues: sweep_state.quality.active_venues,
                stale_venues: sweep_state.quality.stale_venues,
            },
        };
        *self.latest_state.write() = state.clone();
        state
    }

    fn push_event(&self, event: ToxicEvent) {
        let semantic_key = toxic_event_semantic_key(&event);
        if self.seen_event_ids.read().contains(&semantic_key) {
            return;
        }
        self.seen_event_ids.write().insert(semantic_key);
        let mut events = self.recent_events.write();
        if let Some(store) = &self.store {
            if let Err(err) = store.insert_event(&event) {
                tracing::warn!("failed to persist toxic event: {err}");
            }
        }
        events.push(event);
        if events.len() > self.recent_event_limit {
            let overflow = events.len() - self.recent_event_limit;
            events.drain(0..overflow);
        }
    }
}

impl RegimeAwareProvider for ToxicService {
    fn regime_manager(&self) -> &Arc<RegimeThresholdManager> {
        &self.regime_manager
    }
}

pub fn toxic_event_semantic_key(event: &ToxicEvent) -> String {
    let direction = match event.direction {
        ToxicDirection::Buy => "buy",
        ToxicDirection::Sell => "sell",
        ToxicDirection::Neutral => "neutral",
    };
    let venue = event
        .leader_venue
        .map(|venue| venue.as_key())
        .unwrap_or("unknown");
    let bucket = if event.window_ms == 0 {
        event.ts
    } else {
        event.ts - event.ts.rem_euclid(event.window_ms as i64)
    };
    format!(
        "{}:{}:{}:{}:{}:{}",
        event.symbol,
        direction,
        event.window_ms,
        venue,
        event.severity.label(),
        bucket
    )
}

pub fn latest_toxic_summary(state: &ToxicState) -> (ToxicDirection, f64, bool) {
    if let Some(event) = &state.latest_event {
        return (event.direction, event.toxic_volume_btc, true);
    }
    state
        .results
        .values()
        .max_by(|left, right| left.toxic_volume_btc.total_cmp(&right.toxic_volume_btc))
        .map(|result| {
            (
                result.direction,
                result.toxic_volume_btc,
                result.alert_triggered,
            )
        })
        .unwrap_or((ToxicDirection::Neutral, 0.0, false))
}

fn empty_state(windows_ms: &[ToxicWindowMs], now_ts: i64, threshold_btc: f64) -> ToxicState {
    ToxicState {
        symbol: "BTC-PERP".to_string(),
        updated_at: now_ts,
        threshold_btc,
        windows_ms: windows_ms.to_vec(),
        results: BTreeMap::new(),
        latest_event: None,
        recent_events: Vec::new(),
        quality: ToxicQuality {
            has_flow: false,
            has_markout: false,
            has_sweep: false,
            has_liquidation: false,
            liquidation: None,
            active_venues: Vec::new(),
            stale_venues: Vec::new(),
        },
    }
}

fn empty_flow_window(window_ms: ToxicWindowMs, now_ts: i64) -> FlowWindow {
    FlowWindow {
        symbol: "BTC-PERP".to_string(),
        window_ms,
        now_ts,
        aggressive_buy_btc: 0.0,
        aggressive_sell_btc: 0.0,
        aggressive_buy_usd: 0.0,
        aggressive_sell_usd: 0.0,
        net_aggressive_btc: 0.0,
        abs_aggressive_btc: 0.0,
        trade_count: 0,
        buy_trade_count: 0,
        sell_trade_count: 0,
        avg_trade_size_btc: 0.0,
        max_trade_size_btc: 0.0,
        venue_breakdown: empty_venue_breakdown(),
        mid_start: None,
        mid_end: None,
        price_move_bps: None,
        spread_bps_median: None,
        imbalance_10bps_median: None,
        data_quality: DataQuality {
            has_trades: false,
            has_books: false,
            active_venues: Vec::new(),
            stale_venues: Vec::new(),
        },
    }
}
