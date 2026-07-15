use serde::{Deserialize, Serialize, Serializer};

use crate::contract_whale_monitor::types::{ContractWhaleEvidenceState, ContractWhaleSignal};
use crate::normalizers::symbol::canonical_base_asset;

const OBSERVED_CWM_OI_CHANGE_PCT_BUCKET: f64 = 0.20;
const OBSERVED_CWM_FUNDING_RATE_THRESHOLD: f64 = 0.0001;
use crate::runtime::{metric_provenance::MetricLineage, tof_metrics::TofDirection};

#[derive(Debug, Clone, Deserialize)]
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
    #[serde(default)]
    pub observed_liquidation_notional: Option<f64>,
    #[serde(default)]
    pub lineage: MetricLineage,
    #[serde(default)]
    pub liquidation_lineage: MetricLineage,
}

impl Serialize for PerpTofMetrics {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        #[derive(Serialize)]
        #[serde(rename_all = "camelCase")]
        struct Wire<'a> {
            oi_change: Option<f64>,
            oi_direction: Option<&'a str>,
            funding_rate: Option<f64>,
            funding_side: Option<&'a str>,
            liquidation_pressure: Option<f64>,
            squeeze_risk_proxy: Option<f64>,
            observed_liquidation_notional: Option<f64>,
            squeeze_side: Option<&'a str>,
            agg_buy_volume: Option<f64>,
            agg_sell_volume: Option<f64>,
            direction_bias: Option<TofDirection>,
            metrics_direction: Option<TofDirection>,
            risk_score: Option<u8>,
            data_quality: Option<f64>,
            candidate_type: &'a str,
            explain_tags: &'a [String],
            confidence: Option<f64>,
            lineage: &'a MetricLineage,
            liquidation_lineage: &'a MetricLineage,
        }
        let available = self.lineage.available;
        let liquidation_available = self.liquidation_lineage.available;
        Wire {
            oi_change: available.then_some(self.oi_change),
            oi_direction: available.then_some(self.oi_direction.as_str()),
            funding_rate: available.then_some(self.funding_rate),
            funding_side: available.then_some(self.funding_side.as_str()),
            liquidation_pressure: liquidation_available.then_some(self.liquidation_pressure),
            squeeze_risk_proxy: liquidation_available.then_some(self.liquidation_pressure),
            observed_liquidation_notional: self.observed_liquidation_notional,
            squeeze_side: liquidation_available.then_some(self.squeeze_side.as_str()),
            agg_buy_volume: available.then_some(self.agg_buy_volume),
            agg_sell_volume: available.then_some(self.agg_sell_volume),
            direction_bias: available.then_some(self.direction_bias),
            metrics_direction: available.then_some(self.metrics_direction),
            risk_score: self.lineage.alert_eligible.then_some(self.risk_score),
            data_quality: available.then_some(self.data_quality),
            candidate_type: &self.candidate_type,
            explain_tags: &self.explain_tags,
            confidence: available.then_some(self.confidence),
            lineage: &self.lineage,
            liquidation_lineage: &self.liquidation_lineage,
        }
        .serialize(serializer)
    }
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

#[derive(Debug, Clone)]
pub struct ObservedPerpSnapshot {
    pub symbol: String,
    pub observed_at_ms: i64,
    pub price_change_bps: Option<f64>,
    pub oi_change: Option<f64>,
    pub funding_rate: Option<f64>,
    pub total_volume: f64,
    pub net_volume: f64,
    pub observed_liquidation_notional: Option<f64>,
    pub squeeze_risk_proxy: Option<f64>,
    pub data_quality: Option<f64>,
}

