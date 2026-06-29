use crate::contract_whale_monitor::types::{
    ContractWhaleLiquidityBehavior, ContractWhaleRankedEvent, ContractWhaleRegimeSnapshot,
    ContractWhaleSignal, ContractWhaleSignalType,
};

use super::{
    liquidity::behavior_for_signal,
    strength::{direction_bias_label, score_signal_strength, strength_label},
};
use crate::contract_whale_monitor::trading::fine_tune::{
    adjusted_trade_score, fine_tune_reject_reason, should_prune_similar_setup,
};
use crate::semantic::contract::SemanticType;

pub fn rank_market_events(
    items: &[ContractWhaleSignal],
    regime: &ContractWhaleRegimeSnapshot,
    liquidity_behaviors: &[ContractWhaleLiquidityBehavior],
) -> Vec<ContractWhaleRankedEvent> {
    let mut ranked = items
        .iter()
        .filter_map(|signal| {
            let base_strength = score_signal_strength(signal);
            let regime_alignment = regime_alignment(regime, signal);
            let liquidity_behavior = behavior_for_signal(signal);
            let behavior_boost = liquidity_behaviors
                .iter()
                .find(|item| item.behavior == liquidity_behavior)
                .map(|item| item.confidence / 10)
                .unwrap_or_default();
            let boosted_strength = base_strength.saturating_add(behavior_boost).min(100);
            let final_strength =
                adjusted_trade_score(signal, boosted_strength, &regime.regime, liquidity_behavior);

            if fine_tune_reject_reason(signal, final_strength, &regime.regime, liquidity_behavior)
                .is_some()
            {
                return None;
            }

            Some(ContractWhaleRankedEvent {
                semantic_type: SemanticType::Analysis,
                signal_id: signal.id.clone(),
                rank: 0,
                event_type: event_type_label(signal.signal_type).to_string(),
                direction_bias: direction_bias_label(signal.direction).to_string(),
                strength_score: final_strength,
                strength_label: strength_label(final_strength).to_string(),
                regime_alignment: regime_alignment.to_string(),
                liquidity_behavior: liquidity_behavior.to_string(),
                window_sec: signal.window_sec,
                rationale: build_rationale(signal, liquidity_behavior, regime_alignment),
            })
        })
        .collect::<Vec<_>>();

    ranked.sort_by(|left, right| {
        right
            .strength_score
            .cmp(&left.strength_score)
            .then_with(|| right.window_sec.cmp(&left.window_sec))
    });
    let mut pruned = Vec::new();
    for candidate in ranked {
        let candidate_signal = items.iter().find(|item| item.id == candidate.signal_id);
        let should_prune = pruned.iter().any(|kept: &ContractWhaleRankedEvent| {
            let Some(kept_signal) = items.iter().find(|item| item.id == kept.signal_id) else {
                return false;
            };
            let Some(candidate_signal) = candidate_signal else {
                return false;
            };
            should_prune_similar_setup(
                kept_signal,
                &kept.direction_bias,
                kept.strength_score,
                candidate_signal,
                &candidate.direction_bias,
                candidate.strength_score,
            )
        });
        if should_prune {
            continue;
        }
        pruned.push(candidate);
        if pruned.len() >= 3 {
            break;
        }
    }
    ranked = pruned;
    for (index, item) in ranked.iter_mut().enumerate() {
        item.rank = index + 1;
    }
    ranked
}

fn event_type_label(signal_type: ContractWhaleSignalType) -> &'static str {
    match signal_type {
        ContractWhaleSignalType::AggressiveBuy => "主力拉盘",
        ContractWhaleSignalType::AggressiveSell => "主力砸盘",
        ContractWhaleSignalType::DownsideAbsorption => "下方吸收",
        ContractWhaleSignalType::UpsideSuppression => "上方压制",
    }
}

fn regime_alignment(
    regime: &ContractWhaleRegimeSnapshot,
    signal: &ContractWhaleSignal,
) -> &'static str {
    match regime.regime.as_str() {
        "TRENDING_UP" if signal.net_volume_btc > 0.0 => "aligned",
        "TRENDING_DOWN" if signal.net_volume_btc < 0.0 => "aligned",
        "RANGING"
            if matches!(
                signal.signal_type,
                ContractWhaleSignalType::DownsideAbsorption
                    | ContractWhaleSignalType::UpsideSuppression
            ) =>
        {
            "aligned"
        }
        "LIQUIDATION_PHASE" if signal.liquidation_suspected => "aligned",
        _ => "mixed",
    }
}

fn build_rationale(
    signal: &ContractWhaleSignal,
    liquidity_behavior: &str,
    regime_alignment: &str,
) -> String {
    let mut parts = Vec::new();
    if signal.multi_exchange_confirmed {
        parts.push("多交易所确认".to_string());
    }
    if !signal.merged_from.is_empty() {
        parts.push("多窗口对齐".to_string());
    }
    if let Some(price_move_pct) = signal.price_move_pct {
        if price_move_pct.abs() >= 0.08 {
            parts.push(format!("价格响应 {price_move_pct:+.2}%"));
        }
    }
    parts.push(format!("流动性行为 {liquidity_behavior}"));
    if regime_alignment == "aligned" {
        parts.push("与当前市场状态同向".to_string());
    }
    if parts.is_empty() {
        signal.final_result.clone()
    } else {
        parts.join("，")
    }
}
