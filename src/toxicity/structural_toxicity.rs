use crate::types::{
    liquidation::{
        LiquidationToxicDirection, LiquidationToxicSignal, LiquidationToxicityRecentResponse,
    },
    orderbook_wall::{
        OrderbookWallInterpretationReport, OrderbookWallInterpretationSignal,
        OrderbookWallInterpretationType, OrderbookWallLifecycleEvent, OrderbookWallLifecycleReport,
        OrderbookWallSide,
    },
    structural_toxicity::{
        StructuralLevelType, StructuralToxicDirection, StructuralToxicSignal,
        StructuralToxicSignalType, StructuralToxicityRecentResponse,
    },
    toxic_flow::{
        ActiveTradeToxicSignal, ActiveTradeToxicSignalType, ActiveTradeToxicityRecentResponse,
        ToxicConfidence,
    },
};

const LEVEL_MATCH_BPS: f64 = 35.0;
const LIQUIDATION_LEVEL_MATCH_BPS: f64 = 90.0;
const SWEEP_WICK_RATIO_THRESHOLD: f64 = 0.18;
const FAILED_MOVE_PRICE_CHANGE_BPS: f64 = 12.0;

pub fn analyze_structural_toxicity(
    requested_symbol: &str,
    active_trade_recent: &ActiveTradeToxicityRecentResponse,
    liquidation_recent: &LiquidationToxicityRecentResponse,
    wall_lifecycle_report: &OrderbookWallLifecycleReport,
    wall_interpretation_report: &OrderbookWallInterpretationReport,
) -> StructuralToxicityRecentResponse {
    let mut warnings = Vec::new();
    warnings.extend(active_trade_recent.warnings.clone());
    warnings.extend(liquidation_recent.warnings.clone());
    warnings.extend(wall_lifecycle_report.warnings.clone());
    warnings.extend(wall_interpretation_report.warnings.clone());

    let mut no_trade_reasons = vec![
        "structural toxicity is analysis_only and does not emit trading instructions".to_string(),
    ];
    no_trade_reasons.extend(active_trade_recent.no_trade_reasons.clone());
    no_trade_reasons.extend(liquidation_recent.no_trade_reasons.clone());
    no_trade_reasons.extend(wall_lifecycle_report.no_trade_reasons.clone());
    no_trade_reasons.extend(wall_interpretation_report.no_trade_reasons.clone());
    no_trade_reasons.sort();
    no_trade_reasons.dedup();

    let mut signals = Vec::new();
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
    let wall_interp_signals = wall_interpretation_report
        .signals
        .iter()
        .filter(|signal| signal.symbol.eq_ignore_ascii_case(requested_symbol))
        .collect::<Vec<_>>();
    let wall_events = wall_lifecycle_report
        .recent_events
        .iter()
        .filter(|event| event.symbol.eq_ignore_ascii_case(requested_symbol))
        .collect::<Vec<_>>();

    for signal in active_signals
        .iter()
        .copied()
        .filter(|signal| is_one_hour_buy(signal))
    {
        signals.extend(build_upside_structure_signals(
            requested_symbol,
            signal,
            &liquidation_signals,
            &wall_interp_signals,
            &wall_events,
        ));
    }

    for signal in active_signals
        .iter()
        .copied()
        .filter(|signal| is_one_hour_sell(signal))
    {
        signals.extend(build_downside_structure_signals(
            requested_symbol,
            signal,
            &liquidation_signals,
            &wall_interp_signals,
            &wall_events,
        ));
    }

    signals.extend(build_wall_spoof_confluence_signals(
        requested_symbol,
        &liquidation_signals,
        &wall_interp_signals,
        &wall_events,
    ));
    signals.extend(build_liquidation_wall_confluence_signals(
        requested_symbol,
        &liquidation_signals,
        &wall_interp_signals,
        &wall_events,
    ));

    dedupe_signals(&mut signals);

    let status = if active_signals.is_empty()
        && liquidation_signals.is_empty()
        && wall_interp_signals.is_empty()
        && wall_events.is_empty()
    {
        "insufficient_data"
    } else if signals.is_empty() {
        "neutral"
    } else {
        "structure_active"
    };

    StructuralToxicityRecentResponse {
        read_only: true,
        runtime_modified: false,
        mode: "analysis_only".to_string(),
        selected_symbol: requested_symbol.to_string(),
        status: status.to_string(),
        warnings,
        no_trade_reasons,
        signals,
    }
}

