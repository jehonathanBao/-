use std::collections::BTreeMap;

use btc_toxic_flow_monitor_rs::{
    alerts::formatter::format_alert_message,
    types::{
        market::Venue,
        sweep::LiquidityThinnessResult,
        toxic::{
            ToxicDirection, ToxicEvent, ToxicQuality, ToxicSeverity, ToxicState, ToxicVolumeResult,
        },
    },
};

#[test]
fn buy_alert_message_formats_expected_fields() {
    let state = build_state(build_event(
        ToxicDirection::Buy,
        ToxicSeverity::Alert,
        Some(Venue::Binance),
        true,
    ));

    let message = format_alert_message(state.latest_event.as_ref().unwrap(), &state);
    assert!(message.contains("Severity: ALERT"));
    assert!(message.contains("Direction: BUY TOXIC"));
    assert!(message.contains("Toxic Volume: 1,284.2 BTC"));
    assert!(message.contains("Threshold: 1,000 BTC"));
    assert!(message.contains("1s: +2.1 bps"));
    assert!(message.contains("5s: +4.8 bps"));
    assert!(message.contains("Leader Venue: BINANCE"));
    assert!(message.contains("Ask Thin: true"));
    assert!(message.contains("Bid Thin: false"));
    assert!(message.contains("Spread Widened: true"));
    assert!(message.contains("Confirmed: true"));
    assert_eq!(
        message
            .lines()
            .filter(|line| line.starts_with("- "))
            .count(),
        10
    );
}

#[test]
fn sell_alert_message_handles_missing_leader() {
    let state = build_state(build_event(
        ToxicDirection::Sell,
        ToxicSeverity::Extreme,
        None,
        false,
    ));

    let message = format_alert_message(state.latest_event.as_ref().unwrap(), &state);
    assert!(message.contains("Severity: EXTREME"));
    assert!(message.contains("Direction: SELL TOXIC"));
    assert!(message.contains("Leader Venue: unknown"));
    assert!(message.contains("Bid Thin: true"));
    assert!(message.contains("Confirmed: false"));
}

fn build_state(event: ToxicEvent) -> ToxicState {
    let result = ToxicVolumeResult {
        symbol: "BTC-PERP".to_string(),
        window_ms: event.window_ms,
        ts: event.ts,
        direction: event.direction,
        severity: event.severity,
        toxic_ratio: 0.82,
        toxic_volume_btc: event.toxic_volume_btc,
        threshold_btc: event.threshold_btc,
        alert_triggered: true,
        aggressive_buy_btc: event.aggressive_buy_btc,
        aggressive_sell_btc: event.aggressive_sell_btc,
        net_aggressive_btc: event.net_aggressive_btc,
        abs_aggressive_btc: event.abs_aggressive_btc,
        markout_1s_bps: event.markout_1s_bps,
        markout_5s_bps: event.markout_5s_bps,
        markout_confirmed: true,
        sweep_detected: event.sweep_detected,
        liquidity_thin: event.liquidity_thin,
        liquidity: event.liquidity.clone(),
        cross_venue_confirmed: event.cross_venue_confirmed,
        vpin_enabled: event.vpin_enabled,
        vpin: event.vpin,
        vpin_zscore: event.vpin_zscore,
        vpin_spike: event.vpin_spike,
        vpin_high: event.vpin_high,
        vpin_extreme: event.vpin_extreme,
        liquidation_enabled: event.liquidation_enabled,
        nearest_cluster_side: event.nearest_cluster_side,
        cluster_distance_bps: event.cluster_distance_bps,
        cluster_notional_usd: event.cluster_notional_usd,
        cluster_density: event.cluster_density,
        liq_hunt_pressure: event.liq_hunt_pressure,
        liq_cluster_nearby: event.liq_cluster_nearby,
        possible_liq_hunt_setup: event.possible_liq_hunt_setup,
        leader_venue: event.leader_venue,
        venue_breakdown: BTreeMap::new(),
        reason_codes: event.reason_codes.clone(),
    };

    let mut results = BTreeMap::new();
    results.insert(event.window_ms.to_string(), result);

    ToxicState {
        symbol: "BTC-PERP".to_string(),
        updated_at: event.ts,
        threshold_btc: event.threshold_btc,
        windows_ms: vec![1000, 5000, 15000, 60000],
        results,
        latest_event: Some(event.clone()),
        recent_events: vec![event],
        quality: ToxicQuality {
            has_flow: true,
            has_markout: true,
            has_sweep: true,
            has_liquidation: true,
            liquidation: None,
            active_venues: vec![Venue::Binance, Venue::Bybit, Venue::Okx],
            stale_venues: Vec::new(),
        },
    }
}

