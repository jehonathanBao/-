use serde::{Deserialize, Serialize};

use crate::runtime::tof_metrics::TofDirection;

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PerpTofMetrics {
    pub oi_change: f64,
    pub oi_direction: String,
    pub funding_rate: f64,
    pub funding_side: String,
    pub liquidation_pressure: f64,
    pub squeeze_side: String,
    pub agg_buy_volume: f64,
    pub agg_sell_volume: f64,
    pub direction_bias: TofDirection,
    pub metrics_direction: TofDirection,
    pub risk_score: u8,
    pub data_quality: f64,
    pub candidate_type: String,
    pub explain_tags: Vec<String>,
    pub confidence: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct MergedTofCandidate {
    pub final_candidate_type: String,
    pub risk_score: u8,
    pub metrics_direction: TofDirection,
    pub confidence: f64,
    pub explain_tags: Vec<String>,
}

#[derive(Debug, Clone)]
pub struct PerpTofInput<'a> {
    pub symbol: &'a str,
    pub spot_candidate_type: &'a str,
    pub spot_direction: TofDirection,
    pub spot_risk_score: u8,
    pub spot_data_quality: f64,
    pub spot_confidence: f64,
    pub summary: &'a str,
}

#[derive(Debug, Clone)]
pub struct PerpScenarioInput {
    pub price_change_bps: f64,
    pub oi_change: f64,
    pub funding_rate: f64,
    pub liquidation_pressure: f64,
    pub squeeze_side: String,
    pub agg_buy_volume: f64,
    pub agg_sell_volume: f64,
    pub data_quality: f64,
}

pub fn build_perp_tof_metrics(input: &PerpTofInput<'_>) -> PerpTofMetrics {
    if !env_bool("PERP_TOF_ENABLED", true) {
        return disabled_perp_metrics(input.spot_data_quality);
    }
    let scenario = synthetic_perp_scenario(input);
    classify_perp_scenario(&scenario)
}

pub fn merge_spot_perp_candidate(
    spot_candidate_type: &str,
    spot_direction: TofDirection,
    spot_risk_score: u8,
    spot_tags: &[String],
    perp_metrics: &PerpTofMetrics,
) -> MergedTofCandidate {
    let metrics_direction = combine_direction(spot_direction, perp_metrics.metrics_direction);
    let directional_boost = if metrics_direction == spot_direction
        && metrics_direction == perp_metrics.metrics_direction
        && metrics_direction != TofDirection::Mixed
    {
        7.0
    } else if metrics_direction == TofDirection::Mixed {
        -6.0
    } else {
        0.0
    };
    let risk_score = clamp_score(
        0.58 * spot_risk_score as f64 + 0.42 * perp_metrics.risk_score as f64 + directional_boost,
    )
    .round() as u8;
    let confidence = clamp_score(
        (spot_risk_score as f64 + perp_metrics.confidence + perp_metrics.data_quality) / 3.0
            + directional_boost,
    );
    let mut explain_tags = spot_tags.to_vec();
    explain_tags.push(spot_candidate_type.to_string());
    explain_tags.extend(perp_metrics.explain_tags.clone());
    explain_tags.sort();
    explain_tags.dedup();
    MergedTofCandidate {
        final_candidate_type: final_candidate_type(metrics_direction, risk_score),
        risk_score,
        metrics_direction,
        confidence,
        explain_tags,
    }
}

pub fn classify_open_interest(price_change_bps: f64, oi_change: f64) -> (String, TofDirection) {
    match (price_change_bps >= 0.0, oi_change >= 0.0) {
        (true, true) => ("long_increase".to_string(), TofDirection::Bullish),
        (true, false) => ("short_decrease".to_string(), TofDirection::Bullish),
        (false, true) => ("short_increase".to_string(), TofDirection::Bearish),
        (false, false) => ("long_decrease".to_string(), TofDirection::Bearish),
    }
}

