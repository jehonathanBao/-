use super::types::{
    AdvisoryDirection, BehaviorWindowMetrics, CapitalPhase, CapitalStructureView, ContractTick,
    ContractTickSide, CostBasisEstimate, DistributionRisk, EstimatedPositionSize, ExpectedHoldTime,
    FlowActorRegime, ImpactResponse, LastAccumulationNode, LatentPositionPoint, LiquidityDepletion,
    OfiWindowMetrics, PositionPathSegment, PositionSmoothing, PositionValidityGate, RegimeState,
    SignalCompressionState, SmartMoneyDecomposition, SmartMoneyPositionReconstruction,
    StabilityRegime, StableSignals, TimeHorizonInference, TokenFlowRegime, TokenFlowSignal,
    TradeSignalAdvisory, TradingStabilityKernel,
};
use crate::normalizers::trade::now_ms;

pub struct NewTokenFlowEngine;

impl NewTokenFlowEngine {
    pub fn analyze_ticks(symbol: &str, ticks: &[ContractTick]) -> TokenFlowSignal {
        if ticks.is_empty() {
            return TokenFlowSignal {
                symbol: symbol.to_string(),
                regime: TokenFlowRegime::Neutral,
                strength: 0.0,
                confidence: 0.0,
                ofi_windows: vec![],
                flow_persistence: 0.0,
                impact_response: ImpactResponse::default(),
                liquidity_depletion: LiquidityDepletion::default(),
                actor_decomposition: SmartMoneyDecomposition::default(),
                signal_compression: SignalCompressionState::default(),
                capital_structure: CapitalStructureView::default(),
                position_reconstruction: SmartMoneyPositionReconstruction::default(),
                evidence: vec!["no_contract_flow_observed".to_string()],
                read_only: true,
                detector: "new_token_flow_engine_v1".to_string(),
                updated_at_ms: now_ms(),
            };
        }

        let mut buy_pressure = 0.0;
        let mut sell_pressure = 0.0;
        let mut total_size = 0.0;
        let mut imbalance_sum = 0.0;
        let mut min_price = f64::MAX;
        let mut max_price = 0.0_f64;

        for tick in ticks {
            let size = tick.size.max(0.0);
            total_size += size;
            let pressure = size * tick.aggression.clamp(0.0, 1.0);
            match tick.side {
                ContractTickSide::Buy => buy_pressure += pressure,
                ContractTickSide::Sell => sell_pressure += pressure,
            }
            imbalance_sum += tick.orderbook_imbalance.clamp(-1.0, 1.0);
            min_price = min_price.min(tick.price);
            max_price = max_price.max(tick.price);
        }

        let first_price = ticks.first().map(|tick| tick.price).unwrap_or_default();
        let last_price = ticks.last().map(|tick| tick.price).unwrap_or_default();
        let pressure_total = (buy_pressure + sell_pressure).max(0.000_001);
        let net_ratio = (buy_pressure - sell_pressure) / pressure_total;
        let avg_imbalance = imbalance_sum / ticks.len() as f64;
        let price_change = if first_price > 0.0 {
            (last_price - first_price) / first_price
        } else {
            0.0
        };
        let price_range = if first_price > 0.0 {
            (max_price - min_price).abs() / first_price
        } else {
            0.0
        };
        let activity = (total_size / ticks.len() as f64).min(100.0) / 100.0;
        let ofi_windows = build_ofi_windows(ticks);
        let flow_persistence = ofi_windows
            .iter()
            .map(|window| window.persistence)
            .fold(0.0_f64, f64::max);
        let impact_response = build_impact_response(ticks, total_size, price_change);
        let liquidity_depletion =
            build_liquidity_depletion(buy_pressure, sell_pressure, pressure_total, avg_imbalance);
        let actor_decomposition = build_actor_decomposition(
            &ofi_windows,
            flow_persistence,
            &impact_response,
            &liquidity_depletion,
            price_range,
            avg_imbalance,
            activity,
        );
        let signal_compression = build_signal_compression(
            net_ratio,
            price_change,
            price_range,
            flow_persistence,
            &ofi_windows,
            &impact_response,
            &liquidity_depletion,
            &actor_decomposition,
            avg_imbalance,
        );
        let capital_structure = build_capital_structure(
            ticks,
            net_ratio,
            price_change,
            price_range,
            flow_persistence,
            &impact_response,
            &liquidity_depletion,
            &signal_compression,
        );
        let position_reconstruction = build_position_reconstruction(
            ticks,
            &capital_structure,
            &impact_response,
            &liquidity_depletion,
            &signal_compression,
        );

        let (regime, strength, confidence, mut evidence) = if net_ratio > 0.18
            && avg_imbalance > 0.08
            && (-0.006..=0.018).contains(&price_change)
            && price_range < 0.04
            && impact_response.absorption_score >= 0.45
        {
            (
                TokenFlowRegime::Accumulation,
                clamp01(
                    0.38 + net_ratio.abs() * 0.25
                        + avg_imbalance.max(0.0) * 0.2
                        + activity * 0.1
                        + impact_response.absorption_score * 0.18
                        + flow_persistence * 0.12,
                ),
                clamp01(
                    0.48 + net_ratio.abs() * 0.16
                        + (0.04 - price_range).max(0.0) * 2.0
                        + flow_persistence * 0.18,
                ),
                vec![
                    "buy_aggression_with_compressed_price".to_string(),
                    "bid_depth_supporting_flow".to_string(),
                    "impact_absorption_detected".to_string(),
                ],
            )
        } else if net_ratio < -0.18
            && avg_imbalance < -0.06
            && liquidity_depletion.bid_depletion_rate >= liquidity_depletion.replenishment_rate
        {
            (
                TokenFlowRegime::Distribution,
                clamp01(
                    0.40 + net_ratio.abs() * 0.28
                        + avg_imbalance.abs() * 0.18
                        + activity * 0.1
                        + liquidity_depletion.depletion_pressure * 0.18
                        + flow_persistence * 0.12,
                ),
                clamp01(
                    0.48 + net_ratio.abs() * 0.18 + price_range.min(0.05) + flow_persistence * 0.15,
                ),
                vec![
                    "sell_aggression_dominates".to_string(),
                    "liquidity_exhaustion_on_bid_side".to_string(),
                    "bid_depletion_exceeds_replenishment".to_string(),
                ],
            )
        } else if net_ratio.abs() > 0.28
            && price_change.signum() == net_ratio.signum()
            && flow_persistence > 0.45
        {
            (
                TokenFlowRegime::Building,
                clamp01(
                    0.36 + net_ratio.abs() * 0.30
                        + price_change.abs().min(0.05) * 1.5
                        + flow_persistence * 0.2
                        + impact_response.thin_liquidity_score * 0.08,
                ),
                clamp01(0.44 + net_ratio.abs() * 0.18 + activity * 0.14 + flow_persistence * 0.18),
                vec![
                    "sustained_directional_flow".to_string(),
                    "price_response_confirms_flow".to_string(),
                    "rolling_ofi_persistence_confirmed".to_string(),
                ],
            )
        } else {
            (
                TokenFlowRegime::Neutral,
                clamp01(net_ratio.abs() * 0.35 + activity * 0.12),
                clamp01(0.35 + ticks.len() as f64 / 50.0),
                vec!["no_durable_contract_flow_regime".to_string()],
            )
        };

        evidence.push(format!("net_pressure_ratio={:.2}", net_ratio));
        evidence.push(format!("price_range_pct={:.2}", price_range * 100.0));
        evidence.push(format!(
            "impact_classification={}",
            impact_response.classification
        ));
        evidence.push(format!("flow_persistence={:.2}", flow_persistence));
        evidence.push(format!(
            "dominant_actor={:?}",
            actor_decomposition.dominant_actor
        ));
        evidence.push(format!(
            "scl_smp={:.2},mfe={:.2},lsm={:.2}",
            signal_compression.smart_money_pressure,
            signal_compression.momentum_flow_exhaustion,
            signal_compression.liquidity_stress_manipulation
        ));
        evidence.push(format!("capital_phase={:?}", capital_structure.phase));
        evidence.push(format!(
            "cost_basis_vwap={:.6}",
            capital_structure.cost_basis.vwap_anchor
        ));
        evidence.push(format!(
            "reconstruction_confidence={:.2}",
            position_reconstruction.confidence
        ));

        TokenFlowSignal {
            symbol: symbol.to_string(),
            regime,
            strength,
            confidence,
            ofi_windows,
            flow_persistence,
            impact_response,
            liquidity_depletion,
            actor_decomposition,
            signal_compression,
            capital_structure,
            position_reconstruction,
            evidence,
            read_only: true,
            detector: "new_token_flow_engine_v1".to_string(),
            updated_at_ms: now_ms(),
        }
    }
}

