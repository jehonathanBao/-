use std::sync::Arc;

use parking_lot::RwLock;

use crate::{
    config::AppConfig,
    market_data::{
        book_state::BookState,
        event_bus::{MarketDataBus, MarketDataEvent},
        price_index::{PriceIndex, PriceSnapshot},
        rolling_windows::RollingWindows,
        trade_ring_buffer::TradeRingBuffer,
    },
    normalizers::trade::now_ms,
    types::{flow::FlowState, market::NormalizedTrade},
};

#[derive(Clone)]
pub struct FlowWindowService {
    bus: MarketDataBus,
    windows_ms: Vec<u64>,
    stale_ms: i64,
    compute_interval_ms: u64,
    trade_buffer: Arc<RwLock<TradeRingBuffer>>,
    book_state: Arc<RwLock<BookState>>,
    price_index: Arc<RwLock<PriceIndex>>,
    latest_state: Arc<RwLock<FlowState>>,
    task: Arc<RwLock<Option<tokio::task::JoinHandle<()>>>>,
    symbol: String,
}

impl FlowWindowService {
    pub fn new(bus: MarketDataBus, config: &AppConfig) -> Self {
        let max_age = config.max_buffer_age_ms.max(120_000);
        let windows_ms = config.windows_ms.clone();
        let initial_state = FlowState {
            symbol: config.symbol.clone(),
            updated_at: now_ms(),
            windows: Default::default(),
        };
        Self {
            bus,
            windows_ms,
            stale_ms: config.book_stale_ms,
            compute_interval_ms: config.flow_compute_interval_ms,
            trade_buffer: Arc::new(RwLock::new(TradeRingBuffer::new(max_age))),
            book_state: Arc::new(RwLock::new(BookState::default())),
            price_index: Arc::new(RwLock::new(PriceIndex::new(max_age, config.book_stale_ms))),
            latest_state: Arc::new(RwLock::new(initial_state)),
            task: Arc::new(RwLock::new(None)),
            symbol: config.symbol.clone(),
        }
    }

    pub fn start(&self) {
        if self.task.read().is_some() {
            return;
        }
        let mut rx = self.bus.subscribe();
        let trade_buffer = self.trade_buffer.clone();
        let book_state = self.book_state.clone();
        let price_index = self.price_index.clone();
        let latest_state = self.latest_state.clone();
        let windows_ms = self.windows_ms.clone();
        let stale_ms = self.stale_ms;
        let compute_interval_ms = self.compute_interval_ms;
        let quality = self.bus.quality_tracker();
        let symbol = self.symbol.clone();

        let handle = tokio::spawn(async move {
            let mut interval =
                tokio::time::interval(std::time::Duration::from_millis(compute_interval_ms));
            loop {
                tokio::select! {
                    event = rx.recv() => {
                        match event {
                            Ok(MarketDataEvent::Trade(trade)) => trade_buffer.write().add_trade(trade),
                            Ok(MarketDataEvent::Book(book)) => {
                                book_state.write().update_book(book.clone());
                                price_index.write().update_book(book);
                            }
                            Ok(MarketDataEvent::VenueHealth(_)) => {}
                            Err(tokio::sync::broadcast::error::RecvError::Lagged(skipped)) => {
                                quality.record_flow_window_lagged(skipped);
                            }
                            Err(tokio::sync::broadcast::error::RecvError::Closed) => break,
                        }
                    }
                    _ = interval.tick() => {
                        let now = now_ms();
                        trade_buffer.write().prune(now);
                        let state = {
                            let trades = trade_buffer.read();
                            let books = book_state.read();
                            let prices = price_index.read();
                            RollingWindows::new_for_symbol(
                                &trades,
                                &books,
                                &prices,
                                &windows_ms,
                                stale_ms,
                                &symbol,
                            )
                            .compute_all(now)
                        };
                        *latest_state.write() = state;
                    }
                }
            }
        });
        *self.task.write() = Some(handle);
    }

    pub fn stop(&self) {
        if let Some(handle) = self.task.write().take() {
            handle.abort();
        }
    }

    pub fn latest_state(&self) -> FlowState {
        self.latest_state.read().clone()
    }

