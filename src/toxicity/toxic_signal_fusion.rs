use crate::types::{
    liquidation::{
        LiquidationToxicDirection, LiquidationToxicSignal, LiquidationToxicSignalType,
        LiquidationToxicityRecentResponse,
    },
    orderbook_wall::{
        OrderbookWallInterpretationReport, OrderbookWallInterpretationSignal,
        OrderbookWallInterpretationType, OrderbookWallLifecycleEvent, OrderbookWallLifecycleReport,
        OrderbookWallSide,
    },
    structural_toxicity::{
        StructuralToxicSignal, StructuralToxicSignalType, StructuralToxicityRecentResponse,
    },
    toxic_flow::{
        ActiveTradeToxicSignal, ActiveTradeToxicSignalType, ActiveTradeToxicityRecentResponse,
        ToxicConfidence,
    },
    toxic_signal::{
        ToxicChaseRisk, ToxicSignal, ToxicSignalDirection, ToxicSignalRecentResponse,
        ToxicSignalType, ToxicSupportingEvidence,
    },
};

pub fn analyze_toxic_signal_fusion(
    requested_symbol: &str,
    active_trade_recent: &ActiveTradeToxicityRecentResponse,
    liquidation_recent: &LiquidationToxicityRecentResponse,
    wall_lifecycle_report: &OrderbookWallLifecycleReport,
    wall_interpretation_report: &OrderbookWallInterpretationReport,
    structural_recent: &StructuralToxicityRecentResponse,
) -> ToxicSignalRecentResponse {
    let mut warnings = Vec::new();
    warnings.extend(active_trade_recent.warnings.clone());
    warnings.extend(liquidation_recent.warnings.clone());
    warnings.extend(wall_lifecycle_report.warnings.clone());
    warnings.extend(wall_interpretation_report.warnings.clone());
    warnings.extend(structural_recent.warnings.clone());
    warnings.sort();
    warnings.dedup();

    let mut no_trade_reasons =
        vec!["toxic fusion is analysis_only and does not emit trading instructions".to_string()];
    no_trade_reasons.extend(active_trade_recent.no_trade_reasons.clone());
    no_trade_reasons.extend(liquidation_recent.no_trade_reasons.clone());
    no_trade_reasons.extend(wall_lifecycle_report.no_trade_reasons.clone());
    no_trade_reasons.extend(wall_interpretation_report.no_trade_reasons.clone());
    no_trade_reasons.extend(structural_recent.no_trade_reasons.clone());
    no_trade_reasons.sort();
    no_trade_reasons.dedup();

    let active_signals = active_trade_recent
        .signals
        .iter()
        .filter(|signal| signal.symbol.eq_ignore_ascii_case(requested_symbol))
        .collect::<Vec<_>>();
    let liquidation_signals = liquidation_recent
        .signals
        .iter()
        .filter(|signal| signal.symbol.eq_ignore_ascii_case(requested_symbol))
        .collect::<Vec<_>>();
    let wall_events = wall_lifecycle_report
        .recent_events
        .iter()
        .filter(|event| event.symbol.eq_ignore_ascii_case(requested_symbol))
        .collect::<Vec<_>>();
    let wall_signals = wall_interpretation_report
        .signals
        .iter()
        .filter(|signal| signal.symbol.eq_ignore_ascii_case(requested_symbol))
        .collect::<Vec<_>>();
    let structural_signals = structural_recent
        .signals
        .iter()
        .filter(|signal| signal.symbol.eq_ignore_ascii_case(requested_symbol))
        .collect::<Vec<_>>();

    let mut signals = Vec::new();
    if let Some(signal) = build_short_bias_signal(
        requested_symbol,
        &active_signals,
        &liquidation_signals,
        &wall_events,
        &wall_signals,
        &structural_signals,
    ) {
        signals.push(signal);
    }
    if let Some(signal) = build_long_bias_signal(
        requested_symbol,
        &active_signals,
        &liquidation_signals,
        &wall_events,
        &wall_signals,
        &structural_signals,
    ) {
        signals.push(signal);
    }
    if let Some(signal) = build_upside_squeeze_signal(
        requested_symbol,
        &active_signals,
        &liquidation_signals,
        &wall_events,
        &wall_signals,
        &structural_signals,
    ) {
        signals.push(signal);
    }
    if let Some(signal) = build_downside_squeeze_signal(
        requested_symbol,
        &active_signals,
        &liquidation_signals,
        &wall_events,
        &wall_signals,
        &structural_signals,
    ) {
        signals.push(signal);
    }
    signals.extend(build_structural_trap_signals(
        requested_symbol,
        &structural_signals,
        &wall_signals,
        &wall_events,
    ));
    signals.extend(build_liquidity_sweep_reversal_signals(
        requested_symbol,
        &active_signals,
        &wall_events,
        &wall_signals,
        &structural_signals,
    ));
    signals.extend(build_absorption_reversal_signals(
        requested_symbol,
        &active_signals,
        &wall_events,
        &wall_signals,
        &structural_signals,
    ));
    signals.extend(build_conflict_signals(
        requested_symbol,
        &active_signals,
        &liquidation_signals,
        &wall_events,
        &wall_signals,
        &structural_signals,
    ));

    dedupe_signals(&mut signals);
    let status = if active_signals.is_empty()
        && liquidation_signals.is_empty()
        && wall_events.is_empty()
        && wall_signals.is_empty()
        && structural_signals.is_empty()
    {
        "insufficient_data"
    } else if signals.is_empty() {
        "neutral"
    } else {
        "fusion_active"
    };

    ToxicSignalRecentResponse {
        read_only: true,
        runtime_modified: false,
        analysis_only: true,
        execution_enabled: false,
        mode: "analysis_only".to_string(),
        selected_symbol: requested_symbol.to_string(),
        status: status.to_string(),
        warnings,
        no_trade_reasons,
        signals,
    }
}