fn build_event(
    direction: ToxicDirection,
    severity: ToxicSeverity,
    leader_venue: Option<Venue>,
    liquidity_thin: bool,
) -> ToxicEvent {
    ToxicEvent {
        id: "event-1".to_string(),
        ts: 1_760_000_000_000,
        symbol: "BTC-PERP".to_string(),
        direction,
        severity,
        toxic_volume_btc: if matches!(direction, ToxicDirection::Buy) {
            1_284.2
        } else {
            1_120.0
        },
        threshold_btc: 1_000.0,
        window_ms: 5_000,
        leader_venue,
        aggressive_buy_btc: if matches!(direction, ToxicDirection::Buy) {
            1_566.0
        } else {
            100.0
        },
        aggressive_sell_btc: if matches!(direction, ToxicDirection::Sell) {
            1_120.0
        } else {
            220.0
        },
        net_aggressive_btc: if matches!(direction, ToxicDirection::Buy) {
            1_346.0
        } else {
            -1_020.0
        },
        abs_aggressive_btc: 1_786.0,
        markout_1s_bps: Some(if matches!(direction, ToxicDirection::Buy) {
            2.1
        } else {
            1.8
        }),
        markout_5s_bps: Some(if matches!(direction, ToxicDirection::Buy) {
            4.8
        } else {
            3.7
        }),
        sweep_detected: true,
        liquidity_thin,
        liquidity: Some(LiquidityThinnessResult {
            symbol: "BTC-PERP".to_string(),
            window_ms: 5_000,
            bid_depth_start_btc: Some(1_000.0),
            bid_depth_end_btc: Some(if matches!(direction, ToxicDirection::Sell) {
                600.0
            } else {
                980.0
            }),
            ask_depth_start_btc: Some(1_000.0),
            ask_depth_end_btc: Some(if matches!(direction, ToxicDirection::Buy) {
                600.0
            } else {
                980.0
            }),
            bid_depth_drop_ratio: Some(if matches!(direction, ToxicDirection::Sell) {
                0.4
            } else {
                0.02
            }),
            ask_depth_drop_ratio: Some(if matches!(direction, ToxicDirection::Buy) {
                0.4
            } else {
                0.02
            }),
            spread_start_bps: Some(2.0),
            spread_end_bps: Some(3.5),
            spread_widen_ratio: Some(0.75),
            bid_thin: matches!(direction, ToxicDirection::Sell),
            ask_thin: matches!(direction, ToxicDirection::Buy),
            spread_widened: true,
            reason_codes: vec!["spread_widened".to_string()],
        }),
        cross_venue_confirmed: matches!(direction, ToxicDirection::Buy),
        vpin_enabled: true,
        vpin: Some(0.82),
        vpin_zscore: Some(2.8),
        vpin_spike: true,
        vpin_high: false,
        vpin_extreme: false,
        liquidation_enabled: true,
        nearest_cluster_side: Some(if matches!(direction, ToxicDirection::Buy) {
            btc_toxic_flow_monitor_rs::types::liquidation::LiquidationClusterSide::ShortAbove
        } else {
            btc_toxic_flow_monitor_rs::types::liquidation::LiquidationClusterSide::LongBelow
        }),
        cluster_distance_bps: Some(12.0),
        cluster_notional_usd: Some(2_500_000.0),
        cluster_density: Some(0.65),
        liq_hunt_pressure: 0.78,
        liq_cluster_nearby: true,
        possible_liq_hunt_setup: true,
        reason_codes: (0..12).map(|idx| format!("reason_{idx}")).collect(),
    }
}
