use crate::types::{
    toxic_signal_alert_preview::ToxicSignalAlertPreviewResponse,
    toxic_signal_group::ToxicSignalGroupRecentResponse,
    toxic_signal_health::{
        ToxicSignalHealthIssue, ToxicSignalHealthStatusResponse, ToxicSignalHealthSummary,
        ToxicSignalHealthSummaryResponse,
    },
    toxic_signal_history::{ToxicSignalHistoryRecentResponse, ToxicSignalHistoryStatusResponse},
    toxic_signal_inbox::{ToxicSignalInboxItem, ToxicSignalInboxRecentResponse},
    toxic_signal_report::ToxicSignalReportDailyResponse,
};

const HEALTH_MODE: &str = "diagnostic_only";

pub fn build_toxic_signal_health_summary(
    requested_symbol: &str,
    inbox_recent: &ToxicSignalInboxRecentResponse,
    group_recent: &ToxicSignalGroupRecentResponse,
    daily_report: &ToxicSignalReportDailyResponse,
    alert_preview: &ToxicSignalAlertPreviewResponse,
    history_status: &ToxicSignalHistoryStatusResponse,
    history_recent: &ToxicSignalHistoryRecentResponse,
) -> ToxicSignalHealthSummaryResponse {
    let summary = ToxicSignalHealthSummary {
        inbox_available: !inbox_recent.items.is_empty(),
        groups_available: !group_recent.groups.is_empty(),
        detail_available: !inbox_recent.items.is_empty(),
        daily_report_available: daily_report.summary.total_signals > 0,
        alert_preview_available: !alert_preview.items.is_empty(),
        history_available: !history_recent.items.is_empty()
            || !history_recent.group_items.is_empty()
            || history_status.current_signals > 0,
        total_signals: inbox_recent.items.len(),
        signals_with_markout: inbox_recent
            .items
            .iter()
            .filter(|item| item.markout.available)
            .count(),
        signals_missing_markout: inbox_recent
            .items
            .iter()
            .filter(|item| !item.markout.available)
            .count(),
        signals_with_quality: inbox_recent
            .items
            .iter()
            .filter(|item| item.quality.available)
            .count(),
        signals_missing_quality: inbox_recent
            .items
            .iter()
            .filter(|item| !item.quality.available)
            .count(),
        signals_with_recommendation: inbox_recent
            .items
            .iter()
            .filter(|item| item.recommendation.available)
            .count(),
        signals_missing_recommendation: inbox_recent
            .items
            .iter()
            .filter(|item| !item.recommendation.available)
            .count(),
        signals_with_governance: inbox_recent
            .items
            .iter()
            .filter(|item| item.governance.ledger_available)
            .count(),
        signals_missing_governance: inbox_recent
            .items
            .iter()
            .filter(|item| !item.governance.ledger_available)
            .count(),
        not_enough_data_count: inbox_recent
            .items
            .iter()
            .filter(|item| is_not_enough_data(item))
            .count(),
    };
    let issues = build_issues(requested_symbol, &summary);
    let health_bucket = health_bucket(&summary);

    ToxicSignalHealthSummaryResponse {
        read_only: true,
        runtime_modified: false,
        analysis_only: true,
        execution_enabled: false,
        health_mode: HEALTH_MODE.to_string(),
        repair_enabled: false,
        backfill_enabled: false,
        runtime_mutation_enabled: false,
        selected_symbol: requested_symbol.to_string(),
        summary,
        health_bucket,
        issues,
        operator_notes: vec![
            "Signal health only. No repair, backfill, runtime action, notification, or trading action is performed.".to_string(),
            "Use this view to spot missing markout, quality, recommendation, governance, or history coverage before trusting a signal stream.".to_string(),
        ],
    }
}

pub fn build_toxic_signal_health_status(
    summary: &ToxicSignalHealthSummaryResponse,
) -> ToxicSignalHealthStatusResponse {
    ToxicSignalHealthStatusResponse {
        read_only: true,
        runtime_modified: false,
        analysis_only: true,
        execution_enabled: false,
        health_mode: HEALTH_MODE.to_string(),
        repair_enabled: false,
        backfill_enabled: false,
        runtime_mutation_enabled: false,
        enabled: true,
        status: if summary.summary.total_signals == 0 {
            "signal_health_unavailable".to_string()
        } else {
            "signal_health_ready".to_string()
        },
        selected_symbol: summary.selected_symbol.clone(),
        health_bucket: summary.health_bucket.clone(),
        total_signals: summary.summary.total_signals,
        issue_count: summary.issues.len(),
        safety_boundary: vec![
            "readOnly=true".to_string(),
            "runtimeModified=false".to_string(),
            "analysisOnly=true".to_string(),
            "executionEnabled=false".to_string(),
            "healthMode=diagnostic_only".to_string(),
            "repairEnabled=false".to_string(),
            "backfillEnabled=false".to_string(),
            "runtimeMutationEnabled=false".to_string(),
            "No order placement".to_string(),
            "No cancel/amend".to_string(),
            "No wallet/signing".to_string(),
            "No transaction construction".to_string(),
            "No live trading".to_string(),
            "No notification sending".to_string(),
            "No webhook".to_string(),
            "No Telegram".to_string(),
            "No DB write".to_string(),
            "No file write".to_string(),
        ],
    }
}

