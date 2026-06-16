use btc_toxic_flow_monitor_rs::toxic_v3::{
    BreakoutBias, CascadeDirection, DealerBias, DecisionEngine, Direction, EvaluationEngine,
    InMemorySignalStore, MarketFlowExchange, MarketFlowTick, MarketRegime, SignalAggregator,
    SignalSource, SignalStore, SystemEvaluationSample, SystemEvaluationVerdict, SystemHistory,
};

#[test]
fn labeled_history_reports_accuracy_false_positive_and_false_negative_rates() {
    let true_positive = sample("BTCUSDT", Direction::Buy, MarketRegime::FragileAccumulation)
        .with_observed_liquidation(true)
        .with_observed_squeeze(true)
        .with_observed_breakout(true);
    let false_positive = sample("BTCUSDT", Direction::Buy, MarketRegime::FragileAccumulation)
        .with_observed_liquidation(false)
        .with_observed_squeeze(false)
        .with_observed_breakout(false);
    let false_negative = SystemEvaluationSample {
        predicted_liquidation: false,
        predicted_squeeze: false,
        predicted_breakout: false,
        ..sample("BTCUSDT", Direction::Neutral, MarketRegime::Stable)
    }
    .with_observed_liquidation(true)
    .with_observed_squeeze(true)
    .with_observed_breakout(true);
    let history = SystemHistory {
        samples: vec![true_positive, false_positive, false_negative],
    };

    let evaluation = EvaluationEngine::evaluate(&history);

    assert_eq!(evaluation.evaluated_sample_count, 3);
    assert_eq!(evaluation.labeled_event_count, 9);
    assert!(
        (evaluation.prediction_accuracy - (1.0 / 3.0)).abs() < 0.001,
        "{evaluation:?}"
    );
    assert!(
        (evaluation.false_positive_rate - (1.0 / 3.0)).abs() < 0.001,
        "{evaluation:?}"
    );
    assert!(
        (evaluation.false_negative_rate - (1.0 / 3.0)).abs() < 0.001,
        "{evaluation:?}"
    );
}

#[test]
fn stable_consistent_unlabeled_history_gets_structural_confidence_not_fake_labels() {
    let history = SystemHistory {
        samples: vec![
            sample("ETHUSDT", Direction::Buy, MarketRegime::FragileAccumulation),
            sample("ETHUSDT", Direction::Buy, MarketRegime::FragileAccumulation),
            sample("ETHUSDT", Direction::Buy, MarketRegime::FragileAccumulation),
        ],
    };

    let evaluation = EvaluationEngine::evaluate(&history);

    assert_eq!(evaluation.labeled_event_count, 0);
    assert!(evaluation.regime_stability_score >= 0.95, "{evaluation:?}");
    assert!(
        evaluation.structural_consistency_score >= 0.75,
        "{evaluation:?}"
    );
    assert!(evaluation.system_confidence >= 0.70, "{evaluation:?}");
    assert!(evaluation
        .risk_factors
        .iter()
        .any(|factor| factor == "no_observed_outcome_labels"));
}

#[test]
fn regime_churn_and_cross_layer_conflict_reduce_confidence() {
    let mut conflict_one = sample("SOLUSDT", Direction::Buy, MarketRegime::FragileAccumulation);
    conflict_one.glce_bias = BreakoutBias::ShortSqueeze;
    conflict_one.lhcs_direction = CascadeDirection::DownwardSqueeze;
    conflict_one.dealer_bias = DealerBias::SellRallies;
    conflict_one.mff_direction = Direction::Sell;

    let mut conflict_two = sample(
        "SOLUSDT",
        Direction::Sell,
        MarketRegime::CriticalInstability,
    );
    conflict_two.glce_bias = BreakoutBias::LongSqueeze;
    conflict_two.lhcs_direction = CascadeDirection::UpwardSqueeze;
    conflict_two.dealer_bias = DealerBias::BuyDips;
    conflict_two.mff_direction = Direction::Buy;

    let history = SystemHistory {
        samples: vec![
            conflict_one,
            conflict_two,
            sample("SOLUSDT", Direction::Neutral, MarketRegime::Stable),
            sample("SOLUSDT", Direction::Buy, MarketRegime::Compression),
        ],
    };

    let evaluation = EvaluationEngine::evaluate(&history);

    assert!(evaluation.regime_stability_score < 0.55, "{evaluation:?}");
    assert!(
        evaluation.structural_consistency_score < 0.70,
        "{evaluation:?}"
    );
    assert!(matches!(
        evaluation.verdict,
        SystemEvaluationVerdict::NeedsCalibration | SystemEvaluationVerdict::Unreliable
    ));
}