fn clamp01(value: f64) -> f64 {
    value.clamp(0.0, 1.0)
}

fn clamp_signed(value: f64) -> f64 {
    value.clamp(-1.0, 1.0)
}

fn build_ofi_windows(ticks: &[ContractTick]) -> Vec<OfiWindowMetrics> {
    [5_u64, 30, 60]
        .into_iter()
        .map(|window_sec| build_ofi_window(ticks, window_sec))
        .collect()
}

fn build_ofi_window(ticks: &[ContractTick], window_sec: u64) -> OfiWindowMetrics {
    let latest_ts = ticks
        .iter()
        .map(|tick| tick.timestamp)
        .max()
        .unwrap_or_default();
    let window_ms = window_sec.saturating_mul(1000);
    let scoped = ticks
        .iter()
        .filter(|tick| {
            latest_ts < window_ms || tick.timestamp >= latest_ts.saturating_sub(window_ms)
        })
        .collect::<Vec<_>>();
    let selected = if scoped.is_empty() {
        ticks.iter().collect::<Vec<_>>()
    } else {
        scoped
    };

    let mut buy_pressure = 0.0;
    let mut sell_pressure = 0.0;
    let mut signed_sum = 0.0;
    let mut decayed_sum = 0.0;
    let mut decayed_abs = 0.0;
    let mut same_direction_runs = 0_usize;
    let mut previous_sign = 0_i8;

    for tick in selected {
        let pressure = tick.size.max(0.0) * tick.aggression.clamp(0.0, 1.0);
        let sign = match tick.side {
            ContractTickSide::Buy => 1_i8,
            ContractTickSide::Sell => -1_i8,
        };
        if sign > 0 {
            buy_pressure += pressure;
        } else {
            sell_pressure += pressure;
        }
        if previous_sign == sign {
            same_direction_runs += 1;
        }
        previous_sign = sign;
        let signed = pressure * f64::from(sign);
        signed_sum += signed;
        let age_ms = latest_ts.saturating_sub(tick.timestamp);
        let half_life_ms = (window_ms / 2).max(1);
        let decay = (-(age_ms as f64) / half_life_ms as f64).exp();
        decayed_sum += signed * decay;
        decayed_abs += pressure.abs() * decay;
    }

    let total = (buy_pressure + sell_pressure).max(0.000_001);
    let normalized_ofi = signed_sum / total;
    OfiWindowMetrics {
        window_sec,
        buy_pressure,
        sell_pressure,
        net_ofi: buy_pressure - sell_pressure,
        normalized_ofi,
        decay_weighted_ofi: decayed_sum / decayed_abs.max(0.000_001),
        persistence: clamp01(
            0.5 * normalized_ofi.abs()
                + 0.5 * same_direction_runs as f64 / ticks.len().max(1) as f64,
        ),
    }
}

fn build_impact_response(
    ticks: &[ContractTick],
    total_volume: f64,
    price_change: f64,
) -> ImpactResponse {
    let abs_price_move_pct = price_change.abs() * 100.0;
    let impact_per_volume = abs_price_move_pct / total_volume.max(0.000_001);
    let volume_intensity = clamp01(total_volume / 120.0);
    let low_price_response = clamp01((0.9 - abs_price_move_pct).max(0.0) / 0.9);
    let high_price_response = clamp01(abs_price_move_pct / 2.0);
    let absorption_score = clamp01(volume_intensity * 0.62 + low_price_response * 0.38);
    let thin_liquidity_score =
        clamp01(high_price_response * 0.65 + (1.0 - volume_intensity) * 0.35);
    let classification = if absorption_score >= 0.65 && absorption_score >= thin_liquidity_score {
        "absorption"
    } else if thin_liquidity_score >= 0.60 {
        "thin_liquidity"
    } else if ticks.len() <= 2 {
        "insufficient_window"
    } else {
        "balanced_response"
    };

    ImpactResponse {
        price_move_pct: price_change,
        total_volume,
        impact_per_volume,
        absorption_score,
        thin_liquidity_score,
        classification: classification.to_string(),
    }
}

fn build_liquidity_depletion(
    buy_pressure: f64,
    sell_pressure: f64,
    pressure_total: f64,
    avg_imbalance: f64,
) -> LiquidityDepletion {
    let buy_share = buy_pressure / pressure_total.max(0.000_001);
    let sell_share = sell_pressure / pressure_total.max(0.000_001);
    let bid_depletion_rate = clamp01(sell_share * (-avg_imbalance).max(0.0) * 2.0);
    let ask_depletion_rate = clamp01(buy_share * avg_imbalance.max(0.0) * 2.0);
    let replenishment_rate = clamp01(avg_imbalance.abs() * 1.4);
    let depletion_pressure =
        clamp01(bid_depletion_rate.max(ask_depletion_rate) - replenishment_rate * 0.5);
    LiquidityDepletion {
        bid_depletion_rate,
        ask_depletion_rate,
        replenishment_rate,
        depletion_pressure,
    }
}

fn build_actor_decomposition(
    ofi_windows: &[OfiWindowMetrics],
    flow_persistence: f64,
    impact: &ImpactResponse,
    depletion: &LiquidityDepletion,
    price_range: f64,
    avg_imbalance: f64,
    activity: f64,
) -> SmartMoneyDecomposition {
    let primary_ofi = ofi_windows
        .iter()
        .find(|window| window.window_sec == 30)
        .or_else(|| ofi_windows.first())
        .map(|window| window.decay_weighted_ofi.abs())
        .unwrap_or_default();
    let volatility_compression = clamp01((0.035 - price_range).max(0.0) / 0.035);
    let replenishment = depletion.replenishment_rate;
    let depletion_pressure = depletion.depletion_pressure;

    let lp_score = clamp01(
        impact.absorption_score * 0.42
            + replenishment * 0.24
            + volatility_compression * 0.18
            + activity * 0.10
            + (1.0 - primary_ofi).max(0.0) * 0.06,
    ) * (1.0 - impact.thin_liquidity_score * 0.18);
    let momentum_score = clamp01(
        impact.thin_liquidity_score * 0.55
            + primary_ofi * 0.24
            + depletion_pressure * 0.16
            + (1.0 - volatility_compression) * 0.15,
    );
    let smart_money_score = clamp01(
        flow_persistence * 0.30
            + impact.absorption_score * 0.24
            + primary_ofi * 0.18
            + volatility_compression * 0.16
            + avg_imbalance.abs().min(1.0) * 0.12,
    ) * (1.0 - impact.thin_liquidity_score * 0.30);

    let total = (lp_score + momentum_score + smart_money_score).max(0.000_001);
    let lp_probability = lp_score / total;
    let momentum_probability = momentum_score / total;
    let smart_money_probability = smart_money_score / total;
    let dominant_actor = dominant_actor(
        lp_probability,
        momentum_probability,
        smart_money_probability,
    );
    let confidence = actor_confidence(
        lp_probability,
        momentum_probability,
        smart_money_probability,
    );
    let explanation_tags = actor_tags(
        dominant_actor,
        impact,
        flow_persistence,
        volatility_compression,
        replenishment,
        depletion_pressure,
    );

    SmartMoneyDecomposition {
        liquidity_provider_probability: lp_probability,
        momentum_chaser_probability: momentum_probability,
        smart_money_probability,
        dominant_actor,
        lp_score,
        momentum_score,
        smart_money_score,
        confidence,
        explanation_tags,
    }
}

