use crate::types::{
    toxic_signal_alert_preview::ToxicSignalAlertPreviewItem,
    toxic_signal_group::{ToxicSignalGroup, ToxicSignalGroupOperatorAction},
    toxic_signal_history::{
        ToxicSignalHistoryAlertItem, ToxicSignalHistoryAlertRecentResponse,
        ToxicSignalHistoryGroupItem, ToxicSignalHistoryRecentResponse,
        ToxicSignalHistoryReportItem, ToxicSignalHistoryReportRecentResponse,
        ToxicSignalHistorySignalItem, ToxicSignalHistorySignalLookupResponse,
        ToxicSignalHistoryStatusResponse,
    },
    toxic_signal_inbox::{ToxicSignalInboxItem, ToxicSignalInboxOperatorAction},
    toxic_signal_report::ToxicSignalReportDailyResponse,
};

pub const TOXIC_SIGNAL_HISTORY_RETENTION_MODE: &str = "in_memory_bounded";

#[derive(Debug, Clone)]
pub struct ToxicSignalHistoryStatusView {
    pub max_signals: usize,
    pub max_groups: usize,
    pub max_alerts: usize,
    pub max_reports: usize,
    pub current_signals: usize,
    pub current_groups: usize,
    pub current_alerts: usize,
    pub current_reports: usize,
}

pub fn build_signal_history_item(
    item: &ToxicSignalInboxItem,
    history_recorded_at_ms: u64,
) -> ToxicSignalHistorySignalItem {
    ToxicSignalHistorySignalItem {
        signal_id: item.signal_id.clone(),
        symbol: item.symbol.clone(),
        signal_kind: item.signal_kind.clone(),
        direction_bias: item.direction_bias.clone(),
        severity: item.severity.clone(),
        confidence: item.confidence,
        created_at_ms: item.created_at_ms,
        markout_one_minute: item.markout.one_minute.clone(),
        markout_five_minute: item.markout.five_minute.clone(),
        markout_fifteen_minute: item.markout.fifteen_minute.clone(),
        markout_one_hour: item.markout.one_hour.clone(),
        quality_bucket: item.quality.quality_bucket.clone(),
        recommendation_action: item.recommendation.action.clone(),
        no_trade_only: item.recommendation.no_trade_only,
        source: "signal_inbox".to_string(),
        history_recorded_at_ms,
        operator_action: inbox_operator_action_label(item.operator_action).to_string(),
    }
}

pub fn build_group_history_item(
    item: &ToxicSignalGroup,
    history_recorded_at_ms: u64,
) -> ToxicSignalHistoryGroupItem {
    ToxicSignalHistoryGroupItem {
        group_id: item.group_id.clone(),
        symbol: item.symbol.clone(),
        signal_kind: item.signal_kind.clone(),
        direction_bias: item.direction_bias.clone(),
        count: item.count,
        first_seen_at_ms: item.first_seen_at_ms,
        last_seen_at_ms: item.last_seen_at_ms,
        max_severity: item.max_severity.clone(),
        avg_confidence: item.avg_confidence,
        representative_signal_id: item.representative_signal_id.clone(),
        member_signal_ids: item.member_signal_ids.clone(),
        source: "signal_groups".to_string(),
        history_recorded_at_ms,
        operator_action: group_operator_action_label(item.operator_action).to_string(),
    }
}

pub fn build_alert_history_item(
    item: &ToxicSignalAlertPreviewItem,
    history_recorded_at_ms: u64,
) -> ToxicSignalHistoryAlertItem {
    ToxicSignalHistoryAlertItem {
        signal_id: item.signal_id.clone(),
        symbol: item.symbol.clone(),
        signal_kind: item.signal_kind.clone(),
        preview_status: item.preview_status.clone(),
        would_notify_if_enabled: item.would_notify_if_enabled,
        no_trade_only: item.no_trade_only,
        markout_readiness: item.markout_readiness.clone(),
        source: "signal_alert_preview".to_string(),
        history_recorded_at_ms,
        notification_sent: false,
        execution_triggered: false,
    }
}

pub fn build_report_history_item(
    report: &ToxicSignalReportDailyResponse,
    history_recorded_at_ms: u64,
) -> ToxicSignalHistoryReportItem {
    ToxicSignalHistoryReportItem {
        report_type: report.report_type.clone(),
        date: report.date.clone(),
        symbol: report.filter.symbol.clone(),
        total_signals: report.summary.total_signals,
        grouped_signals: report.summary.grouped_signals,
        high_severity_signals: report.summary.high_severity_signals,
        no_trade_only_candidates: report.summary.no_trade_only_candidates,
        downgrade_candidates: report.summary.downgrade_candidates,
        not_enough_data_signals: report.summary.not_enough_data_signals,
        source: "signal_report".to_string(),
        history_recorded_at_ms,
    }
}

