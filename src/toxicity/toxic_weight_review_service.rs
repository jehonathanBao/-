use crate::{
    market_data::price_index::PriceSnapshot,
    toxicity::{
        toxic_weight_recommendation_service::toxic_weight_recommendation_summary,
        toxic_weight_review::{
            build_toxic_weight_review_export, build_toxic_weight_review_status,
            build_toxic_weight_review_summary,
        },
    },
    types::{
        toxic_signal::ToxicSignalRecentResponse,
        toxic_weight_review::{
            ToxicWeightReviewExportResponse, ToxicWeightReviewStatusResponse,
            ToxicWeightReviewSummaryResponse,
        },
    },
};

pub fn toxic_weight_review_summary<F1, F2>(
    requested_symbol: &str,
    fusion_recent: &ToxicSignalRecentResponse,
    snapshot_at_or_before: F1,
    snapshots_since: F2,
) -> ToxicWeightReviewSummaryResponse
where
    F1: Fn(i64) -> Option<PriceSnapshot>,
    F2: Fn(i64) -> Vec<PriceSnapshot>,
{
    let recommendation_summary = toxic_weight_recommendation_summary(
        requested_symbol,
        fusion_recent,
        snapshot_at_or_before,
        snapshots_since,
    );
    build_toxic_weight_review_summary(&recommendation_summary)
}

pub fn toxic_weight_review_status<F1, F2>(
    requested_symbol: &str,
    fusion_recent: &ToxicSignalRecentResponse,
    snapshot_at_or_before: F1,
    snapshots_since: F2,
) -> ToxicWeightReviewStatusResponse
where
    F1: Fn(i64) -> Option<PriceSnapshot>,
    F2: Fn(i64) -> Vec<PriceSnapshot>,
{
    let summary = toxic_weight_review_summary(
        requested_symbol,
        fusion_recent,
        snapshot_at_or_before,
        snapshots_since,
    );
    build_toxic_weight_review_status(&summary)
}

pub fn toxic_weight_review_export<F1, F2>(
    requested_symbol: &str,
    fusion_recent: &ToxicSignalRecentResponse,
    snapshot_at_or_before: F1,
    snapshots_since: F2,
) -> ToxicWeightReviewExportResponse
where
    F1: Fn(i64) -> Option<PriceSnapshot>,
    F2: Fn(i64) -> Vec<PriceSnapshot>,
{
    let summary = toxic_weight_review_summary(
        requested_symbol,
        fusion_recent,
        snapshot_at_or_before,
        snapshots_since,
    );
    build_toxic_weight_review_export(&summary)
}
