use std::collections::BTreeMap;

use crate::types::{
    flow::{empty_venue_breakdown, DataQuality, FlowState, FlowWindow, VenueFlowBreakdown},
    market::{AggressorSide, NormalizedTrade, Venue},
};

use super::{book_state::BookState, price_index::PriceIndex, trade_ring_buffer::TradeRingBuffer};

pub struct RollingWindows<'a> {
    trade_buffer: &'a TradeRingBuffer,
    book_state: &'a BookState,
    price_index: &'a PriceIndex,
    windows_ms: &'a [u64],
    stale_ms: i64,
    symbol_filter: Option<String>,
}

impl<'a> RollingWindows<'a> {
    pub fn new(
        trade_buffer: &'a TradeRingBuffer,
        book_state: &'a BookState,
        price_index: &'a PriceIndex,
        windows_ms: &'a [u64],
        stale_ms: i64,
    ) -> Self {
        Self {
            trade_buffer,
            book_state,
            price_index,
            windows_ms,
            stale_ms,
            symbol_filter: None,
        }
    }

    pub fn new_for_symbol(
        trade_buffer: &'a TradeRingBuffer,
        book_state: &'a BookState,
        price_index: &'a PriceIndex,
        windows_ms: &'a [u64],
        stale_ms: i64,
        symbol: &str,
    ) -> Self {
        Self {
            trade_buffer,
            book_state,
            price_index,
            windows_ms,
            stale_ms,
            symbol_filter: Some(symbol_prefix(symbol)),
        }
    }

    pub fn compute_all(&self, now_ts: i64) -> FlowState {
        let windows = self
            .windows_ms
            .iter()
            .map(|window_ms| {
                (
                    window_ms.to_string(),
                    self.compute_window(*window_ms, now_ts),
                )
            })
            .collect::<BTreeMap<_, _>>();
        FlowState {
            symbol: self.infer_symbol(now_ts),
            updated_at: now_ts,
            windows,
        }
    }

