use std::{fs, path::PathBuf};

use btc_toxic_flow_monitor_rs::calibration::{
    calibration_types::{
        CalibrationRecommendation, CalibrationReport, CalibrationRunSummary, EventOutcome,
        OutcomeLabel, ReasonCodeStat,
    },
    manual_parameter_export::{ManualParameterExportRequest, ManualParameterExportStore},
    parameter_recommendation_review_store::{
        ParameterRecommendationReviewStore, ReviewDecisionInput, ReviewStatus,
    },
};
use btc_toxic_flow_monitor_rs::types::{
    market::Venue,
    toxic::{ToxicDirection, ToxicEvent, ToxicSeverity},
};

#[test]
fn export_store_creates_manual_only_files_for_approved_recommendations() {
    let report_dir = temp_report_dir("manual_export_store");
    write_calibration_report(&report_dir, "calibration-4000", 4_000);

    let review_store = ParameterRecommendationReviewStore::new(&report_dir);
    let cards = review_store.list_recommendations().expect("cards");
    let threshold = cards
        .iter()
        .find(|card| card.parameter_key == "toxicity.threshold_btc")
        .expect("threshold recommendation");
    let vpin = cards
        .iter()
        .find(|card| card.parameter_key == "vpin.bucket_size_btc")
        .expect("vpin recommendation");

    review_store
        .append_review(
            ReviewDecisionInput {
                recommendation_id: threshold.recommendation_id.clone(),
                report_id: threshold.report_id.clone(),
                status: ReviewStatus::ApprovedForManualApply,
                reviewer_note: Some("looks good".to_string()),
                reviewer: Some("codex".to_string()),
            },
            5_000,
        )
        .expect("approve threshold");
    review_store
        .append_review(
            ReviewDecisionInput {
                recommendation_id: vpin.recommendation_id.clone(),
                report_id: vpin.report_id.clone(),
                status: ReviewStatus::Watch,
                reviewer_note: Some("not yet".to_string()),
                reviewer: Some("codex".to_string()),
            },
            5_100,
        )
        .expect("watch vpin");

    let export_store = ManualParameterExportStore::new(&report_dir);
    let response = export_store
        .create_export(ManualParameterExportRequest {
            include_statuses: Some(vec![ReviewStatus::ApprovedForManualApply]),
            operator: Some("manual".to_string()),
            note: Some("R14 smoke export".to_string()),
        })
        .expect("create export");

    assert!(response.ok);
    assert!(response.export_created);
    assert_eq!(response.apply_mode, "manual_only");
    assert!(!response.runtime_modified);
    assert_eq!(response.recommendation_count, 1);

    let latest = export_store
        .latest_export()
        .expect("latest")
        .expect("export");
    assert_eq!(latest.summary.recommendation_count, 1);
    assert_eq!(latest.export.items.len(), 1);
    assert!(latest.export.safety.manual_only);
    assert!(!latest.export.safety.runtime_modified);
    assert!(!latest.export.safety.auto_apply_supported);
    assert!(latest.export.safety.requires_human_review);
    assert_eq!(
        latest.export.items[0].parameter_key,
        "toxicity.threshold_btc"
    );
    assert!(latest
        .markdown_content
        .as_deref()
        .unwrap_or_default()
        .contains("Manual Parameter Patch Export"));
}

#[test]
fn export_store_returns_no_file_when_no_approved_recommendations_exist() {
    let report_dir = temp_report_dir("manual_export_none");
    write_calibration_report(&report_dir, "calibration-5000", 5_000);
    let review_store = ParameterRecommendationReviewStore::new(&report_dir);
    let cards = review_store.list_recommendations().expect("cards");
    let threshold = cards
        .iter()
        .find(|card| card.parameter_key == "toxicity.threshold_btc")
        .expect("threshold recommendation");
    review_store
        .append_review(
            ReviewDecisionInput {
                recommendation_id: threshold.recommendation_id.clone(),
                report_id: threshold.report_id.clone(),
                status: ReviewStatus::Watch,
                reviewer_note: None,
                reviewer: None,
            },
            6_000,
        )
        .expect("watch threshold");

    let export_store = ManualParameterExportStore::new(&report_dir);
    let response = export_store
        .create_export(ManualParameterExportRequest {
            include_statuses: Some(vec![ReviewStatus::ApprovedForManualApply]),
            operator: Some("manual".to_string()),
            note: Some("should stay empty".to_string()),
        })
        .expect("create export");

    assert!(response.ok);
    assert!(!response.export_created);
    assert_eq!(
        response.reason.as_deref(),
        Some("no_approved_recommendations")
    );
    assert_eq!(response.recommendation_count, 0);
    assert!(export_store
        .list_exports()
        .expect("list exports")
        .is_empty());
}

fn write_calibration_report(dir: &PathBuf, report_id: &str, generated_at: i64) {
    fs::create_dir_all(dir).expect("create report dir");
    let report = sample_report(generated_at);
    fs::write(
        dir.join(format!("{report_id}.json")),
        serde_json::to_string_pretty(&report).expect("serialize report"),
    )
    .expect("write report json");
    fs::write(dir.join(format!("{report_id}.md")), "# calibration\n")
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
        threshold_comparison: vec![baseline.clone()],
        toxic_ratio_comparison: vec![baseline.clone()],
        vpin_parameter_comparison: vec![baseline.clone()],
        liq_hunt_score_comparison: vec![baseline.clone()],
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
                title: "VPIN Parameter Comparison".to_string(),
                detail: "Best VPIN sweep used bucket_size 250 BTC, lookback 100, z-score 3.0 (hit_rate 0.66).".to_string(),
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
            reason_codes: vec!["large_aggressive_flow".to_string()],
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
