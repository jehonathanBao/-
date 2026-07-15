use std::collections::BTreeMap;

use serde::{Deserialize, Serialize, Serializer};

use crate::runtime::metric_provenance::MetricLineage;

const EPSILON: f64 = 1e-9;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum TofDirection {
    Bullish,
    Bearish,
    Neutral,
    Mixed,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TofMetrics {
    pub trade_imbalance: f64,
    pub trade_imbalance_score: f64,
    pub vpin_proxy: f64,
    pub vpin_bucket_count: usize,
    pub vpin_window_volume: f64,
    pub bid_depth_withdrawal: f64,
    pub ask_depth_withdrawal: f64,
    pub depth_withdrawal_score: f64,
    pub spread_bps: f64,
    pub spread_widening_score: f64,
    pub order_churn_score: f64,
    pub book_update_rate: f64,
    pub trade_rate: f64,
    pub liquidity_vacuum_score: f64,
    pub thin_side: String,
    pub metrics_direction: TofDirection,
    pub metrics_confidence: f64,
    pub tof_score: f64,
    pub final_risk_score: u8,
    pub metrics_completeness: f64,
    #[serde(default)]
    pub vpin_zscore: Option<f64>,
    #[serde(default)]
    pub vpin_percentile: Option<f64>,
    #[serde(default)]
    pub per_venue_vpin: BTreeMap<String, f64>,
    #[serde(default)]
    pub lineage: MetricLineage,
    #[serde(default)]
    pub metric_lineage: BTreeMap<String, MetricLineage>,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct TofMetricsWire<'a> {
    trade_imbalance: Option<f64>,
    trade_imbalance_score: Option<f64>,
    vpin_proxy: Option<f64>,
    vpin_bucket_count: Option<usize>,
    vpin_window_volume: Option<f64>,
    vpin_zscore: Option<f64>,
    vpin_percentile: Option<f64>,
    per_venue_vpin: Option<&'a BTreeMap<String, f64>>,
    bid_depth_withdrawal: Option<f64>,
    ask_depth_withdrawal: Option<f64>,
    depth_withdrawal_score: Option<f64>,
    spread_bps: Option<f64>,
    spread_widening_score: Option<f64>,
    order_churn_score: Option<f64>,
    book_update_rate: Option<f64>,
    trade_rate: Option<f64>,
    liquidity_vacuum_score: Option<f64>,
    thin_side: Option<&'a str>,
    metrics_direction: Option<TofDirection>,
    metrics_confidence: Option<f64>,
    tof_score: Option<f64>,
    toxicity_hazard_score: Option<f64>,
    final_risk_score: u8,
    metrics_completeness: Option<f64>,
    lineage: &'a MetricLineage,
    metric_lineage: &'a BTreeMap<String, MetricLineage>,
}

impl Serialize for TofMetrics {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        let visible = |key: &str| {
            self.metric_lineage
                .get(key)
                .unwrap_or(&self.lineage)
                .available
        };
        let metric = |key: &str, value: f64| visible(key).then_some(value);
        TofMetricsWire {
            trade_imbalance: metric("tradeImbalance", self.trade_imbalance),
            trade_imbalance_score: metric("tradeImbalance", self.trade_imbalance_score),
            vpin_proxy: metric("vpin", self.vpin_proxy),
            vpin_bucket_count: visible("vpin").then_some(self.vpin_bucket_count),
            vpin_window_volume: metric("vpin", self.vpin_window_volume),
            vpin_zscore: visible("vpin").then_some(self.vpin_zscore).flatten(),
            vpin_percentile: visible("vpin").then_some(self.vpin_percentile).flatten(),
            per_venue_vpin: visible("vpin").then_some(&self.per_venue_vpin),
            bid_depth_withdrawal: metric("depth", self.bid_depth_withdrawal),
            ask_depth_withdrawal: metric("depth", self.ask_depth_withdrawal),
            depth_withdrawal_score: metric("depth", self.depth_withdrawal_score),
            spread_bps: metric("spread", self.spread_bps),
            spread_widening_score: metric("spread", self.spread_widening_score),
            order_churn_score: metric("bookUpdateRate", self.order_churn_score),
            book_update_rate: metric("bookUpdateRate", self.book_update_rate),
            trade_rate: metric("tradeRate", self.trade_rate),
            liquidity_vacuum_score: metric("liquidityVacuum", self.liquidity_vacuum_score),
            thin_side: visible("depth").then_some(self.thin_side.as_str()),
            metrics_direction: self.lineage.available.then_some(self.metrics_direction),
            metrics_confidence: metric("hazard", self.metrics_confidence),
            tof_score: metric("hazard", self.tof_score),
            toxicity_hazard_score: metric("hazard", self.tof_score),
            final_risk_score: self.final_risk_score,
            metrics_completeness: metric("hazard", self.metrics_completeness),
            lineage: &self.lineage,
            metric_lineage: &self.metric_lineage,
        }
        .serialize(serializer)
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DirectionResolution {
    pub final_direction: TofDirection,
    pub direction_label: String,
    pub direction_confidence: f64,
    pub direction_source: String,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TofSignalEnhancement {
    pub tof_metrics: TofMetrics,
    pub candidate_type: String,
    pub explain_tags: Vec<String>,
    pub direction: TofDirection,
    pub direction_label: String,
    pub direction_confidence: f64,
    pub direction_source: String,
    pub tof_score: f64,
    pub final_risk_score: u8,
}

impl Serialize for TofSignalEnhancement {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        #[derive(Serialize)]
        #[serde(rename_all = "camelCase")]
        struct Wire<'a> {
            tof_metrics: &'a TofMetrics,
            candidate_type: &'a str,
            explain_tags: &'a [String],
            direction: TofDirection,
            direction_label: &'a str,
            direction_confidence: f64,
            direction_source: &'a str,
            tof_score: Option<f64>,
            toxicity_hazard_score: Option<f64>,
            final_risk_score: u8,
        }
        let hazard = self.tof_metrics.lineage.available.then_some(self.tof_score);
        Wire {
            tof_metrics: &self.tof_metrics,
            candidate_type: &self.candidate_type,
            explain_tags: &self.explain_tags,
            direction: self.direction,
            direction_label: &self.direction_label,
            direction_confidence: self.direction_confidence,
            direction_source: &self.direction_source,
            tof_score: hazard,
            toxicity_hazard_score: hazard,
            final_risk_score: self.final_risk_score,
        }
        .serialize(serializer)
    }
}

#[derive(Debug, Clone)]
pub struct ObservedTofSnapshot {
    pub symbol: String,
    pub observed_at_ms: i64,
    pub buy_volume: f64,
    pub sell_volume: f64,
    pub trade_count: u64,
    pub window_ms: u64,
    pub vpin: Option<f64>,
    pub vpin_zscore: Option<f64>,
    pub vpin_percentile: Option<f64>,
    pub vpin_bucket_count: usize,
    pub vpin_window_volume: f64,
    pub per_venue_vpin: BTreeMap<String, f64>,
    pub bid_depth_withdrawal: Option<f64>,
    pub ask_depth_withdrawal: Option<f64>,
    pub spread_bps: Option<f64>,
    pub book_update_rate: Option<f64>,
    pub sweep_score: Option<f64>,
}

#[derive(Debug, Clone)]
pub struct TofSummaryInput<'a> {
    pub signal_kind: &'a str,
    pub direction_bias: &'a str,
    pub severity: &'a str,
    pub confidence: f64,
    pub quality_bucket: &'a str,
    pub summary: &'a str,
    pub existing_risk_score: u8,
    pub existing_data_quality: f64,
}

