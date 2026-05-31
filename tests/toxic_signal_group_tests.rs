use btc_toxic_flow_monitor_rs::{
    toxicity::toxic_signal_group::{
        build_toxic_signal_group_detail, build_toxic_signal_group_recent,
        build_toxic_signal_group_status,
    },
    types::toxic_signal_group::ToxicSignalGroupOperatorAction,
    types::toxic_signal_inbox::{
        ToxicSignalInboxFusionSummary, ToxicSignalInboxGovernanceSummary, ToxicSignalInboxItem,
        ToxicSignalInboxMarkoutSummary, ToxicSignalInboxOperatorAction,
        ToxicSignalInboxQualitySummary, ToxicSignalInboxRecentResponse,
        ToxicSignalInboxRecommendationSummary, ToxicSignalInboxReplaySummary,
    },
};

#[test]
fn same_symbol_kind_direction_within_cooldown_are_grouped() {
    let inbox = inbox_recent(vec![
        inbox_item(
            "signal-a",
            "BTCUSDT",
            "short_bias_toxic_flow",
            "short",
            "medium",
            0.60,
            1_000,
        ),
        inbox_item(
            "signal-b",
            "BTCUSDT",
            "short_bias_toxic_flow",
            "short",
            "high",
            0.80,
            120_000,
        ),
        inbox_item(
            "signal-c",
            "BTCUSDT",
            "short_bias_toxic_flow",
            "short",
            "low",
            0.40,
            240_000,
        ),
    ]);
    let before = serde_json::to_value(&inbox).expect("before inbox json");
    let recent = build_toxic_signal_group_recent("BTCUSDT", &inbox);
    let status = build_toxic_signal_group_status(&recent);
    let after = serde_json::to_value(&inbox).expect("after inbox json");

    assert_eq!(before, after);
    assert_eq!(recent.groups.len(), 1);
    assert!(recent.read_only);
    assert!(!recent.runtime_modified);
    assert!(recent.analysis_only);
    assert!(!recent.execution_enabled);
    assert_eq!(recent.groups[0].count, 3);
    assert_eq!(recent.groups[0].representative_signal_id, "signal-b");
    assert_eq!(recent.groups[0].max_severity, "high");
    assert_eq!(recent.groups[0].avg_confidence, 0.60);
    assert!(recent.groups[0].original_signals_preserved);
    assert_eq!(
        recent.groups[0].operator_action,
        ToxicSignalGroupOperatorAction::ReviewGroupedSignal
    );
    assert_eq!(status.group_count, 1);
}

#[test]
fn signals_outside_cooldown_do_not_group() {
    let inbox = inbox_recent(vec![
        inbox_item(
            "signal-a",
            "BTCUSDT",
            "short_bias_toxic_flow",
            "short",
            "medium",
            0.60,
            1_000,
        ),
        inbox_item(
            "signal-b",
            "BTCUSDT",
            "short_bias_toxic_flow",
            "short",
            "high",
            0.80,
            310_001,
        ),
    ]);
    let recent = build_toxic_signal_group_recent("BTCUSDT", &inbox);

    assert_eq!(recent.groups.len(), 2);
}

#[test]
fn different_symbol_kind_or_direction_do_not_group() {
    let inbox = inbox_recent(vec![
        inbox_item(
            "signal-a",
            "BTCUSDT",
            "short_bias_toxic_flow",
            "short",
            "medium",
            0.60,
            1_000,
        ),
        inbox_item(
            "signal-b",
            "ETHUSDT",
            "short_bias_toxic_flow",
            "short",
            "medium",
            0.61,
            2_000,
        ),
        inbox_item(
            "signal-c",
            "BTCUSDT",
            "long_bias_toxic_flow",
            "long",
            "medium",
            0.62,
            3_000,
        ),
        inbox_item(
            "signal-d",
            "BTCUSDT",
            "short_bias_toxic_flow",
            "neutral",
            "medium",
            0.63,
            4_000,
        ),
    ]);
    let recent = build_toxic_signal_group_recent("BTCUSDT", &inbox);

    assert_eq!(recent.groups.len(), 4);
}

