use crate::{
    market_data::price_index::PriceSnapshot,
    toxicity::{
        toxic_markout::build_toxic_markout_recent,
        toxic_weight_recommendation::{
            build_toxic_weight_recommendation_status, build_toxic_weight_recommendation_summary,
        },
    },
    types::{
        toxic_signal::ToxicSignalRecentResponse,
        toxic_weight_recommendation::{
            ToxicWeightRecommendationStatusResponse, ToxicWeightRecommendationSummaryResponse,
        },
    },
};

pub fn toxic_weight_recommendation_summary<F1, F2>(
    requested_symbol: &str,
    fusion_recent: &ToxicSignalRecentResponse,
    snapshot_at_or_before: F1,
    snapshots_since: F2,
) -> ToxicWeightRecommendationSummaryResponse
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
    build_toxic_weight_recommendation_summary(&recent)
}

pub fn toxic_weight_recommendation_status<F1, F2>(
    requested_symbol: &str,
    fusion_recent: &ToxicSignalRecentResponse,
    snapshot_at_or_before: F1,
    snapshots_since: F2,
) -> ToxicWeightRecommendationStatusResponse
where
    F1: Fn(i64) -> Option<PriceSnapshot>,
    F2: Fn(i64) -> Vec<PriceSnapshot>,
{
    let summary = toxic_weight_recommendation_summary(
        requested_symbol,
        fusion_recent,
        snapshot_at_or_before,
        snapshots_since,
    );
    build_toxic_weight_recommendation_status(&summary)
}
