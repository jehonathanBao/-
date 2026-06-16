use serde::{Deserialize, Serialize};

use super::{
    gex::{DealerBias, GammaExposureState},
    glce::GLCEState,
    lhcs::{CascadeDirection, LHCSState},
    types::{clamp01, Direction, MarketFlowTick},
};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, Default)]
#[serde(rename_all = "snake_case")]
pub enum MarketRegime {
    Stable,
    Compression,
    FragileAccumulation,
    FragileDistribution,
    CriticalInstability,
    #[default]
    Unknown,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct MarketForceField {
    pub symbol: String,
    pub total_stress: f64,
    pub liquidity_field: f64,
    pub gamma_field: f64,
    pub liquidation_field: f64,
    pub cascade_field: f64,
    pub directional_bias: Direction,
    pub instability_index: f64,
    pub regime_state: MarketRegime,
}

pub struct MarketForceFieldEngine;

impl MarketForceFieldEngine {
    pub fn compute(
        flow: &MarketFlowTick,
        glce: &GLCEState,
        lhcs: &LHCSState,
        gex: &GammaExposureState,
    ) -> MarketForceField {
        compute_force_field(flow, glce, lhcs, gex)
    }
}

pub fn compute_force_field(
    flow: &MarketFlowTick,
    glce: &GLCEState,
    lhcs: &LHCSState,
    gex: &GammaExposureState,
) -> MarketForceField {
    let liquidity_field = liquidity_field(flow);
    let gamma_field = gamma_field(gex);
    let liquidation_field = liquidation_field(glce);
    let cascade_field = cascade_field(lhcs);

    let total_stress = clamp01(
        liquidity_field * 0.30
            + gamma_field * 0.25
            + liquidation_field * 0.25
            + cascade_field * 0.20,
    );
    let directional_bias = infer_direction(flow, gex, lhcs);
    let instability_index = clamp01(total_stress * 1.35 + volatility_instability(flow) * 0.20);
    let regime_state = classify_regime(total_stress, directional_bias, gex, glce, lhcs);

    MarketForceField {
        symbol: flow.symbol.clone(),
        total_stress,
        liquidity_field,
        gamma_field,
        liquidation_field,
        cascade_field,
        directional_bias,
        instability_index,
        regime_state,
    }
}

fn liquidity_field(flow: &MarketFlowTick) -> f64 {
    let total = (flow.buy_volume + flow.sell_volume).max(1.0);
    let order_flow_imbalance = clamp01(flow.net_flow.abs() / total);
    let acceleration = clamp01(flow.flow_acceleration.abs() / (total + 1.0));
    let spread_pressure_proxy = clamp01(flow.realized_vol);
    let depth_decay_proxy = clamp01(flow.dynamic_multiple / 10.0);

    clamp01(
        order_flow_imbalance * 0.35
            + acceleration * 0.25
            + spread_pressure_proxy * 0.20
            + depth_decay_proxy * 0.20,
    )
}

fn gamma_field(gex: &GammaExposureState) -> f64 {
    let gex_pressure = clamp01(gex.total_gex.abs() / 10_000.0);
    let dealer_hedging_pressure = match gex.dealer_position_bias {
        DealerBias::SellRallies => 1.0,
        DealerBias::BuyDips => 0.55,
        DealerBias::Neutral => 0.20,
    };

    clamp01(
        gex_pressure * 0.42
            + gex.squeeze_probability * 0.30
            + gex.price_pin_pressure_index * 0.16
            + dealer_hedging_pressure * 0.12,
    )
}

fn liquidation_field(glce: &GLCEState) -> f64 {
    clamp01(
        glce.liquidation_risk * 0.45 + glce.gamma_pressure * 0.25 + glce.squeeze_probability * 0.30,
    )
}

fn cascade_field(lhcs: &LHCSState) -> f64 {
    let propagation_strength = lhcs
        .cascade_state
        .propagation_chain
        .iter()
        .map(|step| clamp01(step.impact_amplification / 3.5))
        .fold(0.0, f64::max);
    let void_depth = lhcs
        .liquidity_void_zones
        .iter()
        .map(|zone| zone.risk_score)
        .fold(0.0, f64::max);

    clamp01(
        lhcs.cascade_state.cascade_probability * 0.50
            + propagation_strength * 0.30
            + void_depth * 0.20,
    )
}

fn infer_direction(flow: &MarketFlowTick, gex: &GammaExposureState, lhcs: &LHCSState) -> Direction {
    let mut buy_pressure = if flow.net_flow > 0.0 {
        clamp01(flow.net_flow.abs() / (flow.buy_volume + flow.sell_volume + 1.0))
    } else {
        0.0
    };
    let mut sell_pressure = if flow.net_flow < 0.0 {
        clamp01(flow.net_flow.abs() / (flow.buy_volume + flow.sell_volume + 1.0))
    } else {
        0.0
    };

    match lhcs.cascade_state.direction_bias {
        CascadeDirection::UpwardSqueeze => buy_pressure += 0.25,
        CascadeDirection::DownwardSqueeze => sell_pressure += 0.25,
        CascadeDirection::Neutral => {}
    }
    match gex.dealer_position_bias {
        DealerBias::SellRallies if flow.price_move_pct > 0.0 => sell_pressure += 0.15,
        DealerBias::SellRallies if flow.price_move_pct < 0.0 => sell_pressure += 0.10,
        DealerBias::BuyDips if flow.price_move_pct < 0.0 => buy_pressure += 0.15,
        DealerBias::BuyDips if flow.price_move_pct > 0.0 => buy_pressure += 0.10,
        DealerBias::Neutral | DealerBias::SellRallies | DealerBias::BuyDips => {}
    }

    if buy_pressure > sell_pressure * 1.20 && buy_pressure >= 0.15 {
        Direction::Buy
    } else if sell_pressure > buy_pressure * 1.20 && sell_pressure >= 0.15 {
        Direction::Sell
    } else {
        Direction::Neutral
    }
}

fn classify_regime(
    total_stress: f64,
    direction: Direction,
    gex: &GammaExposureState,
    glce: &GLCEState,
    lhcs: &LHCSState,
) -> MarketRegime {
    if total_stress >= 0.78 || lhcs.cascade_state.cascade_probability >= 0.82 {
        return MarketRegime::CriticalInstability;
    }
    if total_stress >= 0.58 {
        return match direction {
            Direction::Buy if gex.dealer_position_bias == DealerBias::BuyDips => {
                MarketRegime::FragileAccumulation
            }
            Direction::Sell if glce.liquidation_risk >= 0.50 => MarketRegime::FragileDistribution,
            Direction::Buy => MarketRegime::FragileAccumulation,
            Direction::Sell => MarketRegime::FragileDistribution,
            Direction::Absorption | Direction::Suppression | Direction::Neutral => {
                MarketRegime::Compression
            }
        };
    }
    if total_stress >= 0.35 || gex.price_pin_pressure_index >= 0.65 {
        MarketRegime::Compression
    } else {
        MarketRegime::Stable
    }
}

fn volatility_instability(flow: &MarketFlowTick) -> f64 {
    clamp01(flow.realized_vol * 0.55 + flow.price_move_pct.abs() * 0.45)
}
