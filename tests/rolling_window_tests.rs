use btc_toxic_flow_monitor_rs::{
    market_data::{
        book_state::BookState, price_index::PriceIndex, rolling_windows::RollingWindows,
        trade_ring_buffer::TradeRingBuffer,
    },
    types::market::{AggressorSide, NormalizedTrade, Venue},
};

fn trade(venue: Venue, side: AggressorSide, size_btc: f64) -> NormalizedTrade {
    trade_for_symbol(venue, "BTC-PERP", side, size_btc)
}

fn trade_for_symbol(
    venue: Venue,
    symbol: &str,
    side: AggressorSide,
    size_btc: f64,
) -> NormalizedTrade {
    NormalizedTrade {
        venue,
        symbol: symbol.to_string(),
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

#[test]
fn rolling_window_can_compute_btc_and_eth_separately() {
    let mut buffer = TradeRingBuffer::new(120_000);
    buffer.add_trade(trade_for_symbol(
        Venue::Binance,
        "BTC-PERP",
        AggressorSide::Buy,
        400.0,
    ));
    buffer.add_trade(trade_for_symbol(
        Venue::Binance,
        "ETH-PERP",
        AggressorSide::Sell,
        8_000.0,
    ));
    let book_state = BookState::default();
    let price_index = PriceIndex::new(120_000, 5_000);
    let windows = [5000];

    let btc =
        RollingWindows::new_for_symbol(&buffer, &book_state, &price_index, &windows, 5_000, "BTC")
            .compute_window(5000, 10_000);
    let eth =
        RollingWindows::new_for_symbol(&buffer, &book_state, &price_index, &windows, 5_000, "ETH")
            .compute_window(5000, 10_000);

    assert_eq!(btc.symbol, "BTC-PERP");
    assert_eq!(btc.aggressive_buy_btc, 400.0);
    assert_eq!(btc.aggressive_sell_btc, 0.0);
    assert_eq!(eth.symbol, "ETH-PERP");
    assert_eq!(eth.aggressive_buy_btc, 0.0);
    assert_eq!(eth.aggressive_sell_btc, 8_000.0);
}

#[test]
fn trade_dedupe_key_includes_symbol() {
    let mut buffer = TradeRingBuffer::new(120_000);
    let mut btc = trade_for_symbol(Venue::Binance, "BTC-PERP", AggressorSide::Buy, 1.0);
    btc.trade_id = Some("shared-id".to_string());
    let mut eth = trade_for_symbol(Venue::Binance, "ETH-PERP", AggressorSide::Buy, 2.0);
    eth.trade_id = Some("shared-id".to_string());

    buffer.add_trade(btc);
    buffer.add_trade(eth);

    assert_eq!(buffer.len(), 2);
}
