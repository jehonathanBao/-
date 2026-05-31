use std::{collections::VecDeque, sync::Arc};

use parking_lot::RwLock;

use crate::{
    toxicity::toxic_signal_history::{
        build_alert_history_item, build_group_history_item, build_report_history_item,
        build_signal_history_item, build_toxic_signal_history_alert_recent,
        build_toxic_signal_history_recent, build_toxic_signal_history_report_recent,
        build_toxic_signal_history_signal_lookup, build_toxic_signal_history_status,
        ToxicSignalHistoryStatusView,
    },
    types::{
        toxic_signal_alert_preview::ToxicSignalAlertPreviewResponse,
        toxic_signal_group::ToxicSignalGroupRecentResponse,
        toxic_signal_history::{
            ToxicSignalHistoryAlertItem, ToxicSignalHistoryAlertRecentResponse,
            ToxicSignalHistoryGroupItem, ToxicSignalHistoryRecentResponse,
            ToxicSignalHistoryReportItem, ToxicSignalHistoryReportRecentResponse,
            ToxicSignalHistorySignalItem, ToxicSignalHistorySignalLookupResponse,
            ToxicSignalHistoryStatusResponse,
        },
        toxic_signal_inbox::ToxicSignalInboxRecentResponse,
        toxic_signal_report::ToxicSignalReportDailyResponse,
    },
};

const DEFAULT_MAX_SIGNALS: usize = 1000;
const DEFAULT_MAX_GROUPS: usize = 300;
const DEFAULT_MAX_ALERTS: usize = 300;
const DEFAULT_MAX_REPORTS: usize = 30;

#[derive(Debug)]
struct ToxicSignalHistoryStore {
    max_signals: usize,
    max_groups: usize,
    max_alerts: usize,
    max_reports: usize,
    signals: VecDeque<ToxicSignalHistorySignalItem>,
    groups: VecDeque<ToxicSignalHistoryGroupItem>,
    alerts: VecDeque<ToxicSignalHistoryAlertItem>,
    reports: VecDeque<ToxicSignalHistoryReportItem>,
}

impl ToxicSignalHistoryStore {
    fn new(max_signals: usize, max_groups: usize, max_alerts: usize, max_reports: usize) -> Self {
        Self {
            max_signals,
            max_groups,
            max_alerts,
            max_reports,
            signals: VecDeque::new(),
            groups: VecDeque::new(),
            alerts: VecDeque::new(),
            reports: VecDeque::new(),
        }
    }
}

#[derive(Clone, Debug)]
pub struct ToxicSignalHistoryService {
    store: Arc<RwLock<ToxicSignalHistoryStore>>,
}

impl Default for ToxicSignalHistoryService {
    fn default() -> Self {
        Self::new(
            DEFAULT_MAX_SIGNALS,
            DEFAULT_MAX_GROUPS,
            DEFAULT_MAX_ALERTS,
            DEFAULT_MAX_REPORTS,
        )
    }
}

impl ToxicSignalHistoryService {
    pub fn new(
        max_signals: usize,
        max_groups: usize,
        max_alerts: usize,
        max_reports: usize,
    ) -> Self {
        Self {
            store: Arc::new(RwLock::new(ToxicSignalHistoryStore::new(
                max_signals,
                max_groups,
                max_alerts,
                max_reports,
            ))),
        }
    }

    pub fn record_snapshot(
        &self,
        history_recorded_at_ms: u64,
        inbox_recent: &ToxicSignalInboxRecentResponse,
        group_recent: &ToxicSignalGroupRecentResponse,
        alert_preview: &ToxicSignalAlertPreviewResponse,
        daily_report: &ToxicSignalReportDailyResponse,
    ) {
        let mut store = self.store.write();
        let max_signals = store.max_signals;
        let max_groups = store.max_groups;
        let max_alerts = store.max_alerts;
        let max_reports = store.max_reports;

        for item in &inbox_recent.items {
            let history_item = build_signal_history_item(item, history_recorded_at_ms);
            upsert_signal(&mut store.signals, max_signals, history_item);
        }

        for item in &group_recent.groups {
            let history_item = build_group_history_item(item, history_recorded_at_ms);
            upsert_group(&mut store.groups, max_groups, history_item);
        }

        for item in &alert_preview.items {
            let history_item = build_alert_history_item(item, history_recorded_at_ms);
            upsert_alert(&mut store.alerts, max_alerts, history_item);
        }

        upsert_report(
            &mut store.reports,
            max_reports,
            build_report_history_item(daily_report, history_recorded_at_ms),
        );
    }

