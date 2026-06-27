use crate::contract_whale_monitor::types::{
    ContractWhaleNoTradeZone, ContractWhalePriceResponseType, ContractWhaleSignal,
};

#[derive(Debug, Clone)]
pub struct NoiseFilterResult {
    pub accepted: bool,
    pub reason: String,
}

pub fn evaluate_tradeability(signal: &ContractWhaleSignal, score: u8) -> NoiseFilterResult {
    if score < 70 {
        return NoiseFilterResult {
            accepted: false,
            reason: "score_below_trade_threshold".to_string(),
        };
    }
    if signal.event_quality.quality_score < 0.60 {
        return NoiseFilterResult {
            accepted: false,
            reason: "quality_below_threshold".to_string(),
        };
    }
    if signal.dominance < 0.55 {
        return NoiseFilterResult {
            accepted: false,
            reason: "dominance_below_threshold".to_string(),
        };
    }
    if matches!(
        signal.price_response_type,
        ContractWhalePriceResponseType::NoClearResponse
    ) {
        return NoiseFilterResult {
            accepted: false,
            reason: "price_response_missing".to_string(),
        };
    }
    if signal.merged_from.is_empty()
        && !signal.multi_exchange_confirmed
        && signal.event_lifecycle.update_count <= 1
    {
        return NoiseFilterResult {
            accepted: false,
            reason: "single_window_spike_only".to_string(),
        };
    }
    NoiseFilterResult {
        accepted: true,
        reason: "accepted".to_string(),
    }
}

pub fn to_no_trade_zone(signal: &ContractWhaleSignal, reason: &str) -> Option<ContractWhaleNoTradeZone> {
    let anchor = signal
        .order_price_usd
        .or(signal.current_market_price_usd)
        .unwrap_or_default();
    if anchor <= 0.0 {
        return None;
    }
    let band_pct = (signal.price_move_pct.unwrap_or(0.12).abs().max(0.12) / 100.0).clamp(0.0012, 0.0040);
    let low_price = round2(anchor * (1.0 - band_pct));
    let high_price = round2(anchor * (1.0 + band_pct));
    Some(ContractWhaleNoTradeZone {
        reason: human_reason(reason).to_string(),
        range_label: format_price_range(low_price, high_price),
        low_price,
        high_price,
    })
}

fn human_reason(reason: &str) -> &'static str {
    match reason {
        "price_response_missing" => "价格响应不足，当前更像低分震荡 chop。",
        "single_window_spike_only" => "只有单窗口脉冲，没有形成跨窗口确认。",
        "dominance_below_threshold" => "净方向占比不足，暂时不具备交易优势。",
        "quality_below_threshold" => "事件质量分偏低，先保持观察。",
        _ => "当前不满足交易门槛，保留为 no-trade 观察区。",
    }
}

fn format_price_range(low: f64, high: f64) -> String {
    format!("{:.0} - {:.0}", low, high)
}

fn round2(value: f64) -> f64 {
    (value * 100.0).round() / 100.0
}
