use std::{
    fmt::Write,
    fs,
    path::{Path, PathBuf},
};

use anyhow::Context;

use crate::{config::AppConfig, replay::replay_runner::ReplayRunner};

use super::{
    calibration_types::{
        CalibrationReport, CalibrationRunSummary, CalibrationScenario, EventOutcome, OutcomeLabel,
    },
    false_positive_report::{top_false_positives, top_hits},
    outcome_labeler::OutcomeLabeler,
    parameter_grid::CalibrationGrid,
    reason_code_stats::build_reason_code_stats,
    threshold_report::build_recommendations,
};

pub struct CalibrationRunner {
    base_config: AppConfig,
    grid: CalibrationGrid,
}

impl CalibrationRunner {
    pub fn new(base_config: AppConfig) -> Self {
        let grid = CalibrationGrid::default_for_config(&base_config);
        Self { base_config, grid }
    }

    pub fn with_grid(base_config: AppConfig, grid: CalibrationGrid) -> Self {
        Self { base_config, grid }
    }

    pub fn run_file(&self, path: &str) -> anyhow::Result<CalibrationReport> {
        let labeler = OutcomeLabeler::from_replay_file(path)?;

        let baseline_run = self.run_scenario(path, &self.grid.baseline, &labeler)?;
        let threshold_comparison =
            self.run_group(path, &self.grid.threshold_comparison, &labeler)?;
        let toxic_ratio_comparison =
            self.run_group(path, &self.grid.toxic_ratio_comparison, &labeler)?;
        let vpin_parameter_comparison =
            self.run_group(path, &self.grid.vpin_parameter_comparison, &labeler)?;
        let liq_hunt_score_comparison =
            self.run_group(path, &self.grid.liq_hunt_score_comparison, &labeler)?;

        let reason_code_stats = build_reason_code_stats(&baseline_run.outcomes);
        let top_false_positives = top_false_positives(&baseline_run.outcomes, 10);
        let top_hits = top_hits(&baseline_run.outcomes, 10);
        let recommendations = build_recommendations(
            &baseline_run.summary,
            &threshold_comparison,
            &toxic_ratio_comparison,
            &vpin_parameter_comparison,
            &liq_hunt_score_comparison,
        );

        Ok(CalibrationReport {
            input_path: path.to_string(),
            generated_at: chrono::Utc::now().timestamp_millis(),
            baseline: baseline_run.summary,
            event_outcomes: baseline_run.outcomes,
            threshold_comparison,
            toxic_ratio_comparison,
            vpin_parameter_comparison,
            liq_hunt_score_comparison,
            reason_code_stats,
            top_false_positives,
            top_hits,
            recommendations,
        })
    }

    pub fn write_report(
        &self,
        report: &CalibrationReport,
        dir: impl AsRef<Path>,
    ) -> anyhow::Result<(PathBuf, PathBuf)> {
        fs::create_dir_all(dir.as_ref())
            .with_context(|| format!("failed to create report dir {}", dir.as_ref().display()))?;
        let ts = chrono::Utc::now().timestamp_millis();
        let md_path = dir.as_ref().join(format!("calibration-{ts}.md"));
        let json_path = dir.as_ref().join(format!("calibration-{ts}.json"));

        fs::write(&md_path, calibration_markdown(report))
            .with_context(|| format!("failed to write {}", md_path.display()))?;
        fs::write(&json_path, serde_json::to_string_pretty(report)?)
            .with_context(|| format!("failed to write {}", json_path.display()))?;
        Ok((md_path, json_path))
    }

    fn run_group(
        &self,
        path: &str,
        scenarios: &[CalibrationScenario],
        labeler: &OutcomeLabeler,
    ) -> anyhow::Result<Vec<CalibrationRunSummary>> {
        scenarios
            .iter()
            .map(|scenario| {
                self.run_scenario(path, scenario, labeler)
                    .map(|run| run.summary)
            })
            .collect()
    }

