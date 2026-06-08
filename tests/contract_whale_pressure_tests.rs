use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};

use btc_toxic_flow_monitor_rs::{
    contract_whale_monitor::{
        aggregator::{aggregate_1s_buckets, rolling_window_stats},
        detector::detect_contract_whale_signal,
        types::{ContractExchange, ContractTrade, ContractTradeSide},
    },
    storage::{contract_whale_repo::ContractWhaleRepo, SqliteStore},
};

#[test]
fn cwm_high_frequency_trade_input_keeps_aggregation_bounded() {
    const SECONDS: usize = 600;
    const TRADES_PER_SECOND: usize = 5_000;

    let store = temp_store("cwm-high-frequency");
    let base_ts = 1_712_400_000_000_i64;
    let started = Instant::now();
    let mut all_buckets = Vec::with_capacity(SECONDS * 3);
    let mut generated_signals = 0usize;

    for second in 0..SECONDS {
        let ts = base_ts + second as i64 * 1_000;
        let trades = synthetic_second_trades(ts, TRADES_PER_SECOND);
        let buckets = aggregate_1s_buckets(&trades);
        store.upsert_contract_flow_buckets(&buckets).unwrap();
        all_buckets.extend(buckets);

        if second % 15 == 14 {
            if let Some(stats) =
                rolling_window_stats(&all_buckets, "BTC", 15, ts, Some(0.18), Some(7.2), 90)
            {
                if detect_contract_whale_signal(&stats).is_some() {
                    generated_signals += 1;
                }
            }
        }
    }

    let elapsed = started.elapsed();
    let rows = store
        .list_contract_flow_buckets_between("BTC", base_ts, base_ts + SECONDS as i64 * 1_000)
        .unwrap();

    assert_eq!(all_buckets.len(), SECONDS * 3);
    assert_eq!(rows.len(), SECONDS * 3);
    assert!(generated_signals > 0);
    assert!(
        elapsed < Duration::from_secs(30),
        "high-frequency smoke took {elapsed:?}"
    );
}

fn synthetic_second_trades(ts: i64, count: usize) -> Vec<ContractTrade> {
    (0..count)
        .map(|index| {
            let exchange = match index % 3 {
                0 => ContractExchange::Binance,
                1 => ContractExchange::Okx,
                _ => ContractExchange::Bitfinex,
            };
            let side = if index % 5 == 0 {
                ContractTradeSide::Sell
            } else {
                ContractTradeSide::Buy
            };
            ContractTrade {
                ts,
                exchange,
                symbol: "BTC".to_string(),
                market: "perp".to_string(),
                price: 70_000.0 + (index % 20) as f64,
                qty_btc: 0.04,
                notional_usd: (70_000.0 + (index % 20) as f64) * 0.04,
                side,
                raw_trade_count: Some(1),
            }
        })
        .collect()
}

fn temp_store(name: &str) -> SqliteStore {
    let unique = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_nanos();
    let path = std::env::temp_dir().join(format!(
        "btc-toxic-flow-{name}-{unique}-{}.sqlite",
        std::process::id()
    ));
    let store = SqliteStore::open(path.to_str().unwrap()).unwrap();
    store.migrate().unwrap();
    store
}