fn build_upside_structure_signals(
    symbol: &str,
    signal: &ActiveTradeToxicSignal,
    liquidation_signals: &[&LiquidationToxicSignal],
    wall_interp_signals: &[&OrderbookWallInterpretationSignal],
    wall_events: &[&OrderbookWallLifecycleEvent],
) -> Vec<StructuralToxicSignal> {
    let Some(level_price) = signal.high else {
        return Vec::new();
    };
    let current_price = signal.close.unwrap_or(level_price);
    let upper_wick_ratio = signal.upper_wick_ratio.unwrap_or_default();
    let price_change_bps = signal.price_change_bps.unwrap_or_default();
    let reclaim_or_reject = current_price < level_price;
    let sweep_distance_usd = (level_price - current_price).max(0.0);
    let sweep_distance_bps = bps_distance(level_price, current_price);
    let time_outside_level_ms = estimate_time_outside(signal.window_ms, upper_wick_ratio);
    let matched_liquidation = matching_liquidation_signals(
        liquidation_signals,
        LiquidationToxicDirection::Upside,
        level_price,
    );
    let matched_wall_interpretation = matching_wall_interpretation_signals(
        wall_interp_signals,
        OrderbookWallSide::Ask,
        level_price,
    );
    let matched_wall_events =
        matching_wall_events(wall_events, OrderbookWallSide::Ask, level_price);
    let active_ids = vec![signal.signal_id.clone()];
    let linked_liquidation_ids = matched_liquidation
        .iter()
        .map(|liq| liq.signal_id.clone())
        .collect::<Vec<_>>();
    let linked_wall_interp_ids = matched_wall_interpretation
        .iter()
        .map(|wall| wall.signal_id.clone())
        .collect::<Vec<_>>();
    let linked_wall_ids = matched_wall_events
        .iter()
        .map(|event| event.event_id.clone())
        .collect::<Vec<_>>();

    let mut structural_signals = Vec::new();

    if upper_wick_ratio >= SWEEP_WICK_RATIO_THRESHOLD && reclaim_or_reject {
        structural_signals.push(build_signal(
            symbol,
            signal.ts_ms,
            StructuralToxicSignalType::LiquiditySweepHigh,
            StructuralToxicDirection::UpsideTrap,
            StructuralLevelType::RecentSwingHigh,
            level_price,
            current_price,
            Some(sweep_distance_usd),
            Some(sweep_distance_bps),
            reclaim_or_reject,
            time_outside_level_ms,
            active_ids.clone(),
            linked_liquidation_ids.clone(),
            linked_wall_ids.clone(),
            linked_wall_interp_ids.clone(),
            82,
            ToxicConfidence::High,
            vec![
                "price swept above recent swing high and closed back below the level".to_string(),
                "upper wick dominance suggests an upside liquidity sweep candidate".to_string(),
            ],
        ));
    }

    if reclaim_or_reject && price_change_bps <= FAILED_MOVE_PRICE_CHANGE_BPS {
        structural_signals.push(build_signal(
            symbol,
            signal.ts_ms,
            StructuralToxicSignalType::FailedBreakout,
            StructuralToxicDirection::UpsideTrap,
            StructuralLevelType::RecentSwingHigh,
            level_price,
            current_price,
            Some(sweep_distance_usd),
            Some(sweep_distance_bps),
            reclaim_or_reject,
            time_outside_level_ms,
            active_ids.clone(),
            linked_liquidation_ids.clone(),
            linked_wall_ids.clone(),
            linked_wall_interp_ids.clone(),
            76,
            ToxicConfidence::Medium,
            vec![
                "bullish delta pushed into resistance but could not hold the breakout".to_string(),
                "price closed back under the structure level".to_string(),
            ],
        ));
    }

    if !matched_liquidation.is_empty() && upper_wick_ratio >= SWEEP_WICK_RATIO_THRESHOLD {
        structural_signals.push(build_signal(
            symbol,
            signal.ts_ms,
            StructuralToxicSignalType::StopHuntUpside,
            StructuralToxicDirection::UpsideTrap,
            StructuralLevelType::LiquidationClusterLevel,
            level_price,
            current_price,
            Some(sweep_distance_usd),
            Some(sweep_distance_bps),
            reclaim_or_reject,
            time_outside_level_ms,
            active_ids.clone(),
            linked_liquidation_ids.clone(),
            linked_wall_ids.clone(),
            linked_wall_interp_ids.clone(),
            85,
            ToxicConfidence::High,
            vec![
                "upside sweep aligned with a nearby upside liquidation cluster".to_string(),
                "structure rejected after running likely stop liquidity above the level"
                    .to_string(),
            ],
        ));
    }

    if matched_wall_interpretation.iter().any(|wall| {
        matches!(
            wall.signal_type,
            OrderbookWallInterpretationType::AskAbsorption
                | OrderbookWallInterpretationType::PersistentAskWall
        )
    }) {
        structural_signals.push(build_signal(
            symbol,
            signal.ts_ms,
            StructuralToxicSignalType::KeyLevelAbsorption,
            StructuralToxicDirection::BearishReversalCandidate,
            StructuralLevelType::WallPriceLevel,
            best_wall_level(&matched_wall_interpretation, level_price),
            current_price,
            Some(sweep_distance_usd),
            Some(sweep_distance_bps),
            reclaim_or_reject,
            time_outside_level_ms,
            active_ids.clone(),
            linked_liquidation_ids.clone(),
            linked_wall_ids.clone(),
            linked_wall_interp_ids.clone(),
            79,
            confidence_for_score(79),
            vec![
                "buy-side delta reached a resistance area with ask absorption / persistent ask wall evidence"
                    .to_string(),
                "price failed to continue cleanly above the key level".to_string(),
            ],
        ));
    }

    if matched_wall_interpretation
        .iter()
        .any(|wall| wall.signal_type == OrderbookWallInterpretationType::SpoofAskWall)
    {
        structural_signals.push(build_signal(
            symbol,
            signal.ts_ms,
            StructuralToxicSignalType::KeyLevelSpoofConfluence,
            StructuralToxicDirection::UpsideTrap,
            StructuralLevelType::WallPriceLevel,
            best_wall_level(&matched_wall_interpretation, level_price),
            current_price,
            Some(sweep_distance_usd),
            Some(sweep_distance_bps),
            reclaim_or_reject,
            time_outside_level_ms,
            active_ids.clone(),
            linked_liquidation_ids.clone(),
            linked_wall_ids.clone(),
            linked_wall_interp_ids.clone(),
            81,
            ToxicConfidence::High,
            vec![
                "spoof ask wall evidence appeared at the same resistance region".to_string(),
                "the structure rejection suggests a key-level spoof confluence".to_string(),
            ],
        ));
        structural_signals.push(build_signal(
            symbol,
            signal.ts_ms,
            StructuralToxicSignalType::ResistanceTrap,
            StructuralToxicDirection::UpsideTrap,
            StructuralLevelType::RecentSwingHigh,
            level_price,
            current_price,
            Some(sweep_distance_usd),
            Some(sweep_distance_bps),
            reclaim_or_reject,
            time_outside_level_ms,
            active_ids.clone(),
            linked_liquidation_ids.clone(),
            linked_wall_ids.clone(),
            linked_wall_interp_ids.clone(),
            77,
            ToxicConfidence::Medium,
            vec![
                "resistance held after spoof-style wall behavior near the same level".to_string(),
                "late breakout buyers risk being trapped back inside the structure".to_string(),
            ],
        ));
        structural_signals.push(build_signal(
            symbol,
            signal.ts_ms,
            StructuralToxicSignalType::BearTrapCandidate,
            StructuralToxicDirection::UpsideTrap,
            StructuralLevelType::RecentSwingHigh,
            level_price,
            current_price,
            Some(sweep_distance_usd),
            Some(sweep_distance_bps),
            reclaim_or_reject,
            time_outside_level_ms,
            active_ids.clone(),
            linked_liquidation_ids.clone(),
            linked_wall_ids.clone(),
            linked_wall_interp_ids.clone(),
            74,
            ToxicConfidence::Medium,
            vec![
                "upside continuation failed after the level sweep".to_string(),
                "structure behavior matches a bear-trap candidate for breakout chasers".to_string(),
            ],
        ));
    }

    if reclaim_or_reject {
        structural_signals.push(build_signal(
            symbol,
            signal.ts_ms,
            StructuralToxicSignalType::DeltaStructureDivergence,
            StructuralToxicDirection::UpsideTrap,
            StructuralLevelType::RecentSwingHigh,
            level_price,
            current_price,
            Some(sweep_distance_usd),
            Some(sweep_distance_bps),
            reclaim_or_reject,
            time_outside_level_ms,
            active_ids,
            linked_liquidation_ids,
            linked_wall_ids,
            linked_wall_interp_ids,
            75,
            ToxicConfidence::Medium,
            vec![
                "one_hour_delta_buy_dominant failed to produce sustained upside continuation"
                    .to_string(),
                "delta direction diverged from the structural result".to_string(),
            ],
        ));
    }

    structural_signals
}