    fn run_scenario(
        &self,
        path: &str,
        scenario: &CalibrationScenario,
        labeler: &OutcomeLabeler,
    ) -> anyhow::Result<ScenarioRun> {
        let mut config = self.base_config.clone();
        config.toxic_volume_alert_btc = scenario.toxic_threshold_btc;
        config.vpin_bucket_size_btc = scenario.vpin_bucket_size_btc;
        config.vpin_lookback_buckets = scenario.vpin_lookback_buckets;
        config.vpin_spike_zscore = scenario.vpin_spike_zscore;
        config.liq_hunt_likely_score = scenario.liq_hunt_likely_score;
        config.liq_hunt_active_score = scenario.liq_hunt_active_score;

        let mut runner = ReplayRunner::new(config);
        let replay_report = runner.run_file(path)?;
        let filtered_events = replay_report
            .detected_events
            .into_iter()
            .filter(|event| event.toxic_volume_btc >= scenario.toxic_threshold_btc)
            .filter(|event| {
                let ratio = if event.threshold_btc > 0.0 {
                    event.toxic_volume_btc / event.threshold_btc
                } else {
                    0.0
                };
                ratio >= scenario.min_toxic_ratio
            })
            .collect::<Vec<_>>();
        let outcomes = labeler.label_events(&filtered_events);
        let summary = summarize_outcomes(scenario, &filtered_events, &outcomes);
        Ok(ScenarioRun { summary, outcomes })
    }
}

struct ScenarioRun {
    summary: CalibrationRunSummary,
    outcomes: Vec<EventOutcome>,
}

fn summarize_outcomes(
    scenario: &CalibrationScenario,
    filtered_events: &[crate::types::toxic::ToxicEvent],
    outcomes: &[EventOutcome],
) -> CalibrationRunSummary {
    let hit_count = outcomes
        .iter()
        .filter(|outcome| outcome.label == OutcomeLabel::Hit)
        .count();
    let false_positive_count = outcomes
        .iter()
        .filter(|outcome| outcome.label == OutcomeLabel::FalsePositive)
        .count();
    let neutral_count = outcomes
        .iter()
        .filter(|outcome| outcome.label == OutcomeLabel::Neutral)
        .count();
    let unknown_count = outcomes
        .iter()
        .filter(|outcome| outcome.label == OutcomeLabel::Unknown)
        .count();
    let event_count = outcomes.len();

    CalibrationRunSummary {
        group: scenario.group.to_string(),
        label: scenario.label.clone(),
        toxic_threshold_btc: scenario.toxic_threshold_btc,
        min_toxic_ratio: scenario.min_toxic_ratio,
        vpin_bucket_size_btc: scenario.vpin_bucket_size_btc,
        vpin_lookback_buckets: scenario.vpin_lookback_buckets,
        vpin_spike_zscore: scenario.vpin_spike_zscore,
        liq_hunt_likely_score: scenario.liq_hunt_likely_score,
        liq_hunt_active_score: scenario.liq_hunt_active_score,
        event_count,
        hit_count,
        false_positive_count,
        neutral_count,
        unknown_count,
        hit_rate: ratio(hit_count, event_count),
        false_positive_rate: ratio(false_positive_count, event_count),
        max_toxic_volume_btc: filtered_events
            .iter()
            .map(|event| event.toxic_volume_btc)
            .fold(0.0, f64::max),
    }
}

fn ratio(numerator: usize, denominator: usize) -> f64 {
    if denominator == 0 {
        0.0
    } else {
        numerator as f64 / denominator as f64
    }
}