    pub fn compute_window(&self, window_ms: u64, now_ts: i64) -> FlowWindow {
        let since_ts = now_ts - window_ms as i64;
        let trades = self
            .trade_buffer
            .get_trades_since(since_ts)
            .into_iter()
            .filter(|trade| trade.ts <= now_ts)
            .filter(|trade| self.symbol_matches(&trade.symbol))
            .collect::<Vec<_>>();
        let mut venue_breakdown = empty_venue_breakdown();
        let mut aggressive_buy_btc = 0.0;
        let mut aggressive_sell_btc = 0.0;
        let mut aggressive_buy_usd = 0.0;
        let mut aggressive_sell_usd = 0.0;
        let mut buy_trade_count = 0;
        let mut sell_trade_count = 0;
        let mut max_trade_size_btc = 0.0;

        for trade in &trades {
            let stats = venue_breakdown
                .get_mut(trade.venue.as_key())
                .expect("venue breakdown contains all venues");
            add_trade(stats, trade);
            max_trade_size_btc = f64::max(max_trade_size_btc, trade.size_btc);
            match trade.aggressor_side {
                AggressorSide::Buy => {
                    aggressive_buy_btc += trade.size_btc;
                    aggressive_buy_usd += trade.size_usd;
                    buy_trade_count += 1;
                }
                AggressorSide::Sell => {
                    aggressive_sell_btc += trade.size_btc;
                    aggressive_sell_usd += trade.size_usd;
                    sell_trade_count += 1;
                }
            }
        }

        for venue in Venue::ALL {
            if let Some(stats) = venue_breakdown.get_mut(venue.as_key()) {
                stats.net_aggressive_btc = stats.aggressive_buy_btc - stats.aggressive_sell_btc;
                stats.abs_aggressive_btc = stats.aggressive_buy_btc + stats.aggressive_sell_btc;
            }
        }

        let abs_aggressive_btc = aggressive_buy_btc + aggressive_sell_btc;
        let mid_start = if let Some(symbol) = self.symbol_filter.as_deref() {
            self.price_index
                .mid_at_or_before_for_symbol(since_ts, symbol)
        } else {
            self.price_index.mid_at_or_before(since_ts)
        };
        let current = if let Some(symbol) = self.symbol_filter.as_deref() {
            self.price_index.current_snapshot_for_symbol(now_ts, symbol)
        } else {
            self.price_index.current_snapshot(now_ts)
        };
        let mid_end = current.as_ref().map(|snapshot| snapshot.index_mid);
        let price_move_bps = match (mid_start, mid_end) {
            (Some(start), Some(end)) if start > 0.0 => Some(((end - start) / start) * 10_000.0),
            _ => None,
        };
        let active_venues = self
            .symbol_filter
            .as_deref()
            .map(|symbol| {
                self.book_state
                    .active_venues_for_symbol(symbol, now_ts, self.stale_ms)
            })
            .unwrap_or_else(|| self.book_state.active_venues(now_ts, self.stale_ms));
        let stale_venues = self
            .symbol_filter
            .as_deref()
            .map(|symbol| {
                self.book_state
                    .stale_venues_for_symbol(symbol, now_ts, self.stale_ms)
            })
            .unwrap_or_else(|| self.book_state.stale_venues(now_ts, self.stale_ms));
        let trade_count = trades.len() as u64;

        FlowWindow {
            symbol: trades
                .last()
                .map(|trade| trade.symbol.clone())
                .unwrap_or_else(|| self.infer_symbol(now_ts)),
            window_ms,
            now_ts,
            aggressive_buy_btc,
            aggressive_sell_btc,
            aggressive_buy_usd,
            aggressive_sell_usd,
            net_aggressive_btc: aggressive_buy_btc - aggressive_sell_btc,
            abs_aggressive_btc,
            trade_count,
            buy_trade_count,
            sell_trade_count,
            avg_trade_size_btc: if trade_count == 0 {
                0.0
            } else {
                abs_aggressive_btc / trade_count as f64
            },
            max_trade_size_btc,
            venue_breakdown,
            mid_start,
            mid_end,
            price_move_bps,
            spread_bps_median: current
                .as_ref()
                .and_then(|snapshot| snapshot.spread_bps_median),
            imbalance_10bps_median: current
                .as_ref()
                .and_then(|snapshot| snapshot.imbalance_10bps_median),
            data_quality: DataQuality {
                has_trades: trade_count > 0,
                has_books: !active_venues.is_empty(),
                active_venues,
                stale_venues,
            },
        }
    }

    fn infer_symbol(&self, now_ts: i64) -> String {
        if let Some(symbol) = self.symbol_filter.as_deref() {
            return format!("{symbol}-PERP");
        }
        self.trade_buffer
            .get_trades_since(now_ts - self.stale_ms)
            .into_iter()
            .rfind(|trade| self.symbol_matches(&trade.symbol))
            .map(|trade| trade.symbol.clone())
            .or_else(|| {
                self.book_state
                    .latest_books()
                    .values()
                    .find(|book| now_ts - book.ts <= self.stale_ms)
                    .map(|book| book.symbol.clone())
            })
            .unwrap_or_else(|| "BTC-PERP".to_string())
    }

    fn symbol_matches(&self, symbol: &str) -> bool {
        self.symbol_filter
            .as_deref()
            .is_none_or(|filter| symbol_prefix(symbol) == filter)
    }
}

fn add_trade(stats: &mut VenueFlowBreakdown, trade: &NormalizedTrade) {
    stats.trade_count += 1;
    stats.last_trade_ts = Some(stats.last_trade_ts.unwrap_or(0).max(trade.ts));
    match trade.aggressor_side {
        AggressorSide::Buy => {
            stats.aggressive_buy_btc += trade.size_btc;
            stats.aggressive_buy_usd += trade.size_usd;
            stats.buy_trade_count += 1;
        }
        AggressorSide::Sell => {
            stats.aggressive_sell_btc += trade.size_btc;
            stats.aggressive_sell_usd += trade.size_usd;
            stats.sell_trade_count += 1;
        }
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
