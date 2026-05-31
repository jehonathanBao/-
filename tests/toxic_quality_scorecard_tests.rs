use btc_toxic_flow_monitor_rs::{
    toxicity::toxic_quality_scorecard::{
        build_toxic_quality_scorecard_status, build_toxic_quality_scorecard_summary,
    },
    types::toxic_markout::{
        ToxicMarkoutOutcome, ToxicMarkoutRecentResponse, ToxicMarkoutSignal, ToxicMarkoutWindow,
    },
};

#[test]
fn quality_scorecard_aggregates_signal_types_and_windows() {
    let recent = ToxicMarkoutRecentResponse {
        read_only: true,
        runtime_modified: false,
        analysis_only: true,
        execution_enabled: false,
        mode: "analysis_only".to_string(),
        selected_symbol: "BTC-PERP".to_string(),
        status: "markout_ready".to_string(),
        warnings: Vec::new(),
        signals: vec![
            signal(
                "short_bias_toxic_flow",
                ToxicMarkoutOutcome::Adverse,
                vec![
                    window("+1m", ToxicMarkoutOutcome::Adverse),
                    window("+5m", ToxicMarkoutOutcome::Aligned),
                ],
                vec!["follow-through broke".to_string()],
            ),
            signal(
                "short_bias_toxic_flow",
                ToxicMarkoutOutcome::Adverse,
                vec![
                    window("+1m", ToxicMarkoutOutcome::Adverse),
                    window("+5m", ToxicMarkoutOutcome::Neutral),
                ],
                vec!["follow-through broke".to_string()],
            ),
            signal(
                "trap_risk",
                ToxicMarkoutOutcome::Neutral,
                vec![
                    window("+1m", ToxicMarkoutOutcome::Neutral),
                    window("+5m", ToxicMarkoutOutcome::NotEnoughData),
                ],
                vec!["trap context only".to_string()],
            ),
        ],
    };

    let summary = build_toxic_quality_scorecard_summary(&recent);
    let status = build_toxic_quality_scorecard_status(&summary);

    assert!(summary.read_only);
    assert!(!summary.runtime_modified);
    assert_eq!(summary.mode, "analysis_only");
    assert_eq!(summary.total_evaluations, 3);
    assert_eq!(summary.adverse_ratio, 0.6667);
    assert_eq!(summary.neutral_ratio, 0.3333);
    assert_eq!(summary.by_signal_type.len(), 2);
    assert_eq!(summary.by_window.len(), 2);
    assert_eq!(summary.by_symbol.len(), 1);

    let short_bias = summary
        .by_signal_type
        .iter()
        .find(|bucket| bucket.key == "short_bias_toxic_flow")
        .expect("short bias bucket");
    assert!(short_bias.downgrade_candidate);
    assert!(!short_bias.no_trade_candidate);

    let trap = summary
        .by_signal_type
        .iter()
        .find(|bucket| bucket.key == "trap_risk")
        .expect("trap bucket");
    assert!(trap.no_trade_candidate);
    assert_eq!(
        trap.top_no_trade_reasons,
        vec!["trap context only".to_string()]
    );

    assert_eq!(summary.downgrade_candidates.len(), 1);
    assert_eq!(summary.no_trade_candidates.len(), 1);

    assert_eq!(status.selected_symbol, "BTC-PERP");
    assert_eq!(status.total_evaluations, 3);
    assert_eq!(status.signal_type_count, 2);
    assert_eq!(status.window_count, 2);
    assert_eq!(status.downgrade_candidate_count, 1);
    assert_eq!(status.no_trade_candidate_count, 1);
}

#[test]
fn quality_scorecard_handles_empty_markout_set() {
    let recent = ToxicMarkoutRecentResponse {
        read_only: true,
        runtime_modified: false,
        analysis_only: true,
        execution_enabled: false,
        mode: "analysis_only".to_string(),
        selected_symbol: "BTC-PERP".to_string(),
        status: "no_markout_signal".to_string(),
        warnings: Vec::new(),
        signals: Vec::new(),
    };

    let summary = build_toxic_quality_scorecard_summary(&recent);

    assert_eq!(summary.status, "no_quality_data");
    assert_eq!(summary.total_evaluations, 0);
    assert!(summary.by_signal_type.is_empty());
    assert!(summary.by_window.is_empty());
    assert!(summary.downgrade_candidates.is_empty());
    assert!(summary.no_trade_candidates.is_empty());
    assert!(!summary.warnings.is_empty());
}

fn signal(
    signal_kind: &str,
    outcome: ToxicMarkoutOutcome,
    windows: Vec<ToxicMarkoutWindow>,
    no_trade_reasons: Vec<String>,
) -> ToxicMarkoutSignal {
    ToxicMarkoutSignal {
        signal_id: format!("{signal_kind}-id"),
        symbol: "BTC-PERP".to_string(),
        signal_kind: signal_kind.to_string(),
        direction: "SHORT_BIAS".to_string(),
        toxicity_score: 80,
        confidence: "HIGH".to_string(),
        created_at_ms: 1_000,
        overall_outcome: outcome,
        aligned_windows: windows
            .iter()
            .filter(|window| window.outcome == ToxicMarkoutOutcome::Aligned)
            .count(),
        adverse_windows: windows
            .iter()
            .filter(|window| window.outcome == ToxicMarkoutOutcome::Adverse)
            .count(),
        neutral_windows: windows
            .iter()
            .filter(|window| window.outcome == ToxicMarkoutOutcome::Neutral)
            .count(),
        missing_windows: windows
            .iter()
            .filter(|window| window.outcome == ToxicMarkoutOutcome::NotEnoughData)
            .count(),
        windows,
        no_trade_reasons,
        read_only: true,
    }
}

fn window(label: &str, outcome: ToxicMarkoutOutcome) -> ToxicMarkoutWindow {
    ToxicMarkoutWindow {
        label: label.to_string(),
        horizon_ms: 60_000,
        outcome,
        markout_bps: Some(12.5),
        price_at_signal: Some(100_000.0),
        price_at_horizon: Some(99_950.0),
        note: "test".to_string(),
    }
}
