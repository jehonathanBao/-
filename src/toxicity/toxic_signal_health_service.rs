use crate::{
    toxicity::toxic_signal_health::{
        build_toxic_signal_health_status, build_toxic_signal_health_summary,
    },
    types::{
        toxic_signal_alert_preview::ToxicSignalAlertPreviewResponse,
        toxic_signal_group::ToxicSignalGroupRecentResponse,
        toxic_signal_health::{ToxicSignalHealthStatusResponse, ToxicSignalHealthSummaryResponse},
        toxic_signal_history::{
            ToxicSignalHistoryRecentResponse, ToxicSignalHistoryStatusResponse,
        },
        toxic_signal_inbox::ToxicSignalInboxRecentResponse,
        toxic_signal_report::ToxicSignalReportDailyResponse,
    },
};

pub fn toxic_signal_health_summary(
    requested_symbol: &str,
    inbox_recent: &ToxicSignalInboxRecentResponse,
    group_recent: &ToxicSignalGroupRecentResponse,
    daily_report: &ToxicSignalReportDailyResponse,
    alert_preview: &ToxicSignalAlertPreviewResponse,
    history_status: &ToxicSignalHistoryStatusResponse,
    history_recent: &ToxicSignalHistoryRecentResponse,
) -> ToxicSignalHealthSummaryResponse {
    build_toxic_signal_health_summary(
        requested_symbol,
        inbox_recent,
        group_recent,
        daily_report,
        alert_preview,
        history_status,
        history_recent,
    )
}

pub fn toxic_signal_health_status(
    summary: &ToxicSignalHealthSummaryResponse,
) -> ToxicSignalHealthStatusResponse {
    build_toxic_signal_health_status(summary)
}
