use super::types::{
    AltContractDirection, AltContractMarketTier, AltContractSeverity, AltContractSignal,
};

#[derive(Debug, Clone, Copy, Default, serde::Serialize, serde::Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct MarketStateVector {
    pub global_liquidity: f64,
    pub risk_pressure: f64,
    pub directional_bias: f64,
    pub volatility_regime: f64,
    pub cross_asset_correlation: f64,
    pub manipulation_index: f64,
    pub flow_intensity: f64,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct AssetProjection {
    pub asset: String,
    pub contribution_vector: MarketStateVector,
    pub weight: f64,
}

#[derive(Debug, Clone, Copy, Default, serde::Serialize, serde::Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct CrossAssetField {
    pub btc_influence: f64,
    pub eth_response: f64,
    pub alt_amplification: f64,
    pub bsc_absorption: f64,
}

#[derive(Debug, Clone, Default, serde::Serialize, serde::Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct UnifiedMarketCognition {
    pub market_state: MarketStateVector,
    pub dominant_force: String,
    pub dominant_asset: String,
    pub risk_regime: String,
    pub liquidity_direction: String,
    pub manipulation_field: f64,
    pub cross_asset_field: CrossAssetField,
    pub unified_signal: UnifiedSignal,
    pub read_only: bool,
    pub direct_discord_gate: bool,
}

#[derive(Debug, Clone, Default, serde::Serialize, serde::Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct UnifiedSignal {
    pub global_regime: String,
    pub dominant_asset: String,
    pub confidence: f64,
    pub cross_asset_alignment: f64,
    pub manipulation_risk: f64,
}

pub fn projection_from_alt_signal(signal: &AltContractSignal) -> AssetProjection {
    let directional_bias = direction_bias(signal);
    let flow_intensity = signal_flow_intensity(signal);
    let risk_pressure = signal_risk_pressure(signal);
    let volatility_regime = volatility_regime(signal);
    let manipulation_index = manipulation_index(signal);
    let global_liquidity = liquidity_contribution(signal);

    AssetProjection {
        asset: signal.symbol.to_ascii_uppercase(),
        contribution_vector: MarketStateVector {
            global_liquidity,
            risk_pressure,
            directional_bias,
            volatility_regime,
            cross_asset_correlation: 0.0,
            manipulation_index,
            flow_intensity,
        },
        weight: asset_weight(signal.market_tier, &signal.symbol),
    }
}

pub fn cognize_market_state(projections: &[AssetProjection]) -> UnifiedMarketCognition {
    if projections.is_empty() {
        return UnifiedMarketCognition {
            market_state: MarketStateVector::default(),
            dominant_force: "none".to_string(),
            dominant_asset: "none".to_string(),
            risk_regime: "idle".to_string(),
            liquidity_direction: "neutral".to_string(),
            manipulation_field: 0.0,
            cross_asset_field: CrossAssetField::default(),
            unified_signal: UnifiedSignal {
                global_regime: "NoData".to_string(),
                dominant_asset: "none".to_string(),
                confidence: 0.0,
                cross_asset_alignment: 0.0,
                manipulation_risk: 0.0,
            },
            read_only: true,
            direct_discord_gate: false,
        };
    }

    let weighted_state = weighted_market_state(projections);
    let dominant_asset = dominant_asset(projections);
    let cross_asset_alignment = cross_asset_alignment(projections);
    let manipulation_field = weighted_state.manipulation_index;
    let cross_asset_field = cross_asset_field(projections);
    let global_regime = global_regime(
        &dominant_asset,
        &weighted_state,
        cross_asset_alignment,
        manipulation_field,
        projections,
    );
    let confidence =
        cognition_confidence(&weighted_state, cross_asset_alignment, manipulation_field);

    UnifiedMarketCognition {
        market_state: MarketStateVector {
            cross_asset_correlation: cross_asset_alignment,
            ..weighted_state
        },
        dominant_force: dominant_force(&weighted_state),
        dominant_asset: dominant_asset.clone(),
        risk_regime: risk_regime(&weighted_state, manipulation_field),
        liquidity_direction: liquidity_direction(weighted_state.directional_bias),
        manipulation_field: round2(manipulation_field),
        cross_asset_field,
        unified_signal: UnifiedSignal {
            global_regime,
            dominant_asset,
            confidence,
            cross_asset_alignment,
            manipulation_risk: round2(manipulation_field),
        },
        read_only: true,
        direct_discord_gate: false,
    }
}

