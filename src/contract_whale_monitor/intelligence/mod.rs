use crate::contract_whale_monitor::types::{
    ContractWhaleIntelligenceResponse, ContractWhaleMarketStructureLite,
    ContractWhaleNoiseSuppressionSummary, ContractWhaleSignal,
};

pub mod liquidity;
pub mod opportunity;
pub mod ranking;
pub mod regime;
pub mod strength;

use self::{
    liquidity::derive_liquidity_behaviors,
    opportunity::derive_opportunity_map,
    ranking::rank_market_events,
    regime::derive_market_regime,
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

    ContractWhaleIntelligenceResponse {
        symbol: symbol.to_string(),
        timestamp,
        market_regime,
        liquidity_behaviors,
        ranked_events,
        opportunity_map,
        noise_suppression,
    }
}
