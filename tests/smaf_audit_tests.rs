use std::collections::{BTreeMap, VecDeque};

use btc_toxic_flow_monitor_rs::binance_alt_contract_monitor::{
    config::BinanceAltContractRuntimeConfig,
    detector::detect_alt_contract_signal,
    smaf::{audit_smart_money_system, SmafAuditInput},
    types::{
        AltContractContext, AltContractDirection, AltContractExchangeContribution,
        AltContractExchangeStatus, AltContractSignal, AltContractSourceSnapshot,
        AltContractSymbolTier, AltContractWindowStats,
    },
};

const NOW_MS: i64 = 1_700_000_000_000;

#[test]
fn data_disconnect_is_reported_as_high_risk() {
    let signals = VecDeque::new();
    let report = audit_smart_money_system(SmafAuditInput {
        enabled: true,
        now_ms: NOW_MS,
        exchanges: &exchange_map(false, NOW_MS - 10 * 60_000),
        signals: &signals,
        last_oi_poll_at: None,
        last_force_order_at: None,
        last_mark_price_at: None,
        last_ticker_at: None,
        errors1h: 3,
    });

    assert_eq!(report.data_audit.data_risk_level, "high");
    assert!(report.data_audit.integrity_score < 60.0, "{report:?}");
    assert!(report
        .critical_issues
        .contains(&"data_integrity_low".to_string()));
}

#[test]
fn single_source_pollution_is_detected() {
    let signals = VecDeque::from(vec![
        signal("SOL", NOW_MS - 240_000, "Markup", "Markup", true),
        signal("DOGE", NOW_MS - 180_000, "Markup", "Markup", true),
        signal("XRP", NOW_MS - 120_000, "Markup", "Markup", true),
    ]);
    let report = healthy_report(signals);

    assert_eq!(report.signal_audit.single_source_dependency, 100.0);
    assert!(
        report
            .critical_issues
            .contains(&"single_source_dependency_high".to_string()),
        "{report:?}"
    );
}

#[test]
fn rapid_lifecycle_switching_reduces_behavior_integrity() {
    let signals = VecDeque::from(vec![
        signal("SOL", NOW_MS - 300_000, "Accumulation", "Markup", false),
        signal("DOGE", NOW_MS - 240_000, "Markup", "Distribution", false),
        signal("XRP", NOW_MS - 180_000, "Distribution", "Markdown", false),
        signal("ADA", NOW_MS - 120_000, "Markdown", "Accumulation", false),
    ]);
    let report = healthy_report(signals);

    assert!(
        report.behavior_audit.transition_entropy >= 90.0,
        "{report:?}"
    );
    assert!(
        report.behavior_audit.structural_integrity < 70.0,
        "{report:?}"
    );
    assert!(
        report
            .critical_issues
            .contains(&"lifecycle_transition_entropy_high".to_string()),
        "{report:?}"
    );
}

#[test]
fn prediction_flip_rate_is_reported() {
    let signals = VecDeque::from(vec![
        signal("SOL", NOW_MS - 300_000, "Markup", "Distribution", false),
        signal("DOGE", NOW_MS - 240_000, "Markup", "Markdown", false),
        signal("XRP", NOW_MS - 180_000, "Markup", "Accumulation", false),
        signal("ADA", NOW_MS - 120_000, "Markup", "Distribution", false),
    ]);
    let report = healthy_report(signals);

    assert!(report.prediction_audit.flip_rate >= 90.0, "{report:?}");
    assert!(
        report
            .critical_issues
            .contains(&"prediction_flip_rate_high".to_string()),
        "{report:?}"
    );
}

#[test]
fn stable_two_source_system_scores_above_eighty_five() {
    let signals = VecDeque::from(vec![
        signal("SOL", NOW_MS - 300_000, "Markup", "Markup", false),
        signal("DOGE", NOW_MS - 220_000, "Markup", "Markup", false),
        signal("XRP", NOW_MS - 140_000, "Markup", "Markup", false),
        signal("ADA", NOW_MS - 60_000, "Markup", "Markup", false),
    ]);
    let report = healthy_report(signals);

    assert!(report.smaf_score >= 85.0, "{report:?}");
    assert!(report.critical_issues.is_empty(), "{report:?}");
}

fn healthy_report(
    signals: VecDeque<AltContractSignal>,
) -> btc_toxic_flow_monitor_rs::binance_alt_contract_monitor::types::AltContractSmafReport {
    audit_smart_money_system(SmafAuditInput {
        enabled: true,
        now_ms: NOW_MS,
        exchanges: &exchange_map(true, NOW_MS - 5_000),
        signals: &signals,
        last_oi_poll_at: Some(NOW_MS - 15_000),
        last_force_order_at: Some(NOW_MS - 20_000),
        last_mark_price_at: Some(NOW_MS - 10_000),
        last_ticker_at: Some(NOW_MS - 10_000),
        errors1h: 0,
    })
}

fn exchange_map(
    connected: bool,
    last_trade_at: i64,
) -> BTreeMap<String, AltContractExchangeStatus> {
    let mut exchanges = BTreeMap::new();
    exchanges.insert(
        "binance".to_string(),
        AltContractExchangeStatus {
            connected,
            status: if connected { "connected" } else { "stale" }.to_string(),
            last_trade_at: Some(last_trade_at),
            latency_ms: Some(NOW_MS.saturating_sub(last_trade_at)),
            reconnect_count: 0,
            last_error: None,
        },
    );
    exchanges.insert(
        "bitfinex".to_string(),
        AltContractExchangeStatus {
            connected,
            status: if connected { "connected" } else { "stale" }.to_string(),
            last_trade_at: Some(last_trade_at),
            latency_ms: Some(NOW_MS.saturating_sub(last_trade_at)),
            reconnect_count: 0,
            last_error: None,
        },
    );
    exchanges
}

fn signal(
    symbol: &str,
    ts: i64,
    lifecycle_state: &str,
    next_state: &str,
    single_source: bool,
) -> AltContractSignal {
    let config = BinanceAltContractRuntimeConfig::default();
    let context = AltContractContext {
        oi_change_1m_base: Some(2_000.0),
        oi_change_pct: Some(1.3),
        oi_updated_at: Some(ts - 5_000),
        price_move_1m_pct: Some(0.4),
        ..AltContractContext::default()
    };
    let mut signal = detect_alt_contract_signal(&stats(symbol, ts), &context, &config)
        .expect("fixture should produce a BACM signal");
    signal.id = format!("smaf-{symbol}-{ts}");
    signal.smart_money_lifecycle.lifecycle_state = lifecycle_state.to_string();
    signal.smart_money_lifecycle.state_confidence = 86.0;
    signal.smart_money_prediction.next_state = next_state.to_string();
    signal.smart_money_prediction.confidence = 84.0;
    signal.smart_money_prediction.probability = 82.0;
    signal.market_regime.regime = "Accumulation".to_string();
    signal.data_quality = 92;
    if !single_source {
        signal.active_sources.push(AltContractSourceSnapshot {
            exchange: "bitfinex".to_string(),
            market_type: "perp".to_string(),
            role: "confirmation".to_string(),
            enabled: true,
            status: "active".to_string(),
        });
        signal.exchanges.push(AltContractExchangeContribution {
            exchange: "bitfinex".to_string(),
            total_volume_base: 1_000.0,
            total_notional_usd: 200_000.0,
            net_volume_base: 700.0,
            dominance: 0.7,
            trade_count: 30,
            ..AltContractExchangeContribution::default()
        });
    }
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