fn build_short_bias_signal(
    symbol: &str,
    active_signals: &[&ActiveTradeToxicSignal],
    liquidation_signals: &[&LiquidationToxicSignal],
    wall_events: &[&OrderbookWallLifecycleEvent],
    wall_signals: &[&OrderbookWallInterpretationSignal],
    structural_signals: &[&StructuralToxicSignal],
) -> Option<ToxicSignal> {
    let active = active_signals
        .iter()
        .copied()
        .find(|signal| is_buy_pressure(signal))?;
    let wall = wall_signals
        .iter()
        .copied()
        .find(|signal| is_ask_absorption_like(signal))?;
    let structural = structural_signals
        .iter()
        .copied()
        .find(|signal| is_short_bias_structure(signal))?;
    let liquidation = liquidation_signals
        .iter()
        .copied()
        .find(|signal| is_upside_liquidation(signal));
    let lifecycle = nearest_wall_event(wall_events, OrderbookWallSide::Ask, wall.wall_price);
    let current_price = structural.current_price;
    let invalidation_price = Some(round2(structural.level_price.max(wall.wall_price)));
    let stop_distance = invalidation_price.map(|price| round2((price - current_price).abs()));
    let mut supporting_evidence = vec![
        active_evidence(active, 72, "buy-side aggression pressed into resistance"),
        wall_interpretation_evidence(wall, 82, "ask-side liquidity absorbed the aggressive flow"),
        structural_evidence(
            structural,
            84,
            "structure failed to confirm the breakout attempt",
        ),
    ];
    let mut linked_liquidation_ids = Vec::new();
    if let Some(liq) = liquidation {
        supporting_evidence.push(liquidation_evidence(
            liq,
            65,
            "upside liquidation pressure sat near the same failure area",
        ));
        linked_liquidation_ids.push(liq.signal_id.clone());
    }
    let linked_wall_lifecycle_signal_ids = lifecycle
        .iter()
        .map(|event| event.event_id.clone())
        .collect::<Vec<_>>();
    let mut reason = vec![
        "active trade toxicity aligned with ask absorption".to_string(),
        "structural failed breakout confirmed the direction".to_string(),
        "classified as short-bias toxic flow candidate".to_string(),
    ];
    reason.push("This is an invalidation reference, not an order instruction.".to_string());
    Some(build_signal(
        symbol,
        active.ts_ms.max(structural.ts_ms).max(wall.ts_ms),
        ToxicSignalType::ShortBiasToxicFlow,
        ToxicSignalDirection::ShortBias,
        88,
        ToxicConfidence::High,
        "Buy-side delta failed near resistance with ask absorption and failed breakout."
            .to_string(),
        reason,
        supporting_evidence,
        invalidation_price,
        stop_distance,
        ToxicChaseRisk::Medium,
        vec!["This is an invalidation reference, not an order instruction.".to_string()],
        vec![active.signal_id.clone()],
        linked_liquidation_ids,
        linked_wall_lifecycle_signal_ids,
        vec![wall.signal_id.clone()],
        vec![structural.signal_id.clone()],
    ))
}

fn build_long_bias_signal(
    symbol: &str,
    active_signals: &[&ActiveTradeToxicSignal],
    liquidation_signals: &[&LiquidationToxicSignal],
    wall_events: &[&OrderbookWallLifecycleEvent],
    wall_signals: &[&OrderbookWallInterpretationSignal],
    structural_signals: &[&StructuralToxicSignal],
) -> Option<ToxicSignal> {
    let active = active_signals
        .iter()
        .copied()
        .find(|signal| is_sell_pressure(signal))?;
    let wall = wall_signals
        .iter()
        .copied()
        .find(|signal| is_bid_absorption_like(signal))?;
    let structural = structural_signals
        .iter()
        .copied()
        .find(|signal| is_long_bias_structure(signal))?;
    let liquidation = liquidation_signals
        .iter()
        .copied()
        .find(|signal| is_downside_liquidation(signal));
    let lifecycle = nearest_wall_event(wall_events, OrderbookWallSide::Bid, wall.wall_price);
    let current_price = structural.current_price;
    let invalidation_price = Some(round2(structural.level_price.min(wall.wall_price)));
    let stop_distance = invalidation_price.map(|price| round2((current_price - price).abs()));
    let mut supporting_evidence = vec![
        active_evidence(active, 72, "sell-side aggression pressed into support"),
        wall_interpretation_evidence(wall, 82, "bid-side liquidity absorbed the aggressive flow"),
        structural_evidence(
            structural,
            84,
            "structure failed to confirm the breakdown attempt",
        ),
    ];
    let mut linked_liquidation_ids = Vec::new();
    if let Some(liq) = liquidation {
        supporting_evidence.push(liquidation_evidence(
            liq,
            65,
            "downside liquidation pressure sat near the same failure area",
        ));
        linked_liquidation_ids.push(liq.signal_id.clone());
    }
    let linked_wall_lifecycle_signal_ids = lifecycle
        .iter()
        .map(|event| event.event_id.clone())
        .collect::<Vec<_>>();
    let mut reason = vec![
        "active trade toxicity aligned with bid absorption".to_string(),
        "structural failed breakdown confirmed the direction".to_string(),
        "classified as long-bias toxic flow candidate".to_string(),
    ];
    reason.push("This is an invalidation reference, not an order instruction.".to_string());
    Some(build_signal(
        symbol,
        active.ts_ms.max(structural.ts_ms).max(wall.ts_ms),
        ToxicSignalType::LongBiasToxicFlow,
        ToxicSignalDirection::LongBias,
        88,
        ToxicConfidence::High,
        "Sell-side delta failed near support with bid absorption and failed breakdown.".to_string(),
        reason,
        supporting_evidence,
        invalidation_price,
        stop_distance,
        ToxicChaseRisk::Medium,
        vec!["This is an invalidation reference, not an order instruction.".to_string()],
        vec![active.signal_id.clone()],
        linked_liquidation_ids,
        linked_wall_lifecycle_signal_ids,
        vec![wall.signal_id.clone()],
        vec![structural.signal_id.clone()],
    ))
}

