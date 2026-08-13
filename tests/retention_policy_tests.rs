use btc_toxic_flow_monitor_rs::storage::retention_policy::{
    classify_contract, classify_spot, ContractRetentionFacts, RetentionClass, RetentionPolicy,
    SpotRetentionFacts,
};

#[test]
fn retention_policy_uses_seven_thirty_three_hundred_sixty_five_days() {
    let policy = RetentionPolicy::default();
    assert_eq!(policy.days(RetentionClass::Ordinary), 7);
    assert_eq!(policy.days(RetentionClass::Important), 30);
    assert_eq!(policy.days(RetentionClass::Critical), 365);
}

#[test]
fn ordinary_contract_s_grade_without_extreme_evidence_is_not_critical() {
    let facts = ContractRetentionFacts {
        severity: "S".to_string(),
        impact_level: Some("S".to_string()),
        total_volume_btc: 1_500.0,
        window_sec: 15,
        net_volume_btc: 1_200.0,
        liquidation_btc: 0.0,
        multi_exchange_confirmed: true,
        behavior_confirmed: false,
        discord_sent: false,
    };
    assert_eq!(classify_contract(&facts), RetentionClass::Important);
}

#[test]
fn extreme_contract_event_is_critical_and_large_behavior_is_important() {
    let critical = ContractRetentionFacts {
        severity: "S".to_string(),
        impact_level: Some("S".to_string()),
        total_volume_btc: 25_000.0,
        window_sec: 60,
        net_volume_btc: -20_000.0,
        liquidation_btc: 0.0,
        multi_exchange_confirmed: true,
        behavior_confirmed: false,
        discord_sent: false,
    };
    assert_eq!(classify_contract(&critical), RetentionClass::Critical);

    let important = ContractRetentionFacts {
        net_volume_btc: 600.0,
        behavior_confirmed: true,
        ..ContractRetentionFacts::default()
    };
    assert_eq!(classify_contract(&important), RetentionClass::Important);
}

#[test]
fn spot_tiers_use_asset_native_thresholds() {
    let btc_important = SpotRetentionFacts {
        symbol: "BTC".to_string(),
        net_volume_base: 100.0,
        ..SpotRetentionFacts::default()
    };
    let eth_critical = SpotRetentionFacts {
        symbol: "ETH".to_string(),
        net_volume_base: 5_000.0,
        ..SpotRetentionFacts::default()
    };
    assert_eq!(classify_spot(&btc_important), RetentionClass::Important);
    assert_eq!(classify_spot(&eth_critical), RetentionClass::Critical);
}
