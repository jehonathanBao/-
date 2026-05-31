use std::{
    fs,
    path::{Path, PathBuf},
    time::UNIX_EPOCH,
};

use anyhow::Context;

use super::calibration_types::{CalibrationReport, CalibrationRunSummary};

#[derive(Debug, Clone, serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CalibrationReportSummary {
    pub id: String,
    pub json_path: String,
    pub markdown_path: Option<String>,
    pub created_at_ms: Option<i64>,
    pub event_count: usize,
    pub hit_count: usize,
    pub false_positive_count: usize,
    pub unknown_count: usize,
    pub best_threshold: Option<f64>,
    pub best_liq_hunt_score: Option<f64>,
}

#[derive(Debug, Clone, serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CalibrationReportEntry {
    pub summary: CalibrationReportSummary,
    pub report: CalibrationReport,
    pub markdown_content: Option<String>,
}

pub struct CalibrationReportStore {
    dir: PathBuf,
}

impl CalibrationReportStore {
    pub fn new(dir: impl Into<PathBuf>) -> Self {
        Self { dir: dir.into() }
    }

    pub fn list_reports(&self) -> anyhow::Result<Vec<CalibrationReportSummary>> {
        let mut reports = self.load_entries()?;
        reports.sort_by(|left, right| {
            right
                .summary
                .created_at_ms
                .unwrap_or_default()
                .cmp(&left.summary.created_at_ms.unwrap_or_default())
        });
        Ok(reports.into_iter().map(|entry| entry.summary).collect())
    }

    pub fn latest_report(&self) -> anyhow::Result<Option<CalibrationReportEntry>> {
        let mut reports = self.load_entries()?;
        reports.sort_by(|left, right| {
            right
                .summary
                .created_at_ms
                .unwrap_or_default()
                .cmp(&left.summary.created_at_ms.unwrap_or_default())
        });
        Ok(reports.into_iter().next())
    }

    pub fn get_report(&self, report_id: &str) -> anyhow::Result<Option<CalibrationReportEntry>> {
        let path = self.dir.join(format!("{report_id}.json"));
        if !path.exists() {
            return Ok(None);
        }
        self.load_entry(&path).map(Some)
    }

    fn load_entries(&self) -> anyhow::Result<Vec<CalibrationReportEntry>> {
        if !self.dir.exists() {
            return Ok(Vec::new());
        }

        let mut entries = Vec::new();
        for entry in
            fs::read_dir(&self.dir).with_context(|| format!("read {}", self.dir.display()))?
        {
            let entry = entry?;
            if !entry.file_type()?.is_file() {
                continue;
            }
            let path = entry.path();
            let Some(file_name) = path.file_name().and_then(|name| name.to_str()) else {
                continue;
            };
            if !file_name.starts_with("calibration-") || !file_name.ends_with(".json") {
                continue;
            }
            entries.push(self.load_entry(&path)?);
        }

        Ok(entries)
    }

    fn load_entry(&self, json_path: &Path) -> anyhow::Result<CalibrationReportEntry> {
        let json_text = fs::read_to_string(json_path)
            .with_context(|| format!("read {}", json_path.display()))?;
        let report: CalibrationReport = serde_json::from_str(&json_text)
            .with_context(|| format!("parse calibration report {}", json_path.display()))?;
        let id = json_path
            .file_stem()
            .and_then(|stem| stem.to_str())
            .unwrap_or_default()
            .to_string();
        let markdown_path = self.dir.join(format!("{id}.md"));
        let markdown_content = fs::read_to_string(&markdown_path).ok();

        Ok(CalibrationReportEntry {
            summary: CalibrationReportSummary {
                id,
                json_path: json_path.display().to_string(),
                markdown_path: markdown_content
                    .as_ref()
                    .map(|_| markdown_path.display().to_string()),
                created_at_ms: report_timestamp(json_path, &report),
                event_count: report.baseline.event_count,
                hit_count: report.baseline.hit_count,
                false_positive_count: report.baseline.false_positive_count,
                unknown_count: report.baseline.unknown_count,
                best_threshold: best_threshold(&report),
                best_liq_hunt_score: best_liq_hunt_score(&report),
            },
            report,
            markdown_content,
        })
    }
}

fn report_timestamp(path: &Path, report: &CalibrationReport) -> Option<i64> {
    if report.generated_at > 0 {
        return Some(report.generated_at);
    }

    fs::metadata(path)
        .ok()
        .and_then(|metadata| metadata.modified().ok())
        .and_then(|modified| modified.duration_since(UNIX_EPOCH).ok())
        .map(|duration| duration.as_millis() as i64)
}

fn best_threshold(report: &CalibrationReport) -> Option<f64> {
    report
        .threshold_comparison
        .iter()
        .max_by(|left, right| score_summary(left).total_cmp(&score_summary(right)))
        .map(|summary| summary.toxic_threshold_btc)
}

fn best_liq_hunt_score(report: &CalibrationReport) -> Option<f64> {
    report
        .liq_hunt_score_comparison
        .iter()
        .max_by(|left, right| score_summary(left).total_cmp(&score_summary(right)))
        .map(|summary| summary.liq_hunt_active_score)
}

fn score_summary(summary: &CalibrationRunSummary) -> f64 {
    summary.hit_rate - summary.false_positive_rate
}
