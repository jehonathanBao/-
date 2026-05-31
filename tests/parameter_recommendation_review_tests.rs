use std::{fs, path::PathBuf};

use btc_toxic_flow_monitor_rs::calibration::{
    calibration_types::{
        CalibrationRecommendation, CalibrationReport, CalibrationRunSummary, EventOutcome,
        OutcomeLabel, ReasonCodeStat,
    },
    parameter_recommendation_review_store::{
        ParameterRecommendationReviewStore, ReviewDecisionInput, ReviewStatus,
    },
};
use btc_toxic_flow_monitor_rs::types::{
    market::Venue,
    toxic::{ToxicDirection, ToxicEvent, ToxicSeverity},
};

#[test]
fn recommendation_store_extracts_cards_and_persists_reviews() {
    let report_dir = temp_report_dir("parameter_review_store");
    write_calibration_report(&report_dir, ReportSpec::default());

    let store = ParameterRecommendationReviewStore::new(&report_dir);
    let cards = store.list_recommendations().expect("list recommendations");
    assert_eq!(cards.len(), 7);
    assert_eq!(cards[0].report_id, "calibration-2000");
    assert!(cards
        .iter()
        .any(|card| card.parameter_key == "vpin.bucket_size_btc"));
    assert!(cards
        .iter()
        .any(|card| card.parameter_key == "liq_hunt.active_score"));

    let threshold_card = cards
        .iter()
        .find(|card| card.parameter_key == "toxicity.threshold_btc")
        .expect("threshold card");
    assert_eq!(threshold_card.current_value, Some(1000.0));
    assert_eq!(threshold_card.recommended_value, Some(1200.0));
    assert_eq!(threshold_card.direction, "raise");

    let review = store
        .append_review(
            ReviewDecisionInput {
                recommendation_id: threshold_card.recommendation_id.clone(),
                report_id: threshold_card.report_id.clone(),
                status: ReviewStatus::ApprovedForManualApply,
                reviewer_note: Some("review only".to_string()),
                reviewer: Some("codex".to_string()),
            },
            3_000,
        )
        .expect("append review");
    assert_eq!(review.status, ReviewStatus::ApprovedForManualApply);
    assert!(store.ledger_path().exists());

    let refreshed = store.list_recommendations().expect("refresh cards");
    let threshold_card = refreshed
        .iter()
        .find(|card| card.parameter_key == "toxicity.threshold_btc")
        .expect("threshold card after review");
    assert_eq!(
        threshold_card
            .current_review
            .as_ref()
            .map(|review| review.status),
        Some(ReviewStatus::ApprovedForManualApply)
    );
}

#[derive(Debug, Clone)]
struct ReportSpec {
    report_id: String,
    generated_at: i64,
}

impl Default for ReportSpec {
    fn default() -> Self {
        Self {
            report_id: "calibration-2000".to_string(),
            generated_at: 2_000,
        }
    }
}

fn write_calibration_report(dir: &PathBuf, spec: ReportSpec) {
    fs::create_dir_all(dir).expect("create report dir");
    let report = sample_report(spec.generated_at);
    fs::write(
        dir.join(format!("{}.json", spec.report_id)),
        serde_json::to_string_pretty(&report).expect("serialize report"),
    )
    .expect("write report json");
    fs::write(
        dir.join(format!("{}.md", spec.report_id)),
        "# calibration\n",
    )
    .expect("write report markdown");
}

