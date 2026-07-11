use std::collections::VecDeque;

use btc_toxic_flow_monitor_rs::binance_alt_contract_monitor::{
    config::BinanceAltContractRuntimeConfig,
    detector::detect_alt_contract_signal,
    smll::audit_self_learning_loop,
    types::{
        AltContractContext, AltContractDirection, AltContractExchangeContribution,
        AltContractSignal, AltContractSymbolTier, AltContractWindowStats,
    },
};

const NOW_MS: i64 = 1_700_000_000_000;

#[test]
fn accumulated_prediction_errors_suggest_prediction_recalibration() {
    let signals = VecDeque::from(vec![
        wrong_buy_signal("SOL", NOW_MS - 240_000),
        wrong_buy_signal("DOGE", NOW_MS - 180_000),
        wrong_buy_signal("XRP", NOW_MS - 120_000),
        correct_buy_signal("ADA", NOW_MS - 60_000),
    ]);

    let report = audit_self_learning_loop(NOW_MS, &signals);

    assert!(report.accuracy_rate < 60.0, "{report:?}");
    assert!(report
        .error_reports
        .iter()
        .any(|item| item.affected_module == "SMP"));
    assert!(report
        .calibration_updates
        .iter()
        .any(|item| item.parameter == "smp.confidence_cap"));
    assert!(report.protected_realtime);
}

#[test]
fn repeated_oi_misdirection_reduces_suggested_oi_weight() {
    let signals = VecDeque::from(vec![
        wrong_buy_signal("SOL", NOW_MS - 240_000),
        wrong_buy_signal("DOGE", NOW_MS - 180_000),
        wrong_buy_signal("XRP", NOW_MS - 120_000),
    ]);

    let report = audit_self_learning_loop(NOW_MS, &signals);

    assert!(report.suggested_weights.oi_weight < 1.0, "{report:?}");
    assert!(report
        .calibration_updates
        .iter()
        .any(|item| item.parameter == "mcss.oi_weight"));
}

#[test]
fn liquidation_false_build_reduces_liquidation_weight() {
    let mut first = wrong_buy_signal("SOL", NOW_MS - 240_000);
    first.liquidation_suspected = true;
    first.force_order_snapshot = true;
    let mut second = wrong_buy_signal("DOGE", NOW_MS - 180_000);
    second.liquidation_suspected = true;
    second.force_order_snapshot = true;
    let mut third = correct_buy_signal("XRP", NOW_MS - 120_000);
    third.liquidation_suspected = true;
    third.force_order_snapshot = true;
    let signals = VecDeque::from(vec![first, second, third]);

    let report = audit_self_learning_loop(NOW_MS, &signals);

    assert!(
        report.suggested_weights.liquidation_weight < 1.0,
        "{report:?}"
    );
    assert!(report
        .calibration_updates
        .iter()
        .any(|item| item.parameter == "mcss.liquidation_weight"));
}

#[test]
fn regime_drift_triggers_retrain_suggestion() {
    let mut one = correct_buy_signal("SOL", NOW_MS - 300_000);
    one.smart_money_lifecycle.lifecycle_state = "Accumulation".to_string();
    one.smart_money_prediction.next_state = "Markup".to_string();
    let mut two = correct_buy_signal("DOGE", NOW_MS - 240_000);
    two.smart_money_lifecycle.lifecycle_state = "Markup".to_string();
    two.smart_money_prediction.next_state = "Distribution".to_string();
    let mut three = correct_buy_signal("XRP", NOW_MS - 180_000);
    three.smart_money_lifecycle.lifecycle_state = "Distribution".to_string();
    three.smart_money_prediction.next_state = "Markdown".to_string();
    let mut four = correct_buy_signal("ADA", NOW_MS - 120_000);
    four.smart_money_lifecycle.lifecycle_state = "Markdown".to_string();
    four.smart_money_prediction.next_state = "Accumulation".to_string();
    let signals = VecDeque::from(vec![one, two, three, four]);

    let report = audit_self_learning_loop(NOW_MS, &signals);

    assert!(report.drift_report.drift_detected, "{report:?}");
    assert!(report.drift_report.suggested_retrain, "{report:?}");
    assert!(report
        .calibration_updates
        .iter()
        .any(|item| item.parameter == "smle.transition_recalibration"));
}

fn wrong_buy_signal(symbol: &str, ts: i64) -> AltContractSignal {
    let mut signal = signal(symbol, ts);
    signal.price_move_pct = Some(-0.45);
    signal.post_signal_status = "failed".to_string();
    signal.failed_at = Some(ts + 5 * 60_000);
    signal.smart_money_prediction.confidence = 86.0;
    signal.score_breakdown.oi_score = 18.0;
    signal.oi_change_pct = Some(1.2);
    signal
}

fn correct_buy_signal(symbol: &str, ts: i64) -> AltContractSignal {
    let mut signal = signal(symbol, ts);
    signal.price_move_pct = Some(0.45);
    signal.post_signal_status = "validated".to_string();
    signal.validated_at = Some(ts + 5 * 60_000);
    signal.smart_money_prediction.confidence = 84.0;
    signal.score_breakdown.oi_score = 18.0;
    signal.oi_change_pct = Some(1.2);
    signal
}

fn signal(symbol: &str, ts: i64) -> AltContractSignal {
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
    signal.id = format!("smll-{symbol}-{ts}");
    signal.smart_money_lifecycle.lifecycle_state = "Markup".to_string();
    signal.smart_money_lifecycle.state_confidence = 82.0;
    signal.smart_money_prediction.next_state = "Markup".to_string();
    signal.smart_money_prediction.confidence = 82.0;
    signal.smart_money_prediction.probability = 80.0;
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