fn build_downside_structure_signals(
    symbol: &str,
    signal: &ActiveTradeToxicSignal,
    liquidation_signals: &[&LiquidationToxicSignal],
    wall_interp_signals: &[&OrderbookWallInterpretationSignal],
    wall_events: &[&OrderbookWallLifecycleEvent],
) -> Vec<StructuralToxicSignal> {
    let Some(level_price) = signal.low else {
        return Vec::new();
    };
    let current_price = signal.close.unwrap_or(level_price);
    let lower_wick_ratio = signal.lower_wick_ratio.unwrap_or_default();
    let price_change_bps = signal.price_change_bps.unwrap_or_default();
    let reclaim_or_reject = current_price > level_price;
    let sweep_distance_usd = (current_price - level_price).max(0.0);
    let sweep_distance_bps = bps_distance(level_price, current_price);
    let time_outside_level_ms = estimate_time_outside(signal.window_ms, lower_wick_ratio);
    let matched_liquidation = matching_liquidation_signals(
        liquidation_signals,
        LiquidationToxicDirection::Downside,
        level_price,
    );
    let matched_wall_interpretation = matching_wall_interpretation_signals(
        wall_interp_signals,
        OrderbookWallSide::Bid,
        level_price,
    );
    let matched_wall_events =
        matching_wall_events(wall_events, OrderbookWallSide::Bid, level_price);
    let active_ids = vec![signal.signal_id.clone()];
    let linked_liquidation_ids = matched_liquidation
        .iter()
        .map(|liq| liq.signal_id.clone())
        .collect::<Vec<_>>();
    let linked_wall_interp_ids = matched_wall_interpretation
        .iter()
        .map(|wall| wall.signal_id.clone())
        .collect::<Vec<_>>();
    let linked_wall_ids = matched_wall_events
        .iter()
        .map(|event| event.event_id.clone())
        .collect::<Vec<_>>();

    let mut structural_signals = Vec::new();

    if lower_wick_ratio >= SWEEP_WICK_RATIO_THRESHOLD && reclaim_or_reject {
        structural_signals.push(build_signal(
            symbol,
            signal.ts_ms,
            StructuralToxicSignalType::LiquiditySweepLow,
            StructuralToxicDirection::DownsideTrap,
            StructuralLevelType::RecentSwingLow,
            level_price,
            current_price,
            Some(sweep_distance_usd),
            Some(sweep_distance_bps),
            reclaim_or_reject,
            time_outside_level_ms,
            active_ids.clone(),
            linked_liquidation_ids.clone(),
            linked_wall_ids.clone(),
            linked_wall_interp_ids.clone(),
            82,
            ToxicConfidence::High,
            vec![
                "price swept below recent swing low and reclaimed back above the level".to_string(),
                "lower wick dominance suggests a downside liquidity sweep candidate".to_string(),
            ],
        ));
    }

    if reclaim_or_reject && price_change_bps >= -FAILED_MOVE_PRICE_CHANGE_BPS {
        structural_signals.push(build_signal(
            symbol,
            signal.ts_ms,
            StructuralToxicSignalType::FailedBreakdown,
            StructuralToxicDirection::DownsideTrap,
            StructuralLevelType::RecentSwingLow,
            level_price,
            current_price,
            Some(sweep_distance_usd),
            Some(sweep_distance_bps),
            reclaim_or_reject,
            time_outside_level_ms,
            active_ids.clone(),
            linked_liquidation_ids.clone(),
            linked_wall_ids.clone(),
            linked_wall_interp_ids.clone(),
            76,
            ToxicConfidence::Medium,
            vec![
                "sell-side delta pressed through support but the breakdown did not hold"
                    .to_string(),
                "price reclaimed back above the structure level".to_string(),
            ],
        ));
    }

    if !matched_liquidation.is_empty() && lower_wick_ratio >= SWEEP_WICK_RATIO_THRESHOLD {
        structural_signals.push(build_signal(
            symbol,
            signal.ts_ms,
            StructuralToxicSignalType::StopHuntDownside,
            StructuralToxicDirection::DownsideTrap,
            StructuralLevelType::LiquidationClusterLevel,
            level_price,
            current_price,
            Some(sweep_distance_usd),
            Some(sweep_distance_bps),
            reclaim_or_reject,
            time_outside_level_ms,
            active_ids.clone(),
            linked_liquidation_ids.clone(),
            linked_wall_ids.clone(),
            linked_wall_interp_ids.clone(),
            85,
            ToxicConfidence::High,
            vec![
                "downside sweep aligned with a nearby downside liquidation cluster".to_string(),
                "structure reclaimed after likely stop liquidity was harvested below the level"
                    .to_string(),
            ],
        ));
    }

    if matched_wall_interpretation.iter().any(|wall| {
        matches!(
            wall.signal_type,
            OrderbookWallInterpretationType::BidAbsorption
                | OrderbookWallInterpretationType::PersistentBidWall
        )
    }) {
        structural_signals.push(build_signal(
            symbol,
            signal.ts_ms,
            StructuralToxicSignalType::KeyLevelAbsorption,
            StructuralToxicDirection::BullishReversalCandidate,
            StructuralLevelType::WallPriceLevel,
            best_wall_level(&matched_wall_interpretation, level_price),
            current_price,
            Some(sweep_distance_usd),
            Some(sweep_distance_bps),
            reclaim_or_reject,
            time_outside_level_ms,
            active_ids.clone(),
            linked_liquidation_ids.clone(),
            linked_wall_ids.clone(),
            linked_wall_interp_ids.clone(),
            79,
            confidence_for_score(79),
            vec![
                "sell-side delta reached a support area with bid absorption / persistent bid wall evidence"
                    .to_string(),
                "price failed to continue cleanly below the key level".to_string(),
            ],
        ));
    }

    if matched_wall_interpretation
        .iter()
        .any(|wall| wall.signal_type == OrderbookWallInterpretationType::SpoofBidWall)
    {
        structural_signals.push(build_signal(
            symbol,
            signal.ts_ms,
            StructuralToxicSignalType::KeyLevelSpoofConfluence,
            StructuralToxicDirection::DownsideTrap,
            StructuralLevelType::WallPriceLevel,
            best_wall_level(&matched_wall_interpretation, level_price),
            current_price,
            Some(sweep_distance_usd),
            Some(sweep_distance_bps),
            reclaim_or_reject,
            time_outside_level_ms,
            active_ids.clone(),
            linked_liquidation_ids.clone(),
            linked_wall_ids.clone(),
            linked_wall_interp_ids.clone(),
            81,
            ToxicConfidence::High,
            vec![
                "spoof bid wall evidence appeared at the same support region".to_string(),
                "the structure reclaim suggests a key-level spoof confluence".to_string(),
            ],
        ));
        structural_signals.push(build_signal(
            symbol,
            signal.ts_ms,
            StructuralToxicSignalType::SupportTrap,
            StructuralToxicDirection::DownsideTrap,
            StructuralLevelType::RecentSwingLow,
            level_price,
            current_price,
            Some(sweep_distance_usd),
            Some(sweep_distance_bps),
            reclaim_or_reject,
            time_outside_level_ms,
            active_ids.clone(),
            linked_liquidation_ids.clone(),
            linked_wall_ids.clone(),
            linked_wall_interp_ids.clone(),
            77,
            ToxicConfidence::Medium,
            vec![
                "support reclaimed after spoof-style bid wall behavior near the same level"
                    .to_string(),
                "late breakdown sellers risk being trapped back inside the structure".to_string(),
            ],
        ));
        structural_signals.push(build_signal(
            symbol,
            signal.ts_ms,
            StructuralToxicSignalType::BullTrapCandidate,
            StructuralToxicDirection::DownsideTrap,
            StructuralLevelType::RecentSwingLow,
            level_price,
            current_price,
            Some(sweep_distance_usd),
            Some(sweep_distance_bps),
            reclaim_or_reject,
            time_outside_level_ms,
            active_ids.clone(),
            linked_liquidation_ids.clone(),
            linked_wall_ids.clone(),
            linked_wall_interp_ids.clone(),
            74,
            ToxicConfidence::Medium,
            vec![
                "downside continuation failed after the level sweep".to_string(),
                "structure behavior matches a bull-trap candidate for breakdown chasers"
                    .to_string(),
            ],
        ));
    }

    if reclaim_or_reject {
        structural_signals.push(build_signal(
            symbol,
            signal.ts_ms,
            StructuralToxicSignalType::DeltaStructureDivergence,
            StructuralToxicDirection::DownsideTrap,
            StructuralLevelType::RecentSwingLow,
            level_price,
            current_price,
            Some(sweep_distance_usd),
            Some(sweep_distance_bps),
            reclaim_or_reject,
            time_outside_level_ms,
            active_ids,
            linked_liquidation_ids,
            linked_wall_ids,
            linked_wall_interp_ids,
            75,
            ToxicConfidence::Medium,
            vec![
                "one_hour_delta_sell_dominant failed to produce sustained downside continuation"
                    .to_string(),
                "delta direction diverged from the structural result".to_string(),
            ],
        ));
    }

    structural_signals
}

