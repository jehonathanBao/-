use std::collections::BTreeMap;

use btc_toxic_flow_monitor_rs::{
    toxic_v3::build_btc_liquidation_dashboard,
    types::flow::{DataQuality, FlowState, FlowWindow},
};

#[test]
fn btc_dashboard_builds_read_only_snapshot_from_btc_flow() {
    let dashboard = build_btc_liquidation_dashboard(&flow_state("BTC", 900.0, 100.0), 1_700_000);

    assert_eq!(dashboard.symbol, "BTC");
    assert!(dashboard.read_only);
    assert!(dashboard.live);
    assert_eq!(dashboard.current_price_usd, Some(62_000.0));
    assert!(!dashboard.liquidation_heatmap.is_empty());
    assert!(dashboard.sources.liquidation.contains("proxy"));
}

#[test]
fn btc_dashboard_ignores_non_btc_flow() {
    let dashboard = build_btc_liquidation_dashboard(&flow_state("ETH", 900.0, 100.0), 1_700_000);

    assert_eq!(dashboard.data_status, "non_btc_flow_ignored");
    assert!(!dashboard.live);
    assert!(dashboard.liquidation_heatmap.is_empty());
}

fn flow_state(symbol: &str, buy_btc: f64, sell_btc: f64) -> FlowState {
    let window = FlowWindow {
        symbol: symbol.to_string(),
        window_ms: 60_000,
        now_ts: 1_700_000,
        aggressive_buy_btc: buy_btc,
        aggressive_sell_btc: sell_btc,
        aggressive_buy_usd: buy_btc * 62_000.0,
        aggressive_sell_usd: sell_btc * 62_000.0,
        net_aggressive_btc: buy_btc - sell_btc,
        abs_aggressive_btc: (buy_btc - sell_btc).abs(),
        trade_count: 120,
        buy_trade_count: 90,
        sell_trade_count: 30,
        avg_trade_size_btc: 8.0,
        max_trade_size_btc: 42.0,
        venue_breakdown: BTreeMap::new(),
        mid_start: Some(61_950.0),
        mid_end: Some(62_050.0),
        price_move_bps: Some(16.0),
        spread_bps_median: Some(1.2),
        imbalance_10bps_median: Some(0.25),
        data_quality: DataQuality {
            has_trades: true,
            has_books: true,
            active_venues: vec!["binance".to_string(), "bitfinex".to_string()],
            stale_venues: Vec::new(),
        },
    };

    FlowState {
        symbol: symbol.to_string(),
        updated_at: 1_700_000,
        windows: BTreeMap::from([("60000".to_string(), window)]),
    }
}