pub fn trade_imbalance(buy_volume: f64, sell_volume: f64) -> f64 {
    ((buy_volume - sell_volume) / (buy_volume + sell_volume).max(EPSILON)).clamp(-1.0, 1.0)
}

pub fn vpin_proxy(bucket_imbalances: &[f64]) -> f64 {
    if bucket_imbalances.is_empty() {
        return 0.0;
    }
    let avg_abs = bucket_imbalances
        .iter()
        .map(|value| value.abs().clamp(0.0, 1.0))
        .sum::<f64>()
        / bucket_imbalances.len() as f64;
    clamp_score(avg_abs * 100.0)
}

pub fn relative_vpin_score(vpin_zscore: Option<f64>, vpin_percentile: Option<f64>) -> f64 {
    let zscore_score = vpin_zscore
        .filter(|value| value.is_finite())
        .map(|value| clamp_score(value.max(0.0) / 2.5 * 100.0));
    let percentile_score = vpin_percentile
        .filter(|value| value.is_finite())
        .map(|value| clamp_score((value.clamp(0.0, 1.0) - 0.5) * 200.0));

    match (zscore_score, percentile_score) {
        (Some(zscore), Some(percentile)) => clamp_score(0.40 * zscore + 0.60 * percentile),
        (Some(zscore), None) => zscore,
        (None, Some(percentile)) => percentile,
        (None, None) => 0.0,
    }
}

