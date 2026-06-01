use std::{
    collections::BTreeMap,
    fs,
    sync::Arc,
    time::{SystemTime, UNIX_EPOCH},
};

use parking_lot::RwLock;

use btc_toxic_flow_monitor_rs::{
    alerts::{
        alert_service::{AlertService, ToxicStateSource},
        sidecar::ToxicFlowSidecarWriter,
        telegram::TelegramClient,
    },
    config::thresholds::AlertGateConfig,
    types::{
        market::Venue,
        sweep::LiquidityThinnessResult,
        toxic::{
            ToxicDirection, ToxicEvent, ToxicQuality, ToxicSeverity, ToxicState, ToxicVolumeResult,
        },
    },
};

#[tokio::test]
async fn severity_below_threshold_is_not_sent() {
    let source = mock_source(build_state(build_event(
        ToxicDirection::Buy,
        ToxicSeverity::Watch,
        false,
        true,
        false,
    )));
    let client = TelegramClient::mock_success(true);
    let service =
        AlertService::with_client(source, client.clone(), AlertGateConfig::default(), 250);

    service.process_once_for_tests(1).await;

    let state = service.get_state();
    assert_eq!(client.sent_messages().len(), 0);
    assert_eq!(state.sent_count, 0);
    assert_eq!(state.suppressed_count, 1);
}

#[tokio::test]
async fn alert_is_sent_when_gates_pass() {
    let source = mock_source(build_state(build_event(
        ToxicDirection::Buy,
        ToxicSeverity::Alert,
        true,
        true,
        true,
    )));
    let client = TelegramClient::mock_success(true);
    let service =
        AlertService::with_client(source, client.clone(), AlertGateConfig::default(), 250);

    service.process_once_for_tests(1).await;

    let state = service.get_state();
    assert_eq!(client.sent_messages().len(), 1);
    assert_eq!(state.sent_count, 1);
    assert_eq!(state.suppressed_count, 0);
    assert!(client.sent_messages()[0].contains("BTC Perp Toxic Flow Alert"));
}

#[tokio::test]
async fn cross_venue_gate_suppresses_when_missing() {
    let source = mock_source(build_state(build_event(
        ToxicDirection::Buy,
        ToxicSeverity::Alert,
        false,
        true,
        true,
    )));
    let client = TelegramClient::mock_success(true);
    let service =
        AlertService::with_client(source, client.clone(), AlertGateConfig::default(), 250);

    service.process_once_for_tests(1).await;

    let state = service.get_state();
    assert_eq!(client.sent_messages().len(), 0);
    assert_eq!(state.sent_count, 0);
    assert_eq!(state.suppressed_count, 1);
}

#[tokio::test]
async fn markout_gate_suppresses_when_missing() {
    let source = mock_source(build_state(build_event(
        ToxicDirection::Sell,
        ToxicSeverity::Alert,
        true,
        false,
        true,
    )));
    let client = TelegramClient::mock_success(true);
    let service =
        AlertService::with_client(source, client.clone(), AlertGateConfig::default(), 250);

    service.process_once_for_tests(1).await;

    let state = service.get_state();
    assert_eq!(client.sent_messages().len(), 0);
    assert_eq!(state.sent_count, 0);
    assert_eq!(state.suppressed_count, 1);
}

#[tokio::test]
async fn disabled_telegram_is_noop() {
    let source = mock_source(build_state(build_event(
        ToxicDirection::Sell,
        ToxicSeverity::Extreme,
        true,
        true,
        true,
    )));
    let client = TelegramClient::mock_success(false);
    let service =
        AlertService::with_client(source, client.clone(), AlertGateConfig::default(), 250);

    service.process_once_for_tests(1).await;

    let state = service.get_state();
    assert_eq!(client.sent_messages().len(), 0);
    assert_eq!(state.sent_count, 0);
    assert_eq!(state.suppressed_count, 0);
}

