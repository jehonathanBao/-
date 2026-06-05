use std::{
    collections::BTreeMap,
    fs,
    path::{Path, PathBuf},
};

use anyhow::Context;
use chrono::Utc;
use serde::{Deserialize, Serialize};

use crate::{
    replay::{
        calibration_report::{build_calibration_report, CandidateCalibrationReport},
        candidate_replay_event::{
            load_candidate_replay_file, CandidateReplayEvent, CandidateReplayEventType,
        },
        candidate_replay_runner::{run_candidate_replay_events, CandidateReplaySummary},
        replay_config::{AlertGateConfig, ProductionReplayConfig},
        score_calibration_recommendation::{
            recommend_score_calibration, ScoreCalibrationRecommendation,
        },
    },
    types::toxic_signal::ToxicSignal,
};

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ProductionReplayReport {
    pub candidate_only_notice: String,
    pub total_events: usize,
    pub total_trades: usize,
    pub total_book_deltas: usize,
    pub total_snapshots: usize,
    pub total_snapshot_resets: usize,
    pub total_signals: usize,
    pub signals_by_type: BTreeMap<String, usize>,
    pub signals_by_symbol: BTreeMap<String, usize>,
    pub signals_by_venue: BTreeMap<String, usize>,
    pub average_score: f64,
    pub max_score: u8,
    pub average_data_quality: f64,
    pub calibration: CandidateCalibrationReport,
    pub detector_avg_markout: BTreeMap<String, DetectorMarkoutReport>,
    pub venue_data_quality: BTreeMap<String, f64>,
    pub high_score_candidates: Vec<ToxicSignal>,
    pub possible_false_positives: Vec<ToxicSignal>,
    pub possible_false_negatives: Vec<ToxicSignal>,
    pub signals: Vec<ToxicSignal>,
    pub recommendation: ScoreCalibrationRecommendation,
    pub read_only: bool,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DetectorMarkoutReport {
    pub markout_1s_bps: Option<f64>,
    pub markout_5s_bps: Option<f64>,
    pub markout_30s_bps: Option<f64>,
}

#[derive(Debug, Clone)]
pub struct ProductionReplayOutputPaths {
    pub report_dir: PathBuf,
    pub summary_json: Option<PathBuf>,
    pub signals_json: Option<PathBuf>,
    pub calibration_json: Option<PathBuf>,
    pub calibration_md: Option<PathBuf>,
    pub high_score_candidates_csv: Option<PathBuf>,
    pub possible_false_positives_csv: Option<PathBuf>,
    pub possible_false_negatives_csv: Option<PathBuf>,
}

pub fn run_production_replay(
    config: &ProductionReplayConfig,
) -> anyhow::Result<ProductionReplayReport> {
    let events = load_candidate_replay_file(config.input_path()).with_context(|| {
        format!(
            "production replay input is unavailable; put real L2/trade data at {}",
            config.input_path().display()
        )
    })?;
    let events = prepare_events(events, config);
    let counts = EventCounts::from_events(&events);
    let summary = run_candidate_replay_events(config.input_path().display().to_string(), events);
    Ok(build_production_report(summary, counts, config.alert_gate))
}

pub fn write_production_report(
    report: &ProductionReplayReport,
    config: &ProductionReplayConfig,
) -> anyhow::Result<ProductionReplayOutputPaths> {
    let report_dir = production_report_dir(config);
    fs::create_dir_all(&report_dir)
        .with_context(|| format!("failed to create report dir {}", report_dir.display()))?;

    let mut paths = ProductionReplayOutputPaths {
        report_dir: report_dir.clone(),
        summary_json: None,
        signals_json: None,
        calibration_json: None,
        calibration_md: None,
        high_score_candidates_csv: None,
        possible_false_positives_csv: None,
        possible_false_negatives_csv: None,
    };

    if config.output.write_json {
        paths.summary_json = Some(write_json(
            &report_dir.join("summary.json"),
            &summary_without_signal_lists(report),
        )?);
        paths.signals_json = Some(write_json(
            &report_dir.join("signals.json"),
            &report_signals(report),
        )?);
        paths.calibration_json = Some(write_json(
            &report_dir.join("calibration.json"),
            &report.calibration,
        )?);
    }

    if config.output.write_markdown {
        let path = report_dir.join("calibration.md");
        fs::write(&path, render_markdown(report))
            .with_context(|| format!("failed to write {}", path.display()))?;
        paths.calibration_md = Some(path);
    }

    if config.output.write_csv {
        paths.high_score_candidates_csv = Some(write_signal_csv(
            &report_dir.join("high_score_candidates.csv"),
            &report.high_score_candidates,
        )?);
        paths.possible_false_positives_csv = Some(write_signal_csv(
            &report_dir.join("possible_false_positives.csv"),
            &report.possible_false_positives,
        )?);
        paths.possible_false_negatives_csv = Some(write_signal_csv(
            &report_dir.join("possible_false_negatives.csv"),
            &report.possible_false_negatives,
        )?);
    }

    Ok(paths)
}

fn prepare_events(
    mut events: Vec<CandidateReplayEvent>,
    config: &ProductionReplayConfig,
) -> Vec<CandidateReplayEvent> {
    if config.replay.sort_by_ts {
        events.sort_by_key(|event| event.ts_ms);
    }
    events.retain(|event| {
        let after_start =
            config.replay.start_ts_ms == 0 || event.ts_ms >= config.replay.start_ts_ms;
        let before_end = config.replay.end_ts_ms == 0 || event.ts_ms <= config.replay.end_ts_ms;
        after_start && before_end
    });
    if config.replay.max_events > 0 {
        events.truncate(config.replay.max_events);
    }
    events
}

fn build_production_report(
    summary: CandidateReplaySummary,
    counts: EventCounts,
    gate: AlertGateConfig,
) -> ProductionReplayReport {
    let calibration = build_calibration_report(&summary.signals);
    let high_score_candidates = summary
        .signals
        .iter()
        .filter(|signal| {
            signal.toxicity_score >= gate.min_score
                && signal.data_quality.unwrap_or(0.0) >= gate.min_data_quality
        })
        .cloned()
        .collect::<Vec<_>>();
    let possible_false_positives = summary
        .signals
        .iter()
        .filter(|signal| signal.toxicity_score >= gate.min_score)
        .filter(|signal| {
            signal
                .evidence
                .as_ref()
                .and_then(|evidence| evidence.markout_5s_bps)
                .is_none_or(|markout| markout.abs() < 1.0)
        })
        .cloned()
        .collect::<Vec<_>>();
    let possible_false_negatives = summary
        .signals
        .iter()
        .filter(|signal| signal.toxicity_score < gate.min_score)
        .filter(|signal| {
            signal
                .evidence
                .as_ref()
                .and_then(|evidence| evidence.markout_5s_bps)
                .is_some_and(|markout| markout.abs() >= 5.0)
        })
        .cloned()
        .collect::<Vec<_>>();
    let report_stub = ProductionReplayReport {
        candidate_only_notice: "Candidate only. Not confirmed manipulation.".to_string(),
        total_events: counts.total_events,
        total_trades: counts.total_trades,
        total_book_deltas: counts.total_book_deltas,
        total_snapshots: counts.total_snapshots,
        total_snapshot_resets: counts.total_snapshot_resets,
        total_signals: summary.total_signals,
        signals_by_type: summary.signals_by_type.clone(),
        signals_by_symbol: summary.signals_by_symbol.clone(),
        signals_by_venue: signals_by_venue(&summary.signals),
        average_score: summary.average_score,
        max_score: summary.max_score,
        average_data_quality: summary.data_quality_average,
        detector_avg_markout: detector_markout(&summary.signals),
        venue_data_quality: calibration.venue_average_data_quality.clone(),
        calibration,
        high_score_candidates,
        possible_false_positives,
        possible_false_negatives,
        signals: summary.signals.clone(),
        recommendation: ScoreCalibrationRecommendation::default(),
        read_only: true,
    };
    let recommendation = recommend_score_calibration(&report_stub);
    ProductionReplayReport {
        recommendation,
        ..report_stub
    }
}

fn production_report_dir(config: &ProductionReplayConfig) -> PathBuf {
    let venue = config.input.venue.as_deref().unwrap_or("venue");
    let symbol = config.input.symbol.as_deref().unwrap_or("symbol");
    let timestamp = Utc::now().format("%Y%m%dT%H%M%SZ");
    config.output_root().join(format!(
        "{}_{}_{}",
        safe_name(venue),
        safe_name(symbol),
        timestamp
    ))
}

fn safe_name(value: &str) -> String {
    value
        .chars()
        .map(|ch| {
            if ch.is_ascii_alphanumeric() || ch == '-' || ch == '_' {
                ch
            } else {
                '_'
            }
        })
        .collect()
}

fn write_json<T: Serialize>(path: &Path, value: &T) -> anyhow::Result<PathBuf> {
    let json = serde_json::to_string_pretty(value)?;
    fs::write(path, json).with_context(|| format!("failed to write {}", path.display()))?;
    Ok(path.to_path_buf())
}

fn write_signal_csv(path: &Path, signals: &[ToxicSignal]) -> anyhow::Result<PathBuf> {
    let mut out =
        String::from("signal_id,symbol,signal_type,score,data_quality,markout_5s_bps,reason\n");
    for signal in signals {
        let markout = signal
            .evidence
            .as_ref()
            .and_then(|evidence| evidence.markout_5s_bps)
            .map(|value| value.to_string())
            .unwrap_or_default();
        out.push_str(&format!(
            "{},{},{:?},{},{},{},{}\n",
            csv_escape(&signal.signal_id),
            csv_escape(&signal.symbol),
            signal.signal_type,
            signal.toxicity_score,
            signal.data_quality.unwrap_or(0.0),
            markout,
            csv_escape(&signal.primary_reason)
        ));
    }
    fs::write(path, out).with_context(|| format!("failed to write {}", path.display()))?;
    Ok(path.to_path_buf())
}

fn csv_escape(value: &str) -> String {
    let value = sanitize_csv_cell(value);
    format!("\"{}\"", value.replace('"', "\"\""))
}

pub fn sanitize_csv_cell(value: &str) -> String {
    if value.trim().is_empty() {
        return value.to_string();
    }
    if matches!(
        value.trim_start().chars().next(),
        Some('=' | '+' | '-' | '@')
    ) {
        return format!("'{value}");
    }
    value.to_string()
}

fn render_markdown(report: &ProductionReplayReport) -> String {
    format!(
        r#"# Production Replay Calibration

Candidate only. Not confirmed manipulation.

- total_events: {}
- total_signals: {}
- average_score: {:.2}
- max_score: {}
- average_data_quality: {:.2}
- high_score_candidates: {}
- possible_false_positives: {}
- possible_false_negatives: {}

## Recommendation

- should_adjust_weights: {}
- reason: {}
- warning: {}

## Recommended Actions

{}
"#,
        report.total_events,
        report.total_signals,
        report.average_score,
        report.max_score,
        report.average_data_quality,
        report.high_score_candidates.len(),
        report.possible_false_positives.len(),
        report.possible_false_negatives.len(),
        report.recommendation.should_adjust_weights,
        report.recommendation.reason,
        report.recommendation.warning.as_deref().unwrap_or("None"),
        report
            .recommendation
            .recommended_actions
            .iter()
            .map(|action| format!("- {action}"))
            .collect::<Vec<_>>()
            .join("\n")
    )
}

fn summary_without_signal_lists(report: &ProductionReplayReport) -> serde_json::Value {
    serde_json::json!({
        "candidateOnlyNotice": report.candidate_only_notice,
        "totalEvents": report.total_events,
        "totalTrades": report.total_trades,
        "totalBookDeltas": report.total_book_deltas,
        "totalSnapshots": report.total_snapshots,
        "totalSnapshotResets": report.total_snapshot_resets,
        "totalSignals": report.total_signals,
        "signalsByType": report.signals_by_type.clone(),
        "signalsBySymbol": report.signals_by_symbol.clone(),
        "signalsByVenue": report.signals_by_venue.clone(),
        "averageScore": report.average_score,
        "maxScore": report.max_score,
        "averageDataQuality": report.average_data_quality,
        "detectorAvgMarkout": report.detector_avg_markout.clone(),
        "venueDataQuality": report.venue_data_quality.clone(),
        "highScoreCandidateCount": report.high_score_candidates.len(),
        "possibleFalsePositiveCount": report.possible_false_positives.len(),
        "possibleFalseNegativeCount": report.possible_false_negatives.len(),
        "recommendation": report.recommendation.clone(),
        "readOnly": report.read_only,
    })
}

fn report_signals(report: &ProductionReplayReport) -> Vec<&ToxicSignal> {
    report.signals.iter().collect()
}

fn detector_markout(signals: &[ToxicSignal]) -> BTreeMap<String, DetectorMarkoutReport> {
    let mut grouped: BTreeMap<String, Vec<&ToxicSignal>> = BTreeMap::new();
    for signal in signals {
        grouped
            .entry(format!("{:?}", signal.signal_type))
            .or_default()
            .push(signal);
    }
    grouped
        .into_iter()
        .map(|(detector, signals)| {
            let report = DetectorMarkoutReport {
                markout_1s_bps: average_markout(&signals, |evidence| evidence.markout_1s_bps),
                markout_5s_bps: average_markout(&signals, |evidence| evidence.markout_5s_bps),
                markout_30s_bps: average_markout(&signals, |evidence| evidence.markout_30s_bps),
            };
            (detector, report)
        })
        .collect()
}

fn average_markout(
    signals: &[&ToxicSignal],
    select: impl Fn(&crate::types::toxic_signal::SignalEvidence) -> Option<f64>,
) -> Option<f64> {
    let values = signals
        .iter()
        .filter_map(|signal| signal.evidence.as_ref().and_then(&select))
        .collect::<Vec<_>>();
    if values.is_empty() {
        None
    } else {
        Some(values.iter().sum::<f64>() / values.len() as f64)
    }
}

fn signals_by_venue(signals: &[ToxicSignal]) -> BTreeMap<String, usize> {
    let mut out = BTreeMap::new();
    for signal in signals {
        let venue = signal
            .evidence
            .as_ref()
            .map(|evidence| evidence.venue.clone())
            .unwrap_or_else(|| "unknown".to_string());
        *out.entry(venue).or_insert(0) += 1;
    }
    out
}

#[derive(Debug, Clone, Copy, Default)]
struct EventCounts {
    total_events: usize,
    total_trades: usize,
    total_book_deltas: usize,
    total_snapshots: usize,
    total_snapshot_resets: usize,
}

impl EventCounts {
    fn from_events(events: &[CandidateReplayEvent]) -> Self {
        let mut counts = Self {
            total_events: events.len(),
            ..Self::default()
        };
        for event in events {
            match event.event_type {
                CandidateReplayEventType::Trade => counts.total_trades += 1,
                CandidateReplayEventType::BookDelta => counts.total_book_deltas += 1,
                CandidateReplayEventType::Snapshot => counts.total_snapshots += 1,
                CandidateReplayEventType::SnapshotReset => counts.total_snapshot_resets += 1,
            }
        }
        counts
    }
}
