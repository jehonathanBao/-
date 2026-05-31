use btc_toxic_flow_monitor_rs::alerts::deduper::AlertDeduper;

#[test]
fn deduper_allows_first_and_blocks_within_window() {
    let mut deduper = AlertDeduper::new(30_000);

    assert!(deduper.should_send("buy:5000:binance:alert", 1_000));
    deduper.mark_sent("buy:5000:binance:alert", 1_000);

    assert!(!deduper.should_send("buy:5000:binance:alert", 10_000));
    assert!(deduper.should_send("sell:5000:binance:alert", 10_000));
    assert!(deduper.should_send("buy:1000:binance:alert", 10_000));
    assert!(deduper.should_send("buy:5000:bybit:alert", 10_000));
}

#[test]
fn deduper_allows_again_after_window() {
    let mut deduper = AlertDeduper::new(30_000);

    assert!(deduper.should_send("buy:5000:binance:alert", 1_000));
    deduper.mark_sent("buy:5000:binance:alert", 1_000);

    assert!(deduper.should_send("buy:5000:binance:alert", 31_000));
}
