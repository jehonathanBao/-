use btc_toxic_flow_monitor_rs::{
    toxicity::liquidation_toxicity::analyze_liquidation_toxicity,
    types::{
        liquidation::{
            EstimatedLiquidationCluster, LiquidationClusterSide, LiquidationMetrics,
            LiquidationState, LiquidationToxicSignalType,
        },
        toxic::ToxicDirection,
        toxic_flow::{
            ActiveTradeToxicSignal, ActiveTradeToxicSignalType, ActiveTradeToxicityRecentResponse,
            ToxicConfidence, ToxicSide,
        },
    },
};

#[test]
fn nearby_cluster_produces_liquidation_cluster_nearby() {
    let assessment = analyze_liquidation_toxicity(
        "BTC-PERP",
        &liquidation_state(
            Some(cluster(
                LiquidationClusterSide::ShortAbove,
                100_300.0,
                30.0,
                2_500_000.0,
                0.68,
            )),
            Some(cluster(
                LiquidationClusterSide::LongBelow,
                99_100.0,
                90.0,
                1_200_000.0,
                0.42,
            )),
            vec![],
        ),
        &empty_active_trade_recent(),
    );

    assert!(assessment
        .signals
        .iter()
        .any(|signal| signal.signal_type == LiquidationToxicSignalType::LiquidationClusterNearby));
    assert!(assessment.signals.iter().all(|signal| signal.read_only));
}

#[test]
fn denser_nearer_upside_cluster_produces_upside_magnet() {
    let assessment = analyze_liquidation_toxicity(
        "BTC-PERP",
        &liquidation_state(
            Some(cluster(
                LiquidationClusterSide::ShortAbove,
                100_250.0,
                25.0,
                3_000_000.0,
                0.82,
            )),
            Some(cluster(
                LiquidationClusterSide::LongBelow,
                99_100.0,
                90.0,
                1_200_000.0,
                0.35,
            )),
            vec![],
        ),
        &empty_active_trade_recent(),
    );

    assert!(assessment
        .signals
        .iter()
        .any(|signal| signal.signal_type == LiquidationToxicSignalType::UpsideLiquidationMagnet));
}

#[test]
fn denser_nearer_downside_cluster_produces_downside_magnet() {
    let assessment = analyze_liquidation_toxicity(
        "BTC-PERP",
        &liquidation_state(
            Some(cluster(
                LiquidationClusterSide::ShortAbove,
                100_900.0,
                90.0,
                1_100_000.0,
                0.30,
            )),
            Some(cluster(
                LiquidationClusterSide::LongBelow,
                99_700.0,
                30.0,
                2_800_000.0,
                0.78,
            )),
            vec![],
        ),
        &empty_active_trade_recent(),
    );

    assert!(assessment
        .signals
        .iter()
        .any(|signal| signal.signal_type == LiquidationToxicSignalType::DownsideLiquidationMagnet));
}

#[test]
fn upside_cluster_plus_bullish_flow_produces_short_squeeze_risk() {
    let assessment = analyze_liquidation_toxicity(
        "BTC-PERP",
        &liquidation_state(
            Some(cluster(
                LiquidationClusterSide::ShortAbove,
                100_260.0,
                26.0,
                3_200_000.0,
                0.80,
            )),
            None,
            vec![],
        ),
        &active_trade_recent(vec![active_signal(
            "active-buy-1",
            ActiveTradeToxicSignalType::LargeAggressiveBuy,
            ToxicSide::Buy,
        )]),
    );

    let signal = assessment
        .signals
        .iter()
        .find(|signal| signal.signal_type == LiquidationToxicSignalType::ShortSqueezeRisk)
        .expect("short squeeze signal");
    assert_eq!(
        signal.linked_active_trade_signal_ids,
        vec!["active-buy-1".to_string()]
    );
}

#[test]
fn downside_cluster_plus_bearish_flow_produces_long_squeeze_risk() {
    let assessment = analyze_liquidation_toxicity(
        "BTC-PERP",
        &liquidation_state(
            None,
            Some(cluster(
                LiquidationClusterSide::LongBelow,
                99_740.0,
                26.0,
                3_200_000.0,
                0.80,
            )),
            vec![],
        ),
        &active_trade_recent(vec![active_signal(
            "active-sell-1",
            ActiveTradeToxicSignalType::LargeAggressiveSell,
            ToxicSide::Sell,
        )]),
    );

    assert!(assessment
        .signals
        .iter()
        .any(|signal| signal.signal_type == LiquidationToxicSignalType::LongSqueezeRisk));
}

#[test]
fn stepped_clusters_produce_liquidation_cascade_candidate() {
    let nearby = cluster(
        LiquidationClusterSide::ShortAbove,
        100_250.0,
        25.0,
        2_000_000.0,
        0.60,
    );
    let second = cluster(
        LiquidationClusterSide::ShortAbove,
        100_520.0,
        52.0,
        1_900_000.0,
        0.58,
    );
    let assessment = analyze_liquidation_toxicity(
        "BTC-PERP",
        &liquidation_state(Some(nearby.clone()), None, vec![nearby, second]),
        &empty_active_trade_recent(),
    );

    assert!(assessment.signals.iter().any(
        |signal| signal.signal_type == LiquidationToxicSignalType::LiquidationCascadeCandidate
    ));
}

