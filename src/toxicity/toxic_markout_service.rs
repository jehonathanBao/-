use crate::{
    market_data::price_index::PriceSnapshot,
    toxicity::toxic_markout::{
        build_toxic_markout_by_signal_id, build_toxic_markout_recent, build_toxic_markout_status,
    },
    types::{
        toxic_markout::{
            ToxicMarkoutDetailResponse, ToxicMarkoutRecentResponse, ToxicMarkoutStatusResponse,
        },
        toxic_signal::ToxicSignalRecentResponse,
    },
};

pub fn toxic_markout_recent<F1, F2>(
    requested_symbol: &str,
    fusion_recent: &ToxicSignalRecentResponse,
    snapshot_at_or_before: F1,
    snapshots_since: F2,
) -> ToxicMarkoutRecentResponse
where
    F1: Fn(i64) -> Option<PriceSnapshot>,
    F2: Fn(i64) -> Vec<PriceSnapshot>,
{
    build_toxic_markout_recent(
        requested_symbol,
        fusion_recent,
        snapshot_at_or_before,
        snapshots_since,
    )
}

pub fn toxic_markout_status<F1, F2>(
    requested_symbol: &str,
    fusion_recent: &ToxicSignalRecentResponse,
    snapshot_at_or_before: F1,
    snapshots_since: F2,
) -> ToxicMarkoutStatusResponse
where
    F1: Fn(i64) -> Option<PriceSnapshot>,
    F2: Fn(i64) -> Vec<PriceSnapshot>,
{
    build_toxic_markout_status(
        requested_symbol,
        fusion_recent,
        snapshot_at_or_before,
        snapshots_since,
    )
}

pub fn toxic_markout_by_signal_id<F1, F2>(
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
    build_toxic_markout_by_signal_id(
        requested_symbol,
        signal_id,
        fusion_recent,
        snapshot_at_or_before,
        snapshots_since,
    )
}
