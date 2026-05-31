use btc_toxic_flow_monitor_rs::{
    market_data::price_index::PriceIndex,
    types::market::{NormalizedBook, Venue},
};

fn book(venue: Venue, mid: f64, ts: i64) -> NormalizedBook {
    NormalizedBook {
        venue,
        symbol: "BTC-PERP".to_string(),
        ts,
        best_bid: mid - 1.0,
        best_ask: mid + 1.0,
        bids: vec![(mid - 1.0, 1.0)],
        asks: vec![(mid + 1.0, 1.0)],
        mid,
        spread_bps: 2.0,
        bid_depth_btc_10bps: 1.0,
        ask_depth_btc_10bps: 1.0,
        bid_depth_usd_10bps: mid - 1.0,
        ask_depth_usd_10bps: mid + 1.0,
        imbalance_10bps: 0.0,
    }
}

#[test]
fn price_index_uses_median_mid() {
    let mut index = PriceIndex::new(120_000, 5_000);
    index.update_book(book(Venue::Binance, 100.0, 1000));
    index.update_book(book(Venue::Bybit, 101.0, 1000));
    index.update_book(book(Venue::Okx, 10_000.0, 1000));

    assert_eq!(index.current_mid(1000), Some(101.0));
}

#[test]
fn price_index_returns_historical_mid() {
    let mut index = PriceIndex::new(120_000, 5_000);
    index.update_book(book(Venue::Binance, 100.0, 1000));
    index.update_book(book(Venue::Binance, 110.0, 2000));

    assert_eq!(index.mid_at_or_before(1500), Some(100.0));
}