fn build_wall_spoof_confluence_signals(
    symbol: &str,
    liquidation_signals: &[&LiquidationToxicSignal],
    wall_interp_signals: &[&OrderbookWallInterpretationSignal],
    wall_events: &[&OrderbookWallLifecycleEvent],
) -> Vec<StructuralToxicSignal> {
    let mut signals = Vec::new();
    for wall in wall_interp_signals {
        match wall.signal_type {
            OrderbookWallInterpretationType::SpoofAskWall => {
                let linked_liquidation = matching_liquidation_signals(
                    liquidation_signals,
                    LiquidationToxicDirection::Upside,
                    wall.wall_price,
                );
                let linked_wall_ids =
                    matching_wall_events(wall_events, OrderbookWallSide::Ask, wall.wall_price)
                        .into_iter()
                        .map(|event| event.event_id.clone())
                        .collect::<Vec<_>>();
                signals.push(build_signal(
                    symbol,
                    wall.ts_ms,
                    StructuralToxicSignalType::KeyLevelSpoofConfluence,
                    StructuralToxicDirection::UpsideTrap,
                    StructuralLevelType::WallPriceLevel,
                    wall.wall_price,
                    wall.wall_price,
                    None,
                    None,
                    true,
                    Some(wall.persistence_ms),
                    Vec::new(),
                    linked_liquidation
                        .iter()
                        .map(|liq| liq.signal_id.clone())
                        .collect(),
                    linked_wall_ids,
                    vec![wall.signal_id.clone()],
                    78,
                    ToxicConfidence::Medium,
                    vec![
                        "spoof ask wall aligns with a key resistance area".to_string(),
                        "structure should be watched for failed breakout behavior".to_string(),
                    ],
                ));
            }
            OrderbookWallInterpretationType::SpoofBidWall => {
                let linked_liquidation = matching_liquidation_signals(
                    liquidation_signals,
                    LiquidationToxicDirection::Downside,
                    wall.wall_price,
                );
                let linked_wall_ids =
                    matching_wall_events(wall_events, OrderbookWallSide::Bid, wall.wall_price)
                        .into_iter()
                        .map(|event| event.event_id.clone())
                        .collect::<Vec<_>>();
                signals.push(build_signal(
                    symbol,
                    wall.ts_ms,
                    StructuralToxicSignalType::KeyLevelSpoofConfluence,
                    StructuralToxicDirection::DownsideTrap,
                    StructuralLevelType::WallPriceLevel,
                    wall.wall_price,
                    wall.wall_price,
                    None,
                    None,
                    true,
                    Some(wall.persistence_ms),
                    Vec::new(),
                    linked_liquidation
                        .iter()
                        .map(|liq| liq.signal_id.clone())
                        .collect(),
                    linked_wall_ids,
                    vec![wall.signal_id.clone()],
                    78,
                    ToxicConfidence::Medium,
                    vec![
                        "spoof bid wall aligns with a key support area".to_string(),
                        "structure should be watched for failed breakdown behavior".to_string(),
                    ],
                ));
            }
            _ => {}
        }
    }
    signals
}

