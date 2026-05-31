use std::path::PathBuf;

use chrono::{Local, TimeZone};

use crate::{
    calibration::{
        manual_apply_dryrun_validator::{
            DryRunStatus, ManualApplyDryRunReport, ManualApplyDryRunValidator,
        },
        manual_apply_evidence_pack::ManualApplyEvidencePackStore,
        manual_apply_runbook::{ManualApplyRunbook, ManualApplyRunbookStore},
        manual_audit_story::ManualAuditStoryStore,
        manual_evidence_freshness::{ManualEvidenceFreshnessStatus, ManualEvidenceFreshnessStore},
        manual_parameter_export::{ManualParameterExportEntry, ManualParameterExportStore},
        manual_signoff_store::{
            ManualSignoffGateStatus, ManualSignoffStatusResponse, ManualSignoffStore,
        },
        manual_startup_check::{ManualStartupCheckStore, ManualStartupStatus},
        parameter_patch_diff::{ParameterPatchAudit, ParameterPatchDiffStore},
        parameter_recommendation_review_store::{
            ParameterRecommendationReviewStore, RecommendationCard, ReviewStatus,
        },
    },
    config::AppConfig,
};

#[derive(Debug, Clone, serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct GovernanceIssue {
    pub code: String,
    pub severity: String,
    pub message: String,
    pub next_action: String,
}

#[derive(Debug, Clone, serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct GovernanceStageStatus {
    pub stage: String,
    pub status: String,
    pub artifact_id: Option<String>,
    pub summary: String,
    pub blocking: bool,
    pub warnings: Vec<String>,
    pub updated_at: Option<String>,
}

#[derive(Debug, Clone, serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ManualApplyGovernanceLinks {
    pub startup_check: String,
    pub signoff_status: String,
    pub evidence_freshness: String,
    pub audit_story: String,
    pub evidence_pack: String,
    pub runbook: String,
    pub dry_run: String,
    pub patch_diff_audit: String,
}

#[derive(Debug, Clone, serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ManualApplyGovernanceEvidence {
    pub current_evidence_fingerprint: Option<String>,
    pub latest_signoff_fingerprint: Option<String>,
    pub changed_evidence: Vec<String>,
    pub ttl_ms: Option<i64>,
    pub age_ms: Option<i64>,
    pub expires_in_ms: Option<i64>,
}

#[derive(Debug, Clone, serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ManualApplyGovernanceIndex {
    pub read_only: bool,
    pub runtime_modified: bool,
    pub apply_mode: String,
    pub governance_status: String,
    pub final_gate: String,
    pub readiness_status: String,
    pub signoff_status: String,
    pub freshness_status: String,
    pub audit_story_status: String,
    pub latest_export_id: Option<String>,
    pub latest_evidence_pack_id: Option<String>,
    pub latest_audit_story_id: Option<String>,
    pub current_blocker: Option<String>,
    pub stages: Vec<GovernanceStageStatus>,
    pub blocking_issues: Vec<GovernanceIssue>,
    pub blocking_reasons: Vec<GovernanceIssue>,
    pub warnings: Vec<GovernanceIssue>,
    pub next_owner: String,
    pub next_action: String,
    pub next_required_action: String,
    pub evidence: ManualApplyGovernanceEvidence,
    pub links: ManualApplyGovernanceLinks,
    pub safety_boundary: Vec<String>,
    pub generated_at_ms: i64,
    pub markdown: String,
}

pub struct ManualApplyGovernanceIndexStore {
    review_store: ParameterRecommendationReviewStore,
    export_store: ManualParameterExportStore,
    diff_store: ParameterPatchDiffStore,
    runbook_store: ManualApplyRunbookStore,
    dry_run_validator: ManualApplyDryRunValidator,
    evidence_pack_store: ManualApplyEvidencePackStore,
    startup_store: ManualStartupCheckStore,
    signoff_store: ManualSignoffStore,
    freshness_store: ManualEvidenceFreshnessStore,
    audit_story_store: ManualAuditStoryStore,
}