fn build_signal_compression(
    net_ratio: f64,
    price_change: f64,
    price_range: f64,
    flow_persistence: f64,
    ofi_windows: &[OfiWindowMetrics],
    impact: &ImpactResponse,
    depletion: &LiquidityDepletion,
    actor: &SmartMoneyDecomposition,
    avg_imbalance: f64,
) -> SignalCompressionState {
    let flow_sign = if net_ratio.abs() < 0.05 {
        0.0
    } else {
        net_ratio.signum()
    };
    let smart_money_pressure = clamp_signed(
        flow_sign
            * clamp01(
                actor.smart_money_probability * 0.38
                    + flow_persistence * 0.26
                    + impact.absorption_score * 0.22
                    + clamp01((0.04 - price_range).max(0.0) / 0.04) * 0.14
                    - clamp01(price_range / 0.08) * 0.28,
            ),
    );

    let price_sign = if price_change.abs() < 0.000_5 {
        0.0
    } else {
        price_change.signum()
    };
    let flow_price_alignment = if flow_sign == 0.0 || price_sign == 0.0 {
        0.0
    } else if flow_sign == price_sign {
        1.0
    } else {
        -1.0
    };
    let continuation = flow_price_alignment
        * clamp01(
            impact.thin_liquidity_score * 0.45
                + actor.momentum_chaser_probability * 0.28
                + flow_persistence * 0.18
                + net_ratio.abs() * 0.09,
        );
    let delta_divergence = if flow_price_alignment < 0.0 {
        net_ratio.abs() * 0.42
    } else {
        0.0
    };
    let absorption_drag = impact.absorption_score * (1.0 - price_range.min(0.04) / 0.04) * 0.35;
    let momentum_flow_exhaustion = clamp_signed(continuation - delta_divergence - absorption_drag);

    let stress = clamp01(
        depletion.depletion_pressure * 0.36
            + impact.thin_liquidity_score * 0.24
            + (1.0 - depletion.replenishment_rate).max(0.0) * 0.16
            + actor.momentum_chaser_probability * 0.14
            + avg_imbalance.abs().min(1.0) * 0.10,
    );
    let stable_liquidity =
        clamp01(depletion.replenishment_rate * 0.36 + impact.absorption_score * 0.24);
    let liquidity_stress_manipulation = clamp_signed(stress - stable_liquidity * 0.55);

    let stable_signals = build_stable_signals(
        smart_money_pressure,
        momentum_flow_exhaustion,
        liquidity_stress_manipulation,
        ofi_windows,
    );
    let smp_stable = stable_signals.smp_stable;
    let mfe_stable = stable_signals.mfe_stable;
    let lsm_stable = stable_signals.lsm_stable;

    let mut tags = Vec::new();
    if smp_stable > 0.35 {
        tags.push("smart_money_accumulation_pressure".to_string());
    } else if smp_stable < -0.35 {
        tags.push("smart_money_distribution_pressure".to_string());
    } else {
        tags.push("smart_money_pressure_neutral".to_string());
    }
    if mfe_stable > 0.35 {
        tags.push("momentum_continuation".to_string());
    } else if mfe_stable < -0.35 {
        tags.push("momentum_exhaustion_or_divergence".to_string());
    }
    if lsm_stable > 0.50 {
        tags.push("liquidity_stress_high".to_string());
    } else if lsm_stable < -0.20 {
        tags.push("stable_liquidity_environment".to_string());
    }
    if stable_signals.stability_score >= 0.62 {
        tags.push("temporal_stability_confirmed".to_string());
    } else {
        tags.push("temporal_stability_waiting".to_string());
    }

    let risk_score = clamp01(
        lsm_stable.max(0.0) * 0.52
            + (-mfe_stable).max(0.0) * 0.28
            + (-smp_stable).max(0.0) * mfe_stable.max(0.0) * 0.20,
    );
    let (trade_permission, position_size_multiplier, reason) =
        if lsm_stable > 0.70 && mfe_stable < -0.50 {
            tags.push("pvg_block_manipulation_risk_too_high".to_string());
            (false, 0.0, "manipulation_risk_too_high")
        } else if smp_stable < -0.25 && mfe_stable > 0.25 {
            tags.push("pvg_block_distribution_against_momentum".to_string());
            (false, 0.0, "distribution_against_momentum")
        } else if risk_score >= 0.72 {
            tags.push("pvg_reduce_high_risk".to_string());
            (true, 0.25, "reduced_size_high_risk")
        } else {
            tags.push("pvg_advisory_allowed".to_string());
            (true, clamp01(1.0 - risk_score), "advisory_allowed")
        };
    let position_validity_gate = PositionValidityGate {
        risk_score,
        trade_permission,
        position_size_multiplier,
        reason: reason.to_string(),
        advisory_only: true,
    };
    let regime_state = build_regime_state(
        smp_stable,
        mfe_stable,
        lsm_stable,
        price_range,
        &stable_signals,
    );
    let stability_kernel = build_trading_stability_kernel(
        smp_stable,
        mfe_stable,
        lsm_stable,
        price_range,
        &regime_state,
        &position_validity_gate,
    );

    SignalCompressionState {
        smart_money_pressure: smp_stable,
        momentum_flow_exhaustion: mfe_stable,
        liquidity_stress_manipulation: lsm_stable,
        stable_signals,
        regime_state,
        position_validity_gate,
        stability_kernel,
        explanation_tags: tags,
        read_only: true,
    }
}

