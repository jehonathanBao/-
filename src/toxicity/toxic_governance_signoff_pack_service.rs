use crate::{
    market_data::price_index::PriceSnapshot,
    toxicity::{
        toxic_governance_review_pack_service::toxic_governance_review_pack_summary,
        toxic_governance_signoff_pack::{
            build_toxic_governance_signoff_pack_export, build_toxic_governance_signoff_pack_status,
            build_toxic_governance_signoff_pack_summary,
        },
    },
    types::{
        toxic_governance_signoff_pack::{
            ToxicGovernanceSignoffPackExportResponse, ToxicGovernanceSignoffPackStatusResponse,
            ToxicGovernanceSignoffPackSummaryResponse,
        },
        toxic_signal::ToxicSignalRecentResponse,
    },
};

pub fn toxic_governance_signoff_pack_summary<F1, F2>(
    requested_symbol: &str,
    fusion_recent: &ToxicSignalRecentResponse,
    snapshot_at_or_before: F1,
    snapshots_since: F2,
) -> ToxicGovernanceSignoffPackSummaryResponse
where
    F1: Fn(i64) -> Option<PriceSnapshot>,
    F2: Fn(i64) -> Vec<PriceSnapshot>,
{
    let review_pack = toxic_governance_review_pack_summary(
        requested_symbol,
        fusion_recent,
        snapshot_at_or_before,
        snapshots_since,
    );
    build_toxic_governance_signoff_pack_summary(&review_pack)
}

pub fn toxic_governance_signoff_pack_status<F1, F2>(
    requested_symbol: &str,
    fusion_recent: &ToxicSignalRecentResponse,
    snapshot_at_or_before: F1,
    snapshots_since: F2,
) -> ToxicGovernanceSignoffPackStatusResponse
where
    F1: Fn(i64) -> Option<PriceSnapshot>,
    F2: Fn(i64) -> Vec<PriceSnapshot>,
{
    let summary = toxic_governance_signoff_pack_summary(
        requested_symbol,
        fusion_recent,
        snapshot_at_or_before,
        snapshots_since,
    );
    build_toxic_governance_signoff_pack_status(&summary)
}

pub fn toxic_governance_signoff_pack_export<F1, F2>(
    requested_symbol: &str,
    fusion_recent: &ToxicSignalRecentResponse,
    snapshot_at_or_before: F1,
    snapshots_since: F2,
) -> ToxicGovernanceSignoffPackExportResponse
where
    F1: Fn(i64) -> Option<PriceSnapshot>,
    F2: Fn(i64) -> Vec<PriceSnapshot>,
{
    let summary = toxic_governance_signoff_pack_summary(
        requested_symbol,
        fusion_recent,
        snapshot_at_or_before,
        snapshots_since,
    );
    build_toxic_governance_signoff_pack_export(&summary)
}
