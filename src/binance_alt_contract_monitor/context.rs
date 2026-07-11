use super::types::AltContractContext;

pub fn empty_context() -> AltContractContext {
    AltContractContext::default()
}

pub fn context_for_window(context: &AltContractContext, window_sec: u64) -> AltContractContext {
    if context.oi_change_1m.period_sec == 0 && context.oi_change_5m.period_sec == 0 {
        return context.clone();
    }
    let mut selected = context.clone();
    selected.oi_change_1m_base = None;
    selected.oi_change_5m_base = None;
    selected.oi_change_pct = None;

    match window_sec {
        60 if selected.oi_change_1m.available && !selected.oi_change_1m.stale => {
            selected.oi_change_1m_base = selected.oi_change_1m.delta;
            selected.oi_change_pct = selected.oi_change_1m.delta_pct;
        }
        300 if selected.oi_change_5m.available && !selected.oi_change_5m.stale => {
            selected.oi_change_5m_base = selected.oi_change_5m.delta;
            selected.oi_change_pct = selected.oi_change_5m.delta_pct;
        }
        _ => {}
    }
    selected
}

pub fn context_data_quality_penalty(context: &AltContractContext) -> u8 {
    let mut penalty = 0_u8;
    if context.force_order_snapshot {
        penalty = penalty.saturating_add(5);
    }
    if context.oi_change_1m_base.is_none() && context.oi_change_5m_base.is_none() {
        penalty = penalty.saturating_add(15);
    }
    if context
        .ticker_quote_volume_24h_usd
        .is_none_or(|_| context.ticker_updated_at.is_none())
    {
        penalty = penalty.saturating_add(10);
    } else if context.ticker_updated_at.is_some_and(|updated_at| {
        crate::normalizers::trade::now_ms().saturating_sub(updated_at) > 120_000
    }) {
        penalty = penalty.saturating_add(15);
    }
    if context.mark_price_usd.is_none() {
        penalty = penalty.saturating_add(5);
    }
    penalty
}
