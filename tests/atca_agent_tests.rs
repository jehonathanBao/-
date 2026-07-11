use std::collections::VecDeque;

use btc_toxic_flow_monitor_rs::binance_alt_contract_monitor::{
    atca::run_trading_cognition_agent,
    config::BinanceAltContractRuntimeConfig,
    detector::detect_alt_contract_signal,
    types::{
        AltContractContext, AltContractDirection, AltContractExchangeContribution,
        AltContractSeverity, AltContractSignal, AltContractSignalType, AltContractSmafReport,
        AltContractSmllReport, AltContractSymbolTier, AltContractWindowStats,
    },
};

const NOW_MS: i64 = 1_700_000_000_000;

#[test]
fn accumulation_maps_to_accumulate_intent() {
    let mut signal = base_signal("SOL", NOW_MS);
    signal.signal_type = AltContractSignalType::MainForceLongBuild;
    signal.smart_money_lifecycle.lifecycle_state = "Accumulation".to_string();
    signal.market_regime.regime = "Accumulation".to_string();
    signal.oi_change_pct = Some(1.2);
    signal.price_move_pct = Some(0.02);
    let report = report_for(vec![signal]);

    let agent = &report.agents[0];
    assert_eq!(agent.state, "Accumulation");
    assert_eq!(agent.intent, "accumulate");
    assert_eq!(agent.market_state.oi_movement, "expanding");
    assert_eq!(agent.market_state.price_structure, "flat");
    assert!(report.protected_realtime);
}

#[test]
fn fake_breakout_maps_to_manipulation_trap_intent() {
    let mut signal = base_signal("DOGE", NOW_MS);
    signal.market_regime.regime = "Manipulation".to_string();
    signal.smart_money_lifecycle.lifecycle_state = "Markup".to_string();
    signal.smart_money_prediction.next_state = "Distribution".to_string();
    signal.price_move_pct = Some(0.62);
    signal.oi_change_pct = Some(-0.4);
    let report = report_for(vec![signal]);

    let agent = &report.agents[0];
    assert_eq!(agent.state, "Manipulation");
    assert_eq!(agent.intent, "trap");
    assert_eq!(agent.prediction, "Distribution");
    assert_eq!(agent.market_state.price_structure, "breakout_up");
}

#[test]
fn distribution_maps_to_distribute_intent() {
    let mut signal = base_signal("XRP", NOW_MS);
    signal.signal_type = AltContractSignalType::UpsideResistance;
    signal.smart_money_lifecycle.lifecycle_state = "Distribution".to_string();
    signal.market_regime.regime = "Distribution".to_string();
    signal.price_move_pct = Some(0.28);
    signal.oi_change_pct = Some(-0.8);
    let report = report_for(vec![signal]);

    let agent = &report.agents[0];
    assert_eq!(agent.state, "Distribution");
    assert_eq!(agent.intent, "distribute");
    assert_eq!(agent.market_state.oi_movement, "contracting");
}

#[test]
fn liquidation_spike_maps_to_exit_liquidity_intent() {
    let mut signal = base_signal("ADA", NOW_MS);
    signal.signal_type = AltContractSignalType::LiquidationCascade;
    signal.liquidation_suspected = true;
    signal.force_order_snapshot = true;
    signal.oi_change_pct = Some(-1.4);
    signal.price_move_pct = Some(-0.75);
    let report = report_for(vec![signal]);

    let agent = &report.agents[0];
    assert_eq!(agent.intent, "exit_liquidity");
    assert_eq!(agent.risk, "liquidation_risk");
    assert_eq!(agent.market_state.liquidation_pressure, "elevated");
}

fn report_for(
    signals: Vec<AltContractSignal>,
) -> btc_toxic_flow_monitor_rs::binance_alt_contract_monitor::types::AltContractAtcaReport {
    let smaf = AltContractSmafReport {
        smaf_score: 92.0,
        risk_level: "Production Ready".to_string(),
        ..AltContractSmafReport::default()
    };
    let smll = AltContractSmllReport {
        sample_size: 4,
        learning_score: 80.0,
        ..AltContractSmllReport::default()
    };
    run_trading_cognition_agent(NOW_MS, &VecDeque::from(signals), &smaf, &smll)
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
    signal.id = format!("atca-{symbol}-{ts}");
    signal.severity = AltContractSeverity::Critical;
    signal.data_quality = 92;
    signal.master_capital_strength.mcss = 82.0;
    signal.market_regime.confidence = 82.0;
    signal.smart_money_lifecycle.state_confidence = 82.0;
    signal.smart_money_prediction.confidence = 82.0;
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
