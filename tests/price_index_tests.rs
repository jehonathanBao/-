use btc_toxic_flow_monitor_rs::{
    market_data::price_index::PriceIndex,
    types::market::{NormalizedBook, Venue},
};

fn book(venue: Venue, mid: f64, ts: i64) -> NormalizedBook {
    book_for_symbol(venue, "BTC-PERP", mid, ts)
}

fn book_for_symbol(venue: Venue, symbol: &str, mid: f64, ts: i64) -> NormalizedBook {
    NormalizedBook {
        venue,
        symbol: symbol.to_string(),
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
fn historical_snapshots_are_scoped_to_the_requested_symbol() {
    let mut index = PriceIndex::new(120_000, 5_000);
    index.update_book(book_for_symbol(Venue::Binance, "BTC-PERP", 100.0, 1_000));
    index.update_book(book_for_symbol(Venue::Binance, "ETH-PERP", 2_000.0, 1_100));
    index.update_book(book_for_symbol(Venue::Binance, "BTC-PERP", 110.0, 2_000));
    index.update_book(book_for_symbol(Venue::Binance, "ETH-PERP", 2_100.0, 2_100));

    assert_eq!(
        index
            .snapshot_at_or_before_for_symbol(1_500, "BTC-PERP")
            .map(|snapshot| snapshot.index_mid),
        Some(100.0)
    );
    assert_eq!(
        index
            .snapshot_at_or_before_for_symbol(1_500, "ETH-PERP")
            .map(|snapshot| snapshot.index_mid),
        Some(2_000.0)
    );
    assert_eq!(
        index
            .snapshots_since_for_symbol(1_500, "ETH-PERP")
            .into_iter()
            .map(|snapshot| snapshot.index_mid)
            .collect::<Vec<_>>(),
        vec![2_100.0]
    );
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