fn build_liquidation_wall_confluence_signals(
    symbol: &str,
    liquidation_signals: &[&LiquidationToxicSignal],
    wall_interp_signals: &[&OrderbookWallInterpretationSignal],
    wall_events: &[&OrderbookWallLifecycleEvent],
) -> Vec<StructuralToxicSignal> {
    let mut signals = Vec::new();
    for liq in liquidation_signals {
        let side = match liq.direction {
            LiquidationToxicDirection::Upside => OrderbookWallSide::Ask,
            LiquidationToxicDirection::Downside => OrderbookWallSide::Bid,
            LiquidationToxicDirection::Neutral => continue,
        };
        let matching_walls =
            matching_wall_interpretation_signals(wall_interp_signals, side, liq.cluster_price);
        if matching_walls.is_empty() {
            continue;
        }
        let matching_events = matching_wall_events(wall_events, side, liq.cluster_price)
            .into_iter()
            .map(|event| event.event_id.clone())
            .collect::<Vec<_>>();
        let direction = match liq.direction {
            LiquidationToxicDirection::Upside => StructuralToxicDirection::UpsideTrap,
            LiquidationToxicDirection::Downside => StructuralToxicDirection::DownsideTrap,
            LiquidationToxicDirection::Neutral => StructuralToxicDirection::Neutral,
        };
        signals.push(build_signal(
            symbol,
            liq.ts_ms,
            StructuralToxicSignalType::LiquidationWallConfluence,
            direction,
            StructuralLevelType::LiquidationClusterLevel,
            liq.cluster_price,
            liq.current_price,
            Some(liq.distance_usd.abs()),
            Some(liq.distance_bps.abs()),
            true,
            None,
            liq.linked_active_trade_signal_ids.clone(),
            vec![liq.signal_id.clone()],
            matching_events,
            matching_walls
                .iter()
                .map(|wall| wall.signal_id.clone())
                .collect(),
            80,
            ToxicConfidence::High,
            vec![
                "liquidation cluster and wall behavior converged in the same structural zone"
                    .to_string(),
                "watch for a stop-run, failed breakout, or failed breakdown around the level"
                    .to_string(),
            ],
        ));
    }
    signals
}