pub fn observed_perp_snapshot_from_cwm(
    requested_symbol: &str,
    signal: &ContractWhaleSignal,
) -> Option<ObservedPerpSnapshot> {
    let requested_base = canonical_base_asset(requested_symbol)?;
    let signal_base = canonical_base_asset(&signal.symbol)?;
    if requested_base != signal_base {
        return None;
    }
    let oi_change = match signal.classification_v2.evidence.oi {
        ContractWhaleEvidenceState::Available(value) => Some(value),
        _ => None,
    };
    let funding_rate = match signal.classification_v2.evidence.funding {
        ContractWhaleEvidenceState::Available(value) => Some(value),
        _ => None,
    };
    let total_volume = signal.total_notional_usd.max(0.0);
    let net_ratio = if signal.total_volume_btc > 0.0 {
        (signal.net_volume_btc / signal.total_volume_btc).clamp(-1.0, 1.0)
    } else {
        0.0
    };
    let liquidation_status = signal
        .classification_v2
        .evidence
        .liquidation_status
        .trim()
        .to_ascii_lowercase();
    let observed_liquidation_notional = (liquidation_status == "live")
        .then_some(signal.liquidation_notional_usd)
        .filter(|value| value.is_finite() && *value >= 0.0);
    let squeeze_risk_proxy = (liquidation_status == "inferred")
        .then_some(signal.liquidation_ratio)
        .flatten()
        .filter(|value| value.is_finite())
        .map(|value| (value * 100.0).clamp(0.0, 100.0));
    Some(ObservedPerpSnapshot {
        symbol: requested_symbol.trim().to_ascii_uppercase(),
        observed_at_ms: signal.ts,
        price_change_bps: signal
            .price_move_pct
            .filter(|value| value.is_finite())
            .map(|value| value * 100.0),
        oi_change,
        funding_rate,
        total_volume,
        net_volume: total_volume * net_ratio,
        observed_liquidation_notional,
        squeeze_risk_proxy,
        data_quality: Some(signal.data_quality as f64),
    })
}

pub fn build_perp_tof_metrics(input: &PerpTofInput<'_>) -> PerpTofMetrics {
    let _ = input;
    unavailable_perp_metrics("observed_perp_evidence_unavailable")
}

pub fn build_perp_tof_metrics_from_observed(
    snapshot: &ObservedPerpSnapshot,
    requested_symbol: &str,
    candidate_at_ms: i64,
    now_ms: i64,
) -> PerpTofMetrics {
    if !snapshot.symbol.eq_ignore_ascii_case(requested_symbol) {
        return unavailable_perp_metrics("symbol_mismatch");
    }
    if !is_fresh_observation(snapshot.observed_at_ms, candidate_at_ms, now_ms) {
        return unavailable_perp_metrics("observed_perp_stale");
    }
    let has_oi = snapshot.oi_change.is_some_and(f64::is_finite);
    let has_funding = snapshot.funding_rate.is_some_and(f64::is_finite);
    let has_price_change = snapshot.price_change_bps.is_some_and(f64::is_finite);
    let total = snapshot.total_volume.max(0.0);
    let net = snapshot.net_volume.clamp(-total, total);
    let agg_buy_volume = ((total + net) / 2.0).max(0.0);
    let agg_sell_volume = ((total - net) / 2.0).max(0.0);
    let has_flow = total.is_finite() && total > 0.0;
    let observed_liquidation = snapshot
        .observed_liquidation_notional
        .filter(|value| value.is_finite() && *value >= 0.0);
    let liquidation_score = observed_liquidation
        .map(|notional| clamp_score(notional / 1_000_000.0))
        .unwrap_or(0.0);
    let squeeze_side = if net > 0.0 {
        "short"
    } else if net < 0.0 {
        "long"
    } else {
        "neutral"
    };
    let mut metrics = classify_perp_scenario_with_thresholds(
        &PerpScenarioInput {
            price_change_bps: snapshot.price_change_bps.unwrap_or(0.0),
            oi_change: snapshot.oi_change.unwrap_or(0.0),
            funding_rate: snapshot.funding_rate.unwrap_or(0.0),
            liquidation_pressure: liquidation_score,
            squeeze_side: squeeze_side.to_string(),
            agg_buy_volume,
            agg_sell_volume,
            data_quality: snapshot.data_quality.unwrap_or(0.0),
        },
        OBSERVED_CWM_OI_CHANGE_PCT_BUCKET,
        OBSERVED_CWM_FUNDING_RATE_THRESHOLD,
    );
    let complete = has_price_change && has_oi && has_funding && has_flow;
    metrics.lineage = if complete {
        MetricLineage::calculated("contract_whale_monitor", snapshot.observed_at_ms, true)
    } else {
        MetricLineage::unavailable("incomplete_observed_perp")
    };
    metrics.observed_liquidation_notional = observed_liquidation;
    if observed_liquidation.is_some() {
        metrics.liquidation_lineage =
            MetricLineage::observed("contract_whale_liquidation", snapshot.observed_at_ms, true);
    } else if let Some(proxy) = snapshot
        .squeeze_risk_proxy
        .filter(|value| value.is_finite())
    {
        metrics.liquidation_pressure = proxy.clamp(0.0, 100.0);
        metrics.liquidation_lineage =
            MetricLineage::inferred("contract_whale_squeeze_proxy", snapshot.observed_at_ms);
        metrics
            .explain_tags
            .push("Squeeze proxy display only".to_string());
    } else {
        metrics.liquidation_lineage = MetricLineage::unavailable("liquidation_flow_unavailable");
    }
    if !complete {
        metrics.risk_score = 0;
        metrics.confidence = 0.0;
        metrics.oi_direction = "unknown".to_string();
        metrics.direction_bias = TofDirection::Neutral;
        metrics.metrics_direction = TofDirection::Neutral;
        metrics.candidate_type = "PerpEvidenceUnavailable".to_string();
        metrics.explain_tags = vec!["Observed perp prerequisites incomplete".to_string()];
    }
    metrics
}

