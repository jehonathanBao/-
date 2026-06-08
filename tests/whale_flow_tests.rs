use std::collections::BTreeMap;

use btc_toxic_flow_monitor_rs::{
    config::{
        venues::{VenueConfig, VenueConfigs},
        AppConfig,
    },
    market_data::quality::MarketDataQualitySnapshot,
    toxicity::{
        whale_flow_monitor::WhaleFlowAnalysisInputs, whale_flow_service::build_whale_flow_recent,
    },
    types::{
        flow::{DataQuality, FlowState, FlowWindow, VenueFlowBreakdown},
        liquidation::LiquidationToxicityRecentResponse,
        market::{Venue, VenueConnectionStatus, VenueHealth},
        orderbook_wall::{
            OrderbookWallInterpretationReport, OrderbookWallInterpretationSignal,
            OrderbookWallInterpretationType, OrderbookWallLifecycleReport, OrderbookWallSide,
        },
        status::VenueHealthMap,
        structural_toxicity::StructuralToxicityRecentResponse,
        sweep::{SweepQuality, SweepState},
        toxic::ToxicSeverity,
        toxic_flow::{ActiveTradeToxicityRecentResponse, ToxicConfidence, ToxicSide},
        toxic_signal::ToxicSignalRecentResponse,
        whale_flow_signal::WhaleFlowCandidateType,
    },
};

#[test]
fn whale_flow_generates_aggressive_buy_candidate_without_mutating_inputs() {
    let flow_state = sample_flow_state();
    let flow_before = serde_json::to_value(&flow_state).expect("flow before");
    let sweep_state = empty_sweep_state();
    let active_trade_recent = empty_active_trade_recent();
    let liquidation_recent = empty_liquidation_recent();
    let wall_lifecycle_report = empty_wall_lifecycle_report();
    let wall_interpretation_report = empty_wall_interpretation_report();
    let structural_recent = empty_structural_recent();
    let fusion_recent = empty_fusion_recent();
    let config = test_config();
    let venue_health = sample_venue_health();
    let inputs = WhaleFlowAnalysisInputs {
        requested_symbol: "BTC-PERP",
        config: &config,
        venue_health: &venue_health,
        flow_state: &flow_state,
        sweep_state: &sweep_state,
        market_data_quality: sample_quality_snapshot(),
        active_trade_recent: &active_trade_recent,
        liquidation_recent: &liquidation_recent,
        wall_lifecycle_report: &wall_lifecycle_report,
        wall_interpretation_report: &wall_interpretation_report,
        structural_recent: &structural_recent,
        fusion_recent: &fusion_recent,
    };
    let recent = build_whale_flow_recent(&inputs);

    assert!(recent.read_only);
    assert!(recent.analysis_only);
    assert!(!recent.execution_enabled);
    assert_eq!(recent.status, "candidate_active");
    assert_eq!(recent.candidates.len(), 1);
    assert_eq!(recent.venue_coverage.enabled_venues, 3);
    assert_eq!(recent.venue_coverage.connected_venues, 2);
    assert_eq!(recent.venue_coverage.active_trade_venues, 2);
    assert_eq!(recent.venue_coverage.active_book_venues, 0);
    assert_eq!(
        recent.baseline_quality.baseline_source,
        "sixty_second_fallback"
    );
    assert!(recent.baseline_quality.fallback_used);
    assert!(recent.candidates[0]
        .diagnostics
        .confidence_modifiers
        .contains(&"baseline_fallback_used".to_string()));
    assert_eq!(
        recent.candidates[0].candidate_type,
        WhaleFlowCandidateType::AggressiveBuy
    );
    assert_eq!(recent.candidates[0].direction, ToxicSide::Buy);
    assert!(recent
        .warnings
        .iter()
        .any(|warning| warning.contains("1h baseline is unavailable")));
    assert_eq!(
        flow_before,
        serde_json::to_value(&flow_state).expect("flow after")
    );
}