fn matching_liquidation_signals<'a>(
    liquidation_signals: &'a [&LiquidationToxicSignal],
    direction: LiquidationToxicDirection,
    level_price: f64,
) -> Vec<&'a LiquidationToxicSignal> {
    liquidation_signals
        .iter()
        .copied()
        .filter(|signal| {
            signal.direction == direction
                && bps_distance(signal.cluster_price, level_price) <= LIQUIDATION_LEVEL_MATCH_BPS
        })
        .collect()
}

fn matching_wall_interpretation_signals<'a>(
    wall_signals: &'a [&OrderbookWallInterpretationSignal],
    side: OrderbookWallSide,
    level_price: f64,
) -> Vec<&'a OrderbookWallInterpretationSignal> {
    wall_signals
        .iter()
        .copied()
        .filter(|signal| {
            signal.side == side && bps_distance(signal.wall_price, level_price) <= LEVEL_MATCH_BPS
        })
        .collect()
}

fn matching_wall_events<'a>(
    wall_events: &'a [&OrderbookWallLifecycleEvent],
    side: OrderbookWallSide,
    level_price: f64,
) -> Vec<&'a OrderbookWallLifecycleEvent> {
    wall_events
        .iter()
        .copied()
        .filter(|event| {
            event.side == side && bps_distance(event.price, level_price) <= LEVEL_MATCH_BPS
        })
        .collect()
}

