use super::types::AltContractContext;

pub fn empty_context() -> AltContractContext {
    AltContractContext::default()
}

pub fn context_data_quality_penalty(context: &AltContractContext) -> u8 {
    let mut penalty = 0_u8;
    if context.force_order_snapshot {
        penalty = penalty.saturating_add(5);
    }
    if context.oi_change_1m_base.is_none() && context.oi_change_5m_base.is_none() {
        penalty = penalty.saturating_add(5);
    }
    penalty
}
