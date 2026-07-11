use std::collections::VecDeque;

use btc_toxic_flow_monitor_rs::binance_alt_contract_monitor::{
    amios::run_market_intelligence_os,
    config::BinanceAltContractRuntimeConfig,
    detector::detect_alt_contract_signal,
    types::{
        AltContractAtcaReport, AltContractContext, AltContractDirection,
        AltContractExchangeContribution, AltContractSeverity, AltContractSignal,
        AltContractSignalType, AltContractSmafReport, AltContractSmllReport, AltContractSymbolTier,
        AltContractWindowStats,
    },
};

const NOW_MS: i64 = 1_700_000_000_000;

#[test]
fn market_os_idles_without_recent_signals_and_stays_read_only() {
    let report = run_market_intelligence_os(
        NOW_MS,
        &VecDeque::new(),
        &smaf(90.0),
        &smll(false),
        &atca("waiting_for_signals", 0.0),
    );

    assert_eq!(report.os_status, "idle");
    assert_eq!(report.market_state, "CALM");
    assert_eq!(report.scheduler_decision, "standby");
    assert!(report.current_states.is_empty());
    assert!(report.read_only);
    assert!(report.protected_realtime);
    assert!(!report.direct_discord_gate);
}

#[test]
fn market_os_enters_active_control_mode_for_coherent_main_force_signal() {
    let mut signal = base_signal("SOL", NOW_MS);
    signal.signal_type = AltContractSignalType::MainForceLongBuild;
    signal.severity = AltContractSeverity::Critical;
    signal.market_control_graph.control_strength = 84.0;
    signal.market_control_graph.control_type = "ControlAccumulation".to_string();
    signal.market_control_graph.dominant_side = "buy".to_string();
    signal.smart_money_lifecycle.lifecycle_state = "Markup".to_string();
    signal.smart_money_lifecycle.lifecycle_score = 82.0;
    signal.smart_money_lifecycle.state_confidence = 83.0;
    signal.smart_money_prediction.next_state = "Distribution".to_string();
    signal.smart_money_prediction.confidence = 76.0;
    signal.signal_confidence.confidence_score = 87.0;

    let report = report_for(vec![signal]);

    assert_eq!(report.os_status, "running");
    assert_eq!(report.market_state, "ACTIVE_CONTROL_MODE");
    assert_eq!(report.scheduler_decision, "monitor_high_confidence");
    assert!(report.kernel_load >= 70.0, "{report:?}");
    assert!(report.confidence >= 80.0, "{report:?}");
    assert!(report
        .active_processes
        .iter()
        .any(|process| process.name == "MCG" && process.layer == "graph"));
    assert!(report
        .active_processes
        .iter()
        .any(|process| process.name == "SCC" && process.status == "calibrated"));
    assert_eq!(report.current_states[0].market_state, "ACTIVE_CONTROL_MODE");
    assert!(report.current_states[0]
        .control
        .contains("ControlAccumulation"));
    assert!(report.read_only);
    assert!(!report.direct_discord_gate);
}

#[test]
fn liquidation_signal_is_treated_as_os_interrupt_not_execution() {
    let mut signal = base_signal("ADA", NOW_MS);
    signal.signal_type = AltContractSignalType::LiquidationCascade;
    signal.severity = AltContractSeverity::S;
    signal.liquidation_suspected = true;
    signal.force_order_snapshot = true;
    signal.price_move_pct = Some(-0.9);
    signal.oi_change_pct = Some(-1.8);
    signal.signal_confidence.confidence_score = 58.0;

    let report = report_for(vec![signal]);

    assert_eq!(report.market_state, "INTERRUPT_LIQUIDATION_MODE");
    assert_eq!(report.scheduler_decision, "interrupt_priority");
    assert_eq!(report.current_states[0].risk, "liquidation_risk");
    assert!(report.current_states[0]
        .explanation
        .contains("OS interrupt"));
    assert!(report.read_only);
    assert!(!report.direct_discord_gate);
}

#[test]
fn degraded_audit_state_marks_os_as_degraded() {
    let mut signal = base_signal("DOGE", NOW_MS);
    signal.signal_confidence.confidence_score = 72.0;
    let report = run_market_intelligence_os(
        NOW_MS,
        &VecDeque::from(vec![signal]),
        &smaf(48.0),
        &smll(true),
        &atca("degraded_cognition", 70.0),
    );

    assert_eq!(report.os_status, "degraded");
    assert_eq!(report.risk, "system_risk");
    assert!(report.audit_summary.contains("read_only=true"));
}