fn calibration_markdown(report: &CalibrationReport) -> String {
    let mut out = String::new();
    let _ = writeln!(out, "# BTC Toxic Flow Calibration Report");
    let _ = writeln!(out);
    let _ = writeln!(out, "- Input: {}", report.input_path);
    let _ = writeln!(out, "- Generated At: {}", report.generated_at);
    let _ = writeln!(out);
    let _ = writeln!(out, "## Event Outcome Summary");
    let _ = writeln!(out, "- Events: {}", report.baseline.event_count);
    let _ = writeln!(out, "- Hits: {}", report.baseline.hit_count);
    let _ = writeln!(
        out,
        "- False Positives: {}",
        report.baseline.false_positive_count
    );
    let _ = writeln!(out, "- Unknown: {}", report.baseline.unknown_count);
    let _ = writeln!(out, "- Hit Rate: {:.2}", report.baseline.hit_rate);
    let _ = writeln!(
        out,
        "- False Positive Rate: {:.2}",
        report.baseline.false_positive_rate
    );
    let _ = writeln!(out);
    write_comparison_section(
        &mut out,
        "Threshold Comparison",
        &report.threshold_comparison,
    );
    write_comparison_section(
        &mut out,
        "Toxic Ratio Comparison",
        &report.toxic_ratio_comparison,
    );
    write_comparison_section(
        &mut out,
        "VPIN Parameter Comparison",
        &report.vpin_parameter_comparison,
    );
    write_comparison_section(
        &mut out,
        "Liq Hunt Score Comparison",
        &report.liq_hunt_score_comparison,
    );

    let _ = writeln!(out, "## Reason Code Stats");
    for stat in &report.reason_code_stats {
        let _ = writeln!(
            out,
            "- {}: total={} hit_rate={:.2} false_positive_rate={:.2}",
            stat.reason_code, stat.total_count, stat.hit_rate, stat.false_positive_rate
        );
    }
    let _ = writeln!(out);
    write_outcome_section(&mut out, "Top False Positives", &report.top_false_positives);
    write_outcome_section(&mut out, "Top Hits", &report.top_hits);
    let _ = writeln!(out, "## Recommendations");
    for recommendation in &report.recommendations {
        let _ = writeln!(out, "- {}: {}", recommendation.title, recommendation.detail);
    }
    out
}

fn write_comparison_section(out: &mut String, title: &str, rows: &[CalibrationRunSummary]) {
    let _ = writeln!(out, "## {title}");
    let _ = writeln!(
        out,
        "| Label | Events | Hit Rate | False Positive Rate | Threshold | Min Ratio | VPIN Bucket | VPIN Lookback | VPIN Z | Likely | Active |"
    );
    let _ = writeln!(
        out,
        "|---|---:|---:|---:|---:|---:|---:|---:|---:|---:|---:|"
    );
    for row in rows {
        let _ = writeln!(
            out,
            "| {} | {} | {:.2} | {:.2} | {:.0} | {:.2} | {:.0} | {} | {:.1} | {:.0} | {:.0} |",
            row.label,
            row.event_count,
            row.hit_rate,
            row.false_positive_rate,
            row.toxic_threshold_btc,
            row.min_toxic_ratio,
            row.vpin_bucket_size_btc,
            row.vpin_lookback_buckets,
            row.vpin_spike_zscore,
            row.liq_hunt_likely_score,
            row.liq_hunt_active_score,
        );
    }
    let _ = writeln!(out);
}

fn write_outcome_section(out: &mut String, title: &str, outcomes: &[EventOutcome]) {
    let _ = writeln!(out, "## {title}");
    if outcomes.is_empty() {
        let _ = writeln!(out, "- None");
        let _ = writeln!(out);
        return;
    }
    for outcome in outcomes {
        let _ = writeln!(
            out,
            "- ts={} direction={:?} severity={:?} toxic_volume_btc={:.1} primary_move_bps={:?} reasons={}",
            outcome.event.ts,
            outcome.event.direction,
            outcome.event.severity,
            outcome.event.toxic_volume_btc,
            outcome.primary_move_bps,
            outcome.event.reason_codes.join(", ")
        );
    }
    let _ = writeln!(out);
}
