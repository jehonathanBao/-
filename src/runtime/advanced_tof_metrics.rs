use serde::{Deserialize, Serialize, Serializer};

use crate::runtime::{
    metric_provenance::MetricLineage,
    perp_tof_metrics::PerpTofMetrics,
    tof_metrics::{relative_vpin_score, TofDirection, TofMetrics},
};

const OBSERVED_CWM_OI_CHANGE_PCT_BUCKET: f64 = 0.20;
const OBSERVED_CWM_FUNDING_RATE_THRESHOLD: f64 = 0.0001;

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AdvancedTofMetrics {
    pub vpin_enhanced: f64,
    pub large_order_flow_cluster: f64,
    pub historical_funding_oi_trend: f64,
    pub market_pressure_heatmap: f64,
    pub spot_risk_score: u8,
    pub spot_tof_score: f64,
    pub perp_score: u8,
    pub final_risk_score: u8,
    pub data_quality: f64,
    pub metrics_completeness: f64,
    pub fresh_data_coverage: f64,
    pub candidate_type: String,
    pub final_candidate_type: String,
    pub metrics_direction: TofDirection,
    pub confidence: f64,
    pub explain_tags: Vec<String>,
    #[serde(default)]
    pub lineage: MetricLineage,
}

impl Serialize for AdvancedTofMetrics {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        #[derive(Serialize)]
        #[serde(rename_all = "camelCase")]
        struct Wire<'a> {
            vpin_enhanced: Option<f64>,
            large_order_flow_cluster: Option<f64>,
            historical_funding_oi_trend: Option<f64>,
            market_pressure_heatmap: Option<f64>,
            spot_risk_score: u8,
            spot_tof_score: Option<f64>,
            perp_score: Option<u8>,
            final_risk_score: u8,
            data_quality: Option<f64>,
            metrics_completeness: Option<f64>,
            fresh_data_coverage: Option<f64>,
            candidate_type: &'a str,
            final_candidate_type: &'a str,
            metrics_direction: Option<TofDirection>,
            confidence: Option<f64>,
            explain_tags: &'a [String],
            lineage: &'a MetricLineage,
        }
        let available = self.lineage.available;
        Wire {
            vpin_enhanced: available.then_some(self.vpin_enhanced),
            large_order_flow_cluster: available.then_some(self.large_order_flow_cluster),
            historical_funding_oi_trend: available.then_some(self.historical_funding_oi_trend),
            market_pressure_heatmap: available.then_some(self.market_pressure_heatmap),
            spot_risk_score: self.spot_risk_score,
            spot_tof_score: available.then_some(self.spot_tof_score),
            perp_score: available.then_some(self.perp_score),
            final_risk_score: self.final_risk_score,
            data_quality: available.then_some(self.data_quality),
            metrics_completeness: available.then_some(self.metrics_completeness),
            fresh_data_coverage: available.then_some(self.fresh_data_coverage),
            candidate_type: &self.candidate_type,
            final_candidate_type: &self.final_candidate_type,
            metrics_direction: available.then_some(self.metrics_direction),
            confidence: available.then_some(self.confidence),
            explain_tags: &self.explain_tags,
            lineage: &self.lineage,
        }
        .serialize(serializer)
    }
}

#[derive(Debug, Clone)]
pub struct AdvancedTofInput<'a> {
    pub symbol: &'a str,
    pub spot_candidate_type: &'a str,
    pub spot_direction: TofDirection,
    pub spot_risk_score: u8,
    pub spot_data_quality: f64,
    pub spot_confidence: f64,
    pub tof_metrics: &'a TofMetrics,
    pub spot_tags: &'a [String],
    pub perp_metrics: &'a PerpTofMetrics,
    pub summary: &'a str,
}