fn build_trading_stability_kernel(
    smp: f64,
    mfe: f64,
    lsm: f64,
    price_range: f64,
    regime_state: &RegimeState,
    gate: &PositionValidityGate,
) -> TradingStabilityKernel {
    let regime = regime_state.current;
    let regime_quality =
        clamp01(regime_quality(regime, smp, mfe, lsm) * 0.55 + regime_state.stability * 0.45);
    let volatility_adjustment = clamp01(1.0 - price_range / 0.08);
    let drawdown_adjustment = 1.0;
    let confidence = clamp01(
        smp.abs() * 0.36 + mfe.abs() * 0.24 + regime_quality * 0.28 + (1.0 - lsm.max(0.0)) * 0.12,
    );

    let (direction, hold_time, invalidation_condition, reason) =
        if !gate.trade_permission || matches!(regime, StabilityRegime::Manipulation) {
            (
                AdvisoryDirection::NoTrade,
                ExpectedHoldTime::None,
                "pvg_or_manipulation_regime_blocks_trade".to_string(),
                "risk_gate_blocks_advisory_trade".to_string(),
            )
        } else if smp > 0.35 && mfe >= -0.20 && lsm < 0.60 {
            (
                AdvisoryDirection::Long,
                if matches!(regime, StabilityRegime::LiquidityExpansion) {
                    ExpectedHoldTime::Mid
                } else {
                    ExpectedHoldTime::Short
                },
                "smp_turns_negative_or_lsm_above_0_65".to_string(),
                "smart_money_pressure_supports_long_bias".to_string(),
            )
        } else if smp < -0.35 && mfe <= 0.20 && lsm < 0.60 {
            (
                AdvisoryDirection::Short,
                ExpectedHoldTime::Mid,
                "smp_turns_positive_or_mfe_above_0_35".to_string(),
                "distribution_pressure_supports_short_bias".to_string(),
            )
        } else if mfe > 0.45 && lsm < 0.45 {
            (
                if smp >= 0.0 {
                    AdvisoryDirection::Long
                } else {
                    AdvisoryDirection::Short
                },
                ExpectedHoldTime::Short,
                "mfe_falls_below_0_15_or_lsm_above_0_55".to_string(),
                "momentum_continuation_window".to_string(),
            )
        } else {
            (
                AdvisoryDirection::NoTrade,
                ExpectedHoldTime::None,
                "wait_for_regime_quality_above_threshold".to_string(),
                "no_clean_structure_window".to_string(),
            )
        };

    let size_multiplier = if matches!(direction, AdvisoryDirection::NoTrade) {
        0.0
    } else {
        clamp01(confidence * regime_quality * volatility_adjustment * drawdown_adjustment)
            * gate.position_size_multiplier
    };

    TradingStabilityKernel {
        regime,
        regime_quality,
        trade_signal: TradeSignalAdvisory {
            direction,
            confidence,
            expected_hold_time: hold_time,
            invalidation_condition,
            reason,
            advisory_only: true,
        },
        position_smoothing: PositionSmoothing {
            suggested_size_multiplier: size_multiplier,
            volatility_adjustment,
            drawdown_adjustment,
            reason: "confidence_x_regime_quality_x_volatility_x_pvg".to_string(),
        },
        read_only: true,
    }
}

fn build_stable_signals(
    smp: f64,
    mfe: f64,
    lsm: f64,
    ofi_windows: &[OfiWindowMetrics],
) -> StableSignals {
    let alpha = 0.30;
    let mut smp_ema = smp;
    let mut mfe_ema = mfe;
    let mut lsm_ema = lsm;
    let mut previous_sign = 0_i8;
    let mut flips = 0_u32;
    let mut persistent = 0_u32;
    let mut persistence_sum = 0.0;

    for window in ofi_windows {
        let direction = sign_bucket(window.decay_weighted_ofi);
        if previous_sign != 0 && direction != 0 && direction != previous_sign {
            flips += 1;
        }
        if direction != 0 {
            previous_sign = direction;
        }
        if direction == sign_bucket(smp) && window.persistence >= 0.25 {
            persistent += 1;
        }
        persistence_sum += window.persistence;

        let smp_observed = clamp_signed(
            window.normalized_ofi * (0.50 + window.persistence * 0.35)
                + window.decay_weighted_ofi * 0.15,
        );
        let mfe_observed =
            clamp_signed(window.decay_weighted_ofi * 0.65 + window.normalized_ofi * 0.35);
        let lsm_observed =
            clamp_signed((1.0 - window.persistence).max(0.0) * window.normalized_ofi.abs() * 0.65);

        smp_ema = ema(smp_ema, smp_observed, alpha);
        mfe_ema = ema(mfe_ema, mfe_observed, alpha);
        lsm_ema = ema(lsm_ema, lsm_observed, alpha);
    }

    let total_windows = ofi_windows.len().max(1) as f64;
    let avg_persistence = persistence_sum / total_windows;
    let persistence_ratio = persistent as f64 / total_windows;
    let flip_penalty = flips as f64 / total_windows;
    let stability_score =
        clamp01(avg_persistence * 0.45 + persistence_ratio * 0.40 + (1.0 - flip_penalty) * 0.15);
    let stability_weight = 0.55 + stability_score * 0.45;

    StableSignals {
        smp_stable: clamp_signed(smp_ema * stability_weight),
        mfe_stable: clamp_signed(mfe_ema * stability_weight),
        lsm_stable: clamp_signed(lsm_ema * (0.70 + (1.0 - flip_penalty) * 0.30)),
        stability_score,
        persistence_windows: persistent,
        flip_penalty,
    }
}

fn build_regime_state(
    smp: f64,
    mfe: f64,
    lsm: f64,
    price_range: f64,
    stable: &StableSignals,
) -> RegimeState {
    let raw = classify_stability_regime(smp, mfe, lsm, price_range);
    let raw_quality = regime_quality(raw, smp, mfe, lsm);
    let inertia = stable.stability_score;
    let confidence = clamp01(raw_quality * 0.62 + inertia * 0.38);
    let transition_risk = if stable.flip_penalty >= 0.50 || inertia < 0.35 {
        "high"
    } else if stable.flip_penalty >= 0.25 || inertia < 0.55 {
        "medium"
    } else {
        "low"
    };

    RegimeState {
        current: raw,
        confidence,
        stability: inertia,
        transition_risk: transition_risk.to_string(),
    }
}

fn ema(previous: f64, observed: f64, alpha: f64) -> f64 {
    previous * (1.0 - alpha) + observed * alpha
}

fn sign_bucket(value: f64) -> i8 {
    if value > 0.08 {
        1
    } else if value < -0.08 {
        -1
    } else {
        0
    }
}

fn build_capital_structure(
    ticks: &[ContractTick],
    net_ratio: f64,
    price_change: f64,
    price_range: f64,
    flow_persistence: f64,
    impact: &ImpactResponse,
    depletion: &LiquidityDepletion,
    compression: &SignalCompressionState,
) -> CapitalStructureView {
    let behavior_windows = build_behavior_windows(ticks);
    let distribution_risk = build_distribution_risk(
        net_ratio,
        price_change,
        price_range,
        flow_persistence,
        impact,
        depletion,
        compression,
        &behavior_windows,
    );
    let (phase, phase_confidence) = classify_capital_phase(
        net_ratio,
        price_change,
        price_range,
        flow_persistence,
        impact,
        depletion,
        compression,
        distribution_risk.score,
        &behavior_windows,
    );
    let cost_basis = estimate_cost_basis(ticks, &behavior_windows, phase_confidence);
    let estimated_position = estimate_position_size(
        ticks,
        &behavior_windows,
        &cost_basis,
        phase,
        phase_confidence,
    );
    let horizon = infer_time_horizon(ticks, phase, phase_confidence, &behavior_windows);

    let mut evidence = vec![
        format!("phase={}", phase_label(phase)),
        format!("phase_confidence={:.2}", phase_confidence),
        format!("cost_vwap={:.6}", cost_basis.vwap_anchor),
        format!("distribution_risk={:.2}", distribution_risk.score),
    ];
    if matches!(phase, CapitalPhase::Accumulation) {
        evidence.push("sustained_buy_pressure_with_low_price_drift".to_string());
    }
    if matches!(phase, CapitalPhase::Distribution) {
        evidence.push("volume_expansion_with_delta_divergence".to_string());
    }
    if matches!(phase, CapitalPhase::Markup) {
        evidence.push("directional_flow_with_price_expansion".to_string());
    }
    if matches!(phase, CapitalPhase::Breakdown) {
        evidence.push("sell_pressure_with_downside_structure".to_string());
    }

    CapitalStructureView {
        phase,
        phase_label: phase_label(phase).to_string(),
        phase_confidence,
        behavior_windows,
        cost_basis,
        estimated_position,
        horizon,
        distribution_risk,
        evidence,
        read_only: true,
    }
}