pub fn depth_withdrawal(before: f64, after: f64) -> f64 {
    if before <= 0.0 {
        return 0.0;
    }
    (((before - after).max(0.0) / before) * 100.0).clamp(0.0, 100.0)
}

pub fn spread_bps(best_bid: f64, best_ask: f64) -> f64 {
    let mid = (best_bid + best_ask) / 2.0;
    if best_bid <= 0.0 || best_ask <= 0.0 || best_ask < best_bid || mid <= 0.0 {
        return 0.0;
    }
    ((best_ask - best_bid) / mid) * 10_000.0
}

pub fn spread_widening_score(spread_bps: f64, threshold_bps: f64) -> f64 {
    if threshold_bps <= 0.0 {
        return 0.0;
    }
    clamp_score((spread_bps / threshold_bps) * 70.0)
}

pub fn order_churn_score(book_update_rate: f64, trade_rate: f64, threshold: f64) -> f64 {
    if threshold <= 0.0 {
        return 0.0;
    }
    let quiet_trade_factor = (1.0 - (trade_rate / (book_update_rate + EPSILON))).clamp(0.0, 1.0);
    clamp_score((book_update_rate / threshold) * 70.0 * quiet_trade_factor)
}

pub fn tof_score(metrics: &TofMetrics) -> f64 {
    let relative_vpin = relative_vpin_score(metrics.vpin_zscore, metrics.vpin_percentile);
    clamp_score(
        0.25 * relative_vpin
            + 0.20 * metrics.trade_imbalance.abs() * 100.0
            + 0.20 * metrics.depth_withdrawal_score
            + 0.15 * metrics.spread_widening_score
            + 0.10 * metrics.order_churn_score
            + 0.10 * metrics.liquidity_vacuum_score,
    )
}

pub fn final_risk_score(existing_risk_score: f64, tof_score: f64) -> u8 {
    let existing_risk_score = clamp_score(existing_risk_score);
    let tof_score = clamp_score(tof_score);
    let existing_weight = env_f64("TOF_SCORE_WEIGHT_EXISTING", 0.60);
    let metrics_weight = env_f64("TOF_SCORE_WEIGHT_METRICS", 0.40);
    let weight_sum = existing_weight + metrics_weight;
    let weighted_score = if weight_sum > 0.0 {
        (existing_risk_score * existing_weight + tof_score * metrics_weight) / weight_sum
    } else {
        existing_risk_score
    };
    clamp_score(weighted_score).round() as u8
}

pub fn resolve_metrics_direction(
    trade_imbalance: f64,
    bid_depth_withdrawal: f64,
    ask_depth_withdrawal: f64,
    threshold: f64,
) -> TofDirection {
    let trade_direction = if trade_imbalance > 0.25 {
        TofDirection::Bullish
    } else if trade_imbalance < -0.25 {
        TofDirection::Bearish
    } else {
        TofDirection::Neutral
    };
    let depth_direction = match (
        bid_depth_withdrawal > threshold,
        ask_depth_withdrawal > threshold,
    ) {
        (true, false) => TofDirection::Bearish,
        (false, true) => TofDirection::Bullish,
        (true, true) => TofDirection::Mixed,
        (false, false) => TofDirection::Neutral,
    };
    combine_directions(trade_direction, depth_direction)
}