impl ManualApplyGovernanceIndexStore {
    pub fn new(report_dir: impl Into<PathBuf>, config: AppConfig) -> Self {
        let report_dir = report_dir.into();
        Self {
            review_store: ParameterRecommendationReviewStore::new(report_dir.clone()),
            export_store: ManualParameterExportStore::new(report_dir.clone()),
            diff_store: ParameterPatchDiffStore::new(report_dir.clone(), Some(config.clone())),
            runbook_store: ManualApplyRunbookStore::new(report_dir.clone(), Some(config.clone())),
            dry_run_validator: ManualApplyDryRunValidator::new(
                report_dir.clone(),
                Some(config.clone()),
            ),
            evidence_pack_store: ManualApplyEvidencePackStore::new(
                report_dir.clone(),
                Some(config.clone()),
            ),
            startup_store: ManualStartupCheckStore::new(report_dir.clone(), config.clone()),
            signoff_store: ManualSignoffStore::new(report_dir.clone(), config.clone()),
            freshness_store: ManualEvidenceFreshnessStore::new(report_dir.clone(), config.clone()),
            audit_story_store: ManualAuditStoryStore::new(report_dir, config),
        }
    }

    pub fn build_index(&self) -> anyhow::Result<ManualApplyGovernanceIndex> {
        let now_ms = Local::now().timestamp_millis();
        let recommendations = self.review_store.latest_recommendations()?;
        let latest_export = self.export_store.latest_export()?;
        let latest_diff = self.read_optional(|| self.diff_store.latest_audit())?;
        let latest_runbook = self.read_optional(|| self.runbook_store.latest_runbook())?;
        let latest_dry_run = self.read_optional(|| self.dry_run_validator.latest_report())?;
        let latest_evidence_pack = self.read_optional(|| self.evidence_pack_store.latest_pack())?;
        let startup = self.startup_store.run_check()?;
        let signoff = self.signoff_store.status()?;
        let freshness = self.freshness_store.freshness()?;
        let audit_story = self.audit_story_store.build_story().ok();

        let readiness_status = startup_status_label(startup.status).to_string();
        let signoff_status = signoff_status_label(signoff.status).to_string();
        let freshness_status = freshness_status_label(freshness.status).to_string();
        let audit_story_status = if audit_story.is_some() {
            "AVAILABLE".to_string()
        } else {
            "UNAVAILABLE".to_string()
        };
        let final_gate = audit_story
            .as_ref()
            .map(|story| story.final_gate.clone())
            .unwrap_or_else(|| "BLOCKED".to_string());

        let governance_status = derive_governance_status(
            startup.status,
            signoff.status,
            freshness.status,
            audit_story.is_some(),
        )
        .to_string();

        let blocking_issues = build_blocking_issues(
            startup.status,
            signoff.status,
            freshness.status,
            audit_story.is_some(),
        );
        let warnings = build_warning_issues(
            &recommendations,
            latest_export.is_none(),
            latest_diff.is_none(),
            latest_runbook.is_none(),
            latest_dry_run.is_none(),
            latest_evidence_pack.is_none(),
        );

        let (next_owner, next_required_action) = if let Some(story) = audit_story.as_ref() {
            (story.next_owner.clone(), story.next_action.clone())
        } else {
            derive_fallback_owner_action(startup.status, signoff.status, freshness.status)
        };

        let latest_export_id = latest_export
            .as_ref()
            .map(|entry| entry.summary.export_id.clone());
        let latest_evidence_pack_id = latest_evidence_pack
            .as_ref()
            .map(|pack| pack.evidence_pack_id.clone());
        let latest_audit_story_id = latest_export_id
            .as_ref()
            .map(|export_id| format!("manual-audit-story:{export_id}"));

        let stages = vec![
            review_stage(&recommendations),
            export_stage(latest_export.as_ref()),
            patch_diff_stage(latest_diff.as_ref()),
            runbook_stage(latest_runbook.as_ref()),
            dry_run_stage(latest_dry_run.as_ref()),
            signoff_stage(&signoff),
        ];

        let mut index = ManualApplyGovernanceIndex {
            read_only: true,
            runtime_modified: false,
            apply_mode: "governance_index_only".to_string(),
            governance_status,
            final_gate,
            readiness_status,
            signoff_status,
            freshness_status,
            audit_story_status,
            latest_export_id,
            latest_evidence_pack_id,
            latest_audit_story_id,
            current_blocker: blocking_issues.first().map(|issue| issue.code.clone()),
            stages,
            blocking_issues: blocking_issues.clone(),
            blocking_reasons: blocking_issues,
            warnings,
            next_owner,
            next_action: next_required_action.clone(),
            next_required_action,
            evidence: ManualApplyGovernanceEvidence {
                current_evidence_fingerprint: Some(freshness.current_evidence_fingerprint.clone()),
                latest_signoff_fingerprint: freshness.latest_signoff_fingerprint.clone(),
                changed_evidence: freshness.changed_evidence.clone(),
                ttl_ms: Some(freshness.ttl_ms),
                age_ms: freshness.age_ms,
                expires_in_ms: freshness.expires_in_ms,
            },
            links: ManualApplyGovernanceLinks {
                startup_check: "/api/calibration/manual-startup/check".to_string(),
                signoff_status: "/api/calibration/manual-signoff/status".to_string(),
                evidence_freshness: "/api/calibration/manual-evidence/freshness".to_string(),
                audit_story: "/api/calibration/manual-audit-story".to_string(),
                evidence_pack: "/api/parameter-review/exports/latest/evidence-pack".to_string(),
                runbook: "/api/parameter-review/exports/latest/runbook".to_string(),
                dry_run: "/api/parameter-review/exports/latest/dry-run".to_string(),
                patch_diff_audit: "/api/parameter-review/exports/latest/audit".to_string(),
            },
            safety_boundary: safety_boundary(),
            generated_at_ms: now_ms,
            markdown: String::new(),
        };
        index.markdown = render_markdown(&index);
        Ok(index)
    }