#[tokio::test]
async fn sidecar_jsonl_is_written_when_gates_pass_and_telegram_is_disabled() {
    let source = mock_source(build_state(build_event(
        ToxicDirection::Sell,
        ToxicSeverity::Extreme,
        true,
        true,
        true,
    )));
    let client = TelegramClient::mock_success(false);
    let events_path = temp_events_path("toxic-flow-sidecar-alert");
    let sidecar_writer =
        ToxicFlowSidecarWriter::new(true, Some(events_path.to_string_lossy().to_string()));
    let service = AlertService::with_client_and_sidecar(
        source,
        client.clone(),
        sidecar_writer,
        AlertGateConfig::default(),
        250,
    );

    service.process_once_for_tests(1).await;
    service.process_once_for_tests(10).await;

    let state = service.get_state();
    assert_eq!(client.sent_messages().len(), 0);
    assert_eq!(state.sent_count, 1);
    assert_eq!(state.suppressed_count, 1);

    let content = fs::read_to_string(events_path).expect("sidecar events jsonl");
    let lines = content.lines().collect::<Vec<_>>();
    assert_eq!(lines.len(), 1);
    let event: serde_json::Value = serde_json::from_str(lines[0]).expect("sidecar json");
    assert_eq!(event["schemaVersion"], "toxic-flow-rs.sidecar.v1");
    assert_eq!(event["source"], "toxic-flow-rs");
    assert_eq!(event["kind"], "toxic_flow_spike");
    assert_eq!(event["severity"], "critical");
    assert_eq!(event["payload"]["readOnly"], true);
    assert_eq!(event["payload"]["direction"], "sell");
    assert!(!event.as_object().unwrap().contains_key("webhook"));
    assert!(!event.as_object().unwrap().contains_key("discord"));
}

#[tokio::test]
async fn telegram_failure_is_recorded_without_panicking() {
    let source = mock_source(build_state(build_event(
        ToxicDirection::Buy,
        ToxicSeverity::Extreme,
        true,
        true,
        true,
    )));
    let client = TelegramClient::mock_failure(true);
    let service =
        AlertService::with_client(source, client.clone(), AlertGateConfig::default(), 250);

    service.process_once_for_tests(1).await;

    let state = service.get_state();
    assert_eq!(client.sent_messages().len(), 0);
    assert_eq!(state.sent_count, 0);
    assert!(state.last_error.is_some());
}

#[tokio::test]
async fn runtime_acceptance_test_alert_writes_sidecar_event_via_alert_service() {
    let source = mock_source(build_state(build_event(
        ToxicDirection::Buy,
        ToxicSeverity::Alert,
        true,
        true,
        true,
    )));
    let client = TelegramClient::mock_success(false);
    let events_path = temp_events_path("toxic-flow-runtime-acceptance");
    let sidecar_writer =
        ToxicFlowSidecarWriter::new(true, Some(events_path.to_string_lossy().to_string()));
    let service = AlertService::with_client_and_sidecar(
        source,
        client,
        sidecar_writer,
        AlertGateConfig::default(),
        250,
    );

    let first = service
        .emit_runtime_acceptance_test_alert(
            1_760_000_000_111,
            &btc_toxic_flow_monitor_rs::alerts::alert_service::DevTestSidecarAlertInput {
                severity: ToxicSeverity::Warning,
                venue: Venue::Binance,
                symbol: "BTCUSDT".to_string(),
                dedupe_suffix: "manual-001".to_string(),
            },
        )
        .expect("first test alert");
    let second = service
        .emit_runtime_acceptance_test_alert(
            1_760_000_000_222,
            &btc_toxic_flow_monitor_rs::alerts::alert_service::DevTestSidecarAlertInput {
                severity: ToxicSeverity::Warning,
                venue: Venue::Binance,
                symbol: "BTCUSDT".to_string(),
                dedupe_suffix: "manual-001".to_string(),
            },
        )
        .expect("duplicate test alert");
    let third = service
        .emit_runtime_acceptance_test_alert(
            1_760_000_000_333,
            &btc_toxic_flow_monitor_rs::alerts::alert_service::DevTestSidecarAlertInput {
                severity: ToxicSeverity::Warning,
                venue: Venue::Binance,
                symbol: "BTCUSDT".to_string(),
                dedupe_suffix: "manual-002".to_string(),
            },
        )
        .expect("new key test alert");

    assert!(first.sidecar_written);
    assert!(!first.deduped);
    assert!(!second.sidecar_written);
    assert!(second.deduped);
    assert!(third.sidecar_written);
    assert!(!third.deduped);
    assert_ne!(first.dedupe_key, third.dedupe_key);
    assert!(first.dedupe_key.ends_with("manual-001"));
    assert!(third.dedupe_key.ends_with("manual-002"));

    let content = fs::read_to_string(events_path).expect("sidecar events jsonl");
    let lines = content.lines().collect::<Vec<_>>();
    assert_eq!(lines.len(), 2);
    let event: serde_json::Value = serde_json::from_str(lines[0]).expect("sidecar json");
    assert_eq!(event["schemaVersion"], "toxic-flow-rs.sidecar.v1");
    assert_eq!(event["source"], "toxic-flow-rs");
    assert_eq!(event["kind"], "runtime_acceptance_test");
    assert_eq!(event["severity"], "warning");
    assert_eq!(event["title"], "Runtime acceptance test alert");
    assert_eq!(
        event["summary"],
        "This is a monitor-generated sidecar test alert."
    );
    assert_eq!(event["venue"], "binance");
    assert_eq!(event["symbol"], "BTCUSDT");
    assert_eq!(event["payload"]["readOnly"], true);
    assert_eq!(event["payload"]["test"], true);
    assert_eq!(
        event["payload"]["generatedBy"],
        "monitor_dev_test_alert_endpoint"
    );
    assert!(!event.as_object().unwrap().contains_key("webhook"));
    assert!(!event.as_object().unwrap().contains_key("discord"));
}