pub fn classify_funding(
    funding_rate: f64,
    oi_change: f64,
    threshold: f64,
) -> (String, String, f64) {
    let oi_factor = (oi_change.abs() / env_f64("PERP_OI_BUCKET_SIZE", 100_000.0)).clamp(0.0, 2.0);
    if funding_rate > threshold && oi_factor >= 0.5 {
        (
            "CrowdedLongCandidate".to_string(),
            "long".to_string(),
            clamp_score((funding_rate.abs() / threshold) * 55.0 + oi_factor * 20.0),
        )
    } else if funding_rate < -threshold && oi_factor >= 0.5 {
        (
            "CrowdedShortCandidate".to_string(),
            "short".to_string(),
            clamp_score((funding_rate.abs() / threshold) * 55.0 + oi_factor * 20.0),
        )
    } else {
        (
            "FundingNeutralCandidate".to_string(),
            "neutral".to_string(),
            0.0,
        )
    }
}

pub fn classify_liquidation_pressure(
    liquidation_pressure: f64,
    squeeze_side: &str,
) -> (String, TofDirection, f64) {
    let score = clamp_score(liquidation_pressure);
    match squeeze_side.trim().to_ascii_lowercase().as_str() {
        "long" => (
            "LongSqueezeCandidate".to_string(),
            TofDirection::Bearish,
            score,
        ),
        "short" => (
            "ShortSqueezeCandidate".to_string(),
            TofDirection::Bullish,
            score,
        ),
        _ => (
            "LiquidationNeutralCandidate".to_string(),
            TofDirection::Neutral,
            score,
        ),
    }
}

pub fn classify_aggressive_order_flow(
    agg_buy_volume: f64,
    agg_sell_volume: f64,
) -> (TofDirection, f64) {
    let total = (agg_buy_volume + agg_sell_volume).max(1.0);
    let imbalance = ((agg_buy_volume - agg_sell_volume) / total).clamp(-1.0, 1.0);
    let score = clamp_score(imbalance.abs() * 100.0);
    if imbalance > 0.18 {
        (TofDirection::Bullish, score)
    } else if imbalance < -0.18 {
        (TofDirection::Bearish, score)
    } else {
        (TofDirection::Neutral, score)
    }
}

pub fn classify_perp_scenario(input: &PerpScenarioInput) -> PerpTofMetrics {
    let (oi_direction, oi_metrics_direction) =
        classify_open_interest(input.price_change_bps, input.oi_change);
    let funding_threshold = env_f64("PERP_FUNDING_THRESHOLD", 0.05);
    let (funding_candidate, funding_side, funding_score) =
        classify_funding(input.funding_rate, input.oi_change, funding_threshold);
    let funding_direction = match funding_side.as_str() {
        "long" => TofDirection::Bearish,
        "short" => TofDirection::Bullish,
        _ => TofDirection::Neutral,
    };
    let (liquidation_candidate, liquidation_direction, liquidation_score) =
        classify_liquidation_pressure(input.liquidation_pressure, &input.squeeze_side);
    let (flow_direction, flow_score) =
        classify_aggressive_order_flow(input.agg_buy_volume, input.agg_sell_volume);
    let metrics_direction = directional_vote(&[
        oi_metrics_direction,
        funding_direction,
        liquidation_direction,
        flow_direction,
    ]);
    let oi_score =
        clamp_score(input.oi_change.abs() / env_f64("PERP_OI_BUCKET_SIZE", 100_000.0) * 75.0);
    let risk_score = clamp_score(
        0.25 * oi_score + 0.25 * funding_score + 0.25 * liquidation_score + 0.25 * flow_score,
    )
    .round() as u8;
    let candidate_type = dominant_candidate_type(
        &[
            ("OpenInterestCandidate", oi_score),
            (&funding_candidate, funding_score),
            (&liquidation_candidate, liquidation_score),
            ("AggressiveOrderFlowCandidate", flow_score),
        ],
        metrics_direction,
    );
    let explain_tags = explain_tags(
        &oi_direction,
        &funding_candidate,
        &funding_side,
        &liquidation_candidate,
        &input.squeeze_side,
        flow_direction,
    );
    PerpTofMetrics {
        oi_change: input.oi_change,
        oi_direction,
        funding_rate: input.funding_rate,
        funding_side,
        liquidation_pressure: input.liquidation_pressure,
        squeeze_side: input.squeeze_side.clone(),
        agg_buy_volume: input.agg_buy_volume,
        agg_sell_volume: input.agg_sell_volume,
        direction_bias: flow_direction,
        metrics_direction,
        risk_score,
        data_quality: clamp_score(input.data_quality),
        candidate_type,
        explain_tags,
        confidence: clamp_score((risk_score as f64 + input.data_quality) / 2.0),
    }
}

