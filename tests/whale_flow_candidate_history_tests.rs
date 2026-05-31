use btc_toxic_flow_monitor_rs::{
    toxicity::whale_flow_candidate_history_service::WhaleFlowCandidateHistoryService,
    types::{
        toxic_flow::{ToxicConfidence, ToxicSide},
        whale_flow_signal::{
            WhaleFlowBaselineQuality, WhaleFlowCandidate, WhaleFlowCandidateDiagnostics,
            WhaleFlowCandidateType, WhaleFlowDataQualitySummary, WhaleFlowRecentResponse,
            WhaleFlowThresholds, WhaleFlowVenueCoverage,
        },
    },
};

#[test]
fn whale_candidate_history_is_bounded_in_memory_and_deduped() {
    let service = WhaleFlowCandidateHistoryService::new(2);

    service.record_report(&report_with_candidates(vec![
        candidate("candidate-1", "BTC-PERP", 1_000),
        candidate("candidate-2", "BTC-PERP", 2_000),
    ]));
    service.record_report(&report_with_candidates(vec![
        candidate("candidate-2", "BTC-PERP", 3_000),
        candidate("candidate-3", "ETH-PERP", 4_000),
    ]));

    let snapshot = service.snapshot();
    assert_eq!(service.len(), 2);
    assert_eq!(snapshot.current_candidates, 2);
    assert_eq!(snapshot.max_candidates, 2);
    assert_eq!(snapshot.recorded_count, 4);
    assert_eq!(snapshot.deduplicated_count, 1);
    assert_eq!(snapshot.evicted_count, 1);
    let all = service.recent_candidates("ALL");
    assert_eq!(all[0].candidate_id, "candidate-3");
    assert_eq!(all[1].candidate_id, "candidate-2");
    assert_eq!(all[1].ts_ms, 3_000);
    assert!(service.recent_candidates("BTC-PERP").len() <= 1);
}

fn report_with_candidates(candidates: Vec<WhaleFlowCandidate>) -> WhaleFlowRecentResponse {
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
        connected_venues: 1,
        data_quality: WhaleFlowDataQualitySummary {
            status: "healthy".to_string(),
            venue_coverage_status: "healthy".to_string(),
            baseline_status: "healthy".to_string(),
            latest_trade_available: true,
            latest_book_available: true,
            operator_warning: None,
        },
        venue_coverage: WhaleFlowVenueCoverage {
            configured_venues: 1,
            enabled_venues: 1,
            connected_venues: 1,
            active_trade_venues: 1,
            active_book_venues: 1,
            venues_with_recent_trades: vec!["binance".to_string()],
            venues_with_recent_books: vec!["binance".to_string()],
            venues_missing_trades: Vec::new(),
            venues_missing_books: Vec::new(),
            min_venue_confluence_required: 1,
            venue_confluence_satisfied: true,
        },
        baseline_quality: WhaleFlowBaselineQuality {
            relative_volume_multiple: Some(6.0),
            baseline_source: "one_hour_normalized".to_string(),
            baseline_window_ms: Some(3_600_000),
            fallback_used: false,
            insufficient_history: false,
            operator_warning: None,
        },
        thresholds: WhaleFlowThresholds {
            one_second_btc: 100.0,
            five_second_btc: 300.0,
            fifteen_second_btc: 800.0,
            sixty_second_btc: 2_000.0,
            direction_ratio_min: 0.70,
            relative_volume_multiple_min: 5.0,
            min_venue_confirmations: 2,
        },
        warnings: Vec::new(),
        no_trade_reasons: Vec::new(),
        no_candidate_reasons: Vec::new(),
        degradation_warnings: Vec::new(),
        candidates,
    }
}

fn candidate(candidate_id: &str, symbol: &str, ts_ms: u64) -> WhaleFlowCandidate {
    WhaleFlowCandidate {
        candidate_id: candidate_id.to_string(),
        symbol: symbol.to_string(),
        ts_ms,
        window: "5s".to_string(),
        window_ms: 5_000,
        volume_btc: 350.0,
        gross_volume_btc: 420.0,
        direction: ToxicSide::Buy,
        direction_bias: 0.82,
        historical_volume_ratio: Some(6.0),
        historical_baseline_window_ms: Some(3_600_000),
        price_impact_bps: Some(2.1),
        depth_drop_ratio: Some(0.35),
        same_direction_venues: 2,
        candidate_type: WhaleFlowCandidateType::AggressiveBuy,
        toxicity_score: 80,
        confidence: ToxicConfidence::Medium,
        primary_reason: "bounded whale gate passed".to_string(),
        reason: vec!["test candidate".to_string()],
        linked_active_trade_signal_ids: Vec::new(),
        linked_liquidation_signal_ids: Vec::new(),
        linked_wall_candidate_ids: Vec::new(),
        linked_wall_interpretation_signal_ids: Vec::new(),
        linked_structural_signal_ids: Vec::new(),
        linked_fusion_signal_ids: Vec::new(),
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