fn build_issues(
    requested_symbol: &str,
    summary: &ToxicSignalHealthSummary,
) -> Vec<ToxicSignalHealthIssue> {
    let mut issues = Vec::new();

    if summary.total_signals == 0 {
        issues.push(ToxicSignalHealthIssue {
            kind: if requested_symbol.eq_ignore_ascii_case("ALL") {
                "empty_inbox".to_string()
            } else {
                "symbol_not_found".to_string()
            },
            severity: "warning".to_string(),
            count: 1,
            operator_note: if requested_symbol.eq_ignore_ascii_case("ALL") {
                "No recent signals are available for the current read-only health view.".to_string()
            } else {
                format!(
                    "No recent signals matched symbol {} in the current read-only health view.",
                    requested_symbol
                )
            },
        });
        return issues;
    }

    if !summary.groups_available {
        issues.push(ToxicSignalHealthIssue {
            kind: "missing_groups".to_string(),
            severity: "warning".to_string(),
            count: 1,
            operator_note: "Grouped signal coverage is unavailable for the current selection."
                .to_string(),
        });
    }

    if !summary.history_available {
        issues.push(ToxicSignalHealthIssue {
            kind: "missing_history".to_string(),
            severity: "warning".to_string(),
            count: 1,
            operator_note: "Recent in-memory history is unavailable for the current selection."
                .to_string(),
        });
    }

    if summary.signals_missing_markout > 0 {
        issues.push(ToxicSignalHealthIssue {
            kind: "missing_markout".to_string(),
            severity: "warning".to_string(),
            count: summary.signals_missing_markout,
            operator_note: "Some signals do not have markout coverage yet.".to_string(),
        });
    }

    if summary.signals_missing_quality > 0 {
        issues.push(ToxicSignalHealthIssue {
            kind: "missing_quality".to_string(),
            severity: "warning".to_string(),
            count: summary.signals_missing_quality,
            operator_note: "Some signals do not have a quality summary yet.".to_string(),
        });
    }

    if summary.signals_missing_recommendation > 0 {
        issues.push(ToxicSignalHealthIssue {
            kind: "missing_recommendation".to_string(),
            severity: "warning".to_string(),
            count: summary.signals_missing_recommendation,
            operator_note: "Some signals do not have a recommendation summary yet.".to_string(),
        });
    }

    if summary.signals_missing_governance > 0 {
        issues.push(ToxicSignalHealthIssue {
            kind: "missing_governance".to_string(),
            severity: "info".to_string(),
            count: summary.signals_missing_governance,
            operator_note:
                "Governance ledger entries are optional and may be unavailable for newer signals."
                    .to_string(),
        });
    }

    if !summary.daily_report_available {
        issues.push(ToxicSignalHealthIssue {
            kind: "daily_report_unavailable".to_string(),
            severity: "warning".to_string(),
            count: 1,
            operator_note: "Daily report coverage is unavailable because the inbox is empty."
                .to_string(),
        });
    }

    if !summary.alert_preview_available {
        issues.push(ToxicSignalHealthIssue {
            kind: "alert_preview_unavailable".to_string(),
            severity: "warning".to_string(),
            count: 1,
            operator_note: "Alert preview coverage is unavailable because there are no current signals to review.".to_string(),
        });
    }

    let not_enough_ratio = summary.not_enough_data_count as f64 / summary.total_signals as f64;
    if not_enough_ratio >= 0.40 {
        issues.push(ToxicSignalHealthIssue {
            kind: "high_not_enough_data_ratio".to_string(),
            severity: "warning".to_string(),
            count: summary.not_enough_data_count,
            operator_note: "A large share of signals still falls into not_enough_data coverage."
                .to_string(),
        });
    }

    issues
}

fn health_bucket(summary: &ToxicSignalHealthSummary) -> String {
    if summary.total_signals == 0 || !summary.inbox_available {
        return "unavailable".to_string();
    }

    let unavailable_count = [
        summary.groups_available,
        summary.detail_available,
        summary.daily_report_available,
        summary.alert_preview_available,
        summary.history_available,
    ]
    .into_iter()
    .filter(|available| !available)
    .count();
    let not_enough_ratio = summary.not_enough_data_count as f64 / summary.total_signals as f64;
    let markout_ratio = summary.signals_with_markout as f64 / summary.total_signals as f64;
    let quality_ratio = summary.signals_with_quality as f64 / summary.total_signals as f64;
    let recommendation_ratio =
        summary.signals_with_recommendation as f64 / summary.total_signals as f64;

    if unavailable_count >= 2 {
        "degraded".to_string()
    } else if not_enough_ratio >= 0.50 || markout_ratio < 0.50 {
        "thin_data".to_string()
    } else if unavailable_count == 0
        && markout_ratio >= 0.85
        && quality_ratio >= 0.85
        && recommendation_ratio >= 0.80
    {
        "excellent".to_string()
    } else {
        "good".to_string()
    }
}

fn is_not_enough_data(item: &ToxicSignalInboxItem) -> bool {
    !item.quality.available
        || !item.recommendation.available
        || item
            .quality
            .quality_bucket
            .eq_ignore_ascii_case("not_enough_data")
        || [
            item.markout.one_minute.as_str(),
            item.markout.five_minute.as_str(),
            item.markout.fifteen_minute.as_str(),
            item.markout.one_hour.as_str(),
        ]
        .iter()
        .any(|outcome| outcome.eq_ignore_ascii_case("not_enough_data"))
}