pub fn build_advanced_tof_metrics(input: &AdvancedTofInput<'_>) -> AdvancedTofMetrics {
    if !env_bool("ADVANCED_TOF_ENABLED", true)
        || !input.tof_metrics.lineage.alert_eligible
        || !input.perp_metrics.lineage.alert_eligible
    {
        return disabled_advanced_metrics(input);
    }
    let vpin_enhanced = vpin_enhanced_score(
        relative_vpin_score(
            input.tof_metrics.vpin_zscore,
            input.tof_metrics.vpin_percentile,
        ),
        input.tof_metrics.trade_imbalance_score,
        input.tof_metrics.depth_withdrawal_score,
        input.tof_metrics.spread_widening_score,
    );
    let large_order_flow_cluster = large_order_flow_cluster_score(
        input.tof_metrics.order_churn_score,
        input.perp_metrics.agg_buy_volume,
        input.perp_metrics.agg_sell_volume,
    );
    let historical_funding_oi_trend = historical_funding_oi_trend_score_with_thresholds(
        input.perp_metrics.funding_rate,
        input.perp_metrics.oi_change,
        OBSERVED_CWM_FUNDING_RATE_THRESHOLD,
        OBSERVED_CWM_OI_CHANGE_PCT_BUCKET,
    );
    let alert_eligible_liquidation_pressure =
        if input.perp_metrics.liquidation_lineage.alert_eligible {
            input.perp_metrics.liquidation_pressure
        } else {
            0.0
        };
    let market_pressure_heatmap = market_pressure_heatmap_score(
        input.spot_direction,
        input.tof_metrics.metrics_direction,
        input.perp_metrics.metrics_direction,
        alert_eligible_liquidation_pressure,
        vpin_enhanced,
    );
    let metrics_direction = directional_vote(&[
        input.spot_direction,
        input.tof_metrics.metrics_direction,
        input.perp_metrics.metrics_direction,
    ]);
    let final_risk_score = input.spot_risk_score;
    let metrics_completeness = clamp_score(
        0.45 * input.tof_metrics.metrics_completeness * 100.0
            + 0.30 * completeness_from_perp(input.perp_metrics)
            + 0.25
                * indicator_completeness(&[
                    vpin_enhanced,
                    large_order_flow_cluster,
                    historical_funding_oi_trend,
                    market_pressure_heatmap,
                ]),
    );
    let fresh_data_coverage = fresh_data_coverage(input.spot_confidence, input.perp_metrics);
    let data_quality = clamp_score(input.spot_data_quality);
    let mut explain_tags = input.spot_tags.to_vec();
    explain_tags.extend(input.perp_metrics.explain_tags.clone());
    explain_tags.extend(advanced_explain_tags(
        vpin_enhanced,
        large_order_flow_cluster,
        historical_funding_oi_trend,
        market_pressure_heatmap,
    ));
    explain_tags.push(input.spot_candidate_type.to_string());
    explain_tags.sort();
    explain_tags.dedup();
    AdvancedTofMetrics {
        vpin_enhanced,
        large_order_flow_cluster,
        historical_funding_oi_trend,
        market_pressure_heatmap,
        spot_risk_score: input.spot_risk_score,
        spot_tof_score: input.tof_metrics.tof_score,
        perp_score: input.perp_metrics.risk_score,
        final_risk_score,
        data_quality,
        metrics_completeness,
        fresh_data_coverage,
        candidate_type: dominant_advanced_candidate_type(
            input.summary,
            vpin_enhanced,
            large_order_flow_cluster,
            historical_funding_oi_trend,
            market_pressure_heatmap,
        ),
        final_candidate_type: final_candidate_type(metrics_direction, final_risk_score),
        metrics_direction,
        confidence: clamp_score(
            (final_risk_score as f64 + data_quality + metrics_completeness) / 3.0,
        ),
        explain_tags,
        lineage: MetricLineage::calculated(
            "advanced_observed_formula_v1",
            input
                .tof_metrics
                .lineage
                .observed_at_ms
                .unwrap_or_default()
                .min(
                    input
                        .perp_metrics
                        .lineage
                        .observed_at_ms
                        .unwrap_or_default(),
                ),
            true,
        ),
    }
}

