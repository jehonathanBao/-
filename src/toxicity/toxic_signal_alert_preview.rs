use std::{collections::BTreeMap, fmt::Write};

use crate::types::{
    toxic_signal_alert_preview::{
        ToxicSignalAlertPreviewBucket, ToxicSignalAlertPreviewExplainResponse,
        ToxicSignalAlertPreviewFilter, ToxicSignalAlertPreviewGate, ToxicSignalAlertPreviewItem,
        ToxicSignalAlertPreviewResponse, ToxicSignalAlertPreviewStatusResponse,
        ToxicSignalAlertPreviewSummary,
    },
    toxic_signal_inbox::{ToxicSignalInboxItem, ToxicSignalInboxRecentResponse},
};

pub fn build_toxic_signal_alert_preview(
    requested_symbol: &str,
    inbox_recent: &ToxicSignalInboxRecentResponse,
    gate: ToxicSignalAlertPreviewGate,
) -> ToxicSignalAlertPreviewResponse {
    let items = inbox_recent
        .items
        .iter()
        .map(build_item)
        .collect::<Vec<_>>();
    let summary = build_summary(&items);
    let by_symbol = build_buckets(&items, |item| (&item.symbol, &item.symbol));
    let by_signal_kind = build_buckets(&items, |item| (&item.signal_kind, &item.signal_kind));
    let operator_notes = vec![
        "Notification preview only. No notification is sent from this view.".to_string(),
        "Signal-only review surface. No webhook, order placement, or live execution is available."
            .to_string(),
        "Manual review required before enabling any downstream notification path.".to_string(),
    ];
    let markdown = build_markdown(MarkdownView {
        selected_symbol: requested_symbol,
        gate: &gate,
        summary: &summary,
        items: &items,
        operator_notes: &operator_notes,
    });

    ToxicSignalAlertPreviewResponse {
        read_only: true,
        runtime_modified: false,
        analysis_only: true,
        execution_enabled: false,
        manual_review_required: true,
        notification_sent: false,
        execution_triggered: false,
        preview_only: true,
        runtime_weight_modified: false,
        config_modified: false,
        mode: "notification_preview_only".to_string(),
        status: if items.is_empty() {
            "empty_notification_preview".to_string()
        } else {
            "notification_preview_ready".to_string()
        },
        selected_symbol: requested_symbol.to_string(),
        filter: build_filter(requested_symbol),
        gate,
        summary,
        by_symbol,
        by_signal_kind,
        items,
        operator_notes,
        markdown,
    }
}

pub fn build_toxic_signal_alert_explain(
    signal_id: &str,
    inbox_recent: &ToxicSignalInboxRecentResponse,
    gate: &ToxicSignalAlertPreviewGate,
) -> ToxicSignalAlertPreviewExplainResponse {
    let item = inbox_recent
        .items
        .iter()
        .find(|item| item.signal_id == signal_id);

    match item {
        Some(item) => {
            let breakdown = build_decision_breakdown(item, gate);
            ToxicSignalAlertPreviewExplainResponse {
                read_only: true,
                runtime_modified: false,
                analysis_only: true,
                execution_enabled: false,
                notification_sent: false,
                execution_triggered: false,
                found: true,
                signal_id: item.signal_id.clone(),
                symbol: item.symbol.clone(),
                alert_decision: breakdown.preview_status,
                decision_reasons: breakdown.decision_reasons,
                suppression_reasons: breakdown.suppression_reasons,
                missing_inputs: breakdown.missing_inputs,
                operator_note: "Preview only. No notification was sent.".to_string(),
                reason: None,
            }
        }
        None => ToxicSignalAlertPreviewExplainResponse {
            read_only: true,
            runtime_modified: false,
            analysis_only: true,
            execution_enabled: false,
            notification_sent: false,
            execution_triggered: false,
            found: false,
            signal_id: signal_id.to_string(),
            symbol: String::new(),
            alert_decision: "not_found".to_string(),
            decision_reasons: Vec::new(),
            suppression_reasons: Vec::new(),
            missing_inputs: Vec::new(),
            operator_note: "Preview only. No notification was sent.".to_string(),
            reason: Some("signal_id_not_found_in_alert_preview".to_string()),
        },
    }
}