    pub fn markdown(&self) -> anyhow::Result<String> {
        Ok(self.build_index()?.markdown)
    }

    fn read_optional<T>(
        &self,
        reader: impl FnOnce() -> anyhow::Result<Option<T>>,
    ) -> anyhow::Result<Option<T>> {
        match reader() {
            Ok(value) => Ok(value),
            Err(err) if err.to_string() == "current_config_unavailable" => Ok(None),
            Err(err) => Err(err),
        }
    }
}

pub fn derive_governance_status(
    startup_status: ManualStartupStatus,
    signoff_status: ManualSignoffGateStatus,
    freshness_status: ManualEvidenceFreshnessStatus,
    audit_story_available: bool,
) -> &'static str {
    if startup_status == ManualStartupStatus::ReadyForManualApply
        && signoff_status == ManualSignoffGateStatus::SignedOff
        && freshness_status == ManualEvidenceFreshnessStatus::Fresh
        && audit_story_available
    {
        return "READY_FOR_EXTERNAL_MANUAL_APPLY";
    }

    if freshness_status == ManualEvidenceFreshnessStatus::MissingEvidence
        || startup_status == ManualStartupStatus::MissingReport
    {
        return "BLOCKED_BY_MISSING_EVIDENCE";
    }
    if startup_status != ManualStartupStatus::ReadyForManualApply {
        return "BLOCKED_BY_READINESS";
    }
    if freshness_status == ManualEvidenceFreshnessStatus::Stale
        || signoff_status == ManualSignoffGateStatus::SignoffStale
    {
        return "BLOCKED_BY_STALE_EVIDENCE";
    }
    if freshness_status == ManualEvidenceFreshnessStatus::Expired
        || signoff_status == ManualSignoffGateStatus::SignoffExpired
    {
        return "BLOCKED_BY_EXPIRED_SIGNOFF";
    }
    if !audit_story_available {
        return "BLOCKED_BY_AUDIT_STORY";
    }
    if signoff_status != ManualSignoffGateStatus::SignedOff {
        return "BLOCKED_BY_SIGNOFF";
    }
    if startup_status != ManualStartupStatus::ReadyForManualApply {
        return "BLOCKED_BY_READINESS";
    }
    "UNKNOWN"
}

