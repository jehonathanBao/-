use btc_toxic_flow_monitor_rs::{
    toxicity::toxic_weight_recommendation::{
        build_toxic_weight_recommendation_status, build_toxic_weight_recommendation_summary,
    },
    types::{
        toxic_markout::{
            ToxicMarkoutOutcome, ToxicMarkoutRecentResponse, ToxicMarkoutSignal, ToxicMarkoutWindow,
        },
        toxic_weight_recommendation::{
            ToxicWeightRecommendationKind, ToxicWeightRecommendationSummaryResponse,
        },
    },
};

#[test]
fn weight_recommendation_classifies_signal_types_without_mutating_runtime() {
    let recent = ToxicMarkoutRecentResponse {
        read_only: true,
        runtime_modified: false,
        analysis_only: true,
        execution_enabled: false,
        mode: "analysis_only".to_string(),
        selected_symbol: "BTC-PERP".to_string(),
        status: "markout_ready".to_string(),
        warnings: Vec::new(),
        signals: [
            repeated_signal("BTC-PERP", "rare_signal", 10, ToxicMarkoutOutcome::Aligned),
            repeated_signal(
                "BTC-PERP",
                "short_bias_toxic_flow",
                20,
                ToxicMarkoutOutcome::Aligned,
            ),
            repeated_signal(
                "BTC-PERP",
                "long_bias_toxic_flow",
                20,
                ToxicMarkoutOutcome::Neutral,
            ),
            mixed_signal(
                "BTC-PERP",
                "bull_trap_risk",
                9,
                ToxicMarkoutOutcome::Adverse,
                11,
                ToxicMarkoutOutcome::Neutral,
            ),
            repeated_signal("BTC-PERP", "trap_risk", 20, ToxicMarkoutOutcome::Adverse),
            repeated_signal(
                "BTC-PERP",
                "spoof_cluster",
                50,
                ToxicMarkoutOutcome::Adverse,
            ),
        ]
        .into_iter()
        .flatten()
        .collect(),
    };

    let summary = build_toxic_weight_recommendation_summary(&recent);
    let status = build_toxic_weight_recommendation_status(&summary);

    assert!(summary.read_only);
    assert!(summary.analysis_only);
    assert!(!summary.runtime_modified);
    assert!(!summary.runtime_weight_modified);
    assert!(!summary.config_modified);
    assert_eq!(summary.total_recommendations, 6);

    assert_eq!(
        find(&summary, "rare_signal").recommendation,
        ToxicWeightRecommendationKind::InsufficientData
    );
    assert_eq!(
        find(&summary, "short_bias_toxic_flow").recommendation,
        ToxicWeightRecommendationKind::SlightUpgradeCandidate
    );
    assert_eq!(
        find(&summary, "long_bias_toxic_flow").recommendation,
        ToxicWeightRecommendationKind::Keep
    );
    assert_eq!(
        find(&summary, "bull_trap_risk").recommendation,
        ToxicWeightRecommendationKind::DowngradeCandidate
    );
    assert_eq!(
        find(&summary, "trap_risk").recommendation,
        ToxicWeightRecommendationKind::NoTradeOnlyCandidate
    );
    assert_eq!(
        find(&summary, "spoof_cluster").recommendation,
        ToxicWeightRecommendationKind::DisableCandidate
    );

    assert_eq!(summary.keep_count, 1);
    assert_eq!(summary.slight_upgrade_candidate_count, 1);
    assert_eq!(summary.downgrade_candidate_count, 1);
    assert_eq!(summary.no_trade_only_candidate_count, 1);
    assert_eq!(summary.disable_candidate_count, 1);
    assert_eq!(summary.insufficient_data_count, 1);
    assert_eq!(summary.by_signal_type.len(), 6);
    assert_eq!(summary.by_symbol.len(), 1);
    assert!(!summary.review_flags.is_empty());

    assert!(status.read_only);
    assert!(status.analysis_only);
    assert!(!status.runtime_modified);
    assert!(!status.runtime_weight_modified);
    assert!(!status.config_modified);
    assert_eq!(status.total_recommendations, 6);
    assert!(status.manual_review_required_count >= 4);
}

