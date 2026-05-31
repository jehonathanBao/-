use crate::{
    toxicity::active_trade_toxicity::analyze_active_trade_toxicity,
    types::{
        flow::{FlowState, FlowWindow},
        market::AggressorSide,
        markout::MarkoutState,
        sweep::{SweepDirection, SweepResult, SweepState},
        toxic_flow::{
            ActiveTradeToxicSignal, ActiveTradeToxicSignalType, ActiveTradeToxicityRecentResponse,
            ActiveTradeToxicityStatusResponse, ToxicConfidence, ToxicSide,
        },
    },
};

const DEFAULT_WINDOW_MS: u64 = 5_000;
const ONE_HOUR_WINDOW_MS: u64 = 3_600_000;
const ONE_HOUR_DELTA_THRESHOLD: f64 = 2_000.0;
const ONE_HOUR_TIMEFRAME: &str = "1h";

pub fn build_active_trade_toxicity_recent(
    requested_symbol: &str,
    flow_state: &FlowState,
    sweep_state: &SweepState,
    markout_state: &MarkoutState,
) -> ActiveTradeToxicityRecentResponse {
    let report = analyze_active_trade_toxicity(requested_symbol, flow_state, sweep_state);
    let window = select_window(flow_state, requested_symbol);
    let one_hour_window = select_one_hour_window(flow_state, requested_symbol);
    let sweep = select_sweep(sweep_state, requested_symbol);
    let markouts = dominant_side_markouts(markout_state, &report.side_bias);

    let mut signals = match window {
        Some(window) => build_signals(requested_symbol, &report, window, sweep, markouts),
        None => Vec::new(),
    };
    if let Some(one_hour_window) = one_hour_window {
        signals.extend(build_one_hour_delta_signals(
            requested_symbol,
            one_hour_window,
            markouts,
        ));
    }

    ActiveTradeToxicityRecentResponse {
        read_only: true,
        runtime_modified: false,
        mode: "analysis_only".to_string(),
        selected_symbol: requested_symbol.to_string(),
        status: report.status,
        score: report.score,
        side_bias: report.side_bias,
        warnings: report.warnings,
        no_trade_reasons: report.no_trade_reasons,
        signals,
    }
}

pub fn build_active_trade_toxicity_status(
    requested_symbol: &str,
    flow_state: &FlowState,
    sweep_state: &SweepState,
    markout_state: &MarkoutState,
) -> ActiveTradeToxicityStatusResponse {
    let recent = build_active_trade_toxicity_recent(
        requested_symbol,
        flow_state,
        sweep_state,
        markout_state,
    );
    ActiveTradeToxicityStatusResponse {
        read_only: true,
        runtime_modified: false,
        enabled: true,
        mode: "analysis_only".to_string(),
        signal_count: recent.signals.len(),
        last_signal_at_ms: recent.signals.iter().map(|signal| signal.ts_ms).max(),
        safety_boundary: vec![
            "readOnly=true".to_string(),
            "runtimeModified=false".to_string(),
            "analysis_only".to_string(),
            "No order execution".to_string(),
            "No wallet".to_string(),
            "No signing".to_string(),
            "No transaction construction".to_string(),
        ],
    }
}