#[test]
fn whale_flow_promotes_absorption_when_wall_evidence_exists() {
    let wall_report = OrderbookWallInterpretationReport {
        read_only: true,
        runtime_modified: false,
        analysis_mode: "analysis_only".to_string(),
        selected_symbol: "BTC-PERP".to_string(),
        generated_at_ms: 5_000,
        status: "ready".to_string(),
        signals: vec![OrderbookWallInterpretationSignal {
            signal_id: "wall-ask-absorption".to_string(),
            symbol: "BTC-PERP".to_string(),
            ts_ms: 5_000,
            wall_id: "ask-wall-1".to_string(),
            signal_type: OrderbookWallInterpretationType::AskAbsorption,
            side: OrderbookWallSide::Ask,
            wall_price: 100_050.0,
            wall_notional_usd: 50_000_000.0,
            distance_to_mid_bps: 4.0,
            persistence_ms: 6_000,
            touch_count: 3,
            consumed_ratio: 0.65,
            cancel_ratio: 0.10,
            moved_count: 0,
            aggressive_volume_against_wall: Some(420.0),
            post_touch_markout_bps: Some(0.4),
            spoof_score: 8,
            absorption_score: 93,
            inducement_score: 15,
            toxicity_score: 90,
            confidence: ToxicConfidence::High,
            reason: vec!["ask wall held while aggressive buy volume stayed elevated".to_string()],
            read_only: true,
        }],
        warnings: Vec::new(),
        no_trade_reasons: Vec::new(),
    };

    let flow_state = sample_flow_state();
    let sweep_state = empty_sweep_state();
    let active_trade_recent = empty_active_trade_recent();
    let liquidation_recent = empty_liquidation_recent();
    let wall_lifecycle_report = empty_wall_lifecycle_report();
    let structural_recent = empty_structural_recent();
    let fusion_recent = empty_fusion_recent();
    let config = test_config();
    let venue_health = sample_venue_health();
    let inputs = WhaleFlowAnalysisInputs {
        requested_symbol: "BTC-PERP",
        config: &config,
        venue_health: &venue_health,
        flow_state: &flow_state,
        sweep_state: &sweep_state,
        market_data_quality: sample_quality_snapshot(),
        active_trade_recent: &active_trade_recent,
        liquidation_recent: &liquidation_recent,
        wall_lifecycle_report: &wall_lifecycle_report,
        wall_interpretation_report: &wall_report,
        structural_recent: &structural_recent,
        fusion_recent: &fusion_recent,
    };
    let recent = build_whale_flow_recent(&inputs);

    assert_eq!(recent.candidates.len(), 1);
    assert_eq!(
        recent.candidates[0].candidate_type,
        WhaleFlowCandidateType::Absorption
    );
    assert_eq!(
        recent.candidates[0].linked_wall_interpretation_signal_ids,
        vec!["wall-ask-absorption".to_string()]
    );
    assert!(recent.candidates[0]
        .diagnostics
        .why_candidate
        .iter()
        .any(|item| item.contains("venues confirmed")));
}

#[test]
fn whale_flow_reports_data_insufficient_when_trade_windows_are_empty() {
    let flow_state = FlowState {
        symbol: "BTC-PERP".to_string(),
        updated_at: 0,
        windows: BTreeMap::new(),
    };
    let sweep_state = empty_sweep_state();
    let active_trade_recent = empty_active_trade_recent();
    let liquidation_recent = empty_liquidation_recent();
    let wall_lifecycle_report = empty_wall_lifecycle_report();
    let wall_interpretation_report = empty_wall_interpretation_report();
    let structural_recent = empty_structural_recent();
    let fusion_recent = empty_fusion_recent();
    let config = test_config();
    let venue_health = sample_venue_health();
    let inputs = WhaleFlowAnalysisInputs {
        requested_symbol: "BTC-PERP",
        config: &config,
        venue_health: &venue_health,
        flow_state: &flow_state,
        sweep_state: &sweep_state,
        market_data_quality: MarketDataQualitySnapshot::default(),
        active_trade_recent: &active_trade_recent,
        liquidation_recent: &liquidation_recent,
        wall_lifecycle_report: &wall_lifecycle_report,
        wall_interpretation_report: &wall_interpretation_report,
        structural_recent: &structural_recent,
        fusion_recent: &fusion_recent,
    };
    let recent = build_whale_flow_recent(&inputs);

    assert_eq!(recent.status, "data_insufficient");
    assert!(recent.candidates.is_empty());
    assert!(recent
        .warnings
        .iter()
        .any(|warning| warning.contains("Data insufficient")));
    assert!(recent.no_candidate_reasons.is_empty());
}

