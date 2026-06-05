use crate::types::{
    market::{AggressorSide, NormalizedBook, NormalizedTrade, Venue},
    orderbook_delta::{
        ManipulationEvidenceChecklist, ManipulationResolutionStatus, ManipulationScoreBreakdown,
        ManipulationSignalType, ManipulationSignalV2, OrderBookDeltaEvent,
        OrderBookDeltaEvidenceSource, OrderBookDeltaType, VenueReliability,
    },
    orderbook_wall::OrderbookWallSide,
    toxic_flow::ToxicConfidence,
};

use super::cancel_trade_ratio::{compute_cancel_to_trade_ratio, high_cancel_without_fill};

pub const DETECTOR_VERSION: &str = "orderbook-delta-evidence-v1";

const LARGE_WALL_NOTIONAL_USD: f64 = 200_000.0;
const NEAR_TOUCH_BPS: f64 = 5.0;
const SPOOF_MAX_LIFETIME_MS: i64 = 5_000;
const LOW_FILL_RATIO: f64 = 0.20;
const HIGH_CANCEL_RATIO: f64 = 0.70;
const LAYER_MIN_LEVELS: usize = 3;
const SYNC_WINDOW_MS: i64 = 1_500;
const ICEBERG_MIN_REFILLS: usize = 3;
const ICEBERG_HIDDEN_RATIO: f64 = 3.0;

#[derive(Debug, Clone, Copy)]
pub struct DeltaDetectorContext {
    pub window_ms: u64,
    pub markout_1s_bps: Option<f64>,
    pub markout_5s_bps: Option<f64>,
    pub markout_30s_bps: Option<f64>,
    pub venue_reliability_score: f64,
}

impl Default for DeltaDetectorContext {
    fn default() -> Self {
        Self {
            window_ms: 5_000,
            markout_1s_bps: None,
            markout_5s_bps: None,
            markout_30s_bps: None,
            venue_reliability_score: 1.0,
        }
    }
}

#[derive(Debug, Clone)]
pub struct OrderBookDeltaDetector {
    context: DeltaDetectorContext,
}

impl OrderBookDeltaDetector {
    pub fn new(context: DeltaDetectorContext) -> Self {
        Self { context }
    }

    pub fn detect(
        &self,
        deltas: &[OrderBookDeltaEvent],
        trades: &[NormalizedTrade],
    ) -> Vec<ManipulationSignalV2> {
        let mut signals = Vec::new();
        signals.extend(self.detect_spoofing(deltas, trades));
        signals.extend(self.detect_layering(deltas, trades));
        signals.extend(self.detect_iceberg(deltas, trades));
        dedupe_signals(signals)
    }

    fn detect_spoofing(
        &self,
        deltas: &[OrderBookDeltaEvent],
        trades: &[NormalizedTrade],
    ) -> Vec<ManipulationSignalV2> {
        let mut signals = Vec::new();
        for add in deltas
            .iter()
            .filter(|delta| delta.delta_type == OrderBookDeltaType::Add)
        {
            if notional(add.price, add.qty_after) < LARGE_WALL_NOTIONAL_USD {
                continue;
            }
            let Some(remove) = deltas.iter().find(|delta| {
                same_level(add, delta)
                    && matches!(
                        delta.delta_type,
                        OrderBookDeltaType::Cancel
                            | OrderBookDeltaType::Reduce
                            | OrderBookDeltaType::Remove
                    )
                    && delta.ts >= add.ts
                    && delta.ts.saturating_sub(add.ts) <= SPOOF_MAX_LIFETIME_MS
            }) else {
                continue;
            };

            let canceled_qty = remove.cancel_qty.unwrap_or_else(|| remove.delta_qty.abs());
            if ratio(canceled_qty, add.qty_after).is_none_or(|value| value < HIGH_CANCEL_RATIO) {
                continue;
            }
            let fill_qty = remove.fill_qty.unwrap_or(0.0);
            let traded_against = traded_against_level(remove, trades);
            let cancel_to_trade_ratio = ratio(canceled_qty, traded_against + fill_qty);
            let mut checklist = ManipulationEvidenceChecklist {
                large_wall_appeared: true,
                near_touch: add
                    .distance_to_touch_bps
                    .or(remove.distance_to_touch_bps)
                    .is_some_and(|bps| bps <= NEAR_TOUCH_BPS),
                low_fill_participation: ratio(fill_qty + traded_against, canceled_qty)
                    .is_some_and(|value| value <= LOW_FILL_RATIO),
                wall_removed: true,
                post_remove_markout: markout_matches_side(remove.side, &self.context),
                opposite_aggressive_flow: opposite_aggressive_flow(remove, trades),
                ..ManipulationEvidenceChecklist::default()
            };
            let evidence_count = spoof_evidence_count(&checklist);
            let risk_score = bounded_score(25 + evidence_count * 11);
            let confidence = confidence_from_score(risk_score);
            if evidence_count < 6 {
                checklist.post_remove_markout = markout_matches_side(remove.side, &self.context);
            }

            signals.push(self.build_signal(
                ManipulationSignalType::SpoofingCandidate,
                remove,
                Some(add.price),
                add.qty_after,
                canceled_qty,
                fill_qty + traded_against,
                cancel_to_trade_ratio,
                risk_score,
                confidence,
                checklist,
                vec![
                    "large visible wall appeared and later reduced or disappeared".to_string(),
                    "signal remains candidate until full spoof evidence chain is present"
                        .to_string(),
                ],
            ));
        }
        signals
    }