pub fn resolve_final_direction(
    detector_direction: TofDirection,
    metrics_direction: TofDirection,
    metrics_confidence: f64,
) -> DirectionResolution {
    let final_direction = match (detector_direction, metrics_direction) {
        (TofDirection::Neutral, direction) => direction,
        (direction, TofDirection::Neutral) => direction,
        (left, right) if left == right => left,
        (TofDirection::Mixed, _) | (_, TofDirection::Mixed) => TofDirection::Mixed,
        _ => TofDirection::Mixed,
    };
    let direction_source = match (detector_direction, metrics_direction, final_direction) {
        (left, right, final_direction) if left == right && left == final_direction => {
            "detector+tof_metrics"
        }
        (_, TofDirection::Neutral, _) => "detector",
        (TofDirection::Neutral, _, _) => "tof_metrics",
        (_, _, TofDirection::Mixed) => "conflict_detector_tof_metrics",
        _ => "detector+tof_metrics",
    }
    .to_string();
    let direction_confidence = match direction_source.as_str() {
        "detector+tof_metrics" => (metrics_confidence + 12.0).min(95.0),
        "conflict_detector_tof_metrics" => (metrics_confidence * 0.55).max(30.0),
        _ => metrics_confidence.clamp(35.0, 82.0),
    };
    DirectionResolution {
        final_direction,
        direction_label: direction_label(final_direction).to_string(),
        direction_confidence,
        direction_source,
    }
}

pub fn enhance_signal_summary(input: &TofSummaryInput<'_>) -> TofSignalEnhancement {
    let detector_direction = direction_from_text(input.direction_bias);
    let metrics = unavailable_tof_metrics(input.existing_risk_score, "observed_tof_unavailable");
    TofSignalEnhancement {
        candidate_type: input.signal_kind.to_string(),
        tof_score: 0.0,
        final_risk_score: input.existing_risk_score,
        tof_metrics: metrics,
        explain_tags: Vec::new(),
        direction: detector_direction,
        direction_label: direction_label(detector_direction).to_string(),
        direction_confidence: clamp_score(input.confidence.clamp(0.0, 1.0) * 100.0),
        direction_source: "detector".to_string(),
    }
}

