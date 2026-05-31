use std::collections::BTreeMap;

use btc_toxic_flow_monitor_rs::{
    toxicity::cross_venue_confirmation::{cross_venue_confirmed, leader_venue},
    types::{flow::VenueFlowBreakdown, market::Venue, toxic::ToxicDirection},
};

#[test]
fn buy_direction_confirms_when_two_venues_buy() {
    let breakdown = breakdown(vec![
        (Venue::Binance, 100.0, 0.0),
        (Venue::Bybit, 50.0, 0.0),
        (Venue::Okx, 0.0, 20.0),
    ]);

    assert!(cross_venue_confirmed(ToxicDirection::Buy, &breakdown, 2));
}

#[test]
fn sell_direction_confirms_when_two_venues_sell() {
    let breakdown = breakdown(vec![
        (Venue::Binance, 0.0, 100.0),
        (Venue::Okx, 0.0, 50.0),
        (Venue::Bybit, 20.0, 0.0),
    ]);

    assert!(cross_venue_confirmed(ToxicDirection::Sell, &breakdown, 2));
}

#[test]
fn single_venue_does_not_confirm() {
    let breakdown = breakdown(vec![
        (Venue::Binance, 100.0, 0.0),
        (Venue::Bybit, 0.0, 0.0),
        (Venue::Okx, 0.0, 0.0),
    ]);

    assert!(!cross_venue_confirmed(ToxicDirection::Buy, &breakdown, 2));
}

#[test]
fn mixed_flow_without_directional_majority_does_not_confirm() {
    let breakdown = breakdown(vec![
        (Venue::Binance, 100.0, 100.0),
        (Venue::Bybit, 50.0, 50.0),
        (Venue::Okx, 20.0, 20.0),
    ]);

    assert!(!cross_venue_confirmed(ToxicDirection::Buy, &breakdown, 2));
    assert!(!cross_venue_confirmed(ToxicDirection::Sell, &breakdown, 2));
}

#[test]
fn leader_venue_chooses_direction_side_maximum() {
    let buy_breakdown = breakdown(vec![
        (Venue::Binance, 300.0, 50.0),
        (Venue::Bybit, 500.0, 10.0),
        (Venue::Okx, 200.0, 80.0),
    ]);

    assert_eq!(
        leader_venue(ToxicDirection::Buy, &buy_breakdown),
        Some(Venue::Bybit)
    );

    let sell_breakdown = breakdown(vec![
        (Venue::Binance, 50.0, 300.0),
        (Venue::Bybit, 10.0, 500.0),
        (Venue::Okx, 80.0, 200.0),
    ]);

    assert_eq!(
        leader_venue(ToxicDirection::Sell, &sell_breakdown),
        Some(Venue::Bybit)
    );
}

fn breakdown(entries: Vec<(Venue, f64, f64)>) -> BTreeMap<String, VenueFlowBreakdown> {
    let mut map = BTreeMap::new();
    for venue in Venue::ALL {
        map.insert(venue.as_key().to_string(), empty_breakdown());
    }
    for (venue, buy, sell) in entries {
        map.insert(
            venue.as_key().to_string(),
            VenueFlowBreakdown {
                aggressive_buy_btc: buy,
                aggressive_sell_btc: sell,
                aggressive_buy_usd: buy * 100.0,
                aggressive_sell_usd: sell * 100.0,
                net_aggressive_btc: buy - sell,
                abs_aggressive_btc: buy + sell,
                trade_count: 1,
                buy_trade_count: if buy > 0.0 { 1 } else { 0 },
                sell_trade_count: if sell > 0.0 { 1 } else { 0 },
                last_trade_ts: Some(1),
            },
        );
    }
    map
}

fn empty_breakdown() -> VenueFlowBreakdown {
    VenueFlowBreakdown {
        aggressive_buy_btc: 0.0,
        aggressive_sell_btc: 0.0,
        aggressive_buy_usd: 0.0,
        aggressive_sell_usd: 0.0,
        net_aggressive_btc: 0.0,
        abs_aggressive_btc: 0.0,
        trade_count: 0,
        buy_trade_count: 0,
        sell_trade_count: 0,
        last_trade_ts: None,
    }
}
