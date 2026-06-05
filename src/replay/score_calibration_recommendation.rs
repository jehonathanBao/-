use serde::{Deserialize, Serialize};

use crate::replay::production_report::ProductionReplayReport;

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ScoreCalibrationRecommendation {
    pub should_adjust_weights: bool,
    pub reason: String,
    pub recommended_actions: Vec<String>,
    pub warning: Option<String>,
}

pub fn recommend_score_calibration(
    report: &ProductionReplayReport,
) -> ScoreCalibrationRecommendation {
    let mut actions = Vec::new();

    if report.total_signals < 30 {
        return ScoreCalibrationRecommendation {
            should_adjust_weights: false,
            reason: "not_enough_replay_samples".to_string(),
            recommended_actions: vec![
                "Run replay on a larger reviewed historical L2/trade dataset.".to_string(),
                "Keep current scoreBreakdown weights unchanged.".to_string(),
            ],
            warning: Some(
                "Actual score weight tuning requires labeled FP/FN or reviewed production replay evidence."
                    .to_string(),
            ),
        };
    }

    actions.push(
        "Review possible_false_positives and possible_false_negatives before tuning.".to_string(),
    );

    let low_quality_high_score_count = report
        .high_score_candidates
        .iter()
        .filter(|signal| signal.data_quality.unwrap_or(0.0) < 70.0)
        .count();
    if low_quality_high_score_count > 0 {
        actions.push(
            "Consider increasing data_quality weighting or raising the Discord alert gate."
                .to_string(),
        );
    }

    let high_bucket_markout = bucket_markout(report, "80-100");
    let low_bucket_markout = bucket_markout(report, "20-40")
        .or_else(|| bucket_markout(report, "0-20"))
        .unwrap_or(0.0);
    if let Some(high_markout) = high_bucket_markout {
        if high_markout.abs() <= low_bucket_markout.abs() {
            actions.push(
                "High-score bucket markout is not clearly stronger than low-score buckets; review detector evidence and scoreBreakdown."
                    .to_string(),
            );
        }
    } else {
        actions.push(
            "High-score bucket lacks resolved markout; collect more future price evidence."
                .to_string(),
        );
    }

    ScoreCalibrationRecommendation {
        should_adjust_weights: false,
        reason: "manual_review_required_before_weight_changes".to_string(),
        recommended_actions: actions,
        warning: Some(
            "No automatic scoreBreakdown weight changes were applied by this tool.".to_string(),
        ),
    }
}

fn bucket_markout(report: &ProductionReplayReport, bucket: &str) -> Option<f64> {
    report
        .calibration
        .score_buckets
        .iter()
        .find(|item| item.bucket == bucket)
        .and_then(|item| item.average_markout_5s_bps)
}