fn review_stage(recommendations: &[RecommendationCard]) -> GovernanceStageStatus {
    let pending_count = recommendations
        .iter()
        .filter(|card| {
            card.current_review.is_none()
                || card
                    .current_review
                    .as_ref()
                    .map(|review| {
                        matches!(
                            review.status,
                            ReviewStatus::Pending
                                | ReviewStatus::Watch
                                | ReviewStatus::NeedsMoreData
                        )
                    })
                    .unwrap_or(false)
        })
        .count();
    let report_id = recommendations.first().map(|card| card.report_id.clone());
    let updated_at = recommendations
        .first()
        .and_then(|card| card.generated_at)
        .and_then(format_timestamp_ms);

    if recommendations.is_empty() {
        return GovernanceStageStatus {
            stage: "recommendation_review".to_string(),
            status: "missing".to_string(),
            artifact_id: None,
            summary: "No recommendation review evidence is available yet.".to_string(),
            blocking: true,
            warnings: vec!["Latest recommendation review is unavailable.".to_string()],
            updated_at: None,
        };
    }

    GovernanceStageStatus {
        stage: "recommendation_review".to_string(),
        status: if pending_count > 0 {
            "warning".to_string()
        } else {
            "passed".to_string()
        },
        artifact_id: report_id,
        summary: if pending_count > 0 {
            format!("{pending_count} recommendation(s) still need review.")
        } else {
            "All latest recommendations have a review decision.".to_string()
        },
        blocking: false,
        warnings: if pending_count > 0 {
            vec![
                "Recommendation review is still pending for part of the latest report.".to_string(),
            ]
        } else {
            Vec::new()
        },
        updated_at,
    }
}

fn export_stage(export: Option<&ManualParameterExportEntry>) -> GovernanceStageStatus {
    match export {
        Some(entry) => GovernanceStageStatus {
            stage: "manual_export".to_string(),
            status: "present".to_string(),
            artifact_id: Some(entry.summary.export_id.clone()),
            summary: format!(
                "Manual export is available with {} approved recommendation(s).",
                entry.summary.recommendation_count
            ),
            blocking: false,
            warnings: Vec::new(),
            updated_at: entry.summary.created_at_ms.and_then(format_timestamp_ms),
        },
        None => GovernanceStageStatus {
            stage: "manual_export".to_string(),
            status: "missing".to_string(),
            artifact_id: None,
            summary: "Manual export has not been generated yet.".to_string(),
            blocking: true,
            warnings: vec!["Manual export is missing.".to_string()],
            updated_at: None,
        },
    }
}

fn patch_diff_stage(diff: Option<&ParameterPatchAudit>) -> GovernanceStageStatus {
    match diff {
        Some(diff) => GovernanceStageStatus {
            stage: "patch_diff_audit".to_string(),
            status: "present".to_string(),
            artifact_id: Some(diff.export_id.clone()),
            summary: format!(
                "Patch diff / audit is available with {} changed field(s).",
                diff.summary.changed_fields_count
            ),
            blocking: false,
            warnings: diff.warnings.clone(),
            updated_at: None,
        },
        None => GovernanceStageStatus {
            stage: "patch_diff_audit".to_string(),
            status: "not_available".to_string(),
            artifact_id: None,
            summary: "Patch diff / audit is unavailable.".to_string(),
            blocking: false,
            warnings: vec!["Patch diff / audit could not be loaded.".to_string()],
            updated_at: None,
        },
    }
}

fn runbook_stage(runbook: Option<&ManualApplyRunbook>) -> GovernanceStageStatus {
    match runbook {
        Some(runbook) => GovernanceStageStatus {
            stage: "manual_apply_runbook".to_string(),
            status: "present".to_string(),
            artifact_id: Some(runbook.export_id.clone()),
            summary: format!(
                "Manual apply runbook is available with risk level {}.",
                runbook.summary.risk_level
            ),
            blocking: false,
            warnings: Vec::new(),
            updated_at: None,
        },
        None => GovernanceStageStatus {
            stage: "manual_apply_runbook".to_string(),
            status: "not_available".to_string(),
            artifact_id: None,
            summary: "Manual apply runbook is unavailable.".to_string(),
            blocking: false,
            warnings: vec!["Manual apply runbook could not be loaded.".to_string()],
            updated_at: None,
        },
    }
}