#[test]
fn whale_flow_explains_baseline_insufficient_and_missing_depth_when_no_candidate() {
    let mut flow_state = sample_flow_state();
    flow_state.windows.remove("60000");
    for window in flow_state.windows.values_mut() {
        window.data_quality.has_books = false;
        window.data_quality.active_venues = vec!["binance".to_string()];
        window.aggressive_buy_btc = 120.0;
        window.aggressive_sell_btc = 30.0;
        window.net_aggressive_btc = 90.0;
        window.abs_aggressive_btc = 150.0;
        window.venue_breakdown.remove("bybit");
    }

    let sweep_state = empty_sweep_state();
    let active_trade_recent = empty_active_trade_recent();
    let liquidation_recent = empty_liquidation_recent();
    let wall_lifecycle_report = empty_wall_lifecycle_report();
    let wall_interpretation_report = empty_wall_interpretation_report();
    let structural_recent = empty_structural_recent();
    let fusion_recent = empty_fusion_recent();
    let config = test_config();
    let venue_health = sample_venue_health();
    let inputs = WhaleFlowAnalysisInputs {
        requested_symbol: "BTC-PERP",
        config: &config,
        venue_health: &venue_health,
        flow_state: &flow_state,
        sweep_state: &sweep_state,
        market_data_quality: MarketDataQualitySnapshot::default(),
        active_trade_recent: &active_trade_recent,
        liquidation_recent: &liquidation_recent,
        wall_lifecycle_report: &wall_lifecycle_report,
        wall_interpretation_report: &wall_interpretation_report,
        structural_recent: &structural_recent,
        fusion_recent: &fusion_recent,
    };
    let recent = build_whale_flow_recent(&inputs);

    assert_eq!(recent.status, "no_whale_flow");
    assert_eq!(
        recent.baseline_quality.baseline_source,
        "longer_window_fallback"
    );
    assert!(recent
        .no_candidate_reasons
        .iter()
        .any(|reason| reason.contains("venue confluence below required minimum")));
    assert!(recent
        .degradation_warnings
        .iter()
        .any(|reason| reason.contains("depth unavailable")));
    assert_eq!(recent.data_quality.status, "partial");
}

fn sample_flow_state() -> FlowState {
    let mut windows = BTreeMap::new();
    windows.insert(
        "1000".to_string(),
        flow_window(1_000, 30.0, 12.0, Some(0.3)),
    );
    windows.insert(
        "5000".to_string(),
        flow_window(5_000, 420.0, 60.0, Some(2.6)),
    );
    windows.insert(
        "15000".to_string(),
        flow_window(15_000, 520.0, 180.0, Some(1.0)),
    );
    windows.insert(
        "60000".to_string(),
        flow_window(60_000, 700.0, 200.0, Some(0.8)),
    );
    FlowState {
        symbol: "BTC-PERP".to_string(),
        updated_at: 5_000,
        windows,
    }
}

