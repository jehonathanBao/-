use std::{
    collections::{BTreeMap, BTreeSet},
    fmt::Write,
};

use crate::types::{
    toxic_quality_scorecard::ToxicQualityScorecardSummaryResponse,
    toxic_signal_group::ToxicSignalGroupRecentResponse,
    toxic_signal_history::{ToxicSignalHistoryAlertItem, ToxicSignalHistorySignalItem},
    toxic_signal_inbox::{ToxicSignalInboxItem, ToxicSignalInboxRecentResponse},
    toxic_signal_report::{
        ToxicSignalReportBucket, ToxicSignalReportDailyResponse, ToxicSignalReportFilter,
        ToxicSignalReportMarkoutSummary, ToxicSignalReportRollingResponse,
        ToxicSignalReportStatusResponse, ToxicSignalReportSummary, ToxicSignalReportTopGroup,
        ToxicSignalRollingDigestSummary,
    },
    toxic_weight_recommendation::ToxicWeightRecommendationSummaryResponse,
};

pub fn build_toxic_signal_report_status(
    requested_symbol: &str,
    report_date: &str,
    inbox_recent: &ToxicSignalInboxRecentResponse,
    group_recent: &ToxicSignalGroupRecentResponse,
) -> ToxicSignalReportStatusResponse {
    ToxicSignalReportStatusResponse {
        read_only: true,
        runtime_modified: false,
        analysis_only: true,
        execution_enabled: false,
        manual_review_required: true,
        runtime_weight_modified: false,
        config_modified: false,
        enabled: true,
        report_type: "daily".to_string(),
        mode: "analysis_only".to_string(),
        selected_symbol: requested_symbol.to_string(),
        status: if inbox_recent.items.is_empty() {
            "empty_daily_report".to_string()
        } else {
            "daily_report_ready".to_string()
        },
        date: report_date.to_string(),
        filter: build_filter(requested_symbol),
        total_signals: inbox_recent.items.len(),
        group_count: group_recent.groups.len(),
        last_signal_at_ms: inbox_recent
            .items
            .iter()
            .map(|item| item.created_at_ms)
            .max(),
        safety_boundary: vec![
            "readOnly=true".to_string(),
            "runtimeModified=false".to_string(),
            "analysisOnly=true".to_string(),
            "executionEnabled=false".to_string(),
            "Manual review required".to_string(),
            "No order placement".to_string(),
            "No cancel/amend".to_string(),
            "No wallet/signing".to_string(),
            "No transaction construction".to_string(),
            "No live trading".to_string(),
            "No runtime config mutation".to_string(),
        ],
    }
}

pub fn build_toxic_signal_daily_report(
    requested_symbol: &str,
    report_date: &str,
    inbox_recent: &ToxicSignalInboxRecentResponse,
    group_recent: &ToxicSignalGroupRecentResponse,
    quality_summary: &ToxicQualityScorecardSummaryResponse,
    _recommendation_summary: &ToxicWeightRecommendationSummaryResponse,
) -> ToxicSignalReportDailyResponse {
    let downgrade_keys: BTreeSet<&str> = quality_summary
        .downgrade_candidates
        .iter()
        .map(|candidate| candidate.key.as_str())
        .collect();
    let summary = build_summary(inbox_recent, group_recent, &downgrade_keys);
    let markout_summary = build_markout_summary(inbox_recent);
    let by_symbol = build_buckets_by_symbol(inbox_recent, &downgrade_keys);
    let by_signal_kind = build_buckets_by_signal_kind(inbox_recent, &downgrade_keys);
    let top_groups = build_top_groups(group_recent);
    let operator_notes = vec![
        "Signal-only report. No trading action is available.".to_string(),
        "Use this digest for manual review and monitoring.".to_string(),
        "Manual review required.".to_string(),
    ];
    let markdown = build_markdown(MarkdownView {
        report_date,
        summary: &summary,
        markout_summary: &markout_summary,
        by_symbol: &by_symbol,
        by_signal_kind: &by_signal_kind,
        top_groups: &top_groups,
        operator_notes: &operator_notes,
    });

    ToxicSignalReportDailyResponse {
        read_only: true,
        runtime_modified: false,
        analysis_only: true,
        execution_enabled: false,
        manual_review_required: true,
        runtime_weight_modified: false,
        config_modified: false,
        report_type: "daily".to_string(),
        mode: "analysis_only".to_string(),
        date: report_date.to_string(),
        filter: build_filter(requested_symbol),
        summary,
        markout_summary,
        by_symbol,
        by_signal_kind,
        top_groups,
        operator_notes,
        markdown,
    }
}