    fn detect_layering(
        &self,
        deltas: &[OrderBookDeltaEvent],
        trades: &[NormalizedTrade],
    ) -> Vec<ManipulationSignalV2> {
        let mut signals = Vec::new();
        for (venue, symbol, side) in unique_venue_symbol_sides(deltas) {
            let adds = deltas
                .iter()
                .filter(|delta| {
                    delta.venue == venue
                        && delta.symbol == symbol
                        && delta.side == side
                        && delta.delta_type == OrderBookDeltaType::Add
                        && notional(delta.price, delta.qty_after) >= LARGE_WALL_NOTIONAL_USD
                })
                .collect::<Vec<_>>();
            if distinct_prices(&adds) < LAYER_MIN_LEVELS || !within_sync_window(&adds) {
                continue;
            }
            let removals = deltas
                .iter()
                .filter(|delta| {
                    delta.venue == venue
                        && delta.symbol == symbol
                        && delta.side == side
                        && matches!(
                            delta.delta_type,
                            OrderBookDeltaType::Cancel
                                | OrderBookDeltaType::Reduce
                                | OrderBookDeltaType::Remove
                        )
                        && adds.iter().any(|add| same_level(add, delta))
                })
                .collect::<Vec<_>>();
            if distinct_prices(&removals) < LAYER_MIN_LEVELS || !within_sync_window(&removals) {
                continue;
            }

            let add_qty = adds.iter().map(|delta| delta.qty_after).sum::<f64>();
            let cancel_qty = removals
                .iter()
                .map(|delta| delta.cancel_qty.unwrap_or_else(|| delta.delta_qty.abs()))
                .sum::<f64>();
            let fill_qty = removals
                .iter()
                .map(|delta| delta.fill_qty.unwrap_or(0.0) + traded_against_level(delta, trades))
                .sum::<f64>();
            let cancel_to_trade_ratio = compute_cancel_to_trade_ratio(cancel_qty, fill_qty);
            let mut checklist = ManipulationEvidenceChecklist {
                large_wall_appeared: true,
                synchronized_levels: true,
                wall_removed: true,
                high_cancel_ratio: cancel_to_trade_ratio
                    .is_some_and(|value| value >= HIGH_CANCEL_RATIO)
                    || high_cancel_without_fill(cancel_qty, fill_qty),
                low_fill_participation: ratio(fill_qty, cancel_qty)
                    .is_some_and(|value| value <= LOW_FILL_RATIO),
                post_remove_markout: markout_matches_side(side, &self.context),
                ..ManipulationEvidenceChecklist::default()
            };
            let evidence_count = [
                checklist.large_wall_appeared,
                checklist.synchronized_levels,
                checklist.wall_removed,
                checklist.high_cancel_ratio,
                checklist.low_fill_participation,
                checklist.post_remove_markout,
            ]
            .into_iter()
            .filter(|value| *value)
            .count() as u8;
            let risk_score = bounded_score(30 + evidence_count * 10);
            if !checklist.post_remove_markout {
                checklist.post_remove_markout = false;
            }
            if let Some(anchor) = removals.first() {
                signals.push(self.build_signal(
                    ManipulationSignalType::LayeringCandidate,
                    anchor,
                    None,
                    add_qty,
                    cancel_qty,
                    fill_qty,
                    cancel_to_trade_ratio,
                    risk_score,
                    confidence_from_score(risk_score),
                    checklist,
                    vec![
                        "multiple same-side levels appeared and were reduced in sync".to_string(),
                        "price-level L2 evidence cannot prove native order intent".to_string(),
                    ],
                ));
            }
        }
        signals
    }

