use btc_toxic_flow_monitor_rs::{
    toxicity::whale_flow_calibration_service::build_whale_flow_threshold_calibration_report,
    types::{
        toxic_flow::{ToxicConfidence, ToxicSide},
        toxic_markout::{ToxicMarkoutOutcome, ToxicMarkoutRecentResponse, ToxicMarkoutSignal},
        toxic_signal_history::ToxicSignalHistoryStatusResponse,
        whale_flow_signal::{
            WhaleFlowBaselineQuality, WhaleFlowCandidate, WhaleFlowCandidateDiagnostics,
            WhaleFlowCandidateType, WhaleFlowDataQualitySummary, WhaleFlowRecentResponse,
            WhaleFlowThresholds, WhaleFlowVenueCoverage,
        },
    },
};

#[test]
fn calibration_report_groups_thresholds_and_requires_more_data_below_sample_floor() {
    let report = build_whale_flow_threshold_calibration_report(
        "BTC-PERP",
        &sample_whale_flow_recent(),
        &sample_markout_recent(),
        &sample_history_status(),
    );

    assert!(report.read_only);
    assert!(report.analysis_only);
    assert!(!report.execution_enabled);
    assert_eq!(report.status, "current_snapshot_only");
    assert_eq!(report.sample_status.total_candidates, 4);
    assert_eq!(report.sample_status.linked_markout_samples, 3);
    assert_eq!(report.sample_status.resolved_markout_evidence_count, 3);
    assert_eq!(report.sample_status.unresolved_markout_count, 1);
    assert!(!report.sample_status.enough_data);
    assert_eq!(
        report.sample_status.blocked_reason.as_deref(),
        Some("current_snapshot_only")
    );
    assert!(report.evidence_source.uses_current_snapshot_only);
    assert!(report.evidence_source.current_snapshot_fallback_used);
    assert_eq!(report.sample_status.retention_mode, "in_memory_bounded");

    assert_eq!(
        report.threshold_performance.one_second_btc.candidate_count,
        1
    );
    assert_eq!(
        report
            .threshold_performance
            .one_second_btc
            .not_enough_data_rate,
        1.0
    );
    assert_eq!(
        report.threshold_performance.five_second_btc.aligned_rate,
        1.0
    );
    assert_eq!(
        report.threshold_performance.fifteen_second_btc.adverse_rate,
        1.0
    );
    assert_eq!(
        report.threshold_performance.sixty_second_btc.neutral_rate,
        1.0
    );

    let aggressive_buy = report
        .by_classification
        .iter()
        .find(|item| item.classification == "aggressive_buy")
        .expect("aggressive buy bucket");
    assert_eq!(aggressive_buy.sample_count, 1);
    assert_eq!(aggressive_buy.aligned_rate, 1.0);

    let absorption = report
        .by_classification
        .iter()
        .find(|item| item.classification == "absorption")
        .expect("absorption bucket");
    assert_eq!(absorption.sample_count, 1);
    assert_eq!(absorption.adverse_rate, 1.0);

    let one_hour = report
        .baseline_source_quality
        .iter()
        .find(|item| item.baseline_source == "one_hour_normalized")
        .expect("one hour bucket");
    assert_eq!(one_hour.sample_count, 2);

    let fallback = report
        .baseline_source_quality
        .iter()
        .find(|item| item.baseline_source == "sixty_second_fallback")
        .expect("sixty-second fallback bucket");
    assert_eq!(fallback.sample_count, 1);

    assert!(report
        .manual_tuning_notes
        .iter()
        .all(|note| note.suggested_action == "needs_more_data"));
    assert!(report
        .warnings
        .iter()
        .any(|warning| warning.contains("Calibration evidence too thin")));
    assert!(report.markdown.contains("No order placement"));
    assert!(report.markdown.contains("No wallet/signing"));
    assert!(report.markdown.contains("No live trading"));
}