fn dry_run_stage(dry_run: Option<&ManualApplyDryRunReport>) -> GovernanceStageStatus {
    match dry_run {
        Some(dry_run) => GovernanceStageStatus {
            stage: "dry_run_validation".to_string(),
            status: match dry_run.status {
                DryRunStatus::Passed => "passed".to_string(),
                DryRunStatus::PassedWithWarnings => "warning".to_string(),
                DryRunStatus::Failed => "failed".to_string(),
            },
            artifact_id: Some(dry_run.export_id.clone()),
            summary: format!(
                "Dry-run validation is {}.",
                dry_run_status_label(dry_run.status)
            ),
            blocking: dry_run.status == DryRunStatus::Failed,
            warnings: dry_run
                .warnings
                .iter()
                .map(|issue| issue.message.clone())
                .collect(),
            updated_at: None,
        },
        None => GovernanceStageStatus {
            stage: "dry_run_validation".to_string(),
            status: "not_available".to_string(),
            artifact_id: None,
            summary: "Dry-run validation is unavailable.".to_string(),
            blocking: false,
            warnings: vec!["Dry-run validation report could not be loaded.".to_string()],
            updated_at: None,
        },
    }
}

fn signoff_stage(signoff: &ManualSignoffStatusResponse) -> GovernanceStageStatus {
    GovernanceStageStatus {
        stage: "operator_signoff".to_string(),
        status: match signoff.status {
            ManualSignoffGateStatus::SignedOff => "passed".to_string(),
            ManualSignoffGateStatus::NoSignoff => "missing".to_string(),
            ManualSignoffGateStatus::Rejected => "failed".to_string(),
            ManualSignoffGateStatus::SignoffStale | ManualSignoffGateStatus::SignoffExpired => {
                "warning".to_string()
            }
            ManualSignoffGateStatus::ReadinessNotReady => "warning".to_string(),
        },
        artifact_id: signoff
            .latest_signoff
            .as_ref()
            .map(|record| record.signoff_id.clone()),
        summary: format!(
            "Operator sign-off status is {}.",
            signoff_status_label(signoff.status)
        ),
        blocking: matches!(
            signoff.status,
            ManualSignoffGateStatus::NoSignoff | ManualSignoffGateStatus::Rejected
        ),
        warnings: if signoff.status == ManualSignoffGateStatus::SignedOff {
            Vec::new()
        } else {
            vec![signoff.next_action.clone()]
        },
        updated_at: signoff
            .latest_signoff
            .as_ref()
            .and_then(|record| format_timestamp_ms(record.created_at_ms)),
    }
}

