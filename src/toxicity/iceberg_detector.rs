use crate::{
    toxicity::orderbook_delta_evidence::{DeltaDetectorContext, OrderBookDeltaDetector},
    types::{
        market::NormalizedTrade,
        orderbook_delta::{ManipulationSignalType, ManipulationSignalV2, OrderBookDeltaEvent},
    },
};

pub fn detect_iceberg_candidates(
    deltas: &[OrderBookDeltaEvent],
    trades: &[NormalizedTrade],
    context: DeltaDetectorContext,
) -> Vec<ManipulationSignalV2> {
    OrderBookDeltaDetector::new(context)
        .detect(deltas, trades)
        .into_iter()
        .filter(|signal| signal.signal_type == ManipulationSignalType::IcebergCandidate)
        .collect()
}
