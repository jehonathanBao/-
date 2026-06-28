use crate::contract_whale_monitor::types::{
    ContractWhaleIntelligenceResponse, ContractWhaleMarketStructureLite,
    ContractWhaleNoiseSuppressionSummary, ContractWhaleSignal,
};

pub mod liquidity;
pub mod opportunity;
pub mod ranking;
pub mod regime;
pub mod risk;
pub mod signal_compression;
pub mod strength;

use self::{
    liquidity::derive_liquidity_behaviors,
    opportunity::derive_opportunity_map,
    ranking::rank_market_events,
    regime::derive_market_regime,
    risk::build_risk_context,
    signal_compression::{build_signal_compression_summary, build_trade_ideas},
};

pub fn build_intelligence_response(
    symbol: &str,
    items: &[ContractWhaleSignal],
    market_structure_lite: &ContractWhaleMarketStructureLite,
    noise_suppression: ContractWhaleNoiseSuppressionSummary,
    timestamp: i64,
) -> ContractWhaleIntelligenceResponse {
    let market_regime = derive_market_regime(items, market_structure_lite);
    let liquidity_behaviors = derive_liquidity_behaviors(items);
    let ranked_events = rank_market_events(items, &market_regime, &liquidity_behaviors);
    let opportunity_map =
        derive_opportunity_map(items, &market_regime, &liquidity_behaviors, &ranked_events);
    let trade_ideas = build_trade_ideas(items, &ranked_events, market_structure_lite);
    let signal_compression = build_signal_compression_summary(ranked_events.len(), &trade_ideas);
    let risk_context = build_risk_context(items, &liquidity_behaviors);

    ContractWhaleIntelligenceResponse {
        symbol: symbol.to_string(),
        timestamp,
        market_regime,
        liquidity_behaviors,
        ranked_events,
        opportunity_map,
        noise_suppression,
        signal_compression,
        trade_ideas,
        risk_context,
    }
}
