use std::{
    collections::{BTreeMap, BTreeSet},
    path::PathBuf,
};

use crate::{
    calibration::{
        manual_apply_runbook::{ManualApplyRunbook, ManualApplyRunbookStore},
        manual_parameter_export::{ManualParameterExportEntry, ManualParameterExportStore},
        parameter_patch_diff::{ParameterPatchChangeType, ParameterPatchDiffStore},
    },
    config::AppConfig,
};

#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum DryRunStatus {
    Passed,
    PassedWithWarnings,
    Failed,
}

#[derive(Debug, Clone, serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DryRunIssue {
    pub code: String,
    pub field_path: Option<String>,
    pub message: String,
    pub severity: String,
}

#[derive(Debug, Clone, serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DryRunCheck {
    pub name: String,
    pub passed: bool,
    pub status: String,
    pub issue_count: usize,
}

#[derive(Debug, Clone, serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ManualApplyDryRunReport {
    pub export_id: String,
    pub apply_mode: String,
    pub runtime_modified: bool,
    pub can_apply_manually: bool,
    pub status: DryRunStatus,
    pub blocking_issues: Vec<DryRunIssue>,
    pub warnings: Vec<DryRunIssue>,
    pub checks: Vec<DryRunCheck>,
}

pub struct ManualApplyDryRunValidator {
    export_store: ManualParameterExportStore,
    diff_store: ParameterPatchDiffStore,
    runbook_store: ManualApplyRunbookStore,
}

impl ManualApplyDryRunValidator {
    pub fn new(report_dir: impl Into<PathBuf>, config: Option<AppConfig>) -> Self {
        let report_dir = report_dir.into();
        Self {
            export_store: ManualParameterExportStore::new(report_dir.clone()),
            diff_store: ParameterPatchDiffStore::new(report_dir.clone(), config.clone()),
            runbook_store: ManualApplyRunbookStore::new(report_dir, config),
        }
    }

    pub fn latest_report(&self) -> anyhow::Result<Option<ManualApplyDryRunReport>> {
        let Some(export) = self.export_store.latest_export()? else {
            return Ok(None);
        };
        Ok(Some(self.build_report(export)?))
    }

    pub fn report_by_id(&self, export_id: &str) -> anyhow::Result<Option<ManualApplyDryRunReport>> {
        let Some(export) = self.export_store.get_export(export_id)? else {
            return Ok(None);
        };
        Ok(Some(self.build_report(export)?))
    }

    pub fn latest_markdown(&self) -> anyhow::Result<Option<String>> {
        let Some(report) = self.latest_report()? else {
            return Ok(None);
        };
        Ok(Some(render_markdown(&report)))
    }

    pub fn markdown_by_id(&self, export_id: &str) -> anyhow::Result<Option<String>> {
        let Some(report) = self.report_by_id(export_id)? else {
            return Ok(None);
        };
        Ok(Some(render_markdown(&report)))
    }

