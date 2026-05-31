use std::path::PathBuf;

use crate::{calibration::parameter_patch_diff::ParameterPatchDiffStore, config::AppConfig};

#[derive(Debug, Clone, serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ManualApplyRunbookSource {
    pub export_path: String,
    pub diff_available: bool,
    pub audit_available: bool,
}

#[derive(Debug, Clone, serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ManualApplyRunbookSummary {
    pub total_patch_fields: usize,
    pub changed_fields: usize,
    pub unchanged_fields: usize,
    pub missing_in_current_config: usize,
    pub risk_level: String,
}

#[derive(Debug, Clone, serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ManualChecklistItem {
    pub id: String,
    pub title: String,
    pub required: bool,
    pub status: String,
    pub note: Option<String>,
}

#[derive(Debug, Clone, serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ManualStep {
    pub step: usize,
    pub title: String,
    pub instruction: String,
}

#[derive(Debug, Clone, serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct RunbookFieldChange {
    pub path: String,
    pub current_value: String,
    pub recommended_value: String,
    pub status: String,
    pub action: String,
    pub notes: Vec<String>,
}

#[derive(Debug, Clone, serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PostApplyVerificationStep {
    pub id: String,
    pub title: String,
    pub command: Option<String>,
    pub instruction: Option<String>,
    pub required: bool,
}

#[derive(Debug, Clone, serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct RollbackStep {
    pub step: usize,
    pub title: String,
    pub instruction: String,
}

#[derive(Debug, Clone, serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct RunbookSafetyGuards {
    pub auto_apply_enabled: bool,
    pub runtime_reload_enabled: bool,
    pub calibration_runner_triggered: bool,
    pub trading_path_touched: bool,
}

#[derive(Debug, Clone, serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ManualApplyRunbook {
    pub ok: bool,
    pub export_id: String,
    pub apply_mode: String,
    pub runtime_modified: bool,
    pub generated_at: Option<String>,
    pub source: ManualApplyRunbookSource,
    pub summary: ManualApplyRunbookSummary,
    pub pre_apply_checklist: Vec<ManualChecklistItem>,
    pub manual_steps: Vec<ManualStep>,
    pub field_changes: Vec<RunbookFieldChange>,
    pub post_apply_verification: Vec<PostApplyVerificationStep>,
    pub rollback_plan: Vec<RollbackStep>,
    pub safety_guards: RunbookSafetyGuards,
}

pub struct ManualApplyRunbookStore {
    diff_store: ParameterPatchDiffStore,
}

impl ManualApplyRunbookStore {
    pub fn new(report_dir: impl Into<PathBuf>, config: Option<AppConfig>) -> Self {
        Self {
            diff_store: ParameterPatchDiffStore::new(report_dir, config),
        }
    }

    pub fn latest_runbook(&self) -> anyhow::Result<Option<ManualApplyRunbook>> {
        let Some(diff) = self.diff_store.latest_diff()? else {
            return Ok(None);
        };
        let Some(audit) = self.diff_store.latest_audit()? else {
            return Ok(None);
        };
        Ok(Some(build_runbook(diff, audit)))
    }

    pub fn runbook_by_id(&self, export_id: &str) -> anyhow::Result<Option<ManualApplyRunbook>> {
        let Some(diff) = self.diff_store.diff_by_id(export_id)? else {
            return Ok(None);
        };
        let Some(audit) = self.diff_store.audit_by_id(export_id)? else {
            return Ok(None);
        };
        Ok(Some(build_runbook(diff, audit)))
    }

    pub fn latest_markdown(&self) -> anyhow::Result<Option<String>> {
        let Some(runbook) = self.latest_runbook()? else {
            return Ok(None);
        };
        Ok(Some(render_markdown(&runbook)))
    }

    pub fn markdown_by_id(&self, export_id: &str) -> anyhow::Result<Option<String>> {
        let Some(runbook) = self.runbook_by_id(export_id)? else {
            return Ok(None);
        };
        Ok(Some(render_markdown(&runbook)))
    }
}