fn build_behavior_windows(ticks: &[ContractTick]) -> Vec<BehaviorWindowMetrics> {
    [60_u64, 300, 900, 3600, 14_400]
        .into_iter()
        .map(|window_sec| build_behavior_window(ticks, window_sec))
        .collect()
}

fn build_behavior_window(ticks: &[ContractTick], window_sec: u64) -> BehaviorWindowMetrics {
    let latest_ts = ticks
        .iter()
        .map(|tick| tick.timestamp)
        .max()
        .unwrap_or_default();
    let window_ms = window_sec.saturating_mul(1000);
    let scoped = ticks
        .iter()
        .filter(|tick| {
            latest_ts < window_ms || tick.timestamp >= latest_ts.saturating_sub(window_ms)
        })
        .collect::<Vec<_>>();
    let selected = if scoped.is_empty() {
        ticks.iter().collect::<Vec<_>>()
    } else {
        scoped
    };

    let mut buy_pressure = 0.0;
    let mut sell_pressure = 0.0;
    let mut volume = 0.0;
    let mut price_volume = 0.0;
    let mut imbalance_sum = 0.0;
    let mut min_price = f64::MAX;
    let mut max_price = 0.0_f64;

    for tick in selected.iter().copied() {
        let size = tick.size.max(0.0);
        let pressure = size * tick.aggression.clamp(0.0, 1.0);
        match tick.side {
            ContractTickSide::Buy => buy_pressure += pressure,
            ContractTickSide::Sell => sell_pressure += pressure,
        }
        volume += size;
        price_volume += tick.price.max(0.0) * size;
        imbalance_sum += tick.orderbook_imbalance.clamp(-1.0, 1.0);
        min_price = min_price.min(tick.price);
        max_price = max_price.max(tick.price);
    }

    let first_price = selected.first().map(|tick| tick.price).unwrap_or_default();
    let last_price = selected.last().map(|tick| tick.price).unwrap_or_default();
    let pressure_total = (buy_pressure + sell_pressure).max(0.000_001);
    let cumulative_delta = buy_pressure - sell_pressure;
    let normalized_ofi = cumulative_delta / pressure_total;
    let vwap = price_volume / volume.max(0.000_001);
    let price_drift_pct = if first_price > 0.0 {
        (last_price - first_price) / first_price
    } else {
        0.0
    };
    let volatility_pct = if first_price > 0.0 {
        (max_price - min_price).abs() / first_price
    } else {
        0.0
    };
    let avg_imbalance = imbalance_sum / selected.len().max(1) as f64;
    let volume_intensity = clamp01(volume / 240.0);
    let low_drift = clamp01((0.012 - price_drift_pct.abs()).max(0.0) / 0.012);
    let low_volatility = clamp01((0.045 - volatility_pct).max(0.0) / 0.045);
    let absorption_score =
        clamp01(volume_intensity * 0.36 + low_drift * 0.34 + low_volatility * 0.30);
    let bid_replenishment_score =
        clamp01(avg_imbalance.max(0.0) * 1.7 + low_volatility * 0.28 + absorption_score * 0.18);

    BehaviorWindowMetrics {
        window_sec,
        cumulative_delta,
        normalized_ofi,
        vwap,
        volume,
        price_drift_pct,
        volatility_pct,
        absorption_score,
        bid_replenishment_score,
    }
}

fn estimate_cost_basis(
    ticks: &[ContractTick],
    windows: &[BehaviorWindowMetrics],
    phase_confidence: f64,
) -> CostBasisEstimate {
    let selected = windows
        .iter()
        .filter(|window| {
            window.normalized_ofi > 0.12
                && window.volatility_pct < 0.05
                && window.absorption_score > 0.40
        })
        .collect::<Vec<_>>();

    let mut density_peak = 0.0;
    let mut best_density = -1.0_f64;
    for window in windows {
        let density = window.volume.max(0.0)
            * (0.35 + window.absorption_score * 0.30)
            * (0.35 + window.normalized_ofi.max(0.0) * 0.25)
            * (1.0 - window.volatility_pct.min(0.08) / 0.08).max(0.05);
        if density > best_density && window.vwap > 0.0 {
            best_density = density;
            density_peak = window.vwap;
        }
    }

    let (weighted_vwap, weighted_volume, avg_volatility) = if selected.is_empty() {
        let mut volume = 0.0;
        let mut price_volume = 0.0;
        let mut volatility_sum = 0.0;
        for window in windows {
            volume += window.volume.max(0.0);
            price_volume += window.vwap * window.volume.max(0.0);
            volatility_sum += window.volatility_pct;
        }
        (
            price_volume / volume.max(0.000_001),
            volume,
            volatility_sum / windows.len().max(1) as f64,
        )
    } else {
        let mut volume = 0.0;
        let mut price_volume = 0.0;
        let mut volatility_sum = 0.0;
        for window in selected {
            let weight = window.volume.max(0.0)
                * (0.50 + window.absorption_score * 0.30 + window.normalized_ofi.max(0.0) * 0.20);
            volume += weight;
            price_volume += window.vwap * weight;
            volatility_sum += window.volatility_pct;
        }
        (
            price_volume / volume.max(0.000_001),
            volume,
            volatility_sum / windows.len().max(1) as f64,
        )
    };

    let fallback_price = ticks.last().map(|tick| tick.price).unwrap_or_default();
    let anchor = if weighted_vwap > 0.0 {
        weighted_vwap
    } else {
        fallback_price
    };
    let density_peak = if density_peak > 0.0 {
        density_peak
    } else {
        anchor
    };
    let mut min_price = f64::MAX;
    let mut max_price = 0.0_f64;
    for tick in ticks {
        min_price = min_price.min(tick.price);
        max_price = max_price.max(tick.price);
    }
    let density_quality = (best_density / 500.0).clamp(0.0, 1.0);
    let band_pct = (avg_volatility * (1.0 - density_quality * 0.35)).clamp(0.0012, 0.022);
    let lower = (anchor * (1.0 - band_pct)).min(min_price);
    let upper = (anchor * (1.0 + band_pct)).max(max_price);
    CostBasisEstimate {
        lower,
        upper,
        vwap_anchor: anchor,
        density_peak,
        confidence: clamp01(
            phase_confidence * 0.62 + (weighted_volume / 500.0).min(0.22) + density_quality * 0.16,
        ),
    }
}

