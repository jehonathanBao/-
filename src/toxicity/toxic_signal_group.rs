use std::collections::HashMap;

use crate::types::{
    toxic_signal_group::{
        ToxicSignalGroup, ToxicSignalGroupDetailResponse, ToxicSignalGroupOperatorAction,
        ToxicSignalGroupRecentResponse, ToxicSignalGroupStatusResponse,
    },
    toxic_signal_inbox::{
        ToxicSignalInboxItem, ToxicSignalInboxOperatorAction, ToxicSignalInboxRecentResponse,
    },
};

const DEFAULT_COOLDOWN_WINDOW_MS: u64 = 300_000;

pub fn build_toxic_signal_group_recent(
    requested_symbol: &str,
    inbox_recent: &ToxicSignalInboxRecentResponse,
) -> ToxicSignalGroupRecentResponse {
    let mut items = inbox_recent.items.clone();
    items.sort_by_key(|item| item.created_at_ms);

    let mut groups = Vec::<ToxicSignalGroup>::new();
    let mut open_group_by_key = HashMap::<String, usize>::new();

    for item in items {
        let key = format!(
            "{}|{}|{}",
            item.symbol.to_ascii_lowercase(),
            item.signal_kind,
            item.direction_bias
        );
        let maybe_existing = open_group_by_key.get(&key).copied();
        if let Some(index) = maybe_existing {
            let within_window = item
                .created_at_ms
                .saturating_sub(groups[index].last_seen_at_ms)
                <= DEFAULT_COOLDOWN_WINDOW_MS;
            if within_window {
                merge_item_into_group(&mut groups[index], &item);
                continue;
            }
        }

        let next_index = groups.len();
        groups.push(group_from_item(next_index, &item));
        open_group_by_key.insert(key, next_index);
    }

    ToxicSignalGroupRecentResponse {
        read_only: true,
        runtime_modified: false,
        analysis_only: true,
        execution_enabled: false,
        manual_review_required: true,
        runtime_weight_modified: false,
        config_modified: false,
        mode: "analysis_only".to_string(),
        selected_symbol: requested_symbol.to_string(),
        status: if groups.is_empty() {
            "empty_signal_groups".to_string()
        } else {
            "signal_groups_ready".to_string()
        },
        cooldown_window_ms: DEFAULT_COOLDOWN_WINDOW_MS,
        warnings: build_warnings(inbox_recent),
        groups,
    }
}

pub fn build_toxic_signal_group_status(
    recent: &ToxicSignalGroupRecentResponse,
) -> ToxicSignalGroupStatusResponse {
    ToxicSignalGroupStatusResponse {
        read_only: true,
        runtime_modified: false,
        analysis_only: true,
        execution_enabled: false,
        manual_review_required: true,
        runtime_weight_modified: false,
        config_modified: false,
        enabled: true,
        mode: "analysis_only".to_string(),
        selected_symbol: recent.selected_symbol.clone(),
        status: recent.status.clone(),
        cooldown_window_ms: recent.cooldown_window_ms,
        group_count: recent.groups.len(),
        last_group_at_ms: recent
            .groups
            .iter()
            .map(|group| group.last_seen_at_ms)
            .max(),
        safety_boundary: vec![
            "readOnly=true".to_string(),
            "runtimeModified=false".to_string(),
            "analysisOnly=true".to_string(),
            "executionEnabled=false".to_string(),
            "Original signals preserved".to_string(),
            "No order placement".to_string(),
            "No cancel/amend".to_string(),
            "No wallet/signing".to_string(),
            "No transaction construction".to_string(),
            "No live trading".to_string(),
        ],
    }
}

pub fn build_toxic_signal_group_detail(
    requested_symbol: &str,
    group_id: &str,
    recent: &ToxicSignalGroupRecentResponse,
) -> ToxicSignalGroupDetailResponse {
    let group = recent
        .groups
        .iter()
        .find(|group| group.group_id == group_id)
        .cloned();
    ToxicSignalGroupDetailResponse {
        read_only: true,
        runtime_modified: false,
        analysis_only: true,
        execution_enabled: false,
        manual_review_required: true,
        runtime_weight_modified: false,
        config_modified: false,
        mode: "analysis_only".to_string(),
        selected_symbol: requested_symbol.to_string(),
        available: group.is_some(),
        reason: group
            .is_none()
            .then(|| "group_id_not_found_in_read_only_signal_groups".to_string()),
        group,
    }
}