pub fn build_toxic_signal_alert_preview_status(
    preview: &ToxicSignalAlertPreviewResponse,
) -> ToxicSignalAlertPreviewStatusResponse {
    ToxicSignalAlertPreviewStatusResponse {
        read_only: true,
        runtime_modified: false,
        analysis_only: true,
        execution_enabled: false,
        manual_review_required: true,
        notification_sent: false,
        execution_triggered: false,
        preview_only: true,
        runtime_weight_modified: false,
        config_modified: false,
        enabled: true,
        mode: "notification_preview_only".to_string(),
        status: preview.status.clone(),
        selected_symbol: preview.selected_symbol.clone(),
        filter: preview.filter.clone(),
        gate: preview.gate.clone(),
        total_signals: preview.summary.total_signals,
        notify_candidate_count: preview.summary.notify_candidates,
        suppressed_count: preview.summary.suppressed_signals,
        safety_boundary: vec![
            "readOnly=true".to_string(),
            "analysisOnly=true".to_string(),
            "executionEnabled=false".to_string(),
            "notificationSent=false".to_string(),
            "executionTriggered=false".to_string(),
            "previewOnly=true".to_string(),
            "No webhook".to_string(),
            "No order placement".to_string(),
            "No cancel/amend".to_string(),
            "No wallet/signing".to_string(),
            "No live trading".to_string(),
        ],
    }
}

fn build_filter(requested_symbol: &str) -> ToxicSignalAlertPreviewFilter {
    ToxicSignalAlertPreviewFilter {
        symbol: requested_symbol.to_string(),
        view_only: true,
        persistent_watchlist_enabled: false,
        runtime_monitor_modified: false,
    }
}

fn build_item(item: &ToxicSignalInboxItem) -> ToxicSignalAlertPreviewItem {
    let breakdown = build_decision_breakdown(
        item,
        &ToxicSignalAlertPreviewGate {
            dedup_window_ms: 0,
            min_severity: String::new(),
            require_cross_venue: false,
            require_markout: false,
            require_liquidity_drain: false,
            telegram_enabled: false,
            notification_sent: false,
            execution_triggered: false,
        },
    );

    ToxicSignalAlertPreviewItem {
        signal_id: item.signal_id.clone(),
        symbol: item.symbol.clone(),
        signal_kind: item.signal_kind.clone(),
        direction_bias: item.direction_bias.clone(),
        severity: item.severity.clone(),
        confidence: item.confidence,
        would_notify_if_enabled: breakdown.would_notify_if_enabled,
        preview_status: breakdown.preview_status.clone(),
        no_trade_only: breakdown.no_trade_only,
        quality_bucket: item.quality.quality_bucket.clone(),
        latest_governance_decision: item.governance.latest_decision.clone(),
        markout_readiness: breakdown.markout_readiness,
        suppression_reasons: breakdown.suppression_reasons,
        review_reasons: breakdown.decision_reasons.clone(),
        preview_message: build_preview_message(item, &breakdown.preview_status),
        notification_sent: false,
        execution_triggered: false,
        read_only: true,
        analysis_only: true,
        execution_enabled: false,
    }
}

#[derive(Debug, Clone)]
struct DecisionBreakdown {
    preview_status: String,
    would_notify_if_enabled: bool,
    no_trade_only: bool,
    markout_readiness: String,
    decision_reasons: Vec<String>,
    suppression_reasons: Vec<String>,
    missing_inputs: Vec<String>,
}