fn synthetic_perp_scenario(input: &PerpTofInput<'_>) -> PerpScenarioInput {
    let _symbol = input.symbol;
    let text = format!("{} {}", input.spot_candidate_type, input.summary).to_ascii_lowercase();
    let direction_sign = match input.spot_direction {
        TofDirection::Bullish => 1.0,
        TofDirection::Bearish => -1.0,
        _ => 0.0,
    };
    let bucket = env_f64("PERP_OI_BUCKET_SIZE", 100_000.0);
    let confidence = input.spot_confidence.clamp(0.0, 1.0);
    let oi_change = if direction_sign == 0.0 {
        0.0
    } else {
        direction_sign * bucket * (0.75 + confidence)
    };
    let funding_rate = if direction_sign > 0.0 {
        -env_f64("PERP_FUNDING_THRESHOLD", 0.05) * (1.05 + confidence * 0.7)
    } else if direction_sign < 0.0 {
        env_f64("PERP_FUNDING_THRESHOLD", 0.05) * (1.05 + confidence * 0.7)
    } else {
        0.0
    };
    let squeeze_side = if direction_sign > 0.0 {
        "short"
    } else if direction_sign < 0.0 {
        "long"
    } else {
        "neutral"
    }
    .to_string();
    let flow_base = env_f64("PERP_AGF_VOLUME_THRESHOLD", 1_000_000.0) * (0.75 + confidence);
    let (agg_buy_volume, agg_sell_volume) = if direction_sign >= 0.0 {
        (flow_base, flow_base * 0.34)
    } else {
        (flow_base * 0.34, flow_base)
    };
    let liquidation_boost = if text.contains("liquidation") || text.contains("liq") {
        12.0
    } else {
        0.0
    };
    let liquidation_window_factor = (env_duration_seconds("PERP_LIQUIDATION_WINDOW", 300) as f64
        / 300.0)
        .sqrt()
        .clamp(0.75, 1.25);
    PerpScenarioInput {
        price_change_bps: direction_sign * (8.0 + input.spot_risk_score as f64 * 0.12),
        oi_change,
        funding_rate,
        liquidation_pressure: clamp_score(
            (input.spot_risk_score as f64 * 0.72 + liquidation_boost) * liquidation_window_factor,
        ),
        squeeze_side,
        agg_buy_volume,
        agg_sell_volume,
        data_quality: input.spot_data_quality,
    }
}

fn disabled_perp_metrics(data_quality: f64) -> PerpTofMetrics {
    PerpTofMetrics {
        oi_change: 0.0,
        oi_direction: "neutral".to_string(),
        funding_rate: 0.0,
        funding_side: "neutral".to_string(),
        liquidation_pressure: 0.0,
        squeeze_side: "neutral".to_string(),
        agg_buy_volume: 0.0,
        agg_sell_volume: 0.0,
        direction_bias: TofDirection::Neutral,
        metrics_direction: TofDirection::Neutral,
        risk_score: 0,
        data_quality: clamp_score(data_quality),
        candidate_type: "PerpTofDisabled".to_string(),
        explain_tags: vec!["Perp TOF disabled".to_string()],
        confidence: 0.0,
    }
}

fn directional_vote(directions: &[TofDirection]) -> TofDirection {
    let bullish = directions
        .iter()
        .filter(|direction| **direction == TofDirection::Bullish)
        .count();
    let bearish = directions
        .iter()
        .filter(|direction| **direction == TofDirection::Bearish)
        .count();
    match bullish.cmp(&bearish) {
        std::cmp::Ordering::Greater => TofDirection::Bullish,
        std::cmp::Ordering::Less => TofDirection::Bearish,
        std::cmp::Ordering::Equal if bullish == 0 => TofDirection::Neutral,
        std::cmp::Ordering::Equal => TofDirection::Mixed,
    }
}

