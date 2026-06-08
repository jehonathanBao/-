use btc_toxic_flow_monitor_rs::spot_whale_monitor::{
    config::SpotWhaleRuntimeConfig,
    detector::{detect_spot_whale_signal_with_config, discord_gate},
    normalizer::{
        normalize_binance_spot_trade, normalize_coinbase_market_trades_json, BinanceSpotAggTrade,
    },
    types::{
        SpotExchange, SpotExchangeContribution, SpotTradeSide, SpotWhaleSeverity,
        SpotWhaleSignalType, SpotWhaleWindowStats,
    },
};

#[test]
fn spot_normalizers_map_taker_direction_and_units() {
    let maker_buy = normalize_binance_spot_trade(BinanceSpotAggTrade {
        s: "BTCUSDT".to_string(),
        p: "70000".to_string(),
        q: "1.25".to_string(),
        trade_time: Some(1_700_000_000_000),
        event_time: None,
        m: true,
        a: Some(serde_json::json!(42)),
        t: None,
    })
    .expect("binance maker buy");
    let taker_buy = normalize_binance_spot_trade(BinanceSpotAggTrade {
        s: "ETHUSDT".to_string(),
        p: "3500".to_string(),
        q: "12".to_string(),
        trade_time: Some(1_700_000_000_001),
        event_time: None,
        m: false,
        a: None,
        t: Some(serde_json::json!(43)),
    })
    .expect("binance taker buy");

    assert_eq!(maker_buy.exchange, SpotExchange::Binance);
    assert_eq!(maker_buy.symbol, "BTC");
    assert_eq!(maker_buy.side, SpotTradeSide::Sell);
    assert_eq!(maker_buy.notional_usd, 87_500.0);
    assert_eq!(taker_buy.symbol, "ETH");
    assert_eq!(taker_buy.side, SpotTradeSide::Buy);
    assert_eq!(taker_buy.notional_usd, 42_000.0);

    let coinbase_payload = serde_json::json!({
        "channel": "market_trades",
        "events": [{
            "type": "update",
            "trades": [
                {"trade_id": "1", "product_id": "BTC-USD", "price": "70010", "size": "0.5", "side": "BUY", "time": "2026-06-08T00:00:00Z"},
                {"trade_id": "2", "product_id": "ETH-USD", "price": "3501", "size": "2.0", "side": "SELL", "time": "2026-06-08T00:00:00Z"}
            ]
        }]
    });
    let trades = normalize_coinbase_market_trades_json(&coinbase_payload);

    assert_eq!(trades.len(), 2);
    assert_eq!(trades[0].exchange, SpotExchange::Coinbase);
    assert_eq!(trades[0].side, SpotTradeSide::Sell);
    assert_eq!(trades[1].symbol, "ETH");
    assert_eq!(trades[1].side, SpotTradeSide::Buy);
}

#[test]
fn spot_detector_generates_critical_multi_exchange_buy_signal() {
    let config = SpotWhaleRuntimeConfig::default();
    let stats = high_conviction_stats();

    let signal = detect_spot_whale_signal_with_config(&stats, &config).expect("spot signal");

    assert_eq!(signal.symbol, "BTC");
    assert_eq!(signal.signal_type, SpotWhaleSignalType::SpotAggressiveBuy);
    assert_eq!(signal.severity, SpotWhaleSeverity::Critical);
    assert!(signal.score >= 80);
    assert!(signal.discord_eligible);
    assert_eq!(signal.discord_reason, "critical_or_s_gate");
    assert!(signal.read_only);
    assert!(signal.analysis_only);
    assert!(!signal.execution_enabled);
}

#[test]
fn spot_discord_gate_rejects_medium_and_low_quality() {
    let (eligible, reason) = discord_gate(SpotWhaleSeverity::Medium, 95, true, 95);
    assert!(!eligible);
    assert_eq!(reason, "medium_or_low_display_only");

    let (eligible, reason) = discord_gate(SpotWhaleSeverity::Critical, 95, true, 69);
    assert!(!eligible);
    assert_eq!(reason, "data_quality_display_only");
}

fn high_conviction_stats() -> SpotWhaleWindowStats {
    SpotWhaleWindowStats {
        symbol: "BTC".to_string(),
        window_sec: 15,
        ts: 1_700_000_015_000,
        buy_volume_base: 850.0,
        sell_volume_base: 120.0,
        total_volume_base: 970.0,
        net_volume_base: 730.0,
        total_notional_usd: 67_900_000.0,
        dominance: 730.0 / 970.0,
        price_move_pct: Some(0.22),
        coinbase_premium_pct: Some(0.03),
        exchange_count: 2,
        main_exchange: Some("binance".to_string()),
        exchanges: vec![
            contribution("binance", 520.0, 80.0),
            contribution("coinbase", 330.0, 40.0),
        ],
        dynamic_multiple: Some(9.0),
        multi_exchange_confirmed: true,
        data_quality: 92,
        startup_age_ms: Some(120_000),
    }
}

fn contribution(exchange: &str, buy: f64, sell: f64) -> SpotExchangeContribution {
    let total = buy + sell;
    let net = buy - sell;
    SpotExchangeContribution {
        exchange: exchange.to_string(),
        buy_volume_base: buy,
        sell_volume_base: sell,
        total_volume_base: total,
        buy_notional_usd: buy * 70_000.0,
        sell_notional_usd: sell * 70_000.0,
        total_notional_usd: total * 70_000.0,
        net_volume_base: net,
        dominance: net.abs() / total,
        trade_count: 10,
    }
}