fn build_upside_squeeze_signal(
    symbol: &str,
    active_signals: &[&ActiveTradeToxicSignal],
    liquidation_signals: &[&LiquidationToxicSignal],
    wall_events: &[&OrderbookWallLifecycleEvent],
    wall_signals: &[&OrderbookWallInterpretationSignal],
    structural_signals: &[&StructuralToxicSignal],
) -> Option<ToxicSignal> {
    let active = active_signals
        .iter()
        .copied()
        .find(|signal| is_buy_pressure(signal))?;
    let liquidation = liquidation_signals
        .iter()
        .copied()
        .find(|signal| is_upside_liquidation(signal))?;
    let wall = wall_signals
        .iter()
        .copied()
        .find(|signal| is_upside_squeeze_wall(signal))?;
    if structural_signals
        .iter()
        .any(|signal| signal.signal_type == StructuralToxicSignalType::FailedBreakout)
    {
        return None;
    }
    let lifecycle = nearest_wall_event(wall_events, OrderbookWallSide::Ask, wall.wall_price);
    Some(build_signal(
        symbol,
        active.ts_ms.max(liquidation.ts_ms).max(wall.ts_ms),
        ToxicSignalType::SqueezeRiskUpside,
        ToxicSignalDirection::LongBias,
        79,
        ToxicConfidence::Medium,
        "Upside liquidation pressure aligned with improving buy flow and liquidity removal above."
            .to_string(),
        vec![
            "upside liquidation magnet remains active".to_string(),
            "liquidity above the market was removed instead of capping the move".to_string(),
            "this is a squeeze-risk watch, not a trade instruction".to_string(),
        ],
        vec![
            active_evidence(active, 68, "buy pressure is strengthening"),
            liquidation_evidence(
                liquidation,
                82,
                "upside liquidation cluster is still attractive",
            ),
            wall_interpretation_evidence(
                wall,
                74,
                "resistance liquidity pulled or failed instead of absorbing the move",
            ),
        ],
        None,
        None,
        ToxicChaseRisk::Medium,
        vec!["squeeze risk is informational only and does not imply a chase".to_string()],
        vec![active.signal_id.clone()],
        vec![liquidation.signal_id.clone()],
        lifecycle
            .iter()
            .map(|event| event.event_id.clone())
            .collect(),
        vec![wall.signal_id.clone()],
        Vec::new(),
    ))
}

fn build_downside_squeeze_signal(
    symbol: &str,
    active_signals: &[&ActiveTradeToxicSignal],
    liquidation_signals: &[&LiquidationToxicSignal],
    wall_events: &[&OrderbookWallLifecycleEvent],
    wall_signals: &[&OrderbookWallInterpretationSignal],
    structural_signals: &[&StructuralToxicSignal],
) -> Option<ToxicSignal> {
    let active = active_signals
        .iter()
        .copied()
        .find(|signal| is_sell_pressure(signal))?;
    let liquidation = liquidation_signals
        .iter()
        .copied()
        .find(|signal| is_downside_liquidation(signal))?;
    let wall = wall_signals
        .iter()
        .copied()
        .find(|signal| is_downside_squeeze_wall(signal))?;
    if structural_signals
        .iter()
        .any(|signal| signal.signal_type == StructuralToxicSignalType::FailedBreakdown)
    {
        return None;
    }
    let lifecycle = nearest_wall_event(wall_events, OrderbookWallSide::Bid, wall.wall_price);
    Some(build_signal(
        symbol,
        active.ts_ms.max(liquidation.ts_ms).max(wall.ts_ms),
        ToxicSignalType::SqueezeRiskDownside,
        ToxicSignalDirection::ShortBias,
        79,
        ToxicConfidence::Medium,
        "Downside liquidation pressure aligned with improving sell flow and liquidity removal below."
            .to_string(),
        vec![
            "downside liquidation magnet remains active".to_string(),
            "liquidity below the market was removed instead of defending the move".to_string(),
            "this is a squeeze-risk watch, not a trade instruction".to_string(),
        ],
        vec![
            active_evidence(active, 68, "sell pressure is strengthening"),
            liquidation_evidence(liquidation, 82, "downside liquidation cluster is still attractive"),
            wall_interpretation_evidence(
                wall,
                74,
                "support liquidity pulled or failed instead of absorbing the move",
            ),
        ],
        None,
        None,
        ToxicChaseRisk::Medium,
        vec!["squeeze risk is informational only and does not imply a chase".to_string()],
        vec![active.signal_id.clone()],
        vec![liquidation.signal_id.clone()],
        lifecycle.iter().map(|event| event.event_id.clone()).collect(),
        vec![wall.signal_id.clone()],
        Vec::new(),
    ))
}