fn build_blocking_issues(
    startup_status: ManualStartupStatus,
    signoff_status: ManualSignoffGateStatus,
    freshness_status: ManualEvidenceFreshnessStatus,
    audit_story_available: bool,
) -> Vec<GovernanceIssue> {
    let mut issues = Vec::new();

    if startup_status != ManualStartupStatus::ReadyForManualApply {
        issues.push(match startup_status {
            ManualStartupStatus::Blocked | ManualStartupStatus::NeedsReview => GovernanceIssue {
                code: "READINESS_NOT_READY".to_string(),
                severity: "error".to_string(),
                message: "Startup readiness is not ready.".to_string(),
                next_action: "Run Manual Startup Check and resolve failed checks.".to_string(),
            },
            ManualStartupStatus::MissingReport => GovernanceIssue {
                code: "MISSING_EVIDENCE".to_string(),
                severity: "error".to_string(),
                message: "Required manual-apply evidence is missing.".to_string(),
                next_action:
                    "Restore or regenerate report / export / diff / runbook / dry-run evidence."
                        .to_string(),
            },
            ManualStartupStatus::ReadyForManualApply => unreachable!(),
        });
    }

    match signoff_status {
        ManualSignoffGateStatus::NoSignoff => issues.push(GovernanceIssue {
            code: "SIGNOFF_MISSING".to_string(),
            severity: "warning".to_string(),
            message: "Operator sign-off is missing.".to_string(),
            next_action: "Review evidence pack and create a valid sign-off.".to_string(),
        }),
        ManualSignoffGateStatus::Rejected => issues.push(GovernanceIssue {
            code: "SIGNOFF_REJECTED".to_string(),
            severity: "warning".to_string(),
            message: "Latest operator sign-off rejected manual apply.".to_string(),
            next_action: "Review evidence again before requesting another sign-off.".to_string(),
        }),
        ManualSignoffGateStatus::SignoffStale => issues.push(GovernanceIssue {
            code: "EVIDENCE_STALE".to_string(),
            severity: "warning".to_string(),
            message: "Evidence changed after sign-off.".to_string(),
            next_action: "Review changed evidence and sign off again.".to_string(),
        }),
        ManualSignoffGateStatus::SignoffExpired => issues.push(GovernanceIssue {
            code: "SIGNOFF_EXPIRED".to_string(),
            severity: "warning".to_string(),
            message: "Latest sign-off has expired.".to_string(),
            next_action: "Refresh evidence and create a new sign-off.".to_string(),
        }),
        _ => {}
    }

    match freshness_status {
        ManualEvidenceFreshnessStatus::Stale => issues.push(GovernanceIssue {
            code: "EVIDENCE_STALE".to_string(),
            severity: "warning".to_string(),
            message: "Evidence changed after sign-off.".to_string(),
            next_action: "Review changed evidence and sign off again.".to_string(),
        }),
        ManualEvidenceFreshnessStatus::Expired => issues.push(GovernanceIssue {
            code: "EVIDENCE_EXPIRED".to_string(),
            severity: "warning".to_string(),
            message: "Evidence freshness has expired.".to_string(),
            next_action: "Refresh evidence and create a new sign-off.".to_string(),
        }),
        ManualEvidenceFreshnessStatus::MissingEvidence => issues.push(GovernanceIssue {
            code: "MISSING_EVIDENCE".to_string(),
            severity: "error".to_string(),
            message: "Manual apply evidence is incomplete.".to_string(),
            next_action:
                "Restore or regenerate report / export / diff / runbook / dry-run evidence."
                    .to_string(),
        }),
        _ => {}
    }

    if !audit_story_available {
        issues.push(GovernanceIssue {
            code: "AUDIT_STORY_UNAVAILABLE".to_string(),
            severity: "warning".to_string(),
            message: "Manual audit story is unavailable.".to_string(),
            next_action: "Refresh Manual Audit Story export.".to_string(),
        });
    }

    issues
}

fn build_warning_issues(
    recommendations: &[RecommendationCard],
    export_missing: bool,
    diff_missing: bool,
    runbook_missing: bool,
    dry_run_missing: bool,
    evidence_pack_missing: bool,
) -> Vec<GovernanceIssue> {
    let mut warnings = Vec::new();
    let pending_count = recommendations
        .iter()
        .filter(|card| {
            card.current_review.is_none()
                || card
                    .current_review
                    .as_ref()
                    .map(|review| {
                        matches!(
                            review.status,
                            ReviewStatus::Pending
                                | ReviewStatus::Watch
                                | ReviewStatus::NeedsMoreData
                        )
                    })
                    .unwrap_or(false)
        })
        .count();

    if pending_count > 0 {
        warnings.push(GovernanceIssue {
            code: "REVIEW_PENDING".to_string(),
            severity: "warning".to_string(),
            message: format!("{pending_count} recommendation(s) still need review."),
            next_action: "Review outstanding recommendations before external manual apply."
                .to_string(),
        });
    }
    if export_missing {
        warnings.push(GovernanceIssue {
            code: "EXPORT_UNAVAILABLE".to_string(),
            severity: "warning".to_string(),
            message: "Manual export is unavailable.".to_string(),
            next_action: "Generate a manual export from approved recommendations.".to_string(),
        });
    }
    if diff_missing {
        warnings.push(GovernanceIssue {
            code: "PATCH_DIFF_UNAVAILABLE".to_string(),
            severity: "warning".to_string(),
            message: "Patch diff / audit is unavailable.".to_string(),
            next_action: "Refresh patch diff / audit before continuing.".to_string(),
        });
    }
    if runbook_missing {
        warnings.push(GovernanceIssue {
            code: "RUNBOOK_UNAVAILABLE".to_string(),
            severity: "warning".to_string(),
            message: "Manual apply runbook is unavailable.".to_string(),
            next_action: "Refresh the manual apply runbook.".to_string(),
        });
    }
    if dry_run_missing {
        warnings.push(GovernanceIssue {
            code: "DRY_RUN_UNAVAILABLE".to_string(),
            severity: "warning".to_string(),
            message: "Dry-run validation report is unavailable.".to_string(),
            next_action: "Generate or refresh the dry-run validation report.".to_string(),
        });
    }
    if evidence_pack_missing {
        warnings.push(GovernanceIssue {
            code: "EVIDENCE_PACK_UNAVAILABLE".to_string(),
            severity: "warning".to_string(),
            message: "Evidence pack is unavailable.".to_string(),
            next_action: "Refresh the evidence pack before operator review.".to_string(),
        });
    }
    warnings
}

