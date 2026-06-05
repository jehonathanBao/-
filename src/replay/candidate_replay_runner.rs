use std::collections::{BTreeMap, BTreeSet};

use serde::{Deserialize, Serialize};

use crate::{
    replay::{
        candidate_replay_event::{
            load_candidate_replay_file, CandidateReplayEvent, CandidateReplayEventType,
        },
        markout_evaluator::{evaluate_candidate_markout, ReplayPricePoint},
    },
    toxicity::orderbook_delta_evidence::{DeltaDetectorContext, OrderBookDeltaDetector},
    types::{
        market::{AggressorSide, NormalizedTrade},
        orderbook_delta::{
            ManipulationSignalType, ManipulationSignalV2, OrderBookDeltaEvent,
            OrderBookDeltaEvidenceSource, OrderBookDeltaType,
        },
        toxic_signal::ToxicSignal,
    },
};

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CandidateReplaySummary {
    pub input_path: String,
    pub total_events: usize,
    pub total_signals: usize,
    pub signals_by_type: BTreeMap<String, usize>,
    pub signals_by_symbol: BTreeMap<String, usize>,
    pub average_score: f64,
    pub max_score: u8,
    pub deduped_count: usize,
    pub data_quality_average: f64,
    pub signals: Vec<ToxicSignal>,
}

pub fn run_candidate_replay_file(
    path: impl AsRef<std::path::Path>,
) -> anyhow::Result<CandidateReplaySummary> {
    let input_path = path.as_ref().display().to_string();
    let events = load_candidate_replay_file(&path)?;
    Ok(run_candidate_replay_events(input_path, events))
}

pub fn run_candidate_replay_events(
    input_path: String,
    mut events: Vec<CandidateReplayEvent>,
) -> CandidateReplaySummary {
    events.sort_by_key(|event| event.ts_ms);
    let total_events = events.len();
    let mut deltas = Vec::new();
    let mut trades = Vec::new();
    let mut prices = Vec::new();
    let detector = OrderBookDeltaDetector::new(DeltaDetectorContext::default());
    let mut collected = Vec::new();

    for event in events {
        match event.event_type {
            CandidateReplayEventType::Trade => {
                if let Some(trade) = trade_from_event(&event) {
                    trades.push(trade);
                }
            }
            CandidateReplayEventType::BookDelta | CandidateReplayEventType::SnapshotReset => {
                if let Some(delta) = delta_from_event(&event) {
                    deltas.push(delta);
                }
            }
            CandidateReplayEventType::Snapshot => {
                if let Some(price) = event.price {
                    prices.push(ReplayPricePoint {
                        ts_ms: event.ts_ms,
                        mid: price,
                    });
                }
            }
        }
        collected.extend(detector.detect(&deltas, &trades));
    }

    let mut seen = BTreeSet::new();
    let mut deduped_count = 0;
    let mut unique = Vec::new();
    for signal in collected {
        if seen.insert(signal.dedupe_key.clone()) {
            unique.push(signal);
        } else {
            deduped_count += 1;
        }
    }

    let signals = unique
        .into_iter()
        .map(|signal| to_marked_toxic_signal(signal, &prices))
        .collect::<Vec<_>>();

    let total_signals = signals.len();
    let mut signals_by_type = BTreeMap::new();
    let mut signals_by_symbol = BTreeMap::new();
    let mut total_score = 0.0;
    let mut max_score = 0;
    let mut total_data_quality = 0.0;

    for signal in &signals {
        *signals_by_type
            .entry(format!("{:?}", signal.signal_type))
            .or_insert(0) += 1;
        *signals_by_symbol.entry(signal.symbol.clone()).or_insert(0) += 1;
        total_score += signal.toxicity_score as f64;
        max_score = max_score.max(signal.toxicity_score);
        total_data_quality += signal.data_quality.unwrap_or(0.0);
    }

    CandidateReplaySummary {
        input_path,
        total_events,
        total_signals,
        signals_by_type,
        signals_by_symbol,
        average_score: average(total_score, total_signals),
        max_score,
        deduped_count,
        data_quality_average: average(total_data_quality, total_signals),
        signals,
    }
}

