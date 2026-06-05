use crate::types::toxic_signal::ScoreBreakdown;

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