fn build_signals(
    symbol: &str,
    report: &crate::types::toxic_flow::ActiveTradeToxicityReport,
    window: &FlowWindow,
    sweep: Option<&SweepResult>,
    markouts: (Option<f64>, Option<f64>, Option<f64>),
) -> Vec<ActiveTradeToxicSignal> {
    let mut signals = Vec::new();
    let buy_volume = window.aggressive_buy_usd.max(0.0);
    let sell_volume = window.aggressive_sell_usd.max(0.0);
    let total_volume = buy_volume + sell_volume;
    let net_notional = buy_volume - sell_volume;
    let dominant_side = if net_notional > 0.0 {
        ToxicSide::Buy
    } else if net_notional < 0.0 {
        ToxicSide::Sell
    } else {
        ToxicSide::Neutral
    };
    let aggressive_volume = match dominant_side {
        ToxicSide::Buy => window.aggressive_buy_btc.max(0.0),
        ToxicSide::Sell => window.aggressive_sell_btc.max(0.0),
        ToxicSide::Neutral => window.abs_aggressive_btc.max(0.0),
    };
    let notional_usd = match dominant_side {
        ToxicSide::Buy => buy_volume,
        ToxicSide::Sell => sell_volume,
        ToxicSide::Neutral => total_volume,
    };
    let imbalance_ratio = report.features.imbalance_ratio;
    let price_impact_bps = sweep
        .and_then(|result| result.price_impact_bps)
        .or(window.price_move_bps);
    let confidence = confidence_for(report.score, imbalance_ratio);
    let toxicity_score = clamp_score(report.score);
    let markout_5s = markouts.0;
    let markout_15s = markouts.1;
    let markout_60s = markouts.2;

    if total_volume >= 500_000.0 && imbalance_ratio >= 0.60 {
        let signal_type = match dominant_side {
            ToxicSide::Buy => Some(ActiveTradeToxicSignalType::LargeAggressiveBuy),
            ToxicSide::Sell => Some(ActiveTradeToxicSignalType::LargeAggressiveSell),
            ToxicSide::Neutral => None,
        };
        if let Some(signal_type) = signal_type {
            signals.push(build_signal(
                symbol,
                window,
                signal_type,
                dominant_side,
                aggressive_volume,
                notional_usd,
                net_notional,
                buy_volume,
                sell_volume,
                imbalance_ratio,
                price_impact_bps,
                markout_5s,
                markout_15s,
                markout_60s,
                toxicity_score,
                confidence,
                vec![
                    "aggressive notional exceeded the active window baseline".to_string(),
                    "trade direction dominated the short window".to_string(),
                ],
                None,
                None,
                None,
                None,
                None,
                None,
                derive_candle_ohlc(window),
            ));
        }
    }

    if let Some(sweep) = sweep {
        if sweep.sweep_detected {
            let signal_type = match sweep.direction {
                SweepDirection::Buy => Some(ActiveTradeToxicSignalType::BuySweep),
                SweepDirection::Sell => Some(ActiveTradeToxicSignalType::SellSweep),
                SweepDirection::None => None,
            };
            if let Some(signal_type) = signal_type {
                let side = match sweep.direction {
                    SweepDirection::Buy => ToxicSide::Buy,
                    SweepDirection::Sell => ToxicSide::Sell,
                    SweepDirection::None => ToxicSide::Neutral,
                };
                signals.push(build_signal(
                    symbol,
                    window,
                    signal_type,
                    side,
                    sweep.swept_volume_btc.max(0.0),
                    sweep.swept_volume_usd.max(0.0),
                    net_notional,
                    buy_volume,
                    sell_volume,
                    imbalance_ratio,
                    sweep.price_impact_bps,
                    markout_5s,
                    markout_15s,
                    markout_60s,
                    clamp_score((report.score + 10.0).min(100.0)),
                    confidence,
                    vec![
                        "same-direction aggressive flow crossed the sweep threshold".to_string(),
                        "price impact and venue sweep activity lined up in the short window"
                            .to_string(),
                    ],
                    None,
                    None,
                    None,
                    None,
                    None,
                    None,
                    derive_candle_ohlc(window),
                ));
            }
        }
    }

    if report.features.volume_spike_score >= 35.0 && imbalance_ratio >= 0.45 {
        signals.push(build_signal(
            symbol,
            window,
            ActiveTradeToxicSignalType::CvdSpike,
            dominant_side,
            aggressive_volume,
            total_volume,
            net_notional,
            buy_volume,
            sell_volume,
            imbalance_ratio,
            price_impact_bps,
            markout_5s,
            markout_15s,
            markout_60s,
            toxicity_score,
            confidence,
            vec![
                "net aggressive flow expanded quickly versus nearby windows".to_string(),
                "volume spike score indicates an abnormal short-window delta".to_string(),
            ],
            None,
            None,
            None,
            None,
            None,
            None,
            derive_candle_ohlc(window),
        ));
    }

    if imbalance_ratio >= 0.55 {
        signals.push(build_signal(
            symbol,
            window,
            ActiveTradeToxicSignalType::TradeImbalance,
            dominant_side,
            aggressive_volume,
            total_volume,
            net_notional,
            buy_volume,
            sell_volume,
            imbalance_ratio,
            price_impact_bps,
            markout_5s,
            markout_15s,
            markout_60s,
            toxicity_score,
            confidence,
            vec![
                "buy and sell aggressive volume are materially imbalanced".to_string(),
                "directional concentration stayed elevated across the selected window".to_string(),
            ],
            None,
            None,
            None,
            None,
            None,
            None,
            derive_candle_ohlc(window),
        ));
    }

    let absorption_candidate =
        imbalance_ratio >= 0.55 && price_impact_bps.unwrap_or_default().abs() <= 0.75;
    if absorption_candidate {
        signals.push(build_signal(
            symbol,
            window,
            ActiveTradeToxicSignalType::AbsorptionCandidate,
            dominant_side,
            aggressive_volume,
            total_volume,
            net_notional,
            buy_volume,
            sell_volume,
            imbalance_ratio,
            price_impact_bps,
            markout_5s,
            markout_15s,
            markout_60s,
            clamp_score((report.score + 5.0).min(100.0)),
            confidence,
            vec![
                "aggressive flow was strong but short-window price impact stayed muted".to_string(),
                "possible absorption candidate requires confirmation from later layers".to_string(),
            ],
            None,
            None,
            None,
            None,
            None,
            None,
            derive_candle_ohlc(window),
        ));
    }

    let adverse_markout = match dominant_side {
        ToxicSide::Buy => markout_5s
            .or(markout_15s)
            .or(markout_60s)
            .is_some_and(|value| value < 0.0),
        ToxicSide::Sell => markout_5s
            .or(markout_15s)
            .or(markout_60s)
            .is_some_and(|value| value > 0.0),
        ToxicSide::Neutral => false,
    };
    if adverse_markout {
        signals.push(build_signal(
            symbol,
            window,
            ActiveTradeToxicSignalType::AdverseMarkout,
            dominant_side,
            aggressive_volume,
            total_volume,
            net_notional,
            buy_volume,
            sell_volume,
            imbalance_ratio,
            price_impact_bps,
            markout_5s,
            markout_15s,
            markout_60s,
            clamp_score((report.score + 8.0).min(100.0)),
            ToxicConfidence::Medium,
            vec![
                "forward markout moved against the dominant aggressive side".to_string(),
                "recent flow may be toxic even if the initial pressure looked directional"
                    .to_string(),
            ],
            None,
            None,
            None,
            None,
            None,
            None,
            derive_candle_ohlc(window),
        ));
    }

    signals
}

