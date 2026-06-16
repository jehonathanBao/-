use serde::{Deserialize, Serialize};

use super::{
    glce::{BreakoutBias, GLCEState},
    types::{clamp01, MarketFlowTick},
};

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct LiquidationHeatmap {
    pub symbol: String,
    pub price_bins: Vec<PriceBin>,
    pub density_map: Vec<f64>,
    pub high_risk_zones: Vec<PriceZone>,
}

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct PriceBin {
    /// Normalized price level. 1.0 is the current reference price until the
    /// upstream flow tick carries an absolute mark price.
    pub price: f64,
    pub leverage_density: f64,
    pub liquidation_volume: f64,
}

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct PriceZone {
    pub lower: f64,
    pub upper: f64,
    pub risk_score: f64,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct CascadeState {
    pub cascade_probability: f64,
    pub direction_bias: CascadeDirection,
    pub propagation_chain: Vec<CascadeStep>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "snake_case")]
pub enum CascadeDirection {
    UpwardSqueeze,
    DownwardSqueeze,
    #[default]
    Neutral,
}

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct CascadeStep {
    pub price_level: f64,
    pub expected_liquidation: f64,
    pub impact_amplification: f64,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct LHCSState {
    pub symbol: String,
    pub liquidation_heatmap: LiquidationHeatmap,
    pub cascade_state: CascadeState,
    pub liquidity_void_zones: Vec<PriceZone>,
    pub trigger_levels: Vec<PriceLevelTrigger>,
}

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct PriceLevelTrigger {
    pub price: f64,
    pub probability: f64,
}

pub struct LHCSEngine;

impl LHCSEngine {
    pub fn compute(flow: &MarketFlowTick, glce: &GLCEState) -> LHCSState {
        compute_lhcs_state(flow, glce)
    }
}

pub fn compute_lhcs_state(flow: &MarketFlowTick, glce: &GLCEState) -> LHCSState {
    let liquidation_heatmap = build_heatmap(flow, glce);
    let direction_bias = determine_direction(&liquidation_heatmap, glce);
    let liquidity_void_zones = detect_liquidity_voids(&liquidation_heatmap);
    let trigger_levels = trigger_levels(&liquidation_heatmap, direction_bias);
    let cascade_probability = compute_cascade_probability(glce, &liquidation_heatmap, flow);
    let propagation_chain = simulate_cascade(&liquidation_heatmap, direction_bias);

    LHCSState {
        symbol: flow.symbol.clone(),
        liquidation_heatmap,
        cascade_state: CascadeState {
            cascade_probability,
            direction_bias,
            propagation_chain,
        },
        liquidity_void_zones,
        trigger_levels,
    }
}

pub fn build_heatmap(flow: &MarketFlowTick, glce: &GLCEState) -> LiquidationHeatmap {
    let mut price_bins = Vec::with_capacity(11);
    let step = grid_step(flow);
    let direction_skew = clamp01(flow.net_flow.abs() / (flow.buy_volume + flow.sell_volume + 1.0));

    for index in -5..=5 {
        let distance = index as f64;
        let price = 1.0 + distance * step;
        let proximity = 1.0 / (1.0 + distance.abs());
        let side_bias = side_bias(index, flow.net_flow, direction_skew);
        let leverage_density =
            liquidation_density(flow, glce, proximity, side_bias, distance.abs());

        price_bins.push(PriceBin {
            price,
            leverage_density,
            liquidation_volume: leverage_density * flow.avg_trade_size.max(1.0),
        });
    }

    let density_map = price_bins
        .iter()
        .map(|bin| bin.leverage_density)
        .collect::<Vec<_>>();
    let high_risk_zones = detect_clusters(&price_bins);

    LiquidationHeatmap {
        symbol: flow.symbol.clone(),
        price_bins,
        density_map,
        high_risk_zones,
    }
}

pub fn compute_cascade_probability(
    glce: &GLCEState,
    heatmap: &LiquidationHeatmap,
    flow: &MarketFlowTick,
) -> f64 {
    let density_spike = max_density(&heatmap.price_bins);
    let void_pressure = detect_liquidity_voids(heatmap)
        .iter()
        .map(|zone| zone.risk_score)
        .fold(0.0, f64::max);
    let raw = glce.gamma_pressure * 0.30
        + glce.liquidation_risk * 0.25
        + density_spike * 0.25
        + clamp01(flow.realized_vol) * 0.12
        + void_pressure * 0.08;

    sigmoid((raw - 0.42) * 7.5)
}

pub fn simulate_cascade(
    heatmap: &LiquidationHeatmap,
    direction_bias: CascadeDirection,
) -> Vec<CascadeStep> {
    let mut candidates = heatmap
        .price_bins
        .iter()
        .filter(|bin| match direction_bias {
            CascadeDirection::UpwardSqueeze => bin.price >= 1.0,
            CascadeDirection::DownwardSqueeze => bin.price <= 1.0,
            CascadeDirection::Neutral => true,
        })
        .copied()
        .collect::<Vec<_>>();
    candidates.sort_by(|left, right| {
        right
            .leverage_density
            .partial_cmp(&left.leverage_density)
            .unwrap_or(std::cmp::Ordering::Equal)
    });

    let mut impact = 1.0;
    let mut path = Vec::new();
    for level in candidates {
        if level.leverage_density < 0.30 {
            continue;
        }
        impact = (impact * (1.0 + level.leverage_density * 0.35)).min(3.5);
        path.push(CascadeStep {
            price_level: level.price,
            expected_liquidation: level.liquidation_volume,
            impact_amplification: impact,
        });
        if impact >= 3.0 || path.len() >= 5 {
            break;
        }
    }
    path
}

pub fn determine_direction(heatmap: &LiquidationHeatmap, glce: &GLCEState) -> CascadeDirection {
    let upper_liq = heatmap
        .price_bins
        .iter()
        .filter(|bin| bin.price > 1.0)
        .map(|bin| bin.leverage_density)
        .sum::<f64>();
    let lower_liq = heatmap
        .price_bins
        .iter()
        .filter(|bin| bin.price < 1.0)
        .map(|bin| bin.leverage_density)
        .sum::<f64>();

    match glce.breakout_bias {
        BreakoutBias::LongSqueeze if upper_liq >= lower_liq * 0.90 => {
            CascadeDirection::UpwardSqueeze
        }
        BreakoutBias::ShortSqueeze if lower_liq >= upper_liq * 0.90 => {
            CascadeDirection::DownwardSqueeze
        }
        _ if upper_liq > lower_liq * 1.35 => CascadeDirection::UpwardSqueeze,
        _ if lower_liq > upper_liq * 1.35 => CascadeDirection::DownwardSqueeze,
        _ => CascadeDirection::Neutral,
    }
}

fn liquidation_density(
    flow: &MarketFlowTick,
    glce: &GLCEState,
    proximity: f64,
    side_bias: f64,
    distance: f64,
) -> f64 {
    let oi_pressure =
        clamp01(flow.open_interest_delta.abs() / (flow.buy_volume + flow.sell_volume + 1.0));
    let liquidation_pressure = clamp01(flow.liquidation_pressure);
    let volatility_weight = clamp01(flow.realized_vol);
    let leverage_cluster = clamp01(0.45 + proximity * 0.30 + side_bias * 0.25);
    let distance_weight = clamp01(1.0 - distance * 0.10);

    clamp01(
        oi_pressure * 0.22
            + liquidation_pressure * 0.25
            + volatility_weight * 0.16
            + glce.gamma_pressure * 0.17
            + glce.liquidation_risk * 0.14
            + leverage_cluster * distance_weight * 0.06,
    )
}

fn side_bias(index: i32, net_flow: f64, direction_skew: f64) -> f64 {
    if index == 0 {
        return 0.5;
    }
    if net_flow > 0.0 && index > 0 {
        0.5 + direction_skew * 0.5
    } else if net_flow < 0.0 && index < 0 {
        0.5 + direction_skew * 0.5
    } else {
        (0.5 - direction_skew * 0.25).max(0.0)
    }
}

fn grid_step(flow: &MarketFlowTick) -> f64 {
    (flow.realized_vol.abs() * 0.006)
        .max(flow.price_move_pct.abs() * 0.0015)
        .clamp(0.001, 0.018)
}

fn detect_clusters(bins: &[PriceBin]) -> Vec<PriceZone> {
    let max_density = max_density(bins);
    if max_density <= f64::EPSILON {
        return Vec::new();
    }
    bins.iter()
        .filter(|bin| bin.leverage_density >= (max_density * 0.72).max(0.45))
        .map(|bin| PriceZone {
            lower: bin.price - 0.0008,
            upper: bin.price + 0.0008,
            risk_score: bin.leverage_density,
        })
        .collect()
}

fn detect_liquidity_voids(heatmap: &LiquidationHeatmap) -> Vec<PriceZone> {
    let bins = &heatmap.price_bins;
    bins.windows(3)
        .filter_map(|window| {
            let left = window[0];
            let middle = window[1];
            let right = window[2];
            let neighbor_density = (left.leverage_density + right.leverage_density) / 2.0;
            if neighbor_density >= 0.35 && middle.leverage_density <= neighbor_density * 0.45 {
                Some(PriceZone {
                    lower: left.price.min(middle.price),
                    upper: right.price.max(middle.price),
                    risk_score: clamp01(neighbor_density - middle.leverage_density),
                })
            } else {
                None
            }
        })
        .collect()
}

fn trigger_levels(
    heatmap: &LiquidationHeatmap,
    direction_bias: CascadeDirection,
) -> Vec<PriceLevelTrigger> {
    let mut levels = heatmap
        .price_bins
        .iter()
        .filter(|bin| match direction_bias {
            CascadeDirection::UpwardSqueeze => bin.price >= 1.0,
            CascadeDirection::DownwardSqueeze => bin.price <= 1.0,
            CascadeDirection::Neutral => true,
        })
        .map(|bin| PriceLevelTrigger {
            price: bin.price,
            probability: bin.leverage_density,
        })
        .collect::<Vec<_>>();
    levels.sort_by(|left, right| {
        right
            .probability
            .partial_cmp(&left.probability)
            .unwrap_or(std::cmp::Ordering::Equal)
    });
    levels.truncate(3);
    levels
}

fn max_density(bins: &[PriceBin]) -> f64 {
    bins.iter()
        .map(|bin| bin.leverage_density)
        .fold(0.0, f64::max)
}

fn sigmoid(value: f64) -> f64 {
    if !value.is_finite() {
        return 0.0;
    }
    1.0 / (1.0 + (-value).exp())
}
