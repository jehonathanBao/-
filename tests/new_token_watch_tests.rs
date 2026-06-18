use btc_toxic_flow_monitor_rs::toxic_v3::new_token_watch::{
    AdvisoryDirection, CapitalPhase, ContractTick, ContractTickSide, FlowActorRegime,
    MarketPriceSnapshot, NewTokenFlowEngine, PriceSource, StabilityRegime, TokenFlowRegime,
    TokenWatchManager, MAX_ACTIVE_TOKENS,
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
    let reconstruction = manager
        .get_reconstruction("ABCUSDT", "15m")
        .expect("reconstruction response");
    assert_eq!(reconstruction.symbol, "ABCUSDT");
    assert_eq!(reconstruction.timeframe, "15m");
    assert_eq!(
        reconstruction.market_price_source,
        PriceSource::Reconstructed
    );
    assert_eq!(reconstruction.analysis_price_source, PriceSource::Vwap);
    assert_eq!(reconstruction.current_price, reconstruction.market_price);
    assert!(reconstruction.analysis_price > 0.0);
    assert!(reconstruction.price_fallback_reason.is_some());
    assert!(reconstruction.read_only);
    assert!(reconstruction.cost_basis_low <= reconstruction.vwap_anchor);
    assert!(reconstruction.vwap_anchor <= reconstruction.cost_basis_high);
    assert!(!reconstruction.capital_timeline.phases.is_empty());
    assert!(reconstruction.capital_timeline.total_duration_sec > 0);
    assert!(!reconstruction.position_flow_curve.points.is_empty());
    assert!(
        reconstruction
            .position_flow_curve
            .accumulation_slope_usd_per_min
            >= 0.0
    );
    assert!(reconstruction.liquidity_reaction_map.absorption_ratio >= 0.0);
    assert!(reconstruction.market_dynamics.market_energy.score >= 0.0);
    assert!(reconstruction.market_dynamics.market_energy.score <= 1.0);
    assert!(!reconstruction.market_dynamics.transition_matrix.is_empty());
    assert!(reconstruction.market_dynamics.state_vector.liquidity >= 0.0);
    assert!(reconstruction.market_dynamics.read_only);
    assert!(reconstruction.liquidity_force.read_only);
    assert_eq!(reconstruction.liquidity_force.liquidation_zones.len(), 2);
    assert!(
        reconstruction
            .liquidity_force
            .forced_flow_attribution
            .liquidation_pct
            >= 0.0
    );
    assert!(
        reconstruction
            .liquidity_force
            .stop_loss_cascade
            .cascade_intensity
            >= 0.0
    );
    assert!(reconstruction.trading_decision.read_only);
    assert!(reconstruction.trading_decision.advisory_only);
    assert!((0.0..=1.0).contains(&reconstruction.trading_decision.confidence));
    assert!((0.0..=100.0).contains(&reconstruction.trading_decision.position_size.pct));
    assert!(reconstruction.execution_strategy.read_only);
    assert!(reconstruction.execution_strategy.advisory_only);
    assert!((0.0..=1.0).contains(&reconstruction.execution_strategy.confidence));
    assert!((0.0..=100.0).contains(&reconstruction.execution_strategy.position_size.pct));
    assert!(!reconstruction.execution_strategy.primary_driver.is_empty());
    assert!(reconstruction
        .execution_strategy
        .reasoning
        .iter()
        .any(|item| item.contains("advisory_only_no_exchange_execution")));
    assert!(!reconstruction.phase_timeline.is_empty());
    assert!(!reconstruction.cost_distribution.is_empty());
    assert!(!reconstruction.smart_levels.is_empty());
    let reconstruction_4h = manager
        .get_reconstruction("ABCUSDT", "4h")
        .expect("4h reconstruction response");
    assert_eq!(reconstruction_4h.timeframe, "4h");
    let market_reconstruction = manager
        .get_reconstruction_with_market(
            "ABCUSDT",
            "15m",
            Some(MarketPriceSnapshot {
                price: 123.45,
                source: PriceSource::MarketPerp,
                updated_at_ms: 1,
                change_24h_pct: Some(1.2),
                volume_24h_usd: Some(4_200_000.0),
                high_24h: Some(130.0),
                low_24h: Some(118.0),
                stale: false,
                fallback_reason: None,
            }),
        )
        .expect("market-backed reconstruction response");
    assert_eq!(market_reconstruction.current_price, 123.45);
    assert_eq!(market_reconstruction.market_price, 123.45);
    assert_eq!(
        market_reconstruction.market_price_source,
        PriceSource::MarketPerp
    );
    assert_eq!(market_reconstruction.price_fallback_reason, None);
    assert_eq!(market_reconstruction.change_24h_pct, Some(1.2));
    assert!(
        ((market_reconstruction.analysis_price - 123.45) / 123.45).abs() < 0.05,
        "analysis price must stay anchored to the market price, got {}",
        market_reconstruction.analysis_price
    );
    assert!(market_reconstruction.cost_basis_low > 100.0);
    assert!(market_reconstruction.cost_basis_high < 130.0);
    let small_market_reconstruction = manager
        .get_reconstruction_with_market(
            "ABCUSDT",
            "15m",
            Some(MarketPriceSnapshot {
                price: 0.7215,
                source: PriceSource::MarketPerp,
                updated_at_ms: 2,
                change_24h_pct: Some(-0.4),
                volume_24h_usd: Some(1_500_000.0),
                high_24h: Some(0.75),
                low_24h: Some(0.69),
                stale: false,
                fallback_reason: None,
            }),
        )
        .expect("small-token market-backed reconstruction response");
    assert_eq!(small_market_reconstruction.market_price, 0.7215);
    assert!(
        small_market_reconstruction.analysis_price < 1.0,
        "small-token analysis price must not fall back to the hash-derived mock price"
    );
    assert!(small_market_reconstruction.cost_basis_low < 1.0);
    assert!(small_market_reconstruction.cost_basis_high < 1.0);
    let small_market_chart = manager
        .get_chart_with_market(
            "ABCUSDT",
            "15m",
            Some(MarketPriceSnapshot {
                price: 0.7215,
                source: PriceSource::MarketPerp,
                updated_at_ms: 3,
                change_24h_pct: None,
                volume_24h_usd: None,
                high_24h: None,
                low_24h: None,
                stale: false,
                fallback_reason: None,
            }),
        )
        .expect("small-token market-backed chart response");
    assert!(
        small_market_chart
            .points
            .iter()
            .all(|point| point.price > 0.0 && point.price < 1.0),
        "small-token chart points must be market-anchored"
    );
    let chart = manager.get_chart("abc", "bad_tf").expect("chart response");
    assert_eq!(chart.timeframe, "15m");
    assert_eq!(chart.market_price_source, PriceSource::Reconstructed);
    assert_eq!(chart.analysis_price_source, PriceSource::Vwap);
    assert!(chart.read_only);
    assert!(!chart.points.is_empty());
    let chart_4h = manager.get_chart("abc", "4h").expect("4h chart response");
    assert_eq!(chart_4h.timeframe, "4h");

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
    assert_eq!(signal.capital_structure.phase, CapitalPhase::Accumulation);
    assert_eq!(
        signal.position_reconstruction.regime_label,
        "accumulation_trajectory"
    );
    assert!(!signal.position_reconstruction.accumulation_path.is_empty());
    assert!(signal
        .position_reconstruction
        .last_accumulation_node
        .is_some());
    assert_eq!(
        signal.position_reconstruction.latent_position.len(),
        accumulation.len()
    );
    assert!(signal.position_reconstruction.confidence > 0.35);
    assert_eq!(signal.capital_structure.behavior_windows.len(), 5);
    assert!(signal
        .capital_structure
        .behavior_windows
        .iter()
        .any(|window| window.window_sec == 14_400));
    assert!(signal.capital_structure.phase_confidence > 0.45);
    assert!(
        signal.capital_structure.cost_basis.lower
            <= signal.capital_structure.cost_basis.vwap_anchor
    );
    assert!(
        signal.capital_structure.cost_basis.vwap_anchor
            <= signal.capital_structure.cost_basis.upper
    );
    assert!(
        signal.capital_structure.estimated_position.upper_usd
            >= signal.capital_structure.estimated_position.lower_usd
    );
    assert_eq!(signal.capital_structure.distribution_risk.level, "low");
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
    assert!(matches!(
        signal.capital_structure.phase,
        CapitalPhase::Distribution | CapitalPhase::Breakdown
    ));
    assert!(!signal.position_reconstruction.distribution_path.is_empty());
    assert!(signal
        .position_reconstruction
        .latent_position
        .last()
        .map(|point| point.impact_adjusted_position < 0.0)
        .unwrap_or(false));
    assert!(signal.capital_structure.distribution_risk.score > 0.25);
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
    assert_eq!(signal.capital_structure.phase, CapitalPhase::Markup);
    assert!(signal
        .position_reconstruction
        .accumulation_path
        .iter()
        .any(|segment| matches!(segment.phase, CapitalPhase::Markup)));
    assert!(signal
        .capital_structure
        .behavior_windows
        .iter()
        .any(|window| window.window_sec == 300));
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
