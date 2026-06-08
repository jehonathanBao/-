use btc_toxic_flow_monitor_rs::{
    contract_whale_monitor::types::{
        ContractWhaleDirection, ContractWhaleSeverity, ContractWhaleSignal, ContractWhaleSignalType,
    },
    runtime::cwm_risk_fusion::{build_cwm_risk_contribution, fused_risk_score_with_cwm},
};

#[test]
fn cwm_fusion_uses_four_factor_weight_when_signal_exists() {
    let score = fused_risk_score_with_cwm(80, 70.0, 90, Some(100));

    assert_eq!(score, 83);
}

#[test]
fn cwm_fusion_keeps_existing_advanced_formula_when_signal_missing() {
    let score = fused_risk_score_with_cwm(80, 70.0, 90, None);

    assert_eq!(score, 80);
}

#[test]
fn cwm_contribution_is_safe_and_marks_discord_gate_independent() {
    let signal = sample_cwm_signal();
    let contribution = build_cwm_risk_contribution("BTC-PERP", Some(&signal));
    let json = serde_json::to_string(&contribution).expect("json");

    assert!(contribution.available);
    assert_eq!(contribution.score, Some(94));
    assert_eq!(contribution.weighted_contribution, 14.1);
    assert!(contribution.discord_gate_independent);
    assert!(json.contains("contract_whale_monitor"));
    assert!(json.contains("finalRiskScore"));
    assert!(!json.to_ascii_lowercase().contains("webhook"));
    assert!(!json.to_ascii_lowercase().contains("token"));
    assert!(!json.contains("rawPayload"));
    assert!(!json.to_ascii_lowercase().contains("evidence"));
    assert!(!json.to_ascii_lowercase().contains("markout"));
}

#[test]
fn cwm_contribution_missing_signal_keeps_independent_gate_visible() {
    let contribution = build_cwm_risk_contribution("ETH-PERP", None);

    assert!(!contribution.available);
    assert_eq!(contribution.score, None);
    assert_eq!(contribution.weighted_contribution, 0.0);
    assert!(contribution.discord_gate_independent);
    assert!(contribution.summary.contains("existing TOF score kept"));
}

fn sample_cwm_signal() -> ContractWhaleSignal {
    ContractWhaleSignal {
        id: "contract-whale:BTC:15:1700000000000:buy".to_string(),
        ts: 1_700_000_000_000,
        symbol: "BTC".to_string(),
        window_sec: 15,
        signal_type: ContractWhaleSignalType::AggressiveBuy,
        direction: ContractWhaleDirection::Buy,
        severity: ContractWhaleSeverity::S,
        score: 94,
        total_volume_btc: 4_820.0,
        net_volume_btc: 3_260.0,
        total_notional_usd: 337_000_000.0,
        dominance: 0.676,
        price_move_pct: Some(0.31),
        main_exchange: Some("binance".to_string()),
        exchanges: Vec::new(),
        dynamic_multiple: Some(9.4),
        percentile_level: Some(99.9),
        multi_exchange_confirmed: true,
        liquidation_suspected: false,
        liquidation_long_btc: 0.0,
        liquidation_short_btc: 0.0,
        liquidation_notional_usd: 0.0,
        liquidation_ratio: None,
        price_reversal_ratio: None,
        oi_change_1m_btc: None,
        oi_change_5m_btc: None,
        oi_change_pct: None,
        oi_bias: None,
        funding_rate: None,
        funding_bias: None,
        data_quality: 91,
        discord_eligible: true,
        discord_sent: false,
        discord_sent_at: None,
        discord_reason: "dry_run".to_string(),
        final_result: "多平台主动买入爆发，疑似主力合约拉盘".to_string(),
        read_only: true,
        analysis_only: true,
        execution_enabled: false,
        merged_from: Vec::new(),
    }
}
