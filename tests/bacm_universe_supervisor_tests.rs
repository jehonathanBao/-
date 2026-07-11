use btc_toxic_flow_monitor_rs::binance_alt_contract_monitor::collector::universe_changed;

#[test]
fn a_new_symbol_requires_shard_reconciliation_without_a_process_restart() {
    assert!(universe_changed(
        &["AAAUSDT".to_string(), "BBBUSDT".to_string()],
        &[
            "AAAUSDT".to_string(),
            "BBBUSDT".to_string(),
            "NEWUSDT".to_string(),
        ],
    ));
}

#[test]
fn a_delisted_symbol_requires_shard_reconciliation() {
    assert!(universe_changed(
        &["AAAUSDT".to_string(), "OLDUSDT".to_string()],
        &["AAAUSDT".to_string()],
    ));
}

#[test]
fn reorder_only_does_not_reconnect_healthy_shards() {
    assert!(!universe_changed(
        &["AAAUSDT".to_string(), "BBBUSDT".to_string()],
        &["BBBUSDT".to_string(), "AAAUSDT".to_string()],
    ));
}
