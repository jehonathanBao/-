//! Read-only BTC liquidation dashboard projection.
//!
//! The dashboard is a view model over the existing BTC-only liquidation,
//! GLCE, LHCS, GEX, and market-force engines. It does not place orders,
//! mutate alert gates, or write storage.

use serde::{Deserialize, Serialize};

use crate::types::flow::{FlowState, FlowWindow};

use super::{
    btc_liquidation::{is_btc_symbol, BTCLiquidationEngine},
    gex::GEXEngine,
    glce::GLCEEngine,
    lhcs::LHCSEngine,
    mff::{MarketForceField, MarketForceFieldEngine},
    stealth::StealthEngine,
    types::{clamp01, Direction, MarketFlowExchange, MarketFlowTick},
};

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct BTCLiquidationDashboard {
    pub ts: i64,
    pub symbol: String,
    pub current_price_usd: Option<f64>,
    pub data_status: String,
    pub read_only: bool,
    pub live: bool,
    pub force_field: DashboardForceFieldState,
    pub market_stress: MarketStressOverview,
    pub liquidation_heatmap: Vec<LiqLevel>,
    pub gamma_walls: Vec<DashboardGammaWall>,
    pub squeeze: SqueezeDirectionPanel,
    pub cascade_timeline: Vec<CascadeTimelinePoint>,
    pub liquidity_map: Vec<DashboardLiquidityLevel>,
    pub sources: DashboardDataSources,
    pub notes: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct MarketStressOverview {
    pub stress_score: f64,
    pub liquidity_field: f64,
    pub gamma_field: f64,
    pub liquidation_field: f64,
    pub cascade_field: f64,
    pub instability_index: f64,
    pub directional_bias: String,
    pub regime: String,
    pub cascade_risk: f64,
    pub gamma_pressure: f64,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct DashboardForceFieldState {
    pub ts: i64,
    pub symbol: String,
    pub liquidity_field: f64,
    pub gamma_field: f64,
    pub liquidation_field: f64,
    pub cascade_field: f64,
    pub total_stress: f64,
    pub instability_index: f64,
    pub next_move_bias: String,
    pub squeeze_probability: f64,
    pub cascade_probability: f64,
    pub predicted_regime: String,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct LiqLevel {
    pub price_usd: Option<f64>,
    pub normalized_price: f64,
    pub side: String,
    pub leverage_density: f64,
    pub liquidation_volume: f64,
    pub risk_score: f64,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct DashboardGammaWall {
    pub strike_usd: Option<f64>,
    pub normalized_strike: f64,
    pub gamma_exposure: f64,
    pub call_put_imbalance: f64,
    pub role: String,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct SqueezeDirectionPanel {
    pub up_probability: f64,
    pub down_probability: f64,
    pub dominant_direction: String,
    pub breakout_bias: String,
    pub net_liquidation_bias: f64,
    pub long_liquidation_pressure: f64,
    pub short_liquidation_pressure: f64,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct CascadeTimelinePoint {
    pub step: usize,
    pub price_usd: Option<f64>,
    pub normalized_price: f64,
    pub expected_liquidation: f64,
    pub impact_amplification: f64,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct DashboardLiquidityLevel {
    pub price_usd: Option<f64>,
    pub normalized_price: f64,
    pub side: String,
    pub pressure: f64,
    pub depth_score: f64,
    pub label: String,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DashboardDataSources {
    pub flow: String,
    pub liquidation: String,
    pub options_gamma: String,
    pub orderbook: String,
}

impl Default for DashboardDataSources {
    fn default() -> Self {
        Self {
            flow: "unavailable".to_string(),
            liquidation: "flow_proxy".to_string(),
            options_gamma: "proxy".to_string(),
            orderbook: "unavailable".to_string(),
        }
    }
}

impl Default for BTCLiquidationDashboard {
    fn default() -> Self {
        Self {
            ts: 0,
            symbol: "BTC".to_string(),
            current_price_usd: None,
            data_status: "unavailable".to_string(),
            read_only: true,
            live: false,
            force_field: DashboardForceFieldState {
                symbol: "BTC".to_string(),
                ..Default::default()
            },
            market_stress: MarketStressOverview::default(),
            liquidation_heatmap: Vec::new(),
            gamma_walls: Vec::new(),
            squeeze: SqueezeDirectionPanel::default(),
            cascade_timeline: Vec::new(),
            liquidity_map: Vec::new(),
            sources: DashboardDataSources::default(),
            notes: vec![
                "BTC liquidation dashboard is read-only and does not execute trades.".to_string(),
            ],
        }
    }
}

pub fn build_btc_liquidation_dashboard(
    flow_state: &FlowState,
    now_ms: i64,
) -> BTCLiquidationDashboard {
    let Some(window) = select_dashboard_window(flow_state) else {
        return BTCLiquidationDashboard {
            ts: now_ms,
            data_status: "waiting_for_btc_flow".to_string(),
            ..Default::default()
        };
    };
    if !is_btc_symbol(&window.symbol) && !is_btc_symbol(&flow_state.symbol) {
        return BTCLiquidationDashboard {
            ts: now_ms,
            data_status: "non_btc_flow_ignored".to_string(),
            live: false,
            ..Default::default()
        };
    }

    let price = current_market_price_from_window(window);
    let flow = flow_tick_from_window(window, now_ms);
    let stealth = StealthEngine::analyze(&flow);
    let glce = GLCEEngine::compute(&flow, &stealth);
    let lhcs = LHCSEngine::compute(&flow, &glce);
    let gex = GEXEngine::compute_from_tick(&flow, &glce, &lhcs);
    let force = MarketForceFieldEngine::compute(&flow, &glce, &lhcs, &gex);
    let Some(btc_liq) = BTCLiquidationEngine::compute(&flow, &glce, &lhcs, &gex) else {
        return BTCLiquidationDashboard {
            ts: now_ms,
            data_status: "btc_liquidation_engine_unavailable".to_string(),
            live: false,
            ..Default::default()
        };
    };

    let live = window.trade_count > 0;
    let data_status = if live {
        "flow_proxy_live".to_string()
    } else {
        "waiting_for_btc_flow".to_string()
    };

    BTCLiquidationDashboard {
        ts: now_ms,
        symbol: "BTC".to_string(),
        current_price_usd: price.map(round_price),
        data_status,
        read_only: true,
        live,
        force_field: force_field_state(&force, &btc_liq),
        market_stress: market_stress(&force, btc_liq.cascade_risk, btc_liq.gamma_pressure),
        liquidation_heatmap: lhcs
            .liquidation_heatmap
            .price_bins
            .iter()
            .map(|bin| LiqLevel {
                price_usd: normalize_price(bin.price, price),
                normalized_price: round(bin.price, 5),
                side: side_for_normalized_price(bin.price),
                leverage_density: round(bin.leverage_density, 4),
                liquidation_volume: round(bin.liquidation_volume, 4),
                risk_score: round(bin.leverage_density, 4),
            })
            .collect(),
        gamma_walls: gex
            .gamma_wall_levels
            .iter()
            .map(|wall| DashboardGammaWall {
                strike_usd: normalize_price(wall.strike, price),
                normalized_strike: round(wall.strike, 5),
                gamma_exposure: round(wall.gamma_exposure, 4),
                call_put_imbalance: round(wall.call_put_imbalance, 4),
                role: gamma_wall_role(wall.gamma_exposure),
            })
            .collect(),
        squeeze: SqueezeDirectionPanel {
            up_probability: round(btc_liq.squeeze_up_probability, 4),
            down_probability: round(btc_liq.squeeze_down_probability, 4),
            dominant_direction: dominant_squeeze_direction(
                btc_liq.squeeze_up_probability,
                btc_liq.squeeze_down_probability,
            ),
            breakout_bias: format!("{:?}", glce.breakout_bias).to_ascii_lowercase(),
            net_liquidation_bias: round(btc_liq.net_liquidation_bias, 4),
            long_liquidation_pressure: round(btc_liq.long_liquidation_pressure, 4),
            short_liquidation_pressure: round(btc_liq.short_liquidation_pressure, 4),
        },
        cascade_timeline: lhcs
            .cascade_state
            .propagation_chain
            .iter()
            .enumerate()
            .map(|(index, step)| CascadeTimelinePoint {
                step: index + 1,
                price_usd: normalize_price(step.price_level, price),
                normalized_price: round(step.price_level, 5),
                expected_liquidation: round(step.expected_liquidation, 4),
                impact_amplification: round(step.impact_amplification, 4),
            })
            .collect(),
        liquidity_map: lhcs
            .liquidity_void_zones
            .iter()
            .map(|zone| {
                let normalized = (zone.lower + zone.upper) / 2.0;
                DashboardLiquidityLevel {
                    price_usd: normalize_price(normalized, price),
                    normalized_price: round(normalized, 5),
                    side: side_for_normalized_price(normalized),
                    pressure: round(zone.risk_score, 4),
                    depth_score: round(1.0 - clamp01(zone.risk_score), 4),
                    label: "liquidity_void".to_string(),
                }
            })
            .collect(),
        sources: DashboardDataSources {
            flow: if live { "btc_flow_window" } else { "waiting" }.to_string(),
            liquidation: "flow_proxy".to_string(),
            options_gamma: "proxy_from_btc_flow".to_string(),
            orderbook: if window.mid_end.is_some() || window.mid_start.is_some() {
                "price_index"
            } else {
                "notional_weighted_price"
            }
            .to_string(),
        },
        notes: vec![
            "Read-only BTC liquidation intelligence; no order execution or account access."
                .to_string(),
            "Liquidation and options gamma are proxy projections until dedicated feeds are wired."
                .to_string(),
        ],
    }
}

fn select_dashboard_window(flow_state: &FlowState) -> Option<&FlowWindow> {
    flow_state
        .windows
        .get("60000")
        .or_else(|| flow_state.windows.get("15000"))
        .or_else(|| flow_state.windows.get("5000"))
        .or_else(|| {
            flow_state
                .windows
                .values()
                .max_by_key(|window| window.trade_count)
        })
}

fn flow_tick_from_window(window: &FlowWindow, now_ms: i64) -> MarketFlowTick {
    let total = (window.aggressive_buy_btc + window.aggressive_sell_btc).max(1.0);
    let dominance = clamp01(window.net_aggressive_btc.abs() / total);
    let volume_pressure = clamp01(total / 1_500.0);
    let price_move_pct = window.price_move_bps.unwrap_or_default() / 100.0;
    let realized_vol = clamp01(price_move_pct.abs());
    let liquidation_pressure =
        clamp01(volume_pressure * 0.45 + dominance * 0.35 + realized_vol * 0.20);

    MarketFlowTick {
        ts: window.now_ts.max(now_ms),
        exchange: MarketFlowExchange::Binance,
        symbol: "BTC".to_string(),
        buy_volume: window.aggressive_buy_btc.max(0.0),
        sell_volume: window.aggressive_sell_btc.max(0.0),
        net_flow: window.net_aggressive_btc,
        flow_acceleration: window.net_aggressive_btc,
        trade_count: window.trade_count.min(u32::MAX as u64) as u32,
        avg_trade_size: window.avg_trade_size_btc.max(0.0),
        large_trade_ratio: clamp01(window.max_trade_size_btc / total),
        realized_vol,
        open_interest_delta: 0.0,
        funding_rate: 0.0,
        liquidation_pressure,
        price_move_pct,
        dynamic_multiple: (total / 1_000.0).clamp(0.0, 10.0),
        anomaly_persistence_sec: (window.window_ms / 1000) as f64,
        cross_exchange_dispersion: 0.0,
    }
}

fn current_market_price_from_window(window: &FlowWindow) -> Option<f64> {
    let total_volume = window.aggressive_buy_btc + window.aggressive_sell_btc;
    let total_notional = window.aggressive_buy_usd + window.aggressive_sell_usd;
    if total_volume > f64::EPSILON && total_notional > 0.0 {
        return Some(total_notional / total_volume)
            .filter(|price| price.is_finite() && *price > 0.0);
    }
    window
        .mid_end
        .or(window.mid_start)
        .filter(|price| price.is_finite() && *price > 0.0)
}

fn market_stress(
    force: &MarketForceField,
    cascade_risk: f64,
    gamma_pressure: f64,
) -> MarketStressOverview {
    MarketStressOverview {
        stress_score: round(force.total_stress, 4),
        liquidity_field: round(force.liquidity_field, 4),
        gamma_field: round(force.gamma_field, 4),
        liquidation_field: round(force.liquidation_field, 4),
        cascade_field: round(force.cascade_field, 4),
        instability_index: round(force.instability_index, 4),
        directional_bias: direction_label(force.directional_bias),
        regime: format!("{:?}", force.regime_state).to_ascii_lowercase(),
        cascade_risk: round(cascade_risk, 4),
        gamma_pressure: round(gamma_pressure, 4),
    }
}

fn force_field_state(
    force: &MarketForceField,
    btc_liq: &super::btc_liquidation::BTCLiquidationState,
) -> DashboardForceFieldState {
    DashboardForceFieldState {
        ts: btc_liq.ts,
        symbol: "BTC".to_string(),
        liquidity_field: round(force.liquidity_field, 4),
        gamma_field: round(force.gamma_field, 4),
        liquidation_field: round(force.liquidation_field, 4),
        cascade_field: round(force.cascade_field, 4),
        total_stress: round(force.total_stress, 4),
        instability_index: round(force.instability_index, 4),
        next_move_bias: force_next_move_bias(btc_liq),
        squeeze_probability: round(
            btc_liq
                .squeeze_up_probability
                .max(btc_liq.squeeze_down_probability),
            4,
        ),
        cascade_probability: round(btc_liq.cascade_risk, 4),
        predicted_regime: format!("{:?}", force.regime_state).to_ascii_lowercase(),
    }
}

fn force_next_move_bias(btc_liq: &super::btc_liquidation::BTCLiquidationState) -> String {
    if btc_liq.squeeze_up_probability > btc_liq.squeeze_down_probability * 1.10
        && btc_liq.squeeze_up_probability >= 0.35
    {
        "upward_squeeze".to_string()
    } else if btc_liq.squeeze_down_probability > btc_liq.squeeze_up_probability * 1.10
        && btc_liq.squeeze_down_probability >= 0.35
    {
        "downward_squeeze".to_string()
    } else {
        "neutral".to_string()
    }
}

fn normalize_price(normalized: f64, current_price: Option<f64>) -> Option<f64> {
    current_price.map(|price| round_price(price * normalized))
}

fn side_for_normalized_price(value: f64) -> String {
    if value > 1.0001 {
        "above".to_string()
    } else if value < 0.9999 {
        "below".to_string()
    } else {
        "current".to_string()
    }
}

fn gamma_wall_role(gamma_exposure: f64) -> String {
    if gamma_exposure > 0.0 {
        "support".to_string()
    } else if gamma_exposure < 0.0 {
        "resistance".to_string()
    } else {
        "neutral".to_string()
    }
}

fn dominant_squeeze_direction(up: f64, down: f64) -> String {
    if up > down * 1.10 && up >= 0.35 {
        "up".to_string()
    } else if down > up * 1.10 && down >= 0.35 {
        "down".to_string()
    } else {
        "neutral".to_string()
    }
}

fn direction_label(direction: Direction) -> String {
    match direction {
        Direction::Buy => "buy".to_string(),
        Direction::Sell => "sell".to_string(),
        Direction::Absorption => "absorption".to_string(),
        Direction::Suppression => "suppression".to_string(),
        Direction::Neutral => "neutral".to_string(),
    }
}

fn round_price(value: f64) -> f64 {
    round(value, 2)
}

fn round(value: f64, places: i32) -> f64 {
    if !value.is_finite() {
        return 0.0;
    }
    let factor = 10_f64.powi(places);
    (value * factor).round() / factor
}