fn group_from_item(index: usize, item: &ToxicSignalInboxItem) -> ToxicSignalGroup {
    ToxicSignalGroup {
        group_id: format!(
            "group_{}_{}_{}_{}",
            slug(&item.symbol),
            slug(&item.signal_kind),
            slug(&item.direction_bias),
            index + 1
        ),
        symbol: item.symbol.clone(),
        signal_kind: item.signal_kind.clone(),
        direction_bias: item.direction_bias.clone(),
        count: 1,
        first_seen_at_ms: item.created_at_ms,
        last_seen_at_ms: item.created_at_ms,
        cooldown_window_ms: DEFAULT_COOLDOWN_WINDOW_MS,
        max_severity: item.severity.clone(),
        avg_confidence: item.confidence,
        representative_signal_id: item.signal_id.clone(),
        member_signal_ids: vec![item.signal_id.clone()],
        operator_action: map_operator_action(item.operator_action, 1),
        suppression_hint: "Grouped for display only. Original signals are preserved.".to_string(),
        original_signals_preserved: true,
        representative_confidence: item.confidence,
        read_only: true,
        runtime_modified: false,
        analysis_only: true,
        execution_enabled: false,
    }
}

fn merge_item_into_group(group: &mut ToxicSignalGroup, item: &ToxicSignalInboxItem) {
    let previous_count = group.count as f64;
    group.count += 1;
    group.last_seen_at_ms = item.created_at_ms;
    group.avg_confidence =
        ((group.avg_confidence * previous_count) + item.confidence) / group.count as f64;
    group.member_signal_ids.push(item.signal_id.clone());
    if severity_rank(&item.severity) > severity_rank(&group.max_severity) {
        group.max_severity = item.severity.clone();
    }
    if item.confidence > group.representative_confidence
        || (item.confidence == group.representative_confidence
            && item.created_at_ms >= group.last_seen_at_ms)
    {
        group.representative_signal_id = item.signal_id.clone();
        group.representative_confidence = item.confidence;
    }
    group.operator_action =
        combine_operator_action(group.operator_action, item.operator_action, group.count);
}

fn build_warnings(inbox_recent: &ToxicSignalInboxRecentResponse) -> Vec<String> {
    let mut warnings = inbox_recent.warnings.clone();
    warnings.push("grouped_for_display_only".to_string());
    warnings.sort();
    warnings.dedup();
    warnings
}

fn combine_operator_action(
    current: ToxicSignalGroupOperatorAction,
    incoming: ToxicSignalInboxOperatorAction,
    count: usize,
) -> ToxicSignalGroupOperatorAction {
    let incoming = map_operator_action(incoming, count);
    if matches!(current, ToxicSignalGroupOperatorAction::NoTradeWarningGroup)
        || matches!(
            incoming,
            ToxicSignalGroupOperatorAction::NoTradeWarningGroup
        )
    {
        ToxicSignalGroupOperatorAction::NoTradeWarningGroup
    } else if matches!(current, ToxicSignalGroupOperatorAction::NeedsMoreData)
        || matches!(incoming, ToxicSignalGroupOperatorAction::NeedsMoreData)
    {
        ToxicSignalGroupOperatorAction::NeedsMoreData
    } else if count > 1 {
        ToxicSignalGroupOperatorAction::ReviewGroupedSignal
    } else {
        ToxicSignalGroupOperatorAction::WatchGroupOnly
    }
}

fn map_operator_action(
    action: ToxicSignalInboxOperatorAction,
    count: usize,
) -> ToxicSignalGroupOperatorAction {
    match action {
        ToxicSignalInboxOperatorAction::NoTradeWarning => {
            ToxicSignalGroupOperatorAction::NoTradeWarningGroup
        }
        ToxicSignalInboxOperatorAction::NeedsMoreData => {
            ToxicSignalGroupOperatorAction::NeedsMoreData
        }
        _ if count > 1 => ToxicSignalGroupOperatorAction::ReviewGroupedSignal,
        _ => ToxicSignalGroupOperatorAction::WatchGroupOnly,
    }
}

fn severity_rank(severity: &str) -> u8 {
    match severity {
        "high" => 3,
        "medium" => 2,
        "low" => 1,
        _ => 0,
    }
}

fn slug(value: &str) -> String {
    value
        .chars()
        .map(|ch| match ch {
            'a'..='z' | 'A'..='Z' | '0'..='9' => ch.to_ascii_lowercase(),
            _ => '_',
        })
        .collect()
}
