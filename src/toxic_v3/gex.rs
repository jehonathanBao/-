use serde::{Deserialize, Serialize};

use super::{
    glce::GLCEState,
    lhcs::{CascadeDirection, LHCSState},
    types::{clamp01, MarketFlowTick},
};

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct OptionsSurface {
    pub symbol: String,
    /// Normalized underlying reference. 1.0 means current price until an
    /// upstream absolute options mark is available.
    pub underlying_price: f64,
    pub strikes: Vec<OptionStrike>,
}

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct OptionStrike {
    pub strike: f64,
    pub call_oi: f64,
    pub put_oi: f64,
    pub gamma: f64,
    pub delta: f64,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct GammaExposureState {
    pub symbol: String,
    pub total_gex: f64,
    pub gamma_wall_levels: Vec<GammaWall>,
    pub max_pain: f64,
    pub dealer_position_bias: DealerBias,
    pub squeeze_probability: f64,
    pub price_pin_pressure_index: f64,
}

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct GammaWall {
    pub strike: f64,
    pub gamma_exposure: f64,
    pub call_put_imbalance: f64,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "snake_case")]
pub enum DealerBias {
    BuyDips,
    SellRallies,
    #[default]
    Neutral,
}

pub struct GEXEngine;

impl GEXEngine {
    pub fn compute_from_tick(
        flow: &MarketFlowTick,
        glce: &GLCEState,
        lhcs: &LHCSState,
    ) -> GammaExposureState {
        let surface = proxy_surface_from_tick(flow, glce, lhcs);
        Self::compute_from_surface(&surface, glce, lhcs)
    }

    pub fn compute_from_surface(
        surface: &OptionsSurface,
        glce: &GLCEState,
        lhcs: &LHCSState,
    ) -> GammaExposureState {
        compute_gex(surface, glce, lhcs)
    }
}

pub fn compute_gex(
    surface: &OptionsSurface,
    glce: &GLCEState,
    lhcs: &LHCSState,
) -> GammaExposureState {
    let mut total_gex = 0.0;
    let mut walls = Vec::with_capacity(surface.strikes.len());

    for strike in &surface.strikes {
        let gex = (strike.call_oi - strike.put_oi) * strike.gamma * strike.delta.abs();
        total_gex += gex;
        walls.push(GammaWall {
            strike: strike.strike,
            gamma_exposure: gex,
            call_put_imbalance: strike.call_oi - strike.put_oi,
        });
    }

    walls.sort_by(|left, right| {
        right
            .gamma_exposure
            .abs()
            .partial_cmp(&left.gamma_exposure.abs())
            .unwrap_or(std::cmp::Ordering::Equal)
    });
    walls.truncate(5);

    let max_pain = compute_max_pain(surface);
    let dealer_position_bias = compute_dealer_bias(total_gex);
    let price_pin_pressure_index = compute_price_pin_pressure(surface, max_pain, total_gex);
    let squeeze_probability =
        compute_squeeze_probability(total_gex, &walls, glce, lhcs, price_pin_pressure_index);

    GammaExposureState {
        symbol: surface.symbol.clone(),
        total_gex,
        gamma_wall_levels: walls,
        max_pain,
        dealer_position_bias,
        squeeze_probability,
        price_pin_pressure_index,
    }
}

pub fn compute_max_pain(surface: &OptionsSurface) -> f64 {
    surface
        .strikes
        .iter()
        .min_by(|left, right| {
            total_payout(surface, left.strike)
                .partial_cmp(&total_payout(surface, right.strike))
                .unwrap_or(std::cmp::Ordering::Equal)
        })
        .map(|strike| strike.strike)
        .unwrap_or(surface.underlying_price.max(1.0))
}

pub fn compute_dealer_bias(gex: f64) -> DealerBias {
    if gex > 0.5 {
        DealerBias::BuyDips
    } else if gex < -0.5 {
        DealerBias::SellRallies
    } else {
        DealerBias::Neutral
    }
}

