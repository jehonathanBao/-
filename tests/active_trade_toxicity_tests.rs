use std::collections::BTreeMap;

use btc_toxic_flow_monitor_rs::{
    toxicity::active_trade_toxicity_service::{
        build_active_trade_toxicity_recent, build_active_trade_toxicity_status,
    },
    types::{
        flow::{empty_venue_breakdown, DataQuality, FlowState, FlowWindow},
        market::Venue,
        markout::{DirectionalMarkoutStats, MarkoutQuality, MarkoutState, MarkoutWindowSummary},
        sweep::{SweepDirection, SweepQuality, SweepResult, SweepState},
        toxic_flow::ActiveTradeToxicSignalType,
    },
};

#[test]
fn empty_trades_return_insufficient_data() {
    let recent = build_active_trade_toxicity_recent(
        "BTC-PERP",
        &flow_state(vec![flow_window(5_000, 0.0, 0.0, 0, 0.0, 0.0, Some(0.0))]),
        &empty_sweep_state(),
        &empty_markout_state(),
    );

    assert!(recent.read_only);
    assert!(!recent.runtime_modified);
    assert_eq!(recent.mode, "analysis_only");
    assert_eq!(recent.status, "insufficient_data");
    assert!(recent.signals.is_empty());
    assert!(!recent.no_trade_reasons.is_empty());
}

#[test]
fn balanced_trades_return_neutral() {
    let recent = build_active_trade_toxicity_recent(
        "BTC-PERP",
        &flow_state(vec![flow_window(
            5_000,
            500_000.0,
            500_000.0,
            8,
            25.0,
            8.0,
            Some(0.2),
        )]),
        &empty_sweep_state(),
        &empty_markout_state(),
    );

    assert_eq!(recent.status, "neutral");
    assert_eq!(recent.side_bias, "neutral");
    assert!(
        recent.signals.is_empty()
            || recent
                .signals
                .iter()
                .all(|signal| signal.toxicity_score <= 55)
    );
}

#[test]
fn buy_heavy_aggressive_trades_produce_buy_watch() {
    let recent = build_active_trade_toxicity_recent(
        "BTC-PERP",
        &flow_state(vec![
            flow_window(1_000, 100_000.0, 100_000.0, 2, 5.0, 3.0, Some(0.1)),
            flow_window(5_000, 900_000.0, 100_000.0, 8, 30.0, 9.0, Some(4.5)),
        ]),
        &empty_sweep_state(),
        &empty_markout_state(),
    );

    assert_eq!(recent.status, "buy_toxicity_watch");
    assert_eq!(recent.side_bias, "buy");
    assert!(recent
        .signals
        .iter()
        .any(|signal| signal.signal_type == ActiveTradeToxicSignalType::LargeAggressiveBuy));
    assert!(recent.signals.iter().all(|signal| signal.read_only));
}

#[test]
fn sell_heavy_aggressive_trades_produce_sell_watch() {
    let recent = build_active_trade_toxicity_recent(
        "BTC-PERP",
        &flow_state(vec![
            flow_window(1_000, 120_000.0, 100_000.0, 2, 5.0, 3.0, Some(-0.1)),
            flow_window(5_000, 120_000.0, 980_000.0, 9, 32.0, 10.0, Some(-5.2)),
        ]),
        &empty_sweep_state(),
        &empty_markout_state(),
    );

    assert_eq!(recent.status, "sell_toxicity_watch");
    assert_eq!(recent.side_bias, "sell");
    assert!(recent
        .signals
        .iter()
        .any(|signal| signal.signal_type == ActiveTradeToxicSignalType::LargeAggressiveSell));
}

#[test]
fn buy_sweep_signal_is_exposed_when_sweep_is_detected() {
    let recent = build_active_trade_toxicity_recent(
        "BTC-PERP",
        &flow_state(vec![flow_window(
            5_000,
            850_000.0,
            120_000.0,
            10,
            35.0,
            9.5,
            Some(5.1),
        )]),
        &sweep_state(SweepDirection::Buy, true, Some(6.2)),
        &empty_markout_state(),
    );

    assert!(recent
        .signals
        .iter()
        .any(|signal| signal.signal_type == ActiveTradeToxicSignalType::BuySweep));
}

#[test]
fn absorption_candidate_is_emitted_when_flow_is_strong_but_price_stalls() {
    let recent = build_active_trade_toxicity_recent(
        "BTC-PERP",
        &flow_state(vec![flow_window(
            5_000,
            1_200_000.0,
            200_000.0,
            12,
            40.0,
            10.0,
            Some(0.2),
        )]),
        &empty_sweep_state(),
        &empty_markout_state(),
    );

    assert!(recent
        .signals
        .iter()
        .any(|signal| signal.signal_type == ActiveTradeToxicSignalType::AbsorptionCandidate));
}

