use std::collections::BTreeMap;

use crate::types::{
    liquidation::{
        LiquidationToxicDirection, LiquidationToxicSignal, LiquidationToxicSignalType,
        LiquidationToxicityRecentResponse,
    },
    orderbook_wall::{
        OrderbookWallInterpretationReport, OrderbookWallInterpretationSignal,
        OrderbookWallInterpretationType, OrderbookWallLifecycleEvent, OrderbookWallLifecycleReport,
        OrderbookWallSide, TrackedOrderbookWall,
    },
    toxic_flow::{
        ActiveTradeToxicSignal, ActiveTradeToxicSignalType, ActiveTradeToxicityRecentResponse,
        ToxicConfidence,
    },
};

const PERSISTENCE_THRESHOLD_MS: u64 = 10_000;

pub fn analyze_orderbook_wall_interpretation(
    requested_symbol: &str,
    lifecycle_report: &OrderbookWallLifecycleReport,
    active_trade_recent: &ActiveTradeToxicityRecentResponse,
    liquidation_recent: &LiquidationToxicityRecentResponse,
) -> OrderbookWallInterpretationReport {
    if !lifecycle_report
        .symbol
        .eq_ignore_ascii_case(requested_symbol)
    {
        return OrderbookWallInterpretationReport {
            read_only: true,
            runtime_modified: false,
            analysis_mode: "analysis_only".to_string(),
            selected_symbol: requested_symbol.to_string(),
            generated_at_ms: lifecycle_report.generated_at_ms,
            status: "insufficient_data".to_string(),
            signals: Vec::new(),
            warnings: vec![format!(
                "runtime wall interpretation is currently scoped to {}",
                lifecycle_report.symbol
            )],
            no_trade_reasons: vec![
                "selected symbol is not the runtime wall-interpretation symbol".to_string(),
            ],
        };
    }

    let context = interpretation_context(
        &lifecycle_report.tracked_walls,
        &lifecycle_report.recent_events,
    );
    let active_context = active_trade_context(&active_trade_recent.signals);
    let liquidation_context = liquidation_context(&liquidation_recent.signals);

    let mut signals = Vec::new();
    for (wall_id, wall_ctx) in &context {
        signals.extend(build_spoof_signals(
            requested_symbol,
            wall_id,
            wall_ctx,
            &active_context,
            &liquidation_context,
        ));
        signals.extend(build_persistent_signals(
            requested_symbol,
            wall_id,
            wall_ctx,
        ));
        signals.extend(build_absorption_signals(
            requested_symbol,
            wall_id,
            wall_ctx,
            &active_context,
        ));
        signals.extend(build_pull_and_failure_signals(
            requested_symbol,
            wall_id,
            wall_ctx,
            &liquidation_context,
        ));
        signals.extend(build_inducement_signals(
            requested_symbol,
            wall_id,
            wall_ctx,
            &active_context,
            &liquidation_context,
        ));
    }

    let status = if lifecycle_report.status == "insufficient_data" {
        "insufficient_data"
    } else if signals.is_empty() {
        "neutral"
    } else {
        "interpretation_active"
    };

    OrderbookWallInterpretationReport {
        read_only: true,
        runtime_modified: false,
        analysis_mode: "analysis_only".to_string(),
        selected_symbol: requested_symbol.to_string(),
        generated_at_ms: lifecycle_report.generated_at_ms,
        status: status.to_string(),
        signals,
        warnings: lifecycle_report.warnings.clone(),
        no_trade_reasons: if status == "insufficient_data" {
            vec!["wall interpretation requires lifecycle evidence first".to_string()]
        } else {
            lifecycle_report.no_trade_reasons.clone()
        },
    }
}

#[derive(Default)]
struct WallInterpretationContext {
    tracked: Option<TrackedOrderbookWall>,
    latest_event_ts: u64,
    fake_candidate: bool,
    absorption_candidate: bool,
    inducement_candidate: bool,
    removed: bool,
    consumed: bool,
    partial_fill_count: usize,
    moved_count: usize,
    touch_events: usize,
    side: Option<OrderbookWallSide>,
    price: Option<f64>,
    notional: Option<f64>,
    distance_bps: Option<f64>,
}

#[derive(Default)]
struct ActiveTradeContext {
    bullish_pressure: bool,
    bearish_pressure: bool,
    bullish_volume: Option<f64>,
    bearish_volume: Option<f64>,
    bullish_markout: Option<f64>,
    bearish_markout: Option<f64>,
}

#[derive(Default)]
struct LiquidationContext {
    upside_pressure: bool,
    downside_pressure: bool,
}

