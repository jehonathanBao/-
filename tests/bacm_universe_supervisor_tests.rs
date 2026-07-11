use btc_toxic_flow_monitor_rs::binance_alt_contract_monitor::collector::{
    reconcile_shard_layout, universe_changed,
};

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

#[test]
fn adding_a_symbol_keeps_existing_shards_and_only_changes_one_assignment() {
    let current = vec![
        (0..200).map(|index| format!("HOT{index:03}USDT")).collect(),
        (0..200)
            .map(|index| format!("TAIL{index:03}USDT"))
            .collect(),
    ];
    let mut desired = current.iter().flatten().cloned().collect::<Vec<_>>();
    desired.push("NEWUSDT".to_string());

    let next = reconcile_shard_layout(&current, &desired);

    assert_eq!(next.len(), 3);
    assert_eq!(next[0], current[0]);
    assert_eq!(next[1], current[1]);
    assert_eq!(next[2], vec!["NEWUSDT".to_string()]);
}

#[test]
fn removing_a_symbol_does_not_require_rebuilding_surviving_symbol_sets() {
    let current = vec![
        vec!["AAAUSDT".to_string(), "BBBUSDT".to_string()],
        vec!["CCCUSDT".to_string()],
    ];

    let next = reconcile_shard_layout(&current, &["AAAUSDT".to_string(), "BBBUSDT".to_string()]);

    assert_eq!(
        next,
        vec![vec!["AAAUSDT".to_string(), "BBBUSDT".to_string()]]
    );
}
