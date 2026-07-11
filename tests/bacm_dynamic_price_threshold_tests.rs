use btc_toxic_flow_monitor_rs::binance_alt_contract_monitor::{
    aggregator::adaptive_price_threshold_pct,
    types::{AltContractExchange, AltContractTrade, AltContractTradeSide},
};

fn trade(ts: i64, price: f64) -> AltContractTrade {
    AltContractTrade {
        ts,
        exchange: AltContractExchange::Binance,
        symbol: "ALT".to_string(),
        product_id: "ALTUSDT".to_string(),
        price,
        qty_base: 100.0,
        notional_usd: price * 100.0,
        side: AltContractTradeSide::Buy,
        trade_id: Some(format!("ALT-{ts}")),
    }
}

#[test]
fn insufficient_price_samples_fall_back_to_base_threshold() {
    let trades = vec![
        trade(1_000, 100.0),
        trade(2_000, 100.01),
        trade(3_000, 100.02),
    ];

    assert_eq!(adaptive_price_threshold_pct(&trades), None);
}

#[test]
fn high_micro_volatility_raises_the_price_threshold() {
    let trades = vec![
        trade(1_000, 100.0),
        trade(2_000, 100.5),
        trade(3_000, 99.7),
        trade(4_000, 100.4),
        trade(5_000, 99.9),
    ];

    assert!(adaptive_price_threshold_pct(&trades).is_some_and(|threshold| threshold > 0.05));
}
