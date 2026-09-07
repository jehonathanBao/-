use std::collections::{BTreeMap, BTreeSet};

use super::types::{
    ContractWhaleAction, ContractWhaleSignal, ContractWhaleSignalType, ContractWhaleStealthProfile,
    ContractWhaleTrajectory,
};

pub fn apply_contract_whale_trajectories(signals: &mut [ContractWhaleSignal]) {
    if signals.is_empty() {
        return;
    }

    let actions = signals
        .iter()
        .map(signal_to_action)
        .collect::<Vec<ContractWhaleAction>>();
    for (signal, action) in signals.iter_mut().zip(actions.iter()) {
        signal.whale_action = action.clone();
    }

    let mut grouped_indices: BTreeMap<String, Vec<usize>> = BTreeMap::new();
    for (index, signal) in signals.iter().enumerate() {
        let key = if signal.cluster.cluster_id.trim().is_empty() {
            format!("single:{}:{}", signal.symbol, signal.id)
        } else {
            signal.cluster.cluster_id.clone()
        };
        grouped_indices.entry(key).or_default().push(index);
    }

    for (cluster_id, mut indices) in grouped_indices {
        indices.sort_by_key(|index| signals[*index].ts);
        let trajectory_actions = indices
            .iter()
            .map(|index| signals[*index].whale_action.clone())
            .collect::<Vec<_>>();
        let trajectory = reconstruct_trajectory(&cluster_id, trajectory_actions);
        for index in indices {
            signals[index].trajectory = trajectory.clone();
        }
    }
}

fn signal_to_action(signal: &ContractWhaleSignal) -> ContractWhaleAction {
    ContractWhaleAction {
        ts: signal.ts,
        symbol: signal.symbol.clone(),
        action_type: action_type(signal).to_string(),
        volume: round(
            signal
                .net_volume_btc
                .abs()
                .max(signal.total_volume_btc * signal.dominance),
            4,
        ),
        price_impact: round(signal.price_move_pct.unwrap_or_default().abs(), 4),
        exchange: signal
            .main_exchange
            .as_deref()
            .unwrap_or("unknown")
            .to_ascii_lowercase(),
    }
}

fn reconstruct_trajectory(
    cluster_id: &str,
    mut actions: Vec<ContractWhaleAction>,
) -> ContractWhaleTrajectory {
    actions.sort_by_key(|action| action.ts);
    actions.dedup_by(|a, b| {
        a.ts == b.ts && a.exchange == b.exchange && a.action_type == b.action_type
    });
    let start_ts = actions.first().map(|action| action.ts).unwrap_or_default();
    let end_ts = actions.last().map(|action| action.ts).unwrap_or_default();
    let regime_path = compact_regime_path(&actions);
    let stealth_profile = compute_stealth_profile(&actions);
    let aggressiveness_curve = compute_aggressiveness_curve(&actions);
    let intent = infer_intent(&actions).to_string();
    ContractWhaleTrajectory {
        trajectory_id: format!("whale-trajectory:{cluster_id}"),
        start_ts,
        end_ts,
        duration_ms: end_ts.saturating_sub(start_ts).max(0) as u64,
        actions,
        intent: intent.clone(),
        regime_path,
        stealth_profile,
        aggressiveness_curve,
        conclusion: trajectory_conclusion(&intent).to_string(),
    }
}

fn action_type(signal: &ContractWhaleSignal) -> &'static str {
    if signal.liquidation_suspected {
        return if signal.liquidation_short_btc > signal.liquidation_long_btc * 1.3 {
            "short_liquidation"
        } else if signal.liquidation_long_btc > signal.liquidation_short_btc * 1.3 {
            "long_liquidation"
        } else {
            "liquidation_unknown"
        };
    }
    match signal.signal_type {
        ContractWhaleSignalType::AggressiveBuy => "aggressive_buy",
        ContractWhaleSignalType::AggressiveSell => "aggressive_sell",
        ContractWhaleSignalType::DownsideAbsorption => "passive_absorb",
        ContractWhaleSignalType::UpsideSuppression => "liquidity_probe",
    }
}

