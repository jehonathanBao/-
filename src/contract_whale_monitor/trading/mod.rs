pub mod bias;
pub mod classifier;
pub mod fine_tune;
pub mod noise_filter;
pub mod scoring;

use crate::contract_whale_monitor::types::{
    ContractWhaleMarketStructureLite, ContractWhaleNoTradeZone,
    ContractWhaleNoiseSuppressionSummary, ContractWhaleSignal,
    ContractWhaleTradingDecisionResponse, ContractWhaleTradingEntryZone,
    ContractWhaleTradingInvalidation, ContractWhaleTradingSetup,
};

use self::{
    bias::derive_market_bias,
    classifier::{classify_direction, setup_type_label, TradingDirection},
    fine_tune::{
        adjusted_trade_score, fine_tune_reject_reason, recalibrated_confidence,
        should_prune_similar_setup,
    },
    noise_filter::{evaluate_tradeability, to_no_trade_zone},
    scoring::{confidence_label, score_signal},
};

pub fn build_trading_decision_response(
    symbol: &str,
    items: &[ContractWhaleSignal],
    market_structure_lite: &ContractWhaleMarketStructureLite,
    noise_suppression: ContractWhaleNoiseSuppressionSummary,
    timestamp: i64,
) -> ContractWhaleTradingDecisionResponse {
    let mut top_setups = Vec::new();
    let mut no_trade_zones = Vec::new();

    for signal in items {
        let base_score = score_signal(signal);
        let liquidity_behavior =
            crate::contract_whale_monitor::intelligence::liquidity::behavior_for_signal(signal);
        let regime_context = regime_context_label(market_structure_lite);
        let score = adjusted_trade_score(signal, base_score, &regime_context, liquidity_behavior);
        if let Some(reason) =
            fine_tune_reject_reason(signal, score, &regime_context, liquidity_behavior)
        {
            if let Some(zone) = to_no_trade_zone(signal, reason) {
                push_unique_no_trade_zone(&mut no_trade_zones, zone);
            }
            continue;
        }
        let filter_result = evaluate_tradeability(signal, score);
        if !filter_result.accepted {
            if let Some(zone) = to_no_trade_zone(signal, &filter_result.reason) {
                push_unique_no_trade_zone(&mut no_trade_zones, zone);
            }
            continue;
        }

        let direction = classify_direction(signal, score);
        if direction == TradingDirection::NoTrade {
            if let Some(zone) = to_no_trade_zone(signal, "price_response_missing") {
                push_unique_no_trade_zone(&mut no_trade_zones, zone);
            }
            continue;
        }

        let confidence = recalibrated_confidence(signal, score, liquidity_behavior);
        top_setups.push(ContractWhaleTradingSetup {
            signal_id: signal.id.clone(),
            rank: 0,
            direction: direction.as_str().to_string(),
            setup_type: setup_type_label(signal.signal_type).to_string(),
            score,
            confidence,
            confidence_label: confidence_label(confidence).to_string(),
            regime_context: regime_context_label(market_structure_lite),
            window_sec: signal.window_sec,
            entry_zone: build_entry_zone(signal),
            invalidation: build_invalidation(signal, direction),
            reasons: build_reasons(signal),
        });
    }

    top_setups.sort_by(|left, right| {
        right
            .score
            .cmp(&left.score)
            .then_with(|| right.confidence.cmp(&left.confidence))
            .then_with(|| right.window_sec.cmp(&left.window_sec))
    });
    let mut pruned_setups = Vec::new();
    for setup in top_setups {
        let candidate_signal = items.iter().find(|item| item.id == setup.signal_id);
        let should_prune = pruned_setups
            .iter()
            .any(|kept: &ContractWhaleTradingSetup| {
                let Some(kept_signal) = items.iter().find(|item| item.id == kept.signal_id) else {
                    return false;
                };
                let Some(candidate_signal) = candidate_signal else {
                    return false;
                };
                should_prune_similar_setup(
                    kept_signal,
                    &kept.direction,
                    kept.score,
                    candidate_signal,
                    &setup.direction,
                    setup.score,
                )
            });
        if should_prune {
            continue;
        }
        pruned_setups.push(setup);
        if pruned_setups.len() >= 3 {
            break;
        }
    }
    top_setups = pruned_setups;
    for (index, setup) in top_setups.iter_mut().enumerate() {
        setup.rank = index + 1;
    }

    let bias = derive_market_bias(&top_setups);
    ContractWhaleTradingDecisionResponse {
        symbol: symbol.to_string(),
        timestamp,
        market_bias: bias.market_bias,
        bias_confidence: bias.confidence,
        bias_reason: bias.reason,
        noise_suppression,
        top_setups,
        no_trade_zones,
    }
}

