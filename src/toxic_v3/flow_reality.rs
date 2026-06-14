use super::types::{clamp01, MarketFlowTick, StealthFeatures};

pub fn total_volume(flow: &MarketFlowTick) -> f64 {
    (flow.buy_volume.max(0.0) + flow.sell_volume.max(0.0)).max(0.0)
}

pub fn directional_strength(flow: &MarketFlowTick) -> f64 {
    let total = total_volume(flow);
    if total <= f64::EPSILON {
        return 0.0;
    }
    clamp01((flow.buy_volume - flow.sell_volume).abs() / total)
}

pub fn derive_stealth_features(flow: &MarketFlowTick) -> StealthFeatures {
    let total = total_volume(flow);
    if total <= f64::EPSILON {
        return StealthFeatures::default();
    }

    let avg_trade_share = clamp01(flow.avg_trade_size.max(0.0) / total);
    let fragmentation_from_count = clamp01(flow.trade_count as f64 / 200.0);
    let fragmentation_index = fragmentation_from_count * (1.0 - avg_trade_share);
    let execution_entropy = 1.0 - directional_strength(flow);
    let acceleration_ratio = clamp01(flow.flow_acceleration.abs() / total);
    let timing_jitter = 1.0 - acceleration_ratio;
    let price_impact = flow.price_move_pct.abs() / 100.0;
    let impact_dilution_ratio = clamp01(total / (total + price_impact * 100_000.0));

    StealthFeatures {
        fragmentation_index,
        execution_entropy,
        cross_exchange_sync: 1.0 - clamp01(flow.cross_exchange_dispersion),
        order_size_variance: 1.0 - avg_trade_share,
        timing_jitter,
        impact_dilution_ratio,
        cross_exchange_dispersion: clamp01(flow.cross_exchange_dispersion),
    }
}
