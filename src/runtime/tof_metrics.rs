use serde::{Deserialize, Serialize};

const EPSILON: f64 = 1e-9;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum TofDirection {
    Bullish,
    Bearish,
    Neutral,
    Mixed,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
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
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DirectionResolution {
    pub final_direction: TofDirection,
    pub direction_label: String,
    pub direction_confidence: f64,
    pub direction_source: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
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
    clamp_score(
        0.25 * metrics.vpin_proxy
            + 0.20 * metrics.trade_imbalance.abs() * 100.0
            + 0.20 * metrics.depth_withdrawal_score
            + 0.15 * metrics.spread_widening_score
            + 0.10 * metrics.order_churn_score
            + 0.10 * metrics.liquidity_vacuum_score,
    )
}

pub fn final_risk_score(existing_risk_score: f64, tof_score: f64) -> u8 {
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
    let direction_sign = match detector_direction {
        TofDirection::Bullish => 1.0,
        TofDirection::Bearish => -1.0,
        _ => 0.0,
    };
    let confidence = input.confidence.clamp(0.0, 1.0);
    let trade_imbalance = direction_sign * (0.28 + confidence * 0.45).min(0.88);
    let trade_imbalance_score = trade_imbalance.abs() * 100.0;
    let vpin_proxy =
        clamp_score(0.65 * trade_imbalance_score + 0.35 * input.existing_risk_score as f64);
    let depth_base = (20.0 + input.existing_risk_score as f64 * 0.55).min(92.0);
    let (bid_depth_withdrawal, ask_depth_withdrawal) = match detector_direction {
        TofDirection::Bearish => (depth_base, 10.0 + confidence * 12.0),
        TofDirection::Bullish => (10.0 + confidence * 12.0, depth_base),
        _ => (15.0, 15.0),
    };
    let depth_withdrawal_score = bid_depth_withdrawal.max(ask_depth_withdrawal);
    let spread_bps = inferred_spread_bps(input.signal_kind, input.summary);
    let spread_widening_score =
        spread_widening_score(spread_bps, env_f64("TOF_SPREAD_WIDENING_BPS", 8.0));
    let book_update_rate = inferred_book_update_rate(input.signal_kind);
    let trade_rate = if input.summary.to_ascii_lowercase().contains("成交") {
        32.0
    } else {
        12.0
    };
    let order_churn_score = order_churn_score(
        book_update_rate,
        trade_rate,
        env_f64("TOF_ORDER_CHURN_THRESHOLD", 70.0),
    );
    let liquidity_vacuum_score = liquidity_vacuum_score(input.signal_kind, depth_withdrawal_score);
    let thin_side = thin_side(bid_depth_withdrawal, ask_depth_withdrawal);
    let metrics_direction = resolve_metrics_direction(
        trade_imbalance,
        bid_depth_withdrawal,
        ask_depth_withdrawal,
        env_f64("TOF_DEPTH_WITHDRAWAL_THRESHOLD", 35.0),
    );
    let metrics_completeness = metrics_completeness(input.quality_bucket);
    let mut metrics = TofMetrics {
        trade_imbalance,
        trade_imbalance_score,
        vpin_proxy,
        vpin_bucket_count: env_usize("TOF_VPIN_BUCKET_COUNT", 20),
        vpin_window_volume: env_f64("TOF_VPIN_BUCKET_VOLUME", 100_000.0)
            * env_usize("TOF_VPIN_BUCKET_COUNT", 20) as f64,
        bid_depth_withdrawal,
        ask_depth_withdrawal,
        depth_withdrawal_score,
        spread_bps,
        spread_widening_score,
        order_churn_score,
        book_update_rate,
        trade_rate,
        liquidity_vacuum_score,
        thin_side,
        metrics_direction,
        metrics_confidence: clamp_score(confidence * 100.0 * metrics_completeness),
        tof_score: 0.0,
        final_risk_score: input.existing_risk_score,
        metrics_completeness,
    };
    metrics.tof_score = tof_score(&metrics);
    metrics.final_risk_score =
        final_risk_score(input.existing_risk_score as f64, metrics.tof_score);
    let resolution = resolve_final_direction(
        detector_direction,
        metrics.metrics_direction,
        metrics.metrics_confidence,
    );
    let explain_tags = explain_tags(input.signal_kind, &metrics, resolution.final_direction);
    TofSignalEnhancement {
        candidate_type: candidate_type(input.signal_kind, &metrics),
        tof_score: metrics.tof_score,
        final_risk_score: metrics.final_risk_score,
        tof_metrics: metrics,
        explain_tags,
        direction: resolution.final_direction,
        direction_label: resolution.direction_label,
        direction_confidence: resolution.direction_confidence,
        direction_source: resolution.direction_source,
    }
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
    } else if metrics.vpin_proxy >= env_f64("TOF_VPIN_HIGH_THRESHOLD", 70.0) {
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

fn inferred_spread_bps(signal_kind: &str, summary: &str) -> f64 {
    let text = format!("{} {}", signal_kind, summary).to_ascii_lowercase();
    if text.contains("spread") || text.contains("价差") {
        12.0
    } else if text.contains("liquidity") || text.contains("流动性") {
        8.4
    } else {
        3.5
    }
}

fn inferred_book_update_rate(signal_kind: &str) -> f64 {
    let kind = signal_kind.to_ascii_lowercase();
    if kind.contains("spoof") || kind.contains("layer") {
        130.0
    } else if kind.contains("liquidity") || kind.contains("wall") {
        86.0
    } else {
        48.0
    }
}

fn liquidity_vacuum_score(signal_kind: &str, depth_withdrawal_score: f64) -> f64 {
    let kind = signal_kind.to_ascii_lowercase();
    let boost = if kind.contains("liquidity") || kind.contains("thin") {
        18.0
    } else {
        0.0
    };
    clamp_score(depth_withdrawal_score * 0.80 + boost)
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

fn metrics_completeness(quality_bucket: &str) -> f64 {
    match quality_bucket.to_ascii_lowercase().as_str() {
        "excellent" => 1.0,
        "good" => 0.9,
        "mixed" => 0.78,
        "weak" => 0.62,
        "bad" => 0.52,
        _ => 0.45,
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

fn env_usize(key: &str, default: usize) -> usize {
    std::env::var(key)
        .ok()
        .and_then(|value| value.parse::<usize>().ok())
        .filter(|value| *value > 0)
        .unwrap_or(default)
}