pub fn build_toxic_signal_rolling_report(
    requested_symbol: &str,
    window: &str,
    signal_history: &[ToxicSignalHistorySignalItem],
    alert_history: &[ToxicSignalHistoryAlertItem],
) -> ToxicSignalReportRollingResponse {
    let summary = build_rolling_summary(signal_history, alert_history);
    let operator_notes = vec![
        "Signal-only rolling digest. No trading action is available.".to_string(),
        "Rolling digest is limited to available in-memory history.".to_string(),
        "No database write, no file write, and no notification path are enabled.".to_string(),
    ];
    let markdown = build_rolling_markdown(window, &summary, &operator_notes);

    ToxicSignalReportRollingResponse {
        read_only: true,
        runtime_modified: false,
        analysis_only: true,
        execution_enabled: false,
        manual_review_required: true,
        runtime_weight_modified: false,
        config_modified: false,
        report_type: "rolling".to_string(),
        mode: "analysis_only".to_string(),
        window: window.to_string(),
        retention_mode: "in_memory_bounded".to_string(),
        durable_storage_enabled: false,
        database_write_enabled: false,
        filter: build_filter(requested_symbol),
        summary,
        operator_notes,
        markdown,
    }
}

fn build_filter(requested_symbol: &str) -> ToxicSignalReportFilter {
    ToxicSignalReportFilter {
        symbol: requested_symbol.to_string(),
        view_only: true,
        persistent_watchlist_enabled: false,
        runtime_monitor_modified: false,
    }
}

fn build_rolling_summary(
    signal_history: &[ToxicSignalHistorySignalItem],
    alert_history: &[ToxicSignalHistoryAlertItem],
) -> ToxicSignalRollingDigestSummary {
    let mut aligned = 0usize;
    let mut adverse = 0usize;
    let mut neutral = 0usize;
    let mut not_enough_data = 0usize;
    let mut symbol_counts = BTreeMap::<String, usize>::new();
    let mut signal_kind_counts = BTreeMap::<String, usize>::new();
    let mut no_trade_only_candidates = 0usize;
    let mut downgrade_candidates = 0usize;
    let mut notify_candidates = 0usize;
    let mut review_candidates = 0usize;

    for item in signal_history {
        *symbol_counts.entry(item.symbol.clone()).or_default() += 1;
        *signal_kind_counts
            .entry(item.signal_kind.clone())
            .or_default() += 1;

        if item.no_trade_only {
            no_trade_only_candidates += 1;
        }
        if item
            .recommendation_action
            .eq_ignore_ascii_case("downgrade_candidate")
        {
            downgrade_candidates += 1;
        }

        for outcome in [
            item.markout_one_minute.as_str(),
            item.markout_five_minute.as_str(),
            item.markout_fifteen_minute.as_str(),
            item.markout_one_hour.as_str(),
        ] {
            match outcome {
                "aligned" => aligned += 1,
                "adverse" => adverse += 1,
                "neutral" => neutral += 1,
                _ => not_enough_data += 1,
            }
        }

        if item.quality_bucket.eq_ignore_ascii_case("not_enough_data")
            && ![
                item.markout_one_minute.as_str(),
                item.markout_five_minute.as_str(),
                item.markout_fifteen_minute.as_str(),
                item.markout_one_hour.as_str(),
            ]
            .iter()
            .any(|outcome| outcome.eq_ignore_ascii_case("not_enough_data"))
        {
            not_enough_data += 1;
        }
    }

    for item in alert_history {
        if item.preview_status == "notify_candidate" {
            notify_candidates += 1;
        }
        if item.preview_status == "review_candidate" {
            review_candidates += 1;
        }
    }

    ToxicSignalRollingDigestSummary {
        total_signals: signal_history.len(),
        aligned,
        adverse,
        neutral,
        not_enough_data,
        top_symbols: top_keys(symbol_counts),
        top_signal_kinds: top_keys(signal_kind_counts),
        no_trade_only_candidates,
        downgrade_candidates,
        notify_candidates,
        review_candidates,
    }
}