fn repeated_signal(
    symbol: &str,
    signal_type: &str,
    count: usize,
    dominant_outcome: ToxicMarkoutOutcome,
) -> Vec<ToxicMarkoutSignal> {
    (0..count)
        .map(|idx| ToxicMarkoutSignal {
            signal_id: format!("{signal_type}-{idx}"),
            symbol: symbol.to_string(),
            signal_kind: signal_type.to_string(),
            direction: "sell".to_string(),
            toxicity_score: 80,
            confidence: "high".to_string(),
            created_at_ms: 1_000 + idx as u64,
            overall_outcome: dominant_outcome,
            aligned_windows: usize::from(dominant_outcome == ToxicMarkoutOutcome::Aligned) * 3,
            adverse_windows: usize::from(dominant_outcome == ToxicMarkoutOutcome::Adverse) * 3,
            neutral_windows: usize::from(dominant_outcome == ToxicMarkoutOutcome::Neutral) * 3,
            missing_windows: usize::from(dominant_outcome == ToxicMarkoutOutcome::NotEnoughData)
                * 3,
            windows: windows_for(dominant_outcome),
            no_trade_reasons: if matches!(
                dominant_outcome,
                ToxicMarkoutOutcome::Adverse | ToxicMarkoutOutcome::Neutral
            ) {
                vec!["operator review".to_string()]
            } else {
                Vec::new()
            },
            read_only: true,
        })
        .collect()
}

fn mixed_signal(
    symbol: &str,
    signal_type: &str,
    first_count: usize,
    first_outcome: ToxicMarkoutOutcome,
    second_count: usize,
    second_outcome: ToxicMarkoutOutcome,
) -> Vec<ToxicMarkoutSignal> {
    let mut signals = repeated_signal(symbol, signal_type, first_count, first_outcome);
    signals.extend((0..second_count).map(|idx| ToxicMarkoutSignal {
        signal_id: format!("{signal_type}-mix-{idx}"),
        symbol: symbol.to_string(),
        signal_kind: signal_type.to_string(),
        direction: "sell".to_string(),
        toxicity_score: 80,
        confidence: "high".to_string(),
        created_at_ms: 10_000 + idx as u64,
        overall_outcome: second_outcome,
        aligned_windows: usize::from(second_outcome == ToxicMarkoutOutcome::Aligned) * 3,
        adverse_windows: usize::from(second_outcome == ToxicMarkoutOutcome::Adverse) * 3,
        neutral_windows: usize::from(second_outcome == ToxicMarkoutOutcome::Neutral) * 3,
        missing_windows: usize::from(second_outcome == ToxicMarkoutOutcome::NotEnoughData) * 3,
        windows: windows_for(second_outcome),
        no_trade_reasons: if matches!(
            second_outcome,
            ToxicMarkoutOutcome::Adverse | ToxicMarkoutOutcome::Neutral
        ) {
            vec!["operator review".to_string()]
        } else {
            Vec::new()
        },
        read_only: true,
    }));
    signals
}

fn windows_for(outcome: ToxicMarkoutOutcome) -> Vec<ToxicMarkoutWindow> {
    vec![
        ToxicMarkoutWindow {
            label: "+1m".to_string(),
            horizon_ms: 60_000,
            outcome,
            markout_bps: Some(match outcome {
                ToxicMarkoutOutcome::Aligned => 8.0,
                ToxicMarkoutOutcome::Adverse => -8.0,
                ToxicMarkoutOutcome::Neutral => 1.0,
                ToxicMarkoutOutcome::NotEnoughData => 0.0,
            }),
            price_at_signal: Some(100_000.0),
            price_at_horizon: Some(100_080.0),
            note: "test".to_string(),
        },
        ToxicMarkoutWindow {
            label: "+5m".to_string(),
            horizon_ms: 300_000,
            outcome,
            markout_bps: Some(match outcome {
                ToxicMarkoutOutcome::Aligned => 12.0,
                ToxicMarkoutOutcome::Adverse => -12.0,
                ToxicMarkoutOutcome::Neutral => 0.5,
                ToxicMarkoutOutcome::NotEnoughData => 0.0,
            }),
            price_at_signal: Some(100_000.0),
            price_at_horizon: Some(100_120.0),
            note: "test".to_string(),
        },
        ToxicMarkoutWindow {
            label: "+15m".to_string(),
            horizon_ms: 900_000,
            outcome,
            markout_bps: Some(match outcome {
                ToxicMarkoutOutcome::Aligned => 4.0,
                ToxicMarkoutOutcome::Adverse => -4.0,
                ToxicMarkoutOutcome::Neutral => 0.2,
                ToxicMarkoutOutcome::NotEnoughData => 0.0,
            }),
            price_at_signal: Some(100_000.0),
            price_at_horizon: Some(100_040.0),
            note: "test".to_string(),
        },
    ]
}

fn find<'a>(
    summary: &'a ToxicWeightRecommendationSummaryResponse,
    signal_type: &str,
) -> &'a btc_toxic_flow_monitor_rs::types::toxic_weight_recommendation::ToxicWeightRecommendationItem
{
    summary
        .recommendations
        .iter()
        .find(|item| item.signal_type == signal_type)
        .expect("recommendation item")
}