fn disabled_advanced_metrics(input: &AdvancedTofInput<'_>) -> AdvancedTofMetrics {
    let final_risk_score = input.spot_risk_score;
    let data_quality = clamp_score(input.spot_data_quality);
    let mut explain_tags = input.spot_tags.to_vec();
    explain_tags.push("Advanced TOF disabled".to_string());
    explain_tags.sort();
    explain_tags.dedup();
    AdvancedTofMetrics {
        vpin_enhanced: 0.0,
        large_order_flow_cluster: 0.0,
        historical_funding_oi_trend: 0.0,
        market_pressure_heatmap: 0.0,
        spot_risk_score: input.spot_risk_score,
        spot_tof_score: input.tof_metrics.tof_score,
        perp_score: input.perp_metrics.risk_score,
        final_risk_score,
        data_quality,
        metrics_completeness: input.tof_metrics.metrics_completeness * 100.0,
        fresh_data_coverage: input.spot_confidence.clamp(0.0, 1.0) * 100.0,
        candidate_type: "AdvancedTofDisabled".to_string(),
        final_candidate_type: final_candidate_type(input.spot_direction, final_risk_score),
        metrics_direction: input.spot_direction,
        confidence: clamp_score((final_risk_score as f64 + data_quality) / 2.0),
        explain_tags,
        lineage: MetricLineage::unavailable("advanced_observed_inputs_unavailable"),
    }
}

pub fn fused_risk_score(spot_risk_score: u8, spot_tof_score: f64, perp_score: u8) -> u8 {
    clamp_score(0.4 * spot_risk_score as f64 + 0.3 * spot_tof_score + 0.3 * perp_score as f64)
        .round() as u8
}

pub fn fused_data_quality(
    spot_data_quality: f64,
    perp_data_quality: f64,
    metrics_completeness: f64,
    fresh_data_coverage: f64,
) -> f64 {
    clamp_score(
        0.45 * spot_data_quality
            + 0.25 * perp_data_quality
            + 0.18 * metrics_completeness
            + 0.12 * fresh_data_coverage,
    )
}

pub fn vpin_enhanced_score(
    relative_vpin_score: f64,
    trade_imbalance_score: f64,
    depth_withdrawal_score: f64,
    spread_widening_score: f64,
) -> f64 {
    clamp_score(
        0.45 * relative_vpin_score
            + 0.25 * trade_imbalance_score
            + 0.20 * depth_withdrawal_score
            + 0.10 * spread_widening_score,
    )
}

pub fn large_order_flow_cluster_score(
    order_churn_score: f64,
    agg_buy_volume: f64,
    agg_sell_volume: f64,
) -> f64 {
    let total = (agg_buy_volume + agg_sell_volume).max(1.0);
    let imbalance = ((agg_buy_volume - agg_sell_volume) / total)
        .abs()
        .clamp(0.0, 1.0)
        * 100.0;
    let volume_pressure =
        (total / env_f64("PERP_AGF_VOLUME_THRESHOLD", 1_000_000.0)).clamp(0.0, 2.0) * 35.0;
    clamp_score(0.45 * order_churn_score + 0.35 * imbalance + 0.20 * volume_pressure)
}

pub fn historical_funding_oi_trend_score(funding_rate: f64, oi_change: f64) -> f64 {
    let funding_threshold = env_f64("PERP_FUNDING_THRESHOLD", 0.05);
    let oi_bucket = env_f64("PERP_OI_BUCKET_SIZE", 100_000.0);
    historical_funding_oi_trend_score_with_thresholds(
        funding_rate,
        oi_change,
        funding_threshold,
        oi_bucket,
    )
}

fn historical_funding_oi_trend_score_with_thresholds(
    funding_rate: f64,
    oi_change: f64,
    funding_threshold: f64,
    oi_bucket: f64,
) -> f64 {
    let funding_pressure = (funding_rate.abs() / funding_threshold).clamp(0.0, 2.0) * 45.0;
    let oi_pressure = (oi_change.abs() / oi_bucket).clamp(0.0, 2.0) * 35.0;
    clamp_score(funding_pressure + oi_pressure)
}