#[test]
fn calibration_report_blocks_tuning_when_only_current_snapshot_is_available() {
    let whale_flow = dense_whale_flow_recent();
    let markout = dense_markout_recent();
    let report = build_whale_flow_threshold_calibration_report(
        "BTC-PERP",
        &whale_flow,
        &markout,
        &sample_history_status(),
    );

    assert_eq!(report.status, "current_snapshot_only");
    assert!(!report.sample_status.enough_data);
    assert_eq!(report.sample_status.total_candidates, 20);
    assert_eq!(report.sample_status.linked_markout_samples, 20);
    assert_eq!(report.sample_status.resolved_markout_evidence_count, 20);
    assert_eq!(
        report.sample_status.blocked_reason.as_deref(),
        Some("current_snapshot_only")
    );
    assert!(report.evidence_source.uses_current_snapshot_only);
    assert!(report.evidence_source.current_snapshot_fallback_used);

    let five_second = &report.threshold_performance.five_second_btc;
    assert_eq!(five_second.candidate_count, 20);
    assert!(five_second.aligned_rate > five_second.adverse_rate);
    assert_eq!(five_second.verdict, "keep");

    assert!(report
        .manual_tuning_notes
        .iter()
        .all(|note| note.suggested_action == "needs_more_data"));
}

#[test]
fn aggressive_buy_without_linked_signal_id_does_not_fake_keep() {
    let mut whale_flow = dense_whale_flow_recent();
    for candidate in &mut whale_flow.candidates {
        candidate.linked_fusion_signal_ids.clear();
    }
    let markout = ToxicMarkoutRecentResponse {
        signals: Vec::new(),
        ..dense_markout_recent()
    };

    let report = build_whale_flow_threshold_calibration_report(
        "BTC-PERP",
        &whale_flow,
        &markout,
        &sample_history_status(),
    );

    assert_eq!(report.outcome_linkage.no_outcome_linkage_count, 20);
    assert_eq!(report.sample_status.resolved_markout_evidence_count, 0);
    assert!(!report.sample_status.enough_data);
    assert!(report
        .manual_tuning_notes
        .iter()
        .all(|note| note.suggested_action == "needs_more_data"));
}

#[test]
fn aggressive_buy_without_linked_signal_id_can_use_bounded_fallback_evidence() {
    let mut whale_flow = dense_whale_flow_recent();
    for candidate in &mut whale_flow.candidates {
        candidate.linked_fusion_signal_ids.clear();
    }

    let report = build_whale_flow_threshold_calibration_report(
        "BTC-PERP",
        &whale_flow,
        &dense_markout_recent(),
        &sample_history_status(),
    );

    assert!(report.outcome_linkage.fallback_used);
    assert_eq!(report.outcome_linkage.fallback_matches, 20);
    assert_eq!(report.sample_status.resolved_markout_evidence_count, 20);
    assert!(!report.sample_status.enough_data);
    assert!(report
        .manual_tuning_notes
        .iter()
        .all(|note| note.suggested_action == "needs_more_data"));
}

#[test]
fn candidate_count_enough_but_markout_evidence_thin_blocks_tuning() {
    let mut whale_flow = dense_whale_flow_recent();
    whale_flow.candidates = (0..25)
        .map(|index| {
            let linked_ids = if index < 2 {
                vec![format!("dense-sig-{index}")]
            } else {
                vec![format!("missing-sig-{index}")]
            };
            calibration_candidate(
                &format!("thin-{index}"),
                5_000,
                WhaleFlowCandidateType::AggressiveBuy,
                2,
                Some(3_600_000),
                linked_ids,
            )
        })
        .collect();

    let report = build_whale_flow_threshold_calibration_report(
        "BTC-PERP",
        &whale_flow,
        &dense_markout_recent(),
        &sample_history_status(),
    );

    assert_eq!(report.sample_status.total_candidates, 25);
    assert_eq!(report.sample_status.resolved_markout_evidence_count, 2);
    assert_eq!(report.sample_status.unresolved_markout_count, 23);
    assert!(!report.sample_status.enough_data);
    assert!(report.sample_status.not_enough_data_rate > 0.50);
    assert!(report
        .manual_tuning_notes
        .iter()
        .all(|note| note.suggested_action == "needs_more_data"));
}

