use btc_toxic_flow_monitor_rs::{
    toxicity::markout_engine::MarkoutEngine,
    types::market::{AggressorSide, NormalizedTrade, Venue},
};

#[test]
fn buy_markout_is_positive_when_future_mid_rises() {
    let mut engine = MarkoutEngine::new(vec![1000], 120_000, 5_000);
    engine.on_trade(&trade(
        Venue::Binance,
        0,
        100.0,
        1.0,
        AggressorSide::Buy,
        "a",
    ));
    engine.resolve_due_samples(1000, |_| Some(101.0));

    let state = engine.get_state(1000, true);
    let buy = &state.summaries["1000"].buy;

    assert_eq!(buy.count, 1);
    assert_eq!(buy.positive_count, 1);
    assert_eq!(buy.positive_volume_btc, 1.0);
    assert_eq!(buy.volume_weighted_markout_bps, Some(100.0));
}

#[test]
fn buy_markout_is_negative_when_future_mid_falls() {
    let mut engine = MarkoutEngine::new(vec![1000], 120_000, 5_000);
    engine.on_trade(&trade(
        Venue::Binance,
        0,
        100.0,
        1.0,
        AggressorSide::Buy,
        "a",
    ));
    engine.resolve_due_samples(1000, |_| Some(99.0));

    let state = engine.get_state(1000, true);
    let buy = &state.summaries["1000"].buy;

    assert_eq!(buy.negative_count, 1);
    assert_eq!(buy.negative_volume_btc, 1.0);
    assert_eq!(buy.volume_weighted_markout_bps, Some(-100.0));
}

#[test]
fn sell_markout_is_positive_when_future_mid_falls() {
    let mut engine = MarkoutEngine::new(vec![1000], 120_000, 5_000);
    engine.on_trade(&trade(
        Venue::Bybit,
        0,
        100.0,
        2.0,
        AggressorSide::Sell,
        "a",
    ));
    engine.resolve_due_samples(1000, |_| Some(99.0));

    let state = engine.get_state(1000, true);
    let sell = &state.summaries["1000"].sell;

    assert_eq!(sell.count, 1);
    assert_eq!(sell.positive_count, 1);
    assert_eq!(sell.positive_volume_btc, 2.0);
    assert_eq!(sell.volume_weighted_markout_bps, Some(100.0));
}

#[test]
fn sell_markout_is_negative_when_future_mid_rises() {
    let mut engine = MarkoutEngine::new(vec![1000], 120_000, 5_000);
    engine.on_trade(&trade(
        Venue::Bybit,
        0,
        100.0,
        2.0,
        AggressorSide::Sell,
        "a",
    ));
    engine.resolve_due_samples(1000, |_| Some(101.0));

    let state = engine.get_state(1000, true);
    let sell = &state.summaries["1000"].sell;

    assert_eq!(sell.negative_count, 1);
    assert_eq!(sell.negative_volume_btc, 2.0);
    assert_eq!(sell.volume_weighted_markout_bps, Some(-100.0));
}

#[test]
fn each_trade_creates_three_pending_samples_and_resolves_by_horizon() {
    let mut engine = MarkoutEngine::new(vec![1000, 5000, 15000], 120_000, 5_000);
    engine.on_trade(&trade(Venue::Okx, 0, 100.0, 1.0, AggressorSide::Buy, "a"));

    let pending = engine.get_state(999, true).quality;
    assert_eq!(pending.pending_samples, 3);
    assert_eq!(pending.resolved_samples, 0);

    engine.resolve_due_samples(1000, |_| Some(101.0));
    let partial = engine.get_state(1000, true).quality;
    assert_eq!(partial.pending_samples, 2);
    assert_eq!(partial.resolved_samples, 1);
}

#[test]
fn due_sample_without_future_mid_expires_after_grace() {
    let mut engine = MarkoutEngine::new(vec![1000], 120_000, 5_000);
    engine.on_trade(&trade(Venue::Okx, 0, 100.0, 1.0, AggressorSide::Buy, "a"));

    engine.resolve_due_samples(6000, |_| None);
    assert_eq!(engine.get_state(6000, false).quality.pending_samples, 1);

    engine.resolve_due_samples(6001, |_| None);
    let quality = engine.get_state(6001, false).quality;
    assert_eq!(quality.pending_samples, 0);
    assert_eq!(quality.expired_samples, 1);
}

#[test]
fn duplicate_trade_id_and_invalid_values_do_not_create_samples() {
    let mut engine = MarkoutEngine::new(vec![1000, 5000, 15000], 120_000, 5_000);
    let first = trade(Venue::Binance, 0, 100.0, 1.0, AggressorSide::Buy, "same");
    engine.on_trade(&first);
    engine.on_trade(&first);

    let mut bad_price = first.clone();
    bad_price.trade_id = Some("bad-price".to_string());
    bad_price.price = 0.0;
    engine.on_trade(&bad_price);

    let mut bad_size = first;
    bad_size.trade_id = Some("bad-size".to_string());
    bad_size.size_btc = 0.0;
    engine.on_trade(&bad_size);

    assert_eq!(engine.sample_count(), 3);
}

#[test]
fn volume_weighted_stats_and_venue_breakdown_are_computed() {
    let mut engine = MarkoutEngine::new(vec![1000], 120_000, 5_000);
    engine.on_trade(&trade(
        Venue::Binance,
        0,
        100.0,
        1.0,
        AggressorSide::Buy,
        "a",
    ));
    engine.on_trade(&trade(Venue::Bybit, 1, 100.0, 3.0, AggressorSide::Buy, "b"));
    engine.resolve_due_samples(1001, |ts| match ts {
        1000 => Some(101.0),
        1001 => Some(99.0),
        _ => None,
    });

    let state = engine.get_state(1001, true);
    let summary = &state.summaries["1000"];

    assert_eq!(summary.buy.count, 2);
    assert_eq!(summary.buy.volume_btc, 4.0);
    assert_eq!(summary.buy.avg_markout_bps, Some(0.0));
    assert_eq!(summary.buy.volume_weighted_markout_bps, Some(-50.0));
    assert_eq!(summary.buy.positive_volume_btc, 1.0);
    assert_eq!(summary.buy.negative_volume_btc, 3.0);
    assert!(summary.venue_breakdown.contains_key("binance"));
    assert!(summary.venue_breakdown.contains_key("bybit"));
    assert!(summary.venue_breakdown.contains_key("okx"));
    assert_eq!(summary.venue_breakdown["binance"].buy.positive_count, 1);
    assert_eq!(summary.venue_breakdown["bybit"].buy.negative_count, 1);
    assert_eq!(summary.venue_breakdown["okx"].buy.count, 0);
}

#[test]
fn clear_removes_samples() {
    let mut engine = MarkoutEngine::new(vec![1000], 120_000, 5_000);
    engine.on_trade(&trade(
        Venue::Binance,
        0,
        100.0,
        1.0,
        AggressorSide::Buy,
        "a",
    ));
    assert_eq!(engine.sample_count(), 1);

    engine.clear();

    assert_eq!(engine.sample_count(), 0);
}

fn trade(
    venue: Venue,
    ts: i64,
    price: f64,
    size_btc: f64,
    aggressor_side: AggressorSide,
    trade_id: &str,
) -> NormalizedTrade {
    NormalizedTrade {
        venue,
        symbol: "BTC-PERP".to_string(),
        ts,
        price,
        size_btc,
        size_usd: price * size_btc,
        aggressor_side,
        trade_id: Some(trade_id.to_string()),
    }
}