fn build_decision_breakdown(
    item: &ToxicSignalInboxItem,
    _gate: &ToxicSignalAlertPreviewGate,
) -> DecisionBreakdown {
    let no_trade_only = item.recommendation.no_trade_only;
    let governance_hold = matches!(
        item.governance.latest_decision.as_str(),
        "suppress_for_now" | "reject_recommendation"
    );
    let not_enough_data = has_not_enough_data(item);
    let quality_hold = matches!(item.quality.quality_bucket.as_str(), "bad" | "weak");
    let markout_readiness = markout_readiness(item);
    let missing_inputs = missing_inputs(item);
    let preview_status = if no_trade_only {
        "suppressed_no_trade_only"
    } else if governance_hold {
        "suppressed_governance_hold"
    } else if not_enough_data {
        "not_enough_data"
    } else if quality_hold {
        "suppressed_quality_hold"
    } else if item.severity.eq_ignore_ascii_case("high") && item.confidence >= 0.80 {
        "notify_candidate"
    } else if item.severity.eq_ignore_ascii_case("medium") || item.confidence >= 0.60 {
        "review_candidate"
    } else {
        "suppressed_low_priority"
    }
    .to_string();

    let mut decision_reasons = decision_reasons(
        item,
        &preview_status,
        governance_hold,
        not_enough_data,
        quality_hold,
    );
    let mut suppression_reasons = suppression_reasons(
        item,
        &preview_status,
        governance_hold,
        not_enough_data,
        quality_hold,
    );

    decision_reasons.dedup();
    suppression_reasons.dedup();

    DecisionBreakdown {
        preview_status: preview_status.clone(),
        would_notify_if_enabled: preview_status == "notify_candidate",
        no_trade_only,
        markout_readiness,
        decision_reasons,
        suppression_reasons,
        missing_inputs,
    }
}

fn build_summary(items: &[ToxicSignalAlertPreviewItem]) -> ToxicSignalAlertPreviewSummary {
    ToxicSignalAlertPreviewSummary {
        total_signals: items.len(),
        notify_candidates: items
            .iter()
            .filter(|item| item.preview_status == "notify_candidate")
            .count(),
        review_candidates: items
            .iter()
            .filter(|item| item.preview_status == "review_candidate")
            .count(),
        suppressed_signals: items
            .iter()
            .filter(|item| {
                item.preview_status.starts_with("suppressed_")
                    || item.preview_status == "not_enough_data"
            })
            .count(),
        no_trade_only_signals: items.iter().filter(|item| item.no_trade_only).count(),
        governance_hold_signals: items
            .iter()
            .filter(|item| item.preview_status == "suppressed_governance_hold")
            .count(),
        not_enough_data_signals: items
            .iter()
            .filter(|item| item.preview_status == "not_enough_data")
            .count(),
    }
}

fn build_buckets<'a, F>(
    items: &'a [ToxicSignalAlertPreviewItem],
    key_fn: F,
) -> Vec<ToxicSignalAlertPreviewBucket>
where
    F: Fn(&'a ToxicSignalAlertPreviewItem) -> (&'a str, &'a str),
{
    #[derive(Default)]
    struct BucketAccumulator {
        total_signals: usize,
        notify_candidates: usize,
        review_candidates: usize,
        suppressed_signals: usize,
        no_trade_only_signals: usize,
        not_enough_data_signals: usize,
    }

    let mut buckets: BTreeMap<String, (String, BucketAccumulator)> = BTreeMap::new();
    for item in items {
        let (key, label) = key_fn(item);
        let entry = buckets
            .entry(key.to_string())
            .or_insert_with(|| (label.to_string(), BucketAccumulator::default()));
        let accumulator = &mut entry.1;
        accumulator.total_signals += 1;
        if item.preview_status == "notify_candidate" {
            accumulator.notify_candidates += 1;
        }
        if item.preview_status == "review_candidate" {
            accumulator.review_candidates += 1;
        }
        if item.preview_status.starts_with("suppressed_")
            || item.preview_status == "not_enough_data"
        {
            accumulator.suppressed_signals += 1;
        }
        if item.no_trade_only {
            accumulator.no_trade_only_signals += 1;
        }
        if item.preview_status == "not_enough_data" {
            accumulator.not_enough_data_signals += 1;
        }
    }

    let mut result = buckets
        .into_iter()
        .map(
            |(key, (label, accumulator))| ToxicSignalAlertPreviewBucket {
                key,
                label,
                total_signals: accumulator.total_signals,
                notify_candidates: accumulator.notify_candidates,
                review_candidates: accumulator.review_candidates,
                suppressed_signals: accumulator.suppressed_signals,
                no_trade_only_signals: accumulator.no_trade_only_signals,
                not_enough_data_signals: accumulator.not_enough_data_signals,
            },
        )
        .collect::<Vec<_>>();

    result.sort_by(|left, right| {
        right
            .total_signals
            .cmp(&left.total_signals)
            .then_with(|| right.notify_candidates.cmp(&left.notify_candidates))
            .then_with(|| left.key.cmp(&right.key))
    });
    result
}

struct MarkdownView<'a> {
    selected_symbol: &'a str,
    gate: &'a ToxicSignalAlertPreviewGate,
    summary: &'a ToxicSignalAlertPreviewSummary,
    items: &'a [ToxicSignalAlertPreviewItem],
    operator_notes: &'a [String],
}