    fn build_report(
        &self,
        export: ManualParameterExportEntry,
    ) -> anyhow::Result<ManualApplyDryRunReport> {
        let export_id = export.summary.export_id.clone();
        let diff = self
            .diff_store
            .diff_by_id(&export_id)?
            .ok_or_else(|| anyhow::anyhow!("current_config_unavailable"))?;
        let runbook = self
            .runbook_store
            .runbook_by_id(&export_id)?
            .ok_or_else(|| anyhow::anyhow!("current_config_unavailable"))?;

        let mut blocking_issues = Vec::new();
        let mut warnings = Vec::new();
        let mut checks = Vec::new();

        let schema_issues = validate_schema(&export, &runbook);
        push_check(
            "schema_validation",
            &schema_issues,
            &mut blocking_issues,
            &mut warnings,
            &mut checks,
        );

        let path_issues = validate_allowed_paths(&export);
        push_check(
            "allowed_field_path_validation",
            &path_issues,
            &mut blocking_issues,
            &mut warnings,
            &mut checks,
        );

        let range_issues = validate_ranges(&export);
        push_check(
            "range_validation",
            &range_issues,
            &mut blocking_issues,
            &mut warnings,
            &mut checks,
        );

        let conflict_issues = validate_conflicts(&export);
        push_check(
            "conflict_validation",
            &conflict_issues,
            &mut blocking_issues,
            &mut warnings,
            &mut checks,
        );

        let advisory_issues = validate_advisories(&diff);
        push_check(
            "advisory_validation",
            &advisory_issues,
            &mut blocking_issues,
            &mut warnings,
            &mut checks,
        );

        let rollback_issues = validate_rollback(&export, &runbook, &diff);
        push_check(
            "rollback_validation",
            &rollback_issues,
            &mut blocking_issues,
            &mut warnings,
            &mut checks,
        );

        let runtime_issues = validate_runtime_safety(&runbook);
        push_check(
            "runtime_safety_confirmation",
            &runtime_issues,
            &mut blocking_issues,
            &mut warnings,
            &mut checks,
        );

        let status = if !blocking_issues.is_empty() {
            DryRunStatus::Failed
        } else if !warnings.is_empty() {
            DryRunStatus::PassedWithWarnings
        } else {
            DryRunStatus::Passed
        };

        Ok(ManualApplyDryRunReport {
            export_id,
            apply_mode: "dry_run_only".to_string(),
            runtime_modified: false,
            can_apply_manually: blocking_issues.is_empty(),
            status,
            blocking_issues,
            warnings,
            checks,
        })
    }
}

fn push_check(
    name: &str,
    issues: &[DryRunIssue],
    blocking: &mut Vec<DryRunIssue>,
    warnings: &mut Vec<DryRunIssue>,
    checks: &mut Vec<DryRunCheck>,
) {
    let mut blocking_count = 0usize;
    let mut warning_count = 0usize;
    for issue in issues {
        if issue.severity == "blocking" {
            blocking.push(issue.clone());
            blocking_count += 1;
        } else {
            warnings.push(issue.clone());
            warning_count += 1;
        }
    }
    checks.push(DryRunCheck {
        name: name.to_string(),
        passed: issues.is_empty() || blocking_count == 0,
        status: if blocking_count > 0 {
            "failed".to_string()
        } else if warning_count > 0 {
            "warning".to_string()
        } else {
            "passed".to_string()
        },
        issue_count: issues.len(),
    });
}

fn validate_schema(
    export: &ManualParameterExportEntry,
    runbook: &ManualApplyRunbook,
) -> Vec<DryRunIssue> {
    let mut issues = Vec::new();
    if export.export.schema_version != "manual_parameter_patch.v1" {
        issues.push(blocking_issue(
            "unsupported_patch_version",
            None,
            format!(
                "unsupported schema version: {}",
                export.export.schema_version
            ),
        ));
    }
    if export.export.items.is_empty() {
        issues.push(blocking_issue(
            "empty_patch",
            None,
            "manual patch export contains no items".to_string(),
        ));
    }
    if export.export.export_id.trim().is_empty() {
        issues.push(blocking_issue(
            "missing_export_id",
            None,
            "export_id is missing".to_string(),
        ));
    }
    if runbook.rollback_plan.is_empty() {
        issues.push(blocking_issue(
            "missing_runbook_rollback",
            None,
            "runbook does not contain rollback steps".to_string(),
        ));
    }
    for item in &export.export.items {
        if item.parameter_key.trim().is_empty() {
            issues.push(blocking_issue(
                "invalid_field_change_schema",
                None,
                "parameter_key is missing".to_string(),
            ));
        }
        if item.recommended_value.is_none() {
            issues.push(blocking_issue(
                "missing_new_value",
                Some(item.parameter_key.clone()),
                "recommended_value is missing".to_string(),
            ));
        }
        if item.current_value.is_none() {
            issues.push(blocking_issue(
                "missing_old_value",
                Some(item.parameter_key.clone()),
                "current_value is missing".to_string(),
            ));
        }
    }
    issues
}

fn validate_allowed_paths(export: &ManualParameterExportEntry) -> Vec<DryRunIssue> {
    export
        .export
        .items
        .iter()
        .filter(|item| !allowed_parameter_key(&item.parameter_key))
        .map(|item| {
            blocking_issue(
                "disallowed_field_path",
                Some(item.parameter_key.clone()),
                "field path is outside the allowed calibration parameter set".to_string(),
            )
        })
        .collect()
}