    fn detect_iceberg(
        &self,
        deltas: &[OrderBookDeltaEvent],
        trades: &[NormalizedTrade],
    ) -> Vec<ManipulationSignalV2> {
        let mut signals = Vec::new();
        for (venue, symbol, side, price) in unique_levels(deltas) {
            let refills = deltas
                .iter()
                .filter(|delta| {
                    delta.venue == venue
                        && delta.symbol == symbol
                        && delta.side == side
                        && price_eq(delta.price, price)
                        && delta.delta_type == OrderBookDeltaType::Refill
                })
                .collect::<Vec<_>>();
            if refills.len() < ICEBERG_MIN_REFILLS {
                continue;
            }
            let traded_qty = trades
                .iter()
                .filter(|trade| {
                    trade.venue == venue && trade.symbol == symbol && price_eq(trade.price, price)
                })
                .map(|trade| trade.size_btc)
                .sum::<f64>();
            let max_displayed = refills
                .iter()
                .map(|delta| delta.qty_after.max(delta.qty_before))
                .fold(0.0, f64::max);
            let hidden_ratio = ratio(traded_qty, max_displayed).unwrap_or(0.0);
            let stable_interval = stable_refill_interval(&refills);
            if hidden_ratio < ICEBERG_HIDDEN_RATIO || !stable_interval {
                continue;
            }
            let anchor = refills[refills.len() - 1];
            let checklist = ManipulationEvidenceChecklist {
                repeated_refill: true,
                stable_refill_interval: stable_interval,
                hidden_liquidity_ratio: true,
                low_fill_participation: false,
                ..ManipulationEvidenceChecklist::default()
            };
            let risk_score = bounded_score(58 + refills.len().min(5) as u8 * 6);
            signals.push(self.build_signal(
                ManipulationSignalType::IcebergCandidate,
                anchor,
                Some(price),
                refills.iter().map(|delta| delta.delta_qty.max(0.0)).sum(),
                0.0,
                traded_qty,
                None,
                risk_score,
                confidence_from_score(risk_score),
                checklist,
                vec![
                    "same price repeatedly refilled while executed volume exceeded displayed size"
                        .to_string(),
                    "iceberg signal is inferred from L2 depth and trades".to_string(),
                ],
            ));
        }
        signals
    }

