use crate::{
    market_data::price_index::PriceSnapshot,
    toxicity::{
        toxic_markout::build_toxic_markout_recent,
        toxic_quality_scorecard::{
            build_toxic_quality_scorecard_status, build_toxic_quality_scorecard_summary,
        },
    },
    types::{
        toxic_quality_scorecard::{
            ToxicQualityScorecardStatusResponse, ToxicQualityScorecardSummaryResponse,
        },
        toxic_signal::ToxicSignalRecentResponse,
    },
};

pub fn toxic_quality_scorecard_summary<F1, F2>(
    requested_symbol: &str,
    fusion_recent: &ToxicSignalRecentResponse,
    snapshot_at_or_before: F1,
    snapshots_since: F2,
) -> ToxicQualityScorecardSummaryResponse
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
    build_toxic_quality_scorecard_summary(&recent)
}

pub fn toxic_quality_scorecard_status<F1, F2>(
    requested_symbol: &str,
    fusion_recent: &ToxicSignalRecentResponse,
    snapshot_at_or_before: F1,
    snapshots_since: F2,
) -> ToxicQualityScorecardStatusResponse
where
    F1: Fn(i64) -> Option<PriceSnapshot>,
    F2: Fn(i64) -> Vec<PriceSnapshot>,
{
    let summary = toxic_quality_scorecard_summary(
        requested_symbol,
        fusion_recent,
        snapshot_at_or_before,
        snapshots_since,
    );
    build_toxic_quality_scorecard_status(&summary)
}
