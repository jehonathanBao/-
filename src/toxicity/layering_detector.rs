use crate::{
    toxicity::orderbook_delta_evidence::{DeltaDetectorContext, OrderBookDeltaDetector},
    types::{
        market::NormalizedTrade,
        orderbook_delta::{ManipulationSignalType, ManipulationSignalV2, OrderBookDeltaEvent},
    },
};

pub fn detect_layering_candidates(
    deltas: &[OrderBookDeltaEvent],
    trades: &[NormalizedTrade],
    context: DeltaDetectorContext,
) -> Vec<ManipulationSignalV2> {
    OrderBookDeltaDetector::new(context)
        .detect(deltas, trades)
        .into_iter()
        .filter(|signal| signal.signal_type == ManipulationSignalType::LayeringCandidate)
        .collect()
}
