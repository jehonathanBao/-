use std::collections::BTreeMap;

use crate::contract_whale_monitor::types::{
    ContractWhaleLiquidityBehavior, ContractWhaleOpportunityZone, ContractWhaleRankedEvent,
    ContractWhaleRegimeSnapshot, ContractWhaleSignal,
};

use super::{liquidity::behavior_for_signal, strength::score_signal_strength};

pub fn derive_opportunity_map(
    items: &[ContractWhaleSignal],
    regime: &ContractWhaleRegimeSnapshot,
    liquidity_behaviors: &[ContractWhaleLiquidityBehavior],
    ranked_events: &[ContractWhaleRankedEvent],
) -> Vec<ContractWhaleOpportunityZone> {
    let mut zones = BTreeMap::<String, ContractWhaleOpportunityZone>::new();

    for behavior in liquidity_behaviors {
        let zone_type = behavior_zone_type(&behavior.behavior);
        let description = match behavior.behavior.as_str() {
            "absorption" => "承接区反复出现，适合观察主力是否继续吸收卖压。",
            "breakout_pressure" => "价格上方压力持续累积，等待后续是否形成确认突破。",
            "fake_breakout" => "冲高后跟随不足，假突破风险偏高。",
            "liquidity_sweep" => "流动性被快速抽走，需警惕清算式放大。",
            "distribution" => "分发痕迹增强，注意上方承接是否衰减。",
            _ => "订单块行为正在形成，适合继续观察结构是否确认。",
        };
        upsert_zone(
            &mut zones,
            ContractWhaleOpportunityZone {
                zone_type: zone_type.to_string(),
                label: behavior_label(zone_type).to_string(),
                low_price: behavior.low_price,
                high_price: behavior.high_price,
                range_label: behavior.range_label.clone(),
                strength_score: behavior.strength_score,
                description: description.to_string(),
            },
        );
    }

    for event in ranked_events {
        if let Some(signal) = items.iter().find(|item| item.id == event.signal_id) {
            let zone_type = behavior_zone_type(behavior_for_signal(signal));
            let (low_price, high_price, range_label) = signal_range(signal);
            upsert_zone(
                &mut zones,
                ContractWhaleOpportunityZone {
                    zone_type: zone_type.to_string(),
                    label: behavior_label(zone_type).to_string(),
                    low_price,
                    high_price,
                    range_label,
                    strength_score: score_signal_strength(signal),
                    description: format!(
                        "{} · {} · {}",
                        regime.regime,
                        event.event_type,
                        event.rationale
                    ),
                },
            );
        }
    }

    let mut values = zones.into_values().collect::<Vec<_>>();
    values.sort_by(|left, right| right.strength_score.cmp(&left.strength_score));
    values.truncate(4);
    values
}

fn upsert_zone(
    zones: &mut BTreeMap<String, ContractWhaleOpportunityZone>,
    candidate: ContractWhaleOpportunityZone,
) {
    match zones.get(&candidate.zone_type) {
        Some(existing) if existing.strength_score >= candidate.strength_score => {}
        _ => {
            zones.insert(candidate.zone_type.clone(), candidate);
        }
    }
}

fn behavior_zone_type(behavior: &str) -> &'static str {
    match behavior {
        "absorption" => "absorption_zone",
        "breakout_pressure" => "breakout_pressure_zone",
        "fake_breakout" => "fake_breakout_risk",
        "liquidity_sweep" => "liquidity_sweep_zone",
        "distribution" => "reversal_zone",
        _ => "order_block_zone",
    }
}

fn behavior_label(zone_type: &str) -> &'static str {
    match zone_type {
        "absorption_zone" => "Absorption Zone",
        "breakout_pressure_zone" => "Breakout Pressure",
        "fake_breakout_risk" => "Fake Breakout Risk",
        "liquidity_sweep_zone" => "Liquidity Sweep",
        "reversal_zone" => "Reversal Zone",
        _ => "Order Block",
    }
}

fn signal_range(signal: &ContractWhaleSignal) -> (f64, f64, String) {
    let anchor = signal
        .order_price_usd
        .or(signal.current_market_price_usd)
        .unwrap_or_default();
    if anchor <= 0.0 {
        return (0.0, 0.0, "N/A".to_string());
    }
    let band_pct = (signal.price_move_pct.unwrap_or(0.12).abs() / 100.0).clamp(0.0008, 0.0020);
    let low = round2(anchor * (1.0 - band_pct));
    let high = round2(anchor * (1.0 + band_pct));
    (low, high, format!("{:.0} - {:.0}", low, high))
}

fn round2(value: f64) -> f64 {
    (value * 100.0).round() / 100.0
}
