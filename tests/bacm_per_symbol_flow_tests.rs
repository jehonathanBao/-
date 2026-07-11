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

#[test]
fn baseline_uses_only_the_same_symbol_historical_windows() {
    let mut book = PerSymbolFlowBook::new(3_900);
    for index in 0..20_i64 {
        book.ingest(trade("TAILUSDT", index * 60_000 + 1_000, 100.0));
        book.ingest(trade("HOTUSDT", index * 60_000 + 1_000, 1_000_000.0));
    }
    book.ingest(trade("TAILUSDT", 20 * 60_000 + 1_000, 1_000.0));

    let baseline = book
        .baseline("TAILUSDT", 60, 20 * 60_000 + 59_000, 3_600, 5)
        .expect("tail baseline");

    assert_eq!(baseline.sample_count, 20);
    assert!(baseline.available);
    assert!((baseline.median_notional_usd - 100.0).abs() < f64::EPSILON);
    assert!((baseline.dynamic_multiple - 10.0).abs() < f64::EPSILON);
}

#[test]
fn baseline_reports_unavailable_until_same_symbol_has_enough_history() {
    let mut book = PerSymbolFlowBook::new(3_900);
    for index in 0..4_i64 {
        book.ingest(trade("TAILUSDT", index * 60_000 + 1_000, 100.0));
    }
    book.ingest(trade("TAILUSDT", 4 * 60_000 + 1_000, 1_000.0));

    let baseline = book
        .baseline("TAILUSDT", 60, 4 * 60_000 + 59_000, 3_600, 5)
        .expect("tail baseline");

    assert_eq!(baseline.sample_count, 4);
    assert!(!baseline.available);
    assert_eq!(baseline.dynamic_multiple, 0.0);
}

#[test]
fn a_window_query_only_uses_its_symbol_state_at_full_universe_scale() {
    let mut book = PerSymbolFlowBook::new(3_900);
    for index in 0..10_000 {
        book.ingest(trade(
            &format!("COIN{index}USDT"),
            1_000,
            100.0 + index as f64,
        ));
    }

    let tail = book
        .window("COIN9999USDT", 60, 1_000)
        .expect("tail symbol state");

    assert_eq!(tail.trade_count, 1);
    assert_eq!(tail.total_notional_usd, 10_099.0);
    assert_eq!(book.symbol_count(), 10_000);
}
