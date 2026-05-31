use crate::{
    market_data::price_index::PriceSnapshot,
    toxicity::{
        toxic_governance_proposal_service::toxic_governance_proposal_summary,
        toxic_governance_review_pack::{
            build_toxic_governance_review_pack_export, build_toxic_governance_review_pack_status,
            build_toxic_governance_review_pack_summary,
        },
    },
    types::{
        toxic_governance_review_pack::{
            ToxicGovernanceReviewPackExportResponse, ToxicGovernanceReviewPackStatusResponse,
            ToxicGovernanceReviewPackSummaryResponse,
        },
        toxic_signal::ToxicSignalRecentResponse,
    },
};

pub fn toxic_governance_review_pack_summary<F1, F2>(
    requested_symbol: &str,
    fusion_recent: &ToxicSignalRecentResponse,
    snapshot_at_or_before: F1,
    snapshots_since: F2,
) -> ToxicGovernanceReviewPackSummaryResponse
where
    F1: Fn(i64) -> Option<PriceSnapshot>,
    F2: Fn(i64) -> Vec<PriceSnapshot>,
{
    let proposal_summary = toxic_governance_proposal_summary(
        requested_symbol,
        fusion_recent,
        snapshot_at_or_before,
        snapshots_since,
    );
    build_toxic_governance_review_pack_summary(&proposal_summary)
}

pub fn toxic_governance_review_pack_status<F1, F2>(
    requested_symbol: &str,
    fusion_recent: &ToxicSignalRecentResponse,
    snapshot_at_or_before: F1,
    snapshots_since: F2,
) -> ToxicGovernanceReviewPackStatusResponse
where
    F1: Fn(i64) -> Option<PriceSnapshot>,
    F2: Fn(i64) -> Vec<PriceSnapshot>,
{
    let proposal_summary = toxic_governance_proposal_summary(
        requested_symbol,
        fusion_recent,
        snapshot_at_or_before,
        snapshots_since,
    );
    let summary = build_toxic_governance_review_pack_summary(&proposal_summary);
    build_toxic_governance_review_pack_status(&summary)
}

pub fn toxic_governance_review_pack_export<F1, F2>(
    requested_symbol: &str,
    fusion_recent: &ToxicSignalRecentResponse,
    snapshot_at_or_before: F1,
    snapshots_since: F2,
) -> ToxicGovernanceReviewPackExportResponse
where
    F1: Fn(i64) -> Option<PriceSnapshot>,
    F2: Fn(i64) -> Vec<PriceSnapshot>,
{
    let proposal_summary = toxic_governance_proposal_summary(
        requested_symbol,
        fusion_recent,
        snapshot_at_or_before,
        snapshots_since,
    );
    let summary = build_toxic_governance_review_pack_summary(&proposal_summary);
    build_toxic_governance_review_pack_export(&summary)
}