    pub fn latest_state_for_symbol(&self, symbol: &str) -> FlowState {
        if symbol_prefix(symbol) == symbol_prefix(&self.symbol) {
            return self.latest_state();
        }
        let now = now_ms();
        let state = {
            let trades = self.trade_buffer.read();
            let books = self.book_state.read();
            let prices = self.price_index.read();
            RollingWindows::new_for_symbol(
                &trades,
                &books,
                &prices,
                &self.windows_ms,
                self.stale_ms,
                symbol,
            )
            .compute_all(now)
        };
        state
    }

    pub fn get_latest_flow_state(&self) -> FlowState {
        self.latest_state()
    }

    pub fn shared_state(&self) -> Arc<RwLock<FlowState>> {
        self.latest_state.clone()
    }

    pub fn market_data_quality(&self) -> crate::market_data::quality::MarketDataQualityTracker {
        self.bus.quality_tracker()
    }

    pub fn get_mid_at_or_before(&self, ts: i64) -> Option<f64> {
        self.price_index.read().mid_at_or_before(ts)
    }

    pub fn get_mid_at_or_before_for_symbol(&self, ts: i64, symbol: &str) -> Option<f64> {
        self.price_index
            .read()
            .mid_at_or_before_for_symbol(ts, symbol)
    }

    pub fn has_price_index(&self) -> bool {
        self.price_index.read().current_mid(now_ms()).is_some()
    }

    pub fn has_price_index_for_symbol(&self, symbol: &str) -> bool {
        self.price_index
            .read()
            .current_snapshot_for_symbol(now_ms(), symbol)
            .is_some()
    }

    pub fn get_trades_since(&self, ts: i64) -> Vec<NormalizedTrade> {
        self.trade_buffer.read().get_trades_since(ts)
    }

    pub fn get_price_snapshot_at_or_before(&self, ts: i64) -> Option<PriceSnapshot> {
        self.price_index.read().snapshot_at_or_before(ts)
    }

    pub fn get_price_snapshot_at_or_before_for_symbol(
        &self,
        ts: i64,
        symbol: &str,
    ) -> Option<PriceSnapshot> {
        self.price_index
            .read()
            .snapshot_at_or_before_for_symbol(ts, symbol)
    }

    pub fn get_latest_price_snapshot(&self) -> Option<PriceSnapshot> {
        self.price_index.read().latest_snapshot()
    }

    pub fn get_price_snapshots_since(&self, ts: i64) -> Vec<PriceSnapshot> {
        self.price_index.read().snapshots_since(ts)
    }

    pub fn get_price_snapshots_since_for_symbol(
        &self,
        ts: i64,
        symbol: &str,
    ) -> Vec<PriceSnapshot> {
        self.price_index
            .read()
            .snapshots_since_for_symbol(ts, symbol)
    }

    pub fn get_active_venues(&self, now_ts: i64) -> Vec<crate::types::market::Venue> {
        let books = self.book_state.read();
        let latest = books.latest_books();
        crate::types::market::Venue::ALL
            .into_iter()
            .filter(|venue| {
                latest
                    .get(venue)
                    .is_some_and(|book| now_ts - book.ts <= self.stale_ms)
            })
            .collect()
    }

    pub fn get_stale_venues(&self, now_ts: i64) -> Vec<crate::types::market::Venue> {
        let books = self.book_state.read();
        let latest = books.latest_books();
        crate::types::market::Venue::ALL
            .into_iter()
            .filter(|venue| {
                latest
                    .get(venue)
                    .is_some_and(|book| now_ts - book.ts > self.stale_ms)
            })
            .collect()
    }

    pub fn recompute_for_tests(&self, now_ts: i64) -> FlowState {
        let state = {
            let trades = self.trade_buffer.read();
            let books = self.book_state.read();
            let prices = self.price_index.read();
            RollingWindows::new_for_symbol(
                &trades,
                &books,
                &prices,
                &self.windows_ms,
                self.stale_ms,
                &self.symbol,
            )
            .compute_all(now_ts)
        };
        *self.latest_state.write() = state.clone();
        state
    }

    pub fn add_trade_for_tests(&self, trade: NormalizedTrade) {
        self.trade_buffer.write().add_trade(trade);
    }

    pub fn add_book_for_tests(&self, book: crate::types::market::NormalizedBook) {
        self.book_state.write().update_book(book.clone());
        self.price_index.write().update_book(book);
    }
}

fn symbol_prefix(symbol: &str) -> String {
    symbol
        .trim()
        .split(['-', '_', '/', ':'])
        .next()
        .unwrap_or(symbol)
        .to_ascii_uppercase()
}
