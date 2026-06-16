//! BTC-only liquidation intelligence aggregation.
//!
//! This module deliberately ignores non-BTC symbols. It is a read-only
//! inference layer for BTC liquidation pressure, squeeze probability, cluster
//! zones, and gamma influence. It never places orders or changes alert gates.

use serde::{Deserialize, Serialize};

use super::{
    gex::{DealerBias, GammaExposureState},
    glce::GLCEState,
    lhcs::{CascadeDirection, LHCSState, PriceZone},
    types::{clamp01, MarketFlowTick},
};

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct BTCLiquidationState {
    pub ts: i64,
    pub symbol: String,
    pub long_liquidation_pressure: f64,
    pub short_liquidation_pressure: f64,
    pub net_liquidation_bias: f64,
    pub squeeze_up_probability: f64,
    pub squeeze_down_probability: f64,
    pub liquidation_clusters: Vec<PriceZone>,
    pub cascade_risk: f64,
    pub gamma_pressure: f64,
    pub read_only: bool,
}

pub struct BTCLiquidationEngine;

impl BTCLiquidationEngine {
    pub fn compute(
        flow: &MarketFlowTick,
        glce: &GLCEState,
        lhcs: &LHCSState,
        gex: &GammaExposureState,
    ) -> Option<BTCLiquidationState> {
        if !is_btc_symbol(&flow.symbol) {
            return None;
        }

        let long_liquidation_pressure = long_liquidation_pressure(flow, glce);
        let short_liquidation_pressure = short_liquidation_pressure(flow, glce);
        let gamma_pressure = gamma_pressure(glce, gex);
        let liquidity_void_above = liquidity_void_above(lhcs);
        let liquidity_void_below = liquidity_void_below(lhcs);
        let gamma_wall_support = gamma_wall_support(gex, gamma_pressure);
        let gamma_wall_resistance = gamma_wall_resistance(gex, gamma_pressure);
        let squeeze_up_probability = squeeze_up_probability(
            short_liquidation_pressure,
            liquidity_void_above,
            gamma_wall_support,
            lhcs,
        );
        let squeeze_down_probability = squeeze_down_probability(
            long_liquidation_pressure,
            liquidity_void_below,
            gamma_wall_resistance,
            lhcs,
        );
        let liquidation_clusters = liquidation_clusters(lhcs);
        let cascade_risk = cascade_risk(flow, lhcs, &liquidation_clusters);

        Some(BTCLiquidationState {
            ts: flow.ts,
            symbol: "BTC".to_string(),
            long_liquidation_pressure,
            short_liquidation_pressure,
            net_liquidation_bias: short_liquidation_pressure - long_liquidation_pressure,
            squeeze_up_probability,
            squeeze_down_probability,
            liquidation_clusters,
            cascade_risk,
            gamma_pressure,
            read_only: true,
        })
    }
}

pub fn is_btc_symbol(symbol: &str) -> bool {
    let normalized = symbol
        .chars()
        .filter(|ch| ch.is_ascii_alphanumeric())
        .collect::<String>()
        .to_ascii_uppercase();
    matches!(
        normalized.as_str(),
        "BTC"
            | "BTCPERP"
            | "BTCSWAP"
            | "BTCFUTURES"
            | "BTCUSDT"
            | "BTCUSD"
            | "BTCUSDC"
            | "BTCF0USTF0"
            | "XBT"
            | "XBTPERP"
            | "XBTUSDT"
            | "XBTUSD"
    )
}

fn long_liquidation_pressure(flow: &MarketFlowTick, glce: &GLCEState) -> f64 {
    let long_crowding = long_crowding_proxy(flow);
    let downside_distance = downside_price_distance_proxy(flow);
    let volatility_factor = clamp01(flow.realized_vol);
    let sell_pressure = sell_flow_pressure(flow);

    clamp01(
        long_crowding * 0.32
            + downside_distance * 0.22
            + volatility_factor * 0.18
            + glce.liquidation_risk * 0.18
            + sell_pressure * 0.10,
    )
}

fn short_liquidation_pressure(flow: &MarketFlowTick, glce: &GLCEState) -> f64 {
    let short_crowding = short_crowding_proxy(flow);
    let upside_distance = upside_price_distance_proxy(flow);
    let volatility_factor = clamp01(flow.realized_vol);
    let buy_pressure = buy_flow_pressure(flow);

    clamp01(
        short_crowding * 0.32
            + upside_distance * 0.22
            + volatility_factor * 0.18
            + glce.liquidation_risk * 0.18
            + buy_pressure * 0.10,
    )
}

fn long_crowding_proxy(flow: &MarketFlowTick) -> f64 {
    let funding_long_bias = clamp01(flow.funding_rate.max(0.0) * 1_000.0);
    let oi_expansion = clamp01(flow.open_interest_delta.max(0.0) / total_volume(flow).max(1.0));
    clamp01(0.45 + funding_long_bias * 0.35 + oi_expansion * 0.20)
}

fn short_crowding_proxy(flow: &MarketFlowTick) -> f64 {
    let funding_short_bias = clamp01((-flow.funding_rate).max(0.0) * 1_000.0);
    let oi_expansion = clamp01(flow.open_interest_delta.max(0.0) / total_volume(flow).max(1.0));
    clamp01(0.45 + funding_short_bias * 0.35 + oi_expansion * 0.20)
}