fn build_runbook(
    diff: crate::calibration::parameter_patch_diff::ParameterPatchDiff,
    _audit: crate::calibration::parameter_patch_diff::ParameterPatchAudit,
) -> ManualApplyRunbook {
    let missing_count = diff
        .entries
        .iter()
        .filter(|entry| entry.change_type == crate::calibration::parameter_patch_diff::ParameterPatchChangeType::MissingInCurrentConfig)
        .count();
    let risk_level = if missing_count > 0 || !diff.warnings.is_empty() {
        "review_required"
    } else if diff.summary.changed_fields_count > 0 {
        "manual_change_ready"
    } else {
        "no_effect_change"
    };

    let mut checklist = vec![
        ManualChecklistItem {
            id: "backup-current-config".to_string(),
            title: "Backup current runtime config".to_string(),
            required: true,
            status: "manual_required".to_string(),
            note: Some(
                "Create a manual backup before editing any config file or environment entry."
                    .to_string(),
            ),
        },
        ManualChecklistItem {
            id: "confirm-read-only-boundary".to_string(),
            title: "Confirm this runbook is review-only".to_string(),
            required: true,
            status: "manual_required".to_string(),
            note: Some(
                "This API does not apply parameters and must not be treated as a runtime change path."
                    .to_string(),
            ),
        },
    ];
    if missing_count > 0 {
        checklist.push(ManualChecklistItem {
            id: "resolve-missing-fields".to_string(),
            title: "Resolve fields missing in current config".to_string(),
            required: true,
            status: "manual_required".to_string(),
            note: Some(format!(
                "{missing_count} field(s) were not found in the current config snapshot. Review manually before any edit."
            )),
        });
    }

    let manual_steps = vec![
        ManualStep {
            step: 1,
            title: "Open the current parameter config manually".to_string(),
            instruction:
                "Open the runtime config or environment source by hand. Do not edit through this API."
                    .to_string(),
        },
        ManualStep {
            step: 2,
            title: "Compare each changed field with the diff table".to_string(),
            instruction:
                "Only copy fields marked as changed after verifying the recommendation source and review status."
                    .to_string(),
        },
        ManualStep {
            step: 3,
            title: "Skip unknown or missing fields unless separately reviewed".to_string(),
            instruction:
                "Fields marked missing_in_current_config or unsafe_or_unknown_field require an additional manual decision."
                    .to_string(),
        },
    ];

    let field_changes = diff
        .entries
        .iter()
        .map(|entry| RunbookFieldChange {
            path: entry.key.clone(),
            current_value: entry.current_display.clone(),
            recommended_value: entry.recommended_display.clone(),
            status: serde_json::to_string(&entry.change_type)
                .unwrap_or_else(|_| "\"unknown\"".to_string())
                .trim_matches('"')
                .to_string(),
            action: match entry.change_type {
                crate::calibration::parameter_patch_diff::ParameterPatchChangeType::Changed => {
                    "manual_review_then_copy".to_string()
                }
                crate::calibration::parameter_patch_diff::ParameterPatchChangeType::Unchanged => {
                    "no_change_needed".to_string()
                }
                _ => "manual_review_required".to_string(),
            },
            notes: entry.notes.clone(),
        })
        .collect();

    let post_apply_verification = vec![
        PostApplyVerificationStep {
            id: "run-cargo-check".to_string(),
            title: "Run cargo check".to_string(),
            command: Some("cargo check".to_string()),
            instruction: None,
            required: true,
        },
        PostApplyVerificationStep {
            id: "run-cargo-test".to_string(),
            title: "Run cargo test".to_string(),
            command: Some("cargo test".to_string()),
            instruction: None,
            required: true,
        },
        PostApplyVerificationStep {
            id: "verify-dashboard-boundary".to_string(),
            title: "Verify boundary still reads manual_only".to_string(),
            command: None,
            instruction: Some(
                "Confirm the diff/audit/runbook APIs still report apply_mode=manual_only and runtime_modified=false."
                    .to_string(),
            ),
            required: true,
        },
    ];

    let rollback_plan = vec![
        RollbackStep {
            step: 1,
            title: "Restore backup config".to_string(),
            instruction:
                "Replace any manual config edits with the backup captured before this change."
                    .to_string(),
        },
        RollbackStep {
            step: 2,
            title: "Re-run validation commands".to_string(),
            instruction:
                "Run cargo check and cargo test again after restoring the backup."
                    .to_string(),
        },
        RollbackStep {
            step: 3,
            title: "Confirm dashboard remains read-only".to_string(),
            instruction:
                "Check that no runtime_modified flag or apply endpoint appeared during the rollback."
                    .to_string(),
        },
    ];

    ManualApplyRunbook {
        ok: true,
        export_id: diff.export_id.clone(),
        apply_mode: "manual_only".to_string(),
        runtime_modified: false,
        generated_at: diff.generated_at.clone(),
        source: ManualApplyRunbookSource {
            export_path: diff.export_path.clone(),
            diff_available: true,
            audit_available: true,
        },
        summary: ManualApplyRunbookSummary {
            total_patch_fields: diff.entries.len(),
            changed_fields: diff.summary.changed_fields_count,
            unchanged_fields: diff.summary.unchanged_fields_count,
            missing_in_current_config: missing_count,
            risk_level: risk_level.to_string(),
        },
        pre_apply_checklist: checklist,
        manual_steps,
        field_changes,
        post_apply_verification,
        rollback_plan,
        safety_guards: RunbookSafetyGuards {
            auto_apply_enabled: false,
            runtime_reload_enabled: false,
            calibration_runner_triggered: false,
            trading_path_touched: false,
        },
    }
}