fn build_summary(
    inbox_recent: &ToxicSignalInboxRecentResponse,
    group_recent: &ToxicSignalGroupRecentResponse,
    downgrade_keys: &BTreeSet<&str>,
) -> ToxicSignalReportSummary {
    ToxicSignalReportSummary {
        total_signals: inbox_recent.items.len(),
        grouped_signals: group_recent.groups.iter().map(|group| group.count).sum(),
        high_severity_signals: inbox_recent
            .items
            .iter()
            .filter(|item| item.severity.eq_ignore_ascii_case("high"))
            .count(),
        no_trade_only_candidates: inbox_recent
            .items
            .iter()
            .filter(|item| item.recommendation.no_trade_only)
            .count(),
        downgrade_candidates: inbox_recent
            .items
            .iter()
            .filter(|item| downgrade_keys.contains(item.signal_kind.as_str()))
            .count(),
        not_enough_data_signals: inbox_recent
            .items
            .iter()
            .filter(|item| has_not_enough_data(item))
            .count(),
    }
}

fn build_markout_summary(
    inbox_recent: &ToxicSignalInboxRecentResponse,
) -> ToxicSignalReportMarkoutSummary {
    let mut summary = ToxicSignalReportMarkoutSummary {
        aligned: 0,
        adverse: 0,
        neutral: 0,
        not_enough_data: 0,
    };

    for item in &inbox_recent.items {
        for outcome in [
            item.markout.one_minute.as_str(),
            item.markout.five_minute.as_str(),
            item.markout.fifteen_minute.as_str(),
            item.markout.one_hour.as_str(),
        ] {
            match outcome {
                "aligned" => summary.aligned += 1,
                "adverse" => summary.adverse += 1,
                "neutral" => summary.neutral += 1,
                _ => summary.not_enough_data += 1,
            }
        }
    }

    summary
}

fn build_buckets_by_symbol(
    inbox_recent: &ToxicSignalInboxRecentResponse,
    downgrade_keys: &BTreeSet<&str>,
) -> Vec<ToxicSignalReportBucket> {
    build_buckets(inbox_recent, downgrade_keys, |item| {
        (&item.symbol, &item.symbol)
    })
}

fn build_buckets_by_signal_kind(
    inbox_recent: &ToxicSignalInboxRecentResponse,
    downgrade_keys: &BTreeSet<&str>,
) -> Vec<ToxicSignalReportBucket> {
    build_buckets(inbox_recent, downgrade_keys, |item| {
        (&item.signal_kind, &item.signal_kind)
    })
}

fn build_buckets<'a, F>(
    inbox_recent: &'a ToxicSignalInboxRecentResponse,
    downgrade_keys: &BTreeSet<&str>,
    key_fn: F,
) -> Vec<ToxicSignalReportBucket>
where
    F: Fn(&'a ToxicSignalInboxItem) -> (&'a str, &'a str),
{
    #[derive(Default)]
    struct BucketAccumulator {
        signal_count: usize,
        high_severity_signals: usize,
        no_trade_only_candidates: usize,
        downgrade_candidates: usize,
        not_enough_data_signals: usize,
        confidence_sum: f64,
    }

    let mut buckets: BTreeMap<String, (String, BucketAccumulator)> = BTreeMap::new();

    for item in &inbox_recent.items {
        let (key, label) = key_fn(item);
        let entry = buckets
            .entry(key.to_string())
            .or_insert_with(|| (label.to_string(), BucketAccumulator::default()));
        let accumulator = &mut entry.1;
        accumulator.signal_count += 1;
        accumulator.confidence_sum += item.confidence;
        if item.severity.eq_ignore_ascii_case("high") {
            accumulator.high_severity_signals += 1;
        }
        if item.recommendation.no_trade_only {
            accumulator.no_trade_only_candidates += 1;
        }
        if downgrade_keys.contains(item.signal_kind.as_str()) {
            accumulator.downgrade_candidates += 1;
        }
        if has_not_enough_data(item) {
            accumulator.not_enough_data_signals += 1;
        }
    }

    let mut result: Vec<_> = buckets
        .into_iter()
        .map(|(key, (label, accumulator))| ToxicSignalReportBucket {
            key,
            label,
            signal_count: accumulator.signal_count,
            high_severity_signals: accumulator.high_severity_signals,
            no_trade_only_candidates: accumulator.no_trade_only_candidates,
            downgrade_candidates: accumulator.downgrade_candidates,
            not_enough_data_signals: accumulator.not_enough_data_signals,
            avg_confidence: average_confidence(
                accumulator.confidence_sum,
                accumulator.signal_count,
            ),
        })
        .collect();

    result.sort_by(|left, right| {
        right
            .signal_count
            .cmp(&left.signal_count)
            .then_with(|| right.avg_confidence.total_cmp(&left.avg_confidence))
            .then_with(|| left.key.cmp(&right.key))
    });
    result
}

fn build_top_groups(
    group_recent: &ToxicSignalGroupRecentResponse,
) -> Vec<ToxicSignalReportTopGroup> {
    let mut groups: Vec<_> = group_recent
        .groups
        .iter()
        .map(|group| ToxicSignalReportTopGroup {
            group_id: group.group_id.clone(),
            symbol: group.symbol.clone(),
            signal_kind: group.signal_kind.clone(),
            direction_bias: group.direction_bias.clone(),
            count: group.count,
            first_seen_at_ms: group.first_seen_at_ms,
            last_seen_at_ms: group.last_seen_at_ms,
            max_severity: group.max_severity.clone(),
            avg_confidence: group.avg_confidence,
            representative_signal_id: group.representative_signal_id.clone(),
            original_signals_preserved: group.original_signals_preserved,
        })
        .collect();

    groups.sort_by(|left, right| {
        right
            .count
            .cmp(&left.count)
            .then_with(|| {
                severity_rank(&right.max_severity).cmp(&severity_rank(&left.max_severity))
            })
            .then_with(|| right.avg_confidence.total_cmp(&left.avg_confidence))
            .then_with(|| left.group_id.cmp(&right.group_id))
    });
    groups.truncate(5);
    groups
}

struct MarkdownView<'a> {
    report_date: &'a str,
    summary: &'a ToxicSignalReportSummary,
    markout_summary: &'a ToxicSignalReportMarkoutSummary,
    by_symbol: &'a [ToxicSignalReportBucket],
    by_signal_kind: &'a [ToxicSignalReportBucket],
    top_groups: &'a [ToxicSignalReportTopGroup],
    operator_notes: &'a [String],
}

