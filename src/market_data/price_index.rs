use crate::types::market::NormalizedBook;

use super::book_state::BookState;

#[derive(Debug, Clone)]
pub struct PriceSnapshot {
    pub ts: i64,
    pub index_mid: f64,
    pub spread_bps_median: Option<f64>,
    pub imbalance_10bps_median: Option<f64>,
    pub bid_depth_btc_10bps_median: Option<f64>,
    pub ask_depth_btc_10bps_median: Option<f64>,
}

#[derive(Debug)]
pub struct PriceIndex {
    book_state: BookState,
    snapshots: Vec<(String, PriceSnapshot)>,
    history_ms: i64,
    stale_ms: i64,
}

impl PriceIndex {
    pub fn new(history_ms: i64, stale_ms: i64) -> Self {
        Self {
            book_state: BookState::default(),
            snapshots: Vec::new(),
            history_ms,
            stale_ms,
        }
    }

    pub fn update_book(&mut self, book: NormalizedBook) {
        let ts = book.ts;
        let symbol = book.symbol.clone();
        self.book_state.update_book(book);
        if let Some(snapshot) = self.build_snapshot_for_symbol(ts, &symbol) {
            self.snapshots.push((symbol_prefix(&symbol), snapshot));
            self.prune(ts);
        }
    }

    pub fn current_snapshot(&self, now_ts: i64) -> Option<PriceSnapshot> {
        self.build_snapshot(now_ts)
    }

    pub fn current_mid(&self, now_ts: i64) -> Option<f64> {
        self.current_snapshot(now_ts)
            .map(|snapshot| snapshot.index_mid)
    }

    pub fn current_snapshot_for_symbol(&self, now_ts: i64, symbol: &str) -> Option<PriceSnapshot> {
        self.build_snapshot_for_symbol(now_ts, symbol)
    }

    pub fn mid_at_or_before_for_symbol(&self, ts: i64, symbol: &str) -> Option<f64> {
        let normalized = symbol_prefix(symbol);
        self.snapshots
            .iter()
            .rev()
            .find(|(snapshot_symbol, snapshot)| snapshot_symbol == &normalized && snapshot.ts <= ts)
            .map(|(_, snapshot)| snapshot.index_mid)
    }

    pub fn mid_at_or_before(&self, ts: i64) -> Option<f64> {
        self.snapshots
            .iter()
            .rev()
            .find(|(_, snapshot)| snapshot.ts <= ts)
            .map(|(_, snapshot)| snapshot.index_mid)
    }

    pub fn snapshot_at_or_before(&self, ts: i64) -> Option<PriceSnapshot> {
        self.snapshots
            .iter()
            .rev()
            .find(|(_, snapshot)| snapshot.ts <= ts)
            .map(|(_, snapshot)| snapshot.clone())
    }

    pub fn latest_snapshot(&self) -> Option<PriceSnapshot> {
        self.snapshots.last().map(|(_, snapshot)| snapshot.clone())
    }

    pub fn snapshots_since(&self, ts: i64) -> Vec<PriceSnapshot> {
        self.snapshots
            .iter()
            .filter(|(_, snapshot)| snapshot.ts >= ts)
            .map(|(_, snapshot)| snapshot.clone())
            .collect()
    }

    pub fn book_state(&self) -> &BookState {
        &self.book_state
    }

    fn build_snapshot(&self, now_ts: i64) -> Option<PriceSnapshot> {
        let books = self
            .book_state
            .latest_books()
            .into_values()
            .filter(|book| now_ts - book.ts <= self.stale_ms)
            .collect::<Vec<_>>();
        if books.is_empty() {
            return None;
        }
        Some(PriceSnapshot {
            ts: now_ts,
            index_mid: median(books.iter().map(|book| book.mid).collect()),
            spread_bps_median: Some(median(books.iter().map(|book| book.spread_bps).collect())),
            imbalance_10bps_median: Some(median(
                books.iter().map(|book| book.imbalance_10bps).collect(),
            )),
            bid_depth_btc_10bps_median: Some(median(
                books.iter().map(|book| book.bid_depth_btc_10bps).collect(),
            )),
            ask_depth_btc_10bps_median: Some(median(
                books.iter().map(|book| book.ask_depth_btc_10bps).collect(),
            )),
        })
    }

    fn build_snapshot_for_symbol(&self, now_ts: i64, symbol: &str) -> Option<PriceSnapshot> {
        let books = self
            .book_state
            .latest_books_for_symbol(symbol)
            .into_values()
            .filter(|book| now_ts - book.ts <= self.stale_ms)
            .collect::<Vec<_>>();
        if books.is_empty() {
            return None;
        }
        Some(PriceSnapshot {
            ts: now_ts,
            index_mid: median(books.iter().map(|book| book.mid).collect()),
            spread_bps_median: Some(median(books.iter().map(|book| book.spread_bps).collect())),
            imbalance_10bps_median: Some(median(
                books.iter().map(|book| book.imbalance_10bps).collect(),
            )),
            bid_depth_btc_10bps_median: Some(median(
                books.iter().map(|book| book.bid_depth_btc_10bps).collect(),
            )),
            ask_depth_btc_10bps_median: Some(median(
                books.iter().map(|book| book.ask_depth_btc_10bps).collect(),
            )),
        })
    }

    fn prune(&mut self, now_ts: i64) {
        let cutoff = now_ts - self.history_ms;
        self.snapshots.retain(|(_, snapshot)| snapshot.ts >= cutoff);
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

pub fn median(mut values: Vec<f64>) -> f64 {
    values.sort_by(|a, b| a.total_cmp(b));
    let middle = values.len() / 2;
    if values.len() % 2 == 1 {
        values[middle]
    } else {
        (values[middle - 1] + values[middle]) / 2.0
    }
}