#[test]
fn adverse_markout_is_emitted_when_forward_move_reverses_buy_pressure() {
    let recent = build_active_trade_toxicity_recent(
        "BTC-PERP",
        &flow_state(vec![flow_window(
            5_000,
            1_000_000.0,
            150_000.0,
            11,
            42.0,
            11.0,
            Some(3.0),
        )]),
        &empty_sweep_state(),
        &markout_state_for_buy(Some(-3.5), Some(-4.2), None),
    );

    assert!(recent
        .signals
        .iter()
        .any(|signal| signal.signal_type == ActiveTradeToxicSignalType::AdverseMarkout));
}

#[test]
fn status_reports_analysis_only_and_signal_count() {
    let status = build_active_trade_toxicity_status(
        "BTC-PERP",
        &flow_state(vec![flow_window(
            5_000,
            900_000.0,
            100_000.0,
            8,
            30.0,
            9.0,
            Some(4.5),
        )]),
        &empty_sweep_state(),
        &empty_markout_state(),
    );

    assert!(status.read_only);
    assert!(!status.runtime_modified);
    assert!(status.enabled);
    assert_eq!(status.mode, "analysis_only");
    assert!(status.signal_count > 0);
    assert!(status
        .safety_boundary
        .iter()
        .any(|item| item.contains("No order execution")));
}

#[test]
fn closed_one_hour_buy_delta_generates_buy_dominant_signal() {
    let recent = build_active_trade_toxicity_recent(
        "BTC-PERP",
        &flow_state(vec![hour_flow_window(2_100.0, Some(12.0), true, false)]),
        &empty_sweep_state(),
        &empty_markout_state(),
    );

    let signal = recent
        .signals
        .iter()
        .find(|signal| signal.signal_type == ActiveTradeToxicSignalType::OneHourDeltaBuyDominant)
        .expect("1h buy dominant signal");

    assert_eq!(signal.timeframe.as_deref(), Some("1h"));
    assert_eq!(signal.delta, Some(2_100.0));
    assert_eq!(signal.abs_delta, Some(2_100.0));
    assert_eq!(signal.threshold, Some(2_000.0));
    assert_eq!(
        signal.side,
        btc_toxic_flow_monitor_rs::types::toxic_flow::ToxicSide::Buy
    );
    assert!(signal.read_only);
    assert!(!signal.reason.is_empty());
    assert!(signal.toxicity_score <= 100);
}

#[test]
fn closed_one_hour_sell_delta_generates_sell_dominant_signal() {
    let recent = build_active_trade_toxicity_recent(
        "BTC-PERP",
        &flow_state(vec![hour_flow_window(-2_300.0, Some(-15.0), true, false)]),
        &empty_sweep_state(),
        &empty_markout_state(),
    );

    assert!(recent
        .signals
        .iter()
        .any(|signal| signal.signal_type == ActiveTradeToxicSignalType::OneHourDeltaSellDominant));
}

#[test]
fn one_hour_delta_below_threshold_does_not_generate_signal() {
    let recent = build_active_trade_toxicity_recent(
        "BTC-PERP",
        &flow_state(vec![hour_flow_window(1_999.0, Some(10.0), true, false)]),
        &empty_sweep_state(),
        &empty_markout_state(),
    );

    assert!(!recent.signals.iter().any(|signal| {
        matches!(
            signal.signal_type,
            ActiveTradeToxicSignalType::OneHourDeltaBuyDominant
                | ActiveTradeToxicSignalType::OneHourDeltaSellDominant
        )
    }));
}

#[test]
fn open_one_hour_candle_does_not_generate_signal() {
    let recent = build_active_trade_toxicity_recent(
        "BTC-PERP",
        &flow_state(vec![hour_flow_window(2_100.0, Some(10.0), false, false)]),
        &empty_sweep_state(),
        &empty_markout_state(),
    );

    assert!(!recent.signals.iter().any(|signal| {
        matches!(
            signal.signal_type,
            ActiveTradeToxicSignalType::OneHourDeltaBuyDominant
                | ActiveTradeToxicSignalType::OneHourDeltaSellDominant
        )
    }));
}