fn estimate_position_size(
    ticks: &[ContractTick],
    windows: &[BehaviorWindowMetrics],
    cost_basis: &CostBasisEstimate,
    phase: CapitalPhase,
    phase_confidence: f64,
) -> EstimatedPositionSize {
    let latest_price = ticks
        .last()
        .map(|tick| tick.price)
        .unwrap_or(cost_basis.vwap_anchor);
    let structural_volume = windows
        .iter()
        .filter(|window| match phase {
            CapitalPhase::Accumulation | CapitalPhase::Markup => window.normalized_ofi > 0.05,
            CapitalPhase::Distribution | CapitalPhase::Breakdown => window.normalized_ofi < -0.05,
            CapitalPhase::Neutral => true,
        })
        .map(|window| window.volume.max(0.0) * (0.35 + window.normalized_ofi.abs().min(1.0) * 0.65))
        .sum::<f64>();
    let notional = structural_volume * latest_price.max(cost_basis.vwap_anchor).max(0.0);
    let pressure = match phase {
        CapitalPhase::Neutral => phase_confidence * 0.45,
        _ => phase_confidence,
    };
    EstimatedPositionSize {
        lower_usd: notional * pressure * 0.28,
        upper_usd: notional * pressure * 0.82,
        confidence: clamp01(phase_confidence * 0.80 + structural_volume.min(250.0) / 1250.0),
    }
}

fn infer_time_horizon(
    ticks: &[ContractTick],
    phase: CapitalPhase,
    phase_confidence: f64,
    windows: &[BehaviorWindowMetrics],
) -> TimeHorizonInference {
    let first_ts = ticks
        .iter()
        .map(|tick| tick.timestamp)
        .min()
        .unwrap_or_default();
    let last_ts = ticks
        .iter()
        .map(|tick| tick.timestamp)
        .max()
        .unwrap_or_default();
    let raw_span = last_ts.saturating_sub(first_ts);
    let detected_minutes = if raw_span >= 1000 {
        raw_span as f64 / 60_000.0
    } else {
        ticks.len() as f64 * 5.0 / 60.0
    };
    let confirmed_windows = windows
        .iter()
        .filter(|window| match phase {
            CapitalPhase::Accumulation => {
                window.normalized_ofi > 0.10 && window.absorption_score > 0.35
            }
            CapitalPhase::Markup => window.normalized_ofi > 0.18 && window.price_drift_pct > 0.0,
            CapitalPhase::Distribution => window.normalized_ofi < -0.10,
            CapitalPhase::Breakdown => {
                window.normalized_ofi < -0.10 && window.price_drift_pct < 0.0
            }
            CapitalPhase::Neutral => false,
        })
        .count() as f64;
    let stability_multiplier = 1.0 + confirmed_windows * 0.45 + phase_confidence * 0.80;
    TimeHorizonInference {
        min_minutes: (detected_minutes * 0.75).max(if matches!(phase, CapitalPhase::Neutral) {
            0.0
        } else {
            1.0
        }),
        max_minutes: (detected_minutes * stability_multiplier + 8.0 * confirmed_windows)
            .max(detected_minutes),
        detected_minutes,
    }
}

fn build_distribution_risk(
    net_ratio: f64,
    price_change: f64,
    price_range: f64,
    flow_persistence: f64,
    impact: &ImpactResponse,
    depletion: &LiquidityDepletion,
    compression: &SignalCompressionState,
    windows: &[BehaviorWindowMetrics],
) -> DistributionRisk {
    let primary_window = windows
        .iter()
        .find(|window| window.window_sec == 300)
        .or_else(|| windows.first());
    let window_ofi = primary_window
        .map(|window| window.normalized_ofi)
        .unwrap_or(net_ratio);
    let sell_pressure = (-net_ratio).max(0.0).max((-window_ofi).max(0.0));
    let price_fail = if price_change >= -0.002 {
        clamp01((price_range + impact.absorption_score * 0.01) / 0.04)
    } else {
        clamp01(price_change.abs() / 0.08)
    };
    let delta_divergence = if price_change > 0.0 && net_ratio < 0.0 {
        net_ratio.abs()
    } else {
        0.0
    };
    let score = clamp01(
        sell_pressure * 0.32
            + depletion.bid_depletion_rate * 0.20
            + compression.liquidity_stress_manipulation.max(0.0) * 0.18
            + price_fail * 0.12
            + delta_divergence * 0.10
            + flow_persistence * sell_pressure * 0.08,
    );
    let level = if score >= 0.66 {
        "high"
    } else if score >= 0.33 {
        "medium"
    } else {
        "low"
    };
    let mut reasons = Vec::new();
    if sell_pressure > 0.20 {
        reasons.push("sell_pressure_persistent".to_string());
    }
    if depletion.bid_depletion_rate > depletion.replenishment_rate {
        reasons.push("bid_depletion_exceeds_replenishment".to_string());
    }
    if delta_divergence > 0.10 {
        reasons.push("price_up_delta_down_divergence".to_string());
    }
    if reasons.is_empty() {
        reasons.push("no_distribution_pressure_confirmed".to_string());
    }
    DistributionRisk {
        score,
        level: level.to_string(),
        reasons,
    }
}

fn classify_capital_phase(
    net_ratio: f64,
    price_change: f64,
    price_range: f64,
    flow_persistence: f64,
    impact: &ImpactResponse,
    depletion: &LiquidityDepletion,
    compression: &SignalCompressionState,
    distribution_risk: f64,
    windows: &[BehaviorWindowMetrics],
) -> (CapitalPhase, f64) {
    let w5m = windows
        .iter()
        .find(|window| window.window_sec == 300)
        .or_else(|| windows.first());
    let w15m = windows
        .iter()
        .find(|window| window.window_sec == 900)
        .or(w5m);
    let w5m_ofi = w5m.map(|window| window.normalized_ofi).unwrap_or(net_ratio);
    let w15m_ofi = w15m
        .map(|window| window.normalized_ofi)
        .unwrap_or(net_ratio);
    let w5m_absorption = w5m
        .map(|window| window.absorption_score)
        .unwrap_or(impact.absorption_score);
    let w5m_bid_replenishment = w5m
        .map(|window| window.bid_replenishment_score)
        .unwrap_or(depletion.replenishment_rate);
    let low_drift = clamp01((0.018 - price_change.abs()).max(0.0) / 0.018);
    let low_range = clamp01((0.05 - price_range).max(0.0) / 0.05);

    let accumulation_score = clamp01(
        w5m_ofi.max(0.0) * 0.25
            + w15m_ofi.max(0.0) * 0.15
            + w5m_absorption * 0.20
            + w5m_bid_replenishment * 0.16
            + low_drift * 0.14
            + flow_persistence * 0.10,
    );
    let markup_score = clamp01(
        net_ratio.max(0.0) * 0.24
            + price_change.max(0.0).min(0.08) * 4.0 * 0.24
            + compression.momentum_flow_exhaustion.max(0.0) * 0.20
            + impact.thin_liquidity_score * 0.18
            + flow_persistence * 0.14,
    );
    let distribution_score = clamp01(
        distribution_risk * 0.45
            + (-w5m_ofi).max(0.0) * 0.20
            + impact.absorption_score * (-net_ratio).max(0.0) * 0.14
            + low_range * 0.10
            + compression.smart_money_pressure.min(0.0).abs() * 0.11,
    );
    let breakdown_score = clamp01(
        (-net_ratio).max(0.0) * 0.24
            + (-price_change).max(0.0).min(0.08) * 4.0 * 0.24
            + depletion.bid_depletion_rate * 0.20
            + compression.liquidity_stress_manipulation.max(0.0) * 0.16
            + flow_persistence * 0.16,
    );

    let mut ranked = [
        (CapitalPhase::Accumulation, accumulation_score),
        (CapitalPhase::Markup, markup_score),
        (CapitalPhase::Distribution, distribution_score),
        (CapitalPhase::Breakdown, breakdown_score),
        (CapitalPhase::Neutral, 0.24 + low_range * 0.12),
    ];
    ranked.sort_by(|left, right| right.1.total_cmp(&left.1));
    let (phase, score) = ranked[0];
    if score < 0.36 {
        (CapitalPhase::Neutral, clamp01(score))
    } else {
        (phase, clamp01(score))
    }
}

