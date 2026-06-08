mod support;

use btc_toxic_flow_monitor_rs::{
    api::server::router,
    app::AppState,
    config::{
        venues::{VenueConfig, VenueConfigs},
        AppConfig,
    },
    types::{
        market::Venue,
        toxic::ToxicSeverity,
        toxic_signal_alert_preview::{
            ToxicSignalAlertPreviewBucket, ToxicSignalAlertPreviewFilter,
            ToxicSignalAlertPreviewGate, ToxicSignalAlertPreviewItem,
            ToxicSignalAlertPreviewResponse, ToxicSignalAlertPreviewSummary,
        },
        toxic_signal_group::{
            ToxicSignalGroup, ToxicSignalGroupOperatorAction, ToxicSignalGroupRecentResponse,
        },
        toxic_signal_inbox::{
            ToxicSignalInboxFusionSummary, ToxicSignalInboxGovernanceSummary, ToxicSignalInboxItem,
            ToxicSignalInboxMarkoutSummary, ToxicSignalInboxOperatorAction,
            ToxicSignalInboxQualitySummary, ToxicSignalInboxRecentResponse,
            ToxicSignalInboxRecommendationSummary, ToxicSignalInboxReplaySummary,
        },
        toxic_signal_report::{
            ToxicSignalReportBucket, ToxicSignalReportDailyResponse, ToxicSignalReportFilter,
            ToxicSignalReportMarkoutSummary, ToxicSignalReportSummary, ToxicSignalReportTopGroup,
        },
    },
};
use support::test_http_client;

#[tokio::test]
async fn durable_archive_dryrun_api_returns_read_only_contract_payload() {
    let state = AppState::new(test_config());
    seed_history_snapshot(&state);
    let app = router(state);
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0")
        .await
        .expect("bind");
    let addr = listener.local_addr().expect("addr");
    let server = tokio::spawn(async move {
        axum::serve(listener, app).await.expect("server");
    });

    let response = test_http_client()
        .post(format!(
            "http://{addr}/api/archive/dry-run/write?symbol=BTCUSDT"
        ))
        .send()
        .await
        .expect("dry-run response");
    assert_eq!(response.status(), reqwest::StatusCode::OK);
    let payload: serde_json::Value = response.json().await.expect("dry-run json");

    assert_eq!(payload["ok"], true);
    assert_eq!(payload["readOnly"], true);
    assert_eq!(payload["runtimeModified"], false);
    assert_eq!(payload["analysisOnly"], true);
    assert_eq!(payload["action"], "dry_run_write");
    assert_eq!(payload["archiveWriteEnabled"], false);
    assert_eq!(payload["durableStorageEnabled"], false);
    assert_eq!(payload["databaseWriteEnabled"], false);
    assert_eq!(payload["jsonlWriteEnabled"], false);
    assert_eq!(payload["sqliteWriteEnabled"], false);
    assert_eq!(payload["executionEnabled"], false);
    assert_eq!(payload["notificationSent"], false);
    assert!(payload["recordsPrepared"].as_u64().unwrap_or(0) > 0);
    assert!(payload["records"].is_array());
    assert!(payload["safetyBoundary"].is_array());
    assert!(payload["safetyBoundary"]
        .as_array()
        .expect("safety boundary")
        .iter()
        .any(|item| item == "No order placement"));

    server.abort();
}