fn flow_window(
    window_ms: u64,
    aggressive_buy_btc: f64,
    aggressive_sell_btc: f64,
    price_move_bps: Option<f64>,
) -> FlowWindow {
    let mut venue_breakdown = BTreeMap::new();
    venue_breakdown.insert(
        "binance".to_string(),
        VenueFlowBreakdown {
            aggressive_buy_btc: aggressive_buy_btc * 0.55,
            aggressive_sell_btc: aggressive_sell_btc * 0.35,
            aggressive_buy_usd: 0.0,
            aggressive_sell_usd: 0.0,
            net_aggressive_btc: aggressive_buy_btc * 0.55 - aggressive_sell_btc * 0.35,
            abs_aggressive_btc: aggressive_buy_btc * 0.55 + aggressive_sell_btc * 0.35,
            trade_count: 8,
            buy_trade_count: 5,
            sell_trade_count: 3,
            last_trade_ts: Some(5_000),
        },
    );
    venue_breakdown.insert(
        "bybit".to_string(),
        VenueFlowBreakdown {
            aggressive_buy_btc: aggressive_buy_btc * 0.35,
            aggressive_sell_btc: aggressive_sell_btc * 0.25,
            aggressive_buy_usd: 0.0,
            aggressive_sell_usd: 0.0,
            net_aggressive_btc: aggressive_buy_btc * 0.35 - aggressive_sell_btc * 0.25,
            abs_aggressive_btc: aggressive_buy_btc * 0.35 + aggressive_sell_btc * 0.25,
            trade_count: 6,
            buy_trade_count: 4,
            sell_trade_count: 2,
            last_trade_ts: Some(5_000),
        },
    );
    venue_breakdown.insert(
        "okx".to_string(),
        VenueFlowBreakdown {
            aggressive_buy_btc: aggressive_buy_btc * 0.10,
            aggressive_sell_btc: aggressive_sell_btc * 0.40,
            aggressive_buy_usd: 0.0,
            aggressive_sell_usd: 0.0,
            net_aggressive_btc: aggressive_buy_btc * 0.10 - aggressive_sell_btc * 0.40,
            abs_aggressive_btc: aggressive_buy_btc * 0.10 + aggressive_sell_btc * 0.40,
            trade_count: 3,
            buy_trade_count: 1,
            sell_trade_count: 2,
            last_trade_ts: Some(5_000),
        },
    );

    FlowWindow {
        symbol: "BTC-PERP".to_string(),
        window_ms,
        now_ts: 5_000,
        aggressive_buy_btc,
        aggressive_sell_btc,
        aggressive_buy_usd: 0.0,
        aggressive_sell_usd: 0.0,
        net_aggressive_btc: aggressive_buy_btc - aggressive_sell_btc,
        abs_aggressive_btc: aggressive_buy_btc + aggressive_sell_btc,
        trade_count: 12,
        buy_trade_count: 8,
        sell_trade_count: 4,
        avg_trade_size_btc: 12.0,
        max_trade_size_btc: aggressive_buy_btc.max(aggressive_sell_btc) / 2.0,
        venue_breakdown,
        mid_start: Some(100_000.0),
        mid_end: Some(100_120.0),
        price_move_bps,
        spread_bps_median: Some(0.8),
        imbalance_10bps_median: Some(0.22),
        data_quality: DataQuality {
            has_trades: true,
            has_books: true,
            active_venues: vec!["binance".to_string(), "bybit".to_string()],
            stale_venues: Vec::new(),
        },
    }
}

fn sample_quality_snapshot() -> MarketDataQualitySnapshot {
    MarketDataQualitySnapshot {
        event_bus_dropped_events: 1,
        event_bus_send_errors: 1,
        flow_window_lagged_events: 3,
        markout_lagged_events: 0,
        vpin_lagged_events: 0,
        last_lagged_at_ms: Some(4_000),
    }
}

fn empty_sweep_state() -> SweepState {
    SweepState {
        symbol: "BTC-PERP".to_string(),
        updated_at: 5_000,
        windows_ms: vec![1_000, 5_000, 15_000],
        results: BTreeMap::new(),
        quality: SweepQuality {
            has_trades: true,
            has_books: true,
            active_venues: Vec::new(),
            stale_venues: Vec::new(),
        },
    }
}