#[test]
fn non_hour_window_does_not_generate_one_hour_delta_signal() {
    let mut window = flow_window(
        5_000,
        220_000_000.0,
        10_000_000.0,
        25,
        75.0,
        15.0,
        Some(8.0),
    );
    window.net_aggressive_btc = 2_100.0;
    window.now_ts = closed_hour_ts() as i64;

    let recent = build_active_trade_toxicity_recent(
        "BTC-PERP",
        &flow_state(vec![window]),
        &empty_sweep_state(),
        &empty_markout_state(),
    );

    assert!(!recent.signals.iter().any(|signal| {
        matches!(
            signal.signal_type,
            ActiveTradeToxicSignalType::OneHourDeltaBuyDominant
                | ActiveTradeToxicSignalType::OneHourDeltaSellDominant
        )
    }));
}

#[test]
fn one_hour_buy_delta_without_price_follow_through_adds_absorption_candidate() {
    let recent = build_active_trade_toxicity_recent(
        "BTC-PERP",
        &flow_state(vec![hour_flow_window(2_150.0, Some(0.0), true, true)]),
        &empty_sweep_state(),
        &empty_markout_state(),
    );

    assert!(recent.signals.iter().any(|signal| signal.signal_type
        == ActiveTradeToxicSignalType::AbsorptionCandidate
        && signal.timeframe.as_deref() == Some("1h")));
}

fn flow_state(windows: Vec<FlowWindow>) -> FlowState {
    FlowState {
        symbol: "BTC-PERP".to_string(),
        updated_at: 1_760_000_000_000,
        windows: windows
            .into_iter()
            .map(|window| (window.window_ms.to_string(), window))
            .collect::<BTreeMap<_, _>>(),
    }
}

fn flow_window(
    window_ms: u64,
    aggressive_buy_usd: f64,
    aggressive_sell_usd: f64,
    trade_count: u64,
    max_trade_size_btc: f64,
    avg_trade_size_btc: f64,
    price_move_bps: Option<f64>,
) -> FlowWindow {
    let aggressive_buy_btc = aggressive_buy_usd / 100_000.0;
    let aggressive_sell_btc = aggressive_sell_usd / 100_000.0;
    let buy_trade_count = if aggressive_buy_usd > aggressive_sell_usd {
        trade_count.saturating_sub(2)
    } else {
        2
    };
    let sell_trade_count = trade_count.saturating_sub(buy_trade_count);
    FlowWindow {
        symbol: "BTC-PERP".to_string(),
        window_ms,
        now_ts: 1_760_000_000_000,
        aggressive_buy_btc,
        aggressive_sell_btc,
        aggressive_buy_usd,
        aggressive_sell_usd,
        net_aggressive_btc: aggressive_buy_btc - aggressive_sell_btc,
        abs_aggressive_btc: aggressive_buy_btc + aggressive_sell_btc,
        trade_count,
        buy_trade_count,
        sell_trade_count,
        avg_trade_size_btc,
        max_trade_size_btc,
        venue_breakdown: empty_venue_breakdown(),
        mid_start: Some(100_000.0),
        mid_end: Some(100_005.0),
        price_move_bps,
        spread_bps_median: None,
        imbalance_10bps_median: None,
        data_quality: DataQuality {
            has_trades: trade_count > 0,
            has_books: false,
            active_venues: vec!["binance".to_string()],
            stale_venues: Vec::new(),
        },
    }
}

fn hour_flow_window(
    delta_btc: f64,
    price_move_bps: Option<f64>,
    closed: bool,
    flat_price: bool,
) -> FlowWindow {
    let base_volume_btc = delta_btc.abs().max(2_100.0);
    let (aggressive_buy_btc, aggressive_sell_btc) = if delta_btc >= 0.0 {
        (base_volume_btc + 100.0, 100.0)
    } else {
        (100.0, base_volume_btc + 100.0)
    };
    let open = 100_000.0;
    let close = if flat_price {
        open
    } else {
        match price_move_bps {
            Some(bps) => open * (1.0 + bps / 10_000.0),
            None => open,
        }
    };

    FlowWindow {
        symbol: "BTC-PERP".to_string(),
        window_ms: 3_600_000,
        now_ts: if closed {
            closed_hour_ts() as i64
        } else {
            (closed_hour_ts() + 1_000) as i64
        },
        aggressive_buy_btc,
        aggressive_sell_btc,
        aggressive_buy_usd: aggressive_buy_btc * 100_000.0,
        aggressive_sell_usd: aggressive_sell_btc * 100_000.0,
        net_aggressive_btc: delta_btc,
        abs_aggressive_btc: aggressive_buy_btc + aggressive_sell_btc,
        trade_count: 24,
        buy_trade_count: if delta_btc >= 0.0 { 18 } else { 6 },
        sell_trade_count: if delta_btc >= 0.0 { 6 } else { 18 },
        avg_trade_size_btc: 12.0,
        max_trade_size_btc: base_volume_btc / 2.0,
        venue_breakdown: empty_venue_breakdown(),
        mid_start: Some(open),
        mid_end: Some(close),
        price_move_bps,
        spread_bps_median: None,
        imbalance_10bps_median: None,
        data_quality: DataQuality {
            has_trades: true,
            has_books: false,
            active_venues: vec!["binance".to_string()],
            stale_venues: Vec::new(),
        },
    }
}