fn sample_report(generated_at: i64) -> CalibrationReport {
    let baseline = CalibrationRunSummary {
        group: "baseline".to_string(),
        label: "baseline".to_string(),
        toxic_threshold_btc: 1000.0,
        min_toxic_ratio: 0.60,
        vpin_bucket_size_btc: 100.0,
        vpin_lookback_buckets: 50,
        vpin_spike_zscore: 2.5,
        liq_hunt_likely_score: 50.0,
        liq_hunt_active_score: 75.0,
        event_count: 25,
        hit_count: 16,
        false_positive_count: 1,
        neutral_count: 0,
        unknown_count: 8,
        hit_rate: 0.64,
        false_positive_rate: 0.04,
        max_toxic_volume_btc: 1406.0,
    };
    CalibrationReport {
        input_path: ".\\fixtures\\sample-liquidation-hunt.jsonl".to_string(),
        generated_at,
        baseline: baseline.clone(),
        event_outcomes: vec![sample_outcome()],
        threshold_comparison: vec![
            CalibrationRunSummary {
                label: "threshold_1200".to_string(),
                toxic_threshold_btc: 1200.0,
                ..baseline.clone()
            },
            CalibrationRunSummary {
                label: "threshold_1000".to_string(),
                toxic_threshold_btc: 1000.0,
                ..baseline.clone()
            },
        ],
        toxic_ratio_comparison: vec![CalibrationRunSummary {
            label: "ratio_0_70".to_string(),
            min_toxic_ratio: 0.70,
            ..baseline.clone()
        }],
        vpin_parameter_comparison: vec![CalibrationRunSummary {
            label: "vpin_best".to_string(),
            vpin_bucket_size_btc: 250.0,
            vpin_lookback_buckets: 100,
            vpin_spike_zscore: 3.0,
            ..baseline.clone()
        }],
        liq_hunt_score_comparison: vec![CalibrationRunSummary {
            label: "liq_best".to_string(),
            liq_hunt_likely_score: 60.0,
            liq_hunt_active_score: 80.0,
            ..baseline.clone()
        }],
        reason_code_stats: vec![ReasonCodeStat {
            reason_code: "large_aggressive_flow".to_string(),
            total_count: 25,
            hit_count: 16,
            false_positive_count: 1,
            neutral_count: 0,
            unknown_count: 8,
            hit_rate: 0.64,
            false_positive_rate: 0.04,
        }],
        top_false_positives: vec![sample_outcome()],
        top_hits: vec![sample_outcome()],
        recommendations: vec![
            CalibrationRecommendation {
                title: "Threshold Comparison".to_string(),
                detail: "Best hit rate in the threshold sweep came from 1200 BTC (hit_rate 0.68, false_positive_rate 0.02); baseline is 1000 BTC.".to_string(),
            },
            CalibrationRecommendation {
                title: "Toxic Ratio Comparison".to_string(),
                detail: "Min toxic ratio 0.70 produced hit_rate 0.66 and false_positive_rate 0.03.".to_string(),
            },
            CalibrationRecommendation {
                title: "VPIN Parameter Comparison".to_string(),
                detail: "Best VPIN sweep used bucket_size 250 BTC, lookback 100, z-score 3.0 (hit_rate 0.66).".to_string(),
            },
            CalibrationRecommendation {
                title: "Liq Hunt Score Comparison".to_string(),
                detail: "Likely 60 / Active 80 gave the best liq hunt sweep result (hit_rate 0.67, false_positive_rate 0.02).".to_string(),
            },
        ],
    }
}

fn sample_outcome() -> EventOutcome {
    EventOutcome {
        event: ToxicEvent {
            id: "event-1".to_string(),
            ts: 5_000,
            symbol: "BTC-PERP".to_string(),
            direction: ToxicDirection::Buy,
            severity: ToxicSeverity::Alert,
            toxic_volume_btc: 1_250.0,
            threshold_btc: 1_000.0,
            window_ms: 5_000,
            leader_venue: Some(Venue::Binance),
            aggressive_buy_btc: 1_500.0,
            aggressive_sell_btc: 250.0,
            net_aggressive_btc: 1_250.0,
            abs_aggressive_btc: 1_750.0,
            markout_1s_bps: Some(2.0),
            markout_5s_bps: Some(4.2),
            sweep_detected: true,
            liquidity_thin: true,
            liquidity: None,
            cross_venue_confirmed: true,
            vpin_enabled: true,
            vpin: Some(0.82),
            vpin_zscore: Some(2.8),
            vpin_spike: true,
            vpin_high: true,
            vpin_extreme: false,
            liquidation_enabled: true,
            nearest_cluster_side: None,
            cluster_distance_bps: Some(18.0),
            cluster_notional_usd: Some(55_000_000.0),
            cluster_density: Some(0.74),
            liq_hunt_pressure: 0.68,
            liq_cluster_nearby: true,
            possible_liq_hunt_setup: true,
            reason_codes: vec![
                "large_aggressive_flow".to_string(),
                "vpin_spike".to_string(),
                "possible_liq_hunt_setup".to_string(),
            ],
        },
        current_mid: Some(100_000.0),
        forward_1s_bps: Some(1.5),
        forward_5s_bps: Some(2.5),
        forward_15s_bps: Some(3.0),
        forward_60s_bps: None,
        primary_horizon_ms: Some(5_000),
        primary_move_bps: Some(2.5),
        label: OutcomeLabel::Hit,
    }
}

fn temp_report_dir(name: &str) -> PathBuf {
    let root = std::env::temp_dir().join(format!(
        "btc-toxic-flow-monitor-rs-{name}-{}",
        std::process::id()
    ));
    if root.exists() {
        let _ = fs::remove_dir_all(&root);
    }
    let report_dir = root.join("reports");
    fs::create_dir_all(&report_dir).expect("create temp dir");
    report_dir
}
