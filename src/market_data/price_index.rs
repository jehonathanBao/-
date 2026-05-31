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
    snapshots: Vec<PriceSnapshot>,
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
        self.book_state.update_book(book);
        if let Some(snapshot) = self.build_snapshot(ts) {
            self.snapshots.push(snapshot);
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

    pub fn mid_at_or_before(&self, ts: i64) -> Option<f64> {
        self.snapshots
            .iter()
            .rev()
            .find(|snapshot| snapshot.ts <= ts)
            .map(|snapshot| snapshot.index_mid)
    }

    pub fn snapshot_at_or_before(&self, ts: i64) -> Option<PriceSnapshot> {
        self.snapshots
            .iter()
            .rev()
            .find(|snapshot| snapshot.ts <= ts)
            .cloned()
    }

    pub fn latest_snapshot(&self) -> Option<PriceSnapshot> {
        self.snapshots.last().cloned()
    }

    pub fn snapshots_since(&self, ts: i64) -> Vec<PriceSnapshot> {
        self.snapshots
            .iter()
            .filter(|snapshot| snapshot.ts >= ts)
            .cloned()
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

    fn prune(&mut self, now_ts: i64) {
        let cutoff = now_ts - self.history_ms;
        self.snapshots.retain(|snapshot| snapshot.ts >= cutoff);
    }
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