pub fn build_toxic_signal_history_status(
    view: ToxicSignalHistoryStatusView,
) -> ToxicSignalHistoryStatusResponse {
    ToxicSignalHistoryStatusResponse {
        read_only: true,
        runtime_modified: false,
        analysis_only: true,
        execution_enabled: false,
        retention_mode: TOXIC_SIGNAL_HISTORY_RETENTION_MODE.to_string(),
        durable_storage_enabled: false,
        database_write_enabled: false,
        max_signals: view.max_signals,
        max_groups: view.max_groups,
        max_alerts: view.max_alerts,
        max_reports: view.max_reports,
        current_signals: view.current_signals,
        current_groups: view.current_groups,
        current_alerts: view.current_alerts,
        current_reports: view.current_reports,
        safety_boundary: vec![
            "readOnly=true".to_string(),
            "runtimeModified=false".to_string(),
            "analysisOnly=true".to_string(),
            "executionEnabled=false".to_string(),
            "retentionMode=in_memory_bounded".to_string(),
            "durableStorageEnabled=false".to_string(),
            "databaseWriteEnabled=false".to_string(),
            "No database write".to_string(),
            "No file write".to_string(),
            "No order placement".to_string(),
            "No wallet/signing".to_string(),
            "No live trading".to_string(),
        ],
    }
}

pub fn build_toxic_signal_history_recent(
    selected_symbol: &str,
    items: Vec<ToxicSignalHistorySignalItem>,
    group_items: Vec<ToxicSignalHistoryGroupItem>,
) -> ToxicSignalHistoryRecentResponse {
    ToxicSignalHistoryRecentResponse {
        read_only: true,
        runtime_modified: false,
        analysis_only: true,
        execution_enabled: false,
        retention_mode: TOXIC_SIGNAL_HISTORY_RETENTION_MODE.to_string(),
        durable_storage_enabled: false,
        database_write_enabled: false,
        selected_symbol: selected_symbol.to_string(),
        items,
        group_items,
        operator_notes: vec![
            "Signal history is a read-only in-memory bounded snapshot.".to_string(),
            "History is not durable storage and may be lost after restart.".to_string(),
            "No database write, no file write, and no live trading path are enabled.".to_string(),
        ],
    }
}

pub fn build_toxic_signal_history_signal_lookup(
    signal: Option<ToxicSignalHistorySignalItem>,
) -> ToxicSignalHistorySignalLookupResponse {
    ToxicSignalHistorySignalLookupResponse {
        read_only: true,
        runtime_modified: false,
        analysis_only: true,
        execution_enabled: false,
        found: signal.is_some(),
        signal,
        source: "signal_history".to_string(),
        retention_mode: TOXIC_SIGNAL_HISTORY_RETENTION_MODE.to_string(),
    }
}

pub fn build_toxic_signal_history_alert_recent(
    selected_symbol: &str,
    items: Vec<ToxicSignalHistoryAlertItem>,
) -> ToxicSignalHistoryAlertRecentResponse {
    ToxicSignalHistoryAlertRecentResponse {
        read_only: true,
        runtime_modified: false,
        analysis_only: true,
        execution_enabled: false,
        retention_mode: TOXIC_SIGNAL_HISTORY_RETENTION_MODE.to_string(),
        durable_storage_enabled: false,
        database_write_enabled: false,
        selected_symbol: selected_symbol.to_string(),
        items,
    }
}

pub fn build_toxic_signal_history_report_recent(
    selected_symbol: &str,
    items: Vec<ToxicSignalHistoryReportItem>,
) -> ToxicSignalHistoryReportRecentResponse {
    ToxicSignalHistoryReportRecentResponse {
        read_only: true,
        runtime_modified: false,
        analysis_only: true,
        execution_enabled: false,
        retention_mode: TOXIC_SIGNAL_HISTORY_RETENTION_MODE.to_string(),
        durable_storage_enabled: false,
        database_write_enabled: false,
        selected_symbol: selected_symbol.to_string(),
        items,
    }
}

fn inbox_operator_action_label(action: ToxicSignalInboxOperatorAction) -> &'static str {
    match action {
        ToxicSignalInboxOperatorAction::WatchSignalOnly => "watch_signal_only",
        ToxicSignalInboxOperatorAction::ReviewEvidence => "review_evidence",
        ToxicSignalInboxOperatorAction::ReviewMarkout => "review_markout",
        ToxicSignalInboxOperatorAction::ReviewQuality => "review_quality",
        ToxicSignalInboxOperatorAction::NoTradeWarning => "no_trade_warning",
        ToxicSignalInboxOperatorAction::NeedsMoreData => "needs_more_data",
    }
}

fn group_operator_action_label(action: ToxicSignalGroupOperatorAction) -> &'static str {
    match action {
        ToxicSignalGroupOperatorAction::ReviewGroupedSignal => "review_grouped_signal",
        ToxicSignalGroupOperatorAction::WatchGroupOnly => "watch_group_only",
        ToxicSignalGroupOperatorAction::NeedsMoreData => "needs_more_data",
        ToxicSignalGroupOperatorAction::NoTradeWarningGroup => "no_trade_warning_group",
    }
}
