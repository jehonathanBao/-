use crate::{
    contract_whale_monitor::types::{ContractWhaleRiskContext, ContractWhaleTradeIdea},
    semantic::enforcement::risk_state_from_context,
};

pub fn suppress_decision_support_when_risk_high(
    trade_ideas: &mut Vec<ContractWhaleTradeIdea>,
    risk_context: &ContractWhaleRiskContext,
) {
    if risk_state_from_context(risk_context).suppresses_decision_support() {
        trade_ideas.clear();
    }
}