fn build_one_hour_delta_signals(
    symbol: &str,
    window: &FlowWindow,
    markouts: (Option<f64>, Option<f64>, Option<f64>),
) -> Vec<ActiveTradeToxicSignal> {
    if window.window_ms != ONE_HOUR_WINDOW_MS || !is_closed_hour_candle(window) {
        return Vec::new();
    }

    let delta = window.net_aggressive_btc;
    let abs_delta = delta.abs();
    if abs_delta < ONE_HOUR_DELTA_THRESHOLD {
        return Vec::new();
    }

    let price_change_bps = window
        .price_move_bps
        .or_else(|| derive_price_change_bps(window.mid_start, window.mid_end));
    let side = if delta >= 0.0 {
        ToxicSide::Buy
    } else {
        ToxicSide::Sell
    };
    let signal_type = if delta >= 0.0 {
        ActiveTradeToxicSignalType::OneHourDeltaBuyDominant
    } else {
        ActiveTradeToxicSignalType::OneHourDeltaSellDominant
    };
    let ohlc = derive_candle_ohlc(window);
    let mut signals = vec![build_signal(
        symbol,
        window,
        signal_type,
        side,
        abs_delta,
        window.aggressive_buy_usd.max(0.0) + window.aggressive_sell_usd.max(0.0),
        delta,
        window.aggressive_buy_usd.max(0.0),
        window.aggressive_sell_usd.max(0.0),
        derive_imbalance_ratio(window),
        price_change_bps,
        markouts.0,
        markouts.1,
        markouts.2,
        clamp_score(((abs_delta / ONE_HOUR_DELTA_THRESHOLD) * 45.0).clamp(25.0, 95.0)),
        confidence_for_hour_delta(abs_delta, price_change_bps),
        vec![
            format!(
                "1h candle closed with absolute delta {:.1} BTC above threshold {:.1} BTC",
                abs_delta, ONE_HOUR_DELTA_THRESHOLD
            ),
            if delta >= 0.0 {
                "buy-side delta dominated the closed 1h window".to_string()
            } else {
                "sell-side delta dominated the closed 1h window".to_string()
            },
        ],
        Some(ONE_HOUR_TIMEFRAME.to_string()),
        Some((window.now_ts - window.window_ms as i64).max(0) as u64),
        Some(window.now_ts.max(0) as u64),
        Some(delta),
        Some(abs_delta),
        Some(ONE_HOUR_DELTA_THRESHOLD),
        ohlc,
    )];

    if let Some(absorption_signal) =
        maybe_build_hour_delta_absorption(symbol, window, delta, price_change_bps, markouts, ohlc)
    {
        signals.push(absorption_signal);
    }

    signals
}

