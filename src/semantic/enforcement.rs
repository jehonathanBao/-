use crate::{
    contract_whale_monitor::types::ContractWhaleRiskContext, semantic::contract::SemanticRiskState,
};

pub fn risk_state_from_context(context: &ContractWhaleRiskContext) -> SemanticRiskState {
    if context.fake_breakout_risk.eq_ignore_ascii_case("HIGH") {
        return SemanticRiskState::High;
    }
    if !context.no_trade_zones.is_empty() {
        return SemanticRiskState::Guarded;
    }
    SemanticRiskState::from_label(&context.fake_breakout_risk)
}