fn build_markdown(view: MarkdownView<'_>) -> String {
    let mut markdown = String::new();
    let _ = writeln!(markdown, "# Toxic Signal Daily Report");
    let _ = writeln!(markdown);
    let _ = writeln!(markdown, "Date: {}", view.report_date);
    let _ = writeln!(markdown);
    let _ = writeln!(markdown, "## Summary");
    let _ = writeln!(markdown, "- Total signals: {}", view.summary.total_signals);
    let _ = writeln!(
        markdown,
        "- Grouped signals: {}",
        view.summary.grouped_signals
    );
    let _ = writeln!(
        markdown,
        "- High severity signals: {}",
        view.summary.high_severity_signals
    );
    let _ = writeln!(
        markdown,
        "- No-trade-only candidates: {}",
        view.summary.no_trade_only_candidates
    );
    let _ = writeln!(
        markdown,
        "- Downgrade candidates: {}",
        view.summary.downgrade_candidates
    );
    let _ = writeln!(
        markdown,
        "- Not enough data signals: {}",
        view.summary.not_enough_data_signals
    );
    let _ = writeln!(markdown);
    let _ = writeln!(markdown, "## Markout");
    let _ = writeln!(markdown, "- Aligned: {}", view.markout_summary.aligned);
    let _ = writeln!(markdown, "- Adverse: {}", view.markout_summary.adverse);
    let _ = writeln!(markdown, "- Neutral: {}", view.markout_summary.neutral);
    let _ = writeln!(
        markdown,
        "- Not enough data: {}",
        view.markout_summary.not_enough_data
    );
    let _ = writeln!(markdown);
    let _ = writeln!(markdown, "## By Symbol");
    if view.by_symbol.is_empty() {
        let _ = writeln!(markdown, "- None");
    } else {
        for bucket in view.by_symbol {
            let _ = writeln!(
                markdown,
                "- {}: {} signals, {} high severity, {} no-trade-only, {} downgrade, {} not_enough_data",
                bucket.label,
                bucket.signal_count,
                bucket.high_severity_signals,
                bucket.no_trade_only_candidates,
                bucket.downgrade_candidates,
                bucket.not_enough_data_signals
            );
        }
    }
    let _ = writeln!(markdown);
    let _ = writeln!(markdown, "## By Signal Kind");
    if view.by_signal_kind.is_empty() {
        let _ = writeln!(markdown, "- None");
    } else {
        for bucket in view.by_signal_kind {
            let _ = writeln!(
                markdown,
                "- {}: {} signals, avg confidence {:.2}, {} no-trade-only, {} downgrade",
                bucket.label,
                bucket.signal_count,
                bucket.avg_confidence,
                bucket.no_trade_only_candidates,
                bucket.downgrade_candidates
            );
        }
    }
    let _ = writeln!(markdown);
    let _ = writeln!(markdown, "## Top Groups");
    if view.top_groups.is_empty() {
        let _ = writeln!(markdown, "- None");
    } else {
        for group in view.top_groups {
            let _ = writeln!(
                markdown,
                "- {} / {}: count={}, severity={}, representative={}",
                group.symbol,
                group.signal_kind,
                group.count,
                group.max_severity,
                group.representative_signal_id
            );
        }
    }
    let _ = writeln!(markdown);
    let _ = writeln!(markdown, "## Operator Notes");
    for note in view.operator_notes {
        let _ = writeln!(markdown, "- {note}");
    }
    let _ = writeln!(markdown, "- No order placement");
    let _ = writeln!(markdown, "- No wallet/signing");
    let _ = writeln!(markdown, "- No live trading");
    let _ = writeln!(markdown, "- Manual review required");
    markdown
}