fn temp_events_path(name: &str) -> std::path::PathBuf {
    let nonce = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("system time")
        .as_nanos();
    std::env::temp_dir()
        .join(format!("{name}-{nonce}"))
        .join("events.jsonl")
}

fn mock_source(state: ToxicState) -> Arc<MockToxicSource> {
    Arc::new(MockToxicSource {
        state: Arc::new(RwLock::new(state)),
    })
}

struct MockToxicSource {
    state: Arc<RwLock<ToxicState>>,
}

impl ToxicStateSource for MockToxicSource {
    fn toxic_state(&self) -> ToxicState {
        self.state.read().clone()
    }
}

fn build_state(event: ToxicEvent) -> ToxicState {
    let result = ToxicVolumeResult {
        symbol: "BTC-PERP".to_string(),
        window_ms: event.window_ms,
        ts: event.ts,
        direction: event.direction,
        severity: event.severity,
        toxic_ratio: 0.85,
        toxic_volume_btc: event.toxic_volume_btc,
        threshold_btc: event.threshold_btc,
        alert_triggered: true,
        aggressive_buy_btc: event.aggressive_buy_btc,
        aggressive_sell_btc: event.aggressive_sell_btc,
        net_aggressive_btc: event.net_aggressive_btc,
        abs_aggressive_btc: event.abs_aggressive_btc,
        markout_1s_bps: event.markout_1s_bps,
        markout_5s_bps: event.markout_5s_bps,
        markout_confirmed: event.markout_1s_bps.is_some_and(|bps| bps > 1.0)
            || event.markout_5s_bps.is_some_and(|bps| bps > 3.0),
        sweep_detected: event.sweep_detected,
        liquidity_thin: event.liquidity_thin,
        liquidity: event.liquidity.clone(),
        cross_venue_confirmed: event.cross_venue_confirmed,
        vpin_enabled: event.vpin_enabled,
        vpin: event.vpin,
        vpin_zscore: event.vpin_zscore,
        vpin_spike: event.vpin_spike,
        vpin_high: event.vpin_high,
        vpin_extreme: event.vpin_extreme,
        liquidation_enabled: event.liquidation_enabled,
        nearest_cluster_side: event.nearest_cluster_side,
        cluster_distance_bps: event.cluster_distance_bps,
        cluster_notional_usd: event.cluster_notional_usd,
        cluster_density: event.cluster_density,
        liq_hunt_pressure: event.liq_hunt_pressure,
        liq_cluster_nearby: event.liq_cluster_nearby,
        possible_liq_hunt_setup: event.possible_liq_hunt_setup,
        leader_venue: event.leader_venue,
        venue_breakdown: BTreeMap::new(),
        reason_codes: event.reason_codes.clone(),
    };

    let mut results = BTreeMap::new();
    results.insert(event.window_ms.to_string(), result);

    ToxicState {
        symbol: "BTC-PERP".to_string(),
        updated_at: event.ts,
        threshold_btc: event.threshold_btc,
        windows_ms: vec![1000, 5000, 15000, 60000],
        results,
        latest_event: Some(event.clone()),
        recent_events: vec![event],
        quality: ToxicQuality {
            has_flow: true,
            has_markout: true,
            has_sweep: true,
            has_liquidation: true,
            liquidation: None,
            active_venues: vec![Venue::Binance, Venue::Bybit, Venue::Okx],
            stale_venues: Vec::new(),
        },
    }
}