    #[allow(clippy::too_many_arguments)]
    fn build_signal(
        &self,
        signal_type: ManipulationSignalType,
        anchor: &OrderBookDeltaEvent,
        price: Option<f64>,
        add_qty: f64,
        cancel_qty: f64,
        fill_qty: f64,
        cancel_to_trade_ratio: Option<f64>,
        risk_score: u8,
        confidence: ToxicConfidence,
        checklist: ManipulationEvidenceChecklist,
        reasons: Vec<String>,
    ) -> ManipulationSignalV2 {
        let adjusted_score =
            apply_reliability_to_score(risk_score, self.context.venue_reliability_score);
        ManipulationSignalV2 {
            signal_id: format!(
                "{}:{}",
                signal_type_key(signal_type),
                anchor.semantic_dedupe_key()
            ),
            detector_version: DETECTOR_VERSION.to_string(),
            signal_type,
            venue: anchor.venue,
            symbol: anchor.symbol.clone(),
            side: anchor.side,
            window_ms: self.context.window_ms,
            observed_start_ms: anchor.ts.saturating_sub(self.context.window_ms as i64),
            observed_end_ms: anchor.ts,
            price,
            add_qty,
            cancel_qty,
            fill_qty,
            cancel_to_trade_ratio,
            depth_before: anchor.depth_before,
            depth_after: anchor.depth_after,
            price_impact_bps: self.context.markout_1s_bps,
            markout_1s_bps: self.context.markout_1s_bps,
            markout_5s_bps: self.context.markout_5s_bps,
            markout_30s_bps: self.context.markout_30s_bps,
            risk_score: adjusted_score,
            confidence: if self.context.venue_reliability_score < 0.70 {
                ToxicConfidence::Low
            } else {
                confidence
            },
            score_breakdown: ManipulationScoreBreakdown {
                toxicity_score: adjusted_score,
                confidence_score: confidence_score(confidence),
                data_quality_score: data_quality_score(anchor.evidence_source),
                markout_evidence_score: markout_score(&self.context),
                venue_reliability_score: (self.context.venue_reliability_score.clamp(0.0, 1.0)
                    * 100.0)
                    .round() as u8,
            },
            data_quality: data_quality_label(anchor.evidence_source).to_string(),
            dedupe_key: format!(
                "{}:{}:{}",
                signal_type_key(signal_type),
                anchor.symbol,
                anchor.semantic_dedupe_key()
            ),
            raw_evidence_links: vec![anchor.semantic_dedupe_key()],
            resolution_status: ManipulationResolutionStatus::Candidate,
            evidence_source: anchor.evidence_source,
            evidence_checklist: checklist,
            reasons,
            read_only: true,
        }
    }
}

pub fn derive_l2_deltas(
    previous: &NormalizedBook,
    current: &NormalizedBook,
    sequence: u64,
) -> Vec<OrderBookDeltaEvent> {
    let mut deltas = Vec::new();
    deltas.extend(derive_side_deltas(
        previous,
        current,
        OrderbookWallSide::Bid,
        &previous.bids,
        &current.bids,
        sequence,
    ));
    deltas.extend(derive_side_deltas(
        previous,
        current,
        OrderbookWallSide::Ask,
        &previous.asks,
        &current.asks,
        sequence,
    ));
    deltas
}

pub fn apply_venue_reliability(signal: &mut ManipulationSignalV2, reliability: VenueReliability) {
    if signal.venue != reliability.venue {
        return;
    }
    signal.risk_score =
        apply_reliability_to_score(signal.risk_score, reliability.reliability_score);
    signal.score_breakdown.venue_reliability_score =
        (reliability.reliability_score.clamp(0.0, 1.0) * 100.0).round() as u8;
    if reliability.reliability_score < 0.70 {
        signal.confidence = ToxicConfidence::Low;
        signal
            .reasons
            .push("venue reliability filter downgraded this candidate".to_string());
    }
}

fn derive_side_deltas(
    previous: &NormalizedBook,
    current: &NormalizedBook,
    side: OrderbookWallSide,
    previous_levels: &[(f64, f64)],
    current_levels: &[(f64, f64)],
    sequence: u64,
) -> Vec<OrderBookDeltaEvent> {
    let mut prices = previous_levels
        .iter()
        .chain(current_levels.iter())
        .map(|(price, _)| *price)
        .collect::<Vec<_>>();
    prices.sort_by(f64::total_cmp);
    prices.dedup_by(|left, right| price_eq(*left, *right));

    prices
        .into_iter()
        .filter_map(|price| {
            let before = level_qty(previous_levels, price);
            let after = level_qty(current_levels, price);
            if (before - after).abs() < f64::EPSILON {
                return None;
            }
            let delta_type = if before <= 0.0 && after > 0.0 {
                OrderBookDeltaType::Add
            } else if before > 0.0 && after <= 0.0 {
                OrderBookDeltaType::Remove
            } else if after > before {
                OrderBookDeltaType::Refill
            } else if after < before {
                OrderBookDeltaType::Reduce
            } else {
                OrderBookDeltaType::Unknown
            };
            let delta_qty = after - before;
            let cancel_qty = matches!(
                delta_type,
                OrderBookDeltaType::Reduce | OrderBookDeltaType::Remove
            )
            .then_some(delta_qty.abs());
            Some(OrderBookDeltaEvent {
                venue: current.venue,
                symbol: current.symbol.clone(),
                side,
                price,
                qty_before: before,
                qty_after: after,
                delta_qty,
                delta_type,
                ts: current.ts,
                sequence,
                order_id: None,
                lifetime_ms: None,
                fill_qty: None,
                cancel_qty,
                evidence_source: OrderBookDeltaEvidenceSource::InferredFromL2Delta,
                distance_to_touch_bps: Some(distance_to_touch_bps(side, price, current)),
                depth_before: Some(side_depth(previous, side)),
                depth_after: Some(side_depth(current, side)),
            })
        })
        .collect()
}