#[allow(clippy::too_many_arguments)]
fn build_signal(
    symbol: &str,
    ts_ms: u64,
    signal_type: StructuralToxicSignalType,
    direction: StructuralToxicDirection,
    level_type: StructuralLevelType,
    level_price: f64,
    current_price: f64,
    sweep_distance_usd: Option<f64>,
    sweep_distance_bps: Option<f64>,
    reclaim_or_reject: bool,
    time_outside_level_ms: Option<u64>,
    linked_active_trade_signal_ids: Vec<String>,
    linked_liquidation_signal_ids: Vec<String>,
    linked_wall_signal_ids: Vec<String>,
    linked_wall_interpretation_signal_ids: Vec<String>,
    toxicity_score: u8,
    confidence: ToxicConfidence,
    reason: Vec<String>,
) -> StructuralToxicSignal {
    StructuralToxicSignal {
        signal_id: format!(
            "struct-toxic-{}-{}-{}",
            signal_type_key(signal_type),
            symbol.to_ascii_lowercase(),
            ts_ms
        ),
        symbol: symbol.to_string(),
        ts_ms,
        signal_type,
        direction,
        level_type,
        level_price: round2(level_price),
        current_price: round2(current_price),
        sweep_distance_usd: sweep_distance_usd.map(round2),
        sweep_distance_bps: sweep_distance_bps.map(round2),
        reclaim_or_reject,
        time_outside_level_ms,
        linked_active_trade_signal_ids,
        linked_liquidation_signal_ids,
        linked_wall_signal_ids,
        linked_wall_interpretation_signal_ids,
        toxicity_score,
        confidence,
        reason,
        read_only: true,
    }
}

