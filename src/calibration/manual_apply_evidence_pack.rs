use std::path::PathBuf;

use crate::{
    calibration::{
        manual_apply_dryrun_validator::{
            DryRunStatus, ManualApplyDryRunReport, ManualApplyDryRunValidator,
        },
        manual_apply_runbook::{ManualApplyRunbook, ManualApplyRunbookStore},
        manual_parameter_export::{ManualParameterExportEntry, ManualParameterExportStore},
        parameter_patch_diff::{ParameterPatchAudit, ParameterPatchDiff, ParameterPatchDiffStore},
        parameter_recommendation_review_store::{
            ParameterRecommendationReviewStore, RecommendationCard,
        },
    },
    config::AppConfig,
};

#[derive(Debug, Clone, serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct EvidenceRecommendationSummary {
    pub total_recommendations: usize,
    pub approved_for_manual_apply: usize,
    pub latest_report_id: Option<String>,
    pub latest_generated_at: Option<i64>,
}

#[derive(Debug, Clone, serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct EvidenceExportSummary {
    pub export_id: String,
    pub recommendation_count: usize,
    pub json_path: String,
    pub markdown_path: Option<String>,
}

#[derive(Debug, Clone, serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct EvidenceDiffSummary {
    pub changed_fields: usize,
    pub unchanged_fields: usize,
    pub warnings_count: usize,
}

#[derive(Debug, Clone, serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct EvidenceRunbookSummary {
    pub risk_level: String,
    pub checklist_items: usize,
    pub rollback_steps: usize,
}

#[derive(Debug, Clone, serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct EvidenceDryRunSummary {
    pub status: DryRunStatus,
    pub can_apply_manually: bool,
    pub warning_count: usize,
    pub blocker_count: usize,
}

#[derive(Debug, Clone, serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct OperatorSignoffTemplate {
    pub signoff_required: bool,
    pub signoff_allowed: bool,
    pub operator: Option<String>,
    pub reviewer_note: Option<String>,
    pub decision_options: Vec<String>,
    pub blocking_message: Option<String>,
}

#[derive(Debug, Clone, serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct EvidenceSafetyBoundary {
    pub apply_mode: String,
    pub runtime_modified: bool,
    pub no_runtime_config_changed: bool,
    pub no_calibration_runner_triggered: bool,
    pub no_runtime_reload_triggered: bool,
    pub no_apply_or_reload_endpoint_called: bool,
    pub realtime_pipelines_modified: bool,
}

#[derive(Debug, Clone, serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ManualApplyEvidencePack {
    pub evidence_pack_id: String,
    pub export_id: String,
    pub generated_at: String,
    pub apply_mode: String,
    pub runtime_modified: bool,
    pub recommendation_summary: EvidenceRecommendationSummary,
    pub export_summary: EvidenceExportSummary,
    pub diff_summary: EvidenceDiffSummary,
    pub runbook_summary: EvidenceRunbookSummary,
    pub dry_run_summary: EvidenceDryRunSummary,
    pub warnings: Vec<String>,
    pub blockers: Vec<String>,
    pub signoff_required: bool,
    pub signoff_allowed: bool,
    pub signoff_status: String,
    pub operator_signoff_template: OperatorSignoffTemplate,
    pub safety_boundary: EvidenceSafetyBoundary,
}

pub struct ManualApplyEvidencePackStore {
    review_store: ParameterRecommendationReviewStore,
    export_store: ManualParameterExportStore,
    diff_store: ParameterPatchDiffStore,
    runbook_store: ManualApplyRunbookStore,
    dry_run_validator: ManualApplyDryRunValidator,
}

impl ManualApplyEvidencePackStore {
    pub fn new(report_dir: impl Into<PathBuf>, config: Option<AppConfig>) -> Self {
        let report_dir = report_dir.into();
        Self {
            review_store: ParameterRecommendationReviewStore::new(report_dir.clone()),
            export_store: ManualParameterExportStore::new(report_dir.clone()),
            diff_store: ParameterPatchDiffStore::new(report_dir.clone(), config.clone()),
            runbook_store: ManualApplyRunbookStore::new(report_dir.clone(), config.clone()),
            dry_run_validator: ManualApplyDryRunValidator::new(report_dir, config),
        }
    }

    pub fn latest_pack(&self) -> anyhow::Result<Option<ManualApplyEvidencePack>> {
        let Some(export) = self.export_store.latest_export()? else {
            return Ok(None);
        };
        Ok(Some(self.build_pack(export)?))
    }

    pub fn pack_by_id(&self, export_id: &str) -> anyhow::Result<Option<ManualApplyEvidencePack>> {
        let Some(export) = self.export_store.get_export(export_id)? else {
            return Ok(None);
        };
        Ok(Some(self.build_pack(export)?))
    }

    pub fn latest_markdown(&self) -> anyhow::Result<Option<String>> {
        let Some(pack) = self.latest_pack()? else {
            return Ok(None);
        };
        Ok(Some(render_markdown(&pack)))
    }