pub fn compute_squeeze_probability(
    total_gex: f64,
    walls: &[GammaWall],
    glce: &GLCEState,
    lhcs: &LHCSState,
    price_pin_pressure_index: f64,
) -> f64 {
    let wall_density = gamma_wall_density(walls);
    let gex_pressure = clamp01(total_gex.abs() / 10_000.0);
    let cascade_pressure = lhcs.cascade_state.cascade_probability;
    let direction_alignment = direction_alignment(total_gex, lhcs.cascade_state.direction_bias);
    let raw = gex_pressure * 0.24
        + glce.squeeze_probability * 0.23
        + cascade_pressure * 0.23
        + wall_density * 0.18
        + direction_alignment * 0.07
        + price_pin_pressure_index * 0.05;

    sigmoid((raw - 0.42) * 7.0)
}

fn proxy_surface_from_tick(
    flow: &MarketFlowTick,
    glce: &GLCEState,
    lhcs: &LHCSState,
) -> OptionsSurface {
    let step = (flow.realized_vol.abs() * 0.006)
        .max(flow.price_move_pct.abs() * 0.0015)
        .clamp(0.001, 0.018);
    let total_flow = (flow.buy_volume + flow.sell_volume).max(1.0);
    let directional_strength = clamp01(flow.net_flow.abs() / total_flow);
    let base_oi = flow
        .open_interest_delta
        .abs()
        .max(total_flow * 0.05)
        .max(1.0);
    let bias = if flow.net_flow >= 0.0 { 1.0 } else { -1.0 };
    let squeeze = glce
        .squeeze_probability
        .max(lhcs.cascade_state.cascade_probability);

    let strikes = (-4..=4)
        .map(|index| {
            let distance = index as f64;
            let strike = 1.0 + distance * step;
            let near_weight = 1.0 / (1.0 + distance.abs());
            let wall_weight = clamp01(near_weight * 0.55 + squeeze * 0.45);
            let side_weight = if distance.signum() == bias {
                1.0 + directional_strength
            } else {
                (1.0 - directional_strength * 0.35).max(0.1)
            };
            let call_oi = base_oi * wall_weight * if bias > 0.0 { side_weight } else { 0.75 };
            let put_oi = base_oi * wall_weight * if bias < 0.0 { side_weight } else { 0.75 };
            let gamma = clamp01(0.18 + near_weight * 0.62 + squeeze * 0.20);
            let delta = (0.50 + distance.signum() * 0.12).clamp(0.10, 0.90);

            OptionStrike {
                strike,
                call_oi,
                put_oi,
                gamma,
                delta,
            }
        })
        .collect();

    OptionsSurface {
        symbol: flow.symbol.clone(),
        underlying_price: 1.0,
        strikes,
    }
}

fn total_payout(surface: &OptionsSurface, settlement: f64) -> f64 {
    surface
        .strikes
        .iter()
        .map(|strike| {
            let call_payout = (settlement - strike.strike).max(0.0) * strike.call_oi;
            let put_payout = (strike.strike - settlement).max(0.0) * strike.put_oi;
            call_payout + put_payout
        })
        .sum()
}

fn compute_price_pin_pressure(surface: &OptionsSurface, max_pain: f64, total_gex: f64) -> f64 {
    let distance = (surface.underlying_price - max_pain).abs();
    let distance_pressure = clamp01(1.0 - distance / surface.underlying_price.max(1.0));
    let gex_pressure = clamp01(total_gex.abs() / 10_000.0);
    clamp01(distance_pressure * 0.55 + gex_pressure * 0.45)
}

fn gamma_wall_density(walls: &[GammaWall]) -> f64 {
    let total = walls
        .iter()
        .map(|wall| wall.gamma_exposure.abs())
        .sum::<f64>();
    if total <= f64::EPSILON {
        return 0.0;
    }
    let top = walls
        .iter()
        .take(3)
        .map(|wall| wall.gamma_exposure.abs())
        .sum::<f64>();
    clamp01(top / total)
}

fn direction_alignment(total_gex: f64, cascade_direction: CascadeDirection) -> f64 {
    match (total_gex, cascade_direction) {
        (value, CascadeDirection::UpwardSqueeze) if value < -0.5 => 1.0,
        (value, CascadeDirection::DownwardSqueeze) if value < -0.5 => 1.0,
        (value, CascadeDirection::UpwardSqueeze) if value > 0.5 => 0.45,
        (value, CascadeDirection::DownwardSqueeze) if value > 0.5 => 0.45,
        (_, CascadeDirection::Neutral) => 0.20,
        _ => 0.35,
    }
}

fn sigmoid(value: f64) -> f64 {
    if !value.is_finite() {
        return 0.0;
    }
    1.0 / (1.0 + (-value).exp())
}