fn build_markdown(view: MarkdownView<'_>) -> String {
    let mut markdown = String::new();
    let _ = writeln!(markdown, "# Signal Alert Rules / Notification Preview");
    let _ = writeln!(markdown);
    let _ = writeln!(markdown, "Selected Symbol: {}", view.selected_symbol);
    let _ = writeln!(markdown);
    let _ = writeln!(markdown, "## Summary");
    let _ = writeln!(markdown, "- Total signals: {}", view.summary.total_signals);
    let _ = writeln!(
        markdown,
        "- Notify candidates: {}",
        view.summary.notify_candidates
    );
    let _ = writeln!(
        markdown,
        "- Review candidates: {}",
        view.summary.review_candidates
    );
    let _ = writeln!(
        markdown,
        "- Suppressed signals: {}",
        view.summary.suppressed_signals
    );
    let _ = writeln!(
        markdown,
        "- No-trade-only signals: {}",
        view.summary.no_trade_only_signals
    );
    let _ = writeln!(
        markdown,
        "- Governance hold signals: {}",
        view.summary.governance_hold_signals
    );
    let _ = writeln!(
        markdown,
        "- Not enough data signals: {}",
        view.summary.not_enough_data_signals
    );
    let _ = writeln!(markdown);
    let _ = writeln!(markdown, "## Preview Gate");
    let _ = writeln!(markdown, "- Min severity: {}", view.gate.min_severity);
    let _ = writeln!(
        markdown,
        "- Require cross venue: {}",
        bool_text(view.gate.require_cross_venue)
    );
    let _ = writeln!(
        markdown,
        "- Require markout: {}",
        bool_text(view.gate.require_markout)
    );
    let _ = writeln!(
        markdown,
        "- Require liquidity drain: {}",
        bool_text(view.gate.require_liquidity_drain)
    );
    let _ = writeln!(markdown, "- Notification sent: false");
    let _ = writeln!(markdown, "- Execution triggered: false");
    let _ = writeln!(markdown);
    let _ = writeln!(markdown, "## Preview Items");
    if view.items.is_empty() {
        let _ = writeln!(markdown, "- None");
    } else {
        for item in view.items {
            let _ = writeln!(
                markdown,
                "- {} / {}: {} (wouldNotifyIfEnabled={})",
                item.symbol,
                item.signal_kind,
                item.preview_status,
                bool_text(item.would_notify_if_enabled)
            );
        }
    }
    let _ = writeln!(markdown);
    let _ = writeln!(markdown, "## Operator Notes");
    for note in view.operator_notes {
        let _ = writeln!(markdown, "- {note}");
    }
    let _ = writeln!(markdown, "- No webhook");
    let _ = writeln!(markdown, "- No order placement");
    let _ = writeln!(markdown, "- No wallet/signing");
    let _ = writeln!(markdown, "- No live trading");
    markdown
}

fn has_not_enough_data(item: &ToxicSignalInboxItem) -> bool {
    if !item.quality.available || !item.recommendation.available {
        return true;
    }
    [
        item.markout.one_minute.as_str(),
        item.markout.five_minute.as_str(),
        item.markout.fifteen_minute.as_str(),
        item.markout.one_hour.as_str(),
    ]
    .iter()
    .any(|outcome| outcome.eq_ignore_ascii_case("not_enough_data"))
}

