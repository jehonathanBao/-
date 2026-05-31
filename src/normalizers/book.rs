use crate::{
    normalizers::symbol::require_symbol,
    types::market::{NormalizedBook, Venue},
};

#[derive(Debug, Clone)]
pub struct RawBookInput {
    pub venue: Venue,
    pub symbol: String,
    pub ts: i64,
    pub bids: Vec<(f64, f64)>,
    pub asks: Vec<(f64, f64)>,
}

pub fn normalize_book(input: RawBookInput) -> Option<NormalizedBook> {
    let mut bids = input
        .bids
        .into_iter()
        .filter(|(price, size)| {
            price.is_finite() && size.is_finite() && *price > 0.0 && *size >= 0.0
        })
        .collect::<Vec<_>>();
    let mut asks = input
        .asks
        .into_iter()
        .filter(|(price, size)| {
            price.is_finite() && size.is_finite() && *price > 0.0 && *size >= 0.0
        })
        .collect::<Vec<_>>();
    bids.sort_by(|a, b| b.0.total_cmp(&a.0));
    asks.sort_by(|a, b| a.0.total_cmp(&b.0));

    let (best_bid, _) = *bids.first()?;
    let (best_ask, _) = *asks.first()?;
    if best_bid <= 0.0 || best_ask <= 0.0 || best_ask <= best_bid {
        return None;
    }

    let mid = (best_bid + best_ask) / 2.0;
    let spread_bps = ((best_ask - best_bid) / mid) * 10_000.0;
    let bid_floor = mid * (1.0 - 0.001);
    let ask_ceiling = mid * (1.0 + 0.001);
    let bid_depth = bids
        .iter()
        .filter(|(price, _)| *price >= bid_floor)
        .copied()
        .collect::<Vec<_>>();
    let ask_depth = asks
        .iter()
        .filter(|(price, _)| *price <= ask_ceiling)
        .copied()
        .collect::<Vec<_>>();
    let bid_depth_btc_10bps = bid_depth.iter().map(|(_, size)| *size).sum::<f64>();
    let ask_depth_btc_10bps = ask_depth.iter().map(|(_, size)| *size).sum::<f64>();
    let denominator = bid_depth_btc_10bps + ask_depth_btc_10bps;

    Some(NormalizedBook {
        venue: input.venue,
        symbol: require_symbol(input.venue, &input.symbol).ok()?,
        ts: input.ts,
        best_bid,
        best_ask,
        bids,
        asks,
        mid,
        spread_bps,
        bid_depth_btc_10bps,
        ask_depth_btc_10bps,
        bid_depth_usd_10bps: bid_depth.iter().map(|(price, size)| price * size).sum(),
        ask_depth_usd_10bps: ask_depth.iter().map(|(price, size)| price * size).sum(),
        imbalance_10bps: if denominator == 0.0 {
            0.0
        } else {
            (bid_depth_btc_10bps - ask_depth_btc_10bps) / denominator
        },
    })
}
