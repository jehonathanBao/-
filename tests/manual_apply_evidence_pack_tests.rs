use std::{fs, path::PathBuf};

use btc_toxic_flow_monitor_rs::{
    calibration::{
        calibration_types::{
            CalibrationRecommendation, CalibrationReport, CalibrationRunSummary, EventOutcome,
            OutcomeLabel, ReasonCodeStat,
        },
        manual_apply_evidence_pack::ManualApplyEvidencePackStore,
        manual_parameter_export::{
            ManualParameterExport, ManualParameterExportRequest, ManualParameterExportStore,
        },
        parameter_recommendation_review_store::{
            ParameterRecommendationReviewStore, ReviewDecisionInput, ReviewStatus,
        },
    },
    config::{
        venues::{VenueConfig, VenueConfigs},
        AppConfig,
    },
    types::{
        market::Venue,
        toxic::{ToxicDirection, ToxicEvent, ToxicSeverity},
    },
};

#[test]
fn latest_evidence_pack_includes_safety_boundary_and_markdown() {
    let report_dir = temp_report_dir("manual_apply_evidence_pack");
    write_calibration_report(&report_dir, "calibration-8900", 8_900);
    approve_recommendation(&report_dir, "toxicity.threshold_btc", 8_910);
    create_export(&report_dir);

    let store =
        ManualApplyEvidencePackStore::new(&report_dir, Some(test_config(report_dir.clone())));
    let pack = store.latest_pack().expect("latest pack").expect("pack");

    assert_eq!(pack.apply_mode, "manual_signoff_only");
    assert!(!pack.runtime_modified);
    assert!(pack.signoff_required);
    assert!(pack.signoff_allowed);
    assert_eq!(pack.dry_run_summary.warning_count, 0);
    assert_eq!(pack.dry_run_summary.blocker_count, 0);
    assert!(pack.safety_boundary.no_runtime_config_changed);
    assert!(pack.safety_boundary.no_calibration_runner_triggered);
    assert!(!pack.safety_boundary.realtime_pipelines_modified);

    let markdown = store
        .latest_markdown()
        .expect("latest markdown")
        .expect("markdown");
    assert!(markdown.contains("# Manual Apply Evidence Pack"));
    assert!(markdown.contains("## Operator Sign-off"));
    assert!(markdown.contains("## Safety Boundary"));
    assert!(markdown.contains("apply_mode = manual_signoff_only"));
}

#[test]
fn failed_dry_run_blocks_signoff() {
    let report_dir = temp_report_dir("manual_apply_evidence_pack_failed");
    write_calibration_report(&report_dir, "calibration-8910", 8_910);
    approve_recommendation(&report_dir, "toxicity.threshold_btc", 8_920);
    let export_id = create_export(&report_dir);
    mutate_export(&report_dir, &export_id, |export| {
        export.items.first_mut().expect("item").parameter_key = "runtime.reload_flag".to_string();
    });

    let store =
        ManualApplyEvidencePackStore::new(&report_dir, Some(test_config(report_dir.clone())));
    let pack = store
        .pack_by_id(&export_id)
        .expect("pack by id")
        .expect("pack");

    assert_eq!(
        pack.dry_run_summary.status,
        btc_toxic_flow_monitor_rs::calibration::manual_apply_dryrun_validator::DryRunStatus::Failed
    );
    assert!(pack.signoff_required);
    assert!(!pack.signoff_allowed);
    assert!(!pack.blockers.is_empty());
    assert_eq!(
        pack.operator_signoff_template.blocking_message.as_deref(),
        Some("Operator sign-off is blocked because dry-run failed.")
    );
}

#[test]
fn passed_with_warnings_keeps_signoff_allowed() {
    let report_dir = temp_report_dir("manual_apply_evidence_pack_warning");
    write_calibration_report(&report_dir, "calibration-8920", 8_920);
    approve_recommendation(&report_dir, "toxicity.threshold_btc", 8_930);
    let export_id = create_export(&report_dir);
    mutate_export(&report_dir, &export_id, |export| {
        let item = export.items.first_mut().expect("item");
        item.recommended_value = item.current_value;
    });

    let store =
        ManualApplyEvidencePackStore::new(&report_dir, Some(test_config(report_dir.clone())));
    let pack = store
        .pack_by_id(&export_id)
        .expect("pack by id")
        .expect("pack");

    assert_eq!(pack.dry_run_summary.status, btc_toxic_flow_monitor_rs::calibration::manual_apply_dryrun_validator::DryRunStatus::PassedWithWarnings);
    assert!(pack.signoff_allowed);
    assert!(pack.blockers.is_empty());
    assert!(!pack.warnings.is_empty());
}

fn approve_recommendation(report_dir: &PathBuf, parameter_key: &str, now_ms: i64) {
    let review_store = ParameterRecommendationReviewStore::new(report_dir);
    let card = review_store
        .list_recommendations()
        .expect("cards")
        .into_iter()
        .find(|card| card.parameter_key == parameter_key)
        .expect("matching recommendation");
    review_store
        .append_review(
            ReviewDecisionInput {
                recommendation_id: card.recommendation_id,
                report_id: card.report_id,
                status: ReviewStatus::ApprovedForManualApply,
                reviewer_note: Some("approved".to_string()),
                reviewer: Some("codex".to_string()),
            },
            now_ms,
        )
        .expect("append review");
}

