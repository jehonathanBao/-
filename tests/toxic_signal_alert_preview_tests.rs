use btc_toxic_flow_monitor_rs::{
    toxicity::toxic_signal_alert_preview::{
        build_toxic_signal_alert_explain, build_toxic_signal_alert_preview,
        build_toxic_signal_alert_preview_status,
    },
    types::{
        toxic_signal_alert_preview::ToxicSignalAlertPreviewGate,
        toxic_signal_inbox::{
            ToxicSignalInboxFusionSummary, ToxicSignalInboxGovernanceSummary, ToxicSignalInboxItem,
            ToxicSignalInboxMarkoutSummary, ToxicSignalInboxOperatorAction,
            ToxicSignalInboxQualitySummary, ToxicSignalInboxRecentResponse,
            ToxicSignalInboxRecommendationSummary, ToxicSignalInboxReplaySummary,
        },
    },
};

#[test]
fn toxic_signal_alert_preview_builds_notify_review_and_suppressed_items_without_mutation() {
    let recent = inbox_recent(vec![
        inbox_item(InboxItemSpec {
            signal_id: "signal-notify",
            symbol: "BTC-PERP",
            signal_kind: "short_bias_toxic_flow",
            severity: "high",
            confidence: 0.82,
            markout: ("aligned", "neutral", "neutral", "neutral"),
            quality_bucket: "good",
            recommendation_action: "keep",
            no_trade_only: false,
            governance_decision: "accept_recommendation",
            quality_available: true,
            recommendation_available: true,
        }),
        inbox_item(InboxItemSpec {
            signal_id: "signal-review",
            symbol: "BTC-PERP",
            signal_kind: "squeeze_risk_upside",
            severity: "medium",
            confidence: 0.62,
            markout: ("aligned", "neutral", "neutral", "neutral"),
            quality_bucket: "mixed",
            recommendation_action: "keep",
            no_trade_only: false,
            governance_decision: "missing_ledger_evidence",
            quality_available: true,
            recommendation_available: true,
        }),
        inbox_item(InboxItemSpec {
            signal_id: "signal-suppressed",
            symbol: "BTC-PERP",
            signal_kind: "trap_risk",
            severity: "high",
            confidence: 0.82,
            markout: ("neutral", "neutral", "neutral", "neutral"),
            quality_bucket: "good",
            recommendation_action: "no_trade_only_candidate",
            no_trade_only: true,
            governance_decision: "suppress_for_now",
            quality_available: true,
            recommendation_available: true,
        }),
        inbox_item(InboxItemSpec {
            signal_id: "signal-data",
            symbol: "BTC-PERP",
            signal_kind: "long_bias_toxic_flow",
            severity: "high",
            confidence: 0.82,
            markout: ("aligned", "not_enough_data", "neutral", "neutral"),
            quality_bucket: "good",
            recommendation_action: "keep",
            no_trade_only: false,
            governance_decision: "accept_recommendation",
            quality_available: true,
            recommendation_available: true,
        }),
    ]);
    let recent_before = serde_json::to_value(&recent).expect("serialize recent before");

    let preview = build_toxic_signal_alert_preview("BTC-PERP", &recent, gate());
    let status = build_toxic_signal_alert_preview_status(&preview);

    assert!(preview.read_only);
    assert!(preview.analysis_only);
    assert!(!preview.execution_enabled);
    assert!(!preview.notification_sent);
    assert!(!preview.execution_triggered);
    assert!(preview.preview_only);
    assert_eq!(preview.filter.symbol, "BTC-PERP");
    assert!(preview.filter.view_only);
    assert_eq!(preview.summary.total_signals, 4);
    assert_eq!(preview.summary.notify_candidates, 1);
    assert_eq!(preview.summary.review_candidates, 1);
    assert_eq!(preview.summary.suppressed_signals, 2);
    assert_eq!(preview.summary.no_trade_only_signals, 1);
    assert_eq!(preview.summary.governance_hold_signals, 0);
    assert_eq!(preview.summary.not_enough_data_signals, 1);
    assert_eq!(preview.items[0].preview_status, "notify_candidate");
    assert_eq!(preview.items[1].preview_status, "review_candidate");
    assert_eq!(preview.items[2].preview_status, "suppressed_no_trade_only");
    assert_eq!(preview.items[3].preview_status, "not_enough_data");
    assert!(preview
        .markdown
        .contains("# Signal Alert Rules / Notification Preview"));
    assert!(preview.markdown.contains("Notification sent: false"));
    assert!(preview.markdown.contains("No webhook"));
    assert!(preview.markdown.contains("No live trading"));

    assert!(status.read_only);
    assert!(status.analysis_only);
    assert!(!status.execution_enabled);
    assert!(!status.notification_sent);
    assert!(!status.execution_triggered);
    assert!(status.preview_only);
    assert_eq!(status.total_signals, 4);
    assert_eq!(status.notify_candidate_count, 1);
    assert_eq!(status.suppressed_count, 2);

    assert_eq!(
        serde_json::to_value(&recent).expect("serialize recent after"),
        recent_before
    );
}