fn build_event(
    direction: ToxicDirection,
    severity: ToxicSeverity,
    cross_venue_confirmed: bool,
    markout_ok: bool,
    liquidity_ok: bool,
) -> ToxicEvent {
    let liquidity = LiquidityThinnessResult {
        symbol: "BTC-PERP".to_string(),
        window_ms: 5_000,
        bid_depth_start_btc: Some(1_000.0),
        bid_depth_end_btc: Some(
            if matches!(direction, ToxicDirection::Sell) && liquidity_ok {
                600.0
            } else {
                980.0
            },
        ),
        ask_depth_start_btc: Some(1_000.0),
        ask_depth_end_btc: Some(
            if matches!(direction, ToxicDirection::Buy) && liquidity_ok {
                600.0
            } else {
                980.0
            },
        ),
        bid_depth_drop_ratio: Some(
            if matches!(direction, ToxicDirection::Sell) && liquidity_ok {
                0.4
            } else {
                0.02
            },
        ),
        ask_depth_drop_ratio: Some(
            if matches!(direction, ToxicDirection::Buy) && liquidity_ok {
                0.4
            } else {
                0.02
            },
        ),
        spread_start_bps: Some(2.0),
        spread_end_bps: Some(if liquidity_ok { 3.5 } else { 2.1 }),
        spread_widen_ratio: Some(if liquidity_ok { 0.75 } else { 0.05 }),
        bid_thin: matches!(direction, ToxicDirection::Sell) && liquidity_ok,
        ask_thin: matches!(direction, ToxicDirection::Buy) && liquidity_ok,
        spread_widened: liquidity_ok,
        reason_codes: vec!["spread_widened".to_string()],
    };

    ToxicEvent {
        id: "event-1".to_string(),
        ts: 1_760_000_000_000,
        symbol: "BTC-PERP".to_string(),
        direction,
        severity,
        toxic_volume_btc: if severity == ToxicSeverity::Watch {
            800.0
        } else if matches!(direction, ToxicDirection::Buy) {
            1_284.2
        } else {
            1_120.0
        },
        threshold_btc: 1_000.0,
        window_ms: 5_000,
        leader_venue: Some(Venue::Binance),
        aggressive_buy_btc: if matches!(direction, ToxicDirection::Buy) {
            1_566.0
        } else {
            100.0
        },
        aggressive_sell_btc: if matches!(direction, ToxicDirection::Sell) {
            1_120.0
        } else {
            220.0
        },
        net_aggressive_btc: if matches!(direction, ToxicDirection::Buy) {
            1_346.0
        } else {
            -1_020.0
        },
        abs_aggressive_btc: 1_786.0,
        markout_1s_bps: Some(if markout_ok {
            if matches!(direction, ToxicDirection::Buy) {
                2.1
            } else {
                1.8
            }
        } else {
            0.2
        }),
        markout_5s_bps: Some(if markout_ok {
            if matches!(direction, ToxicDirection::Buy) {
                4.8
            } else {
                3.7
            }
        } else {
            0.3
        }),
        sweep_detected: true,
        liquidity_thin: liquidity_ok,
        liquidity: Some(liquidity),
        cross_venue_confirmed,
        vpin_enabled: true,
        vpin: Some(0.82),
        vpin_zscore: Some(2.8),
        vpin_spike: true,
        vpin_high: false,
        vpin_extreme: false,
        liquidation_enabled: true,
        nearest_cluster_side: Some(if matches!(direction, ToxicDirection::Buy) {
            btc_toxic_flow_monitor_rs::types::liquidation::LiquidationClusterSide::ShortAbove
        } else {
            btc_toxic_flow_monitor_rs::types::liquidation::LiquidationClusterSide::LongBelow
        }),
        cluster_distance_bps: Some(11.0),
        cluster_notional_usd: Some(2_500_000.0),
        cluster_density: Some(0.62),
        liq_hunt_pressure: 0.76,
        liq_cluster_nearby: true,
        possible_liq_hunt_setup: true,
        reason_codes: vec![
            "large_aggressive_flow".to_string(),
            "flow_imbalance_spike".to_string(),
            "markout_1s_confirmed".to_string(),
            "markout_5s_confirmed".to_string(),
            "sweep_detected".to_string(),
            "local_liquidity_drain".to_string(),
            "cross_venue_confirmed".to_string(),
            "threshold_crossed".to_string(),
        ],
    }
}