fn phase_label(phase: CapitalPhase) -> &'static str {
    match phase {
        CapitalPhase::Accumulation => "accumulation",
        CapitalPhase::Markup => "markup",
        CapitalPhase::Distribution => "distribution",
        CapitalPhase::Breakdown => "breakdown",
        CapitalPhase::Neutral => "neutral",
    }
}

fn build_position_reconstruction(
    ticks: &[ContractTick],
    capital: &CapitalStructureView,
    impact: &ImpactResponse,
    depletion: &LiquidityDepletion,
    compression: &SignalCompressionState,
) -> SmartMoneyPositionReconstruction {
    if ticks.is_empty() {
        return SmartMoneyPositionReconstruction::default();
    }

    let segments = build_position_segments(ticks);
    let accumulation_path = segments
        .iter()
        .filter(|segment| {
            matches!(
                segment.phase,
                CapitalPhase::Accumulation | CapitalPhase::Markup
            )
        })
        .cloned()
        .collect::<Vec<_>>();
    let distribution_path = segments
        .iter()
        .filter(|segment| {
            matches!(
                segment.phase,
                CapitalPhase::Distribution | CapitalPhase::Breakdown
            )
        })
        .cloned()
        .collect::<Vec<_>>();
    let last_accumulation_node =
        select_last_accumulation_node(&segments, capital, impact, compression);
    let latent_position = build_latent_position(ticks, impact);

    let accumulation_confidence = accumulation_path
        .iter()
        .map(|segment| segment.confidence)
        .fold(0.0_f64, f64::max);
    let distribution_confidence = distribution_path
        .iter()
        .map(|segment| segment.confidence)
        .fold(0.0_f64, f64::max);
    let node_confidence = last_accumulation_node
        .as_ref()
        .map(|node| node.confidence)
        .unwrap_or_default();
    let confidence = clamp01(
        capital.phase_confidence * 0.34
            + accumulation_confidence * 0.22
            + distribution_confidence * 0.18
            + node_confidence * 0.16
            + compression.stability_kernel.regime_quality * 0.10,
    );

    let regime_label = if distribution_confidence > accumulation_confidence + 0.10 {
        "distribution_trajectory"
    } else if accumulation_confidence > 0.45 {
        "accumulation_trajectory"
    } else if matches!(capital.phase, CapitalPhase::Markup) {
        "markup_after_accumulation"
    } else {
        "neutral_reconstruction"
    };
    let mut evidence = vec![
        format!("segments={}", segments.len()),
        format!("accumulation_segments={}", accumulation_path.len()),
        format!("distribution_segments={}", distribution_path.len()),
        format!("latent_points={}", latent_position.len()),
    ];
    if last_accumulation_node.is_some() {
        evidence.push("last_accumulation_node_detected".to_string());
    }
    if depletion.bid_depletion_rate > depletion.replenishment_rate {
        evidence.push("distribution_path_bid_depletion".to_string());
    }
    if impact.absorption_score > 0.60 {
        evidence.push("low_impact_absorption_supports_reconstruction".to_string());
    }

    SmartMoneyPositionReconstruction {
        accumulation_path,
        last_accumulation_node,
        distribution_path,
        latent_position,
        confidence,
        regime_label: regime_label.to_string(),
        evidence,
        read_only: true,
    }
}

fn build_position_segments(ticks: &[ContractTick]) -> Vec<PositionPathSegment> {
    let segment_count = ticks.len().clamp(1, 3);
    let chunk_size = ticks.len().div_ceil(segment_count);
    ticks
        .chunks(chunk_size)
        .enumerate()
        .map(|(idx, chunk)| build_position_segment(chunk, idx))
        .collect()
}

fn build_position_segment(ticks: &[ContractTick], idx: usize) -> PositionPathSegment {
    let start_price = ticks.first().map(|tick| tick.price).unwrap_or_default();
    let end_price = ticks.last().map(|tick| tick.price).unwrap_or_default();
    let first_ts = ticks.first().map(|tick| tick.timestamp).unwrap_or_default();
    let last_ts = ticks.last().map(|tick| tick.timestamp).unwrap_or(first_ts);
    let duration_ms_or_ticks = last_ts.saturating_sub(first_ts);
    let duration_sec = if duration_ms_or_ticks >= 1000 {
        duration_ms_or_ticks / 1000
    } else {
        ticks.len() as u64 * 5
    };

    let mut buy_pressure = 0.0;
    let mut sell_pressure = 0.0;
    let mut volume = 0.0;
    let mut min_price = f64::MAX;
    let mut max_price = 0.0_f64;
    let mut imbalance_sum = 0.0;
    for tick in ticks {
        let size = tick.size.max(0.0);
        let pressure = size * tick.aggression.clamp(0.0, 1.0);
        match tick.side {
            ContractTickSide::Buy => buy_pressure += pressure,
            ContractTickSide::Sell => sell_pressure += pressure,
        }
        volume += size;
        min_price = min_price.min(tick.price);
        max_price = max_price.max(tick.price);
        imbalance_sum += tick.orderbook_imbalance.clamp(-1.0, 1.0);
    }

    let pressure_total = (buy_pressure + sell_pressure).max(0.000_001);
    let cumulative_delta = buy_pressure - sell_pressure;
    let normalized_delta = cumulative_delta / pressure_total;
    let price_drift = if start_price > 0.0 {
        (end_price - start_price) / start_price
    } else {
        0.0
    };
    let volatility = if start_price > 0.0 {
        (max_price - min_price).abs() / start_price
    } else {
        0.0
    };
    let impact_value = price_drift.abs() / volume.max(0.000_001);
    let avg_imbalance = imbalance_sum / ticks.len().max(1) as f64;
    let low_volatility = clamp01((0.035 - volatility).max(0.0) / 0.035);
    let low_impact = clamp01((0.000_8 - impact_value).max(0.0) / 0.000_8);
    let phase = if normalized_delta > 0.16 && price_drift > 0.006 {
        CapitalPhase::Markup
    } else if normalized_delta > 0.16 && low_volatility > 0.35 && avg_imbalance > -0.05 {
        CapitalPhase::Accumulation
    } else if normalized_delta < -0.16 && price_drift > -0.010 {
        CapitalPhase::Distribution
    } else if normalized_delta < -0.16 && price_drift <= -0.010 {
        CapitalPhase::Breakdown
    } else {
        CapitalPhase::Neutral
    };
    let label = segment_label(phase, idx);
    let mut characteristics = Vec::new();
    if low_impact > 0.55 && volume > 0.0 {
        characteristics.push("minimal_impact_flow".to_string());
    }
    if low_volatility > 0.55 {
        characteristics.push("volatility_compression".to_string());
    }
    if normalized_delta > 0.20 {
        characteristics.push("positive_delta".to_string());
    } else if normalized_delta < -0.20 {
        characteristics.push("negative_delta".to_string());
    }
    if avg_imbalance > 0.10 {
        characteristics.push("bid_replenishment".to_string());
    } else if avg_imbalance < -0.10 {
        characteristics.push("bid_liquidity_depletion".to_string());
    }
    if characteristics.is_empty() {
        characteristics.push("mixed_flow".to_string());
    }

    PositionPathSegment {
        phase,
        label: label.to_string(),
        start_price,
        end_price,
        volume,
        cumulative_delta,
        impact: impact_value,
        duration_sec,
        confidence: clamp01(
            normalized_delta.abs() * 0.30
                + low_volatility * 0.22
                + low_impact * 0.18
                + avg_imbalance.abs().min(1.0) * 0.12
                + (volume / 250.0).min(0.18),
        ),
        characteristics,
    }
}

