use btc_toxic_flow_monitor_rs::binance_alt_contract_monitor::lme::{
    score_liquidity_microstructure, LiquidityMicrostructureInput,
};

#[test]
fn detects_absorption_when_large_flow_does_not_move_price() {
    let result = score_liquidity_microstructure(&LiquidityMicrostructureInput {
        aggressive_buy_notional_usd: 100_000.0,
        aggressive_sell_notional_usd: 2_000_000.0,
        price_move_pct: -0.01,
        replenishment_ratio: Some(0.95),
        ..LiquidityMicrostructureInput::default()
    });

    assert_eq!(result.behavior, "Absorption_Buy");
    assert!(result.absorption_strength >= 80.0, "{result:?}");
    assert!(result
        .explanation_tags
        .iter()
        .any(|tag| tag == "price_absorption"));
    assert!(!result.direct_discord_gate);
    assert!(result.read_only);
}

#[test]
fn detects_liquidity_sweep_up_when_ask_side_is_consumed() {
    let result = score_liquidity_microstructure(&LiquidityMicrostructureInput {
        aggressive_buy_notional_usd: 3_000_000.0,
        aggressive_sell_notional_usd: 300_000.0,
        price_move_pct: 0.42,
        ask_depth_1pct_usd: Some(600_000.0),
        previous_ask_depth_1pct_usd: Some(2_400_000.0),
        spread_bps: Some(8.0),
        previous_spread_bps: Some(3.0),
        ..LiquidityMicrostructureInput::default()
    });

    assert_eq!(result.behavior, "LiquiditySweepUp");
    assert_eq!(result.market_control, "buyer_side_control");
    assert!(result.order_flow_pressure >= 80.0, "{result:?}");
    assert!(result
        .explanation_tags
        .iter()
        .any(|tag| tag == "liquidity_sweep"));
}

#[test]
fn detects_spoofing_when_large_orders_cancel_without_execution() {
    let result = score_liquidity_microstructure(&LiquidityMicrostructureInput {
        large_order_add_usd: Some(5_000_000.0),
        large_order_cancel_usd: Some(4_700_000.0),
        large_order_executed_usd: Some(50_000.0),
        bid_depth_1pct_usd: Some(3_000_000.0),
        ask_depth_1pct_usd: Some(2_900_000.0),
        ..LiquidityMicrostructureInput::default()
    });

    assert_eq!(result.behavior, "SpoofingDetected");
    assert_eq!(result.spoofing_state, "detected");
    assert!(result.spoofing_penalty >= 90.0, "{result:?}");
    assert!(result
        .explanation_tags
        .iter()
        .any(|tag| tag == "spoofing_detected"));
}

#[test]
fn detects_bullish_order_book_imbalance() {
    let result = score_liquidity_microstructure(&LiquidityMicrostructureInput {
        bid_depth_1pct_usd: Some(9_000_000.0),
        ask_depth_1pct_usd: Some(1_000_000.0),
        aggressive_buy_notional_usd: 900_000.0,
        aggressive_sell_notional_usd: 600_000.0,
        price_move_pct: 0.02,
        ..LiquidityMicrostructureInput::default()
    });

    assert_eq!(result.behavior, "BullishImbalance");
    assert!(result.imbalance >= 0.7, "{result:?}");
    assert!(result
        .explanation_tags
        .iter()
        .any(|tag| tag == "bullish_imbalance"));
}