fn interpretation_context(
    tracked_walls: &[TrackedOrderbookWall],
    events: &[OrderbookWallLifecycleEvent],
) -> BTreeMap<String, WallInterpretationContext> {
    let mut context = BTreeMap::new();

    for tracked in tracked_walls {
        context.insert(
            tracked.wall_id.clone(),
            WallInterpretationContext {
                latest_event_ts: tracked.last_seen_ms,
                side: Some(tracked.side),
                price: Some(tracked.price),
                notional: Some(tracked.notional),
                distance_bps: Some(tracked.distance_bps),
                tracked: Some(tracked.clone()),
                ..WallInterpretationContext::default()
            },
        );
    }

    for event in events {
        let entry = context.entry(event.wall_id.clone()).or_default();
        entry.latest_event_ts = entry.latest_event_ts.max(event.observed_at_ms);
        entry.side = Some(event.side);
        entry.price = Some(event.price);
        entry.notional = Some(event.notional);
        entry.distance_bps = Some(event.distance_bps);
        match event.event_type {
            crate::types::orderbook_wall::OrderbookWallEventType::FakeWallCandidate => {
                entry.fake_candidate = true
            }
            crate::types::orderbook_wall::OrderbookWallEventType::AbsorptionCandidate => {
                entry.absorption_candidate = true
            }
            crate::types::orderbook_wall::OrderbookWallEventType::LiquidityInducementCandidate => {
                entry.inducement_candidate = true
            }
            crate::types::orderbook_wall::OrderbookWallEventType::WallRemoved => {
                entry.removed = true
            }
            crate::types::orderbook_wall::OrderbookWallEventType::WallConsumed => {
                entry.consumed = true
            }
            crate::types::orderbook_wall::OrderbookWallEventType::WallTouched => {
                entry.touch_events += 1;
            }
            crate::types::orderbook_wall::OrderbookWallEventType::WallPartiallyFilled => {
                entry.partial_fill_count += 1
            }
            crate::types::orderbook_wall::OrderbookWallEventType::WallMovedUp
            | crate::types::orderbook_wall::OrderbookWallEventType::WallMovedDown => {
                entry.moved_count += 1
            }
            crate::types::orderbook_wall::OrderbookWallEventType::SupportWallAppeared
            | crate::types::orderbook_wall::OrderbookWallEventType::ResistanceWallAppeared
            | crate::types::orderbook_wall::OrderbookWallEventType::WallUpdated => {}
        }
    }

    context
}

fn active_trade_context(signals: &[ActiveTradeToxicSignal]) -> ActiveTradeContext {
    let mut ctx = ActiveTradeContext::default();
    for signal in signals {
        match signal.signal_type {
            ActiveTradeToxicSignalType::LargeAggressiveBuy
            | ActiveTradeToxicSignalType::BuySweep
            | ActiveTradeToxicSignalType::OneHourDeltaBuyDominant => {
                ctx.bullish_pressure = true;
                ctx.bullish_volume =
                    Some(ctx.bullish_volume.unwrap_or_default() + signal.notional_usd.max(0.0));
                ctx.bullish_markout = preferred_markout(signal);
            }
            ActiveTradeToxicSignalType::LargeAggressiveSell
            | ActiveTradeToxicSignalType::SellSweep
            | ActiveTradeToxicSignalType::OneHourDeltaSellDominant => {
                ctx.bearish_pressure = true;
                ctx.bearish_volume =
                    Some(ctx.bearish_volume.unwrap_or_default() + signal.notional_usd.max(0.0));
                ctx.bearish_markout = preferred_markout(signal);
            }
            ActiveTradeToxicSignalType::TradeImbalance
            | ActiveTradeToxicSignalType::CvdSpike
            | ActiveTradeToxicSignalType::AbsorptionCandidate
            | ActiveTradeToxicSignalType::AdverseMarkout => {}
        }
    }
    ctx
}

fn liquidation_context(signals: &[LiquidationToxicSignal]) -> LiquidationContext {
    let mut ctx = LiquidationContext::default();
    for signal in signals {
        match signal.signal_type {
            LiquidationToxicSignalType::UpsideLiquidationMagnet
            | LiquidationToxicSignalType::ShortSqueezeRisk
            | LiquidationToxicSignalType::LiquidationCascadeCandidate
            | LiquidationToxicSignalType::LiquidationDeltaConfluence
                if signal.direction == LiquidationToxicDirection::Upside =>
            {
                ctx.upside_pressure = true;
            }
            LiquidationToxicSignalType::DownsideLiquidationMagnet
            | LiquidationToxicSignalType::LongSqueezeRisk
            | LiquidationToxicSignalType::LiquidationCascadeCandidate
            | LiquidationToxicSignalType::LiquidationDeltaConfluence
                if signal.direction == LiquidationToxicDirection::Downside =>
            {
                ctx.downside_pressure = true;
            }
            _ => {}
        }
    }
    ctx
}