fn build_entry_zone(signal: &ContractWhaleSignal) -> ContractWhaleTradingEntryZone {
    let anchor = signal
        .order_price_usd
        .or(signal.current_market_price_usd)
        .unwrap_or_default();
    if anchor <= 0.0 {
        return ContractWhaleTradingEntryZone::default();
    }
    let band_pct = (signal.price_move_pct.unwrap_or(0.18).abs() / 100.0).clamp(0.0010, 0.0035);
    let low_price = round2(anchor * (1.0 - band_pct));
    let high_price = round2(anchor * (1.0 + band_pct));
    ContractWhaleTradingEntryZone {
        low_price,
        high_price,
        label: format!("{:.0} - {:.0}", low_price, high_price),
    }
}

fn build_invalidation(
    signal: &ContractWhaleSignal,
    direction: TradingDirection,
) -> ContractWhaleTradingInvalidation {
    let anchor = signal
        .order_price_usd
        .or(signal.current_market_price_usd)
        .unwrap_or_default();
    if anchor <= 0.0 {
        return ContractWhaleTradingInvalidation::default();
    }
    let distance_pct =
        ((signal.price_move_pct.unwrap_or(0.20).abs() / 100.0) * 1.35).clamp(0.0025, 0.0080);
    let price_level = match direction {
        TradingDirection::Long => round2(anchor * (1.0 - distance_pct)),
        TradingDirection::Short => round2(anchor * (1.0 + distance_pct)),
        TradingDirection::NoTrade => round2(anchor),
    };
    let reason = match direction {
        TradingDirection::Long => "跌破主力吸收参考位，说明顺势跟随失效。",
        TradingDirection::Short => "重新站回压制参考位上方，说明顺势做空失效。",
        TradingDirection::NoTrade => "当前仅作观察，不构成交易失效线。",
    };
    ContractWhaleTradingInvalidation {
        price_level,
        reason: reason.to_string(),
    }
}

fn build_reasons(signal: &ContractWhaleSignal) -> Vec<String> {
    let mut reasons = Vec::new();
    if !signal.merged_from.is_empty() {
        reasons.push("多窗口主力信号对齐".to_string());
    }
    if signal.multi_exchange_confirmed {
        reasons.push("双交易所确认".to_string());
    }
    if signal.dominance >= 0.55 {
        reasons.push(format!("方向占比 {:.1}%", signal.dominance * 100.0));
    }
    if let Some(price_move_pct) = signal.price_move_pct {
        if price_move_pct.abs() >= 0.10 {
            reasons.push(format!("价格顺势 {:.2}%", price_move_pct));
        }
    }
    if reasons.is_empty() {
        reasons.push("结构分数满足交易观察阈值".to_string());
    }
    reasons
}

fn regime_context_label(market_structure_lite: &ContractWhaleMarketStructureLite) -> String {
    if !market_structure_lite.regime_type.trim().is_empty() {
        market_structure_lite.regime_type.clone()
    } else if !market_structure_lite.status.trim().is_empty() {
        market_structure_lite.status.clone()
    } else {
        "unclear".to_string()
    }
}

fn push_unique_no_trade_zone(
    zones: &mut Vec<ContractWhaleNoTradeZone>,
    candidate: ContractWhaleNoTradeZone,
) {
    if zones.iter().any(|zone| {
        zone.reason == candidate.reason
            && (zone.low_price - candidate.low_price).abs() < 0.01
            && (zone.high_price - candidate.high_price).abs() < 0.01
    }) {
        return;
    }
    zones.push(candidate);
    zones.truncate(3);
}

fn round2(value: f64) -> f64 {
    (value * 100.0).round() / 100.0
}
