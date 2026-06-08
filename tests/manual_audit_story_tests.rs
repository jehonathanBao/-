use std::{
    fs,
    path::{Path, PathBuf},
};

use btc_toxic_flow_monitor_rs::{
    calibration::{
        calibration_types::{
            CalibrationRecommendation, CalibrationReport, CalibrationRunSummary, EventOutcome,
            OutcomeLabel, ReasonCodeStat,
        },
        manual_audit_story::ManualAuditStoryStore,
        manual_parameter_export::{ManualParameterExportRequest, ManualParameterExportStore},
        manual_signoff_store::{ManualSignoffDecision, ManualSignoffInput, ManualSignoffStore},
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
fn manual_audit_story_reports_ready_state() {
    let report_dir = temp_report_dir("manual_audit_story_ready");
    write_ready_calibration_report(&report_dir, "calibration-9500", 9_500);
    approve_all_latest_recommendations(&report_dir, 9_510);
    create_export(&report_dir);
    create_approved_signoff(&report_dir, chrono::Local::now().timestamp_millis());

    let story = ManualAuditStoryStore::new(report_dir, test_config())
        .build_story()
        .expect("story");

    assert!(story.read_only);
    assert_eq!(story.apply_mode, "read_only_audit_story");
    assert!(!story.runtime_modified);
    assert_eq!(story.final_gate, "READY");
    assert_eq!(story.readiness_status, "READY_FOR_MANUAL_APPLY");
    assert_eq!(story.signoff_status, "SIGNED_OFF");
    assert_eq!(story.freshness_status, "FRESH");
    assert!(!story.timeline.is_empty());
    assert!(story
        .timeline
        .iter()
        .all(|item| !item.key.is_empty() && !item.title.is_empty() && !item.summary.is_empty()));
    assert!(story
        .timeline
        .iter()
        .all(|item| item.evidence_ref.is_some() || item.missing_reason.is_some()));
    assert!(story.timeline.iter().all(|item| {
        item.evidence_ref.as_ref().is_none_or(|reference| {
            !reference.kind.is_empty()
                && !reference.label.is_empty()
                && !reference.source_endpoint.is_empty()
        })
    }));
    assert!(story.markdown.contains("# Manual Apply Audit Story"));
    assert!(story.markdown.contains("> Handoff status: READY"));
    assert!(story.markdown.contains("## Manual Gate Summary"));
    assert!(story.markdown.contains("## Startup Readiness"));
    assert!(story.markdown.contains("## Operator Sign-off"));
    assert!(story.markdown.contains("## Evidence Freshness / TTL"));
    assert!(story.markdown.contains("## Changed Evidence"));
    assert!(story.markdown.contains("## Timeline"));
    assert!(story.markdown.contains("## Blockers"));
    assert!(story.markdown.contains("## Warnings"));
    assert!(story.markdown.contains("## Evidence References"));
    assert!(story.markdown.contains("## Remediation Checklist"));
    assert!(story.markdown.contains("## Operator Handoff Note"));
    assert!(story.markdown.contains("## Safety Boundary"));
    assert!(story.markdown.contains("## Generated At"));
    assert!(story.markdown.contains("## Ops Log Snippet"));
    assert!(story.markdown.contains("READY_FOR_MANUAL_APPLY"));
    assert!(story
        .markdown
        .contains("Ready for external manual execution by runbook."));
    assert!(story.markdown.contains("runtime_modified = false"));
    assert!(story.markdown.contains("No private key was used."));
}

#[test]
fn manual_audit_story_surfaces_no_signoff_blocker() {
    let report_dir = temp_report_dir("manual_audit_story_no_signoff");
    write_ready_calibration_report(&report_dir, "calibration-9510", 9_510);
    approve_all_latest_recommendations(&report_dir, 9_520);
    create_export(&report_dir);

    let story = ManualAuditStoryStore::new(report_dir, test_config())
        .build_story()
        .expect("story");

    assert_eq!(story.final_gate, "BLOCKED");
    assert_eq!(story.signoff_status, "NO_SIGNOFF");
    assert_eq!(story.current_blocker.as_deref(), Some("NO_SIGNOFF"));
    assert_eq!(story.next_owner, "Reviewer / Approver");
    assert_eq!(
        story.next_action,
        "Review evidence pack and sign off before external manual apply."
    );
    assert!(story
        .remediation_checklist
        .iter()
        .any(|item| item.contains("sign-off")));
    assert!(story.markdown.contains("NO_SIGNOFF"));
    assert!(story
        .markdown
        .contains("Review evidence pack and sign off before external manual apply."));
    assert!(story
        .timeline
        .iter()
        .any(|item| item.key == "signoff" && item.blocking_reason.is_some()));
}

fn create_approved_signoff(report_dir: &Path, now_ms: i64) {
    ManualSignoffStore::new(report_dir.to_path_buf(), test_config())
        .create_signoff_at(
            ManualSignoffInput {
                operator: "byhdo".to_string(),
                decision: ManualSignoffDecision::Approved,
                note: Some("approved".to_string()),
            },
            now_ms,
        )
        .expect("create signoff");
}

fn create_export(report_dir: &Path) {
    ManualParameterExportStore::new(report_dir)
        .create_export(ManualParameterExportRequest {
            include_statuses: Some(vec![ReviewStatus::ApprovedForManualApply]),
            operator: Some("manual".to_string()),
            note: Some("audit story export".to_string()),
        })
        .expect("create export");
}

fn approve_all_latest_recommendations(report_dir: &Path, now_ms: i64) {
    let review_store = ParameterRecommendationReviewStore::new(report_dir);
    for card in review_store.latest_recommendations().expect("latest cards") {
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
}

fn write_ready_calibration_report(dir: &Path, report_id: &str, generated_at: i64) {
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
        recommendations: vec![CalibrationRecommendation {
            title: "Threshold Comparison".to_string(),
            detail: "Best hit rate in the threshold sweep came from 1200 BTC.".to_string(),
        }],
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

fn test_config() -> AppConfig {
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
        replay_report_dir: ".runtime/reports".to_string(),
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
