use std::collections::BTreeMap;

use crate::types::market::{NormalizedBook, Venue};

#[derive(Debug, Default)]
pub struct BookState {
    latest: BTreeMap<(Venue, String), NormalizedBook>,
}

impl BookState {
    pub fn update_book(&mut self, book: NormalizedBook) {
        self.latest
            .insert((book.venue, book.symbol.to_ascii_uppercase()), book);
    }

    pub fn latest_books(&self) -> BTreeMap<Venue, NormalizedBook> {
        let mut latest = BTreeMap::new();
        for ((venue, _), book) in &self.latest {
            let replace = latest
                .get(venue)
                .is_none_or(|existing: &NormalizedBook| book.ts >= existing.ts);
            if replace {
                latest.insert(*venue, book.clone());
            }
        }
        latest
    }

    pub fn latest_books_for_symbol(&self, symbol: &str) -> BTreeMap<Venue, NormalizedBook> {
        let normalized = symbol_prefix(symbol);
        self.latest
            .iter()
            .filter(|((_, book_symbol), _)| symbol_prefix(book_symbol) == normalized)
            .map(|((venue, _), book)| (*venue, book.clone()))
            .collect()
    }

    pub fn active_venues(&self, now_ts: i64, stale_ms: i64) -> Vec<String> {
        self.latest
            .values()
            .filter(|book| now_ts - book.ts <= stale_ms)
            .map(|book| book.venue.as_key().to_string())
            .collect()
    }

    pub fn stale_venues(&self, now_ts: i64, stale_ms: i64) -> Vec<String> {
        self.latest
            .values()
            .filter(|book| now_ts - book.ts > stale_ms)
            .map(|book| book.venue.as_key().to_string())
            .collect()
    }

    pub fn active_venues_for_symbol(
        &self,
        symbol: &str,
        now_ts: i64,
        stale_ms: i64,
    ) -> Vec<String> {
        self.latest_books_for_symbol(symbol)
            .into_iter()
            .filter(|(_, book)| now_ts - book.ts <= stale_ms)
            .map(|(venue, _)| venue.as_key().to_string())
            .collect()
    }

    pub fn stale_venues_for_symbol(&self, symbol: &str, now_ts: i64, stale_ms: i64) -> Vec<String> {
        self.latest_books_for_symbol(symbol)
            .into_iter()
            .filter(|(_, book)| now_ts - book.ts > stale_ms)
            .map(|(venue, _)| venue.as_key().to_string())
            .collect()
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