fn render_markdown(runbook: &ManualApplyRunbook) -> String {
    let mut lines = vec![
        "# Manual Apply Runbook".to_string(),
        String::new(),
        format!("Export ID: {}", runbook.export_id),
        format!("Apply Mode: {}", runbook.apply_mode),
        format!("Runtime Modified: {}", runbook.runtime_modified),
        String::new(),
        "## Safety Boundary".to_string(),
        String::new(),
        "- This runbook is for human review only.".to_string(),
        "- No runtime config was modified.".to_string(),
        "- No calibration runner was triggered.".to_string(),
        "- No runtime reload was triggered.".to_string(),
        String::new(),
        "## Summary".to_string(),
        String::new(),
        format!(
            "- Total patch fields: {}",
            runbook.summary.total_patch_fields
        ),
        format!("- Changed fields: {}", runbook.summary.changed_fields),
        format!("- Unchanged fields: {}", runbook.summary.unchanged_fields),
        format!(
            "- Missing in current config: {}",
            runbook.summary.missing_in_current_config
        ),
        format!("- Risk level: {}", runbook.summary.risk_level),
        String::new(),
        "## Pre-apply Checklist".to_string(),
        String::new(),
    ];

    for item in &runbook.pre_apply_checklist {
        lines.push(format!(
            "- [{}] {} ({})",
            if item.required {
                "required"
            } else {
                "optional"
            },
            item.title,
            item.status
        ));
        if let Some(note) = &item.note {
            lines.push(format!("  - {}", note));
        }
    }

    lines.push(String::new());
    lines.push("## Manual Steps".to_string());
    lines.push(String::new());
    for step in &runbook.manual_steps {
        lines.push(format!("{}. {}", step.step, step.title));
        lines.push(format!("   {}", step.instruction));
    }

    lines.push(String::new());
    lines.push("## Field Changes".to_string());
    lines.push(String::new());
    for change in &runbook.field_changes {
        lines.push(format!("### {}", change.path));
        lines.push(String::new());
        lines.push(format!("- Current: `{}`", change.current_value));
        lines.push(format!("- Recommended: `{}`", change.recommended_value));
        lines.push(format!("- Status: `{}`", change.status));
        lines.push(format!("- Action: `{}`", change.action));
        if !change.notes.is_empty() {
            lines.push("- Notes:".to_string());
            for note in &change.notes {
                lines.push(format!("  - {}", note));
            }
        }
        lines.push(String::new());
    }

    lines.push("## Post-apply Verification".to_string());
    lines.push(String::new());
    for step in &runbook.post_apply_verification {
        lines.push(format!("- {}", step.title));
        if let Some(command) = &step.command {
            lines.push(format!("  - Command: `{}`", command));
        }
        if let Some(instruction) = &step.instruction {
            lines.push(format!("  - {}", instruction));
        }
    }

    lines.push(String::new());
    lines.push("## Rollback Plan".to_string());
    lines.push(String::new());
    for step in &runbook.rollback_plan {
        lines.push(format!("{}. {}", step.step, step.title));
        lines.push(format!("   {}", step.instruction));
    }

    lines.push(String::new());
    lines.push("## Safety Guards".to_string());
    lines.push(String::new());
    lines.push(format!(
        "- auto_apply_enabled: {}",
        runbook.safety_guards.auto_apply_enabled
    ));
    lines.push(format!(
        "- runtime_reload_enabled: {}",
        runbook.safety_guards.runtime_reload_enabled
    ));
    lines.push(format!(
        "- calibration_runner_triggered: {}",
        runbook.safety_guards.calibration_runner_triggered
    ));
    lines.push(format!(
        "- trading_path_touched: {}",
        runbook.safety_guards.trading_path_touched
    ));

    lines.join("\n")
}
