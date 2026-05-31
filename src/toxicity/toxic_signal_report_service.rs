use crate::{
    toxicity::toxic_signal_report::{
        build_toxic_signal_daily_report, build_toxic_signal_report_status,
        build_toxic_signal_rolling_report,
    },
    types::{
        toxic_quality_scorecard::ToxicQualityScorecardSummaryResponse,
        toxic_signal_group::ToxicSignalGroupRecentResponse,
        toxic_signal_history::{ToxicSignalHistoryAlertItem, ToxicSignalHistorySignalItem},
        toxic_signal_inbox::ToxicSignalInboxRecentResponse,
        toxic_signal_report::{
            ToxicSignalReportDailyResponse, ToxicSignalReportRollingResponse,
            ToxicSignalReportStatusResponse,
        },
        toxic_weight_recommendation::ToxicWeightRecommendationSummaryResponse,
    },
};

pub fn toxic_signal_report_status(
    requested_symbol: &str,
    report_date: &str,
    inbox_recent: &ToxicSignalInboxRecentResponse,
    group_recent: &ToxicSignalGroupRecentResponse,
) -> ToxicSignalReportStatusResponse {
    build_toxic_signal_report_status(requested_symbol, report_date, inbox_recent, group_recent)
}

pub fn toxic_signal_daily_report(
    requested_symbol: &str,
    report_date: &str,
    inbox_recent: &ToxicSignalInboxRecentResponse,
    group_recent: &ToxicSignalGroupRecentResponse,
    quality_summary: &ToxicQualityScorecardSummaryResponse,
    recommendation_summary: &ToxicWeightRecommendationSummaryResponse,
) -> ToxicSignalReportDailyResponse {
    build_toxic_signal_daily_report(
        requested_symbol,
        report_date,
        inbox_recent,
        group_recent,
        quality_summary,
        recommendation_summary,
    )
}

pub fn toxic_signal_rolling_report(
    requested_symbol: &str,
    window: &str,
    signal_history: &[ToxicSignalHistorySignalItem],
    alert_history: &[ToxicSignalHistoryAlertItem],
) -> ToxicSignalReportRollingResponse {
    build_toxic_signal_rolling_report(requested_symbol, window, signal_history, alert_history)
}