pub fn build_tof_metrics_from_observed(
    snapshot: &ObservedTofSnapshot,
    requested_symbol: &str,
    candidate_at_ms: i64,
    now_ms: i64,
    detector_risk_score: u8,
) -> TofMetrics {
    if !snapshot.symbol.eq_ignore_ascii_case(requested_symbol) {
        return unavailable_tof_metrics(detector_risk_score, "symbol_mismatch");
    }
    let fresh = is_fresh_observation(snapshot.observed_at_ms, candidate_at_ms, now_ms);
    if !fresh {
        return unavailable_tof_metrics(detector_risk_score, "observed_tof_stale");
    }

    let total_volume = snapshot.buy_volume + snapshot.sell_volume;
    let has_flow = total_volume.is_finite() && total_volume > EPSILON;
    let has_vpin = snapshot.vpin.is_some_and(|value| value.is_finite());
    let has_depth = snapshot
        .bid_depth_withdrawal
        .zip(snapshot.ask_depth_withdrawal)
        .is_some_and(|(bid, ask)| bid.is_finite() && ask.is_finite());
    let has_spread = snapshot.spread_bps.is_some_and(|value| value.is_finite());
    let sweep_detected = snapshot
        .sweep_score
        .is_some_and(|value| value.is_finite() && value > 0.0);
    let has_liquidity = has_depth || has_spread || sweep_detected;

    let mut metric_lineage = BTreeMap::new();
    metric_lineage.insert(
        "tradeImbalance".to_string(),
        if has_flow {
            MetricLineage::calculated("flow_window_service", snapshot.observed_at_ms, true)
        } else {
            MetricLineage::unavailable("flow_window_empty")
        },
    );
    metric_lineage.insert(
        "tradeRate".to_string(),
        if snapshot.window_ms > 0 && snapshot.trade_count > 0 {
            MetricLineage::calculated("flow_window_service", snapshot.observed_at_ms, true)
        } else {
            MetricLineage::unavailable("trade_rate_unavailable")
        },
    );
    metric_lineage.insert(
        "vpin".to_string(),
        if has_vpin {
            MetricLineage::observed("vpin_service", snapshot.observed_at_ms, true)
        } else {
            MetricLineage::unavailable("vpin_window_incomplete")
        },
    );
    metric_lineage.insert(
        "depth".to_string(),
        if has_depth {
            MetricLineage::calculated("sweep_service_l2", snapshot.observed_at_ms, true)
        } else {
            MetricLineage::unavailable("l2_depth_unavailable")
        },
    );
    metric_lineage.insert(
        "spread".to_string(),
        if has_spread {
            MetricLineage::observed("flow_window_service_l2", snapshot.observed_at_ms, true)
        } else {
            MetricLineage::unavailable("l2_spread_unavailable")
        },
    );
    metric_lineage.insert(
        "bookUpdateRate".to_string(),
        if snapshot.book_update_rate.is_some() {
            MetricLineage::observed("book_event_rate", snapshot.observed_at_ms, true)
        } else {
            MetricLineage::unavailable("book_update_rate_unavailable")
        },
    );
    metric_lineage.insert(
        "liquidityVacuum".to_string(),
        if has_liquidity {
            MetricLineage::calculated("observed_liquidity_inputs", snapshot.observed_at_ms, true)
        } else {
            MetricLineage::unavailable("liquidity_inputs_unavailable")
        },
    );
    metric_lineage.insert(
        "sweep".to_string(),
        if snapshot.sweep_score.is_some_and(|value| value.is_finite()) {
            MetricLineage::observed("sweep_service", snapshot.observed_at_ms, true)
        } else {
            MetricLineage::unavailable("sweep_evidence_unavailable")
        },
    );

    let trade_imbalance = if has_flow {
        trade_imbalance(snapshot.buy_volume, snapshot.sell_volume)
    } else {
        0.0
    };
    let trade_rate = if snapshot.window_ms > 0 {
        snapshot.trade_count as f64 / (snapshot.window_ms as f64 / 1_000.0)
    } else {
        0.0
    };
    let bid_depth_withdrawal = snapshot.bid_depth_withdrawal.unwrap_or(0.0);
    let ask_depth_withdrawal = snapshot.ask_depth_withdrawal.unwrap_or(0.0);
    let depth_withdrawal_score = bid_depth_withdrawal.max(ask_depth_withdrawal);
    let spread_bps = snapshot.spread_bps.unwrap_or(0.0);
    let spread_widening_score = if has_spread {
        spread_widening_score(spread_bps, env_f64("TOF_SPREAD_WIDENING_BPS", 8.0))
    } else {
        0.0
    };
    let book_update_rate = snapshot.book_update_rate.unwrap_or(0.0);
    let order_churn_score = snapshot
        .book_update_rate
        .map(|rate| order_churn_score(rate, trade_rate, env_f64("TOF_ORDER_CHURN_THRESHOLD", 70.0)))
        .unwrap_or(0.0);
    let liquidity_vacuum_score = snapshot
        .sweep_score
        .unwrap_or(depth_withdrawal_score)
        .clamp(0.0, 100.0);
    let metrics_direction = if has_flow || has_depth {
        resolve_metrics_direction(
            trade_imbalance,
            bid_depth_withdrawal,
            ask_depth_withdrawal,
            env_f64("TOF_DEPTH_WITHDRAWAL_THRESHOLD", 35.0),
        )
    } else {
        TofDirection::Neutral
    };
    let complete = has_flow && has_vpin && has_liquidity;
    let lineage = if complete {
        MetricLineage::calculated("observed_tof_formula_v1", snapshot.observed_at_ms, true)
    } else {
        MetricLineage::unavailable("incomplete_observed_tof")
    };
    let present = [
        has_flow,
        has_vpin,
        has_depth,
        has_spread,
        snapshot.book_update_rate.is_some(),
        snapshot.sweep_score.is_some(),
    ]
    .into_iter()
    .filter(|value| *value)
    .count();
    let metrics_completeness = present as f64 / 6.0;
    let mut metrics = TofMetrics {
        trade_imbalance,
        trade_imbalance_score: trade_imbalance.abs() * 100.0,
        vpin_proxy: snapshot.vpin.unwrap_or(0.0).clamp(0.0, 1.0) * 100.0,
        vpin_bucket_count: snapshot.vpin_bucket_count,
        vpin_window_volume: snapshot.vpin_window_volume,
        bid_depth_withdrawal,
        ask_depth_withdrawal,
        depth_withdrawal_score,
        spread_bps,
        spread_widening_score,
        order_churn_score,
        book_update_rate,
        trade_rate,
        liquidity_vacuum_score,
        thin_side: thin_side(bid_depth_withdrawal, ask_depth_withdrawal),
        metrics_direction,
        metrics_confidence: metrics_completeness * 100.0,
        tof_score: 0.0,
        final_risk_score: detector_risk_score,
        metrics_completeness,
        vpin_zscore: snapshot.vpin_zscore,
        vpin_percentile: snapshot.vpin_percentile,
        per_venue_vpin: snapshot.per_venue_vpin.clone(),
        lineage,
        metric_lineage,
    };
    if complete {
        metrics.tof_score = tof_score(&metrics);
        metrics.metric_lineage.insert(
            "hazard".to_string(),
            MetricLineage::calculated("observed_tof_formula_v1", snapshot.observed_at_ms, true),
        );
    } else {
        metrics.metric_lineage.insert(
            "hazard".to_string(),
            MetricLineage::unavailable("incomplete_observed_tof"),
        );
    }
    metrics
}