fn build_structural_trap_signals(
    symbol: &str,
    structural_signals: &[&StructuralToxicSignal],
    wall_signals: &[&OrderbookWallInterpretationSignal],
    wall_events: &[&OrderbookWallLifecycleEvent],
) -> Vec<ToxicSignal> {
    let mut signals = Vec::new();
    for structural in structural_signals.iter().copied() {
        match structural.signal_type {
            StructuralToxicSignalType::BullTrapCandidate => {
                signals.push(build_signal(
                    symbol,
                    structural.ts_ms,
                    ToxicSignalType::BullTrapRisk,
                    ToxicSignalDirection::TrapRisk,
                    76,
                    structural.confidence,
                    "Structure flagged a bull trap candidate.".to_string(),
                    vec![
                        "price behavior matched a bull-trap pattern in the structural layer"
                            .to_string(),
                    ],
                    vec![structural_evidence(
                        structural,
                        78,
                        "structural layer already classified the move as a bull-trap candidate",
                    )],
                    None,
                    None,
                    ToxicChaseRisk::High,
                    vec!["trap risk is high; avoid chasing the move".to_string()],
                    structural.linked_active_trade_signal_ids.clone(),
                    structural.linked_liquidation_signal_ids.clone(),
                    structural.linked_wall_signal_ids.clone(),
                    structural.linked_wall_interpretation_signal_ids.clone(),
                    vec![structural.signal_id.clone()],
                ));
            }
            StructuralToxicSignalType::BearTrapCandidate => {
                signals.push(build_signal(
                    symbol,
                    structural.ts_ms,
                    ToxicSignalType::BearTrapRisk,
                    ToxicSignalDirection::TrapRisk,
                    76,
                    structural.confidence,
                    "Structure flagged a bear trap candidate.".to_string(),
                    vec![
                        "price behavior matched a bear-trap pattern in the structural layer"
                            .to_string(),
                    ],
                    vec![structural_evidence(
                        structural,
                        78,
                        "structural layer already classified the move as a bear-trap candidate",
                    )],
                    None,
                    None,
                    ToxicChaseRisk::High,
                    vec!["trap risk is high; avoid chasing the move".to_string()],
                    structural.linked_active_trade_signal_ids.clone(),
                    structural.linked_liquidation_signal_ids.clone(),
                    structural.linked_wall_signal_ids.clone(),
                    structural.linked_wall_interpretation_signal_ids.clone(),
                    vec![structural.signal_id.clone()],
                ));
            }
            _ => {}
        }
    }

    if !signals.is_empty() {
        return signals;
    }

    let spoof = wall_signals.iter().copied().find(|signal| {
        matches!(
            signal.signal_type,
            OrderbookWallInterpretationType::SpoofAskWall
                | OrderbookWallInterpretationType::SpoofBidWall
        )
    });
    let event = wall_events.first().copied();
    if let Some(spoof) = spoof {
        let event_ids = event
            .iter()
            .map(|lifecycle| lifecycle.event_id.clone())
            .collect::<Vec<_>>();
        signals.push(build_signal(
            symbol,
            spoof.ts_ms,
            ToxicSignalType::TrapRisk,
            ToxicSignalDirection::TrapRisk,
            68,
            ToxicConfidence::Medium,
            "Wall interpretation showed spoof-style behavior near an active level.".to_string(),
            vec![
                "spoof behavior raises trap risk even before stronger structural follow-through"
                    .to_string(),
            ],
            vec![wall_interpretation_evidence(
                spoof,
                70,
                "spoof-style wall behavior increases directional trap risk",
            )],
            None,
            None,
            ToxicChaseRisk::High,
            vec!["spoof-style wall behavior increases trap risk".to_string()],
            Vec::new(),
            Vec::new(),
            event_ids,
            vec![spoof.signal_id.clone()],
            Vec::new(),
        ));
    }
    signals
}