fn is_one_hour_buy(signal: &ActiveTradeToxicSignal) -> bool {
    signal.timeframe.as_deref() == Some("1h")
        && signal.signal_type == ActiveTradeToxicSignalType::OneHourDeltaBuyDominant
}

fn is_one_hour_sell(signal: &ActiveTradeToxicSignal) -> bool {
    signal.timeframe.as_deref() == Some("1h")
        && signal.signal_type == ActiveTradeToxicSignalType::OneHourDeltaSellDominant
}

fn estimate_time_outside(window_ms: u64, wick_ratio: f64) -> Option<u64> {
    if wick_ratio <= 0.0 || window_ms == 0 {
        return None;
    }
    Some(((window_ms as f64) * wick_ratio.min(0.25)).round() as u64)
}

fn best_wall_level(walls: &[&OrderbookWallInterpretationSignal], fallback: f64) -> f64 {
    walls
        .first()
        .map(|wall| wall.wall_price)
        .unwrap_or(fallback)
}

fn confidence_for_score(score: u8) -> ToxicConfidence {
    if score >= 80 {
        ToxicConfidence::High
    } else if score >= 60 {
        ToxicConfidence::Medium
    } else {
        ToxicConfidence::Low
    }
}

fn bps_distance(a: f64, b: f64) -> f64 {
    if a <= 0.0 || b <= 0.0 {
        return 0.0;
    }
    ((a - b).abs() / ((a + b) / 2.0)) * 10_000.0
}

fn dedupe_signals(signals: &mut Vec<StructuralToxicSignal>) {
    signals.sort_by(|left, right| left.signal_id.cmp(&right.signal_id));
    signals.dedup_by(|left, right| {
        left.signal_type == right.signal_type
            && left.level_type == right.level_type
            && left.ts_ms == right.ts_ms
            && left.direction == right.direction
    });
}

fn signal_type_key(signal_type: StructuralToxicSignalType) -> &'static str {
    match signal_type {
        StructuralToxicSignalType::LiquiditySweepHigh => "liquidity_sweep_high",
        StructuralToxicSignalType::LiquiditySweepLow => "liquidity_sweep_low",
        StructuralToxicSignalType::FailedBreakout => "failed_breakout",
        StructuralToxicSignalType::FailedBreakdown => "failed_breakdown",
        StructuralToxicSignalType::StopHuntUpside => "stop_hunt_upside",
        StructuralToxicSignalType::StopHuntDownside => "stop_hunt_downside",
        StructuralToxicSignalType::SupportTrap => "support_trap",
        StructuralToxicSignalType::ResistanceTrap => "resistance_trap",
        StructuralToxicSignalType::BullTrapCandidate => "bull_trap_candidate",
        StructuralToxicSignalType::BearTrapCandidate => "bear_trap_candidate",
        StructuralToxicSignalType::KeyLevelAbsorption => "key_level_absorption",
        StructuralToxicSignalType::KeyLevelSpoofConfluence => "key_level_spoof_confluence",
        StructuralToxicSignalType::LiquidationWallConfluence => "liquidation_wall_confluence",
        StructuralToxicSignalType::DeltaStructureDivergence => "delta_structure_divergence",
    }
}

fn round2(value: f64) -> f64 {
    (value * 100.0).round() / 100.0
}