fn closed_hour_ts() -> u64 {
    1_760_000_400_000
}

fn empty_sweep_state() -> SweepState {
    SweepState {
        symbol: "BTC-PERP".to_string(),
        updated_at: 1_760_000_000_000,
        windows_ms: vec![1_000, 5_000, 15_000],
        results: BTreeMap::new(),
        quality: SweepQuality {
            has_trades: false,
            has_books: false,
            active_venues: Vec::<Venue>::new(),
            stale_venues: Vec::<Venue>::new(),
        },
    }
}

fn sweep_state(
    direction: SweepDirection,
    sweep_detected: bool,
    price_impact_bps: Option<f64>,
) -> SweepState {
    SweepState {
        symbol: "BTC-PERP".to_string(),
        updated_at: 1_760_000_000_000,
        windows_ms: vec![1_000, 5_000, 15_000],
        results: BTreeMap::from([(
            "5000".to_string(),
            SweepResult {
                symbol: "BTC-PERP".to_string(),
                window_ms: 5_000,
                direction,
                sweep_detected,
                swept_volume_btc: 8.2,
                swept_volume_usd: 820_000.0,
                aggressive_buy_btc: 7.5,
                aggressive_sell_btc: 1.0,
                net_aggressive_btc: 6.5,
                trade_count: 10,
                same_direction_trade_count: 8,
                price_start: Some(100_000.0),
                price_end: Some(100_006.0),
                price_impact_bps,
                leader_venue: Some(Venue::Binance),
                venue_breakdown: Default::default(),
                liquidity: None,
                reason_codes: vec!["sweep_detected".to_string()],
            },
        )]),
        quality: SweepQuality {
            has_trades: true,
            has_books: false,
            active_venues: vec![Venue::Binance],
            stale_venues: Vec::new(),
        },
    }
}

fn empty_markout_state() -> MarkoutState {
    MarkoutState {
        symbol: "BTC-PERP".to_string(),
        updated_at: 1_760_000_000_000,
        horizons_ms: vec![5_000, 15_000, 60_000],
        summaries: BTreeMap::new(),
        quality: MarkoutQuality {
            pending_samples: 0,
            resolved_samples: 0,
            expired_samples: 0,
            has_price_index: false,
        },
    }
}

fn markout_state_for_buy(
    markout_5s: Option<f64>,
    markout_15s: Option<f64>,
    markout_60s: Option<f64>,
) -> MarkoutState {
    let mut summaries = BTreeMap::new();
    if let Some(value) = markout_5s {
        summaries.insert("5000".to_string(), markout_summary(value));
    }
    if let Some(value) = markout_15s {
        summaries.insert("15000".to_string(), markout_summary(value));
    }
    if let Some(value) = markout_60s {
        summaries.insert("60000".to_string(), markout_summary(value));
    }
    MarkoutState {
        symbol: "BTC-PERP".to_string(),
        updated_at: 1_760_000_000_000,
        horizons_ms: vec![5_000, 15_000, 60_000],
        summaries,
        quality: MarkoutQuality {
            pending_samples: 0,
            resolved_samples: 3,
            expired_samples: 0,
            has_price_index: true,
        },
    }
}

fn markout_summary(buy_markout: f64) -> MarkoutWindowSummary {
    MarkoutWindowSummary {
        horizon_ms: 5_000,
        buy: DirectionalMarkoutStats {
            count: 1,
            volume_btc: 1.0,
            volume_usd: 100_000.0,
            avg_markout_bps: Some(buy_markout),
            volume_weighted_markout_bps: Some(buy_markout),
            positive_count: u64::from(buy_markout > 0.0),
            negative_count: u64::from(buy_markout < 0.0),
            positive_volume_btc: if buy_markout > 0.0 { 1.0 } else { 0.0 },
            negative_volume_btc: if buy_markout < 0.0 { 1.0 } else { 0.0 },
        },
        sell: DirectionalMarkoutStats::default(),
        venue_breakdown: BTreeMap::new(),
    }
}