fn sample_whale_flow_recent() -> WhaleFlowRecentResponse {
    WhaleFlowRecentResponse {
        read_only: true,
        runtime_modified: false,
        analysis_only: true,
        execution_enabled: false,
        mode: "analysis_only".to_string(),
        selected_symbol: "BTC-PERP".to_string(),
        status: "candidate_active".to_string(),
        history_baseline_mode: "one_hour_normalized".to_string(),
        lagged_events: 0,
        dropped_events: 0,
        flow_windows_populated: true,
        connected_venues: 2,
        data_quality: sample_data_quality(),
        venue_coverage: sample_venue_coverage(),
        baseline_quality: sample_baseline_quality(),
        thresholds: sample_thresholds(),
        warnings: Vec::new(),
        no_trade_reasons: Vec::new(),
        no_candidate_reasons: Vec::new(),
        degradation_warnings: Vec::new(),
        candidates: vec![
            calibration_candidate(
                "candidate-1",
                1_000,
                WhaleFlowCandidateType::AggressiveSell,
                2,
                Some(3_600_000),
                Vec::new(),
            ),
            calibration_candidate(
                "candidate-2",
                5_000,
                WhaleFlowCandidateType::AggressiveBuy,
                2,
                Some(3_600_000),
                vec!["sig-1".to_string()],
            ),
            calibration_candidate(
                "candidate-3",
                15_000,
                WhaleFlowCandidateType::Absorption,
                3,
                Some(60_000),
                vec!["sig-2".to_string()],
            ),
            calibration_candidate(
                "candidate-4",
                60_000,
                WhaleFlowCandidateType::Trap,
                2,
                None,
                vec!["sig-3".to_string()],
            ),
        ],
    }
}

fn dense_whale_flow_recent() -> WhaleFlowRecentResponse {
    let mut report = sample_whale_flow_recent();
    report.candidates = (0..20)
        .map(|index| {
            calibration_candidate(
                &format!("dense-{index}"),
                5_000,
                WhaleFlowCandidateType::AggressiveBuy,
                2,
                Some(3_600_000),
                vec![format!("dense-sig-{index}")],
            )
        })
        .collect();
    report
}

fn calibration_candidate(
    candidate_id: &str,
    window_ms: u64,
    candidate_type: WhaleFlowCandidateType,
    same_direction_venues: usize,
    historical_baseline_window_ms: Option<u64>,
    linked_fusion_signal_ids: Vec<String>,
) -> WhaleFlowCandidate {
    WhaleFlowCandidate {
        candidate_id: candidate_id.to_string(),
        symbol: "BTC-PERP".to_string(),
        ts_ms: 10_000 + window_ms,
        window: match window_ms {
            1_000 => "1s",
            5_000 => "5s",
            15_000 => "15s",
            60_000 => "60s",
            _ => "custom",
        }
        .to_string(),
        window_ms,
        volume_btc: 350.0,
        gross_volume_btc: 420.0,
        direction: ToxicSide::Buy,
        direction_bias: 0.82,
        historical_volume_ratio: Some(6.0),
        historical_baseline_window_ms,
        price_impact_bps: Some(2.1),
        depth_drop_ratio: Some(0.35),
        same_direction_venues,
        candidate_type,
        toxicity_score: 80,
        confidence: ToxicConfidence::Medium,
        primary_reason: "bounded whale gate passed".to_string(),
        reason: vec!["test candidate".to_string()],
        linked_active_trade_signal_ids: Vec::new(),
        linked_liquidation_signal_ids: Vec::new(),
        linked_wall_candidate_ids: Vec::new(),
        linked_wall_interpretation_signal_ids: Vec::new(),
        linked_structural_signal_ids: Vec::new(),
        linked_fusion_signal_ids,
        diagnostics: WhaleFlowCandidateDiagnostics {
            data_quality: "healthy".to_string(),
            why_candidate: vec!["test".to_string()],
            missing_inputs: Vec::new(),
            degradation_reasons: Vec::new(),
            confidence_modifiers: Vec::new(),
        },
        read_only: true,
    }
}

fn sample_markout_recent() -> ToxicMarkoutRecentResponse {
    ToxicMarkoutRecentResponse {
        read_only: true,
        runtime_modified: false,
        analysis_only: true,
        execution_enabled: false,
        mode: "analysis_only".to_string(),
        selected_symbol: "BTC-PERP".to_string(),
        status: "markout_ready".to_string(),
        warnings: Vec::new(),
        signals: vec![
            calibration_markout("sig-1", ToxicMarkoutOutcome::Aligned),
            calibration_markout("sig-2", ToxicMarkoutOutcome::Adverse),
            calibration_markout("sig-3", ToxicMarkoutOutcome::Neutral),
        ],
    }
}