fn to_marked_toxic_signal(
    mut signal: ManipulationSignalV2,
    prices: &[ReplayPricePoint],
) -> ToxicSignal {
    let markout = evaluate_candidate_markout(&signal, prices);
    signal.markout_1s_bps = markout.markout_1s_bps;
    signal.markout_5s_bps = markout.markout_5s_bps;
    signal.markout_30s_bps = markout.markout_30s_bps;
    signal.score_breakdown.markout_evidence_score =
        if markout.markout_1s_bps.is_some() || markout.markout_5s_bps.is_some() {
            80
        } else {
            20
        };
    signal.to_toxic_signal()
}

fn delta_from_event(event: &CandidateReplayEvent) -> Option<OrderBookDeltaEvent> {
    let side = event.side?;
    let price = event.price?;
    let before = event.qty_before.unwrap_or(0.0);
    let after = event.qty_after.unwrap_or_else(|| event.qty.unwrap_or(0.0));
    let delta_type = match event.event_type {
        CandidateReplayEventType::SnapshotReset => OrderBookDeltaType::SnapshotReset,
        CandidateReplayEventType::BookDelta if before <= 0.0 && after > 0.0 => {
            OrderBookDeltaType::Add
        }
        CandidateReplayEventType::BookDelta if before > 0.0 && after <= 0.0 => {
            OrderBookDeltaType::Remove
        }
        CandidateReplayEventType::BookDelta if after > before => OrderBookDeltaType::Refill,
        CandidateReplayEventType::BookDelta if after < before => OrderBookDeltaType::Reduce,
        _ => OrderBookDeltaType::Unknown,
    };
    let delta_qty = after - before;
    Some(OrderBookDeltaEvent {
        venue: event.venue,
        symbol: event.symbol.clone(),
        side,
        price,
        qty_before: before,
        qty_after: after,
        delta_qty,
        delta_type,
        ts: event.ts_ms,
        sequence: event.sequence.unwrap_or(event.ts_ms.max(0) as u64),
        order_id: event.order_id.clone(),
        lifetime_ms: None,
        fill_qty: None,
        cancel_qty: matches!(
            delta_type,
            OrderBookDeltaType::Cancel | OrderBookDeltaType::Reduce | OrderBookDeltaType::Remove
        )
        .then_some(delta_qty.abs()),
        evidence_source: if event.order_id.is_some() {
            OrderBookDeltaEvidenceSource::NativeOrderEvent
        } else {
            OrderBookDeltaEvidenceSource::InferredFromL2Delta
        },
        distance_to_touch_bps: Some(2.0),
        depth_before: Some(before),
        depth_after: Some(after),
    })
}

fn trade_from_event(event: &CandidateReplayEvent) -> Option<NormalizedTrade> {
    let price = event.price?;
    let size_btc = event.qty?;
    Some(NormalizedTrade {
        venue: event.venue,
        symbol: event.symbol.clone(),
        ts: event.ts_ms,
        price,
        size_btc,
        size_usd: price * size_btc,
        aggressor_side: event.aggressor_side.unwrap_or(AggressorSide::Buy),
        trade_id: event.trade_id.clone(),
    })
}

fn average(total: f64, count: usize) -> f64 {
    if count == 0 {
        0.0
    } else {
        total / count as f64
    }
}

pub fn signal_type_key(signal_type: ManipulationSignalType) -> &'static str {
    match signal_type {
        ManipulationSignalType::SpoofingCandidate => "SpoofingCandidate",
        ManipulationSignalType::LayeringCandidate => "LayeringCandidate",
        ManipulationSignalType::IcebergCandidate => "IcebergCandidate",
    }
}
