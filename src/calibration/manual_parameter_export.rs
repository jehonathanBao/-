use std::{
    fs,
    path::{Path, PathBuf},
    time::UNIX_EPOCH,
};

use anyhow::Context;
use chrono::{DateTime, Local};

use super::parameter_recommendation_review_store::{
    ParameterRecommendationReviewStore, RecommendationCard, ReviewStatus,
};

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ManualParameterExportSource {
    pub recommendation_store: String,
    pub review_ledger: String,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ManualParameterExportSafety {
    pub manual_only: bool,
    pub runtime_modified: bool,
    pub auto_apply_supported: bool,
    pub requires_human_review: bool,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ManualPatchHint {
    pub file_hint: String,
    pub old_line_hint: String,
    pub new_line_hint: String,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ManualParameterPatchItem {
    pub recommendation_id: String,
    pub parameter_key: String,
    pub current_value: Option<f64>,
    pub recommended_value: Option<f64>,
    pub direction: String,
    pub confidence: String,
    pub expected_effect: Option<String>,
    pub risk_note: Option<String>,
    pub reason: String,
    pub review: super::parameter_recommendation_review_store::ReviewDecision,
    pub manual_patch_hint: ManualPatchHint,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ManualParameterExport {
    pub schema_version: String,
    pub export_id: String,
    pub created_at: String,
    pub source: ManualParameterExportSource,
    pub safety: ManualParameterExportSafety,
    pub items: Vec<ManualParameterPatchItem>,
}

#[derive(Debug, Clone, serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ManualParameterExportSummary {
    pub export_id: String,
    pub json_path: String,
    pub markdown_path: Option<String>,
    pub created_at_ms: Option<i64>,
    pub recommendation_count: usize,
    pub apply_mode: String,
    pub runtime_modified: bool,
}

#[derive(Debug, Clone, serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ManualParameterExportEntry {
    pub summary: ManualParameterExportSummary,
    pub export: ManualParameterExport,
    pub markdown_content: Option<String>,
}

#[derive(Debug, Clone, serde::Deserialize)]
pub struct ManualParameterExportRequest {
    pub include_statuses: Option<Vec<ReviewStatus>>,
    pub operator: Option<String>,
    pub note: Option<String>,
}

#[derive(Debug, Clone, serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ManualParameterExportResponse {
    pub ok: bool,
    pub export_created: bool,
    pub reason: Option<String>,
    pub export_id: Option<String>,
    pub json_path: Option<String>,
    pub markdown_path: Option<String>,
    pub recommendation_count: usize,
    pub apply_mode: String,
    pub runtime_modified: bool,
}

pub struct ManualParameterExportStore {
    review_store: ParameterRecommendationReviewStore,
    export_dir: PathBuf,
}

impl ManualParameterExportStore {
    pub fn new(report_dir: impl Into<PathBuf>) -> Self {
        let report_dir = report_dir.into();
        let runtime_dir = report_dir
            .parent()
            .map(Path::to_path_buf)
            .unwrap_or_else(|| PathBuf::from(".runtime"));
        let export_dir = runtime_dir.join("exports");
        Self {
            review_store: ParameterRecommendationReviewStore::new(report_dir),
            export_dir,
        }
    }

    pub fn create_export(
        &self,
        request: ManualParameterExportRequest,
    ) -> anyhow::Result<ManualParameterExportResponse> {
        let include_statuses = request
            .include_statuses
            .unwrap_or_else(|| vec![ReviewStatus::ApprovedForManualApply]);
        let allowed_statuses: std::collections::BTreeSet<ReviewStatus> =
            include_statuses.into_iter().collect();
        let approved = self.collect_cards(&allowed_statuses)?;
        if approved.is_empty() {
            return Ok(ManualParameterExportResponse {
                ok: true,
                export_created: false,
                reason: Some("no_approved_recommendations".to_string()),
                export_id: None,
                json_path: None,
                markdown_path: None,
                recommendation_count: 0,
                apply_mode: "manual_only".to_string(),
                runtime_modified: false,
            });
        }

        fs::create_dir_all(&self.export_dir)
            .with_context(|| format!("create {}", self.export_dir.display()))?;
        let now = Local::now();
        let export_id = format!("manual-parameter-patch-{}", now.format("%Y%m%d-%H%M%S"));
        let export = build_export(
            &export_id,
            &approved,
            self.review_store.ledger_path(),
            request.operator,
            request.note,
        );
        let json_path = self.export_dir.join(format!("{export_id}.json"));
        let markdown_path = self.export_dir.join(format!("{export_id}.md"));
        fs::write(&json_path, serde_json::to_string_pretty(&export)?)
            .with_context(|| format!("write {}", json_path.display()))?;
        fs::write(&markdown_path, export_markdown(&export))
            .with_context(|| format!("write {}", markdown_path.display()))?;

        Ok(ManualParameterExportResponse {
            ok: true,
            export_created: true,
            reason: None,
            export_id: Some(export_id),
            json_path: Some(json_path.display().to_string()),
            markdown_path: Some(markdown_path.display().to_string()),
            recommendation_count: export.items.len(),
            apply_mode: "manual_only".to_string(),
            runtime_modified: false,
        })
    }

    pub fn list_exports(&self) -> anyhow::Result<Vec<ManualParameterExportSummary>> {
        let mut entries = self.load_entries()?;
        entries.sort_by(|left, right| {
            right
                .summary
                .created_at_ms
                .unwrap_or_default()
                .cmp(&left.summary.created_at_ms.unwrap_or_default())
        });
        Ok(entries.into_iter().map(|entry| entry.summary).collect())
    }

    pub fn latest_export(&self) -> anyhow::Result<Option<ManualParameterExportEntry>> {
        let mut entries = self.load_entries()?;
        entries.sort_by(|left, right| {
            right
                .summary
                .created_at_ms
                .unwrap_or_default()
                .cmp(&left.summary.created_at_ms.unwrap_or_default())
        });
        Ok(entries.into_iter().next())
    }

    pub fn get_export(
        &self,
        export_id: &str,
    ) -> anyhow::Result<Option<ManualParameterExportEntry>> {
        let path = self.export_dir.join(format!("{export_id}.json"));
        if !path.exists() {
            return Ok(None);
        }
        self.load_entry(&path).map(Some)
    }

    fn collect_cards(
        &self,
        statuses: &std::collections::BTreeSet<ReviewStatus>,
    ) -> anyhow::Result<Vec<RecommendationCard>> {
        let mut cards: Vec<RecommendationCard> = self
            .review_store
            .list_recommendations()?
            .into_iter()
            .filter(|card| {
                card.current_review
                    .as_ref()
                    .map(|review| statuses.contains(&review.status))
                    .unwrap_or(false)
            })
            .collect();
        cards.sort_by(|left, right| left.recommendation_id.cmp(&right.recommendation_id));
        Ok(cards)
    }

    fn load_entries(&self) -> anyhow::Result<Vec<ManualParameterExportEntry>> {
        if !self.export_dir.exists() {
            return Ok(Vec::new());
        }
        let mut entries = Vec::new();
        for entry in fs::read_dir(&self.export_dir)
            .with_context(|| format!("read {}", self.export_dir.display()))?
        {
            let entry = entry?;
            if !entry.file_type()?.is_file() {
                continue;
            }
            let path = entry.path();
            let Some(file_name) = path.file_name().and_then(|name| name.to_str()) else {
                continue;
            };
            if !file_name.starts_with("manual-parameter-patch-") || !file_name.ends_with(".json") {
                continue;
            }
            entries.push(self.load_entry(&path)?);
        }
        Ok(entries)
    }

    fn load_entry(&self, json_path: &Path) -> anyhow::Result<ManualParameterExportEntry> {
        let json_text = fs::read_to_string(json_path)
            .with_context(|| format!("read {}", json_path.display()))?;
        let export: ManualParameterExport = serde_json::from_str(&json_text)
            .with_context(|| format!("parse {}", json_path.display()))?;
        let export_id = json_path
            .file_stem()
            .and_then(|stem| stem.to_str())
            .unwrap_or_default()
            .to_string();
        let markdown_path = self.export_dir.join(format!("{export_id}.md"));
        let markdown_content = fs::read_to_string(&markdown_path).ok();
        Ok(ManualParameterExportEntry {
            summary: ManualParameterExportSummary {
                export_id,
                json_path: json_path.display().to_string(),
                markdown_path: markdown_content
                    .as_ref()
                    .map(|_| markdown_path.display().to_string()),
                created_at_ms: export_timestamp(json_path, &export),
                recommendation_count: export.items.len(),
                apply_mode: "manual_only".to_string(),
                runtime_modified: false,
            },
            export,
            markdown_content,
        })
    }
}

fn build_export(
    export_id: &str,
    cards: &[RecommendationCard],
    ledger_path: &Path,
    operator: Option<String>,
    note: Option<String>,
) -> ManualParameterExport {
    let created_at = Local::now();
    ManualParameterExport {
        schema_version: "manual_parameter_patch.v1".to_string(),
        export_id: export_id.to_string(),
        created_at: created_at.to_rfc3339(),
        source: ManualParameterExportSource {
            recommendation_store: "calibration_reports".to_string(),
            review_ledger: ledger_path.display().to_string(),
        },
        safety: ManualParameterExportSafety {
            manual_only: true,
            runtime_modified: false,
            auto_apply_supported: false,
            requires_human_review: true,
        },
        items: cards
            .iter()
            .filter_map(|card| {
                let review = card.current_review.clone()?;
                Some(ManualParameterPatchItem {
                    recommendation_id: card.recommendation_id.clone(),
                    parameter_key: card.parameter_key.clone(),
                    current_value: card.current_value,
                    recommended_value: card.recommended_value,
                    direction: card.direction.clone(),
                    confidence: card.confidence.clone(),
                    expected_effect: card.expected_effect.clone(),
                    risk_note: merge_notes(card.risk_note.clone(), operator.clone(), note.clone()),
                    reason: card.reason.clone(),
                    review,
                    manual_patch_hint: ManualPatchHint {
                        file_hint: "config or runtime parameter file".to_string(),
                        old_line_hint: format_line_hint(
                            &card.parameter_key,
                            card.current_config_summary
                                .as_deref()
                                .unwrap_or("Unavailable"),
                        ),
                        new_line_hint: format_line_hint(
                            &card.parameter_key,
                            card.recommended_config_summary
                                .as_deref()
                                .unwrap_or("Unavailable"),
                        ),
                    },
                })
            })
            .collect(),
    }
}

fn merge_notes(
    risk_note: Option<String>,
    operator: Option<String>,
    note: Option<String>,
) -> Option<String> {
    let mut parts = Vec::new();
    if let Some(risk_note) = risk_note {
        parts.push(risk_note);
    }
    if let Some(operator) = operator.filter(|value| !value.trim().is_empty()) {
        parts.push(format!("export operator: {operator}"));
    }
    if let Some(note) = note.filter(|value| !value.trim().is_empty()) {
        parts.push(format!("export note: {note}"));
    }
    if parts.is_empty() {
        None
    } else {
        Some(parts.join(" | "))
    }
}

fn format_line_hint(parameter_key: &str, value: &str) -> String {
    format!("{parameter_key} = {value}")
}

fn export_timestamp(path: &Path, export: &ManualParameterExport) -> Option<i64> {
    DateTime::parse_from_rfc3339(&export.created_at)
        .ok()
        .map(|timestamp| timestamp.timestamp_millis())
        .or_else(|| {
            fs::metadata(path)
                .ok()
                .and_then(|metadata| metadata.modified().ok())
                .and_then(|modified| modified.duration_since(UNIX_EPOCH).ok())
                .map(|duration| duration.as_millis() as i64)
        })
}

fn export_markdown(export: &ManualParameterExport) -> String {
    let mut lines = vec![
        "# Manual Parameter Patch Export".to_string(),
        String::new(),
        format!("Export ID: {}", export.export_id),
        "Mode: Manual Only".to_string(),
        "Runtime Modified: false".to_string(),
        "Auto Apply Supported: false".to_string(),
        String::new(),
        "## Safety Boundary".to_string(),
        String::new(),
        "This file is for human review only.".to_string(),
        "No runtime config was modified.".to_string(),
        "No live parameter reload was triggered.".to_string(),
        "No calibration runner was executed.".to_string(),
        String::new(),
        "## Approved Recommendations".to_string(),
        String::new(),
    ];

    for (index, item) in export.items.iter().enumerate() {
        lines.push(format!("### {}. {}", index + 1, item.parameter_key));
        lines.push(String::new());
        lines.push(format!(
            "Current value: `{}`  ",
            item.current_value
                .map(format_scalar)
                .unwrap_or_else(|| "Unavailable".to_string())
        ));
        lines.push(format!(
            "Recommended value: `{}`  ",
            item.recommended_value
                .map(format_scalar)
                .unwrap_or_else(|| "Unavailable".to_string())
        ));
        lines.push(format!("Direction: {}  ", item.direction));
        lines.push(format!("Confidence: {}  ", item.confidence));
        lines.push(String::new());
        lines.push("Expected effect:".to_string());
        lines.push(
            item.expected_effect
                .clone()
                .unwrap_or_else(|| "Unavailable.".to_string()),
        );
        lines.push(String::new());
        lines.push("Risk note:".to_string());
        lines.push(
            item.risk_note
                .clone()
                .unwrap_or_else(|| "Unavailable.".to_string()),
        );
        lines.push(String::new());
        lines.push("Manual patch hint:".to_string());
        lines.push(String::new());
        lines.push("```diff".to_string());
        lines.push(format!("- {}", item.manual_patch_hint.old_line_hint));
        lines.push(format!("+ {}", item.manual_patch_hint.new_line_hint));
        lines.push("```".to_string());
        lines.push(String::new());
    }

    lines.join("\n")
}

fn format_scalar(value: f64) -> String {
    if (value.fract()).abs() < f64::EPSILON {
        format!("{value:.0}")
    } else {
        format!("{value:.2}")
    }
}
