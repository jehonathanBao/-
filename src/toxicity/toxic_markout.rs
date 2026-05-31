use crate::{
    market_data::price_index::PriceSnapshot,
    types::{
        toxic_flow::ToxicConfidence,
        toxic_markout::{
            ToxicMarkoutDetailResponse, ToxicMarkoutOutcome, ToxicMarkoutRecentResponse,
            ToxicMarkoutSignal, ToxicMarkoutStatusResponse, ToxicMarkoutWindow,
        },
        toxic_signal::{
            ToxicSignal, ToxicSignalDirection, ToxicSignalRecentResponse, ToxicSignalType,
        },
    },
};

pub const TOXIC_MARKOUT_WINDOWS: [(&str, u64); 4] = [
    ("+1m", 60_000),
    ("+5m", 300_000),
    ("+15m", 900_000),
    ("+1h", 3_600_000),
];
const NEUTRAL_MARKOUT_BAND_BPS: f64 = 5.0;

pub fn build_toxic_markout_recent<F1, F2>(
    requested_symbol: &str,
    fusion_recent: &ToxicSignalRecentResponse,
    snapshot_at_or_before: F1,
    snapshots_since: F2,
) -> ToxicMarkoutRecentResponse
where
    F1: Fn(i64) -> Option<PriceSnapshot>,
    F2: Fn(i64) -> Vec<PriceSnapshot>,
{
    let mut signals = fusion_recent
        .signals
        .iter()
        .filter(|signal| signal.symbol.eq_ignore_ascii_case(requested_symbol))
        .map(|signal| evaluate_signal(signal, &snapshot_at_or_before, &snapshots_since))
        .collect::<Vec<_>>();
    signals.sort_by_key(|signal| std::cmp::Reverse(signal.created_at_ms));

    ToxicMarkoutRecentResponse {
        read_only: true,
        runtime_modified: false,
        analysis_only: true,
        execution_enabled: false,
        mode: "analysis_only".to_string(),
        selected_symbol: requested_symbol.to_string(),
        status: if signals.is_empty() {
            "no_markout_signal".to_string()
        } else {
            "markout_ready".to_string()
        },
        warnings: fusion_recent.warnings.clone(),
        signals,
    }
}

pub fn build_toxic_markout_status<F1, F2>(
    requested_symbol: &str,
    fusion_recent: &ToxicSignalRecentResponse,
    snapshot_at_or_before: F1,
    snapshots_since: F2,
) -> ToxicMarkoutStatusResponse
where
    F1: Fn(i64) -> Option<PriceSnapshot>,
    F2: Fn(i64) -> Vec<PriceSnapshot>,
{
    let recent = build_toxic_markout_recent(
        requested_symbol,
        fusion_recent,
        snapshot_at_or_before,
        snapshots_since,
    );
    ToxicMarkoutStatusResponse {
        read_only: true,
        runtime_modified: false,
        analysis_only: true,
        execution_enabled: false,
        enabled: true,
        mode: "analysis_only".to_string(),
        signal_count: recent.signals.len(),
        last_signal_at_ms: recent
            .signals
            .iter()
            .map(|signal| signal.created_at_ms)
            .max(),
        safety_boundary: vec![
            "readOnly=true".to_string(),
            "runtimeModified=false".to_string(),
            "analysis_only".to_string(),
            "No order execution".to_string(),
            "No cancel/amend".to_string(),
            "No wallet".to_string(),
            "No signing".to_string(),
            "No transaction construction".to_string(),
        ],
    }
}

pub fn build_toxic_markout_by_signal_id<F1, F2>(
    requested_symbol: &str,
    signal_id: &str,
    fusion_recent: &ToxicSignalRecentResponse,
    snapshot_at_or_before: F1,
    snapshots_since: F2,
) -> ToxicMarkoutDetailResponse
where
    F1: Fn(i64) -> Option<PriceSnapshot>,
    F2: Fn(i64) -> Vec<PriceSnapshot>,
{
    let Some(signal) = fusion_recent.signals.iter().find(|signal| {
        signal.symbol.eq_ignore_ascii_case(requested_symbol) && signal.signal_id == signal_id
    }) else {
        return unavailable_response(requested_symbol, "signal_not_found");
    };

    ToxicMarkoutDetailResponse {
        read_only: true,
        runtime_modified: false,
        analysis_only: true,
        execution_enabled: false,
        mode: "analysis_only".to_string(),
        symbol: requested_symbol.to_string(),
        available: true,
        reason: None,
        signal: Some(evaluate_signal(
            signal,
            &snapshot_at_or_before,
            &snapshots_since,
        )),
    }
}