fn empty_active_trade_recent() -> ActiveTradeToxicityRecentResponse {
    ActiveTradeToxicityRecentResponse {
        read_only: true,
        runtime_modified: false,
        mode: "analysis_only".to_string(),
        selected_symbol: "BTC-PERP".to_string(),
        status: "neutral".to_string(),
        score: 0.0,
        side_bias: "neutral".to_string(),
        warnings: Vec::new(),
        no_trade_reasons: Vec::new(),
        signals: Vec::new(),
    }
}

fn empty_liquidation_recent() -> LiquidationToxicityRecentResponse {
    LiquidationToxicityRecentResponse {
        read_only: true,
        runtime_modified: false,
        mode: "analysis_only".to_string(),
        selected_symbol: "BTC-PERP".to_string(),
        warnings: Vec::new(),
        no_trade_reasons: Vec::new(),
        signals: Vec::new(),
    }
}

fn empty_wall_lifecycle_report() -> OrderbookWallLifecycleReport {
    OrderbookWallLifecycleReport {
        read_only: true,
        runtime_modified: false,
        analysis_mode: "analysis_only".to_string(),
        symbol: "BTC-PERP".to_string(),
        generated_at_ms: 5_000,
        status: "ready".to_string(),
        tracked_walls: Vec::new(),
        recent_events: Vec::new(),
        toxicity_candidates: Vec::new(),
        warnings: Vec::new(),
        no_trade_reasons: Vec::new(),
    }
}

fn empty_wall_interpretation_report() -> OrderbookWallInterpretationReport {
    OrderbookWallInterpretationReport {
        read_only: true,
        runtime_modified: false,
        analysis_mode: "analysis_only".to_string(),
        selected_symbol: "BTC-PERP".to_string(),
        generated_at_ms: 5_000,
        status: "ready".to_string(),
        signals: Vec::new(),
        warnings: Vec::new(),
        no_trade_reasons: Vec::new(),
    }
}

fn empty_structural_recent() -> StructuralToxicityRecentResponse {
    StructuralToxicityRecentResponse {
        read_only: true,
        runtime_modified: false,
        mode: "analysis_only".to_string(),
        selected_symbol: "BTC-PERP".to_string(),
        status: "neutral".to_string(),
        warnings: Vec::new(),
        no_trade_reasons: Vec::new(),
        signals: Vec::new(),
    }
}

fn empty_fusion_recent() -> ToxicSignalRecentResponse {
    ToxicSignalRecentResponse {
        read_only: true,
        runtime_modified: false,
        analysis_only: true,
        execution_enabled: false,
        mode: "analysis_only".to_string(),
        selected_symbol: "BTC-PERP".to_string(),
        status: "neutral".to_string(),
        warnings: Vec::new(),
        no_trade_reasons: Vec::new(),
        signals: Vec::new(),
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
                enabled: true,
            },
            bybit: VenueConfig {
                venue: Venue::Bybit,
                enabled: true,
            },
            okx: VenueConfig {
                venue: Venue::Okx,
                enabled: true,
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
        replay_enabled: false,
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
    }
}

fn sample_venue_health() -> VenueHealthMap {
    let mut venues = VenueHealthMap::new();
    venues.insert("binance".to_string(), connected_venue(Venue::Binance, true));
    venues.insert("bybit".to_string(), connected_venue(Venue::Bybit, false));
    venues.insert("okx".to_string(), disconnected_venue(Venue::Okx));
    venues
}

fn connected_venue(venue: Venue, with_book: bool) -> VenueHealth {
    let mut health = VenueHealth::from_config(venue, true);
    health.status = VenueConnectionStatus::Connected;
    health.last_trade_ts = Some(5_000);
    health.last_book_ts = with_book.then_some(5_000);
    health.start_attempted = true;
    health.connector_constructed = true;
    health
}

fn disconnected_venue(venue: Venue) -> VenueHealth {
    let mut health = VenueHealth::from_config(venue, true);
    health.status = VenueConnectionStatus::Disconnected;
    health.start_attempted = true;
    health.connector_constructed = true;
    health
}
