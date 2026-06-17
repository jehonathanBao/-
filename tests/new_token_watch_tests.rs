use btc_toxic_flow_monitor_rs::toxic_v3::new_token_watch::{
    AdvisoryDirection, ContractTick, ContractTickSide, FlowActorRegime, NewTokenFlowEngine,
    StabilityRegime, TokenFlowRegime, TokenWatchManager, MAX_ACTIVE_TOKENS,
};

fn tick(
    symbol: &str,
    idx: u64,
    side: ContractTickSide,
    price: f64,
    imbalance: f64,
) -> ContractTick {
    ContractTick {
        symbol: symbol.to_string(),
        price,
        size: 10.0 + idx as f64,
        side,
        aggression: 0.82,
        orderbook_imbalance: imbalance,
        timestamp: idx,
    }
}

#[test]
fn manager_add_remove_and_capacity_limit() {
    let manager = TokenWatchManager::default();

    let first = manager.add_token("abc").expect("add abc");
    assert_eq!(first.symbol, "ABCUSDT");
    assert_eq!(manager.list_active_tokens().active_count, 1);

    for idx in 0..(MAX_ACTIVE_TOKENS - 1) {
        manager
            .add_token(&format!("T{idx}USDT"))
            .expect("fill capacity");
    }

    assert_eq!(manager.list_active_tokens().active_count, MAX_ACTIVE_TOKENS);
    let error = manager
        .add_token("overflow")
        .expect_err("capacity must reject");
    assert_eq!(error.to_string(), "max_active_tokens_reached");

    let removed = manager.remove_token("ABCUSDT").expect("remove abc");
    assert_eq!(removed.symbol, "ABCUSDT");
    assert_eq!(
        manager.list_active_tokens().active_count,
        MAX_ACTIVE_TOKENS - 1
    );
}

#[test]
fn engine_detects_accumulation_distribution_and_building() {
    let accumulation = (0..12)
        .map(|idx| {
            tick(
                "ABCUSDT",
                idx,
                ContractTickSide::Buy,
                10.0 + idx as f64 * 0.003,
                0.22,
            )
        })
        .collect::<Vec<_>>();
    let signal = NewTokenFlowEngine::analyze_ticks("ABCUSDT", &accumulation);
    assert_eq!(signal.regime, TokenFlowRegime::Accumulation);
    assert!(signal.strength > 0.6);
    assert_eq!(signal.ofi_windows.len(), 3);
    assert!(signal.flow_persistence > 0.7);
    assert!(signal.impact_response.absorption_score > 0.6);
    assert_eq!(
        signal.actor_decomposition.dominant_actor,
        FlowActorRegime::SmartMoney
    );
    assert!(signal.actor_decomposition.smart_money_probability > 0.35);
    assert!(signal.signal_compression.smart_money_pressure > 0.25);
    assert!(
        signal
            .signal_compression
            .position_validity_gate
            .advisory_only
    );
    assert!(
        signal
            .signal_compression
            .position_validity_gate
            .position_size_multiplier
            <= 1.0
    );
    assert_eq!(
        signal.signal_compression.stability_kernel.regime,
        StabilityRegime::LiquidityExpansion
    );
    assert_eq!(
        signal
            .signal_compression
            .stability_kernel
            .trade_signal
            .direction,
        AdvisoryDirection::Long
    );
    assert!(
        signal
            .signal_compression
            .stability_kernel
            .position_smoothing
            .suggested_size_multiplier
            > 0.0
    );

    let distribution = (0..12)
        .map(|idx| {
            tick(
                "DEFUSDT",
                idx,
                ContractTickSide::Sell,
                10.0 - idx as f64 * 0.002,
                -0.24,
            )
        })
        .collect::<Vec<_>>();
    let signal = NewTokenFlowEngine::analyze_ticks("DEFUSDT", &distribution);
    assert_eq!(signal.regime, TokenFlowRegime::Distribution);
    assert!(signal.liquidity_depletion.bid_depletion_rate > 0.3);
    assert!(signal.signal_compression.smart_money_pressure < -0.10);
    assert_eq!(
        signal
            .signal_compression
            .stability_kernel
            .trade_signal
            .direction,
        AdvisoryDirection::Short
    );
    assert!(signal
        .evidence
        .iter()
        .any(|item| item.contains("sell_aggression")));

    let building = (0..12)
        .map(|idx| {
            tick(
                "GHIUSDT",
                idx,
                ContractTickSide::Buy,
                10.0 + idx as f64 * 0.06,
                0.03,
            )
        })
        .collect::<Vec<_>>();
    let signal = NewTokenFlowEngine::analyze_ticks("GHIUSDT", &building);
    assert_eq!(signal.regime, TokenFlowRegime::Building);
    assert!(signal.confidence > 0.55);
    assert_eq!(signal.impact_response.classification, "thin_liquidity");
    assert!(signal.signal_compression.momentum_flow_exhaustion > 0.0);
    assert!((-1.0..=1.0).contains(&signal.signal_compression.liquidity_stress_manipulation));
    assert_eq!(
        signal.signal_compression.stability_kernel.regime,
        StabilityRegime::Trend
    );
    assert_eq!(
        signal.actor_decomposition.dominant_actor,
        FlowActorRegime::MomentumChaser
    );
    let actor_probability_sum = signal.actor_decomposition.liquidity_provider_probability
        + signal.actor_decomposition.momentum_chaser_probability
        + signal.actor_decomposition.smart_money_probability;
    assert!((actor_probability_sum - 1.0).abs() < 0.000_001);
}