fn weighted_market_state(projections: &[AssetProjection]) -> MarketStateVector {
    let total_weight = projections
        .iter()
        .map(|projection| projection.weight.max(0.0))
        .sum::<f64>();
    if total_weight <= 0.0 {
        return MarketStateVector::default();
    }

    let mut state = MarketStateVector::default();
    for projection in projections {
        let weight = projection.weight.max(0.0) / total_weight;
        state.global_liquidity += projection.contribution_vector.global_liquidity * weight;
        state.risk_pressure += projection.contribution_vector.risk_pressure * weight;
        state.directional_bias += projection.contribution_vector.directional_bias * weight;
        state.volatility_regime += projection.contribution_vector.volatility_regime * weight;
        state.manipulation_index += projection.contribution_vector.manipulation_index * weight;
        state.flow_intensity += projection.contribution_vector.flow_intensity * weight;
    }

    MarketStateVector {
        global_liquidity: round2(state.global_liquidity.clamp(0.0, 100.0)),
        risk_pressure: round2(state.risk_pressure.clamp(0.0, 100.0)),
        directional_bias: round2(state.directional_bias.clamp(-100.0, 100.0)),
        volatility_regime: round2(state.volatility_regime.clamp(0.0, 100.0)),
        cross_asset_correlation: 0.0,
        manipulation_index: round2(state.manipulation_index.clamp(0.0, 100.0)),
        flow_intensity: round2(state.flow_intensity.clamp(0.0, 100.0)),
    }
}

fn dominant_asset(projections: &[AssetProjection]) -> String {
    projections
        .iter()
        .max_by(|left, right| {
            projection_strength(left)
                .partial_cmp(&projection_strength(right))
                .unwrap_or(std::cmp::Ordering::Equal)
        })
        .map(|projection| projection.asset.clone())
        .unwrap_or_else(|| "none".to_string())
}

fn projection_strength(projection: &AssetProjection) -> f64 {
    projection.weight.max(0.0)
        * (projection.contribution_vector.flow_intensity * 0.45
            + projection.contribution_vector.global_liquidity * 0.25
            + projection.contribution_vector.risk_pressure * 0.20
            + projection.contribution_vector.manipulation_index * 0.10)
}

fn cross_asset_alignment(projections: &[AssetProjection]) -> f64 {
    let active = projections
        .iter()
        .filter(|projection| projection.contribution_vector.flow_intensity >= 20.0)
        .collect::<Vec<_>>();
    if active.len() < 2 {
        return 0.0;
    }

    let bullish = active
        .iter()
        .filter(|projection| projection.contribution_vector.directional_bias > 15.0)
        .count();
    let bearish = active
        .iter()
        .filter(|projection| projection.contribution_vector.directional_bias < -15.0)
        .count();
    let majority = bullish.max(bearish) as f64;
    let participation = active.len() as f64 / projections.len().max(1) as f64;
    round2((majority / active.len() as f64) * participation * 100.0)
}

fn cross_asset_field(projections: &[AssetProjection]) -> CrossAssetField {
    let btc = projection_for(projections, "BTC");
    let eth = projection_for(projections, "ETH");
    let alt = projections
        .iter()
        .filter(|projection| !matches_core_asset(&projection.asset))
        .map(|projection| projection.contribution_vector.flow_intensity)
        .fold(0.0, f64::max);
    let bsc = projections
        .iter()
        .filter(|projection| {
            projection.asset.starts_with("BSC") || projection.asset.ends_with("_BSC")
        })
        .map(|projection| projection.contribution_vector.global_liquidity)
        .fold(0.0, f64::max);

    CrossAssetField {
        btc_influence: btc
            .map(|item| item.contribution_vector.flow_intensity)
            .unwrap_or(0.0),
        eth_response: eth
            .map(|item| {
                response_score(
                    item.contribution_vector.directional_bias,
                    btc.map(|btc| btc.contribution_vector.directional_bias)
                        .unwrap_or(0.0),
                    item.contribution_vector.flow_intensity,
                )
            })
            .unwrap_or(0.0),
        alt_amplification: round2(alt),
        bsc_absorption: round2(bsc),
    }
}

