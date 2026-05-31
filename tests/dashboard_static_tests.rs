mod support;
use support::test_http_get;

use btc_toxic_flow_monitor_rs::{
    api::server::router,
    app::AppState,
    config::{
        venues::{VenueConfig, VenueConfigs},
        AppConfig,
    },
    types::{market::Venue, toxic::ToxicSeverity},
};

#[tokio::test]
async fn dashboard_static_routes_return_sorted_filtered_suspicious_order_ui() {
    let state = AppState::new(test_config());
    let app = router(state);
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0")
        .await
        .expect("bind");
    let addr = listener.local_addr().expect("addr");
    let server = tokio::spawn(async move {
        axum::serve(listener, app).await.expect("server");
    });

    let root = test_http_get(format!("http://{addr}/"))
        .await
        .expect("root response");
    assert_eq!(root.status(), reqwest::StatusCode::OK);
    let root_html = root.text().await.expect("root text");
    assert!(root_html.contains("BTC Toxic Flow Monitor"));

    let dashboard = test_http_get(format!("http://{addr}/dashboard"))
        .await
        .expect("dashboard response");
    assert_eq!(dashboard.status(), reqwest::StatusCode::OK);
    let dashboard_html = dashboard.text().await.expect("dashboard text");
    assert!(dashboard_html.contains(r#"body class="monitor-focus""#));
    assert!(dashboard_html.contains("suspiciousToxicOrdersCard"));
    assert!(dashboard_html.contains("monitorFlowCard"));
    assert!(dashboard_html.contains("venueStreamDiagnosticsCard"));
    assert!(dashboard_html.contains("Venue Stream Diagnostics"));
    assert!(dashboard_html.contains("Refresh / Copy JSON only"));
    assert!(dashboard_html.contains("whaleFlowMonitorCard"));
    assert!(dashboard_html.contains("BTC Whale / Large Flow Monitor"));
    assert!(dashboard_html.contains("whaleFlowCompactModeCard"));
    assert!(dashboard_html.contains("Whale Flow Operator Presets"));
    assert!(dashboard_html.contains("Whale Flow Compact View"));
    assert!(dashboard_html.contains("view-only"));
    assert!(dashboard_html.contains("Persistent preset disabled"));
    assert!(dashboard_html.contains("Runtime modified: false"));
    assert!(dashboard_html.contains("No threshold modified"));
    assert!(dashboard_html.contains("No config write"));
    assert!(dashboard_html.contains("No apply/reload"));
    assert!(dashboard_html.contains("whaleFlowCalibrationCard"));
    assert!(dashboard_html.contains("Whale Flow Threshold Calibration"));
    assert!(dashboard_html.contains("whaleFlowCandidateHistoryCard"));
    assert!(dashboard_html.contains("Whale Candidate History"));
    assert!(dashboard_html.contains("Bounded in-memory only"));
    assert!(dashboard_html.contains("Not durable storage"));
    assert!(dashboard_html.contains("No DB write"));
    assert!(dashboard_html.contains("No file write"));
    assert!(dashboard_html.contains("Calibration readiness requires resolved markout evidence"));
    assert!(dashboard_html.contains("suspiciousReplayCard"));
    assert!(dashboard_html.contains("suspiciousReplayDrilldownCard"));
    assert!(dashboard_html.contains("replayHeatmapCard"));

    let suspicious_card = extract_card(&dashboard_html, "suspiciousToxicOrdersCard");
    assert!(suspicious_card.contains("suspiciousToxicOrdersContent"));

    let script = test_http_get(format!("http://{addr}/web/app.js"))
        .await
        .expect("script response");
    assert_eq!(script.status(), reqwest::StatusCode::OK);
    let script_text = script.text().await.expect("script text");
    assert!(script_text.contains("renderSuspiciousToxicOrders"));
    assert!(script_text.contains("renderMonitorFlow"));
    assert!(script_text.contains("renderVenueStreamDiagnostics"));
    assert!(script_text.contains("/api/venues/diagnostics"));
    assert!(script_text.contains("Refresh Venue Diagnostics"));
    assert!(script_text.contains("Copy Venue Diagnostics JSON"));
    assert!(script_text.contains("diagnosticStatus"));
    assert!(script_text.contains("connectorConstructedVenues"));
    assert!(script_text.contains("wsConnectAttemptedVenues"));
    assert!(script_text.contains("wsConnectedVenues"));
    assert!(script_text.contains("venuesWithNetworkErrors"));
    assert!(script_text.contains("activeTradeVenues"));
    assert!(script_text.contains("activeBookVenues"));
    assert!(script_text.contains("tradeActiveVenues"));
    assert!(script_text.contains("bookActiveVenues"));
    assert!(script_text.contains("symbolMappingStatus"));
    assert!(script_text.contains("WebSocket"));
    assert!(script_text.contains("Subscription"));
    assert!(script_text.contains("Symbol Mapping"));
    assert!(script_text.contains("Proxy / Network"));
    assert!(script_text.contains("WS Error Class"));
    assert!(script_text.contains("Ack Mode"));
    assert!(script_text.contains("renderWhaleFlowMonitor"));
    assert!(script_text.contains("refreshWhaleFlowMonitorButton"));
    assert!(script_text.contains("copyWhaleFlowMonitorJsonButton"));
    assert!(script_text.contains("renderWhaleFlowCompactMode"));
    assert!(script_text.contains("whaleFlowCompactModeBadge"));
    assert!(script_text.contains("whaleFlowCompactPresetDefinitions"));
    assert!(script_text.contains("\"all\", \"All\""));
    assert!(script_text.contains("\"high_volume\", \"High Volume\""));
    assert!(script_text.contains("\"venue_confluence_satisfied\", \"Venue Confluence\""));
    assert!(script_text.contains("\"degraded_or_partial_data\", \"Degraded Data\""));
    assert!(script_text.contains("\"calibration_not_ready\", \"Calibration Not Ready\""));
    assert!(script_text.contains("\"needs_more_data\", \"Needs More Data\""));
    assert!(script_text.contains("\"not_enough_data\", \"Not Enough Data\""));
    assert!(script_text.contains("Reset Preset"));
    assert!(script_text.contains("Copy Preset View JSON"));
    assert!(script_text.contains("Current Preset"));
    assert!(script_text.contains("Persistent preset disabled"));
    assert!(script_text.contains("runtimePresetModified"));
    assert!(script_text.contains("selectedPreset"));
    assert!(script_text.contains("matchedItems"));
    assert!(script_text.contains("No whale flow items matched this preset"));
    assert!(script_text.contains("No high volume candidates"));
    assert!(script_text.contains("No degraded data quality candidates"));
    assert!(script_text.contains("No calibration blocked candidates"));
    assert!(script_text.contains("No needs_more_data candidates"));
    assert!(script_text.contains("No not_enough_data candidates"));
    assert!(script_text.contains("No Whale Flow"));
    assert!(script_text.contains("Data insufficient"));
    assert!(script_text.contains("formatWhaleFlowBaselineSource"));
    assert!(script_text.contains("formatWhaleFlowQualityStatus"));
    assert!(script_text.contains("whaleFlowQualityTone"));
    assert!(script_text.contains("No-candidate Reasons"));
    assert!(script_text.contains("Degradation Warnings"));
    assert!(script_text.contains("Baseline Source"));
    assert!(script_text.contains("Venue Coverage"));
    assert!(script_text.contains("Data Quality"));
    assert!(script_text.contains("minVenueConfluenceRequired"));
    assert!(script_text.contains("venueConfluenceSatisfied"));
    assert!(script_text.contains("baselineSource"));
    assert!(script_text.contains("/api/toxicity/whale-flow/status"));
    assert!(script_text.contains("/api/toxicity/whale-flow/recent"));
    assert!(script_text.contains("/api/toxicity/whale-flow/:symbol"));
    assert!(script_text.contains("Market Data Quality"));
    assert!(script_text.contains("monitorQualityTone"));
    assert!(script_text.contains("monitor-quality-strip"));
    assert!(script_text.contains("Lagged Events"));
    assert!(script_text.contains("Dropped Events"));
    assert!(script_text.contains("Flow Windows Populated"));
    assert!(script_text.contains("数据质量降级，当前空列表不能直接理解为无有毒订单。"));
    assert!(script_text.contains("venueStatusSummary"));
    assert!(script_text.contains("not true in current process"));
    assert!(script_text.contains("enableSource"));
    assert!(script_text.contains("renderSuspiciousToxicOrderItem"));
    assert!(script_text.contains("renderSuspiciousReplayDrilldown"));
    assert!(script_text.contains("renderReplayOverlayMetricRows"));
    assert!(script_text.contains("renderSuspiciousReplayOverlaySummary"));
    assert!(script_text.contains("buildWhaleReplayOverlay"));
    assert!(script_text.contains("buildWhaleReplayOverlayMarkdown"));
    assert!(script_text.contains("Refresh Whale Overlay"));
    assert!(script_text.contains("Load Whale Overlay by Symbol"));
    assert!(script_text.contains("Load Whale Overlay by Signal ID"));
    assert!(script_text.contains("Copy Whale Overlay JSON"));
    assert!(script_text.contains("Copy Whale Overlay Markdown"));
    assert!(script_text.contains("Whale Flow Overlay"));
    assert!(script_text.contains("/api/toxicity/whale-flow/history/status"));
    assert!(script_text.contains("/api/toxicity/whale-flow/history/recent"));
    assert!(script_text.contains("/api/toxicity/whale-flow/history/:symbol"));
    assert!(script_text.contains("Refresh Whale History"));
    assert!(script_text.contains("Load Whale History by Symbol"));
    assert!(script_text.contains("Copy Whale History JSON"));
    assert!(script_text.contains("Calibration not ready"));
    assert!(script_text.contains("No whale flow candidate for selected signal."));
    assert!(script_text
        .contains("Whale flow overlay partial: venue/depth/baseline inputs are missing."));
    assert!(script_text.contains("renderWhaleFlowCalibration"));
    assert!(script_text.contains("/api/toxicity/whale-flow/calibration/status"));
    assert!(script_text.contains("/api/toxicity/whale-flow/calibration/report"));
    assert!(script_text.contains("/api/toxicity/whale-flow/calibration/:symbol"));
    assert!(script_text.contains("Refresh Calibration Report"));
    assert!(script_text.contains("Load Calibration by Symbol"));
    assert!(script_text.contains("Copy Calibration JSON"));
    assert!(script_text.contains("Copy Calibration Markdown"));
    assert!(script_text.contains("No whale flow candidates available"));
    assert!(script_text.contains("Calibration evidence too thin"));
    assert!(script_text.contains("Evidence Source"));
    assert!(script_text.contains("Outcome Linkage"));
    assert!(script_text.contains("usesCurrentSnapshotOnly"));
    assert!(script_text.contains("resolvedMarkoutEvidenceCount"));
    assert!(script_text.contains("Baseline insufficient"));
    assert!(script_text.contains("Markout not_enough_data"));
    assert!(script_text.contains("whaleClassification"));
    assert!(script_text.contains("baselineSource"));
    assert!(script_text.contains("dataQuality"));
    assert!(script_text.contains("renderReplayHeatmap"));
    assert!(script_text.contains("buildReplayHeatmapPayload"));
    assert!(script_text.contains("suspiciousOrdersVisibleItems"));
    assert!(script_text.contains("normalizeSuspiciousStatus"));
    assert!(script_text.contains("suspiciousOrdersSortSelect"));
    assert!(script_text.contains("suspiciousOrdersFilterSymbolInput"));
    assert!(script_text.contains("suspiciousOrdersFilterAlertDecisionInput"));
    assert!(script_text.contains("suspiciousOrdersHideNotEnoughDataCheckbox"));
    assert!(script_text.contains("suspiciousOrdersHighSeverityOnlyCheckbox"));
    assert!(script_text.contains("clearSuspiciousOrdersFilterButton"));
    assert!(script_text.contains("resetSuspiciousOrdersSortButton"));
    assert!(script_text.contains("Sort by"));
    assert!(script_text.contains("Severity"));
    assert!(script_text.contains("Confidence"));
    assert!(script_text.contains("CreatedAt"));
    assert!(script_text.contains("Hide not_enough_data"));
    assert!(script_text.contains("High severity only"));
    assert!(script_text.contains("Clear Filter"));
    assert!(script_text.contains("Reset Sort"));
    assert!(script_text.contains("alertDecision"));
    assert!(script_text.contains("status"));
    assert!(script_text.contains("suspiciousOrdersSummaryText"));
    assert!(script_text.contains("readOnly=true"));
    assert!(script_text.contains("analysisOnly=true"));
    assert!(script_text.contains("executionEnabled=false"));
    assert!(script_text.contains("view-only"));
    assert!(script_text.contains("persistentWatchlistEnabled=false"));
    assert!(script_text.contains("runtimeMonitorModified=false"));
    assert!(script_text.contains("No matches"));
    assert!(script_text.contains("No history available"));
    assert!(script_text.contains("Signal not found"));
    assert!(script_text.contains("Detail unavailable"));
    assert!(script_text.contains("Alert explanation unavailable"));
    assert!(script_text.contains("Governance ledger unavailable"));
    assert!(script_text.contains("Markout not_enough_data"));
    assert!(script_text.contains("Markout / Quality / Recommendation Overlay"));
    assert!(script_text.contains("Replay Markout Heatmap"));
    assert!(script_text.contains("Build Heatmap"));
    assert!(script_text.contains("Refresh Heatmap"));
    assert!(script_text.contains("Clear Heatmap Filter"));
    assert!(script_text.contains("Copy Heatmap JSON"));
    assert!(script_text.contains("Copy Heatmap Markdown"));
    assert!(script_text.contains("+1m"));
    assert!(script_text.contains("+5m"));
    assert!(script_text.contains("+15m"));
    assert!(script_text.contains("+1h"));
    assert!(script_text.contains("No signals matched filter"));
    assert!(script_text.contains("Insufficient samples for heatmap"));
    assert!(script_text.contains("Direction bias is a signal attribute, not an order instruction."));
    assert!(script_text.contains("/api/toxicity/signal-inbox/recent"));
    assert!(script_text.contains("/api/toxicity/signal-groups/recent"));
    assert!(script_text.contains("/api/toxicity/signal-history/recent"));
    assert!(script_text.contains("/api/toxicity/signal-history/:symbol"));
    assert!(script_text.contains("/api/toxicity/signal-history/signal/:signal_id"));
    assert!(script_text.contains("/api/toxicity/signal-detail/:signal_id"));
    assert!(script_text.contains("/api/toxicity/signal-alert-preview/explain/:signal_id"));
    assert!(script_text.contains("/api/toxicity/markout/:symbol"));
    assert!(script_text.contains("/api/toxicity/quality-scorecard/:symbol"));
    assert!(script_text.contains("/api/toxicity/weight-recommendation/:symbol"));
    assert!(script_text.contains("/api/toxicity/weight-review/:symbol"));
    assert!(script_text.contains("/api/toxicity/governance-ledger/:symbol"));
    assert!(!script_text.contains("Apply Threshold"));
    assert!(!script_text.contains("Apply Preset"));
    assert!(!script_text.contains("Save Preset"));
    assert!(!script_text.contains("Persist Preset"));
    assert!(!script_text.contains("Update Threshold"));
    assert!(!script_text.contains("Save Config"));
    assert!(!script_text.contains("Patch Config"));
    assert!(!script_text.contains("localStorage"));
    assert!(!script_text.contains("sessionStorage"));
    assert!(!script_text.contains("/api/toxicity/suspicious-replay/:signal_id"));
    assert!(!script_text.contains(r#"$("refreshButton").addEventListener"#));

    let styles = test_http_get(format!("http://{addr}/web/styles.css"))
        .await
        .expect("styles response");
    assert_eq!(styles.status(), reqwest::StatusCode::OK);
    let styles_text = styles.text().await.expect("styles text");
    assert!(styles_text.contains(".suspicious-controls"));
    assert!(styles_text.contains(".control-grid"));
    assert!(styles_text.contains(".checkbox-field"));
    assert!(styles_text.contains(".suspicious-summary"));
    assert!(styles_text.contains(".suspicious-list"));
    assert!(styles_text.contains(".suspicious-order"));
    assert!(styles_text.contains(".suspicious-empty"));
    assert!(styles_text.contains(".monitor-quality-strip"));
    assert!(styles_text.contains(".whale-flow-table"));
    assert!(styles_text.contains(".whale-flow-empty"));
    assert!(styles_text.contains("@media (prefers-reduced-motion: reduce)"));
    assert!(styles_text.contains(".monitor-quality-warning"));
    assert!(styles_text.contains(".venue-diagnostics-notes"));
    assert!(styles_text.contains(".venue-diagnostics-table"));
    assert!(styles_text.contains(".venue-diagnostics-sections"));
    assert!(styles_text.contains(".replay-panel"));
    assert!(styles_text.contains(".replay-history-list"));
    assert!(styles_text.contains(".heatmap-controls"));
    assert!(styles_text.contains(".heatmap-group-list"));
    assert!(styles_text.contains(".heatmap-window-grid"));
    assert!(styles_text.contains(".heatmap-empty"));
    assert!(styles_text.contains(".compact-preset-grid"));
    assert!(styles_text.contains(".compact-summary"));
    assert!(styles_text.contains(".compact-list"));

    server.abort();
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
    }
}

fn extract_card<'a>(html: &'a str, card_id: &str) -> &'a str {
    let start_marker = format!(r#"<article class="card" id="{card_id}">"#);
    let start = html.find(&start_marker).expect("card start");
    let tail = &html[start..];
    let end = tail.find("</article>").expect("card end") + "</article>".len();
    &tail[..end]
}
