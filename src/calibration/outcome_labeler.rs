use anyhow::Context;

use crate::{
    market_data::price_index::{PriceIndex, PriceSnapshot},
    normalizers::book::{normalize_book, RawBookInput},
    replay::{replay_loader::load_jsonl, replay_types::ReplayEvent},
    types::{
        market::Venue,
        toxic::{ToxicDirection, ToxicEvent},
    },
};

use super::calibration_types::{EventOutcome, OutcomeLabel};

#[derive(Debug, Clone)]
pub struct OutcomeLabeler {
    snapshots: Vec<PriceSnapshot>,
}

impl OutcomeLabeler {
    pub fn from_replay_file(path: &str) -> anyhow::Result<Self> {
        let mut events = load_jsonl(path)?;
        events.sort_by_key(event_ts);

        let mut price_index = PriceIndex::new(300_000, 60_000);
        for event in events {
            if let ReplayEvent::Book(record) = event {
                let mut bids = record.bids;
                let mut asks = record.asks;
                if bids.is_empty() {
                    bids.push((record.best_bid, 1.0));
                }
                if asks.is_empty() {
                    asks.push((record.best_ask, 1.0));
                }
                if let Some(book) = normalize_book(RawBookInput {
                    venue: record.venue,
                    symbol: replay_symbol_for_venue(record.venue).to_string(),
                    ts: record.ts,
                    bids,
                    asks,
                }) {
                    price_index.update_book(book);
                }
            }
        }

        let snapshots = price_index.snapshots_since(i64::MIN);
        if snapshots.is_empty() {
            return Err(anyhow::anyhow!(
                "no price snapshots derived from replay books"
            ))
            .with_context(|| format!("failed to build price snapshots from {path}"));
        }
        Ok(Self { snapshots })
    }

    pub fn label_events(&self, events: &[ToxicEvent]) -> Vec<EventOutcome> {
        events.iter().map(|event| self.label_event(event)).collect()
    }

    pub fn label_event(&self, event: &ToxicEvent) -> EventOutcome {
        let current_mid = self.mid_at_or_before(event.ts);
        let forward_1s_bps = self.forward_move_bps(event, 1_000);
        let forward_5s_bps = self.forward_move_bps(event, 5_000);
        let forward_15s_bps = self.forward_move_bps(event, 15_000);
        let forward_60s_bps = self.forward_move_bps(event, 60_000);

        let horizon_priority = [
            (5_000_u64, forward_5s_bps),
            (15_000_u64, forward_15s_bps),
            (1_000_u64, forward_1s_bps),
            (60_000_u64, forward_60s_bps),
        ];
        let primary = horizon_priority
            .into_iter()
            .find(|(_, move_bps)| move_bps.is_some());
        let primary_horizon_ms = primary.map(|(horizon, _)| horizon);
        let primary_move_bps = primary.and_then(|(_, move_bps)| move_bps);

        let label = match primary_move_bps {
            Some(move_bps) if move_bps > 0.0 => OutcomeLabel::Hit,
            Some(move_bps) if move_bps <= -1.0 => OutcomeLabel::FalsePositive,
            Some(_) => OutcomeLabel::Neutral,
            None => OutcomeLabel::Unknown,
        };

        EventOutcome {
            event: event.clone(),
            current_mid,
            forward_1s_bps,
            forward_5s_bps,
            forward_15s_bps,
            forward_60s_bps,
            primary_horizon_ms,
            primary_move_bps,
            label,
        }
    }

    pub fn mid_at_or_before(&self, ts: i64) -> Option<f64> {
        self.snapshots
            .iter()
            .rev()
            .find(|snapshot| snapshot.ts <= ts)
            .map(|snapshot| snapshot.index_mid)
    }

    fn forward_move_bps(&self, event: &ToxicEvent, horizon_ms: i64) -> Option<f64> {
        let start = self.mid_at_or_before(event.ts)?;
        let future_snapshot = self
            .snapshots
            .iter()
            .rev()
            .find(|snapshot| snapshot.ts <= event.ts + horizon_ms && snapshot.ts > event.ts)?;
        let future = future_snapshot.index_mid;
        Some(match event.direction {
            ToxicDirection::Buy => ((future - start) / start) * 10_000.0,
            ToxicDirection::Sell => ((start - future) / start) * 10_000.0,
            ToxicDirection::Neutral => 0.0,
        })
    }
}

fn event_ts(event: &ReplayEvent) -> i64 {
    match event {
        ReplayEvent::Trade(record) => record.ts,
        ReplayEvent::Book(record) => record.ts,
        ReplayEvent::ExpectToxic(record) => record.ts,
    }
}

fn replay_symbol_for_venue(venue: Venue) -> &'static str {
    match venue {
        Venue::Binance => "BTCUSDT",
        Venue::Bybit => "BTCUSDT",
        Venue::Okx => "BTC-USDT-SWAP",
        Venue::Bitfinex => "tBTCF0:USTF0",
    }
}