fn derive_fallback_owner_action(
    startup_status: ManualStartupStatus,
    signoff_status: ManualSignoffGateStatus,
    freshness_status: ManualEvidenceFreshnessStatus,
) -> (String, String) {
    if freshness_status == ManualEvidenceFreshnessStatus::Stale
        || signoff_status == ManualSignoffGateStatus::SignoffStale
    {
        return (
            "Reviewer".to_string(),
            "Review changed evidence and sign off again.".to_string(),
        );
    }
    if freshness_status == ManualEvidenceFreshnessStatus::Expired
        || signoff_status == ManualSignoffGateStatus::SignoffExpired
    {
        return (
            "Operator".to_string(),
            "Refresh evidence and create a new sign-off.".to_string(),
        );
    }
    if signoff_status != ManualSignoffGateStatus::SignedOff {
        return (
            "Reviewer / Approver".to_string(),
            "Review evidence pack and create a valid sign-off.".to_string(),
        );
    }
    if startup_status != ManualStartupStatus::ReadyForManualApply {
        return (
            "Operator".to_string(),
            "Run Manual Startup Check and resolve failed checks.".to_string(),
        );
    }
    (
        "External Manual Executor".to_string(),
        "Follow the manual runbook outside this system.".to_string(),
    )
}

fn render_markdown(index: &ManualApplyGovernanceIndex) -> String {
    let stage_summary = index
        .stages
        .iter()
        .map(|stage| {
            format!(
                "- {}: {}{}{}",
                stage.stage,
                stage.status,
                stage
                    .artifact_id
                    .as_ref()
                    .map(|value| format!(" | artifact={value}"))
                    .unwrap_or_default(),
                stage
                    .updated_at
                    .as_ref()
                    .map(|value| format!(" | updatedAt={value}"))
                    .unwrap_or_default()
            )
        })
        .collect::<Vec<_>>()
        .join("\n");
    let blocking_issues = if index.blocking_issues.is_empty() {
        "- None".to_string()
    } else {
        index
            .blocking_issues
            .iter()
            .map(|issue| {
                format!(
                    "- {} [{}]: {} | next: {}",
                    issue.code, issue.severity, issue.message, issue.next_action
                )
            })
            .collect::<Vec<_>>()
            .join("\n")
    };
    let warnings = if index.warnings.is_empty() {
        "- None".to_string()
    } else {
        index
            .warnings
            .iter()
            .map(|issue| {
                format!(
                    "- {} [{}]: {} | next: {}",
                    issue.code, issue.severity, issue.message, issue.next_action
                )
            })
            .collect::<Vec<_>>()
            .join("\n")
    };
    let safety_boundary = index
        .safety_boundary
        .iter()
        .map(|item| format!("- {item}"))
        .collect::<Vec<_>>()
        .join("\n");

    format!(
        "# Manual Apply Governance Index\n\n\
Read-only: {}\n\
Runtime modified: {}\n\
Apply mode: {}\n\n\
## Summary\n\
- Latest Export ID: {}\n\
- Latest Evidence Pack ID: {}\n\
- Latest Audit Story ID: {}\n\
- Governance Status: {}\n\
- Final Gate: {}\n\
- Current Blocker: {}\n\
- Next Owner: {}\n\
- Next Required Action: {}\n\n\
## Artifact Links\n\
- Startup Check: {}\n\
- Sign-off Status: {}\n\
- Evidence Freshness: {}\n\
- Audit Story: {}\n\
- Evidence Pack: {}\n\
- Runbook: {}\n\
- Dry-run: {}\n\
- Patch Diff / Audit: {}\n\n\
## Stage Summary\n\
{}\n\n\
## Blocking Issues\n\
{}\n\n\
## Warnings\n\
{}\n\n\
## Next Required Action\n\
- {}\n\n\
## Operator Handoff\n\
- Next Owner: {}\n\
- Next Action: {}\n\n\
## Safety Boundary\n\
{}\n",
        index.read_only,
        index.runtime_modified,
        index.apply_mode,
        index
            .latest_export_id
            .clone()
            .unwrap_or_else(|| "Unavailable".to_string()),
        index
            .latest_evidence_pack_id
            .clone()
            .unwrap_or_else(|| "Unavailable".to_string()),
        index
            .latest_audit_story_id
            .clone()
            .unwrap_or_else(|| "Unavailable".to_string()),
        index.governance_status,
        index.final_gate,
        index
            .blocking_issues
            .first()
            .map(|issue| issue.code.clone())
            .unwrap_or_else(|| "None".to_string()),
        index.next_owner,
        index.next_required_action,
        index.links.startup_check,
        index.links.signoff_status,
        index.links.evidence_freshness,
        index.links.audit_story,
        index.links.evidence_pack,
        index.links.runbook,
        index.links.dry_run,
        index.links.patch_diff_audit,
        stage_summary,
        blocking_issues,
        warnings,
        index.next_required_action,
        index.next_owner,
        index.next_required_action,
        safety_boundary
    )
}