fn select_last_accumulation_node(
    segments: &[PositionPathSegment],
    capital: &CapitalStructureView,
    impact: &ImpactResponse,
    compression: &SignalCompressionState,
) -> Option<LastAccumulationNode> {
    let candidate = segments
        .iter()
        .filter(|segment| matches!(segment.phase, CapitalPhase::Accumulation))
        .max_by(|left, right| {
            let left_score = left.confidence + (1.0 - left.impact.min(1.0)) * 0.20;
            let right_score = right.confidence + (1.0 - right.impact.min(1.0)) * 0.20;
            left_score.total_cmp(&right_score)
        });

    let segment = candidate.or_else(|| {
        if matches!(
            capital.phase,
            CapitalPhase::Accumulation | CapitalPhase::Markup
        ) {
            segments
                .iter()
                .max_by(|left, right| left.confidence.total_cmp(&right.confidence))
        } else {
            None
        }
    })?;

    let lower = segment
        .start_price
        .min(segment.end_price)
        .min(capital.cost_basis.lower);
    let upper = segment
        .start_price
        .max(segment.end_price)
        .max(capital.cost_basis.upper);
    let absorption_efficiency = clamp01(
        impact.absorption_score * 0.42
            + capital.cost_basis.confidence * 0.22
            + compression.smart_money_pressure.max(0.0) * 0.20
            + segment.confidence * 0.16,
    );
    let mut characteristics = vec![
        "lowest_volatility_or_highest_absorption_window".to_string(),
        "volume_without_breakout".to_string(),
    ];
    if compression.smart_money_pressure > 0.30 {
        characteristics.push("smart_money_pressure_positive".to_string());
    }
    Some(LastAccumulationNode {
        lower,
        upper,
        duration_sec: segment.duration_sec,
        volatility_pct: (segment.end_price - segment.start_price).abs()
            / segment.start_price.max(0.000_001),
        absorption_efficiency,
        confidence: clamp01(absorption_efficiency * 0.70 + segment.confidence * 0.30),
        characteristics,
    })
}

fn build_latent_position(
    ticks: &[ContractTick],
    impact: &ImpactResponse,
) -> Vec<LatentPositionPoint> {
    let mut position = 0.0;
    let impact_decay = clamp01(1.0 - impact.thin_liquidity_score * 0.45);
    ticks
        .iter()
        .map(|tick| {
            let signed = match tick.side {
                ContractTickSide::Buy => tick.size.max(0.0),
                ContractTickSide::Sell => -tick.size.max(0.0),
            };
            position += signed * tick.aggression.clamp(0.0, 1.0);
            LatentPositionPoint {
                timestamp: tick.timestamp,
                price: tick.price,
                estimated_position: position,
                impact_adjusted_position: position * impact_decay,
            }
        })
        .collect()
}

fn segment_label(phase: CapitalPhase, idx: usize) -> &'static str {
    match phase {
        CapitalPhase::Accumulation => match idx {
            0 => "silent_accumulation",
            1 => "absorption_zone",
            _ => "final_accumulation",
        },
        CapitalPhase::Markup => "markup_expansion",
        CapitalPhase::Distribution => match idx {
            0 => "hidden_distribution",
            1 => "retail_absorption",
            _ => "exit_preparation",
        },
        CapitalPhase::Breakdown => "exit_acceleration",
        CapitalPhase::Neutral => "neutral_segment",
    }
}

fn classify_stability_regime(smp: f64, mfe: f64, lsm: f64, price_range: f64) -> StabilityRegime {
    if lsm > 0.70 && mfe < -0.35 {
        StabilityRegime::Manipulation
    } else if lsm > 0.50 {
        StabilityRegime::LiquidityStress
    } else if mfe > 0.35 && price_range >= 0.015 {
        StabilityRegime::Trend
    } else if smp.abs() > 0.40 && lsm < 0.45 && smp.abs() >= mfe.abs() * 0.75 {
        StabilityRegime::LiquidityExpansion
    } else if mfe > 0.35 {
        StabilityRegime::Trend
    } else if smp.abs() < 0.25 && mfe.abs() < 0.25 {
        StabilityRegime::Chop
    } else {
        StabilityRegime::Neutral
    }
}

fn regime_quality(regime: StabilityRegime, smp: f64, mfe: f64, lsm: f64) -> f64 {
    match regime {
        StabilityRegime::Trend => clamp01(0.45 + mfe.max(0.0) * 0.45 - lsm.max(0.0) * 0.20),
        StabilityRegime::LiquidityExpansion => {
            clamp01(0.42 + smp.abs() * 0.45 - lsm.max(0.0) * 0.18)
        }
        StabilityRegime::LiquidityStress => {
            clamp01(0.35 + lsm.max(0.0) * 0.35 - mfe.max(0.0) * 0.10)
        }
        StabilityRegime::Manipulation => clamp01(0.20 + lsm.max(0.0) * 0.25),
        StabilityRegime::Chop => 0.35,
        StabilityRegime::Neutral => clamp01(0.30 + smp.abs() * 0.12 + mfe.abs() * 0.12),
    }
}

fn dominant_actor(lp: f64, momentum: f64, smart: f64) -> FlowActorRegime {
    let mut ranked = [
        (FlowActorRegime::LiquidityProvider, lp),
        (FlowActorRegime::MomentumChaser, momentum),
        (FlowActorRegime::SmartMoney, smart),
    ];
    ranked.sort_by(|left, right| right.1.total_cmp(&left.1));
    if ranked[0].1 - ranked[1].1 < 0.06 {
        FlowActorRegime::Mixed
    } else {
        ranked[0].0
    }
}

fn actor_confidence(lp: f64, momentum: f64, smart: f64) -> f64 {
    let mut values = [lp, momentum, smart];
    values.sort_by(|left, right| right.total_cmp(left));
    clamp01(0.45 + (values[0] - values[1]) * 1.4)
}

fn actor_tags(
    actor: FlowActorRegime,
    impact: &ImpactResponse,
    flow_persistence: f64,
    volatility_compression: f64,
    replenishment: f64,
    depletion_pressure: f64,
) -> Vec<String> {
    let mut tags = Vec::new();
    match actor {
        FlowActorRegime::LiquidityProvider => tags.push("passive_liquidity_absorption".to_string()),
        FlowActorRegime::MomentumChaser => tags.push("directional_taker_pressure".to_string()),
        FlowActorRegime::SmartMoney => tags.push("persistent_stealth_accumulation".to_string()),
        FlowActorRegime::Mixed => tags.push("mixed_actor_flow".to_string()),
        FlowActorRegime::Unknown => tags.push("unknown_actor_flow".to_string()),
    }
    if impact.absorption_score > 0.65 {
        tags.push("low_impact_high_volume".to_string());
    }
    if impact.thin_liquidity_score > 0.65 {
        tags.push("high_impact_thin_book".to_string());
    }
    if flow_persistence > 0.70 {
        tags.push("persistent_ofi".to_string());
    }
    if volatility_compression > 0.65 {
        tags.push("volatility_compression".to_string());
    }
    if replenishment > depletion_pressure {
        tags.push("replenishment_dominates_depletion".to_string());
    }
    tags
}