    pub fn status(&self) -> ToxicSignalHistoryStatusResponse {
        let store = self.store.read();
        build_toxic_signal_history_status(ToxicSignalHistoryStatusView {
            max_signals: store.max_signals,
            max_groups: store.max_groups,
            max_alerts: store.max_alerts,
            max_reports: store.max_reports,
            current_signals: store.signals.len(),
            current_groups: store.groups.len(),
            current_alerts: store.alerts.len(),
            current_reports: store.reports.len(),
        })
    }

    pub fn recent(&self, selected_symbol: &str) -> ToxicSignalHistoryRecentResponse {
        let store = self.store.read();
        build_toxic_signal_history_recent(
            selected_symbol,
            filter_signals(&store.signals, selected_symbol),
            filter_groups(&store.groups, selected_symbol),
        )
    }

    pub fn signal_by_id(&self, signal_id: &str) -> ToxicSignalHistorySignalLookupResponse {
        let store = self.store.read();
        let signal = store
            .signals
            .iter()
            .find(|item| item.signal_id == signal_id)
            .cloned();
        build_toxic_signal_history_signal_lookup(signal)
    }

    pub fn recent_alerts(&self, selected_symbol: &str) -> ToxicSignalHistoryAlertRecentResponse {
        let store = self.store.read();
        build_toxic_signal_history_alert_recent(
            selected_symbol,
            filter_alerts(&store.alerts, selected_symbol),
        )
    }

    pub fn recent_reports(&self, selected_symbol: &str) -> ToxicSignalHistoryReportRecentResponse {
        let store = self.store.read();
        build_toxic_signal_history_report_recent(
            selected_symbol,
            filter_reports(&store.reports, selected_symbol),
        )
    }
}

fn upsert_signal(
    items: &mut VecDeque<ToxicSignalHistorySignalItem>,
    max_items: usize,
    item: ToxicSignalHistorySignalItem,
) {
    items.retain(|existing| existing.signal_id != item.signal_id);
    items.push_front(item);
    while items.len() > max_items {
        let _ = items.pop_back();
    }
}

fn upsert_group(
    items: &mut VecDeque<ToxicSignalHistoryGroupItem>,
    max_items: usize,
    item: ToxicSignalHistoryGroupItem,
) {
    items.retain(|existing| {
        !(existing.group_id == item.group_id && existing.last_seen_at_ms == item.last_seen_at_ms)
    });
    items.push_front(item);
    while items.len() > max_items {
        let _ = items.pop_back();
    }
}

fn upsert_alert(
    items: &mut VecDeque<ToxicSignalHistoryAlertItem>,
    max_items: usize,
    item: ToxicSignalHistoryAlertItem,
) {
    items.retain(|existing| {
        !(existing.signal_id == item.signal_id && existing.preview_status == item.preview_status)
    });
    items.push_front(item);
    while items.len() > max_items {
        let _ = items.pop_back();
    }
}

fn upsert_report(
    items: &mut VecDeque<ToxicSignalHistoryReportItem>,
    max_items: usize,
    item: ToxicSignalHistoryReportItem,
) {
    items.retain(|existing| {
        !(existing.report_type == item.report_type
            && existing.date == item.date
            && existing.symbol == item.symbol)
    });
    items.push_front(item);
    while items.len() > max_items {
        let _ = items.pop_back();
    }
}

fn filter_signals(
    items: &VecDeque<ToxicSignalHistorySignalItem>,
    selected_symbol: &str,
) -> Vec<ToxicSignalHistorySignalItem> {
    items
        .iter()
        .filter(|item| symbol_matches(item.symbol.as_str(), selected_symbol))
        .cloned()
        .collect()
}

fn filter_groups(
    items: &VecDeque<ToxicSignalHistoryGroupItem>,
    selected_symbol: &str,
) -> Vec<ToxicSignalHistoryGroupItem> {
    items
        .iter()
        .filter(|item| symbol_matches(item.symbol.as_str(), selected_symbol))
        .cloned()
        .collect()
}

fn filter_alerts(
    items: &VecDeque<ToxicSignalHistoryAlertItem>,
    selected_symbol: &str,
) -> Vec<ToxicSignalHistoryAlertItem> {
    items
        .iter()
        .filter(|item| symbol_matches(item.symbol.as_str(), selected_symbol))
        .cloned()
        .collect()
}

fn filter_reports(
    items: &VecDeque<ToxicSignalHistoryReportItem>,
    selected_symbol: &str,
) -> Vec<ToxicSignalHistoryReportItem> {
    items
        .iter()
        .filter(|item| symbol_matches(item.symbol.as_str(), selected_symbol))
        .cloned()
        .collect()
}

fn symbol_matches(item_symbol: &str, selected_symbol: &str) -> bool {
    selected_symbol.eq_ignore_ascii_case("ALL") || item_symbol.eq_ignore_ascii_case(selected_symbol)
}