fn safety_boundary() -> Vec<String> {
    vec![
        "This governance index is read-only.".to_string(),
        "readOnly=true".to_string(),
        "runtimeModified=false".to_string(),
        "No runtime config was changed.".to_string(),
        "No runtime reload was triggered.".to_string(),
        "No calibration runner was triggered.".to_string(),
        "No apply/update-config/reload endpoint was called.".to_string(),
        "Flow / liquidation / toxic realtime pipelines were not modified.".to_string(),
    ]
}

fn startup_status_label(status: ManualStartupStatus) -> &'static str {
    match status {
        ManualStartupStatus::ReadyForManualApply => "READY_FOR_MANUAL_APPLY",
        ManualStartupStatus::Blocked => "BLOCKED",
        ManualStartupStatus::NeedsReview => "NEEDS_REVIEW",
        ManualStartupStatus::MissingReport => "MISSING_REPORT",
    }
}

fn signoff_status_label(status: ManualSignoffGateStatus) -> &'static str {
    match status {
        ManualSignoffGateStatus::NoSignoff => "NO_SIGNOFF",
        ManualSignoffGateStatus::SignedOff => "SIGNED_OFF",
        ManualSignoffGateStatus::Rejected => "REJECTED",
        ManualSignoffGateStatus::SignoffStale => "SIGNOFF_STALE",
        ManualSignoffGateStatus::SignoffExpired => "SIGNOFF_EXPIRED",
        ManualSignoffGateStatus::ReadinessNotReady => "READINESS_NOT_READY",
    }
}

fn freshness_status_label(status: ManualEvidenceFreshnessStatus) -> &'static str {
    match status {
        ManualEvidenceFreshnessStatus::Fresh => "FRESH",
        ManualEvidenceFreshnessStatus::Stale => "STALE",
        ManualEvidenceFreshnessStatus::Expired => "EXPIRED",
        ManualEvidenceFreshnessStatus::NoSignoff => "NO_SIGNOFF",
        ManualEvidenceFreshnessStatus::ReadinessNotReady => "READINESS_NOT_READY",
        ManualEvidenceFreshnessStatus::MissingEvidence => "MISSING_EVIDENCE",
    }
}

fn dry_run_status_label(status: DryRunStatus) -> &'static str {
    match status {
        DryRunStatus::Passed => "passed",
        DryRunStatus::PassedWithWarnings => "passed_with_warnings",
        DryRunStatus::Failed => "failed",
    }
}

fn format_timestamp_ms(ts_ms: i64) -> Option<String> {
    Local
        .timestamp_millis_opt(ts_ms)
        .single()
        .map(|value| value.to_rfc3339())
}
