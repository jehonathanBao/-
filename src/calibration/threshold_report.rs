use super::calibration_types::{CalibrationRecommendation, CalibrationRunSummary};

pub fn build_recommendations(
    baseline: &CalibrationRunSummary,
    threshold_comparison: &[CalibrationRunSummary],
    toxic_ratio_comparison: &[CalibrationRunSummary],
    vpin_parameter_comparison: &[CalibrationRunSummary],
    liq_hunt_score_comparison: &[CalibrationRunSummary],
) -> Vec<CalibrationRecommendation> {
    let mut recommendations = Vec::new();

    if let Some(best_threshold) = threshold_comparison
        .iter()
        .max_by(|left, right| left.hit_rate.total_cmp(&right.hit_rate))
    {
        recommendations.push(CalibrationRecommendation {
            title: "Threshold Comparison".to_string(),
            detail: format!(
                "Best hit rate in the threshold sweep came from {:.0} BTC (hit_rate {:.2}, false_positive_rate {:.2}); baseline is {:.0} BTC.",
                best_threshold.toxic_threshold_btc,
                best_threshold.hit_rate,
                best_threshold.false_positive_rate,
                baseline.toxic_threshold_btc
            ),
        });
    }

    if let Some(best_ratio) = toxic_ratio_comparison
        .iter()
        .max_by(|left, right| left.hit_rate.total_cmp(&right.hit_rate))
    {
        recommendations.push(CalibrationRecommendation {
            title: "Toxic Ratio Comparison".to_string(),
            detail: format!(
                "Min toxic ratio {:.2} produced hit_rate {:.2} and false_positive_rate {:.2}.",
                best_ratio.min_toxic_ratio, best_ratio.hit_rate, best_ratio.false_positive_rate
            ),
        });
    }

    if let Some(best_vpin) = vpin_parameter_comparison
        .iter()
        .max_by(|left, right| left.hit_rate.total_cmp(&right.hit_rate))
    {
        recommendations.push(CalibrationRecommendation {
            title: "VPIN Parameter Comparison".to_string(),
            detail: format!(
                "Best VPIN sweep used bucket_size {:.0} BTC, lookback {}, z-score {:.1} (hit_rate {:.2}).",
                best_vpin.vpin_bucket_size_btc,
                best_vpin.vpin_lookback_buckets,
                best_vpin.vpin_spike_zscore,
                best_vpin.hit_rate
            ),
        });
    }

    if let Some(best_liq_hunt) = liq_hunt_score_comparison
        .iter()
        .max_by(|left, right| left.hit_rate.total_cmp(&right.hit_rate))
    {
        recommendations.push(CalibrationRecommendation {
            title: "Liq Hunt Score Comparison".to_string(),
            detail: format!(
                "Likely {:.0} / Active {:.0} gave the best liq hunt sweep result (hit_rate {:.2}, false_positive_rate {:.2}).",
                best_liq_hunt.liq_hunt_likely_score,
                best_liq_hunt.liq_hunt_active_score,
                best_liq_hunt.hit_rate,
                best_liq_hunt.false_positive_rate
            ),
        });
    }

    recommendations
}