fn dense_markout_recent() -> ToxicMarkoutRecentResponse {
    ToxicMarkoutRecentResponse {
        read_only: true,
        runtime_modified: false,
        analysis_only: true,
        execution_enabled: false,
        mode: "analysis_only".to_string(),
        selected_symbol: "BTC-PERP".to_string(),
        status: "markout_ready".to_string(),
        warnings: Vec::new(),
        signals: (0..20)
            .map(|index| {
                calibration_markout(
                    &format!("dense-sig-{index}"),
                    if index < 12 {
                        ToxicMarkoutOutcome::Aligned
                    } else if index < 16 {
                        ToxicMarkoutOutcome::Adverse
                    } else {
                        ToxicMarkoutOutcome::Neutral
                    },
                )
            })
            .collect(),
    }
}

fn calibration_markout(
    signal_id: &str,
    overall_outcome: ToxicMarkoutOutcome,
) -> ToxicMarkoutSignal {
    ToxicMarkoutSignal {
        signal_id: signal_id.to_string(),
        symbol: "BTC-PERP".to_string(),
        signal_kind: "trap_risk".to_string(),
        direction: "LONG_BIAS".to_string(),
        toxicity_score: 80,
        confidence: "MEDIUM".to_string(),
        created_at_ms: 15_000,
        overall_outcome,
        aligned_windows: usize::from(matches!(overall_outcome, ToxicMarkoutOutcome::Aligned)),
        adverse_windows: usize::from(matches!(overall_outcome, ToxicMarkoutOutcome::Adverse)),
        neutral_windows: usize::from(matches!(overall_outcome, ToxicMarkoutOutcome::Neutral)),
        missing_windows: usize::from(matches!(
            overall_outcome,
            ToxicMarkoutOutcome::NotEnoughData
        )),
        windows: Vec::new(),
        no_trade_reasons: Vec::new(),
        read_only: true,
    }
}

fn sample_history_status() -> ToxicSignalHistoryStatusResponse {
    ToxicSignalHistoryStatusResponse {
        read_only: true,
        runtime_modified: false,
        analysis_only: true,
        execution_enabled: false,
        retention_mode: "in_memory_bounded".to_string(),
        durable_storage_enabled: false,
        database_write_enabled: false,
        max_signals: 1000,
        max_groups: 300,
        max_alerts: 300,
        max_reports: 30,
        current_signals: 12,
        current_groups: 3,
        current_alerts: 0,
        current_reports: 1,
        safety_boundary: Vec::new(),
    }
}

fn sample_thresholds() -> WhaleFlowThresholds {
    WhaleFlowThresholds {
        one_second_btc: 100.0,
        five_second_btc: 300.0,
        fifteen_second_btc: 800.0,
        sixty_second_btc: 2_000.0,
        direction_ratio_min: 0.70,
        relative_volume_multiple_min: 5.0,
        min_venue_confirmations: 2,
    }
}

fn sample_data_quality() -> WhaleFlowDataQualitySummary {
    WhaleFlowDataQualitySummary {
        status: "healthy".to_string(),
        venue_coverage_status: "healthy".to_string(),
        baseline_status: "healthy".to_string(),
        latest_trade_available: true,
        latest_book_available: true,
        operator_warning: None,
    }
}

fn sample_venue_coverage() -> WhaleFlowVenueCoverage {
    WhaleFlowVenueCoverage {
        configured_venues: 3,
        enabled_venues: 2,
        connected_venues: 2,
        active_trade_venues: 2,
        active_book_venues: 2,
        venues_with_recent_trades: vec!["binance".to_string(), "bybit".to_string()],
        venues_with_recent_books: vec!["binance".to_string(), "bybit".to_string()],
        venues_missing_trades: Vec::new(),
        venues_missing_books: Vec::new(),
        min_venue_confluence_required: 2,
        venue_confluence_satisfied: true,
    }
}

fn sample_baseline_quality() -> WhaleFlowBaselineQuality {
    WhaleFlowBaselineQuality {
        relative_volume_multiple: Some(6.2),
        baseline_source: "one_hour_normalized".to_string(),
        baseline_window_ms: Some(3_600_000),
        fallback_used: false,
        insufficient_history: false,
        operator_warning: None,
    }
}
