//! Stable robust baselines for impact-grade scoring.

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ImpactBaselineKey {
    pub symbol: String,
    pub window_sec: u64,
    pub threshold_profile: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RobustImpactBaseline {
    pub key: ImpactBaselineKey,
    pub sample_count: usize,
    pub median_log_volume: f64,
    pub mad_log_volume: f64,
    pub sorted_log_samples: Vec<f64>,
}

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RobustImpactScore {
    pub percentile: f64,
    pub robust_z: f64,
    pub sample_count: usize,
}

pub fn build_robust_impact_baseline(
    key: ImpactBaselineKey,
    samples: impl IntoIterator<Item = f64>,
    min_samples: usize,
) -> Option<RobustImpactBaseline> {
    let mut sorted_log_samples: Vec<f64> = samples
        .into_iter()
        .filter(|value| value.is_finite() && *value > 0.0)
        .map(f64::ln)
        .filter(|value| value.is_finite())
        .collect();
    sorted_log_samples.sort_by(f64::total_cmp);
    if sorted_log_samples.len() < min_samples {
        return None;
    }
    let median_log_volume = median(&sorted_log_samples);
    let mut deviations: Vec<f64> = sorted_log_samples
        .iter()
        .map(|value| (value - median_log_volume).abs())
        .collect();
    deviations.sort_by(f64::total_cmp);
    let mad_log_volume = median(&deviations);
    if !median_log_volume.is_finite() || !mad_log_volume.is_finite() || mad_log_volume <= 0.0 {
        return None;
    }
    Some(RobustImpactBaseline {
        key,
        sample_count: sorted_log_samples.len(),
        median_log_volume,
        mad_log_volume,
        sorted_log_samples,
    })
}

pub fn score_event_impact(
    current_volume: f64,
    baseline: &RobustImpactBaseline,
) -> Option<RobustImpactScore> {
    if !current_volume.is_finite() || current_volume <= 0.0 || baseline.sample_count < 2 {
        return None;
    }
    let log_volume = current_volume.ln();
    if !log_volume.is_finite() || baseline.mad_log_volume <= 0.0 {
        return None;
    }
    let upper_bound = baseline
        .sorted_log_samples
        .partition_point(|sample| *sample <= log_volume);
    let percentile = if upper_bound == 0 {
        0.0
    } else if baseline.sample_count == 1 {
        100.0
    } else {
        ((upper_bound - 1) as f64 / (baseline.sample_count - 1) as f64 * 100.0).clamp(0.0, 100.0)
    };
    let robust_z = (log_volume - baseline.median_log_volume) / (1.4826 * baseline.mad_log_volume);
    if !robust_z.is_finite() {
        return None;
    }
    Some(RobustImpactScore {
        percentile,
        robust_z,
        sample_count: baseline.sample_count,
    })
}

fn median(values: &[f64]) -> f64 {
    let middle = values.len() / 2;
    if values.len() % 2 == 0 {
        (values[middle - 1] + values[middle]) / 2.0
    } else {
        values[middle]
    }
}
