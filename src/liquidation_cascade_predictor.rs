use std::cmp::Ordering;

use serde::{Deserialize, Serialize};

use crate::types::{
    flow::{FlowState, FlowWindow},
    liquidation::{EstimatedLiquidationCluster, LiquidationClusterSide, LiquidationState},
};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum CascadeDirection {
    Up,
    Down,
    Neutral,
}

impl CascadeDirection {
    pub fn as_key(self) -> &'static str {
        match self {
            Self::Up => "UP",
            Self::Down => "DOWN",
            Self::Neutral => "NEUTRAL",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum CascadeStatus {
    Calm,
    Warning,
    Imminent,
    Active,
}

impl CascadeStatus {
    pub fn as_key(self) -> &'static str {
        match self {
            Self::Calm => "CALM",
            Self::Warning => "WARNING",
            Self::Imminent => "IMMINENT",
            Self::Active => "ACTIVE",
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct LcpInput {
    pub symbol: String,
    pub current_price: Option<f64>,
    pub long_cluster_density: f64,
    pub short_cluster_density: f64,
    pub long_cluster_price: Option<f64>,
    pub short_cluster_price: Option<f64>,
    pub oi_change_pct: Option<f64>,
    pub funding_rate: Option<f64>,
    pub sell_volume_spike: f64,
    pub buy_volume_spike: f64,
    pub liquidity_gap_below: f64,
    pub liquidity_gap_above: f64,
    pub liquidation_spike: f64,
}

impl Default for LcpInput {
    fn default() -> Self {
        Self {
            symbol: "BTC".to_string(),
            current_price: None,
            long_cluster_density: 0.0,
            short_cluster_density: 0.0,
            long_cluster_price: None,
            short_cluster_price: None,
            oi_change_pct: None,
            funding_rate: None,
            sell_volume_spike: 0.0,
            buy_volume_spike: 0.0,
            liquidity_gap_below: 0.0,
            liquidity_gap_above: 0.0,
            liquidation_spike: 0.0,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct LcpComponents {
    pub leverage_concentration: f64,
    pub liquidity_gap: f64,
    pub funding_stress: f64,
    pub trigger_proximity: f64,
    pub oi_stress: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct LcpPriceZone {
    pub low: f64,
    pub high: f64,
    pub strength: f64,
    pub side: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct LcpCascadeResponse {
    pub symbol: String,
    pub cascade_probability: f64,
    pub status: CascadeStatus,
    pub direction: CascadeDirection,
    pub estimated_move: String,
    pub time_window: String,
    pub risk_zone: Option<[f64; 2]>,
    pub signals: Vec<String>,
    pub components: LcpComponents,
    pub read_only: bool,
    pub runtime_modified: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct LcpLeverageLevel {
    pub price: f64,
    pub side: String,
    pub intensity: f64,
    pub notional_usd: f64,
    pub distance_bps: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct LcpLeverageMapResponse {
    pub symbol: String,
    pub heatmap: Vec<LcpLeverageLevel>,
    pub high_risk_zones: Vec<LcpPriceZone>,
    pub read_only: bool,
    pub runtime_modified: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct LcpLiquidityGapResponse {
    pub symbol: String,
    pub below_price: f64,
    pub above_price: f64,
    pub dominant_gap: CascadeDirection,
    pub signals: Vec<String>,
    pub read_only: bool,
    pub runtime_modified: bool,
}

pub fn analyze_liquidation_cascade(input: &LcpInput) -> LcpCascadeResponse {
    let direction = dominant_direction(input);
    let leverage_concentration = match direction {
        CascadeDirection::Down => input.long_cluster_density,
        CascadeDirection::Up => input.short_cluster_density,
        CascadeDirection::Neutral => input.long_cluster_density.max(input.short_cluster_density),
    }
    .clamp(0.0, 1.0);
    let liquidity_gap = match direction {
        CascadeDirection::Down => input.liquidity_gap_below,
        CascadeDirection::Up => input.liquidity_gap_above,
        CascadeDirection::Neutral => input.liquidity_gap_below.max(input.liquidity_gap_above),
    }
    .clamp(0.0, 1.0);
    let funding_stress = funding_stress(input.funding_rate);
    let oi_stress = input
        .oi_change_pct
        .map(|change| (change.abs() / 2.0).clamp(0.0, 1.0))
        .unwrap_or(0.0);
    let trigger_proximity = trigger_proximity(input, direction);

    let probability = round4(
        (leverage_concentration * 0.35
            + liquidity_gap * 0.25
            + funding_stress * 0.15
            + oi_stress * 0.05
            + trigger_proximity * 0.20)
            .clamp(0.0, 1.0),
    );
    let status = cascade_status(probability, trigger_proximity, input.liquidation_spike);
    let risk_zone = selected_cluster_price(input, direction).map(price_zone_for_cluster);
    let signals = cascade_signals(
        leverage_concentration,
        liquidity_gap,
        funding_stress,
        trigger_proximity,
        input.liquidation_spike,
    );

    LcpCascadeResponse {
        symbol: input.symbol.clone(),
        cascade_probability: probability,
        status,
        direction,
        estimated_move: estimated_move(probability),
        time_window: time_window(status),
        risk_zone,
        signals,
        components: LcpComponents {
            leverage_concentration: round4(leverage_concentration),
            liquidity_gap: round4(liquidity_gap),
            funding_stress: round4(funding_stress),
            trigger_proximity: round4(trigger_proximity),
            oi_stress: round4(oi_stress),
        },
        read_only: true,
        runtime_modified: false,
    }
}

pub fn leverage_map_from_liquidation_state(
    symbol: &str,
    state: &LiquidationState,
) -> LcpLeverageMapResponse {
    let mut heatmap = state
        .recent_clusters
        .iter()
        .map(leverage_level_from_cluster)
        .collect::<Vec<_>>();
    heatmap.sort_by(|a, b| {
        b.intensity
            .partial_cmp(&a.intensity)
            .unwrap_or(Ordering::Equal)
    });
    heatmap.truncate(24);
    let high_risk_zones = heatmap
        .iter()
        .filter(|level| level.intensity >= 0.60)
        .map(|level| LcpPriceZone {
            low: round2(level.price * 0.9985),
            high: round2(level.price * 1.0015),
            strength: level.intensity,
            side: level.side.clone(),
        })
        .collect();

    LcpLeverageMapResponse {
        symbol: symbol.to_string(),
        heatmap,
        high_risk_zones,
        read_only: true,
        runtime_modified: false,
    }
}

pub fn liquidity_gap_from_input(input: &LcpInput) -> LcpLiquidityGapResponse {
    let dominant_gap = if input.liquidity_gap_below > input.liquidity_gap_above + 0.05 {
        CascadeDirection::Down
    } else if input.liquidity_gap_above > input.liquidity_gap_below + 0.05 {
        CascadeDirection::Up
    } else {
        CascadeDirection::Neutral
    };
    let mut signals = Vec::new();
    if input.liquidity_gap_below >= 0.55 {
        signals.push("LIQUIDITY_VOID_BELOW".to_string());
    }
    if input.liquidity_gap_above >= 0.55 {
        signals.push("LIQUIDITY_VOID_ABOVE".to_string());
    }
    if signals.is_empty() {
        signals.push("LIQUIDITY_NORMAL".to_string());
    }

    LcpLiquidityGapResponse {
        symbol: input.symbol.clone(),
        below_price: round4(input.liquidity_gap_below.clamp(0.0, 1.0)),
        above_price: round4(input.liquidity_gap_above.clamp(0.0, 1.0)),
        dominant_gap,
        signals,
        read_only: true,
        runtime_modified: false,
    }
}

pub fn input_from_runtime_state(
    symbol: &str,
    flow_state: &FlowState,
    liquidation_state: &LiquidationState,
    oi_change_pct: Option<f64>,
    funding_rate: Option<f64>,
) -> LcpInput {
    let selected_window = select_flow_window(flow_state);
    let flow_price = selected_window.and_then(|window| window.mid_end.or(window.mid_start));
    let (buy_volume_spike, sell_volume_spike) =
        selected_window.map(flow_side_ratios).unwrap_or((0.0, 0.0));
    let (liquidity_gap_below, liquidity_gap_above) = selected_window
        .map(liquidity_gaps_from_window)
        .unwrap_or((0.0, 0.0));

    let long_cluster = strongest_cluster(liquidation_state, LiquidationClusterSide::LongBelow);
    let short_cluster = strongest_cluster(liquidation_state, LiquidationClusterSide::ShortAbove);

    LcpInput {
        symbol: symbol.to_string(),
        current_price: liquidation_state.metrics.current_mid.or(flow_price),
        long_cluster_density: long_cluster
            .map(|cluster| cluster.cluster_density)
            .or_else(|| {
                liquidation_state
                    .metrics
                    .nearest_long_liq_cluster_below
                    .as_ref()
                    .map(|cluster| cluster.cluster_density)
            })
            .unwrap_or(0.0)
            .clamp(0.0, 1.0),
        short_cluster_density: short_cluster
            .map(|cluster| cluster.cluster_density)
            .or_else(|| {
                liquidation_state
                    .metrics
                    .nearest_short_liq_cluster_above
                    .as_ref()
                    .map(|cluster| cluster.cluster_density)
            })
            .unwrap_or(0.0)
            .clamp(0.0, 1.0),
        long_cluster_price: long_cluster.map(|cluster| cluster.price).or_else(|| {
            liquidation_state
                .metrics
                .nearest_long_liq_cluster_below
                .as_ref()
                .map(|cluster| cluster.price)
        }),
        short_cluster_price: short_cluster.map(|cluster| cluster.price).or_else(|| {
            liquidation_state
                .metrics
                .nearest_short_liq_cluster_above
                .as_ref()
                .map(|cluster| cluster.price)
        }),
        oi_change_pct,
        funding_rate,
        sell_volume_spike,
        buy_volume_spike,
        liquidity_gap_below,
        liquidity_gap_above,
        liquidation_spike: liquidation_state.metrics.liq_hunt_pressure.clamp(0.0, 1.0),
    }
}

fn dominant_direction(input: &LcpInput) -> CascadeDirection {
    let down_score = input.long_cluster_density * 0.45
        + input.liquidity_gap_below * 0.30
        + input.sell_volume_spike * 0.25;
    let up_score = input.short_cluster_density * 0.45
        + input.liquidity_gap_above * 0.30
        + input.buy_volume_spike * 0.25;

    if down_score > up_score + 0.04 && down_score >= 0.15 {
        CascadeDirection::Down
    } else if up_score > down_score + 0.04 && up_score >= 0.15 {
        CascadeDirection::Up
    } else {
        CascadeDirection::Neutral
    }
}

fn funding_stress(funding_rate: Option<f64>) -> f64 {
    funding_rate
        .map(|rate| (rate.abs() / 0.001).clamp(0.0, 1.0))
        .unwrap_or(0.0)
}

fn trigger_proximity(input: &LcpInput, direction: CascadeDirection) -> f64 {
    let volume_trigger = match direction {
        CascadeDirection::Down => input.sell_volume_spike,
        CascadeDirection::Up => input.buy_volume_spike,
        CascadeDirection::Neutral => input.sell_volume_spike.max(input.buy_volume_spike),
    }
    .clamp(0.0, 1.0);
    let price_trigger = price_proximity_score(
        input.current_price,
        selected_cluster_price(input, direction),
    );

    (volume_trigger * 0.45 + price_trigger * 0.45 + input.liquidation_spike * 0.10).clamp(0.0, 1.0)
}

fn price_proximity_score(current_price: Option<f64>, cluster_price: Option<f64>) -> f64 {
    let (Some(current), Some(cluster)) = (current_price, cluster_price) else {
        return 0.0;
    };
    if current <= f64::EPSILON || cluster <= f64::EPSILON {
        return 0.0;
    }
    let distance_bps = ((current - cluster).abs() / current * 10_000.0).abs();
    if distance_bps <= 10.0 {
        1.0
    } else if distance_bps <= 30.0 {
        0.75
    } else if distance_bps <= 75.0 {
        0.45
    } else if distance_bps <= 150.0 {
        0.20
    } else {
        0.0
    }
}

fn cascade_status(
    probability: f64,
    trigger_proximity: f64,
    liquidation_spike: f64,
) -> CascadeStatus {
    if probability >= 0.75 && liquidation_spike >= 0.50 {
        CascadeStatus::Active
    } else if probability >= 0.75 && trigger_proximity >= 0.55 {
        CascadeStatus::Imminent
    } else if probability >= 0.60 {
        CascadeStatus::Warning
    } else {
        CascadeStatus::Calm
    }
}

fn selected_cluster_price(input: &LcpInput, direction: CascadeDirection) -> Option<f64> {
    match direction {
        CascadeDirection::Down => input.long_cluster_price,
        CascadeDirection::Up => input.short_cluster_price,
        CascadeDirection::Neutral => input.long_cluster_price.or(input.short_cluster_price),
    }
}

fn price_zone_for_cluster(price: f64) -> [f64; 2] {
    [round2(price * 0.9985), round2(price * 1.0015)]
}

fn cascade_signals(
    leverage_concentration: f64,
    liquidity_gap: f64,
    funding_stress: f64,
    trigger_proximity: f64,
    liquidation_spike: f64,
) -> Vec<String> {
    let mut signals = Vec::new();
    if leverage_concentration >= 0.60 {
        signals.push("OI_CLUSTER_HIGH".to_string());
    }
    if liquidity_gap >= 0.55 {
        signals.push("LIQUIDITY_VOID".to_string());
    }
    if funding_stress >= 0.50 {
        signals.push("FUNDING_STRESS".to_string());
    }
    if trigger_proximity >= 0.65 {
        signals.push("TRIGGER_HIT".to_string());
    }
    if liquidation_spike >= 0.50 {
        signals.push("LIQUIDATION_SPIKE".to_string());
    }
    if signals.is_empty() {
        signals.push("CASCADE_RISK_LOW".to_string());
    }
    signals
}

fn estimated_move(probability: f64) -> String {
    if probability >= 0.75 {
        "2.5% - 5%".to_string()
    } else if probability >= 0.60 {
        "1% - 2.5%".to_string()
    } else if probability >= 0.40 {
        "0.5% - 1%".to_string()
    } else {
        "< 0.5%".to_string()
    }
}

fn time_window(status: CascadeStatus) -> String {
    match status {
        CascadeStatus::Active => "now - 5m".to_string(),
        CascadeStatus::Imminent => "5m - 30m".to_string(),
        CascadeStatus::Warning => "15m - 60m".to_string(),
        CascadeStatus::Calm => "no active cascade window".to_string(),
    }
}

fn leverage_level_from_cluster(cluster: &EstimatedLiquidationCluster) -> LcpLeverageLevel {
    LcpLeverageLevel {
        price: round2(cluster.price),
        side: match cluster.side {
            LiquidationClusterSide::ShortAbove => "SHORT_ABOVE".to_string(),
            LiquidationClusterSide::LongBelow => "LONG_BELOW".to_string(),
        },
        intensity: round4(cluster.cluster_density.clamp(0.0, 1.0)),
        notional_usd: round2(cluster.cluster_notional_usd.max(0.0)),
        distance_bps: round2(cluster.distance_bps.abs()),
    }
}

fn select_flow_window(flow_state: &FlowState) -> Option<&FlowWindow> {
    flow_state
        .windows
        .values()
        .filter(|window| window.window_ms <= 60_000)
        .max_by_key(|window| window.window_ms)
        .or_else(|| {
            flow_state
                .windows
                .values()
                .max_by_key(|window| window.window_ms)
        })
}

fn flow_side_ratios(window: &FlowWindow) -> (f64, f64) {
    let total = window.aggressive_buy_btc + window.aggressive_sell_btc;
    if total <= f64::EPSILON {
        return (0.0, 0.0);
    }
    (
        (window.aggressive_buy_btc / total).clamp(0.0, 1.0),
        (window.aggressive_sell_btc / total).clamp(0.0, 1.0),
    )
}

fn liquidity_gaps_from_window(window: &FlowWindow) -> (f64, f64) {
    let imbalance = window
        .imbalance_10bps_median
        .unwrap_or(0.0)
        .clamp(-1.0, 1.0);
    let below_gap = (-imbalance).max(0.0);
    let above_gap = imbalance.max(0.0);
    (below_gap, above_gap)
}

fn strongest_cluster(
    liquidation_state: &LiquidationState,
    side: LiquidationClusterSide,
) -> Option<&EstimatedLiquidationCluster> {
    liquidation_state
        .recent_clusters
        .iter()
        .filter(|cluster| cluster.side == side)
        .max_by(|a, b| {
            a.cluster_density
                .partial_cmp(&b.cluster_density)
                .unwrap_or(Ordering::Equal)
        })
}

fn round2(value: f64) -> f64 {
    (value * 100.0).round() / 100.0
}

fn round4(value: f64) -> f64 {
    (value * 10_000.0).round() / 10_000.0
}