fn validate_ranges(export: &ManualParameterExportEntry) -> Vec<DryRunIssue> {
    let mut issues = Vec::new();
    for item in &export.export.items {
        let Some(value) = item.recommended_value else {
            continue;
        };
        let Some((min, max)) = range_for_key(&item.parameter_key) else {
            continue;
        };
        if value < min || value > max {
            issues.push(blocking_issue(
                "value_out_of_range",
                Some(item.parameter_key.clone()),
                format!("recommended value {value} is outside [{min}, {max}]"),
            ));
        }
    }
    issues
}

fn validate_conflicts(export: &ManualParameterExportEntry) -> Vec<DryRunIssue> {
    let mut issues = Vec::new();
    let mut seen = BTreeSet::new();
    let mut values = BTreeMap::new();
    for item in &export.export.items {
        if !seen.insert(item.parameter_key.clone()) {
            issues.push(blocking_issue(
                "duplicate_field_path",
                Some(item.parameter_key.clone()),
                "field path appears multiple times in the same patch".to_string(),
            ));
        }
        if let Some(value) = item.recommended_value {
            values.insert(item.parameter_key.clone(), value);
        }
    }
    if let (Some(likely), Some(active)) = (
        values.get("liq_hunt.likely_score"),
        values.get("liq_hunt.active_score"),
    ) {
        if likely > active {
            issues.push(blocking_issue(
                "conflicting_liq_hunt_scores",
                None,
                "liq_hunt.likely_score cannot exceed liq_hunt.active_score".to_string(),
            ));
        }
    }
    issues
}

fn validate_rollback(
    export: &ManualParameterExportEntry,
    runbook: &ManualApplyRunbook,
    diff: &crate::calibration::parameter_patch_diff::ParameterPatchDiff,
) -> Vec<DryRunIssue> {
    let mut issues = Vec::new();
    if runbook.rollback_plan.is_empty() {
        issues.push(blocking_issue(
            "missing_rollback_plan",
            None,
            "runbook rollback plan is empty".to_string(),
        ));
    }
    for item in &export.export.items {
        if item.current_value.is_none() {
            issues.push(blocking_issue(
                "rollback_missing_old_value",
                Some(item.parameter_key.clone()),
                "cannot rollback safely without current_value".to_string(),
            ));
        }
    }
    for entry in &diff.entries {
        if matches!(
            entry.change_type,
            ParameterPatchChangeType::MissingInCurrentConfig
                | ParameterPatchChangeType::UnsafeOrUnknownField
        ) {
            issues.push(blocking_issue(
                "rollback_unverifiable_field",
                Some(entry.key.clone()),
                "field cannot be safely rolled back because current config mapping is unavailable"
                    .to_string(),
            ));
        }
    }
    issues
}

fn validate_advisories(
    diff: &crate::calibration::parameter_patch_diff::ParameterPatchDiff,
) -> Vec<DryRunIssue> {
    let mut issues = Vec::new();
    for entry in &diff.entries {
        if matches!(entry.change_type, ParameterPatchChangeType::Unchanged) {
            issues.push(DryRunIssue {
                code: "unchanged_field_in_patch".to_string(),
                field_path: Some(entry.key.clone()),
                message:
                    "recommended value matches current config; patch has no effect for this field"
                        .to_string(),
                severity: "warning".to_string(),
            });
        }
    }
    issues
}

fn validate_runtime_safety(runbook: &ManualApplyRunbook) -> Vec<DryRunIssue> {
    let mut issues = Vec::new();
    if runbook.runtime_modified {
        issues.push(blocking_issue(
            "runtime_modified_true",
            None,
            "runbook unexpectedly reports runtime_modified=true".to_string(),
        ));
    }
    if runbook.safety_guards.auto_apply_enabled {
        issues.push(blocking_issue(
            "auto_apply_enabled",
            None,
            "auto_apply_enabled must remain false".to_string(),
        ));
    }
    if runbook.safety_guards.runtime_reload_enabled {
        issues.push(blocking_issue(
            "runtime_reload_enabled",
            None,
            "runtime_reload_enabled must remain false".to_string(),
        ));
    }
    if runbook.safety_guards.calibration_runner_triggered {
        issues.push(blocking_issue(
            "calibration_runner_triggered",
            None,
            "calibration_runner_triggered must remain false".to_string(),
        ));
    }
    if runbook.safety_guards.trading_path_touched {
        issues.push(blocking_issue(
            "trading_path_touched",
            None,
            "trading_path_touched must remain false".to_string(),
        ));
    }
    issues
}