fn decision_reasons(
    item: &ToxicSignalInboxItem,
    preview_status: &str,
    governance_hold: bool,
    not_enough_data: bool,
    quality_hold: bool,
) -> Vec<String> {
    let mut reasons = Vec::new();

    if item.severity.eq_ignore_ascii_case("high") {
        reasons.push("severity is high".to_string());
    } else if item.severity.eq_ignore_ascii_case("medium") {
        reasons.push("severity reaches review threshold".to_string());
    }

    if item.confidence >= 0.80 {
        reasons.push("confidence is above notify threshold".to_string());
    } else if item.confidence >= 0.60 {
        reasons.push("confidence is above review threshold".to_string());
    }

    if item.quality.available && !quality_hold {
        reasons.push(format!("quality bucket is {}", item.quality.quality_bucket));
    }

    if !item.recommendation.no_trade_only {
        reasons.push("signal is not no_trade_only".to_string());
    }

    if !governance_hold && item.governance.ledger_available {
        reasons.push("governance is not on hold".to_string());
    }

    if !not_enough_data && item.markout.available {
        reasons.push("markout evidence is available".to_string());
    }

    match preview_status {
        "notify_candidate" => {
            reasons.push("signal cleared notify preview rules".to_string());
        }
        "review_candidate" => {
            reasons.push(
                "signal is worth manual review but does not clear notify preview".to_string(),
            );
        }
        "not_enough_data" => {
            reasons.push("preview stopped because required evidence is incomplete".to_string());
        }
        _ => {}
    }

    reasons
}

fn suppression_reasons(
    item: &ToxicSignalInboxItem,
    preview_status: &str,
    governance_hold: bool,
    not_enough_data: bool,
    quality_hold: bool,
) -> Vec<String> {
    let mut reasons = Vec::new();

    match preview_status {
        "suppressed_no_trade_only" => {
            reasons.push("recommendation classified this signal as no_trade_only".to_string())
        }
        "suppressed_governance_hold" if governance_hold => reasons.push(format!(
            "governance decision is {}",
            item.governance.latest_decision
        )),
        "not_enough_data" if not_enough_data => reasons.push(
            "markout, quality, or recommendation evidence is still not_enough_data".to_string(),
        ),
        "suppressed_quality_hold" if quality_hold => {
            reasons.push(format!("quality bucket is {}", item.quality.quality_bucket))
        }
        "suppressed_low_priority" => {
            reasons.push("signal priority is below the preview threshold".to_string())
        }
        _ => {}
    }

    reasons
}

fn missing_inputs(item: &ToxicSignalInboxItem) -> Vec<String> {
    let mut inputs = Vec::new();

    if !item.markout.available {
        inputs.push("markout summary unavailable".to_string());
    }
    for (label, outcome) in [
        ("+1m", item.markout.one_minute.as_str()),
        ("+5m", item.markout.five_minute.as_str()),
        ("+15m", item.markout.fifteen_minute.as_str()),
        ("+1h", item.markout.one_hour.as_str()),
    ] {
        if outcome.eq_ignore_ascii_case("not_enough_data") {
            inputs.push(format!("markout {label} is not_enough_data"));
        }
    }
    if !item.quality.available {
        inputs.push("quality summary unavailable".to_string());
    }
    if !item.recommendation.available {
        inputs.push("recommendation summary unavailable".to_string());
    }
    if !item.governance.ledger_available {
        inputs.push("governance ledger unavailable".to_string());
    }

    inputs.dedup();
    inputs
}

fn markout_readiness(item: &ToxicSignalInboxItem) -> String {
    let outcomes = [
        item.markout.one_minute.as_str(),
        item.markout.five_minute.as_str(),
        item.markout.fifteen_minute.as_str(),
        item.markout.one_hour.as_str(),
    ];
    if outcomes
        .iter()
        .any(|outcome| outcome.eq_ignore_ascii_case("not_enough_data"))
    {
        "not_enough_data".to_string()
    } else if outcomes
        .iter()
        .any(|outcome| outcome.eq_ignore_ascii_case("adverse"))
    {
        "adverse_present".to_string()
    } else if outcomes
        .iter()
        .any(|outcome| outcome.eq_ignore_ascii_case("aligned"))
    {
        "aligned_present".to_string()
    } else {
        "neutral_only".to_string()
    }
}

fn build_preview_message(item: &ToxicSignalInboxItem, preview_status: &str) -> String {
    format!(
        "[Preview] {} / {} / severity={} / confidence={:.0}% / status={} / notificationSent=false / executionTriggered=false",
        item.symbol,
        item.signal_kind,
        item.severity,
        item.confidence * 100.0,
        preview_status
    )
}

fn bool_text(value: bool) -> &'static str {
    if value {
        "true"
    } else {
        "false"
    }
}
