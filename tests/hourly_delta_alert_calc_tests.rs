use btc_toxic_flow_monitor_rs::contract_whale_monitor::hourly_delta_alert::{
    calc::{
        compute_hourly_delta, parse_binance_kline_ws_message, parse_binance_rest_klines,
        should_alert,
    },
    types::{ClosedHourlyKline, HourlyDeltaDirection},
    HourlyDeltaAlertConfig,
};

fn kline(volume: f64, buy: f64, closed: bool) -> ClosedHourlyKline {
    ClosedHourlyKline {
        exchange: "binance".into(),
        symbol: "BTCUSDT".into(),
        interval: "1h".into(),
        open_time_ms: 1_700_000_000_000,
        close_time_ms: 1_700_003_599_999,
        volume_btc: volume,
        taker_buy_btc: buy,
        is_closed: closed,
    }
}

#[test]
fn net_buy_alert_when_delta_above_threshold() {
    // delta = 2*3100 - 5000 = 1200
    let result = compute_hourly_delta(&kline(5_000.0, 3_100.0, true), 1000.0).unwrap();
    assert!((result.delta_btc - 1_200.0).abs() < 1e-9);
    assert_eq!(result.direction, HourlyDeltaDirection::NetBuy);
    assert!(result.above_threshold);
    assert_eq!(result.record_key, "binance:BTCUSDT:1h:1700000000000");
}

#[test]
fn net_sell_alert_when_delta_below_negative_threshold() {
    // delta = 2*2450 - 7700 = -2800
    let result = compute_hourly_delta(&kline(7_700.0, 2_450.0, true), 1000.0).unwrap();
    assert!((result.delta_btc - (-2_800.0)).abs() < 1e-9);
    assert!((result.taker_sell_btc - 5_250.0).abs() < 1e-9);
    assert_eq!(result.direction, HourlyDeltaDirection::NetSell);
    assert!(result.above_threshold);
}

#[test]
fn strict_threshold_excludes_boundary_and_near_miss() {
    assert!(!should_alert(999.99, 1000.0));
    assert!(!should_alert(-999.99, 1000.0));
    assert!(!should_alert(1000.0, 1000.0));
    assert!(!should_alert(-1000.0, 1000.0));
    assert!(should_alert(1000.01, 1000.0));
    assert!(should_alert(-1000.01, 1000.0));

    assert!(
        !compute_hourly_delta(&kline(2_000.0, 1_500.0, true), 1000.0)
            .unwrap()
            .above_threshold
    );
}

#[test]
fn unclosed_kline_never_alerts() {
    assert!(compute_hourly_delta(&kline(10_000.0, 9_000.0, false), 1000.0).is_none());
}

#[test]
fn ignores_non_matching_symbol_or_interval_in_config() {
    let config = HourlyDeltaAlertConfig::default();
    assert!(config.matches_stream("binance", "BTCUSDT", "1h"));
    assert!(!config.matches_stream("binance", "ETHUSDT", "1h"));
    assert!(!config.matches_stream("binance", "BTCUSDT", "15m"));
    assert!(!config.matches_stream("okx", "BTCUSDT", "1h"));
}

#[test]
fn parses_ws_and_rest_kline_payloads() {
    let ws = r#"{"e":"kline","E":1,"s":"BTCUSDT","k":{"t":1700000000000,"T":1700003599999,"s":"BTCUSDT","i":"1h","v":"100","V":"55","x":true}}"#;
    let parsed = parse_binance_kline_ws_message(ws, "binance")
        .unwrap()
        .unwrap();
    assert!(parsed.is_closed);
    assert!((parsed.volume_btc - 100.0).abs() < 1e-9);
    assert!((parsed.taker_buy_btc - 55.0).abs() < 1e-9);

    let rest = serde_json::json!([[
        1700000000000i64,
        "1",
        "2",
        "0.5",
        "1.5",
        "200.5",
        1700003599999i64,
        "0",
        10,
        "80.25",
        "0"
    ]]);
    let rows = parse_binance_rest_klines(&rest, "binance", "BTCUSDT", "1h").unwrap();
    assert_eq!(rows.len(), 1);
    assert!((rows[0].volume_btc - 200.5).abs() < 1e-9);
    assert!((rows[0].taker_buy_btc - 80.25).abs() < 1e-9);
    assert!(rows[0].is_closed);
}