fn report_for(
    signals: Vec<AltContractSignal>,
) -> btc_toxic_flow_monitor_rs::binance_alt_contract_monitor::types::AltContractMarketOsReport {
    run_market_intelligence_os(
        NOW_MS,
        &VecDeque::from(signals),
        &smaf(90.0),
        &smll(false),
        &atca("active_cognition", 82.0),
    )
}

fn smaf(score: f64) -> AltContractSmafReport {
    AltContractSmafReport {
        smaf_score: score,
        risk_level: if score >= 70.0 {
            "Stable".to_string()
        } else {
            "Degraded".to_string()
        },
        critical_issues: if score < 50.0 {
            vec!["data_integrity_low".to_string()]
        } else {
            Vec::new()
        },
        ..AltContractSmafReport::default()
    }
}

fn smll(drift_detected: bool) -> AltContractSmllReport {
    AltContractSmllReport {
        enabled: true,
        protected_realtime: true,
        learning_score: if drift_detected { 44.0 } else { 80.0 },
        sample_size: 4,
        drift_report:
            btc_toxic_flow_monitor_rs::binance_alt_contract_monitor::types::AltContractDriftReport {
                drift_detected,
                reason: if drift_detected {
                    "fixture_drift".to_string()
                } else {
                    "stable".to_string()
                },
                ..Default::default()
            },
        ..AltContractSmllReport::default()
    }
}

fn atca(status: &str, confidence: f64) -> AltContractAtcaReport {
    AltContractAtcaReport {
        enabled: true,
        protected_realtime: true,
        cognition_status: status.to_string(),
        agents: if confidence > 0.0 {
            vec![btc_toxic_flow_monitor_rs::binance_alt_contract_monitor::types::AltContractAgentView {
                symbol: "SOLUSDT".to_string(),
                confidence,
                ..Default::default()
            }]
        } else {
            Vec::new()
        },
        ..AltContractAtcaReport::default()
    }
}

fn base_signal(symbol: &str, ts: i64) -> AltContractSignal {
    let config = BinanceAltContractRuntimeConfig::default();
    let context = AltContractContext {
        oi_change_1m_base: Some(2_000.0),
        oi_change_pct: Some(1.2),
        oi_updated_at: Some(ts - 5_000),
        price_move_1m_pct: Some(0.4),
        ..AltContractContext::default()
    };
    let mut signal = detect_alt_contract_signal(&stats(symbol, ts), &context, &config)
        .expect("fixture should produce BACM signal");
    signal.id = format!("amios-{symbol}-{ts}");
    signal.severity = AltContractSeverity::Critical;
    signal.data_quality = 92;
    signal.abnormal_score = 88;
    signal.build_score = 84;
    signal.master_capital_strength.mcss = 86.0;
    signal.liquidity_microstructure.lms_score = 81.0;
    signal.market_regime.regime = "Accumulation".to_string();
    signal.market_regime.confidence = 82.0;
    signal.smart_money_lifecycle.state_confidence = 82.0;
    signal.smart_money_prediction.confidence = 78.0;
    signal.smart_money_prediction.next_state = "Markup".to_string();
    signal
}

fn stats(symbol: &str, ts: i64) -> AltContractWindowStats {
    AltContractWindowStats {
        symbol: symbol.to_string(),
        product_id: format!("{symbol}USDT"),
        tier: AltContractSymbolTier::B,
        window_sec: 60,
        ts,
        buy_volume_base: 8_500.0,
        sell_volume_base: 1_500.0,
        total_volume_base: 10_000.0,
        net_volume_base: 7_000.0,
        total_notional_usd: 2_000_000.0,
        dominance: 0.70,
        direction: AltContractDirection::Buy,
        trigger_price_usd: Some(200.0),
        price_move_pct: Some(0.4),
        price_threshold_pct: None,
        exchange_count: 1,
        main_exchange: Some("binance".to_string()),
        exchanges: vec![AltContractExchangeContribution {
            exchange: "binance".to_string(),
            buy_volume_base: 8_500.0,
            sell_volume_base: 1_500.0,
            total_volume_base: 10_000.0,
            buy_notional_usd: 1_700_000.0,
            sell_notional_usd: 300_000.0,
            total_notional_usd: 2_000_000.0,
            net_volume_base: 7_000.0,
            dominance: 0.70,
            trade_count: 100,
        }],
        dynamic_multiple: Some(7.0),
        data_quality: 92,
        startup_age_ms: Some(120_000),
    }
}