    pub fn markdown_by_id(&self, export_id: &str) -> anyhow::Result<Option<String>> {
        let Some(pack) = self.pack_by_id(export_id)? else {
            return Ok(None);
        };
        Ok(Some(render_markdown(&pack)))
    }

    fn build_pack(
        &self,
        export: ManualParameterExportEntry,
    ) -> anyhow::Result<ManualApplyEvidencePack> {
        let export_id = export.summary.export_id.clone();
        let diff = self
            .diff_store
            .diff_by_id(&export_id)?
            .ok_or_else(|| anyhow::anyhow!("current_config_unavailable"))?;
        let audit = self
            .diff_store
            .audit_by_id(&export_id)?
            .ok_or_else(|| anyhow::anyhow!("current_config_unavailable"))?;
        let runbook = self
            .runbook_store
            .runbook_by_id(&export_id)?
            .ok_or_else(|| anyhow::anyhow!("current_config_unavailable"))?;
        let dry_run = self
            .dry_run_validator
            .report_by_id(&export_id)?
            .ok_or_else(|| anyhow::anyhow!("current_config_unavailable"))?;

        let recommendations = self.review_store.list_recommendations()?;
        let related_recommendations: Vec<RecommendationCard> = recommendations
            .into_iter()
            .filter(|card| card.report_id == recommendation_report_id(&export))
            .collect();

        Ok(build_pack(
            export,
            diff,
            audit,
            runbook,
            dry_run,
            related_recommendations,
        ))
    }
}

fn build_pack(
    export: ManualParameterExportEntry,
    diff: ParameterPatchDiff,
    audit: ParameterPatchAudit,
    runbook: ManualApplyRunbook,
    dry_run: ManualApplyDryRunReport,
    related_recommendations: Vec<RecommendationCard>,
) -> ManualApplyEvidencePack {
    let export_id = export.summary.export_id.clone();
    let evidence_pack_id = format!("evidence-pack-{export_id}");
    let approved_count = related_recommendations
        .iter()
        .filter(|card| {
            card.current_review
                .as_ref()
                .map(|review| {
                    matches!(
                        review.status,
                        crate::calibration::parameter_recommendation_review_store::ReviewStatus::ApprovedForManualApply
                    )
                })
                .unwrap_or(false)
        })
        .count();
    let warnings = collect_warnings(&diff, &dry_run);
    let blockers = dry_run
        .blocking_issues
        .iter()
        .map(issue_label)
        .collect::<Vec<_>>();
    let signoff_allowed = dry_run.status != DryRunStatus::Failed;
    let signoff_status = match dry_run.status {
        DryRunStatus::Passed => "ready_for_manual_signoff",
        DryRunStatus::PassedWithWarnings => "review_warnings_before_signoff",
        DryRunStatus::Failed => "blocked_by_dry_run",
    }
    .to_string();

    ManualApplyEvidencePack {
        evidence_pack_id,
        export_id: export_id.clone(),
        generated_at: export.export.created_at.clone(),
        apply_mode: "manual_signoff_only".to_string(),
        runtime_modified: false,
        recommendation_summary: EvidenceRecommendationSummary {
            total_recommendations: related_recommendations.len(),
            approved_for_manual_apply: approved_count,
            latest_report_id: related_recommendations
                .first()
                .map(|card| card.report_id.clone()),
            latest_generated_at: related_recommendations
                .first()
                .and_then(|card| card.generated_at),
        },
        export_summary: EvidenceExportSummary {
            export_id: export_id.clone(),
            recommendation_count: export.export.items.len(),
            json_path: export.summary.json_path.clone(),
            markdown_path: export.summary.markdown_path.clone(),
        },
        diff_summary: EvidenceDiffSummary {
            changed_fields: diff.summary.changed_fields_count,
            unchanged_fields: diff.summary.unchanged_fields_count,
            warnings_count: std::cmp::max(diff.summary.warning_count, audit.warnings.len()),
        },
        runbook_summary: EvidenceRunbookSummary {
            risk_level: runbook.summary.risk_level.clone(),
            checklist_items: runbook.pre_apply_checklist.len(),
            rollback_steps: runbook.rollback_plan.len(),
        },
        dry_run_summary: EvidenceDryRunSummary {
            status: dry_run.status,
            can_apply_manually: dry_run.can_apply_manually,
            warning_count: dry_run.warnings.len(),
            blocker_count: dry_run.blocking_issues.len(),
        },
        warnings,
        blockers,
        signoff_required: true,
        signoff_allowed,
        signoff_status,
        operator_signoff_template: OperatorSignoffTemplate {
            signoff_required: true,
            signoff_allowed,
            operator: None,
            reviewer_note: None,
            decision_options: vec![
                "approved_for_manual_apply".to_string(),
                "rejected".to_string(),
                "needs_changes".to_string(),
                "deferred".to_string(),
            ],
            blocking_message: if signoff_allowed {
                None
            } else {
                Some("Operator sign-off is blocked because dry-run failed.".to_string())
            },
        },
        safety_boundary: EvidenceSafetyBoundary {
            apply_mode: "manual_signoff_only".to_string(),
            runtime_modified: false,
            no_runtime_config_changed: true,
            no_calibration_runner_triggered: true,
            no_runtime_reload_triggered: true,
            no_apply_or_reload_endpoint_called: true,
            realtime_pipelines_modified: false,
        },
    }
}