fn build_spoof_signals(
    symbol: &str,
    wall_id: &str,
    ctx: &WallInterpretationContext,
    active_context: &ActiveTradeContext,
    liquidation_context: &LiquidationContext,
) -> Vec<OrderbookWallInterpretationSignal> {
    let Some(side) = ctx.side else {
        return Vec::new();
    };
    if !ctx.fake_candidate {
        return Vec::new();
    }

    let signal_type = match side {
        OrderbookWallSide::Ask => OrderbookWallInterpretationType::SpoofAskWall,
        OrderbookWallSide::Bid => OrderbookWallInterpretationType::SpoofBidWall,
    };
    let mut reason = vec![
        "wall was removed near touch with low apparent consumed participation".to_string(),
        "short-lived removal pattern matches a spoof / fake wall candidate".to_string(),
    ];
    if side == OrderbookWallSide::Ask && active_context.bullish_pressure {
        reason.push(
            "bullish active trade pressure was present while the ask wall disappeared".to_string(),
        );
    }
    if side == OrderbookWallSide::Bid && active_context.bearish_pressure {
        reason.push(
            "bearish active trade pressure was present while the bid wall disappeared".to_string(),
        );
    }
    if side == OrderbookWallSide::Ask && liquidation_context.upside_pressure {
        reason.push(
            "upside liquidation pressure increases the chance of a fake resistance read"
                .to_string(),
        );
    }
    if side == OrderbookWallSide::Bid && liquidation_context.downside_pressure {
        reason.push(
            "downside liquidation pressure increases the chance of a fake support read".to_string(),
        );
    }
    vec![build_signal(
        symbol,
        wall_id,
        signal_type,
        ctx,
        inferred_aggressive_volume(side, active_context),
        preferred_context_markout(side, active_context),
        88,
        12,
        74,
        82,
        ToxicConfidence::High,
        reason,
    )]
}

fn build_persistent_signals(
    symbol: &str,
    wall_id: &str,
    ctx: &WallInterpretationContext,
) -> Vec<OrderbookWallInterpretationSignal> {
    let Some(tracked) = &ctx.tracked else {
        return Vec::new();
    };
    if ctx.fake_candidate {
        return Vec::new();
    }
    let persistence_ms = tracked.last_seen_ms.saturating_sub(tracked.first_seen_ms);
    if persistence_ms < PERSISTENCE_THRESHOLD_MS || tracked.touches == 0 {
        return Vec::new();
    }

    let signal_type = match tracked.side {
        OrderbookWallSide::Ask => OrderbookWallInterpretationType::PersistentAskWall,
        OrderbookWallSide::Bid => OrderbookWallInterpretationType::PersistentBidWall,
    };
    let side_note = if tracked.side == OrderbookWallSide::Ask {
        "ask wall persisted through repeated price tests"
    } else {
        "bid wall persisted through repeated price tests"
    };
    vec![build_signal(
        symbol,
        wall_id,
        signal_type,
        ctx,
        None,
        None,
        10,
        if tracked.side == OrderbookWallSide::Ask {
            52
        } else {
            48
        },
        18,
        63,
        ToxicConfidence::Medium,
        vec![
            side_note.to_string(),
            "persistence, touches, and low fake-wall evidence suggest a real wall candidate"
                .to_string(),
        ],
    )]
}

fn build_absorption_signals(
    symbol: &str,
    wall_id: &str,
    ctx: &WallInterpretationContext,
    active_context: &ActiveTradeContext,
) -> Vec<OrderbookWallInterpretationSignal> {
    let Some(side) = ctx.side else {
        return Vec::new();
    };
    if !ctx.absorption_candidate {
        return Vec::new();
    }

    let aligned = match side {
        OrderbookWallSide::Ask => active_context.bullish_pressure,
        OrderbookWallSide::Bid => active_context.bearish_pressure,
    };
    if !aligned {
        return Vec::new();
    }

    let signal_type = match side {
        OrderbookWallSide::Ask => OrderbookWallInterpretationType::AskAbsorption,
        OrderbookWallSide::Bid => OrderbookWallInterpretationType::BidAbsorption,
    };
    let mut reason = vec![
        "aggressive flow hit the wall but price did not cleanly continue".to_string(),
        "wall lifecycle suggests possible absorption rather than immediate breakout".to_string(),
    ];
    if ctx.partial_fill_count > 0 {
        reason.push(
            "partial fill behavior was recorded before the wall disappeared or stabilized"
                .to_string(),
        );
    }
    vec![build_signal(
        symbol,
        wall_id,
        signal_type,
        ctx,
        inferred_aggressive_volume(side, active_context),
        preferred_context_markout(side, active_context),
        18,
        84,
        32,
        78,
        ToxicConfidence::Medium,
        reason,
    )]
}