fn unavailable_response(symbol: &str, reason: &str) -> ToxicMarkoutDetailResponse {
    ToxicMarkoutDetailResponse {
        read_only: true,
        runtime_modified: false,
        analysis_only: true,
        execution_enabled: false,
        mode: "analysis_only".to_string(),
        symbol: symbol.to_string(),
        available: false,
        reason: Some(reason.to_string()),
        signal: None,
    }
}

fn evaluate_signal<F1, F2>(
    signal: &ToxicSignal,
    snapshot_at_or_before: &F1,
    snapshots_since: &F2,
) -> ToxicMarkoutSignal
where
    F1: Fn(i64) -> Option<PriceSnapshot>,
    F2: Fn(i64) -> Vec<PriceSnapshot>,
{
    let signal_ts = u64_to_i64(signal.ts_ms);
    let base_snapshot = snapshot_at_or_before(signal_ts);
    let future_snapshots = snapshots_since(signal_ts);

    let windows = TOXIC_MARKOUT_WINDOWS
        .iter()
        .map(|(label, horizon_ms)| {
            evaluate_window(
                label,
                *horizon_ms,
                signal,
                base_snapshot.as_ref(),
                &future_snapshots,
            )
        })
        .collect::<Vec<_>>();

    let aligned_windows = windows
        .iter()
        .filter(|window| window.outcome == ToxicMarkoutOutcome::Aligned)
        .count();
    let adverse_windows = windows
        .iter()
        .filter(|window| window.outcome == ToxicMarkoutOutcome::Adverse)
        .count();
    let neutral_windows = windows
        .iter()
        .filter(|window| window.outcome == ToxicMarkoutOutcome::Neutral)
        .count();
    let missing_windows = windows
        .iter()
        .filter(|window| window.outcome == ToxicMarkoutOutcome::NotEnoughData)
        .count();

    ToxicMarkoutSignal {
        signal_id: signal.signal_id.clone(),
        symbol: signal.symbol.clone(),
        signal_kind: signal_type_key(signal),
        direction: direction_key(signal.direction).to_string(),
        toxicity_score: signal.toxicity_score,
        confidence: confidence_key(signal.confidence).to_string(),
        created_at_ms: signal.ts_ms,
        overall_outcome: classify_overall_outcome(
            aligned_windows,
            adverse_windows,
            neutral_windows,
            missing_windows,
        ),
        aligned_windows,
        adverse_windows,
        neutral_windows,
        missing_windows,
        windows,
        no_trade_reasons: signal.no_trade_reasons.clone(),
        read_only: true,
    }
}

fn evaluate_window(
    label: &str,
    horizon_ms: u64,
    signal: &ToxicSignal,
    base_snapshot: Option<&PriceSnapshot>,
    future_snapshots: &[PriceSnapshot],
) -> ToxicMarkoutWindow {
    let Some(base_snapshot) = base_snapshot else {
        return ToxicMarkoutWindow {
            label: label.to_string(),
            horizon_ms,
            outcome: ToxicMarkoutOutcome::NotEnoughData,
            markout_bps: None,
            price_at_signal: None,
            price_at_horizon: None,
            note: "No price snapshot was available at signal time.".to_string(),
        };
    };

    let due_ts = u64_to_i64(signal.ts_ms.saturating_add(horizon_ms));
    let Some(future_snapshot) = future_snapshots
        .iter()
        .find(|snapshot| snapshot.ts >= due_ts)
    else {
        return ToxicMarkoutWindow {
            label: label.to_string(),
            horizon_ms,
            outcome: ToxicMarkoutOutcome::NotEnoughData,
            markout_bps: None,
            price_at_signal: Some(round2(base_snapshot.index_mid)),
            price_at_horizon: None,
            note: "Not enough future price data for this markout window.".to_string(),
        };
    };

    let raw_markout_bps = ((future_snapshot.index_mid - base_snapshot.index_mid)
        / base_snapshot.index_mid)
        * 10_000.0;
    let directional_markout_bps = directional_markout_bps(signal.direction, raw_markout_bps);
    let outcome = classify_window_outcome(signal, directional_markout_bps);
    let note = match outcome {
        ToxicMarkoutOutcome::Aligned => {
            "Price moved with the fused toxic signal direction.".to_string()
        }
        ToxicMarkoutOutcome::Adverse => {
            "Price moved against the fused toxic signal direction.".to_string()
        }
        ToxicMarkoutOutcome::Neutral => {
            if expects_directional_follow_through(signal) {
                "Price stayed near flat after the signal window.".to_string()
            } else {
                "Trap or no-trade signals remain observational and do not require directional follow-through."
                    .to_string()
            }
        }
        ToxicMarkoutOutcome::NotEnoughData => {
            "Not enough data for this markout window.".to_string()
        }
    };

    ToxicMarkoutWindow {
        label: label.to_string(),
        horizon_ms,
        outcome,
        markout_bps: Some(round2(directional_markout_bps)),
        price_at_signal: Some(round2(base_snapshot.index_mid)),
        price_at_horizon: Some(round2(future_snapshot.index_mid)),
        note,
    }
}