fn maybe_build_hour_delta_absorption(
    symbol: &str,
    window: &FlowWindow,
    delta: f64,
    price_change_bps: Option<f64>,
    markouts: (Option<f64>, Option<f64>, Option<f64>),
    ohlc: CandleOhlc,
) -> Option<ActiveTradeToxicSignal> {
    let possible_ask_absorption =
        delta >= ONE_HOUR_DELTA_THRESHOLD && price_change_bps.is_some_and(|value| value <= 0.0);
    let possible_bid_absorption =
        delta <= -ONE_HOUR_DELTA_THRESHOLD && price_change_bps.is_some_and(|value| value >= 0.0);

    if !possible_ask_absorption && !possible_bid_absorption {
        return None;
    }

    let side = if delta >= 0.0 {
        ToxicSide::Buy
    } else {
        ToxicSide::Sell
    };
    let reason = if delta >= 0.0 {
        vec![
            "1h buy delta exceeded the threshold but price did not follow higher".to_string(),
            "possible ask-side absorption candidate; confirmation still needs structure, markout, or wall evidence"
                .to_string(),
        ]
    } else {
        vec![
            "1h sell delta exceeded the threshold but price did not follow lower".to_string(),
            "possible bid-side absorption candidate; confirmation still needs structure, markout, or wall evidence"
                .to_string(),
        ]
    };

    Some(build_signal(
        symbol,
        window,
        ActiveTradeToxicSignalType::AbsorptionCandidate,
        side,
        delta.abs(),
        window.aggressive_buy_usd.max(0.0) + window.aggressive_sell_usd.max(0.0),
        delta,
        window.aggressive_buy_usd.max(0.0),
        window.aggressive_sell_usd.max(0.0),
        derive_imbalance_ratio(window),
        price_change_bps,
        markouts.0,
        markouts.1,
        markouts.2,
        clamp_score(((delta.abs() / ONE_HOUR_DELTA_THRESHOLD) * 50.0).clamp(35.0, 92.0)),
        ToxicConfidence::Medium,
        reason,
        Some(ONE_HOUR_TIMEFRAME.to_string()),
        Some((window.now_ts - window.window_ms as i64).max(0) as u64),
        Some(window.now_ts.max(0) as u64),
        Some(delta),
        Some(delta.abs()),
        Some(ONE_HOUR_DELTA_THRESHOLD),
        ohlc,
    ))
}

