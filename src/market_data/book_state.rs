use std::collections::BTreeMap;

use crate::types::market::{NormalizedBook, Venue};

#[derive(Debug, Default)]
pub struct BookState {
    latest: BTreeMap<Venue, NormalizedBook>,
}

impl BookState {
    pub fn update_book(&mut self, book: NormalizedBook) {
        self.latest.insert(book.venue, book);
    }

    pub fn latest_books(&self) -> BTreeMap<Venue, NormalizedBook> {
        self.latest.clone()
    }

    pub fn active_venues(&self, now_ts: i64, stale_ms: i64) -> Vec<String> {
        self.latest
            .iter()
            .filter(|(_, book)| now_ts - book.ts <= stale_ms)
            .map(|(venue, _)| venue.as_key().to_string())
            .collect()
    }

    pub fn stale_venues(&self, now_ts: i64, stale_ms: i64) -> Vec<String> {
        self.latest
            .iter()
            .filter(|(_, book)| now_ts - book.ts > stale_ms)
            .map(|(venue, _)| venue.as_key().to_string())
            .collect()
    }
}
