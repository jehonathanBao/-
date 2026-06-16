use serde::{Deserialize, Serialize};

use super::{
    feature_store::FeatureVector,
    flow_reality::{directional_strength, total_volume},
    types::{clamp01, MarketFlowTick, StealthState},
};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "snake_case")]
pub enum BreakoutBias {
    LongSqueeze,
    ShortSqueeze,
    #[default]
    Neutral,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct PriceLevel {
    /// Normalized price band. 1.0 means current reference price when the
    /// upstream tick does not yet carry an absolute mark price.
    pub price: f64,
    pub strength: f64,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct GLCEState {
    pub symbol: String,
    pub squeeze_probability: f64,
    pub liquidation_risk: f64,
    pub gamma_pressure: f64,
    pub breakout_bias: BreakoutBias,
    pub liquidity_bands: Vec<PriceLevel>,
}

pub struct GLCEEngine;

impl GLCEEngine {
    pub fn compute(flow: &MarketFlowTick, stealth: &StealthState) -> GLCEState {
        compute_glce_state(flow, stealth, 0.0)
    }

    pub fn compute_vector(vector: &FeatureVector, stealth: &StealthState) -> GLCEState {
        let zscore_pressure = clamp01(vector.flow_zscore.abs() / 10.0);
        compute_glce_state(&vector.tick, stealth, zscore_pressure)
    }
}

pub fn compute_glce_state(
    flow: &MarketFlowTick,
    stealth: &StealthState,
    zscore_pressure: f64,
) -> GLCEState {
    let gamma_pressure = compute_gamma_pressure(flow);
    let liquidation_risk = compute_liquidation_risk(flow, zscore_pressure);
    let squeeze_probability = squeeze_probability(gamma_pressure, liquidation_risk, stealth.gamma);
    let breakout_bias = breakout_bias(flow, squeeze_probability, gamma_pressure, liquidation_risk);
    let liquidity_bands =
        liquidity_bands(flow, breakout_bias, squeeze_probability, liquidation_risk);

    GLCEState {
        symbol: flow.symbol.clone(),
        squeeze_probability,
        liquidation_risk,
        gamma_pressure,
        breakout_bias,
        liquidity_bands,
    }
}

fn compute_gamma_pressure(flow: &MarketFlowTick) -> f64 {
    let total = total_volume(flow);
    let oi_pressure = if total <= f64::EPSILON {
        0.0
    } else {
        clamp01(flow.open_interest_delta.abs() / (total + 1.0))
    };
    let funding_rate_squeeze_factor = clamp01(flow.funding_rate.abs() * 1_000.0);
    let liquidity_imbalance = directional_strength(flow);

    clamp01(oi_pressure * 0.45 + funding_rate_squeeze_factor * 0.25 + liquidity_imbalance * 0.30)
}

fn compute_liquidation_risk(flow: &MarketFlowTick, zscore_pressure: f64) -> f64 {
    let price_proximity_proxy = clamp01(flow.price_move_pct.abs() / 1.0);
    let vol_spike = clamp01(flow.realized_vol);
    let dynamic_pressure = clamp01(flow.dynamic_multiple / 10.0);
    let liquidation_cluster_pressure = clamp01(flow.liquidation_pressure);

    clamp01(
        liquidation_cluster_pressure * 0.40
            + price_proximity_proxy * 0.20
            + vol_spike * 0.18
            + dynamic_pressure * 0.12
            + zscore_pressure * 0.10,
    )
}

fn squeeze_probability(gamma_pressure: f64, liquidation_risk: f64, stealth_gamma: f64) -> f64 {
    let raw = gamma_pressure * 0.40 + liquidation_risk * 0.40 + clamp01(stealth_gamma) * 0.20;
    sigmoid((raw - 0.45) * 7.0)
}

fn sigmoid(value: f64) -> f64 {
    if !value.is_finite() {
        return 0.0;
    }
    1.0 / (1.0 + (-value).exp())
}

fn breakout_bias(
    flow: &MarketFlowTick,
    squeeze_probability: f64,
    gamma_pressure: f64,
    liquidation_risk: f64,
) -> BreakoutBias {
    if squeeze_probability < 0.55 && gamma_pressure < 0.45 && liquidation_risk < 0.45 {
        return BreakoutBias::Neutral;
    }
    if flow.net_flow > 0.0 {
        BreakoutBias::LongSqueeze
    } else if flow.net_flow < 0.0 {
        BreakoutBias::ShortSqueeze
    } else {
        BreakoutBias::Neutral
    }
}

fn liquidity_bands(
    flow: &MarketFlowTick,
    bias: BreakoutBias,
    squeeze_probability: f64,
    liquidation_risk: f64,
) -> Vec<PriceLevel> {
    let base_offset = (flow.realized_vol.abs() * 0.01)
        .max(flow.price_move_pct.abs() * 0.002)
        .clamp(0.001, 0.025);
    let primary_strength = clamp01(squeeze_probability * 0.65 + liquidation_risk * 0.35);
    let secondary_strength = clamp01(primary_strength * 0.72);

    match bias {
        BreakoutBias::LongSqueeze => vec![
            PriceLevel {
                price: 1.0 + base_offset,
                strength: primary_strength,
            },
            PriceLevel {
                price: 1.0 + base_offset * 2.0,
                strength: secondary_strength,
            },
        ],
        BreakoutBias::ShortSqueeze => vec![
            PriceLevel {
                price: 1.0 - base_offset,
                strength: primary_strength,
            },
            PriceLevel {
                price: 1.0 - base_offset * 2.0,
                strength: secondary_strength,
            },
        ],
        BreakoutBias::Neutral => vec![PriceLevel {
            price: 1.0,
            strength: primary_strength,
        }],
    }
}
