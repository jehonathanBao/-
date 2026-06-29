use std::collections::BTreeMap;

use crate::contract_whale_monitor::types::{
    ContractWhaleLiquidityBehavior, ContractWhalePriceResponseType, ContractWhaleSignal,
    ContractWhaleSignalType,
};
use crate::semantic::contract::SemanticType;

use super::strength::score_signal_strength;

pub fn derive_liquidity_behaviors(
    items: &[ContractWhaleSignal],
) -> Vec<ContractWhaleLiquidityBehavior> {
    let mut by_behavior: BTreeMap<String, ContractWhaleLiquidityBehavior> = BTreeMap::new();

    for signal in items {
        let (behavior, label, reason) = classify_liquidity_behavior(signal);
        let strength_score = score_signal_strength(signal);
        let confidence = behavior_confidence(signal, strength_score);
        let (low_price, high_price, range_label) = signal_range(signal);
        let candidate = ContractWhaleLiquidityBehavior {
            semantic_type: SemanticType::Analysis,
            behavior: behavior.to_string(),
            label: label.to_string(),
            strength_score,
            confidence,
            reason: reason.to_string(),
            range_label,
            low_price,
            high_price,
        };

        match by_behavior.get(&candidate.behavior) {
            Some(existing) if existing.strength_score >= candidate.strength_score => {}
            _ => {
                by_behavior.insert(candidate.behavior.clone(), candidate);
            }
        }
    }

    let mut values = by_behavior.into_values().collect::<Vec<_>>();
    values.sort_by(|left, right| {
        right
            .strength_score
            .cmp(&left.strength_score)
            .then_with(|| right.confidence.cmp(&left.confidence))
    });
    values.truncate(4);
    values
}

pub fn behavior_for_signal(signal: &ContractWhaleSignal) -> &'static str {
    classify_liquidity_behavior(signal).0
}

fn classify_liquidity_behavior(
    signal: &ContractWhaleSignal,
) -> (&'static str, &'static str, &'static str) {
    if signal.liquidation_suspected || signal.liquidation_ratio.unwrap_or_default() >= 0.10 {
        return (
            "liquidity_sweep",
            "Liquidity Sweep",
            "成交与清算压力同步放大，价格更像被流动性抽走后快速推进。",
        );
    }
    match (signal.signal_type, signal.price_response_type) {
        (ContractWhaleSignalType::DownsideAbsorption, _)
        | (_, ContractWhalePriceResponseType::DownsideAbsorption) => (
            "absorption",
            "Absorption",
            "卖压释放后价格没有继续下破，说明承接资金在稳定吸收流动性。",
        ),
        (
            ContractWhaleSignalType::UpsideSuppression,
            ContractWhalePriceResponseType::NoClearResponse,
        )
        | (
            ContractWhaleSignalType::UpsideSuppression,
            ContractWhalePriceResponseType::UpsideResistance,
        ) => (
            "fake_breakout",
            "Fake Breakout",
            "冲高阶段出现压制且价格跟随不足，更像假突破风险而非持续上攻。",
        ),
        (ContractWhaleSignalType::AggressiveBuy, ContractWhalePriceResponseType::TrendFollowUp) => {
            (
                "breakout_pressure",
                "Breakout Pressure",
                "主动买盘带来价格顺势抬升，说明上方突破压力正在累积。",
            )
        }
        (
            ContractWhaleSignalType::AggressiveSell,
            ContractWhalePriceResponseType::TrendFollowDown,
        ) => (
            "distribution",
            "Distribution",
            "卖盘主导且价格顺势走弱，更像高位分发而不是单纯震荡。",
        ),
        (ContractWhaleSignalType::AggressiveBuy, _)
        | (ContractWhaleSignalType::AggressiveSell, _) => (
            "order_block_behavior",
            "Order Block",
            "成交量放大但响应一般，先按订单块博弈区处理，等待结构确认。",
        ),
        _ => (
            "order_block_behavior",
            "Order Block",
            "当前主力行为仍在整理，先记录为订单块博弈区。",
        ),
    }
}

fn behavior_confidence(signal: &ContractWhaleSignal, strength_score: u8) -> u8 {
    (f64::from(strength_score) * 0.55
        + signal.event_quality.quality_score * 25.0
        + if signal.multi_exchange_confirmed {
            10.0
        } else {
            0.0
        })
    .round()
    .clamp(0.0, 100.0) as u8
}

fn signal_range(signal: &ContractWhaleSignal) -> (f64, f64, String) {
    let anchor = signal
        .order_price_usd
        .or(signal.current_market_price_usd)
        .unwrap_or_default();
    if anchor <= 0.0 {
        return (0.0, 0.0, "N/A".to_string());
    }
    let band_pct = (signal.price_move_pct.unwrap_or(0.12).abs() / 100.0).clamp(0.0008, 0.0020);
    let low = round2(anchor * (1.0 - band_pct));
    let high = round2(anchor * (1.0 + band_pct));
    (low, high, format_range(low, high))
}

fn format_range(low: f64, high: f64) -> String {
    format!("{:.0} - {:.0}", low, high)
}

fn round2(value: f64) -> f64 {
    (value * 100.0).round() / 100.0
}