#[tokio::test]
async fn durable_archive_dryrun_api_reports_failure_simulation_without_side_effect_flags() {
    let state = AppState::new(test_config());
    let app = router(state);
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0")
        .await
        .expect("bind");
    let addr = listener.local_addr().expect("addr");
    let server = tokio::spawn(async move {
        axum::serve(listener, app).await.expect("server");
    });

    let response = test_http_client()
        .post(format!(
            "http://{addr}/api/archive/dry-run/write?symbol=BTCUSDT"
        ))
        .json(&serde_json::json!({
            "records": [{
                "symbol": "BTCUSDT",
                "signalKind": "short_bias_toxic_flow",
                "createdAtMs": 1000,
                "schemaVersion": "archive.v999",
                "sourceModule": "signal_inbox",
                "evidenceRefs": ["wallet/signing/private-key-path"],
                "privateKey": "never-archive",
                "placeOrder": true,
                "notificationSent": true,
                "executionTriggered": true
            }]
        }))
        .send()
        .await
        .expect("dry-run validation response");
    assert_eq!(response.status(), reqwest::StatusCode::OK);
    let payload: serde_json::Value = response.json().await.expect("dry-run json");

    assert_eq!(payload["readOnly"], true);
    assert_eq!(payload["runtimeModified"], false);
    assert_eq!(payload["executionEnabled"], false);
    assert_eq!(payload["archiveWriteEnabled"], false);
    assert_eq!(payload["databaseWriteEnabled"], false);
    assert_eq!(payload["jsonlWriteEnabled"], false);
    assert_eq!(payload["sqliteWriteEnabled"], false);
    assert_eq!(payload["notificationSent"], false);
    assert_eq!(payload["executionTriggered"], false);
    assert_eq!(payload["validation"]["valid"], false);
    assert_eq!(payload["recordsPrepared"], 0);
    assert!(payload["validation"]["errors"]
        .as_array()
        .expect("errors")
        .iter()
        .any(|item| item == "forbidden_field_present"));
    assert!(payload["validation"]["errors"]
        .as_array()
        .expect("errors")
        .iter()
        .any(|item| item == "unsafe_execution_field_detected"));
    assert_eq!(payload["validation"]["unsafeExecutionFieldDetected"], true);

    server.abort();
}

#[tokio::test]
async fn durable_archive_dryrun_review_pack_api_returns_latest_and_by_id_read_only_payloads() {
    let state = AppState::new(test_config());
    seed_history_snapshot(&state);
    let app = router(state);
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0")
        .await
        .expect("bind");
    let addr = listener.local_addr().expect("addr");
    let server = tokio::spawn(async move {
        axum::serve(listener, app).await.expect("server");
    });

    let latest = test_http_client()
        .get(format!(
            "http://{addr}/api/archive/dry-run/review-pack/latest?symbol=BTCUSDT"
        ))
        .send()
        .await
        .expect("review pack latest response");
    assert_eq!(latest.status(), reqwest::StatusCode::OK);
    let latest_payload: serde_json::Value = latest.json().await.expect("latest review pack");
    assert_eq!(latest_payload["found"], true);
    assert_eq!(latest_payload["readOnly"], true);
    assert_eq!(latest_payload["analysisOnly"], true);
    assert_eq!(latest_payload["manualReviewRequired"], true);
    assert_eq!(latest_payload["archiveWriteEnabled"], false);
    assert_eq!(latest_payload["executionEnabled"], false);
    assert_eq!(latest_payload["notificationSent"], false);
    assert_eq!(latest_payload["executionTriggered"], false);
    assert!(latest_payload["markdown"]
        .as_str()
        .unwrap_or("")
        .contains("Durable Archive Dry-run Review Pack"));

    let dry_run_id = latest_payload["dryRunId"]
        .as_str()
        .expect("dry run id")
        .to_string();

    let by_id = test_http_client()
        .get(format!(
            "http://{addr}/api/archive/dry-run/review-pack/{dry_run_id}?symbol=BTCUSDT"
        ))
        .send()
        .await
        .expect("review pack by id response");
    assert_eq!(by_id.status(), reqwest::StatusCode::OK);
    let by_id_payload: serde_json::Value = by_id.json().await.expect("by id review pack");
    assert_eq!(by_id_payload["found"], true);
    assert_eq!(by_id_payload["dryRunId"], dry_run_id);

    let missing = test_http_client()
        .get(format!(
            "http://{addr}/api/archive/dry-run/review-pack/missing-pack?symbol=BTCUSDT"
        ))
        .send()
        .await
        .expect("review pack missing response");
    assert_eq!(missing.status(), reqwest::StatusCode::OK);
    let missing_payload: serde_json::Value = missing.json().await.expect("missing review pack");
    assert_eq!(missing_payload["found"], false);
    assert_eq!(missing_payload["archiveWriteEnabled"], false);
    assert!(missing_payload["validation"]["errors"]
        .as_array()
        .expect("errors")
        .iter()
        .any(|item| item == "review_pack_not_found"));

    server.abort();
}

fn seed_history_snapshot(state: &AppState) {
    state.signal_history_service_for_tests().record_snapshot(
        1_234,
        &inbox_recent(),
        &group_recent(),
        &alert_preview(),
        &daily_report(),
    );
}

