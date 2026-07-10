use btc_toxic_flow_monitor_rs::binance_alt_contract_monitor::{
    flow_state::PerSymbolFlowBook,
    types::{AltContractExchange, AltContractTrade, AltContractTradeSide},
};

fn trade(symbol: &str, ts: i64, notional_usd: f64) -> AltContractTrade {
    AltContractTrade {
        ts,
        exchange: AltContractExchange::Binance,
        symbol: symbol.trim_end_matches("USDT").to_string(),
        product_id: symbol.to_string(),
        price: 100.0,
        qty_base: notional_usd / 100.0,
        notional_usd,
        side: AltContractTradeSide::Buy,
        trade_id: Some(format!("{symbol}-{ts}")),
    }
}

#[test]
fn hot_symbol_flow_does_not_evict_tail_symbol_window_state() {
    let mut book = PerSymbolFlowBook::new(3_900);
    book.ingest(trade("TAILUSDT", 1_000, 100_000.0));
    for second in 1..=3_800 {
        book.ingest(trade("HOTUSDT", second * 1_000, 10_000.0));
    }

    let tail = book.window("TAILUSDT", 60, 3_800_000).expect("tail state");
    assert_eq!(tail.total_notional_usd, 0.0);
    assert_eq!(book.symbol_count(), 2);
    assert!(book.has_symbol("TAILUSDT"));
}