#[test]
fn one_hour_buy_delta_cluster_alignment_produces_confluence() {
    let assessment = analyze_liquidation_toxicity(
        "BTC-PERP",
        &liquidation_state(
            Some(cluster(
                LiquidationClusterSide::ShortAbove,
                100_280.0,
                28.0,
                2_700_000.0,
                0.74,
            )),
            None,
            vec![],
        ),
        &active_trade_recent(vec![active_signal(
            "delta-buy-1",
            ActiveTradeToxicSignalType::OneHourDeltaBuyDominant,
            ToxicSide::Buy,
        )]),
    );

    let signal = assessment
        .signals
        .iter()
        .find(|signal| signal.signal_type == LiquidationToxicSignalType::LiquidationDeltaConfluence)
        .expect("confluence signal");
    assert_eq!(
        signal.linked_active_trade_signal_ids,
        vec!["delta-buy-1".to_string()]
    );
    assert!(signal.toxicity_score <= 100);
    assert!(!signal.reason.is_empty());
}

fn liquidation_state(
    nearest_short: Option<EstimatedLiquidationCluster>,
    nearest_long: Option<EstimatedLiquidationCluster>,
    recent_clusters: Vec<EstimatedLiquidationCluster>,
) -> LiquidationState {
    LiquidationState {
        symbol: "BTC-PERP".to_string(),
        updated_at: 1_760_000_000_000,
        metrics: LiquidationMetrics {
            enabled: true,
            lookback_ms: 120_000,
            cluster_band_bps: 6.0,
            proximity_threshold_bps: 25.0,
            current_mid: Some(100_000.0),
            nearest_short_liq_cluster_above: nearest_short.clone(),
            nearest_long_liq_cluster_below: nearest_long.clone(),
            dominant_direction: ToxicDirection::Neutral,
            nearest_cluster_side: nearest_short
                .as_ref()
                .map(|cluster| cluster.side)
                .or_else(|| nearest_long.as_ref().map(|cluster| cluster.side)),
            distance_bps: nearest_short
                .as_ref()
                .map(|cluster| cluster.distance_bps)
                .or_else(|| nearest_long.as_ref().map(|cluster| cluster.distance_bps)),
            cluster_notional_usd: nearest_short
                .as_ref()
                .map(|cluster| cluster.cluster_notional_usd)
                .or_else(|| {
                    nearest_long
                        .as_ref()
                        .map(|cluster| cluster.cluster_notional_usd)
                }),
            cluster_density: nearest_short
                .as_ref()
                .map(|cluster| cluster.cluster_density)
                .or_else(|| nearest_long.as_ref().map(|cluster| cluster.cluster_density)),
            liq_hunt_pressure: 0.72,
            liq_cluster_nearby: true,
            possible_liq_hunt_setup: false,
            reason_codes: vec!["liquidation_cluster_detected".to_string()],
        },
        recent_clusters,
    }
}

fn cluster(
    side: LiquidationClusterSide,
    price: f64,
    distance_bps: f64,
    cluster_notional_usd: f64,
    cluster_density: f64,
) -> EstimatedLiquidationCluster {
    EstimatedLiquidationCluster {
        side,
        price,
        distance_bps,
        cluster_notional_usd,
        cluster_density,
        touched_snapshots: 4,
        first_seen_ts: 1_759_999_900_000,
        last_seen_ts: 1_760_000_000_000,
        reason_codes: vec!["cluster".to_string()],
    }
}

fn empty_active_trade_recent() -> ActiveTradeToxicityRecentResponse {
    active_trade_recent(Vec::new())
}

fn active_trade_recent(signals: Vec<ActiveTradeToxicSignal>) -> ActiveTradeToxicityRecentResponse {
    ActiveTradeToxicityRecentResponse {
        read_only: true,
        runtime_modified: false,
        mode: "analysis_only".to_string(),
        selected_symbol: "BTC-PERP".to_string(),
        status: "neutral".to_string(),
        score: 40.0,
        side_bias: "neutral".to_string(),
        warnings: Vec::new(),
        no_trade_reasons: Vec::new(),
        signals,
    }
}

fn active_signal(
    signal_id: &str,
    signal_type: ActiveTradeToxicSignalType,
    side: ToxicSide,
) -> ActiveTradeToxicSignal {
    ActiveTradeToxicSignal {
        signal_id: signal_id.to_string(),
        symbol: "BTC-PERP".to_string(),
        ts_ms: 1_760_000_000_000,
        signal_type,
        side,
        timeframe: Some("1h".to_string()),
        candle_open_ms: None,
        candle_close_ms: None,
        window_ms: 3_600_000,
        delta: Some(2_100.0),
        abs_delta: Some(2_100.0),
        threshold: Some(2_000.0),
        aggressive_volume: 2_100.0,
        notional_usd: 2_100_000.0,
        trade_count: 10,
        cvd_delta: 2_100.0,
        buy_volume: if matches!(side, ToxicSide::Buy) {
            2_000_000.0
        } else {
            100_000.0
        },
        sell_volume: if matches!(side, ToxicSide::Sell) {
            2_000_000.0
        } else {
            100_000.0
        },
        imbalance_ratio: 0.85,
        open: Some(100_000.0),
        high: Some(100_100.0),
        low: Some(99_950.0),
        close: Some(100_020.0),
        price_impact_bps: Some(3.0),
        price_change_bps: Some(2.0),
        upper_wick_ratio: Some(0.1),
        lower_wick_ratio: Some(0.1),
        markout_5s: None,
        markout_15s: None,
        markout_60s: None,
        toxicity_score: 75,
        confidence: ToxicConfidence::High,
        reason: vec!["active trade confluence".to_string()],
        read_only: true,
    }
}