fn classify_window_outcome(
    signal: &ToxicSignal,
    directional_markout_bps: f64,
) -> ToxicMarkoutOutcome {
    if !expects_directional_follow_through(signal) {
        return ToxicMarkoutOutcome::Neutral;
    }
    if directional_markout_bps >= NEUTRAL_MARKOUT_BAND_BPS {
        ToxicMarkoutOutcome::Aligned
    } else if directional_markout_bps <= -NEUTRAL_MARKOUT_BAND_BPS {
        ToxicMarkoutOutcome::Adverse
    } else {
        ToxicMarkoutOutcome::Neutral
    }
}

fn classify_overall_outcome(
    aligned_windows: usize,
    adverse_windows: usize,
    neutral_windows: usize,
    missing_windows: usize,
) -> ToxicMarkoutOutcome {
    if missing_windows == TOXIC_MARKOUT_WINDOWS.len() {
        return ToxicMarkoutOutcome::NotEnoughData;
    }
    if aligned_windows > adverse_windows && aligned_windows > 0 {
        return ToxicMarkoutOutcome::Aligned;
    }
    if adverse_windows > aligned_windows && adverse_windows > 0 {
        return ToxicMarkoutOutcome::Adverse;
    }
    if neutral_windows > 0 || missing_windows > 0 {
        return ToxicMarkoutOutcome::Neutral;
    }
    ToxicMarkoutOutcome::Neutral
}

fn expects_directional_follow_through(signal: &ToxicSignal) -> bool {
    !matches!(
        signal.signal_type,
        ToxicSignalType::TrapRisk | ToxicSignalType::NoTradeChopRisk
    ) && !matches!(
        signal.direction,
        ToxicSignalDirection::TrapRisk | ToxicSignalDirection::Neutral
    )
}

fn directional_markout_bps(direction: ToxicSignalDirection, raw_markout_bps: f64) -> f64 {
    match direction {
        ToxicSignalDirection::ShortBias => -raw_markout_bps,
        ToxicSignalDirection::LongBias => raw_markout_bps,
        ToxicSignalDirection::TrapRisk | ToxicSignalDirection::Neutral => raw_markout_bps.abs(),
    }
}

fn signal_type_key(signal: &ToxicSignal) -> String {
    let mut output = String::new();
    let debug = format!("{:?}", signal.signal_type);
    for (index, ch) in debug.chars().enumerate() {
        if ch.is_uppercase() && index > 0 {
            output.push('_');
        }
        output.extend(ch.to_lowercase());
    }
    output
}

fn direction_key(direction: ToxicSignalDirection) -> &'static str {
    match direction {
        ToxicSignalDirection::ShortBias => "SHORT_BIAS",
        ToxicSignalDirection::LongBias => "LONG_BIAS",
        ToxicSignalDirection::TrapRisk => "TRAP_RISK",
        ToxicSignalDirection::Neutral => "NEUTRAL",
    }
}

fn confidence_key(confidence: ToxicConfidence) -> &'static str {
    match confidence {
        ToxicConfidence::High => "HIGH",
        ToxicConfidence::Medium => "MEDIUM",
        ToxicConfidence::Low => "LOW",
    }
}

fn u64_to_i64(value: u64) -> i64 {
    i64::try_from(value).unwrap_or(i64::MAX)
}

fn round2(value: f64) -> f64 {
    (value * 100.0).round() / 100.0
}