fn infer_intent(actions: &[ContractWhaleAction]) -> &'static str {
    let independent = actions
        .iter()
        .map(|action| action.ts)
        .collect::<BTreeSet<_>>();
    let duration = independent
        .last()
        .zip(independent.first())
        .map(|(end, start)| end.saturating_sub(*start))
        .unwrap_or(0);
    if independent.len() < 3 || duration < 60_000 {
        return "unknown";
    }
    let liquidation_volume = |kind: &str| {
        actions
            .iter()
            .filter(|a| a.action_type == kind)
            .map(|a| a.volume)
            .sum::<f64>()
    };
    let short_liq = liquidation_volume("short_liquidation");
    let long_liq = liquidation_volume("long_liquidation");
    let unknown_liq = liquidation_volume("liquidation_unknown") + liquidation_volume("stop_hunt");
    let buy_pressure = actions
        .iter()
        .filter(|action| {
            matches!(
                action.action_type.as_str(),
                "aggressive_buy" | "passive_absorb" | "short_liquidation"
            )
        })
        .map(|action| action.volume)
        .sum::<f64>();
    let sell_pressure = actions
        .iter()
        .filter(|action| {
            matches!(
                action.action_type.as_str(),
                "aggressive_sell" | "liquidity_probe" | "long_liquidation"
            )
        })
        .map(|action| action.volume)
        .sum::<f64>();

    if short_liq > 0.0 && short_liq > long_liq * 1.3 && buy_pressure > sell_pressure * 1.3 {
        "short_squeeze"
    } else if long_liq > 0.0 && long_liq > short_liq * 1.3 && sell_pressure > buy_pressure * 1.3 {
        "long_liquidation"
    } else if unknown_liq > 0.0 || short_liq + long_liq > 0.0 {
        "liquidation_flow"
    } else if buy_pressure > sell_pressure * 1.3 {
        "accumulation"
    } else if sell_pressure > buy_pressure * 1.3 {
        "distribution"
    } else if actions.len() > 1 {
        "mixed_flow"
    } else {
        "unknown"
    }
}

#[cfg(test)]
mod evidence_regressions {
    use super::*;

    #[test]
    fn main_force_liquidation_trajectory_preserves_side_without_intent_claims() {
        for (action, expected) in [
            ("short_liquidation", "short_squeeze"),
            ("long_liquidation", "long_liquidation"),
            ("liquidation_unknown", "liquidation_flow"),
        ] {
            let values = [1_000, 31_000, 61_000].map(|ts| ContractWhaleAction {
                action_type: action.into(),
                ..buy(ts)
            });
            assert_eq!(infer_intent(&values), expected);
        }
        let mixed = [
            buy(1_000),
            ContractWhaleAction {
                action_type: "aggressive_sell".into(),
                volume: 200.0,
                ..buy(31_000)
            },
            buy(61_000),
        ];
        assert_eq!(infer_intent(&mixed), "mixed_flow");
    }

    fn buy(ts: i64) -> ContractWhaleAction {
        ContractWhaleAction {
            ts,
            symbol: "BTC".to_string(),
            action_type: "aggressive_buy".to_string(),
            volume: 100.0,
            price_impact: 0.1,
            exchange: "binance".to_string(),
        }
    }

    #[test]
    fn one_action_is_not_sustained_accumulation() {
        assert_eq!(infer_intent(&[buy(1_000)]), "unknown");
    }

    #[test]
    fn duplicate_actions_are_not_independent_evidence() {
        assert_eq!(
            infer_intent(&[buy(1_000), buy(1_000), buy(1_000)]),
            "unknown"
        );
    }

    #[test]
    fn sustained_independent_buy_actions_support_a_hypothesis() {
        assert_eq!(
            infer_intent(&[buy(1_000), buy(31_000), buy(61_000)]),
            "accumulation"
        );
    }
}

