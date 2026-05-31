use std::{collections::BTreeMap, sync::Arc};

use parking_lot::RwLock;

use crate::{
    config::AppConfig,
    market_data::flow_window_service::FlowWindowService,
    normalizers::trade::now_ms,
    toxicity::{
        liquidity_thinness::LiquidityThinness,
        sweep_detector::{SweepDetector, SweepInput, SweepParams},
    },
    types::{
        flow::{empty_venue_breakdown, DataQuality, FlowWindow},
        sweep::{SweepDirection, SweepQuality, SweepState, SweepWindowMs},
    },
};

#[derive(Clone)]
pub struct SweepService {
    flow_service: FlowWindowService,
    windows_ms: Vec<SweepWindowMs>,
    compute_interval_ms: u64,
    detector: SweepDetector,
    liquidity: LiquidityThinness,
    latest_state: Arc<RwLock<SweepState>>,
    task: Arc<RwLock<Option<tokio::task::JoinHandle<()>>>>,
}

impl SweepService {
    pub fn new(flow_service: FlowWindowService, config: &AppConfig) -> Self {
        let params = SweepParams::default();
        let windows_ms = if config.sweep_windows_ms.is_empty() {
            vec![1000, 5000, 15000]
        } else {
            config.sweep_windows_ms.clone()
        };
        let latest_state = Arc::new(RwLock::new(empty_state(&windows_ms, now_ms())));
        Self {
            flow_service,
            windows_ms,
            compute_interval_ms: config.sweep_compute_interval_ms,
            liquidity: LiquidityThinness::new(
                params.min_depth_drop_ratio,
                params.min_spread_widen_ratio,
            ),
            detector: SweepDetector::new(params),
            latest_state,
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

    pub fn get_state(&self) -> SweepState {
        self.latest_state.read().clone()
    }

    pub fn compute_once_for_tests(&self, now_ts: i64) -> SweepState {
        self.compute_once(now_ts)
    }

    fn compute_once(&self, now_ts: i64) -> SweepState {
        let flow_state = self.flow_service.get_latest_flow_state();
        let has_books = self.flow_service.get_latest_price_snapshot().is_some();
        let active_venues = self.flow_service.get_active_venues(now_ts);
        let stale_venues = self.flow_service.get_stale_venues(now_ts);
        let mut results = BTreeMap::new();
        let mut has_trades = false;

        for window_ms in &self.windows_ms {
            let since_ts = now_ts - *window_ms as i64;
            let trades = self.flow_service.get_trades_since(since_ts);
            has_trades |= !trades.is_empty();

            let flow_window = flow_state
                .windows
                .get(&window_ms.to_string())
                .cloned()
                .unwrap_or_else(|| empty_flow_window(*window_ms, now_ts));
            let liquidity = self.liquidity.detect(
                "BTC-PERP",
                *window_ms,
                self.flow_service.get_price_snapshot_at_or_before(since_ts),
                self.flow_service.get_latest_price_snapshot(),
            );
            let result = self.detector.detect(SweepInput {
                symbol: "BTC-PERP".to_string(),
                window_ms: *window_ms,
                trades,
                flow_window,
                liquidity: Some(liquidity),
            });
            results.insert(window_ms.to_string(), result);
        }

        let state = SweepState {
            symbol: "BTC-PERP".to_string(),
            updated_at: now_ts,
            windows_ms: self.windows_ms.clone(),
            results,
            quality: SweepQuality {
                has_trades,
                has_books,
                active_venues,
                stale_venues,
            },
        };
        *self.latest_state.write() = state.clone();
        state
    }
}

pub fn last_sweep_summary(state: &SweepState) -> (SweepDirection, bool) {
    state
        .results
        .values()
        .find(|result| result.sweep_detected)
        .map(|result| (result.direction, true))
        .unwrap_or((SweepDirection::None, false))
}

fn empty_state(windows_ms: &[SweepWindowMs], now_ts: i64) -> SweepState {
    SweepState {
        symbol: "BTC-PERP".to_string(),
        updated_at: now_ts,
        windows_ms: windows_ms.to_vec(),
        results: windows_ms
            .iter()
            .map(|window_ms| {
                (
                    window_ms.to_string(),
                    SweepDetector::default().detect(SweepInput {
                        symbol: "BTC-PERP".to_string(),
                        window_ms: *window_ms,
                        trades: Vec::new(),
                        flow_window: empty_flow_window(*window_ms, now_ts),
                        liquidity: Some(
                            LiquidityThinness::default().detect("BTC-PERP", *window_ms, None, None),
                        ),
                    }),
                )
            })
            .collect(),
        quality: SweepQuality {
            has_trades: false,
            has_books: false,
            active_venues: Vec::new(),
            stale_venues: Vec::new(),
        },
    }
}

fn empty_flow_window(window_ms: u64, now_ts: i64) -> FlowWindow {
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