#[test]
fn toxic_signal_alert_preview_marks_governance_hold_and_empty_preview_honestly() {
    let held = inbox_recent(vec![inbox_item(InboxItemSpec {
        signal_id: "signal-held",
        symbol: "BTC-PERP",
        signal_kind: "short_bias_toxic_flow",
        severity: "high",
        confidence: 0.82,
        markout: ("aligned", "neutral", "neutral", "neutral"),
        quality_bucket: "good",
        recommendation_action: "keep",
        no_trade_only: false,
        governance_decision: "suppress_for_now",
        quality_available: true,
        recommendation_available: true,
    })]);
    let empty = inbox_recent(Vec::new());

    let held_preview = build_toxic_signal_alert_preview("BTC-PERP", &held, gate());
    let empty_preview = build_toxic_signal_alert_preview("BTC-PERP", &empty, gate());

    assert_eq!(
        held_preview.items[0].preview_status,
        "suppressed_governance_hold"
    );
    assert_eq!(held_preview.summary.governance_hold_signals, 1);
    assert_eq!(empty_preview.status, "empty_notification_preview");
    assert!(empty_preview.items.is_empty());
    assert_eq!(empty_preview.summary.total_signals, 0);
}

#[test]
fn toxic_signal_alert_explain_breaks_down_notify_and_missing_inputs_honestly() {
    let recent = inbox_recent(vec![
        inbox_item(InboxItemSpec {
            signal_id: "signal-notify",
            symbol: "BTC-PERP",
            signal_kind: "short_bias_toxic_flow",
            severity: "high",
            confidence: 0.82,
            markout: ("aligned", "neutral", "neutral", "neutral"),
            quality_bucket: "good",
            recommendation_action: "keep",
            no_trade_only: false,
            governance_decision: "accept_recommendation",
            quality_available: true,
            recommendation_available: true,
        }),
        inbox_item(InboxItemSpec {
            signal_id: "signal-data",
            symbol: "BTC-PERP",
            signal_kind: "long_bias_toxic_flow",
            severity: "high",
            confidence: 0.82,
            markout: ("aligned", "not_enough_data", "neutral", "neutral"),
            quality_bucket: "not_enough_data",
            recommendation_action: "insufficient_data",
            no_trade_only: false,
            governance_decision: "missing_ledger_evidence",
            quality_available: false,
            recommendation_available: false,
        }),
    ]);

    let notify = build_toxic_signal_alert_explain("signal-notify", &recent, &gate());
    let missing = build_toxic_signal_alert_explain("signal-data", &recent, &gate());
    let absent = build_toxic_signal_alert_explain("missing", &recent, &gate());

    assert!(notify.found);
    assert_eq!(notify.alert_decision, "notify_candidate");
    assert!(notify
        .decision_reasons
        .iter()
        .any(|reason| reason == "severity is high"));
    assert!(notify
        .decision_reasons
        .iter()
        .any(|reason| reason == "confidence is above notify threshold"));
    assert!(notify
        .decision_reasons
        .iter()
        .any(|reason| reason == "quality bucket is good"));
    assert!(notify.suppression_reasons.is_empty());
    assert!(notify.missing_inputs.is_empty());
    assert_eq!(
        notify.operator_note,
        "Preview only. No notification was sent."
    );

    assert!(missing.found);
    assert_eq!(missing.alert_decision, "not_enough_data");
    assert!(missing
        .decision_reasons
        .iter()
        .any(|reason| reason == "preview stopped because required evidence is incomplete"));
    assert!(missing
        .suppression_reasons
        .iter()
        .any(|reason| reason.contains("not_enough_data")));
    assert!(missing
        .missing_inputs
        .iter()
        .any(|reason| reason == "markout +5m is not_enough_data"));
    assert!(missing
        .missing_inputs
        .iter()
        .any(|reason| reason == "quality summary unavailable"));
    assert!(missing
        .missing_inputs
        .iter()
        .any(|reason| reason == "recommendation summary unavailable"));
    assert!(missing
        .missing_inputs
        .iter()
        .any(|reason| reason == "governance ledger unavailable"));

    assert!(!absent.found);
    assert_eq!(absent.alert_decision, "not_found");
    assert_eq!(
        absent.reason.as_deref(),
        Some("signal_id_not_found_in_alert_preview")
    );
}