fn build_liquidity_sweep_reversal_signals(
    symbol: &str,
    active_signals: &[&ActiveTradeToxicSignal],
    wall_events: &[&OrderbookWallLifecycleEvent],
    wall_signals: &[&OrderbookWallInterpretationSignal],
    structural_signals: &[&StructuralToxicSignal],
) -> Vec<ToxicSignal> {
    let mut signals = Vec::new();
    for structural in structural_signals.iter().copied() {
        let (wall, side, primary_reason, direction) = match structural.signal_type {
            StructuralToxicSignalType::LiquiditySweepHigh => (
                wall_signals.iter().copied().find(|signal| {
                    matches!(
                        signal.signal_type,
                        OrderbookWallInterpretationType::AskAbsorption
                            | OrderbookWallInterpretationType::SpoofAskWall
                    )
                }),
                OrderbookWallSide::Ask,
                "Upside liquidity sweep reversed with wall evidence.".to_string(),
                ToxicSignalDirection::ShortBias,
            ),
            StructuralToxicSignalType::LiquiditySweepLow => (
                wall_signals.iter().copied().find(|signal| {
                    matches!(
                        signal.signal_type,
                        OrderbookWallInterpretationType::BidAbsorption
                            | OrderbookWallInterpretationType::SpoofBidWall
                    )
                }),
                OrderbookWallSide::Bid,
                "Downside liquidity sweep reversed with wall evidence.".to_string(),
                ToxicSignalDirection::LongBias,
            ),
            _ => continue,
        };
        let Some(wall) = wall else {
            continue;
        };
        let lifecycle = nearest_wall_event(wall_events, side, wall.wall_price);
        let active = active_signals
            .iter()
            .copied()
            .find(|signal| signal.side_matches_wall(side));
        let mut evidence = vec![
            structural_evidence(
                structural,
                82,
                "structural layer captured a fast sweep and reclaim/reject pattern",
            ),
            wall_interpretation_evidence(
                wall,
                76,
                "wall interpretation added absorption or spoof confirmation around the sweep",
            ),
        ];
        let mut linked_active_ids = Vec::new();
        if let Some(active) = active {
            evidence.push(active_evidence(
                active,
                66,
                "active trade flow aligned with the reversal context",
            ));
            linked_active_ids.push(active.signal_id.clone());
        }
        signals.push(build_signal(
            symbol,
            structural.ts_ms.max(wall.ts_ms),
            ToxicSignalType::LiquiditySweepReversalCandidate,
            direction,
            80,
            ToxicConfidence::High,
            primary_reason,
            vec![
                "liquidity sweep reversed back through the level".to_string(),
                "wall evidence confirmed the reversal context".to_string(),
            ],
            evidence,
            None,
            None,
            ToxicChaseRisk::Medium,
            vec![
                "wait for confirmation; sweep reversals are analysis-only observations".to_string(),
            ],
            linked_active_ids,
            Vec::new(),
            lifecycle
                .iter()
                .map(|event| event.event_id.clone())
                .collect(),
            vec![wall.signal_id.clone()],
            vec![structural.signal_id.clone()],
        ));
    }
    signals
}

fn build_absorption_reversal_signals(
    symbol: &str,
    active_signals: &[&ActiveTradeToxicSignal],
    wall_events: &[&OrderbookWallLifecycleEvent],
    wall_signals: &[&OrderbookWallInterpretationSignal],
    structural_signals: &[&StructuralToxicSignal],
) -> Vec<ToxicSignal> {
    let mut signals = Vec::new();
    for wall in wall_signals.iter().copied() {
        let structural = structural_signals.iter().copied().find(|signal| {
            signal.signal_type == StructuralToxicSignalType::KeyLevelAbsorption
                && ((signal.level_price - wall.wall_price).abs() <= 500.0)
        });
        let Some(structural) = structural else {
            continue;
        };
        let (active, direction, side, primary_reason) = match wall.signal_type {
            OrderbookWallInterpretationType::AskAbsorption => (
                active_signals
                    .iter()
                    .copied()
                    .find(|signal| is_buy_pressure(signal)),
                ToxicSignalDirection::ShortBias,
                OrderbookWallSide::Ask,
                "Buy-side aggression was absorbed at a key level and failed to continue."
                    .to_string(),
            ),
            OrderbookWallInterpretationType::BidAbsorption => (
                active_signals
                    .iter()
                    .copied()
                    .find(|signal| is_sell_pressure(signal)),
                ToxicSignalDirection::LongBias,
                OrderbookWallSide::Bid,
                "Sell-side aggression was absorbed at a key level and failed to continue."
                    .to_string(),
            ),
            _ => continue,
        };
        let Some(active) = active else {
            continue;
        };
        let lifecycle = nearest_wall_event(wall_events, side, wall.wall_price);
        signals.push(build_signal(
            symbol,
            active.ts_ms.max(wall.ts_ms).max(structural.ts_ms),
            ToxicSignalType::AbsorptionReversalCandidate,
            direction,
            83,
            ToxicConfidence::High,
            primary_reason,
            vec![
                "active trade flow hit a key level but price failed to follow".to_string(),
                "wall interpretation and structure both point to absorption".to_string(),
            ],
            vec![
                active_evidence(active, 72, "aggressive flow reached the key level first"),
                wall_interpretation_evidence(wall, 84, "wall interpretation confirmed absorption"),
                structural_evidence(
                    structural,
                    78,
                    "structural layer confirmed the absorption happened at a key level",
                ),
            ],
            None,
            None,
            ToxicChaseRisk::Medium,
            vec![
                "absorption reversal is a watch candidate, not an execution instruction"
                    .to_string(),
            ],
            vec![active.signal_id.clone()],
            structural.linked_liquidation_signal_ids.clone(),
            lifecycle
                .iter()
                .map(|event| event.event_id.clone())
                .collect(),
            vec![wall.signal_id.clone()],
            vec![structural.signal_id.clone()],
        ));
    }
    signals
}

