use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};

use crate::{
    replay::candidate_replay_runner::{run_candidate_replay_file, CandidateReplaySummary},
    types::toxic_signal::ToxicSignal,
};

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ScoreBucketReport {
    pub bucket: String,
    pub signal_count: usize,
    pub average_markout_1s_bps: Option<f64>,
    pub average_markout_5s_bps: Option<f64>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CandidateCalibrationReport {
    pub score_buckets: Vec<ScoreBucketReport>,
    pub detector_average_markout_5s_bps: BTreeMap<String, Option<f64>>,
    pub venue_average_data_quality: BTreeMap<String, f64>,
    pub high_score_without_markout_signal_ids: Vec<String>,
    pub low_score_high_markout_signal_ids: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CandidateReplayCalibrationReport {
    pub summary: CandidateReplaySummary,
    pub calibration: CandidateCalibrationReport,
    pub production_data_ready: bool,
    pub read_only: bool,
}

pub fn run_candidate_calibration_file(
    path: impl AsRef<std::path::Path>,
) -> anyhow::Result<CandidateReplayCalibrationReport> {
    let summary = run_candidate_replay_file(path)?;
    let calibration = build_calibration_report(&summary.signals);
    Ok(CandidateReplayCalibrationReport {
        production_data_ready: summary.total_events > 0,
        summary,
        calibration,
        read_only: true,
    })
}

pub fn build_calibration_report(signals: &[ToxicSignal]) -> CandidateCalibrationReport {
    let buckets = [
        (0, 20, "0-20"),
        (20, 40, "20-40"),
        (40, 60, "40-60"),
        (60, 80, "60-80"),
        (80, 101, "80-100"),
    ];
    let score_buckets = buckets
        .into_iter()
        .map(|(min, max, label)| bucket_report(signals, min, max, label))
        .collect();

    CandidateCalibrationReport {
        score_buckets,
        detector_average_markout_5s_bps: average_markout_by_detector(signals),
        venue_average_data_quality: average_data_quality_by_venue(signals),
        high_score_without_markout_signal_ids: signals
            .iter()
            .filter(|signal| signal.toxicity_score >= 80)
            .filter(|signal| {
                signal
                    .evidence
                    .as_ref()
                    .and_then(|evidence| evidence.markout_5s_bps)
                    .is_none()
            })
            .map(|signal| signal.signal_id.clone())
            .collect(),
        low_score_high_markout_signal_ids: signals
            .iter()
            .filter(|signal| signal.toxicity_score < 40)
            .filter(|signal| {
                signal
                    .evidence
                    .as_ref()
                    .and_then(|evidence| evidence.markout_5s_bps)
                    .is_some_and(|markout| markout >= 5.0)
            })
            .map(|signal| signal.signal_id.clone())
            .collect(),
    }
}

fn bucket_report(signals: &[ToxicSignal], min: u8, max: u8, label: &str) -> ScoreBucketReport {
    let bucket_signals = signals
        .iter()
        .filter(|signal| signal.toxicity_score >= min && signal.toxicity_score < max)
        .collect::<Vec<_>>();
    ScoreBucketReport {
        bucket: label.to_string(),
        signal_count: bucket_signals.len(),
        average_markout_1s_bps: average_optional(bucket_signals.iter().filter_map(|signal| {
            signal
                .evidence
                .as_ref()
                .and_then(|evidence| evidence.markout_1s_bps)
        })),
        average_markout_5s_bps: average_optional(bucket_signals.iter().filter_map(|signal| {
            signal
                .evidence
                .as_ref()
                .and_then(|evidence| evidence.markout_5s_bps)
        })),
    }
}

fn average_markout_by_detector(signals: &[ToxicSignal]) -> BTreeMap<String, Option<f64>> {
    let mut grouped: BTreeMap<String, Vec<f64>> = BTreeMap::new();
    for signal in signals {
        if let Some(markout) = signal
            .evidence
            .as_ref()
            .and_then(|evidence| evidence.markout_5s_bps)
        {
            grouped
                .entry(format!("{:?}", signal.signal_type))
                .or_default()
                .push(markout);
        } else {
            grouped
                .entry(format!("{:?}", signal.signal_type))
                .or_default();
        }
    }
    grouped
        .into_iter()
        .map(|(key, values)| (key, average_optional(values.into_iter())))
        .collect()
}

fn average_data_quality_by_venue(signals: &[ToxicSignal]) -> BTreeMap<String, f64> {
    let mut totals: BTreeMap<String, (f64, usize)> = BTreeMap::new();
    for signal in signals {
        let Some(evidence) = signal.evidence.as_ref() else {
            continue;
        };
        let entry = totals.entry(evidence.venue.clone()).or_insert((0.0, 0));
        entry.0 += signal.data_quality.unwrap_or(0.0);
        entry.1 += 1;
    }
    totals
        .into_iter()
        .map(|(venue, (total, count))| (venue, total / count.max(1) as f64))
        .collect()
}

fn average_optional(values: impl Iterator<Item = f64>) -> Option<f64> {
    let mut total = 0.0;
    let mut count = 0;
    for value in values {
        total += value;
        count += 1;
    }
    (count > 0).then_some(total / count as f64)
}