fn build_pull_and_failure_signals(
    symbol: &str,
    wall_id: &str,
    ctx: &WallInterpretationContext,
    liquidation_context: &LiquidationContext,
) -> Vec<OrderbookWallInterpretationSignal> {
    let Some(side) = ctx.side else {
        return Vec::new();
    };
    let mut signals = Vec::new();
    if ctx.removed {
        let signal_type = match side {
            OrderbookWallSide::Ask => OrderbookWallInterpretationType::LiquidityPullAbove,
            OrderbookWallSide::Bid => OrderbookWallInterpretationType::LiquidityPullBelow,
        };
        let mut reason = vec![
            "visible wall liquidity was removed from the book".to_string(),
            "liquidity pull can precede a faster move through the vacated side".to_string(),
        ];
        if side == OrderbookWallSide::Ask && liquidation_context.upside_pressure {
            reason.push(
                "upside liquidation pressure supports a breakout-through-resistance read"
                    .to_string(),
            );
        }
        if side == OrderbookWallSide::Bid && liquidation_context.downside_pressure {
            reason.push(
                "downside liquidation pressure supports a breakdown-through-support read"
                    .to_string(),
            );
        }
        signals.push(build_signal(
            symbol,
            wall_id,
            signal_type,
            ctx,
            None,
            None,
            54,
            18,
            72,
            69,
            ToxicConfidence::Medium,
            reason,
        ));
    }

    if ctx.consumed {
        let signal_type = match side {
            OrderbookWallSide::Ask => OrderbookWallInterpretationType::ResistanceWallFailure,
            OrderbookWallSide::Bid => OrderbookWallInterpretationType::SupportWallFailure,
        };
        signals.push(build_signal(
            symbol,
            wall_id,
            signal_type,
            ctx,
            None,
            None,
            16,
            48,
            26,
            74,
            ToxicConfidence::Medium,
            vec![
                "price traded into and through the wall level".to_string(),
                "wall consumption looks like a support / resistance failure candidate".to_string(),
            ],
        ));
    }
    signals
}

fn build_inducement_signals(
    symbol: &str,
    wall_id: &str,
    ctx: &WallInterpretationContext,
    active_context: &ActiveTradeContext,
    liquidation_context: &LiquidationContext,
) -> Vec<OrderbookWallInterpretationSignal> {
    let Some(side) = ctx.side else {
        return Vec::new();
    };
    if !ctx.inducement_candidate && !ctx.fake_candidate {
        return Vec::new();
    }

    let signal_type = match side {
        OrderbookWallSide::Ask => OrderbookWallInterpretationType::LiquidityInducementAbove,
        OrderbookWallSide::Bid => OrderbookWallInterpretationType::LiquidityInducementBelow,
    };
    let mut reason = vec![
        "visible wall behavior may have induced positioning before liquidity changed".to_string(),
    ];
    if side == OrderbookWallSide::Ask && active_context.bullish_pressure {
        reason
            .push("ask-side inducement overlapped with bullish active trade pressure".to_string());
    }
    if side == OrderbookWallSide::Bid && active_context.bearish_pressure {
        reason
            .push("bid-side inducement overlapped with bearish active trade pressure".to_string());
    }
    if side == OrderbookWallSide::Ask && liquidation_context.upside_pressure {
        reason.push("upside liquidation pressure amplified the inducement read".to_string());
    }
    if side == OrderbookWallSide::Bid && liquidation_context.downside_pressure {
        reason.push("downside liquidation pressure amplified the inducement read".to_string());
    }
    vec![build_signal(
        symbol,
        wall_id,
        signal_type,
        ctx,
        inferred_aggressive_volume(side, active_context),
        preferred_context_markout(side, active_context),
        36,
        18,
        86,
        73,
        ToxicConfidence::Medium,
        reason,
    )]
}