#[allow(clippy::too_many_arguments)]
fn build_signal(
    symbol: &str,
    window: &FlowWindow,
    signal_type: ActiveTradeToxicSignalType,
    side: ToxicSide,
    aggressive_volume: f64,
    notional_usd: f64,
    cvd_delta: f64,
    buy_volume: f64,
    sell_volume: f64,
    imbalance_ratio: f64,
    price_impact_bps: Option<f64>,
    markout_5s: Option<f64>,
    markout_15s: Option<f64>,
    markout_60s: Option<f64>,
    toxicity_score: u8,
    confidence: ToxicConfidence,
    reason: Vec<String>,
    timeframe: Option<String>,
    candle_open_ms: Option<u64>,
    candle_close_ms: Option<u64>,
    delta: Option<f64>,
    abs_delta: Option<f64>,
    threshold: Option<f64>,
    ohlc: CandleOhlc,
) -> ActiveTradeToxicSignal {
    ActiveTradeToxicSignal {
        signal_id: format!(
            "active-trade-{}-{}-{}",
            signal_type_key(signal_type),
            symbol.to_ascii_lowercase(),
            window.now_ts.max(0)
        ),
        symbol: symbol.to_string(),
        ts_ms: window.now_ts.max(0) as u64,
        signal_type,
        side,
        timeframe,
        candle_open_ms,
        candle_close_ms,
        window_ms: window.window_ms,
        delta: delta.map(round4),
        abs_delta: abs_delta.map(round4),
        threshold: threshold.map(round4),
        aggressive_volume: round4(aggressive_volume),
        notional_usd: round2(notional_usd),
        trade_count: window.trade_count,
        cvd_delta: round2(cvd_delta),
        buy_volume: round2(buy_volume),
        sell_volume: round2(sell_volume),
        imbalance_ratio: round4(imbalance_ratio),
        open: ohlc.open,
        high: ohlc.high,
        low: ohlc.low,
        close: ohlc.close,
        price_impact_bps: price_impact_bps.map(round4),
        price_change_bps: derive_price_change_bps(ohlc.open, ohlc.close).map(round4),
        upper_wick_ratio: ohlc.upper_wick_ratio,
        lower_wick_ratio: ohlc.lower_wick_ratio,
        markout_5s: markout_5s.map(round4),
        markout_15s: markout_15s.map(round4),
        markout_60s: markout_60s.map(round4),
        toxicity_score,
        confidence,
        reason,
        read_only: true,
    }
}

#[derive(Clone, Copy)]
struct CandleOhlc {
    open: Option<f64>,
    high: Option<f64>,
    low: Option<f64>,
    close: Option<f64>,
    upper_wick_ratio: Option<f64>,
    lower_wick_ratio: Option<f64>,
}

fn select_window<'a>(flow_state: &'a FlowState, requested_symbol: &str) -> Option<&'a FlowWindow> {
    if !flow_state.symbol.eq_ignore_ascii_case(requested_symbol) {
        return None;
    }
    flow_state
        .windows
        .get(&DEFAULT_WINDOW_MS.to_string())
        .or_else(|| {
            flow_state
                .windows
                .values()
                .max_by_key(|window| window.trade_count)
        })
}

fn select_one_hour_window<'a>(
    flow_state: &'a FlowState,
    requested_symbol: &str,
) -> Option<&'a FlowWindow> {
    if !flow_state.symbol.eq_ignore_ascii_case(requested_symbol) {
        return None;
    }
    flow_state.windows.get(&ONE_HOUR_WINDOW_MS.to_string())
}

fn select_sweep<'a>(
    sweep_state: &'a SweepState,
    requested_symbol: &str,
) -> Option<&'a SweepResult> {
    if !sweep_state.symbol.eq_ignore_ascii_case(requested_symbol) {
        return None;
    }
    sweep_state
        .results
        .get(&DEFAULT_WINDOW_MS.to_string())
        .or_else(|| {
            sweep_state
                .results
                .values()
                .find(|result| result.sweep_detected)
        })
}

fn dominant_side_markouts(
    markout_state: &MarkoutState,
    side_bias: &str,
) -> (Option<f64>, Option<f64>, Option<f64>) {
    let side = match side_bias {
        "buy" => Some(AggressorSide::Buy),
        "sell" => Some(AggressorSide::Sell),
        _ => None,
    };
    let Some(side) = side else {
        return (None, None, None);
    };

    (
        horizon_markout(markout_state, 5_000, side),
        horizon_markout(markout_state, 15_000, side),
        horizon_markout(markout_state, 60_000, side),
    )
}