fn inbox_recent() -> ToxicSignalInboxRecentResponse {
    ToxicSignalInboxRecentResponse {
        read_only: true,
        runtime_modified: false,
        analysis_only: true,
        execution_enabled: false,
        manual_review_required: true,
        runtime_weight_modified: false,
        config_modified: false,
        mode: "analysis_only".to_string(),
        selected_symbol: "BTCUSDT".to_string(),
        status: "signal_inbox_ready".to_string(),
        warnings: Vec::new(),
        items: vec![ToxicSignalInboxItem {
            signal_id: "signal-btc-1".to_string(),
            symbol: "BTCUSDT".to_string(),
            signal_kind: "short_bias_toxic_flow".to_string(),
            direction_bias: "short".to_string(),
            severity: "high".to_string(),
            confidence: 0.82,
            created_at_ms: 1_000,
            fusion: ToxicSignalInboxFusionSummary {
                available: true,
                summary: "signal-only".to_string(),
            },
            replay: ToxicSignalInboxReplaySummary {
                available: true,
                evidence_count: 2,
            },
            markout: ToxicSignalInboxMarkoutSummary {
                available: true,
                one_minute: "aligned".to_string(),
                five_minute: "neutral".to_string(),
                fifteen_minute: "not_enough_data".to_string(),
                one_hour: "not_enough_data".to_string(),
            },
            quality: ToxicSignalInboxQualitySummary {
                available: true,
                quality_bucket: "good".to_string(),
                aligned_ratio: 0.6,
                adverse_ratio: 0.1,
            },
            recommendation: ToxicSignalInboxRecommendationSummary {
                available: true,
                action: "keep".to_string(),
                no_trade_only: false,
                manual_review_required: true,
            },
            governance: ToxicSignalInboxGovernanceSummary {
                ledger_available: true,
                latest_decision: "watch_more".to_string(),
            },
            operator_action: ToxicSignalInboxOperatorAction::WatchSignalOnly,
            read_only: true,
            runtime_modified: false,
            analysis_only: true,
            execution_enabled: false,
        }],
    }
}

fn group_recent() -> ToxicSignalGroupRecentResponse {
    ToxicSignalGroupRecentResponse {
        read_only: true,
        runtime_modified: false,
        analysis_only: true,
        execution_enabled: false,
        manual_review_required: true,
        runtime_weight_modified: false,
        config_modified: false,
        mode: "analysis_only".to_string(),
        selected_symbol: "BTCUSDT".to_string(),
        status: "signal_groups_ready".to_string(),
        cooldown_window_ms: 300_000,
        warnings: Vec::new(),
        groups: vec![ToxicSignalGroup {
            group_id: "group-btc-1".to_string(),
            symbol: "BTCUSDT".to_string(),
            signal_kind: "short_bias_toxic_flow".to_string(),
            direction_bias: "short".to_string(),
            count: 1,
            first_seen_at_ms: 900,
            last_seen_at_ms: 1_000,
            cooldown_window_ms: 300_000,
            max_severity: "high".to_string(),
            avg_confidence: 0.82,
            representative_signal_id: "signal-btc-1".to_string(),
            member_signal_ids: vec!["signal-btc-1".to_string()],
            operator_action: ToxicSignalGroupOperatorAction::ReviewGroupedSignal,
            suppression_hint: "Grouped for display only.".to_string(),
            original_signals_preserved: true,
            representative_confidence: 0.82,
            read_only: true,
            runtime_modified: false,
            analysis_only: true,
            execution_enabled: false,
        }],
    }
}