fn allowed_parameter_key(key: &str) -> bool {
    matches!(
        key,
        "toxicity.threshold_btc"
            | "toxicity.min_toxic_ratio"
            | "vpin.bucket_size_btc"
            | "vpin.lookback_buckets"
            | "vpin.spike_zscore"
            | "liq_hunt.likely_score"
            | "liq_hunt.active_score"
    )
}

fn range_for_key(key: &str) -> Option<(f64, f64)> {
    match key {
        "toxicity.threshold_btc" => Some((1.0, 1_000_000.0)),
        "toxicity.min_toxic_ratio" => Some((0.0, 1.0)),
        "vpin.bucket_size_btc" => Some((1.0, 100_000.0)),
        "vpin.lookback_buckets" => Some((1.0, 100_000.0)),
        "vpin.spike_zscore" => Some((0.0, 10.0)),
        "liq_hunt.likely_score" => Some((0.0, 100.0)),
        "liq_hunt.active_score" => Some((0.0, 100.0)),
        _ => None,
    }
}

fn blocking_issue(code: &str, field_path: Option<String>, message: String) -> DryRunIssue {
    DryRunIssue {
        code: code.to_string(),
        field_path,
        message,
        severity: "blocking".to_string(),
    }
}

fn render_markdown(report: &ManualApplyDryRunReport) -> String {
    let mut lines = vec![
        "# Manual Apply Dry-run Report".to_string(),
        String::new(),
        format!("Export ID: {}", report.export_id),
        format!("Apply Mode: {}", report.apply_mode),
        format!("Runtime Modified: {}", report.runtime_modified),
        format!("Can Apply Manually: {}", report.can_apply_manually),
        format!("Status: {}", status_label(report.status)),
        String::new(),
        "## Blocking Issues".to_string(),
        String::new(),
    ];
    if report.blocking_issues.is_empty() {
        lines.push("- None".to_string());
    } else {
        for issue in &report.blocking_issues {
            lines.push(format!(
                "- {}{}: {}",
                issue.code,
                issue
                    .field_path
                    .as_ref()
                    .map(|field| format!(" [{}]", field))
                    .unwrap_or_default(),
                issue.message
            ));
        }
    }

    lines.push(String::new());
    lines.push("## Warnings".to_string());
    lines.push(String::new());
    if report.warnings.is_empty() {
        lines.push("- None".to_string());
    } else {
        for issue in &report.warnings {
            lines.push(format!(
                "- {}{}: {}",
                issue.code,
                issue
                    .field_path
                    .as_ref()
                    .map(|field| format!(" [{}]", field))
                    .unwrap_or_default(),
                issue.message
            ));
        }
    }

    lines.push(String::new());
    lines.push("## Checks".to_string());
    lines.push(String::new());
    for check in &report.checks {
        lines.push(format!(
            "- {}: {} (issues={})",
            check.name, check.status, check.issue_count
        ));
    }

    lines.push(String::new());
    lines.push("## Runtime Safety Confirmation".to_string());
    lines.push(String::new());
    lines.push("- runtime_modified: false".to_string());
    lines.push("- apply_mode: dry_run_only".to_string());
    lines.push("- apply_performed: false".to_string());
    lines.push("- config_written: false".to_string());
    lines.push("- reload_required: false".to_string());

    lines.join("\n")
}

fn status_label(status: DryRunStatus) -> &'static str {
    match status {
        DryRunStatus::Passed => "passed",
        DryRunStatus::PassedWithWarnings => "passed_with_warnings",
        DryRunStatus::Failed => "failed",
    }
}
