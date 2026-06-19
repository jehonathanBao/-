use btc_toxic_flow_monitor_rs::liquidation_cascade_predictor::{
    analyze_liquidation_cascade, liquidity_gap_from_input, CascadeDirection, CascadeStatus,
    LcpInput,
};

#[test]
fn downside_long_liquidation_cluster_with_void_warns_cascade() {
    let output = analyze_liquidation_cascade(&LcpInput {
        symbol: "BTC".to_string(),
        current_price: Some(65_050.0),
        long_cluster_density: 0.88,
        long_cluster_price: Some(65_000.0),
        short_cluster_density: 0.20,
        oi_change_pct: Some(-1.4),
        funding_rate: Some(-0.0008),
        sell_volume_spike: 0.82,
        liquidity_gap_below: 0.76,
        liquidation_spike: 0.30,
        ..LcpInput::default()
    });

    assert_eq!(output.direction, CascadeDirection::Down);
    assert_eq!(output.status, CascadeStatus::Imminent);
    assert!(output.cascade_probability >= 0.75, "{output:?}");
    assert!(output.signals.contains(&"OI_CLUSTER_HIGH".to_string()));
    assert!(output.signals.contains(&"LIQUIDITY_VOID".to_string()));
    assert!(output.signals.contains(&"TRIGGER_HIT".to_string()));
    assert_eq!(output.risk_zone, Some([64_902.5, 65_097.5]));
    assert!(output.read_only);
    assert!(!output.runtime_modified);
}

#[test]
fn upside_short_cluster_with_buy_trigger_produces_squeeze_bias() {
    let output = analyze_liquidation_cascade(&LcpInput {
        symbol: "BTC".to_string(),
        current_price: Some(64_950.0),
        short_cluster_density: 0.84,
        short_cluster_price: Some(65_020.0),
        long_cluster_density: 0.18,
        oi_change_pct: Some(1.2),
        funding_rate: Some(0.0006),
        buy_volume_spike: 0.78,
        liquidity_gap_above: 0.71,
        liquidation_spike: 0.10,
        ..LcpInput::default()
    });

    assert_eq!(output.direction, CascadeDirection::Up);
    assert!(matches!(
        output.status,
        CascadeStatus::Warning | CascadeStatus::Imminent
    ));
    assert!(output.cascade_probability >= 0.60, "{output:?}");
    assert!(output.signals.contains(&"OI_CLUSTER_HIGH".to_string()));
    assert!(output.signals.contains(&"LIQUIDITY_VOID".to_string()));
}

#[test]
fn low_density_and_no_gap_stays_calm() {
    let output = analyze_liquidation_cascade(&LcpInput {
        symbol: "BTC".to_string(),
        current_price: Some(65_000.0),
        long_cluster_density: 0.10,
        short_cluster_density: 0.12,
        funding_rate: Some(0.0001),
        buy_volume_spike: 0.45,
        sell_volume_spike: 0.46,
        liquidity_gap_above: 0.05,
        liquidity_gap_below: 0.04,
        ..LcpInput::default()
    });

    assert_eq!(output.direction, CascadeDirection::Neutral);
    assert_eq!(output.status, CascadeStatus::Calm);
    assert!(output.cascade_probability < 0.40, "{output:?}");
    assert_eq!(output.signals, vec!["CASCADE_RISK_LOW".to_string()]);
}

#[test]
fn liquidity_gap_facade_exposes_above_and_below_voids() {
    let output = liquidity_gap_from_input(&LcpInput {
        symbol: "BTC".to_string(),
        liquidity_gap_below: 0.74,
        liquidity_gap_above: 0.20,
        ..LcpInput::default()
    });

    assert_eq!(output.dominant_gap, CascadeDirection::Down);
    assert_eq!(output.below_price, 0.74);
    assert!(output.signals.contains(&"LIQUIDITY_VOID_BELOW".to_string()));
    assert!(output.read_only);
    assert!(!output.runtime_modified);
}
