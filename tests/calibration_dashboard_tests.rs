mod support;
use support::test_http_get;

use std::{fs, path::PathBuf};

use btc_toxic_flow_monitor_rs::{
    api::server::router,
    app::AppState,
    calibration::{
        calibration_report_store::CalibrationReportStore,
        calibration_types::{
            CalibrationRecommendation, CalibrationReport, CalibrationRunSummary, EventOutcome,
            OutcomeLabel, ReasonCodeStat,
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
fn calibration_report_store_sorts_and_loads_latest() {
    let dir = temp_dir("calibration_report_store");
    write_report(
        &dir,
        ReportSpec::new("calibration-1000", 1000)
            .with_counts(3, 1, 1)
            .with_best_params(600.0, 50.0),
    );
    write_report(
        &dir,
        ReportSpec::new("calibration-2000", 2000)
            .with_counts(5, 3, 0)
            .with_best_params(1000.0, 75.0),
    );

    let store = CalibrationReportStore::new(&dir);
    let reports = store.list_reports().expect("list reports");
    assert_eq!(reports.len(), 2);
    assert_eq!(reports[0].id, "calibration-2000");
    assert_eq!(reports[1].id, "calibration-1000");

    let latest = store
        .latest_report()
        .expect("latest report")
        .expect("report");
    assert_eq!(latest.summary.id, "calibration-2000");
    assert_eq!(latest.summary.best_threshold, Some(1000.0));
    assert_eq!(latest.summary.best_liq_hunt_score, Some(75.0));
    assert!(latest
        .markdown_content
        .as_deref()
        .unwrap_or_default()
        .contains("calibration-2000"));
}

#[tokio::test]
async fn calibration_report_api_exposes_list_latest_and_detail() {
    let dir = temp_dir("calibration_report_api");
    write_report(
        &dir,
        ReportSpec::new("calibration-1000", 1000)
            .with_counts(3, 1, 1)
            .with_best_params(600.0, 50.0),
    );
    write_report(
        &dir,
        ReportSpec::new("calibration-2000", 2000)
            .with_counts(5, 3, 0)
            .with_best_params(1000.0, 75.0),
    );

    let state = AppState::new(test_config(dir.to_string_lossy().to_string()));
    let app = router(state);
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0")
        .await
        .expect("bind");
    let addr = listener.local_addr().expect("addr");
    let server = tokio::spawn(async move {
        axum::serve(listener, app).await.expect("server");
    });

    let list = test_http_get(format!("http://{addr}/api/calibration/reports"))
        .await
        .expect("list response")
        .json::<serde_json::Value>()
        .await
        .expect("list json");
    assert_eq!(list["reports"][0]["id"], "calibration-2000");

    let latest = test_http_get(format!("http://{addr}/api/calibration/reports/latest"))
        .await
        .expect("latest response")
        .json::<serde_json::Value>()
        .await
        .expect("latest json");
    assert_eq!(latest["report"]["summary"]["id"], "calibration-2000");
    assert_eq!(latest["report"]["report"]["baseline"]["eventCount"], 5);

    let detail = test_http_get(format!(
        "http://{addr}/api/calibration/reports/calibration-1000"
    ))
    .await
    .expect("detail response")
    .json::<serde_json::Value>()
    .await
    .expect("detail json");
    assert_eq!(detail["report"]["summary"]["id"], "calibration-1000");
    assert!(detail["report"]["markdownContent"]
        .as_str()
        .unwrap_or_default()
        .contains("calibration-1000"));

    server.abort();
}

#[derive(Debug, Clone)]
struct ReportSpec {
    id: String,
    generated_at: i64,
    event_count: usize,
    hit_count: usize,
    false_positive_count: usize,
    best_threshold: f64,
    best_liq_hunt_score: f64,
}

impl ReportSpec {
    fn new(id: &str, generated_at: i64) -> Self {
        Self {
            id: id.to_string(),
            generated_at,
            event_count: 0,
            hit_count: 0,
            false_positive_count: 0,
            best_threshold: 1000.0,
            best_liq_hunt_score: 75.0,
        }
    }

    fn with_counts(
        mut self,
        event_count: usize,
        hit_count: usize,
        false_positive_count: usize,
    ) -> Self {
        self.event_count = event_count;
        self.hit_count = hit_count;
        self.false_positive_count = false_positive_count;
        self
    }

    fn with_best_params(mut self, best_threshold: f64, best_liq_hunt_score: f64) -> Self {
        self.best_threshold = best_threshold;
        self.best_liq_hunt_score = best_liq_hunt_score;
        self
    }
}

#[derive(Debug, Clone)]
struct SummarySpec<'a> {
    label: &'a str,
    toxic_threshold_btc: f64,
    min_toxic_ratio: f64,
    vpin_bucket_size_btc: f64,
    vpin_lookback_buckets: usize,
    vpin_spike_zscore: f64,
    liq_hunt_likely_score: f64,
    liq_hunt_active_score: f64,
    event_count: usize,
    hit_count: usize,
    false_positive_count: usize,
}

fn write_report(dir: &PathBuf, spec: ReportSpec) {
    fs::create_dir_all(dir).expect("create report dir");
    let report = sample_report(spec.clone());
    let json_path = dir.join(format!("{}.json", spec.id));
    let markdown_path = dir.join(format!("{}.md", spec.id));
    fs::write(
        &json_path,
        serde_json::to_string_pretty(&report).expect("serialize report"),
    )
    .expect("write report json");
    fs::write(&markdown_path, format!("# {}\n", spec.id)).expect("write report markdown");
}

fn sample_report(spec: ReportSpec) -> CalibrationReport {
    let baseline = summary(SummarySpec {
        label: "baseline",
        toxic_threshold_btc: spec.best_threshold,
        min_toxic_ratio: 0.60,
        vpin_bucket_size_btc: 100.0,
        vpin_lookback_buckets: 50,
        vpin_spike_zscore: 2.5,
        liq_hunt_likely_score: 50.0,
        liq_hunt_active_score: spec.best_liq_hunt_score,
        event_count: spec.event_count,
        hit_count: spec.hit_count,
        false_positive_count: spec.false_positive_count,
    });
    CalibrationReport {
        input_path: ".\\fixtures\\sample-liquidation-hunt.jsonl".to_string(),
        generated_at: spec.generated_at,
        baseline: baseline.clone(),
        event_outcomes: vec![sample_outcome()],
        threshold_comparison: vec![
            summary(SummarySpec {
                label: "threshold_600",
                toxic_threshold_btc: 600.0,
                min_toxic_ratio: 0.60,
                vpin_bucket_size_btc: 100.0,
                vpin_lookback_buckets: 50,
                vpin_spike_zscore: 2.5,
                liq_hunt_likely_score: 50.0,
                liq_hunt_active_score: 75.0,
                event_count: spec.event_count + 1,
                hit_count: spec.hit_count,
                false_positive_count: spec.false_positive_count + 1,
            }),
            summary(SummarySpec {
                label: "threshold_best",
                toxic_threshold_btc: spec.best_threshold,
                min_toxic_ratio: 0.60,
                vpin_bucket_size_btc: 100.0,
                vpin_lookback_buckets: 50,
                vpin_spike_zscore: 2.5,
                liq_hunt_likely_score: 50.0,
                liq_hunt_active_score: 75.0,
                event_count: spec.event_count,
                hit_count: spec.hit_count,
                false_positive_count: spec.false_positive_count,
            }),
        ],
        toxic_ratio_comparison: vec![baseline.clone()],
        vpin_parameter_comparison: vec![baseline.clone()],
        liq_hunt_score_comparison: vec![
            summary(SummarySpec {
                label: "liq_low",
                toxic_threshold_btc: spec.best_threshold,
                min_toxic_ratio: 0.60,
                vpin_bucket_size_btc: 100.0,
                vpin_lookback_buckets: 50,
                vpin_spike_zscore: 2.5,
                liq_hunt_likely_score: 50.0,
                liq_hunt_active_score: 50.0,
                event_count: spec.event_count,
                hit_count: spec.hit_count.saturating_sub(1),
                false_positive_count: spec.false_positive_count + 1,
            }),
            summary(SummarySpec {
                label: "liq_best",
                toxic_threshold_btc: spec.best_threshold,
                min_toxic_ratio: 0.60,
                vpin_bucket_size_btc: 100.0,
                vpin_lookback_buckets: 50,
                vpin_spike_zscore: 2.5,
                liq_hunt_likely_score: 50.0,
                liq_hunt_active_score: spec.best_liq_hunt_score,
                event_count: spec.event_count,
                hit_count: spec.hit_count,
                false_positive_count: spec.false_positive_count,
            }),
        ],
        reason_code_stats: vec![ReasonCodeStat {
            reason_code: "large_aggressive_flow".to_string(),
            total_count: spec.event_count,
            hit_count: spec.hit_count,
            false_positive_count: spec.false_positive_count,
            neutral_count: 0,
            unknown_count: spec
                .event_count
                .saturating_sub(spec.hit_count + spec.false_positive_count),
            hit_rate: if spec.event_count == 0 {
                0.0
            } else {
                spec.hit_count as f64 / spec.event_count as f64
            },
            false_positive_rate: if spec.event_count == 0 {
                0.0
            } else {
                spec.false_positive_count as f64 / spec.event_count as f64
            },
        }],
        top_false_positives: vec![sample_outcome()],
        top_hits: vec![sample_outcome()],
        recommendations: vec![CalibrationRecommendation {
            title: "Keep threshold".to_string(),
            detail: "The current threshold preserves precision while keeping enough hits."
                .to_string(),
        }],
    }
}

fn summary(spec: SummarySpec<'_>) -> CalibrationRunSummary {
    let unknown_count = spec
        .event_count
        .saturating_sub(spec.hit_count + spec.false_positive_count);
    CalibrationRunSummary {
        group: "test".to_string(),
        label: spec.label.to_string(),
        toxic_threshold_btc: spec.toxic_threshold_btc,
        min_toxic_ratio: spec.min_toxic_ratio,
        vpin_bucket_size_btc: spec.vpin_bucket_size_btc,
        vpin_lookback_buckets: spec.vpin_lookback_buckets,
        vpin_spike_zscore: spec.vpin_spike_zscore,
        liq_hunt_likely_score: spec.liq_hunt_likely_score,
        liq_hunt_active_score: spec.liq_hunt_active_score,
        event_count: spec.event_count,
        hit_count: spec.hit_count,
        false_positive_count: spec.false_positive_count,
        neutral_count: 0,
        unknown_count,
        hit_rate: if spec.event_count == 0 {
            0.0
        } else {
            spec.hit_count as f64 / spec.event_count as f64
        },
        false_positive_rate: if spec.event_count == 0 {
            0.0
        } else {
            spec.false_positive_count as f64 / spec.event_count as f64
        },
        max_toxic_volume_btc: spec.toxic_threshold_btc * 1.2,
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

fn temp_dir(name: &str) -> PathBuf {
    let path = std::env::temp_dir().join(format!(
        "btc-toxic-flow-monitor-rs-{name}-{}",
        std::process::id()
    ));
    if path.exists() {
        let _ = fs::remove_dir_all(&path);
    }
    fs::create_dir_all(&path).expect("create temp dir");
    path
}

fn test_config(replay_report_dir: String) -> AppConfig {
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
        replay_report_dir,
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
    }
}