#[test]
fn no_trade_and_needs_more_data_actions_are_promoted() {
    let no_trade_recent = build_toxic_signal_group_recent(
        "BTCUSDT",
        &inbox_recent(vec![
            inbox_item_with_action(
                "signal-a",
                "BTCUSDT",
                "trap_risk",
                "neutral",
                "medium",
                0.50,
                1_000,
                ToxicSignalInboxOperatorAction::NoTradeWarning,
            ),
            inbox_item_with_action(
                "signal-b",
                "BTCUSDT",
                "trap_risk",
                "neutral",
                "low",
                0.40,
                2_000,
                ToxicSignalInboxOperatorAction::WatchSignalOnly,
            ),
        ]),
    );
    let needs_more_data_recent = build_toxic_signal_group_recent(
        "BTCUSDT",
        &inbox_recent(vec![
            inbox_item_with_action(
                "signal-c",
                "BTCUSDT",
                "short_bias_toxic_flow",
                "short",
                "low",
                0.20,
                1_000,
                ToxicSignalInboxOperatorAction::NeedsMoreData,
            ),
            inbox_item_with_action(
                "signal-d",
                "BTCUSDT",
                "short_bias_toxic_flow",
                "short",
                "medium",
                0.50,
                2_000,
                ToxicSignalInboxOperatorAction::ReviewMarkout,
            ),
        ]),
    );

    assert_eq!(
        no_trade_recent.groups[0].operator_action,
        ToxicSignalGroupOperatorAction::NoTradeWarningGroup
    );
    assert_eq!(
        needs_more_data_recent.groups[0].operator_action,
        ToxicSignalGroupOperatorAction::NeedsMoreData
    );
}

#[test]
fn empty_groups_and_detail_lookup_degrade_cleanly() {
    let recent = build_toxic_signal_group_recent("BTCUSDT", &inbox_recent(Vec::new()));
    let detail = build_toxic_signal_group_detail("BTCUSDT", "missing-group", &recent);

    assert_eq!(recent.status, "empty_signal_groups");
    assert!(recent.groups.is_empty());
    assert!(!detail.available);
    assert_eq!(
        detail.reason.as_deref(),
        Some("group_id_not_found_in_read_only_signal_groups")
    );
}

fn inbox_recent(items: Vec<ToxicSignalInboxItem>) -> ToxicSignalInboxRecentResponse {
    ToxicSignalInboxRecentResponse {
        read_only: true,
        runtime_modified: false,
        analysis_only: true,
        execution_enabled: false,
        manual_review_required: true,
        runtime_weight_modified: false,
        config_modified: false,
        mode: "analysis_only".to_string(),
        selected_symbol: "BTCUSDT".to_string(),
        status: "signal_inbox_ready".to_string(),
        warnings: Vec::new(),
        items,
    }
}

fn inbox_item(
    signal_id: &str,
    symbol: &str,
    signal_kind: &str,
    direction_bias: &str,
    severity: &str,
    confidence: f64,
    created_at_ms: u64,
) -> ToxicSignalInboxItem {
    inbox_item_with_action(
        signal_id,
        symbol,
        signal_kind,
        direction_bias,
        severity,
        confidence,
        created_at_ms,
        ToxicSignalInboxOperatorAction::WatchSignalOnly,
    )
}

#[allow(clippy::too_many_arguments)]
fn inbox_item_with_action(
    signal_id: &str,
    symbol: &str,
    signal_kind: &str,
    direction_bias: &str,
    severity: &str,
    confidence: f64,
    created_at_ms: u64,
    operator_action: ToxicSignalInboxOperatorAction,
) -> ToxicSignalInboxItem {
    ToxicSignalInboxItem {
        signal_id: signal_id.to_string(),
        symbol: symbol.to_string(),
        signal_kind: signal_kind.to_string(),
        direction_bias: direction_bias.to_string(),
        severity: severity.to_string(),
        confidence,
        created_at_ms,
        fusion: ToxicSignalInboxFusionSummary {
            available: true,
            summary: "summary".to_string(),
        },
        replay: ToxicSignalInboxReplaySummary {
            available: true,
            evidence_count: 1,
        },
        markout: ToxicSignalInboxMarkoutSummary {
            available: true,
            one_minute: "aligned".to_string(),
            five_minute: "aligned".to_string(),
            fifteen_minute: "aligned".to_string(),
            one_hour: "aligned".to_string(),
        },
        quality: ToxicSignalInboxQualitySummary {
            available: true,
            quality_bucket: "good".to_string(),
            aligned_ratio: 0.60,
            adverse_ratio: 0.10,
        },
        recommendation: ToxicSignalInboxRecommendationSummary {
            available: true,
            action: "keep".to_string(),
            no_trade_only: matches!(
                operator_action,
                ToxicSignalInboxOperatorAction::NoTradeWarning
            ),
            manual_review_required: true,
        },
        governance: ToxicSignalInboxGovernanceSummary {
            ledger_available: true,
            latest_decision: "watch_more".to_string(),
        },
        operator_action,
        read_only: true,
        runtime_modified: false,
        analysis_only: true,
        execution_enabled: false,
    }
}
