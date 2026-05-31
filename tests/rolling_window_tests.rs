use btc_toxic_flow_monitor_rs::{
    market_data::{
        book_state::BookState, price_index::PriceIndex, rolling_windows::RollingWindows,
        trade_ring_buffer::TradeRingBuffer,
    },
    types::market::{AggressorSide, NormalizedTrade, Venue},
};

fn trade(venue: Venue, side: AggressorSide, size_btc: f64) -> NormalizedTrade {
    NormalizedTrade {
        venue,
        symbol: "BTC-PERP".to_string(),
        ts: 10_000,
        price: 100_000.0,
        size_btc,
        size_usd: size_btc * 100_000.0,
        aggressor_side: side,
        trade_id: None,
    }
}

#[test]
fn rolling_window_aggregates_mock_flow() {
    let mut buffer = TradeRingBuffer::new(120_000);
    buffer.add_trade(trade(Venue::Binance, AggressorSide::Buy, 400.0));
    buffer.add_trade(trade(Venue::Bybit, AggressorSide::Buy, 300.0));
    buffer.add_trade(trade(Venue::Okx, AggressorSide::Buy, 200.0));
    buffer.add_trade(trade(Venue::Binance, AggressorSide::Sell, 100.0));
    let book_state = BookState::default();
    let price_index = PriceIndex::new(120_000, 5_000);
    let windows = [5000];
    let rolling = RollingWindows::new(&buffer, &book_state, &price_index, &windows, 5_000);

    let window = rolling.compute_window(5000, 10_000);

    assert_eq!(window.aggressive_buy_btc, 900.0);
    assert_eq!(window.aggressive_sell_btc, 100.0);
    assert_eq!(window.net_aggressive_btc, 800.0);
    assert_eq!(window.abs_aggressive_btc, 1000.0);
    assert_eq!(window.venue_breakdown["binance"].net_aggressive_btc, 300.0);
    assert_eq!(window.venue_breakdown["bybit"].aggressive_buy_btc, 300.0);
    assert_eq!(window.venue_breakdown["okx"].aggressive_buy_btc, 200.0);
}