fn build_conflict_signals(
    symbol: &str,
    active_signals: &[&ActiveTradeToxicSignal],
    liquidation_signals: &[&LiquidationToxicSignal],
    wall_events: &[&OrderbookWallLifecycleEvent],
    wall_signals: &[&OrderbookWallInterpretationSignal],
    structural_signals: &[&StructuralToxicSignal],
) -> Vec<ToxicSignal> {
    let buy_pressure = active_signals.iter().any(|signal| is_buy_pressure(signal));
    let sell_pressure = active_signals.iter().any(|signal| is_sell_pressure(signal));
    let upside_liq = liquidation_signals
        .iter()
        .any(|signal| is_upside_liquidation(signal));
    let downside_liq = liquidation_signals
        .iter()
        .any(|signal| is_downside_liquidation(signal));
    let upside_spoof_or_absorb = wall_signals.iter().any(|signal| {
        matches!(
            signal.signal_type,
            OrderbookWallInterpretationType::SpoofAskWall
                | OrderbookWallInterpretationType::AskAbsorption
        )
    });
    let downside_spoof_or_absorb = wall_signals.iter().any(|signal| {
        matches!(
            signal.signal_type,
            OrderbookWallInterpretationType::SpoofBidWall
                | OrderbookWallInterpretationType::BidAbsorption
        )
    });
    let upside_trap = structural_signals.iter().any(|signal| {
        matches!(
            signal.signal_type,
            StructuralToxicSignalType::FailedBreakout
                | StructuralToxicSignalType::LiquiditySweepHigh
                | StructuralToxicSignalType::StopHuntUpside
        )
    });
    let downside_trap = structural_signals.iter().any(|signal| {
        matches!(
            signal.signal_type,
            StructuralToxicSignalType::FailedBreakdown
                | StructuralToxicSignalType::LiquiditySweepLow
                | StructuralToxicSignalType::StopHuntDownside
        )
    });

    if !(buy_pressure && sell_pressure
        || upside_liq && downside_liq
        || upside_spoof_or_absorb && downside_spoof_or_absorb
        || upside_trap && downside_trap)
    {
        return Vec::new();
    }

    let mut evidence = Vec::new();
    if let Some(active) = active_signals.first().copied() {
        evidence.push(active_evidence(
            active,
            58,
            "active trade flow is not cleanly one-directional",
        ));
    }
    if let Some(liq) = liquidation_signals.first().copied() {
        evidence.push(liquidation_evidence(
            liq,
            60,
            "liquidation pressure exists but is not cleanly aligned",
        ));
    }
    if let Some(wall) = wall_signals.first().copied() {
        evidence.push(wall_interpretation_evidence(
            wall,
            62,
            "wall interpretation shows mixed spoof / absorption context",
        ));
    }
    if let Some(structural) = structural_signals.first().copied() {
        evidence.push(structural_evidence(
            structural,
            66,
            "structure is conflicting or trap-prone",
        ));
    }

    let linked_active_ids = active_signals
        .iter()
        .map(|signal| signal.signal_id.clone())
        .collect::<Vec<_>>();
    let linked_liquidation_ids = liquidation_signals
        .iter()
        .map(|signal| signal.signal_id.clone())
        .collect::<Vec<_>>();
    let linked_wall_ids = wall_events
        .iter()
        .map(|event| event.event_id.clone())
        .collect::<Vec<_>>();
    let linked_wall_interp_ids = wall_signals
        .iter()
        .map(|signal| signal.signal_id.clone())
        .collect::<Vec<_>>();
    let linked_structural_ids = structural_signals
        .iter()
        .map(|signal| signal.signal_id.clone())
        .collect::<Vec<_>>();
    let no_trade = vec![
        "signals conflict across active trade, wall, liquidation, and structure layers".to_string(),
        "no_trade_chop_risk is informational only and does not imply an execution response"
            .to_string(),
    ];

    vec![
        build_signal(
            symbol,
            latest_ts(
                active_signals,
                liquidation_signals,
                wall_signals,
                structural_signals,
            ),
            ToxicSignalType::TrapRisk,
            ToxicSignalDirection::TrapRisk,
            72,
            ToxicConfidence::Medium,
            "Multiple toxicity layers disagree on direction and increase trap risk.".to_string(),
            vec![
                "directional evidence is conflicted across the fused layers".to_string(),
                "trap risk is elevated until the evidence resolves".to_string(),
            ],
            evidence.clone(),
            None,
            None,
            ToxicChaseRisk::High,
            no_trade.clone(),
            linked_active_ids.clone(),
            linked_liquidation_ids.clone(),
            linked_wall_ids.clone(),
            linked_wall_interp_ids.clone(),
            linked_structural_ids.clone(),
        ),
        build_signal(
            symbol,
            latest_ts(
                active_signals,
                liquidation_signals,
                wall_signals,
                structural_signals,
            ),
            ToxicSignalType::NoTradeChopRisk,
            ToxicSignalDirection::Neutral,
            70,
            ToxicConfidence::Medium,
            "Evidence is too conflicted for a clean directional read.".to_string(),
            vec![
                "upside and downside toxic evidence are both present".to_string(),
                "stand aside until the market leaves the trap-prone area".to_string(),
            ],
            evidence,
            None,
            None,
            ToxicChaseRisk::High,
            no_trade,
            linked_active_ids,
            linked_liquidation_ids,
            linked_wall_ids,
            linked_wall_interp_ids,
            linked_structural_ids,
        ),
    ]
}