fn alert_preview() -> ToxicSignalAlertPreviewResponse {
    ToxicSignalAlertPreviewResponse {
        read_only: true,
        runtime_modified: false,
        analysis_only: true,
        execution_enabled: false,
        manual_review_required: true,
        notification_sent: false,
        execution_triggered: false,
        preview_only: true,
        runtime_weight_modified: false,
        config_modified: false,
        mode: "notification_preview_only".to_string(),
        status: "notification_preview_ready".to_string(),
        selected_symbol: "BTCUSDT".to_string(),
        filter: ToxicSignalAlertPreviewFilter {
            symbol: "BTCUSDT".to_string(),
            view_only: true,
            persistent_watchlist_enabled: false,
            runtime_monitor_modified: false,
        },
        gate: ToxicSignalAlertPreviewGate {
            dedup_window_ms: 30_000,
            min_severity: "alert".to_string(),
            require_cross_venue: true,
            require_markout: true,
            require_liquidity_drain: false,
            telegram_enabled: false,
            notification_sent: false,
            execution_triggered: false,
        },
        summary: ToxicSignalAlertPreviewSummary {
            total_signals: 1,
            notify_candidates: 1,
            review_candidates: 0,
            suppressed_signals: 0,
            no_trade_only_signals: 0,
            governance_hold_signals: 0,
            not_enough_data_signals: 0,
        },
        by_symbol: vec![ToxicSignalAlertPreviewBucket {
            key: "BTCUSDT".to_string(),
            label: "BTCUSDT".to_string(),
            total_signals: 1,
            notify_candidates: 1,
            review_candidates: 0,
            suppressed_signals: 0,
            no_trade_only_signals: 0,
            not_enough_data_signals: 0,
        }],
        by_signal_kind: Vec::new(),
        items: vec![ToxicSignalAlertPreviewItem {
            signal_id: "signal-btc-1".to_string(),
            symbol: "BTCUSDT".to_string(),
            signal_kind: "short_bias_toxic_flow".to_string(),
            direction_bias: "short".to_string(),
            severity: "high".to_string(),
            confidence: 0.82,
            preview_status: "notify_candidate".to_string(),
            would_notify_if_enabled: true,
            no_trade_only: false,
            quality_bucket: "good".to_string(),
            latest_governance_decision: "watch_more".to_string(),
            markout_readiness: "aligned_present".to_string(),
            suppression_reasons: Vec::new(),
            review_reasons: vec!["high severity".to_string()],
            preview_message: "notify".to_string(),
            notification_sent: false,
            execution_triggered: false,
            read_only: true,
            analysis_only: true,
            execution_enabled: false,
        }],
        operator_notes: Vec::new(),
        markdown: "# Preview".to_string(),
    }
}

fn daily_report() -> ToxicSignalReportDailyResponse {
    ToxicSignalReportDailyResponse {
        read_only: true,
        runtime_modified: false,
        analysis_only: true,
        execution_enabled: false,
        manual_review_required: true,
        runtime_weight_modified: false,
        config_modified: false,
        report_type: "daily".to_string(),
        mode: "analysis_only".to_string(),
        date: "2026-05-30".to_string(),
        filter: ToxicSignalReportFilter {
            symbol: "BTCUSDT".to_string(),
            view_only: true,
            persistent_watchlist_enabled: false,
            runtime_monitor_modified: false,
        },
        summary: ToxicSignalReportSummary {
            total_signals: 1,
            grouped_signals: 1,
            high_severity_signals: 1,
            no_trade_only_candidates: 0,
            downgrade_candidates: 0,
            not_enough_data_signals: 1,
        },
        markout_summary: ToxicSignalReportMarkoutSummary {
            aligned: 1,
            adverse: 0,
            neutral: 1,
            not_enough_data: 2,
        },
        by_symbol: vec![ToxicSignalReportBucket {
            key: "BTCUSDT".to_string(),
            label: "BTCUSDT".to_string(),
            signal_count: 1,
            high_severity_signals: 1,
            no_trade_only_candidates: 0,
            downgrade_candidates: 0,
            not_enough_data_signals: 1,
            avg_confidence: 0.82,
        }],
        by_signal_kind: Vec::new(),
        top_groups: vec![ToxicSignalReportTopGroup {
            group_id: "group-btc-1".to_string(),
            symbol: "BTCUSDT".to_string(),
            signal_kind: "short_bias_toxic_flow".to_string(),
            direction_bias: "short".to_string(),
            count: 1,
            first_seen_at_ms: 900,
            last_seen_at_ms: 1_000,
            max_severity: "high".to_string(),
            avg_confidence: 0.82,
            representative_signal_id: "signal-btc-1".to_string(),
            original_signals_preserved: true,
        }],
        operator_notes: vec!["Signal-only report.".to_string()],
        markdown: "# Report".to_string(),
    }
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