fn level_qty(levels: &[(f64, f64)], price: f64) -> f64 {
    levels
        .iter()
        .find(|(level_price, _)| price_eq(*level_price, price))
        .map(|(_, qty)| *qty)
        .unwrap_or(0.0)
}

fn side_depth(book: &NormalizedBook, side: OrderbookWallSide) -> f64 {
    match side {
        OrderbookWallSide::Bid => book.bid_depth_btc_10bps,
        OrderbookWallSide::Ask => book.ask_depth_btc_10bps,
    }
}

fn distance_to_touch_bps(side: OrderbookWallSide, price: f64, book: &NormalizedBook) -> f64 {
    let touch = match side {
        OrderbookWallSide::Bid => book.best_bid,
        OrderbookWallSide::Ask => book.best_ask,
    };
    ((price - touch).abs() / touch.max(1.0)) * 10_000.0
}

fn same_level(left: &OrderBookDeltaEvent, right: &OrderBookDeltaEvent) -> bool {
    left.venue == right.venue
        && left.symbol == right.symbol
        && left.side == right.side
        && price_eq(left.price, right.price)
}

fn price_eq(left: f64, right: f64) -> bool {
    (left - right).abs() <= 0.000_000_1
}

fn notional(price: f64, qty: f64) -> f64 {
    price.abs() * qty.abs()
}

fn ratio(numerator: f64, denominator: f64) -> Option<f64> {
    (denominator > 0.0).then_some(numerator / denominator)
}

fn markout_matches_side(side: OrderbookWallSide, context: &DeltaDetectorContext) -> bool {
    let markout = context
        .markout_1s_bps
        .or(context.markout_5s_bps)
        .or(context.markout_30s_bps);
    match side {
        OrderbookWallSide::Ask => markout.is_some_and(|bps| bps > 0.0),
        OrderbookWallSide::Bid => markout.is_some_and(|bps| bps < 0.0),
    }
}

fn opposite_aggressive_flow(delta: &OrderBookDeltaEvent, trades: &[NormalizedTrade]) -> bool {
    let expected = match delta.side {
        OrderbookWallSide::Ask => AggressorSide::Buy,
        OrderbookWallSide::Bid => AggressorSide::Sell,
    };
    trades.iter().any(|trade| {
        trade.venue == delta.venue
            && trade.symbol == delta.symbol
            && trade.ts >= delta.ts
            && trade.aggressor_side == expected
    })
}

fn traded_against_level(delta: &OrderBookDeltaEvent, trades: &[NormalizedTrade]) -> f64 {
    trades
        .iter()
        .filter(|trade| {
            trade.venue == delta.venue
                && trade.symbol == delta.symbol
                && price_eq(trade.price, delta.price)
        })
        .map(|trade| trade.size_btc)
        .sum()
}

fn spoof_evidence_count(checklist: &ManipulationEvidenceChecklist) -> u8 {
    [
        checklist.large_wall_appeared,
        checklist.near_touch,
        checklist.low_fill_participation,
        checklist.wall_removed,
        checklist.post_remove_markout,
        checklist.opposite_aggressive_flow,
    ]
    .into_iter()
    .filter(|value| *value)
    .count() as u8
}

fn bounded_score(score: u8) -> u8 {
    score.min(100)
}

fn confidence_from_score(score: u8) -> ToxicConfidence {
    if score >= 80 {
        ToxicConfidence::High
    } else if score >= 60 {
        ToxicConfidence::Medium
    } else {
        ToxicConfidence::Low
    }
}