fn compact_regime_path(actions: &[ContractWhaleAction]) -> Vec<String> {
    let mut path = Vec::new();
    for action in actions {
        let regime = match action.action_type.as_str() {
            "aggressive_buy" | "passive_absorb" => "accumulation",
            "aggressive_sell" => "distribution",
            "liquidity_probe" => "passive_sell_candidate",
            "short_liquidation" => "short_squeeze",
            "long_liquidation" => "long_liquidation",
            "stop_hunt" | "liquidation_unknown" => "liquidation_flow",
            _ => "unclear",
        };
        if path.last().is_none_or(|last| last != regime) {
            path.push(regime.to_string());
        }
    }
    path
}

fn compute_stealth_profile(actions: &[ContractWhaleAction]) -> ContractWhaleStealthProfile {
    if actions.is_empty() {
        return ContractWhaleStealthProfile::default();
    }
    let total_volume = actions.iter().map(|action| action.volume).sum::<f64>();
    let max_volume = actions
        .iter()
        .map(|action| action.volume)
        .fold(0.0_f64, f64::max);
    let fragmentation = if total_volume > f64::EPSILON {
        1.0 - (max_volume / total_volume)
    } else {
        0.0
    };
    let entropy = normalized_entropy(actions, total_volume);
    let unique_exchanges = actions
        .iter()
        .map(|action| action.exchange.as_str())
        .collect::<BTreeSet<_>>()
        .len();
    let cross_exchange_dispersion = if actions.len() > 1 {
        ((unique_exchanges.saturating_sub(1)) as f64 / 3.0).clamp(0.0, 1.0)
    } else {
        0.0
    };
    let gamma = ((fragmentation + entropy + cross_exchange_dispersion) / 3.0).clamp(0.0, 1.0);
    ContractWhaleStealthProfile {
        gamma: round(gamma, 4),
        fragmentation: round(fragmentation.clamp(0.0, 1.0), 4),
        entropy: round(entropy.clamp(0.0, 1.0), 4),
        cross_exchange_dispersion: round(cross_exchange_dispersion, 4),
    }
}

fn normalized_entropy(actions: &[ContractWhaleAction], total_volume: f64) -> f64 {
    if actions.len() <= 1 || total_volume <= f64::EPSILON {
        return 0.0;
    }
    let entropy = actions
        .iter()
        .filter_map(|action| {
            let p = action.volume / total_volume;
            (p > 0.0).then(|| -p * p.ln())
        })
        .sum::<f64>();
    let max_entropy = (actions.len() as f64).ln();
    if max_entropy > 0.0 {
        entropy / max_entropy
    } else {
        0.0
    }
}

fn compute_aggressiveness_curve(actions: &[ContractWhaleAction]) -> Vec<f64> {
    let max_volume = actions
        .iter()
        .map(|action| action.volume)
        .fold(0.0_f64, f64::max);
    actions
        .iter()
        .map(|action| {
            let volume_component = if max_volume > f64::EPSILON {
                action.volume / max_volume
            } else {
                0.0
            };
            round(
                (volume_component * 0.75 + (action.price_impact / 1.0).clamp(0.0, 1.0) * 0.25)
                    .clamp(0.0, 1.0),
                4,
            )
        })
        .collect()
}

fn trajectory_conclusion(intent: &str) -> &'static str {
    match intent {
        "accumulation" => "持续买方压力候选；需结合持仓量区分建仓与空头平仓，不能识别交易者身份。",
        "distribution" => "持续卖方压力候选；需结合持仓量区分建仓与多头平仓，不能识别交易者身份。",
        "short_squeeze" => "观察到空头清算与买方压力，不能等同于主动新增多仓。",
        "long_liquidation" => "观察到多头清算与卖方压力，不能等同于主动新增空仓。",
        "stop_hunting" | "liquidation_flow" => "出现清算相关流量；不能据此确认主动猎取止损的意图。",
        "liquidity_manipulation" | "mixed_flow" => "多段方向混合，交易意图尚不能归因。",
        _ => "单点轨迹证据不足，保持观察。",
    }
}

fn round(value: f64, decimals: i32) -> f64 {
    let factor = 10_f64.powi(decimals);
    (value * factor).round() / factor
}