pub fn market_pressure_heatmap_score(
    spot_direction: TofDirection,
    spot_tof_direction: TofDirection,
    perp_direction: TofDirection,
    liquidation_pressure: f64,
    vpin_enhanced: f64,
) -> f64 {
    let alignment = directional_alignment(&[spot_direction, spot_tof_direction, perp_direction]);
    clamp_score(0.35 * liquidation_pressure + 0.35 * vpin_enhanced + 0.30 * alignment)
}

fn completeness_from_perp(metrics: &PerpTofMetrics) -> f64 {
    let mut indicators = vec![
        metrics.oi_change.abs(),
        metrics.funding_rate.abs(),
        metrics.agg_buy_volume + metrics.agg_sell_volume,
    ];
    if metrics.liquidation_lineage.alert_eligible {
        indicators.push(metrics.liquidation_pressure);
    }
    indicator_completeness(&indicators)
}

fn indicator_completeness(values: &[f64]) -> f64 {
    if values.is_empty() {
        return 0.0;
    }
    let present = values
        .iter()
        .filter(|value| value.is_finite() && value.abs() > 0.0)
        .count();
    (present as f64 / values.len() as f64) * 100.0
}

fn fresh_data_coverage(spot_confidence: f64, perp_metrics: &PerpTofMetrics) -> f64 {
    let perp_coverage = completeness_from_perp(perp_metrics);
    clamp_score(0.55 * spot_confidence.clamp(0.0, 1.0) * 100.0 + 0.45 * perp_coverage)
}

fn advanced_explain_tags(
    vpin_enhanced: f64,
    large_order_flow_cluster: f64,
    historical_funding_oi_trend: f64,
    market_pressure_heatmap: f64,
) -> Vec<String> {
    let mut tags = Vec::new();
    if vpin_enhanced >= 75.0 {
        tags.push("VPIN enhanced elevated".to_string());
    }
    if large_order_flow_cluster >= 70.0 {
        tags.push("Large order flow cluster".to_string());
    }
    if historical_funding_oi_trend >= 70.0 {
        tags.push("Historical funding/OI trend".to_string());
    }
    if market_pressure_heatmap >= 70.0 {
        tags.push("Market pressure heatmap".to_string());
    }
    tags
}

fn dominant_advanced_candidate_type(
    summary: &str,
    vpin_enhanced: f64,
    large_order_flow_cluster: f64,
    historical_funding_oi_trend: f64,
    market_pressure_heatmap: f64,
) -> String {
    let text = summary.to_ascii_lowercase();
    let mut candidates = [
        ("VpinEnhancedCandidate", vpin_enhanced),
        ("LargeOrderFlowClusterCandidate", large_order_flow_cluster),
        (
            "HistoricalFundingOiTrendCandidate",
            historical_funding_oi_trend,
        ),
        ("MarketPressureHeatmapCandidate", market_pressure_heatmap),
    ];
    if text.contains("cluster") || text.contains("large") || text.contains("whale") {
        candidates[1].1 += 8.0;
    }
    candidates
        .iter()
        .copied()
        .max_by(|left, right| left.1.total_cmp(&right.1))
        .map(|(candidate, _)| candidate.to_string())
        .unwrap_or_else(|| "AdvancedTofCandidate".to_string())
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
    format!("{risk} {direction} Advanced Candidate")
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

fn directional_alignment(directions: &[TofDirection]) -> f64 {
    match directional_vote(directions) {
        TofDirection::Bullish => {
            let aligned = directions
                .iter()
                .filter(|direction| **direction == TofDirection::Bullish)
                .count();
            (aligned as f64 / directions.len().max(1) as f64) * 100.0
        }
        TofDirection::Bearish => {
            let aligned = directions
                .iter()
                .filter(|direction| **direction == TofDirection::Bearish)
                .count();
            (aligned as f64 / directions.len().max(1) as f64) * 100.0
        }
        TofDirection::Mixed => 45.0,
        TofDirection::Neutral => 20.0,
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