fn collect_warnings(diff: &ParameterPatchDiff, dry_run: &ManualApplyDryRunReport) -> Vec<String> {
    let mut warnings = diff.warnings.clone();
    warnings.extend(dry_run.warnings.iter().map(issue_label));
    warnings
}

fn issue_label(issue: &crate::calibration::manual_apply_dryrun_validator::DryRunIssue) -> String {
    match &issue.field_path {
        Some(field) => format!("{} [{}]: {}", issue.code, field, issue.message),
        None => format!("{}: {}", issue.code, issue.message),
    }
}

fn recommendation_report_id(export: &ManualParameterExportEntry) -> String {
    export
        .export
        .items
        .first()
        .map(|item| item.review.report_id.clone())
        .unwrap_or_default()
}

fn render_markdown(pack: &ManualApplyEvidencePack) -> String {
    let mut lines = vec![
        "# Manual Apply Evidence Pack".to_string(),
        String::new(),
        format!("Evidence Pack ID: {}", pack.evidence_pack_id),
        format!("Export ID: {}", pack.export_id),
        format!("Generated At: {}", pack.generated_at),
        format!("Apply Mode: {}", pack.apply_mode),
        format!("Runtime Modified: {}", pack.runtime_modified),
        String::new(),
        "## Recommendation Review".to_string(),
        String::new(),
        format!(
            "- Total recommendations: {}",
            pack.recommendation_summary.total_recommendations
        ),
        format!(
            "- Approved for manual apply: {}",
            pack.recommendation_summary.approved_for_manual_apply
        ),
        String::new(),
        "## Manual Parameter Export".to_string(),
        String::new(),
        format!(
            "- Recommendation count: {}",
            pack.export_summary.recommendation_count
        ),
        format!("- JSON path: {}", pack.export_summary.json_path),
        format!(
            "- Markdown path: {}",
            pack.export_summary
                .markdown_path
                .clone()
                .unwrap_or_else(|| "Unavailable".to_string())
        ),
        String::new(),
        "## Patch Diff / Audit".to_string(),
        String::new(),
        format!("- Changed fields: {}", pack.diff_summary.changed_fields),
        format!("- Unchanged fields: {}", pack.diff_summary.unchanged_fields),
        format!("- Warnings count: {}", pack.diff_summary.warnings_count),
        String::new(),
        "## Manual Apply Runbook".to_string(),
        String::new(),
        format!("- Risk level: {}", pack.runbook_summary.risk_level),
        format!(
            "- Checklist items: {}",
            pack.runbook_summary.checklist_items
        ),
        format!("- Rollback steps: {}", pack.runbook_summary.rollback_steps),
        String::new(),
        "## Dry-run Validation".to_string(),
        String::new(),
        format!(
            "- Status: {}",
            dry_run_status_label(pack.dry_run_summary.status)
        ),
        format!(
            "- Can apply manually: {}",
            pack.dry_run_summary.can_apply_manually
        ),
        format!("- Warnings: {}", pack.dry_run_summary.warning_count),
        format!("- Blockers: {}", pack.dry_run_summary.blocker_count),
        String::new(),
        "## Operator Sign-off".to_string(),
        String::new(),
        format!("- Signoff required: {}", pack.signoff_required),
        format!("- Signoff allowed: {}", pack.signoff_allowed),
        format!("- Signoff status: {}", pack.signoff_status),
    ];

    if let Some(message) = &pack.operator_signoff_template.blocking_message {
        lines.push(format!("- Blocking message: {}", message));
    }

    lines.push(String::new());
    lines.push("## Warnings".to_string());
    lines.push(String::new());
    if pack.warnings.is_empty() {
        lines.push("- None".to_string());
    } else {
        for warning in &pack.warnings {
            lines.push(format!("- {}", warning));
        }
    }

    lines.push(String::new());
    lines.push("## Blockers".to_string());
    lines.push(String::new());
    if pack.blockers.is_empty() {
        lines.push("- None".to_string());
    } else {
        for blocker in &pack.blockers {
            lines.push(format!("- {}", blocker));
        }
    }

    lines.push(String::new());
    lines.push("## Safety Boundary".to_string());
    lines.push(String::new());
    lines.push("This evidence pack is generated for manual operator review only.".to_string());
    lines.push(String::new());
    lines.push("- apply_mode = manual_signoff_only".to_string());
    lines.push("- runtime_modified = false".to_string());
    lines.push("- no runtime config was changed".to_string());
    lines.push("- no calibration runner was triggered".to_string());
    lines.push("- no runtime reload was triggered".to_string());
    lines.push("- no apply/update-config/reload endpoint was called".to_string());
    lines.push("- flow / liquidation / toxic realtime pipelines were not modified".to_string());

    lines.join("\n")
}

fn dry_run_status_label(status: DryRunStatus) -> &'static str {
    match status {
        DryRunStatus::Passed => "Passed",
        DryRunStatus::PassedWithWarnings => "PassedWithWarnings",
        DryRunStatus::Failed => "Failed",
    }
}