fn build_rolling_markdown(
    window: &str,
    summary: &ToxicSignalRollingDigestSummary,
    operator_notes: &[String],
) -> String {
    let mut markdown = String::new();
    let _ = writeln!(markdown, "# Toxic Signal Rolling Digest");
    let _ = writeln!(markdown);
    let _ = writeln!(markdown, "Window: {window}");
    let _ = writeln!(markdown, "Retention Mode: in_memory_bounded");
    let _ = writeln!(markdown);
    let _ = writeln!(markdown, "## Summary");
    let _ = writeln!(markdown, "- Total signals: {}", summary.total_signals);
    let _ = writeln!(
        markdown,
        "- No-trade-only candidates: {}",
        summary.no_trade_only_candidates
    );
    let _ = writeln!(
        markdown,
        "- Downgrade candidates: {}",
        summary.downgrade_candidates
    );
    let _ = writeln!(
        markdown,
        "- Notify candidates: {}",
        summary.notify_candidates
    );
    let _ = writeln!(
        markdown,
        "- Review candidates: {}",
        summary.review_candidates
    );
    let _ = writeln!(markdown);
    let _ = writeln!(markdown, "## Markout");
    let _ = writeln!(markdown, "- Aligned: {}", summary.aligned);
    let _ = writeln!(markdown, "- Adverse: {}", summary.adverse);
    let _ = writeln!(markdown, "- Neutral: {}", summary.neutral);
    let _ = writeln!(markdown, "- Not enough data: {}", summary.not_enough_data);
    let _ = writeln!(markdown);
    let _ = writeln!(markdown, "## Top Symbols");
    if summary.top_symbols.is_empty() {
        let _ = writeln!(markdown, "- None");
    } else {
        for item in &summary.top_symbols {
            let _ = writeln!(markdown, "- {item}");
        }
    }
    let _ = writeln!(markdown);
    let _ = writeln!(markdown, "## Top Signal Kinds");
    if summary.top_signal_kinds.is_empty() {
        let _ = writeln!(markdown, "- None");
    } else {
        for item in &summary.top_signal_kinds {
            let _ = writeln!(markdown, "- {item}");
        }
    }
    let _ = writeln!(markdown);
    let _ = writeln!(markdown, "## Operator Notes");
    for note in operator_notes {
        let _ = writeln!(markdown, "- {note}");
    }
    let _ = writeln!(markdown, "- No notification sending");
    let _ = writeln!(markdown, "- No order placement");
    let _ = writeln!(markdown, "- No wallet/signing");
    let _ = writeln!(markdown, "- No live trading");
    markdown
}

fn has_not_enough_data(item: &ToxicSignalInboxItem) -> bool {
    [
        item.markout.one_minute.as_str(),
        item.markout.five_minute.as_str(),
        item.markout.fifteen_minute.as_str(),
        item.markout.one_hour.as_str(),
    ]
    .iter()
    .any(|outcome| outcome.eq_ignore_ascii_case("not_enough_data"))
}

fn average_confidence(total: f64, count: usize) -> f64 {
    if count == 0 {
        0.0
    } else {
        total / count as f64
    }
}

fn top_keys(counts: BTreeMap<String, usize>) -> Vec<String> {
    let mut items: Vec<_> = counts.into_iter().collect();
    items.sort_by(|left, right| right.1.cmp(&left.1).then_with(|| left.0.cmp(&right.0)));
    items
        .into_iter()
        .take(5)
        .map(|(key, count)| format!("{key} ({count})"))
        .collect()
}

fn severity_rank(severity: &str) -> usize {
    match severity.to_ascii_lowercase().as_str() {
        "high" => 3,
        "medium" => 2,
        "low" => 1,
        _ => 0,
    }
}