fn projection_for<'a>(
    projections: &'a [AssetProjection],
    asset: &str,
) -> Option<&'a AssetProjection> {
    projections
        .iter()
        .find(|projection| projection.asset.eq_ignore_ascii_case(asset))
}

fn response_score(direction: f64, reference_direction: f64, flow_intensity: f64) -> f64 {
    if reference_direction.abs() < 10.0 || direction.abs() < 10.0 {
        return 0.0;
    }
    let same_direction = direction.signum() == reference_direction.signum();
    let base = if same_direction {
        flow_intensity
    } else {
        flow_intensity * 0.35
    };
    round2(base.clamp(0.0, 100.0))
}

fn global_regime(
    dominant_asset: &str,
    state: &MarketStateVector,
    cross_asset_alignment: f64,
    manipulation_field: f64,
    projections: &[AssetProjection],
) -> String {
    if manipulation_field >= 65.0 {
        return "CrossAssetManipulationRisk".to_string();
    }
    if dominant_asset.eq_ignore_ascii_case("BTC")
        && state.flow_intensity >= 45.0
        && btc_leads_market(projections)
    {
        return "BtcLedRegime".to_string();
    }
    if !matches_core_asset(dominant_asset)
        && dominant_projection_flow(dominant_asset, projections) >= 70.0
        && cross_asset_alignment < 45.0
    {
        return "LocalLiquidityEvent".to_string();
    }
    if cross_asset_alignment >= 70.0
        && state.flow_intensity >= 45.0
        && state.risk_pressure < 75.0
        && state.directional_bias.abs() >= 20.0
    {
        return "GlobalAccumulation".to_string();
    }
    "MixedMarketState".to_string()
}

fn btc_leads_market(projections: &[AssetProjection]) -> bool {
    let Some(btc) = projection_for(projections, "BTC") else {
        return false;
    };
    let btc_strength = projection_strength(btc);
    let non_btc_max = projections
        .iter()
        .filter(|projection| !projection.asset.eq_ignore_ascii_case("BTC"))
        .map(projection_strength)
        .fold(0.0, f64::max);
    btc_strength >= non_btc_max * 1.5
}

fn dominant_projection_flow(dominant_asset: &str, projections: &[AssetProjection]) -> f64 {
    projection_for(projections, dominant_asset)
        .map(|projection| projection.contribution_vector.flow_intensity)
        .unwrap_or_default()
}

fn dominant_force(state: &MarketStateVector) -> String {
    if state.manipulation_index >= 65.0 {
        "manipulation_field".to_string()
    } else if state.directional_bias >= 20.0 {
        "buy_side_liquidity".to_string()
    } else if state.directional_bias <= -20.0 {
        "sell_side_liquidity".to_string()
    } else {
        "balanced_liquidity".to_string()
    }
}

fn risk_regime(state: &MarketStateVector, manipulation_field: f64) -> String {
    if manipulation_field >= 65.0 {
        "manipulation_watch".to_string()
    } else if state.risk_pressure >= 80.0 {
        "high_risk_pressure".to_string()
    } else if state.flow_intensity >= 55.0 {
        "active_flow".to_string()
    } else {
        "calm".to_string()
    }
}

fn liquidity_direction(directional_bias: f64) -> String {
    if directional_bias >= 20.0 {
        "bullish".to_string()
    } else if directional_bias <= -20.0 {
        "bearish".to_string()
    } else {
        "neutral".to_string()
    }
}

fn cognition_confidence(
    state: &MarketStateVector,
    cross_asset_alignment: f64,
    manipulation_field: f64,
) -> f64 {
    let confidence = state.flow_intensity * 0.30
        + state.global_liquidity * 0.20
        + cross_asset_alignment * 0.25
        + (100.0 - manipulation_field).clamp(0.0, 100.0) * 0.15
        + state.risk_pressure * 0.10;
    round2(confidence.clamp(0.0, 100.0))
}

