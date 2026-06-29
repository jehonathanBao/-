use crate::contract_whale_monitor::{
    trading::noise_filter::{evaluate_tradeability, to_no_trade_zone},
    types::{
        ContractWhaleLiquidityBehavior, ContractWhaleNoTradeZone, ContractWhaleRiskContext,
        ContractWhaleSignal,
    },
};
use crate::semantic::contract::{SemanticRiskState, SemanticType};

pub fn build_risk_context(
    items: &[ContractWhaleSignal],
    liquidity_behaviors: &[ContractWhaleLiquidityBehavior],
) -> ContractWhaleRiskContext {
    let mut no_trade_zones = Vec::<ContractWhaleNoTradeZone>::new();
    for signal in items {
        let score = crate::contract_whale_monitor::trading::scoring::score_signal(signal);
        let filter_result = evaluate_tradeability(signal, score);
        if !filter_result.accepted {
            if let Some(zone) = to_no_trade_zone(signal, &filter_result.reason) {
                push_unique_zone(&mut no_trade_zones, zone);
            }
        }
    }

    let fake_breakout_risk = if liquidity_behaviors
        .iter()
        .any(|item| item.behavior == "fake_breakout" && item.confidence >= 70)
    {
        "HIGH"
    } else if liquidity_behaviors
        .iter()
        .any(|item| item.behavior == "fake_breakout")
    {
        "MEDIUM"
    } else if no_trade_zones.is_empty() {
        "LOW"
    } else {
        "GUARDED"
    }
    .to_string();

    let risk_state = if fake_breakout_risk == "HIGH" {
        SemanticRiskState::High
    } else if !no_trade_zones.is_empty() {
        SemanticRiskState::Guarded
    } else {
        SemanticRiskState::Low
    };

    let summary = if fake_breakout_risk == "HIGH" {
        "当前存在较强假突破风险，交易参考需要让位于风险抑制。".to_string()
    } else if !no_trade_zones.is_empty() {
        "当前结构存在 chop / 低响应区，先参考 no-trade 范围。".to_string()
    } else {
        "当前未发现显著 no-trade 结构风险。".to_string()
    };

    ContractWhaleRiskContext {
        semantic_type: SemanticType::RiskOverride,
        risk_state,
        no_trade_zones,
        fake_breakout_risk,
        summary,
    }
}

fn push_unique_zone(
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