pub fn merge_spot_perp_candidate(
    spot_candidate_type: &str,
    spot_direction: TofDirection,
    spot_risk_score: u8,
    spot_tags: &[String],
    perp_metrics: &PerpTofMetrics,
) -> MergedTofCandidate {
    if !perp_metrics.lineage.alert_eligible {
        return MergedTofCandidate {
            final_candidate_type: final_candidate_type(spot_direction, spot_risk_score),
            risk_score: spot_risk_score,
            metrics_direction: spot_direction,
            confidence: spot_risk_score as f64,
            explain_tags: spot_tags.to_vec(),
        };
    }
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
    classify_funding_with_oi_bucket(
        funding_rate,
        oi_change,
        threshold,
        env_f64("PERP_OI_BUCKET_SIZE", 100_000.0),
    )
}

fn classify_funding_with_oi_bucket(
    funding_rate: f64,
    oi_change: f64,
    threshold: f64,
    oi_bucket: f64,
) -> (String, String, f64) {
    let oi_factor = (oi_change.abs() / oi_bucket).clamp(0.0, 2.0);
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
    classify_perp_scenario_with_thresholds(
        input,
        env_f64("PERP_OI_BUCKET_SIZE", 100_000.0),
        env_f64("PERP_FUNDING_THRESHOLD", 0.05),
    )
}

fn classify_perp_scenario_with_thresholds(
    input: &PerpScenarioInput,
    oi_bucket: f64,
    funding_threshold: f64,
) -> PerpTofMetrics {
    let (oi_direction, oi_metrics_direction) =
        classify_open_interest(input.price_change_bps, input.oi_change);
    let (funding_candidate, funding_side, funding_score) = classify_funding_with_oi_bucket(
        input.funding_rate,
        input.oi_change,
        funding_threshold,
        oi_bucket,
    );
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
    let oi_score = clamp_score(input.oi_change.abs() / oi_bucket * 75.0);
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
        observed_liquidation_notional: None,
        lineage: MetricLineage::calculated("observed_perp_fixture", 1, true),
        liquidation_lineage: MetricLineage::calculated("observed_liquidation_fixture", 1, true),
    }
}

fn unavailable_perp_metrics(reason: &str) -> PerpTofMetrics {
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
        data_quality: 0.0,
        candidate_type: "PerpEvidenceUnavailable".to_string(),
        explain_tags: vec!["Perp evidence unavailable".to_string()],
        confidence: 0.0,
        observed_liquidation_notional: None,
        lineage: MetricLineage::unavailable(reason),
        liquidation_lineage: MetricLineage::unavailable("liquidation_flow_unavailable"),
    }
}

fn is_fresh_observation(observed_at_ms: i64, candidate_at_ms: i64, now_ms: i64) -> bool {
    const FRESHNESS_MS: i64 = 120_000;
    const MAX_FUTURE_SKEW_MS: i64 = 5_000;
    observed_at_ms > 0
        && candidate_at_ms > 0
        && now_ms > 0
        && candidate_at_ms <= now_ms.saturating_add(MAX_FUTURE_SKEW_MS)
        && observed_at_ms <= now_ms.saturating_add(MAX_FUTURE_SKEW_MS)
        && observed_at_ms >= now_ms.saturating_sub(FRESHNESS_MS)
        && observed_at_ms <= candidate_at_ms.saturating_add(MAX_FUTURE_SKEW_MS)
        && observed_at_ms >= candidate_at_ms.saturating_sub(FRESHNESS_MS)
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