#[allow(clippy::too_many_arguments)]
fn build_signal(
    symbol: &str,
    ts_ms: u64,
    signal_type: ToxicSignalType,
    direction: ToxicSignalDirection,
    toxicity_score: u8,
    confidence: ToxicConfidence,
    primary_reason: String,
    reason: Vec<String>,
    supporting_evidence: Vec<ToxicSupportingEvidence>,
    invalidation_price: Option<f64>,
    suggested_stop_distance_usd: Option<f64>,
    chase_risk: ToxicChaseRisk,
    no_trade_reasons: Vec<String>,
    linked_active_trade_signal_ids: Vec<String>,
    linked_liquidation_signal_ids: Vec<String>,
    linked_wall_lifecycle_signal_ids: Vec<String>,
    linked_wall_interpretation_signal_ids: Vec<String>,
    linked_structural_signal_ids: Vec<String>,
) -> ToxicSignal {
    ToxicSignal {
        signal_id: format!(
            "toxic-fusion-{}-{}-{}",
            signal_type_key(signal_type),
            symbol.to_ascii_lowercase(),
            ts_ms
        ),
        symbol: symbol.to_string(),
        ts_ms,
        signal_type,
        direction,
        toxicity_score,
        confidence,
        primary_reason,
        reason,
        supporting_evidence,
        invalidation_price,
        suggested_stop_distance_usd,
        chase_risk,
        no_trade_reasons,
        linked_active_trade_signal_ids,
        linked_liquidation_signal_ids,
        linked_wall_lifecycle_signal_ids,
        linked_wall_interpretation_signal_ids,
        linked_structural_signal_ids,
        read_only: true,
    }
}

fn active_evidence(
    signal: &ActiveTradeToxicSignal,
    contribution_score: u8,
    summary: &str,
) -> ToxicSupportingEvidence {
    ToxicSupportingEvidence {
        source: "active_trade".to_string(),
        signal_id: signal.signal_id.clone(),
        signal_type: format!("{:?}", signal.signal_type).to_case(),
        contribution_score,
        summary: summary.to_string(),
    }
}

fn liquidation_evidence(
    signal: &LiquidationToxicSignal,
    contribution_score: u8,
    summary: &str,
) -> ToxicSupportingEvidence {
    ToxicSupportingEvidence {
        source: "liquidation".to_string(),
        signal_id: signal.signal_id.clone(),
        signal_type: format!("{:?}", signal.signal_type).to_case(),
        contribution_score,
        summary: summary.to_string(),
    }
}

fn wall_interpretation_evidence(
    signal: &OrderbookWallInterpretationSignal,
    contribution_score: u8,
    summary: &str,
) -> ToxicSupportingEvidence {
    ToxicSupportingEvidence {
        source: "wall_interpretation".to_string(),
        signal_id: signal.signal_id.clone(),
        signal_type: format!("{:?}", signal.signal_type).to_case(),
        contribution_score,
        summary: summary.to_string(),
    }
}

fn structural_evidence(
    signal: &StructuralToxicSignal,
    contribution_score: u8,
    summary: &str,
) -> ToxicSupportingEvidence {
    ToxicSupportingEvidence {
        source: "structural".to_string(),
        signal_id: signal.signal_id.clone(),
        signal_type: format!("{:?}", signal.signal_type).to_case(),
        contribution_score,
        summary: summary.to_string(),
    }
}

fn is_buy_pressure(signal: &ActiveTradeToxicSignal) -> bool {
    matches!(
        signal.signal_type,
        ActiveTradeToxicSignalType::OneHourDeltaBuyDominant
            | ActiveTradeToxicSignalType::LargeAggressiveBuy
            | ActiveTradeToxicSignalType::BuySweep
    )
}

fn is_sell_pressure(signal: &ActiveTradeToxicSignal) -> bool {
    matches!(
        signal.signal_type,
        ActiveTradeToxicSignalType::OneHourDeltaSellDominant
            | ActiveTradeToxicSignalType::LargeAggressiveSell
            | ActiveTradeToxicSignalType::SellSweep
    )
}

fn is_upside_liquidation(signal: &LiquidationToxicSignal) -> bool {
    matches!(
        signal.signal_type,
        LiquidationToxicSignalType::UpsideLiquidationMagnet
            | LiquidationToxicSignalType::ShortSqueezeRisk
    ) && signal.direction == LiquidationToxicDirection::Upside
}

fn is_downside_liquidation(signal: &LiquidationToxicSignal) -> bool {
    matches!(
        signal.signal_type,
        LiquidationToxicSignalType::DownsideLiquidationMagnet
            | LiquidationToxicSignalType::LongSqueezeRisk
    ) && signal.direction == LiquidationToxicDirection::Downside
}