fn horizon_markout(
    markout_state: &MarkoutState,
    horizon_ms: u64,
    side: AggressorSide,
) -> Option<f64> {
    markout_state
        .summaries
        .get(&horizon_ms.to_string())
        .and_then(|summary| match side {
            AggressorSide::Buy => summary.buy.volume_weighted_markout_bps,
            AggressorSide::Sell => summary.sell.volume_weighted_markout_bps,
        })
}

fn confidence_for(score: f64, imbalance_ratio: f64) -> ToxicConfidence {
    if score >= 75.0 || imbalance_ratio >= 0.80 {
        ToxicConfidence::High
    } else if score >= 45.0 || imbalance_ratio >= 0.55 {
        ToxicConfidence::Medium
    } else {
        ToxicConfidence::Low
    }
}

fn confidence_for_hour_delta(abs_delta: f64, price_change_bps: Option<f64>) -> ToxicConfidence {
    if abs_delta >= ONE_HOUR_DELTA_THRESHOLD * 2.0
        || price_change_bps.is_some_and(|value| value.abs() >= 12.0)
    {
        ToxicConfidence::High
    } else if abs_delta >= ONE_HOUR_DELTA_THRESHOLD * 1.25 {
        ToxicConfidence::Medium
    } else {
        ToxicConfidence::Low
    }
}

fn clamp_score(score: f64) -> u8 {
    score.round().clamp(0.0, 100.0) as u8
}

fn signal_type_key(signal_type: ActiveTradeToxicSignalType) -> &'static str {
    match signal_type {
        ActiveTradeToxicSignalType::LargeAggressiveBuy => "large_aggressive_buy",
        ActiveTradeToxicSignalType::LargeAggressiveSell => "large_aggressive_sell",
        ActiveTradeToxicSignalType::BuySweep => "buy_sweep",
        ActiveTradeToxicSignalType::SellSweep => "sell_sweep",
        ActiveTradeToxicSignalType::CvdSpike => "cvd_spike",
        ActiveTradeToxicSignalType::TradeImbalance => "trade_imbalance",
        ActiveTradeToxicSignalType::OneHourDeltaBuyDominant => "one_hour_delta_buy_dominant",
        ActiveTradeToxicSignalType::OneHourDeltaSellDominant => "one_hour_delta_sell_dominant",
        ActiveTradeToxicSignalType::AbsorptionCandidate => "absorption_candidate",
        ActiveTradeToxicSignalType::AdverseMarkout => "adverse_markout",
    }
}

fn is_closed_hour_candle(window: &FlowWindow) -> bool {
    window.window_ms == ONE_HOUR_WINDOW_MS
        && window.now_ts > 0
        && (window.now_ts as u64).is_multiple_of(ONE_HOUR_WINDOW_MS)
}

fn derive_price_change_bps(open: Option<f64>, close: Option<f64>) -> Option<f64> {
    match (open, close) {
        (Some(open), Some(close)) if open.abs() > f64::EPSILON => {
            Some(((close - open) / open) * 10_000.0)
        }
        _ => None,
    }
}

fn derive_imbalance_ratio(window: &FlowWindow) -> f64 {
    let total = window.aggressive_buy_usd.max(0.0) + window.aggressive_sell_usd.max(0.0);
    if total <= f64::EPSILON {
        0.0
    } else {
        ((window.aggressive_buy_usd - window.aggressive_sell_usd).abs() / total).clamp(0.0, 1.0)
    }
}

fn derive_candle_ohlc(window: &FlowWindow) -> CandleOhlc {
    match (window.mid_start, window.mid_end) {
        (Some(open), Some(close)) => {
            let high = open.max(close);
            let low = open.min(close);
            CandleOhlc {
                open: Some(round4(open)),
                high: Some(round4(high)),
                low: Some(round4(low)),
                close: Some(round4(close)),
                upper_wick_ratio: Some(0.0),
                lower_wick_ratio: Some(0.0),
            }
        }
        _ => CandleOhlc {
            open: None,
            high: None,
            low: None,
            close: None,
            upper_wick_ratio: None,
            lower_wick_ratio: None,
        },
    }
}

fn round2(value: f64) -> f64 {
    (value * 100.0).round() / 100.0
}

fn round4(value: f64) -> f64 {
    (value * 10_000.0).round() / 10_000.0
}