#[test]
fn in_memory_signal_store_can_evaluate_recent_read_only_signals() {
    let decision = DecisionEngine::default();
    let mut store = InMemorySignalStore::new(16);
    let signal = SignalAggregator::evaluate_tick(
        &force_flow("BTCUSDT"),
        SignalSource::FlowInference,
        92.0,
        &decision,
    );

    store.record(&signal);
    let evaluation = store.evaluate_system();

    assert_eq!(evaluation.evaluated_sample_count, 1);
    assert_eq!(evaluation.labeled_event_count, 0);
    assert!(evaluation.system_confidence > 0.0, "{evaluation:?}");
    assert!(!signal.external_dispatch_enabled);
    assert!(signal.enrichment.read_only);
}

fn sample(symbol: &str, direction: Direction, regime: MarketRegime) -> SystemEvaluationSample {
    SystemEvaluationSample {
        ts: 1_700_000_000_000,
        symbol: symbol.to_string(),
        direction,
        signal_type: btc_toxic_flow_monitor_rs::toxic_v3::SignalType::LiquidationCascade,
        predicted_liquidation: direction != Direction::Neutral,
        predicted_squeeze: direction != Direction::Neutral,
        predicted_breakout: direction != Direction::Neutral,
        observed_liquidation: None,
        observed_squeeze: None,
        observed_breakout: None,
        regime,
        glce_bias: match direction {
            Direction::Buy => BreakoutBias::LongSqueeze,
            Direction::Sell => BreakoutBias::ShortSqueeze,
            Direction::Absorption | Direction::Suppression | Direction::Neutral => {
                BreakoutBias::Neutral
            }
        },
        lhcs_direction: match direction {
            Direction::Buy => CascadeDirection::UpwardSqueeze,
            Direction::Sell => CascadeDirection::DownwardSqueeze,
            Direction::Absorption | Direction::Suppression | Direction::Neutral => {
                CascadeDirection::Neutral
            }
        },
        dealer_bias: match direction {
            Direction::Buy => DealerBias::BuyDips,
            Direction::Sell => DealerBias::SellRallies,
            Direction::Absorption | Direction::Suppression | Direction::Neutral => {
                DealerBias::Neutral
            }
        },
        mff_direction: direction,
        mff_stress: if direction == Direction::Neutral {
            0.20
        } else {
            0.72
        },
        glce_squeeze_probability: if direction == Direction::Neutral {
            0.20
        } else {
            0.76
        },
        lhcs_cascade_probability: if direction == Direction::Neutral {
            0.20
        } else {
            0.74
        },
        gex_squeeze_probability: if direction == Direction::Neutral {
            0.20
        } else {
            0.71
        },
    }
}

fn force_flow(symbol: &str) -> MarketFlowTick {
    MarketFlowTick {
        ts: 1_700_000_110_000,
        exchange: MarketFlowExchange::Binance,
        symbol: symbol.to_string(),
        buy_volume: 12_600.0,
        sell_volume: 1_600.0,
        net_flow: 11_000.0,
        flow_acceleration: 6_800.0,
        trade_count: 420,
        avg_trade_size: 30.0,
        large_trade_ratio: 0.84,
        realized_vol: 0.90,
        open_interest_delta: 12_100.0,
        funding_rate: 0.0015,
        liquidation_pressure: 0.90,
        price_move_pct: 0.88,
        dynamic_multiple: 10.0,
        anomaly_persistence_sec: 520.0,
        cross_exchange_dispersion: 0.22,
    }
}
