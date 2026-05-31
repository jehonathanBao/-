use crate::{
    market_data::price_index::PriceSnapshot,
    toxicity::{
        toxic_governance_ledger::load_toxic_governance_decisions,
        toxic_governance_proposal::{
            build_toxic_governance_proposal_export, build_toxic_governance_proposal_status,
            build_toxic_governance_proposal_summary,
        },
        toxic_weight_review_service::toxic_weight_review_summary,
    },
    types::{
        toxic_governance_proposal::{
            ToxicGovernanceProposalExportResponse, ToxicGovernanceProposalStatusResponse,
            ToxicGovernanceProposalSummaryResponse,
        },
        toxic_signal::ToxicSignalRecentResponse,
    },
};

const DEFAULT_GOVERNANCE_LEDGER_PATH: &str = "data/governance/toxic_governance_ledger.jsonl";

pub fn toxic_governance_proposal_summary<F1, F2>(
    requested_symbol: &str,
    fusion_recent: &ToxicSignalRecentResponse,
    snapshot_at_or_before: F1,
    snapshots_since: F2,
) -> ToxicGovernanceProposalSummaryResponse
where
    F1: Fn(i64) -> Option<PriceSnapshot>,
    F2: Fn(i64) -> Vec<PriceSnapshot>,
{
    let review_summary = toxic_weight_review_summary(
        requested_symbol,
        fusion_recent,
        snapshot_at_or_before,
        snapshots_since,
    );
    let path = std::path::Path::new(DEFAULT_GOVERNANCE_LEDGER_PATH);
    let (decisions, mut warnings) = match load_toxic_governance_decisions(path) {
        Ok(entries) => {
            let warnings = if path.exists() {
                Vec::new()
            } else {
                vec!["governance_ledger_file_missing".to_string()]
            };
            (entries, warnings)
        }
        Err(error) => (
            Vec::new(),
            vec![format!("governance_ledger_load_error: {error}")],
        ),
    };
    warnings.extend(review_summary.warnings.clone());
    build_toxic_governance_proposal_summary(&review_summary, &decisions, warnings)
}

pub fn toxic_governance_proposal_status<F1, F2>(
    requested_symbol: &str,
    fusion_recent: &ToxicSignalRecentResponse,
    snapshot_at_or_before: F1,
    snapshots_since: F2,
) -> ToxicGovernanceProposalStatusResponse
where
    F1: Fn(i64) -> Option<PriceSnapshot>,
    F2: Fn(i64) -> Vec<PriceSnapshot>,
{
    let summary = toxic_governance_proposal_summary(
        requested_symbol,
        fusion_recent,
        snapshot_at_or_before,
        snapshots_since,
    );
    build_toxic_governance_proposal_status(&summary)
}

pub fn toxic_governance_proposal_export<F1, F2>(
    requested_symbol: &str,
    fusion_recent: &ToxicSignalRecentResponse,
    snapshot_at_or_before: F1,
    snapshots_since: F2,
) -> ToxicGovernanceProposalExportResponse
where
    F1: Fn(i64) -> Option<PriceSnapshot>,
    F2: Fn(i64) -> Vec<PriceSnapshot>,
{
    let summary = toxic_governance_proposal_summary(
        requested_symbol,
        fusion_recent,
        snapshot_at_or_before,
        snapshots_since,
    );
    build_toxic_governance_proposal_export(&summary)
}
