use crate::contract_whale_monitor::types::{
    ContractWhaleMarketStructureLite, ContractWhaleRegimeSnapshot, ContractWhaleSignal,
};
use crate::semantic::contract::SemanticType;

pub fn derive_market_regime(
    items: &[ContractWhaleSignal],
    market_structure_lite: &ContractWhaleMarketStructureLite,
) -> ContractWhaleRegimeSnapshot {
    if items.is_empty() {
        return ContractWhaleRegimeSnapshot {
            semantic_type: SemanticType::Analysis,
            regime: "RANGING".to_string(),
            confidence: 0,
            reason: "最近 24h 没有新的主力历史信号，保留区间观察。".to_string(),
        };
    }

    let total_net = items.iter().map(|item| item.net_volume_btc).sum::<f64>();
    let total_volume = items.iter().map(|item| item.total_volume_btc).sum::<f64>();
    let directional_persistence = if total_volume > 0.0 {
        (total_net.abs() / total_volume).clamp(0.0, 1.0)
    } else {
        0.0
    };
    let avg_move = items
        .iter()
        .map(|item| item.price_move_pct.unwrap_or_default().abs())
        .sum::<f64>()
        / items.len() as f64;
    let liquidation_ratio = items
        .iter()
        .filter(|item| {
            item.liquidation_suspected || item.liquidation_ratio.unwrap_or_default() >= 0.10
        })
        .count() as f64
        / items.len() as f64;

    let structure_hint = market_structure_lite.regime_type.to_ascii_lowercase();
    let (regime, confidence, reason) =
        if liquidation_ratio >= 0.45 || market_structure_lite.extreme_impact_confirmed {
            (
                "LIQUIDATION_PHASE",
                (75.0 + liquidation_ratio * 20.0).round().clamp(0.0, 100.0) as u8,
                "近期主力事件与清算压力重叠，结构进入清算主导阶段。".to_string(),
            )
        } else if directional_persistence >= 0.50
            && (total_net > 0.0 || structure_hint.contains("long"))
            && avg_move >= 0.12
        {
            (
                "TRENDING_UP",
                (68.0
                    + directional_persistence * 18.0
                    + f64::from(market_structure_lite.confidence) * 0.12)
                    .round()
                    .clamp(0.0, 100.0) as u8,
                "主动买入延续性明显，价格顺势响应，市场更接近上行趋势。".to_string(),
            )
        } else if directional_persistence >= 0.50
            && (total_net < 0.0 || structure_hint.contains("short"))
            && avg_move >= 0.12
        {
            (
                "TRENDING_DOWN",
                (68.0
                    + directional_persistence * 18.0
                    + f64::from(market_structure_lite.confidence) * 0.12)
                    .round()
                    .clamp(0.0, 100.0) as u8,
                "主动卖出持续压制价格，结构更接近下行趋势。".to_string(),
            )
        } else if avg_move >= 0.28 || market_structure_lite.extreme_impact_score >= 65 {
            (
                "HIGH_VOLATILITY",
                (60.0 + avg_move.min(0.8) * 45.0).round().clamp(0.0, 100.0) as u8,
                "波动显著放大，但方向延续不足，当前更像高波动博弈阶段。".to_string(),
            )
        } else {
            (
                "RANGING",
                (62.0 + f64::from(market_structure_lite.data_quality) * 0.18)
                    .round()
                    .clamp(0.0, 100.0) as u8,
                "成交量活跃但价格延续性一般，结构更接近区间整理。".to_string(),
            )
        };

    ContractWhaleRegimeSnapshot {
        semantic_type: SemanticType::Analysis,
        regime: regime.to_string(),
        confidence,
        reason,
    }
}