fn confidence_score(confidence: ToxicConfidence) -> u8 {
    match confidence {
        ToxicConfidence::High => 90,
        ToxicConfidence::Medium => 65,
        ToxicConfidence::Low => 35,
    }
}

fn data_quality_score(source: OrderBookDeltaEvidenceSource) -> u8 {
    match source {
        OrderBookDeltaEvidenceSource::NativeOrderEvent => 95,
        OrderBookDeltaEvidenceSource::InferredFromL2Delta => 68,
    }
}

fn data_quality_label(source: OrderBookDeltaEvidenceSource) -> &'static str {
    match source {
        OrderBookDeltaEvidenceSource::NativeOrderEvent => "native_order_event",
        OrderBookDeltaEvidenceSource::InferredFromL2Delta => "inferred_from_l2_delta",
    }
}

fn markout_score(context: &DeltaDetectorContext) -> u8 {
    if context.markout_1s_bps.is_some() || context.markout_5s_bps.is_some() {
        80
    } else if context.markout_30s_bps.is_some() {
        55
    } else {
        20
    }
}

fn apply_reliability_to_score(score: u8, reliability: f64) -> u8 {
    if reliability >= 0.70 {
        score
    } else {
        ((score as f64) * reliability.clamp(0.0, 1.0)).round() as u8
    }
}

fn unique_venue_symbol_sides(
    deltas: &[OrderBookDeltaEvent],
) -> Vec<(Venue, String, OrderbookWallSide)> {
    let mut values = Vec::new();
    for delta in deltas {
        let value = (delta.venue, delta.symbol.clone(), delta.side);
        if !values.contains(&value) {
            values.push(value);
        }
    }
    values
}

fn unique_levels(deltas: &[OrderBookDeltaEvent]) -> Vec<(Venue, String, OrderbookWallSide, f64)> {
    let mut values = Vec::new();
    for delta in deltas {
        let exists = values.iter().any(|(venue, symbol, side, price)| {
            *venue == delta.venue
                && symbol == &delta.symbol
                && *side == delta.side
                && price_eq(*price, delta.price)
        });
        if !exists {
            values.push((delta.venue, delta.symbol.clone(), delta.side, delta.price));
        }
    }
    values
}

fn distinct_prices(deltas: &[&OrderBookDeltaEvent]) -> usize {
    let mut prices = Vec::new();
    for delta in deltas {
        if !prices.iter().any(|price| price_eq(*price, delta.price)) {
            prices.push(delta.price);
        }
    }
    prices.len()
}

fn within_sync_window(deltas: &[&OrderBookDeltaEvent]) -> bool {
    let Some(min_ts) = deltas.iter().map(|delta| delta.ts).min() else {
        return false;
    };
    let Some(max_ts) = deltas.iter().map(|delta| delta.ts).max() else {
        return false;
    };
    max_ts.saturating_sub(min_ts) <= SYNC_WINDOW_MS
}

fn stable_refill_interval(refills: &[&OrderBookDeltaEvent]) -> bool {
    if refills.len() < ICEBERG_MIN_REFILLS {
        return false;
    }
    let mut ts = refills.iter().map(|delta| delta.ts).collect::<Vec<_>>();
    ts.sort_unstable();
    let intervals = ts
        .windows(2)
        .map(|pair| pair[1].saturating_sub(pair[0]))
        .collect::<Vec<_>>();
    let Some(min_interval) = intervals.iter().min() else {
        return false;
    };
    let Some(max_interval) = intervals.iter().max() else {
        return false;
    };
    *min_interval > 0 && *max_interval <= *min_interval * 2
}

fn dedupe_signals(signals: Vec<ManipulationSignalV2>) -> Vec<ManipulationSignalV2> {
    let mut seen = std::collections::BTreeSet::new();
    let mut deduped = Vec::new();
    for signal in signals {
        if seen.insert(signal.dedupe_key.clone()) {
            deduped.push(signal);
        }
    }
    deduped
}

fn signal_type_key(signal_type: ManipulationSignalType) -> &'static str {
    match signal_type {
        ManipulationSignalType::SpoofingCandidate => "spoofing_candidate",
        ManipulationSignalType::LayeringCandidate => "layering_candidate",
        ManipulationSignalType::IcebergCandidate => "iceberg_candidate",
    }
}