fn downside_price_distance_proxy(flow: &MarketFlowTick) -> f64 {
    let realized_downside = clamp01((-flow.price_move_pct).max(0.0));
    let near_liquidation_band = clamp01(flow.dynamic_multiple / 10.0);
    clamp01(realized_downside * 0.45 + near_liquidation_band * 0.55)
}

fn upside_price_distance_proxy(flow: &MarketFlowTick) -> f64 {
    let realized_upside = clamp01(flow.price_move_pct.max(0.0));
    let near_liquidation_band = clamp01(flow.dynamic_multiple / 10.0);
    clamp01(realized_upside * 0.45 + near_liquidation_band * 0.55)
}

fn buy_flow_pressure(flow: &MarketFlowTick) -> f64 {
    let total = total_volume(flow).max(1.0);
    clamp01(flow.buy_volume / total)
}

fn sell_flow_pressure(flow: &MarketFlowTick) -> f64 {
    let total = total_volume(flow).max(1.0);
    clamp01(flow.sell_volume / total)
}

fn total_volume(flow: &MarketFlowTick) -> f64 {
    flow.buy_volume + flow.sell_volume
}

fn liquidity_void_above(lhcs: &LHCSState) -> f64 {
    lhcs.liquidity_void_zones
        .iter()
        .filter(|zone| zone.lower >= 1.0 || zone.upper >= 1.0)
        .map(|zone| zone.risk_score)
        .fold(0.0, f64::max)
}

fn liquidity_void_below(lhcs: &LHCSState) -> f64 {
    lhcs.liquidity_void_zones
        .iter()
        .filter(|zone| zone.lower <= 1.0 || zone.upper <= 1.0)
        .map(|zone| zone.risk_score)
        .fold(0.0, f64::max)
}

fn gamma_pressure(glce: &GLCEState, gex: &GammaExposureState) -> f64 {
    clamp01(
        glce.gamma_pressure * 0.35
            + gex.squeeze_probability * 0.30
            + gex.price_pin_pressure_index * 0.15
            + clamp01(gex.total_gex.abs() / 10_000.0) * 0.20,
    )
}

fn gamma_wall_support(gex: &GammaExposureState, gamma_pressure: f64) -> f64 {
    let dealer_support = match gex.dealer_position_bias {
        DealerBias::BuyDips => 0.72,
        DealerBias::SellRallies => 0.42,
        DealerBias::Neutral => 0.50,
    };
    clamp01(gamma_pressure * 0.55 + dealer_support * 0.45)
}

fn gamma_wall_resistance(gex: &GammaExposureState, gamma_pressure: f64) -> f64 {
    let dealer_resistance = match gex.dealer_position_bias {
        DealerBias::SellRallies => 0.72,
        DealerBias::BuyDips => 0.42,
        DealerBias::Neutral => 0.50,
    };
    clamp01(gamma_pressure * 0.55 + dealer_resistance * 0.45)
}

fn squeeze_up_probability(
    short_pressure: f64,
    liquidity_void_above: f64,
    gamma_wall_support: f64,
    lhcs: &LHCSState,
) -> f64 {
    let directional_boost = if lhcs.cascade_state.direction_bias == CascadeDirection::UpwardSqueeze
    {
        0.12
    } else {
        0.0
    };
    clamp01(
        short_pressure * 0.42
            + liquidity_void_above * 0.22
            + gamma_wall_support * 0.24
            + lhcs.cascade_state.cascade_probability * 0.12
            + directional_boost,
    )
}

fn squeeze_down_probability(
    long_pressure: f64,
    liquidity_void_below: f64,
    gamma_wall_resistance: f64,
    lhcs: &LHCSState,
) -> f64 {
    let directional_boost =
        if lhcs.cascade_state.direction_bias == CascadeDirection::DownwardSqueeze {
            0.12
        } else {
            0.0
        };
    clamp01(
        long_pressure * 0.42
            + liquidity_void_below * 0.22
            + gamma_wall_resistance * 0.24
            + lhcs.cascade_state.cascade_probability * 0.12
            + directional_boost,
    )
}

fn liquidation_clusters(lhcs: &LHCSState) -> Vec<PriceZone> {
    let mut clusters = lhcs
        .liquidation_heatmap
        .high_risk_zones
        .iter()
        .copied()
        .collect::<Vec<_>>();
    clusters.extend(lhcs.liquidity_void_zones.iter().copied());
    clusters.sort_by(|left, right| {
        right
            .risk_score
            .partial_cmp(&left.risk_score)
            .unwrap_or(std::cmp::Ordering::Equal)
    });
    clusters.truncate(8);
    clusters
}

fn cascade_risk(flow: &MarketFlowTick, lhcs: &LHCSState, clusters: &[PriceZone]) -> f64 {
    let max_cluster_density = clusters
        .iter()
        .map(|zone| zone.risk_score)
        .fold(0.0, f64::max);
    let leverage_concentration = clamp01(
        flow.liquidation_pressure * 0.55
            + clamp01(flow.open_interest_delta.abs() / total_volume(flow).max(1.0)) * 0.45,
    );
    let volatility_expansion =
        clamp01(flow.realized_vol * 0.65 + flow.dynamic_multiple / 10.0 * 0.35);

    clamp01(
        max_cluster_density * 0.32
            + leverage_concentration * 0.28
            + volatility_expansion * 0.20
            + lhcs.cascade_state.cascade_probability * 0.20,
    )
}
