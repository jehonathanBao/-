use btc_toxic_flow_monitor_rs::{
    normalizers::{
        book::{normalize_book, RawBookInput},
        symbol::normalize_symbol,
        trade::{
            normalize_binance_agg_trade, normalize_bybit_trade, normalize_okx_trade,
            BinanceAggTrade, BybitTrade, OkxTrade,
        },
    },
    types::market::{AggressorSide, Venue},
};

#[test]
fn symbol_normalizer_maps_btc_perps() {
    assert_eq!(
        normalize_symbol(Venue::Binance, "BTCUSDT"),
        Some("BTC-PERP")
    );
    assert_eq!(normalize_symbol(Venue::Bybit, "BTCUSDT"), Some("BTC-PERP"));
    assert_eq!(
        normalize_symbol(Venue::Okx, "BTC-USDT-SWAP"),
        Some("BTC-PERP")
    );
    assert_eq!(normalize_symbol(Venue::Binance, "ETHUSDT"), None);
}

#[test]
fn trade_normalizers_map_aggressor_side_and_size_usd() {
    let binance_sell = normalize_binance_agg_trade(BinanceAggTrade {
        s: "BTCUSDT".to_string(),
        a: Some(serde_json::json!(1)),
        p: "100000".to_string(),
        q: "0.5".to_string(),
        trade_time: Some(1),
        event_time: None,
        m: true,
    })
    .expect("trade");
    assert_eq!(binance_sell.aggressor_side, AggressorSide::Sell);
    assert_eq!(binance_sell.size_usd, 50_000.0);

    let bybit_buy = normalize_bybit_trade(BybitTrade {
        s: "BTCUSDT".to_string(),
        trade_time: Some(1),
        p: "100".to_string(),
        v: "2".to_string(),
        side: "Buy".to_string(),
        i: None,
    })
    .expect("trade");
    assert_eq!(bybit_buy.aggressor_side, AggressorSide::Buy);

    let okx_sell = normalize_okx_trade(OkxTrade {
        inst_id: Some("BTC-USDT-SWAP".to_string()),
        trade_id: None,
        px: "100".to_string(),
        sz: "2".to_string(),
        side: "sell".to_string(),
        ts: Some("1".to_string()),
    })
    .expect("trade");
    assert_eq!(okx_sell.aggressor_side, AggressorSide::Sell);
}

#[test]
fn book_normalizer_computes_mid_spread_depth_and_imbalance() {
    let book = normalize_book(RawBookInput {
        venue: Venue::Binance,
        symbol: "BTCUSDT".to_string(),
        ts: 1,
        bids: vec![(99990.0, 1.0), (99900.0, 2.0), (99800.0, 5.0)],
        asks: vec![(100010.0, 3.0), (100050.0, 1.0), (100200.0, 5.0)],
    })
    .expect("book");

    assert_eq!(book.mid, 100_000.0);
    assert_eq!(book.spread_bps, 2.0);
    assert_eq!(book.bid_depth_btc_10bps, 3.0);
    assert_eq!(book.ask_depth_btc_10bps, 4.0);
    assert!((book.imbalance_10bps - (-1.0 / 7.0)).abs() < 1e-9);
}