pub fn enhance_signal_with_observed(
    input: &TofSummaryInput<'_>,
    snapshot: &ObservedTofSnapshot,
    candidate_at_ms: i64,
    now_ms: i64,
) -> TofSignalEnhancement {
    let metrics = build_tof_metrics_from_observed(
        snapshot,
        snapshot.symbol.as_str(),
        candidate_at_ms,
        now_ms,
        input.existing_risk_score,
    );
    let detector_direction = direction_from_text(input.direction_bias);
    let mut tags = explain_tags(input.signal_kind, &metrics, detector_direction);
    if !metrics.lineage.alert_eligible {
        tags.push("observed_tof_incomplete".to_string());
    }
    TofSignalEnhancement {
        candidate_type: candidate_type(input.signal_kind, &metrics),
        tof_score: metrics.tof_score,
        final_risk_score: input.existing_risk_score,
        tof_metrics: metrics,
        explain_tags: tags,
        direction: detector_direction,
        direction_label: direction_label(detector_direction).to_string(),
        direction_confidence: clamp_score(input.confidence.clamp(0.0, 1.0) * 100.0),
        direction_source: "detector".to_string(),
    }
}

fn unavailable_tof_metrics(detector_risk_score: u8, reason: &str) -> TofMetrics {
    TofMetrics {
        trade_imbalance: 0.0,
        trade_imbalance_score: 0.0,
        vpin_proxy: 0.0,
        vpin_bucket_count: 0,
        vpin_window_volume: 0.0,
        bid_depth_withdrawal: 0.0,
        ask_depth_withdrawal: 0.0,
        depth_withdrawal_score: 0.0,
        spread_bps: 0.0,
        spread_widening_score: 0.0,
        order_churn_score: 0.0,
        book_update_rate: 0.0,
        trade_rate: 0.0,
        liquidity_vacuum_score: 0.0,
        thin_side: "unavailable".to_string(),
        metrics_direction: TofDirection::Neutral,
        metrics_confidence: 0.0,
        tof_score: 0.0,
        final_risk_score: detector_risk_score,
        metrics_completeness: 0.0,
        vpin_zscore: None,
        vpin_percentile: None,
        per_venue_vpin: BTreeMap::new(),
        lineage: MetricLineage::unavailable(reason),
        metric_lineage: BTreeMap::new(),
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

pub fn direction_label(direction: TofDirection) -> &'static str {
    match direction {
        TofDirection::Bullish => "🟢 看涨 / Bid-Buy",
        TofDirection::Bearish => "🔴 看跌 / Ask-Sell",
        TofDirection::Neutral => "🟡 中性 / 未知",
        TofDirection::Mixed => "🟡 混合 / 冲突",
    }
}

pub fn direction_from_text(raw: &str) -> TofDirection {
    let value = raw.to_ascii_lowercase();
    if value.contains("bid")
        || value.contains("buy")
        || value.contains("long")
        || value.contains("bull")
    {
        TofDirection::Bullish
    } else if value.contains("ask")
        || value.contains("sell")
        || value.contains("short")
        || value.contains("bear")
    {
        TofDirection::Bearish
    } else if value.contains("mixed") || value.contains("conflict") {
        TofDirection::Mixed
    } else {
        TofDirection::Neutral
    }
}

fn combine_directions(left: TofDirection, right: TofDirection) -> TofDirection {
    match (left, right) {
        (TofDirection::Neutral, direction) | (direction, TofDirection::Neutral) => direction,
        (left, right) if left == right => left,
        (TofDirection::Mixed, _) | (_, TofDirection::Mixed) => TofDirection::Mixed,
        _ => TofDirection::Mixed,
    }
}

fn candidate_type(signal_kind: &str, metrics: &TofMetrics) -> String {
    let kind = signal_kind.to_ascii_lowercase();
    if kind.contains("spoof") || kind.contains("layer") {
        "spoofing_candidate".to_string()
    } else if metrics.liquidity_vacuum_score >= 65.0 {
        "liquidity_vacuum_candidate".to_string()
    } else if relative_vpin_score(metrics.vpin_zscore, metrics.vpin_percentile)
        >= env_f64("TOF_RELATIVE_VPIN_HIGH_THRESHOLD", 70.0)
    {
        "vpin_toxicity_candidate".to_string()
    } else if metrics.spread_widening_score >= 70.0 {
        "spread_widening_candidate".to_string()
    } else if metrics.depth_withdrawal_score >= env_f64("TOF_DEPTH_WITHDRAWAL_THRESHOLD", 35.0) {
        "depth_withdrawal_candidate".to_string()
    } else {
        "toxic_flow_candidate".to_string()
    }
}

fn explain_tags(signal_kind: &str, metrics: &TofMetrics, direction: TofDirection) -> Vec<String> {
    let mut tags = Vec::new();
    if relative_vpin_score(metrics.vpin_zscore, metrics.vpin_percentile)
        >= env_f64("TOF_RELATIVE_VPIN_HIGH_THRESHOLD", 70.0)
    {
        tags.push("relative_vpin_spike".to_string());
    }
    if metrics.vpin_proxy >= env_f64("TOF_VPIN_HIGH_THRESHOLD", 70.0) {
        tags.push("high_vpin_proxy".to_string());
    }
    if metrics.trade_imbalance < -0.25 {
        tags.push("sell_volume_imbalance".to_string());
    } else if metrics.trade_imbalance > 0.25 {
        tags.push("buy_volume_imbalance".to_string());
    }
    if metrics.bid_depth_withdrawal >= env_f64("TOF_DEPTH_WITHDRAWAL_THRESHOLD", 35.0) {
        tags.push("bid_depth_withdrawal".to_string());
    }
    if metrics.ask_depth_withdrawal >= env_f64("TOF_DEPTH_WITHDRAWAL_THRESHOLD", 35.0) {
        tags.push("ask_depth_withdrawal".to_string());
    }
    if metrics.spread_widening_score >= 70.0 {
        tags.push("spread_widening".to_string());
    }
    if metrics.order_churn_score >= 60.0 || signal_kind.to_ascii_lowercase().contains("spoof") {
        tags.push("order_churn_pressure".to_string());
    }
    if direction == TofDirection::Mixed {
        tags.push("direction_conflict".to_string());
    }
    tags.sort();
    tags.dedup();
    tags
}

fn thin_side(bid_depth_withdrawal: f64, ask_depth_withdrawal: f64) -> String {
    let threshold = env_f64("TOF_DEPTH_WITHDRAWAL_THRESHOLD", 35.0);
    match (
        bid_depth_withdrawal >= threshold,
        ask_depth_withdrawal >= threshold,
    ) {
        (true, true) => "both",
        (true, false) => "bid",
        (false, true) => "ask",
        (false, false) => "none",
    }
    .to_string()
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