fn is_ask_absorption_like(signal: &OrderbookWallInterpretationSignal) -> bool {
    matches!(
        signal.signal_type,
        OrderbookWallInterpretationType::AskAbsorption
            | OrderbookWallInterpretationType::PersistentAskWall
    )
}

fn is_bid_absorption_like(signal: &OrderbookWallInterpretationSignal) -> bool {
    matches!(
        signal.signal_type,
        OrderbookWallInterpretationType::BidAbsorption
            | OrderbookWallInterpretationType::PersistentBidWall
    )
}

fn is_upside_squeeze_wall(signal: &OrderbookWallInterpretationSignal) -> bool {
    matches!(
        signal.signal_type,
        OrderbookWallInterpretationType::LiquidityPullAbove
            | OrderbookWallInterpretationType::ResistanceWallFailure
    )
}

fn is_downside_squeeze_wall(signal: &OrderbookWallInterpretationSignal) -> bool {
    matches!(
        signal.signal_type,
        OrderbookWallInterpretationType::LiquidityPullBelow
            | OrderbookWallInterpretationType::SupportWallFailure
    )
}

fn is_short_bias_structure(signal: &StructuralToxicSignal) -> bool {
    matches!(
        signal.signal_type,
        StructuralToxicSignalType::FailedBreakout
            | StructuralToxicSignalType::LiquiditySweepHigh
            | StructuralToxicSignalType::DeltaStructureDivergence
            | StructuralToxicSignalType::ResistanceTrap
            | StructuralToxicSignalType::StopHuntUpside
    )
}

fn is_long_bias_structure(signal: &StructuralToxicSignal) -> bool {
    matches!(
        signal.signal_type,
        StructuralToxicSignalType::FailedBreakdown
            | StructuralToxicSignalType::LiquiditySweepLow
            | StructuralToxicSignalType::DeltaStructureDivergence
            | StructuralToxicSignalType::SupportTrap
            | StructuralToxicSignalType::StopHuntDownside
    )
}

fn nearest_wall_event<'a>(
    wall_events: &'a [&OrderbookWallLifecycleEvent],
    side: OrderbookWallSide,
    wall_price: f64,
) -> Vec<&'a OrderbookWallLifecycleEvent> {
    wall_events
        .iter()
        .copied()
        .filter(|event| event.side == side && (event.price - wall_price).abs() <= 500.0)
        .collect()
}

fn latest_ts(
    active_signals: &[&ActiveTradeToxicSignal],
    liquidation_signals: &[&LiquidationToxicSignal],
    wall_signals: &[&OrderbookWallInterpretationSignal],
    structural_signals: &[&StructuralToxicSignal],
) -> u64 {
    active_signals
        .iter()
        .map(|signal| signal.ts_ms)
        .chain(liquidation_signals.iter().map(|signal| signal.ts_ms))
        .chain(wall_signals.iter().map(|signal| signal.ts_ms))
        .chain(structural_signals.iter().map(|signal| signal.ts_ms))
        .max()
        .unwrap_or(0)
}

fn dedupe_signals(signals: &mut Vec<ToxicSignal>) {
    signals.sort_by(|left, right| left.signal_id.cmp(&right.signal_id));
    signals.dedup_by(|left, right| {
        left.signal_type == right.signal_type
            && left.direction == right.direction
            && left.ts_ms == right.ts_ms
    });
}

fn signal_type_key(signal_type: ToxicSignalType) -> &'static str {
    match signal_type {
        ToxicSignalType::ShortBiasToxicFlow => "short_bias_toxic_flow",
        ToxicSignalType::LongBiasToxicFlow => "long_bias_toxic_flow",
        ToxicSignalType::TrapRisk => "trap_risk",
        ToxicSignalType::BullTrapRisk => "bull_trap_risk",
        ToxicSignalType::BearTrapRisk => "bear_trap_risk",
        ToxicSignalType::SqueezeRiskUpside => "squeeze_risk_upside",
        ToxicSignalType::SqueezeRiskDownside => "squeeze_risk_downside",
        ToxicSignalType::AbsorptionReversalCandidate => "absorption_reversal_candidate",
        ToxicSignalType::LiquiditySweepReversalCandidate => "liquidity_sweep_reversal_candidate",
        ToxicSignalType::NoTradeChopRisk => "no_trade_chop_risk",
    }
}

fn round2(value: f64) -> f64 {
    (value * 100.0).round() / 100.0
}

trait DebugNameCase {
    fn to_case(self) -> String;
}

impl DebugNameCase for String {
    fn to_case(self) -> String {
        let mut output = String::new();
        for (index, ch) in self.chars().enumerate() {
            if ch.is_uppercase() && index > 0 {
                output.push('_');
            }
            output.extend(ch.to_lowercase());
        }
        output
    }
}

trait SideMatchesWall {
    fn side_matches_wall(&self, wall_side: OrderbookWallSide) -> bool;
}

impl SideMatchesWall for ActiveTradeToxicSignal {
    fn side_matches_wall(&self, wall_side: OrderbookWallSide) -> bool {
        match wall_side {
            OrderbookWallSide::Ask => is_buy_pressure(self),
            OrderbookWallSide::Bid => is_sell_pressure(self),
        }
    }
}