fn create_export(report_dir: &PathBuf) -> String {
    ManualParameterExportStore::new(report_dir)
        .create_export(ManualParameterExportRequest {
            include_statuses: Some(vec![ReviewStatus::ApprovedForManualApply]),
            operator: Some("manual".to_string()),
            note: Some("evidence pack test".to_string()),
        })
        .expect("create export")
        .export_id
        .expect("export id")
}

fn mutate_export(
    report_dir: &std::path::Path,
    export_id: &str,
    mutate: impl FnOnce(&mut ManualParameterExport),
) {
    let json_path = report_dir
        .parent()
        .expect("runtime dir")
        .join("exports")
        .join(format!("{export_id}.json"));
    let mut export: ManualParameterExport =
        serde_json::from_str(&fs::read_to_string(&json_path).expect("read export"))
            .expect("parse export");
    mutate(&mut export);
    fs::write(
        &json_path,
        serde_json::to_string_pretty(&export).expect("serialize export"),
    )
    .expect("write export");
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
        threshold_comparison: vec![CalibrationRunSummary {
            label: "threshold_1200".to_string(),
            toxic_threshold_btc: 1200.0,
            ..baseline.clone()
        }],
        toxic_ratio_comparison: vec![CalibrationRunSummary {
            label: "ratio_0_70".to_string(),
            min_toxic_ratio: 0.70,
            ..baseline.clone()
        }],
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
                title: "Toxic Ratio Comparison".to_string(),
                detail: "Min toxic ratio 0.70 produced hit_rate 0.66 and false_positive_rate 0.03.".to_string(),
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

fn test_config(replay_report_dir: PathBuf) -> AppConfig {
    AppConfig {
        app_env: "test".to_string(),
        read_only: true,
        api_host: "127.0.0.1".parse().expect("valid ip"),
        api_port: 0,
        symbol: "BTC-PERP".to_string(),
        toxic_volume_alert_btc: 1000.0,
        windows_ms: vec![1000, 5000, 15000, 60000],
        markout_horizons_ms: vec![1000, 5000, 15000],
        sweep_windows_ms: vec![1000, 5000, 15000],
        venues: VenueConfigs {
            binance: VenueConfig {
                venue: Venue::Binance,
                enabled: false,
            },
            bybit: VenueConfig {
                venue: Venue::Bybit,
                enabled: false,
            },
            okx: VenueConfig {
                venue: Venue::Okx,
                enabled: false,
            },
        },
        flow_compute_interval_ms: 50,
        markout_resolve_interval_ms: 50,
        sweep_compute_interval_ms: 50,
        toxic_compute_interval_ms: 50,
        telegram_enabled: false,
        telegram_bot_token: String::new(),
        telegram_chat_id: String::new(),
        alert_dedup_window_ms: 30_000,
        alert_min_severity: ToxicSeverity::Alert,
        alert_require_cross_venue: true,
        alert_require_markout: true,
        alert_require_liquidity_drain: false,
        sqlite_enabled: false,
        sqlite_path: ".runtime/test.sqlite".to_string(),
        snapshot_persist_interval_ms: 1000,
        raw_snapshot_enabled: false,
        raw_snapshot_sample_rate_ms: 1000,
        replay_enabled: true,
        replay_report_dir: replay_report_dir.to_string_lossy().to_string(),
        vpin_enabled: true,
        vpin_bucket_size_btc: 100.0,
        vpin_lookback_buckets: 50,
        vpin_min_buckets: 10,
        vpin_spike_zscore: 2.5,
        vpin_high_threshold: 0.70,
        vpin_extreme_threshold: 0.85,
        vpin_persist_buckets: true,
        liquidation_enabled: true,
        liquidation_lookback_ms: 120_000,
        liquidation_cluster_band_bps: 6.0,
        liquidation_min_cluster_distance_bps: 5.0,
        liquidation_max_cluster_distance_bps: 150.0,
        liquidation_proximity_threshold_bps: 25.0,
        liquidation_min_cluster_touches: 3,
        liquidation_pressure_threshold: 0.65,
        liq_hunt_cluster_large_notional_usd: 50_000_000.0,
        liq_hunt_near_distance_bps: 25.0,
        liq_hunt_active_score: 75.0,
        liq_hunt_likely_score: 50.0,
        liq_hunt_watch_score: 30.0,
        book_stale_ms: 5000,
        max_buffer_age_ms: 120000,
        contract_whale_monitor:
            btc_toxic_flow_monitor_rs::config::env::ContractWhaleMonitorConfig {
                enabled: false,
                dry_run: true,
            },
        spot_whale_monitor: btc_toxic_flow_monitor_rs::config::env::SpotWhaleMonitorConfig {
            enabled: false,
            dry_run: true,
        },
    }
}