fn gate() -> ToxicSignalAlertPreviewGate {
    ToxicSignalAlertPreviewGate {
        dedup_window_ms: 30_000,
        min_severity: "ALERT".to_string(),
        require_cross_venue: true,
        require_markout: true,
        require_liquidity_drain: false,
        telegram_enabled: false,
        notification_sent: false,
        execution_triggered: false,
    }
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
        selected_symbol: "BTC-PERP".to_string(),
        status: if items.is_empty() {
            "empty_signal_inbox".to_string()
        } else {
            "signal_inbox_ready".to_string()
        },
        warnings: Vec::new(),
        items,
    }
}

struct InboxItemSpec<'a> {
    signal_id: &'a str,
    symbol: &'a str,
    signal_kind: &'a str,
    severity: &'a str,
    confidence: f64,
    markout: (&'a str, &'a str, &'a str, &'a str),
    quality_bucket: &'a str,
    recommendation_action: &'a str,
    no_trade_only: bool,
    governance_decision: &'a str,
    quality_available: bool,
    recommendation_available: bool,
}

fn inbox_item(spec: InboxItemSpec<'_>) -> ToxicSignalInboxItem {
    ToxicSignalInboxItem {
        signal_id: spec.signal_id.to_string(),
        symbol: spec.symbol.to_string(),
        signal_kind: spec.signal_kind.to_string(),
        direction_bias: "short_bias".to_string(),
        severity: spec.severity.to_string(),
        risk_score: 82,
        data_quality_score: Some(82.0),
        confidence: spec.confidence,
        created_at_ms: 1_000,
        fusion: ToxicSignalInboxFusionSummary {
            available: true,
            summary: "summary".to_string(),
        },
        replay: ToxicSignalInboxReplaySummary {
            available: true,
            evidence_count: 2,
        },
        markout: ToxicSignalInboxMarkoutSummary {
            available: true,
            one_minute: spec.markout.0.to_string(),
            five_minute: spec.markout.1.to_string(),
            fifteen_minute: spec.markout.2.to_string(),
            one_hour: spec.markout.3.to_string(),
        },
        quality: ToxicSignalInboxQualitySummary {
            available: spec.quality_available,
            quality_bucket: spec.quality_bucket.to_string(),
            aligned_ratio: 0.6,
            adverse_ratio: 0.2,
        },
        recommendation: ToxicSignalInboxRecommendationSummary {
            available: spec.recommendation_available,
            action: spec.recommendation_action.to_string(),
            no_trade_only: spec.no_trade_only,
            manual_review_required: true,
        },
        governance: ToxicSignalInboxGovernanceSummary {
            ledger_available: spec.governance_decision != "missing_ledger_evidence",
            latest_decision: spec.governance_decision.to_string(),
        },
        operator_action: if spec.no_trade_only {
            ToxicSignalInboxOperatorAction::NoTradeWarning
        } else {
            ToxicSignalInboxOperatorAction::ReviewEvidence
        },
        read_only: true,
        runtime_modified: false,
        analysis_only: true,
        execution_enabled: false,
    }
}
