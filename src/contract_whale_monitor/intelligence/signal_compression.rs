use std::collections::BTreeSet;

use crate::contract_whale_monitor::{
    intelligence::liquidity::behavior_for_signal,
    trading::{
        classifier::{classify_direction, setup_type_label, TradingDirection},
        fine_tune::{
            adjusted_trade_score, fine_tune_reject_reason, recalibrated_confidence,
            should_prune_similar_setup,
        },
        noise_filter::evaluate_tradeability,
        scoring::{confidence_label, score_signal},
    },
    types::{
        ContractWhaleMarketStructureLite, ContractWhaleRankedEvent, ContractWhaleSignal,
        ContractWhaleSignalCompressionSummary, ContractWhaleTradeIdea,
        ContractWhaleTradingEntryZone, ContractWhaleTradingInvalidation,
    },
};

pub fn build_trade_ideas(
    items: &[ContractWhaleSignal],
    ranked_events: &[ContractWhaleRankedEvent],
    market_structure_lite: &ContractWhaleMarketStructureLite,
) -> Vec<ContractWhaleTradeIdea> {
    let mut ideas = Vec::new();
    let mut seen = BTreeSet::new();
    let regime_context = regime_context_label(market_structure_lite);

    for ranked_event in ranked_events {
        let Some(signal) = items.iter().find(|item| item.id == ranked_event.signal_id) else {
            continue;
        };
        let liquidity_behavior = behavior_for_signal(signal);
        let score = adjusted_trade_score(
            signal,
            score_signal(signal),
            &regime_context,
            liquidity_behavior,
        );
        if fine_tune_reject_reason(signal, score, &regime_context, liquidity_behavior).is_some() {
            continue;
        }
        let filter_result = evaluate_tradeability(signal, score);
        if !filter_result.accepted {
            continue;
        }
        let direction = classify_direction(signal, score);
        let direction_bias = match direction {
            TradingDirection::Long => "BULLISH_BIAS",
            TradingDirection::Short => "BEARISH_BIAS",
            TradingDirection::NoTrade => "NEUTRAL_BIAS",
        };
        if direction == TradingDirection::NoTrade {
            continue;
        }
        let setup_family = setup_type_label(signal.signal_type);
        let dedup_key = format!("{setup_family}:{direction_bias}");
        if !seen.insert(dedup_key) {
            continue;
        }
        let confidence = recalibrated_confidence(signal, score, liquidity_behavior);
        let candidate = ContractWhaleTradeIdea {
            signal_id: signal.id.clone(),
            rank: 0,
            setup_type: setup_family.to_string(),
            direction_bias: direction_bias.to_string(),
            score,
            confidence,
            confidence_label: confidence_label(confidence).to_string(),
            entry_zone: build_entry_zone(signal),
            invalidation: build_invalidation(signal, direction),
            structure_context: ranked_event.rationale.clone(),
            regime_context: regime_context.clone(),
            window_sec: signal.window_sec,
        };
        let should_prune = ideas.iter().any(|kept: &ContractWhaleTradeIdea| {
            let Some(kept_signal) = items.iter().find(|item| item.id == kept.signal_id) else {
                return false;
            };
            should_prune_similar_setup(
                kept_signal,
                &kept.direction_bias,
                kept.score,
                signal,
                &candidate.direction_bias,
                candidate.score,
            )
        });
        if should_prune {
            continue;
        }
        ideas.push(candidate);
        if ideas.len() >= 3 {
            break;
        }
    }

    for (index, idea) in ideas.iter_mut().enumerate() {
        idea.rank = index + 1;
    }
    ideas
}

pub fn build_signal_compression_summary(
    candidate_count: usize,
    trade_ideas: &[ContractWhaleTradeIdea],
) -> ContractWhaleSignalCompressionSummary {
    let quality_score = if trade_ideas.is_empty() {
        0
    } else {
        (trade_ideas
            .iter()
            .map(|item| item.confidence as u32)
            .sum::<u32>() as f64
            / trade_ideas.len() as f64)
            .round()
            .clamp(0.0, 100.0) as u8
    };

    ContractWhaleSignalCompressionSummary {
        quality_score,
        top_signal_count: trade_ideas.len(),
        discarded_count: candidate_count.saturating_sub(trade_ideas.len()),
        compression_reason: "cross-window dedup + quality gating + no-trade suppression"
            .to_string(),
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
        TradingDirection::Long => "跌破主力吸收参考位，说明当前结构支持减弱。",
        TradingDirection::Short => "重新站回压制参考位上方，说明当前压制结构失效。",
        TradingDirection::NoTrade => "当前仅作观察，不构成结构失效线。",
    };
    ContractWhaleTradingInvalidation {
        price_level,
        reason: reason.to_string(),
    }
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

fn round2(value: f64) -> f64 {
    (value * 100.0).round() / 100.0
}
