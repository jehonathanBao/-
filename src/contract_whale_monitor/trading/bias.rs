use crate::contract_whale_monitor::types::ContractWhaleTradingSetup;

#[derive(Debug, Clone)]
pub struct MarketBiasSnapshot {
    pub market_bias: String,
    pub confidence: u8,
    pub reason: String,
}

pub fn derive_market_bias(setups: &[ContractWhaleTradingSetup]) -> MarketBiasSnapshot {
    let long_score: u32 = setups
        .iter()
        .filter(|setup| setup.direction_bias == "BULLISH_BIAS")
        .map(|setup| u32::from(setup.score))
        .sum();
    let short_score: u32 = setups
        .iter()
        .filter(|setup| setup.direction_bias == "BEARISH_BIAS")
        .map(|setup| u32::from(setup.score))
        .sum();
    let total = long_score + short_score;
    if total == 0 {
        return MarketBiasSnapshot {
            market_bias: "NEUTRAL".to_string(),
            confidence: 0,
            reason: "当前没有通过交易门槛的 setup，保持中性观察。".to_string(),
        };
    }

    let (market_bias, dominant, opposing) = if long_score >= short_score {
        ("BULLISH", long_score, short_score)
    } else {
        ("BEARISH", short_score, long_score)
    };
    let dominance_gap = dominant.saturating_sub(opposing) as f64;
    let confidence = ((dominance_gap / total as f64) * 100.0)
        .round()
        .clamp(35.0, 100.0) as u8;
    let reason = if market_bias == "BULLISH" {
        format!("多头高分 setup 合计 {dominant}，明显高于空头 {opposing}。")
    } else {
        format!("空头高分 setup 合计 {dominant}，明显高于多头 {opposing}。")
    };
    MarketBiasSnapshot {
        market_bias: market_bias.to_string(),
        confidence,
        reason,
    }
}
