use super::types::{
    AdvisoryDirection, ContractTick, ContractTickSide, ExpectedHoldTime, FlowActorRegime,
    ImpactResponse, LiquidityDepletion, OfiWindowMetrics, PositionSmoothing, PositionValidityGate,
    SignalCompressionState, SmartMoneyDecomposition, StabilityRegime, TokenFlowRegime,
    TokenFlowSignal, TradeSignalAdvisory, TradingStabilityKernel,
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
            &impact_response,
            &liquidity_depletion,
            &actor_decomposition,
            avg_imbalance,
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

    let mut tags = Vec::new();
    if smart_money_pressure > 0.35 {
        tags.push("smart_money_accumulation_pressure".to_string());
    } else if smart_money_pressure < -0.35 {
        tags.push("smart_money_distribution_pressure".to_string());
    } else {
        tags.push("smart_money_pressure_neutral".to_string());
    }
    if momentum_flow_exhaustion > 0.35 {
        tags.push("momentum_continuation".to_string());
    } else if momentum_flow_exhaustion < -0.35 {
        tags.push("momentum_exhaustion_or_divergence".to_string());
    }
    if liquidity_stress_manipulation > 0.50 {
        tags.push("liquidity_stress_high".to_string());
    } else if liquidity_stress_manipulation < -0.20 {
        tags.push("stable_liquidity_environment".to_string());
    }

    let risk_score = clamp01(
        liquidity_stress_manipulation.max(0.0) * 0.52
            + (-momentum_flow_exhaustion).max(0.0) * 0.28
            + (-smart_money_pressure).max(0.0) * momentum_flow_exhaustion.max(0.0) * 0.20,
    );
    let (trade_permission, position_size_multiplier, reason) =
        if liquidity_stress_manipulation > 0.70 && momentum_flow_exhaustion < -0.50 {
            tags.push("pvg_block_manipulation_risk_too_high".to_string());
            (false, 0.0, "manipulation_risk_too_high")
        } else if smart_money_pressure < -0.25 && momentum_flow_exhaustion > 0.25 {
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
    let stability_kernel = build_trading_stability_kernel(
        smart_money_pressure,
        momentum_flow_exhaustion,
        liquidity_stress_manipulation,
        price_range,
        &position_validity_gate,
    );

    SignalCompressionState {
        smart_money_pressure,
        momentum_flow_exhaustion,
        liquidity_stress_manipulation,
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
    gate: &PositionValidityGate,
) -> TradingStabilityKernel {
    let regime = classify_stability_regime(smp, mfe, lsm);
    let regime_quality = regime_quality(regime, smp, mfe, lsm);
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

fn classify_stability_regime(smp: f64, mfe: f64, lsm: f64) -> StabilityRegime {
    if lsm > 0.70 && mfe < -0.35 {
        StabilityRegime::Manipulation
    } else if lsm > 0.50 {
        StabilityRegime::LiquidityStress
    } else if mfe > 0.35 {
        StabilityRegime::Trend
    } else if smp.abs() > 0.40 && lsm < 0.45 {
        StabilityRegime::LiquidityExpansion
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