#[allow(clippy::too_many_arguments)]
fn build_signal(
    symbol: &str,
    wall_id: &str,
    signal_type: OrderbookWallInterpretationType,
    ctx: &WallInterpretationContext,
    aggressive_volume_against_wall: Option<f64>,
    post_touch_markout_bps: Option<f64>,
    spoof_score: u8,
    absorption_score: u8,
    inducement_score: u8,
    toxicity_score: u8,
    confidence: ToxicConfidence,
    reason: Vec<String>,
) -> OrderbookWallInterpretationSignal {
    let side = ctx.side.unwrap_or(OrderbookWallSide::Ask);
    let touch_count = ctx
        .tracked
        .as_ref()
        .map(|tracked| tracked.touches as u32)
        .unwrap_or(ctx.touch_events as u32);
    let persistence_ms = ctx
        .tracked
        .as_ref()
        .map(|tracked| tracked.last_seen_ms.saturating_sub(tracked.first_seen_ms))
        .unwrap_or_default();
    OrderbookWallInterpretationSignal {
        signal_id: format!(
            "wall-int-{}-{}-{}",
            interpretation_key(signal_type),
            symbol.to_ascii_lowercase(),
            ctx.latest_event_ts
        ),
        symbol: symbol.to_string(),
        ts_ms: ctx.latest_event_ts,
        wall_id: wall_id.to_string(),
        signal_type,
        side,
        wall_price: round2(ctx.price.unwrap_or_default()),
        wall_notional_usd: round2(ctx.notional.unwrap_or_default()),
        distance_to_mid_bps: round2(ctx.distance_bps.unwrap_or_default()),
        persistence_ms,
        touch_count,
        consumed_ratio: inferred_consumed_ratio(ctx),
        cancel_ratio: inferred_cancel_ratio(ctx),
        moved_count: ctx.moved_count as u32,
        aggressive_volume_against_wall: aggressive_volume_against_wall.map(round2),
        post_touch_markout_bps: post_touch_markout_bps.map(round2),
        spoof_score,
        absorption_score,
        inducement_score,
        toxicity_score,
        confidence,
        reason,
        read_only: true,
    }
}

fn preferred_markout(signal: &ActiveTradeToxicSignal) -> Option<f64> {
    signal
        .markout_5s
        .or(signal.markout_15s)
        .or(signal.markout_60s)
        .or(signal.price_change_bps)
}

fn preferred_context_markout(
    side: OrderbookWallSide,
    active_context: &ActiveTradeContext,
) -> Option<f64> {
    match side {
        OrderbookWallSide::Ask => active_context.bullish_markout,
        OrderbookWallSide::Bid => active_context.bearish_markout,
    }
}

fn inferred_aggressive_volume(
    side: OrderbookWallSide,
    active_context: &ActiveTradeContext,
) -> Option<f64> {
    match side {
        OrderbookWallSide::Ask => active_context.bullish_volume,
        OrderbookWallSide::Bid => active_context.bearish_volume,
    }
}

fn inferred_consumed_ratio(ctx: &WallInterpretationContext) -> f64 {
    if ctx.consumed {
        0.92
    } else if ctx.absorption_candidate || ctx.partial_fill_count > 0 {
        0.62
    } else if ctx.fake_candidate {
        0.05
    } else {
        0.18
    }
}

fn inferred_cancel_ratio(ctx: &WallInterpretationContext) -> f64 {
    if ctx.fake_candidate {
        0.92
    } else if ctx.removed {
        0.74
    } else if ctx.consumed || ctx.absorption_candidate {
        0.18
    } else {
        0.22
    }
}

fn interpretation_key(signal_type: OrderbookWallInterpretationType) -> &'static str {
    match signal_type {
        OrderbookWallInterpretationType::SpoofAskWall => "spoof_ask_wall",
        OrderbookWallInterpretationType::SpoofBidWall => "spoof_bid_wall",
        OrderbookWallInterpretationType::PersistentAskWall => "persistent_ask_wall",
        OrderbookWallInterpretationType::PersistentBidWall => "persistent_bid_wall",
        OrderbookWallInterpretationType::AskAbsorption => "ask_absorption",
        OrderbookWallInterpretationType::BidAbsorption => "bid_absorption",
        OrderbookWallInterpretationType::LiquidityPullAbove => "liquidity_pull_above",
        OrderbookWallInterpretationType::LiquidityPullBelow => "liquidity_pull_below",
        OrderbookWallInterpretationType::SupportWallFailure => "support_wall_failure",
        OrderbookWallInterpretationType::ResistanceWallFailure => "resistance_wall_failure",
        OrderbookWallInterpretationType::LiquidityInducementAbove => "liquidity_inducement_above",
        OrderbookWallInterpretationType::LiquidityInducementBelow => "liquidity_inducement_below",
    }
}

fn round2(value: f64) -> f64 {
    (value * 100.0).round() / 100.0
}
