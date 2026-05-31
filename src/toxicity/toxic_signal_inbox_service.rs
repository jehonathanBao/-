use crate::{
    toxicity::toxic_signal_inbox::{
        build_toxic_signal_inbox_detail, build_toxic_signal_inbox_recent,
        build_toxic_signal_inbox_status,
    },
    types::{
        toxic_governance_ledger::ToxicGovernanceLedgerSummaryResponse,
        toxic_markout::ToxicMarkoutRecentResponse,
        toxic_quality_scorecard::ToxicQualityScorecardSummaryResponse,
        toxic_replay::ToxicReplayRecentResponse,
        toxic_signal::ToxicSignalRecentResponse,
        toxic_signal_inbox::{
            ToxicSignalInboxDetailResponse, ToxicSignalInboxRecentResponse,
            ToxicSignalInboxStatusResponse,
        },
        toxic_weight_recommendation::ToxicWeightRecommendationSummaryResponse,
    },
};

pub fn toxic_signal_inbox_recent(
    requested_symbol: &str,
    fusion_recent: &ToxicSignalRecentResponse,
    replay_recent: &ToxicReplayRecentResponse,
    markout_recent: &ToxicMarkoutRecentResponse,
    quality_summary: &ToxicQualityScorecardSummaryResponse,
    recommendation_summary: &ToxicWeightRecommendationSummaryResponse,
    governance_summary: &ToxicGovernanceLedgerSummaryResponse,
) -> ToxicSignalInboxRecentResponse {
    build_toxic_signal_inbox_recent(
        requested_symbol,
        fusion_recent,
        replay_recent,
        markout_recent,
        quality_summary,
        recommendation_summary,
        governance_summary,
    )
}

pub fn toxic_signal_inbox_status(
    recent: &ToxicSignalInboxRecentResponse,
) -> ToxicSignalInboxStatusResponse {
    build_toxic_signal_inbox_status(recent)
}

pub fn toxic_signal_inbox_by_signal_id(
    requested_symbol: &str,
    signal_id: &str,
    recent: &ToxicSignalInboxRecentResponse,
) -> ToxicSignalInboxDetailResponse {
    build_toxic_signal_inbox_detail(requested_symbol, signal_id, recent)
}