fn direction_bias(signal: &AltContractSignal) -> f64 {
    if signal.direction_bias != 0 {
        return signal.direction_bias as f64;
    }
    let sign = match signal.direction {
        AltContractDirection::Buy => 1.0,
        AltContractDirection::Sell => -1.0,
        AltContractDirection::Absorption => 0.35,
        AltContractDirection::Suppression => -0.35,
        AltContractDirection::Neutral => 0.0,
    };
    round2((signal.dominance * 100.0 * sign).clamp(-100.0, 100.0))
}

fn signal_flow_intensity(signal: &AltContractSignal) -> f64 {
    let score = signal.abnormal_score as f64 * 0.20
        + signal.build_score as f64 * 0.25
        + signal.master_capital_strength.mcss * 0.25
        + signal.alt_impact_score.final_score * 0.20
        + notional_score(signal.total_notional_usd) * 0.10;
    round2(score.clamp(0.0, 100.0))
}

fn signal_risk_pressure(signal: &AltContractSignal) -> f64 {
    let severity: f64 = match signal.severity {
        AltContractSeverity::Calm => 5.0,
        AltContractSeverity::Medium => 35.0,
        AltContractSeverity::High => 60.0,
        AltContractSeverity::Critical => 80.0,
        AltContractSeverity::S => 95.0,
    };
    let liquidation: f64 = if signal.liquidation_suspected || signal.force_order_snapshot {
        18.0
    } else {
        0.0
    };
    round2((severity + liquidation).clamp(0.0, 100.0))
}

fn volatility_regime(signal: &AltContractSignal) -> f64 {
    let price = signal.price_move_pct.unwrap_or(0.0).abs() * 100.0;
    let dynamic = signal.dynamic_multiple.unwrap_or(0.0) * 8.0;
    round2((price + dynamic).clamp(0.0, 100.0))
}

fn manipulation_index(signal: &AltContractSignal) -> f64 {
    let regime = signal.market_regime.regime.to_ascii_lowercase();
    let control = signal
        .market_control_graph
        .control_type
        .to_ascii_lowercase();
    let micro = signal
        .liquidity_microstructure
        .behavior
        .to_ascii_lowercase();

    let mut score: f64 = 0.0;
    if regime.contains("manipulation") {
        score += 35.0;
    }
    if control.contains("manipulation") {
        score += 25.0;
    }
    if micro.contains("spoof") || micro.contains("sweep") {
        score += 20.0;
    }
    if signal.liquidation_suspected || signal.force_order_snapshot {
        score += 15.0;
    }
    if signal.dynamic_multiple.unwrap_or(0.0) >= 6.0
        && signal.price_move_pct.unwrap_or(0.0).abs() >= 0.5
    {
        score += 10.0;
    }
    round2(score.clamp(0.0, 100.0))
}

fn liquidity_contribution(signal: &AltContractSignal) -> f64 {
    let depth_score = signal
        .depth_1pct_usd
        .or(signal.depth_0_5pct_usd)
        .map(notional_score)
        .unwrap_or_else(|| notional_score(signal.total_notional_usd));
    let data_quality = signal.data_quality as f64;
    round2((depth_score * 0.65 + data_quality * 0.35).clamp(0.0, 100.0))
}

fn notional_score(notional_usd: f64) -> f64 {
    if notional_usd <= 0.0 {
        return 0.0;
    }
    ((notional_usd.log10() - 4.0) * 22.0).clamp(0.0, 100.0)
}

fn asset_weight(market_tier: AltContractMarketTier, symbol: &str) -> f64 {
    match (market_tier, symbol.to_ascii_uppercase().as_str()) {
        (_, "BTC") => 1.25,
        (_, "ETH") => 1.05,
        (AltContractMarketTier::UltraCore, _) => 1.0,
        (AltContractMarketTier::Mainstream, _) => 0.85,
        (AltContractMarketTier::Alt, _) => 0.70,
    }
}

fn matches_core_asset(asset: &str) -> bool {
    matches!(
        asset.to_ascii_uppercase().as_str(),
        "BTC" | "ETH" | "SOL" | "BNB"
    )
}

fn round2(value: f64) -> f64 {
    (value * 100.0).round() / 100.0
}
