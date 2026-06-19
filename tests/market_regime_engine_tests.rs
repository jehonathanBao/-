use btc_toxic_flow_monitor_rs::market_regime_engine::{
    analyze_market_regime, DirectionBias, MarketFeatureSet,
};

#[test]
fn manipulation_detects_oi_divergence_fake_breakout_and_funding_extreme() {
    let output = analyze_market_regime(&MarketFeatureSet {
        symbol: "BTCUSDT".to_string(),
        price_change_5m_pct: Some(0.08),
        oi_change_pct: Some(-0.7),
        volume_spike_multiple: Some(4.6),
        funding_rate: Some(0.0008),
        spot_futures_divergence_pct: Some(0.35),
        liquidation_ratio: Some(0.05),
        price_impact_efficiency: Some(0.02),
        flow_direction: Some(DirectionBias::Long),
        data_quality: Some(1.0),
    });

    assert_eq!(output.regime.regime, "MANIPULATION");
    assert_eq!(output.regime.direction_bias, "SHORT");
    assert!(output.manipulation.score >= 0.75, "{output:?}");
    assert!(output
        .manipulation
        .signals
        .contains(&"OI_DIVERGENCE".to_string()));
    assert!(output
        .manipulation
        .signals
        .contains(&"FAKE_BREAKOUT".to_string()));
    assert!(output
        .manipulation
        .signals
        .contains(&"FUNDING_EXTREME".to_string()));
    assert_eq!(output.signal.allowed_signal_family, "REDUCED_STRENGTH_ONLY");
    assert!(output.signal.adjusted_signal_strength < 0.6);
}

#[test]
fn accumulation_requires_oi_expansion_flat_price_and_stable_volume() {
    let output = analyze_market_regime(&MarketFeatureSet {
        symbol: "ETH".to_string(),
        price_change_5m_pct: Some(0.03),
        oi_change_pct: Some(1.1),
        volume_spike_multiple: Some(1.3),
        funding_rate: Some(0.0001),
        data_quality: Some(0.95),
        ..MarketFeatureSet::default()
    });

    assert_eq!(output.regime.regime, "ACCUMULATION");
    assert_eq!(output.regime.direction_bias, "LONG");
    assert!(output.regime.confidence >= 0.55, "{output:?}");
    assert!(output.manipulation.signals.is_empty());
}

#[test]
fn distribution_detects_price_up_oi_down_and_volume_spike() {
    let output = analyze_market_regime(&MarketFeatureSet {
        symbol: "BTC".to_string(),
        price_change_5m_pct: Some(0.42),
        oi_change_pct: Some(-0.9),
        volume_spike_multiple: Some(2.4),
        funding_rate: Some(0.0002),
        price_impact_efficiency: Some(0.40),
        data_quality: Some(1.0),
        ..MarketFeatureSet::default()
    });

    assert_eq!(output.regime.regime, "DISTRIBUTION");
    assert_eq!(output.regime.direction_bias, "SHORT");
    assert!(output.regime.confidence >= 0.65, "{output:?}");
}

#[test]
fn liquidation_priority_over_manipulation_when_liquidation_cluster_is_heavy() {
    let output = analyze_market_regime(&MarketFeatureSet {
        symbol: "BTC".to_string(),
        price_change_5m_pct: Some(-1.2),
        oi_change_pct: Some(-1.8),
        volume_spike_multiple: Some(3.2),
        funding_rate: Some(-0.001),
        liquidation_ratio: Some(0.42),
        data_quality: Some(0.9),
        ..MarketFeatureSet::default()
    });

    assert_eq!(output.regime.regime, "LIQUIDATION");
    assert!(output
        .manipulation
        .signals
        .contains(&"LIQUIDATION_CLUSTER".to_string()));
    assert_eq!(output.signal.allowed_signal_family, "MEAN_REVERSION_ONLY");
}
