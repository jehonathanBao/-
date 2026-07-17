use crate::types::{regime::RegimeContext, toxic_signal::ScoreBreakdown};

pub fn combine_score_breakdown(breakdown: &ScoreBreakdown) -> f64 {
    let raw_score = breakdown.toxicity_score * 0.45
        + breakdown.confidence * 0.25
        + breakdown.data_quality * 0.15
        + breakdown.markout_evidence * 0.10
        + breakdown.liquidity_impact * 0.05;
    let quality_adjusted = if breakdown.data_quality < 50.0 {
        raw_score * (breakdown.data_quality / 100.0)
    } else {
        raw_score
    };
    quality_adjusted.clamp(0.0, 100.0)
}

/// Apply regime confidence gate + optional global score factor.
///
/// Remains analysis-only: never opens Discord/Telegram gates or enforcement paths.
pub fn combine_score_breakdown_with_regime(
    breakdown: &ScoreBreakdown,
    regime_ctx: &RegimeContext,
) -> f64 {
    let mut score = combine_score_breakdown(breakdown);
    let min_confidence = regime_ctx
        .multipliers
        .get("min_confidence")
        .copied()
        .unwrap_or(0.0)
        .clamp(0.0, 1.0);
    if regime_ctx.confidence + f64::EPSILON < min_confidence {
        score *= 0.85;
    }
    let factor = regime_ctx
        .multipliers
        .get("global_alert_factor")
        .copied()
        .filter(|value| value.is_finite() && *value > 0.0)
        .unwrap_or(1.0)
        .clamp(0.50, 1.50);
    // Higher alert factor → slightly lower displayed candidate score (stricter).
    // Lower factor → slightly higher score (more sensitive).
    score *= 2.0 - factor;
    score.clamp(0.0, 100.0)
}