fn combine_direction(left: TofDirection, right: TofDirection) -> TofDirection {
    match (left, right) {
        (TofDirection::Neutral, direction) | (direction, TofDirection::Neutral) => direction,
        (left, right) if left == right => left,
        (TofDirection::Mixed, _) | (_, TofDirection::Mixed) => TofDirection::Mixed,
        _ => TofDirection::Mixed,
    }
}

fn dominant_candidate_type(candidates: &[(&str, f64)], direction: TofDirection) -> String {
    let (candidate, score) = candidates
        .iter()
        .copied()
        .max_by(|left, right| left.1.total_cmp(&right.1))
        .unwrap_or(("PerpTofCandidate", 0.0));
    if score <= 0.0 {
        return "PerpTofCandidate".to_string();
    }
    if candidate == "FundingNeutralCandidate" || candidate == "LiquidationNeutralCandidate" {
        match direction {
            TofDirection::Bullish | TofDirection::Bearish => {
                "AggressiveOrderFlowCandidate".to_string()
            }
            _ => "PerpTofCandidate".to_string(),
        }
    } else {
        candidate.to_string()
    }
}

fn final_candidate_type(direction: TofDirection, risk_score: u8) -> String {
    let risk = if risk_score >= 90 {
        "Critical"
    } else if risk_score >= 80 {
        "High Risk"
    } else if risk_score >= 65 {
        "Medium Risk"
    } else {
        "Watch"
    };
    let direction = match direction {
        TofDirection::Bullish => "Bullish",
        TofDirection::Bearish => "Bearish",
        TofDirection::Mixed => "Mixed",
        TofDirection::Neutral => "Neutral",
    };
    format!("{risk} {direction} Candidate")
}

fn explain_tags(
    oi_direction: &str,
    funding_candidate: &str,
    funding_side: &str,
    liquidation_candidate: &str,
    squeeze_side: &str,
    flow_direction: TofDirection,
) -> Vec<String> {
    let mut tags = vec![format!("OI {}", oi_direction.replace('_', " "))];
    if funding_candidate != "FundingNeutralCandidate" {
        tags.push(match funding_side {
            "long" => "Funding overheat long crowding".to_string(),
            "short" => "Funding overheat short crowding".to_string(),
            _ => "Funding neutral".to_string(),
        });
    }
    if liquidation_candidate != "LiquidationNeutralCandidate" {
        tags.push(format!("{} liquidation cluster", capitalize(squeeze_side)));
    }
    match flow_direction {
        TofDirection::Bullish => tags.push("Aggressive buy flow".to_string()),
        TofDirection::Bearish => tags.push("Aggressive sell flow".to_string()),
        _ => {}
    }
    tags.sort();
    tags.dedup();
    tags
}

fn capitalize(value: &str) -> String {
    let mut chars = value.chars();
    match chars.next() {
        Some(first) => first.to_uppercase().collect::<String>() + chars.as_str(),
        None => "Neutral".to_string(),
    }
}

fn clamp_score(value: f64) -> f64 {
    if value.is_finite() {
        value.clamp(0.0, 100.0)
    } else {
        0.0
    }
}

fn env_f64(key: &str, default: f64) -> f64 {
    std::env::var(key)
        .ok()
        .and_then(|value| value.parse::<f64>().ok())
        .filter(|value| value.is_finite() && *value > 0.0)
        .unwrap_or(default)
}

fn env_bool(key: &str, default: bool) -> bool {
    std::env::var(key)
        .ok()
        .map(|value| {
            matches!(
                value.trim().to_ascii_lowercase().as_str(),
                "1" | "true" | "yes" | "on"
            )
        })
        .unwrap_or(default)
}

fn env_duration_seconds(key: &str, default: u64) -> u64 {
    std::env::var(key)
        .ok()
        .and_then(|value| parse_duration_seconds(&value))
        .filter(|seconds| *seconds > 0)
        .unwrap_or(default)
}

fn parse_duration_seconds(value: &str) -> Option<u64> {
    let trimmed = value.trim().to_ascii_lowercase();
    if let Some(minutes) = trimmed.strip_suffix('m') {
        minutes.trim().parse::<u64>().ok().map(|value| value * 60)
    } else if let Some(seconds) = trimmed.strip_suffix('s') {
        seconds.trim().parse::<u64>().ok()
    } else {
        trimmed.parse::<u64>().ok()
    }
}
