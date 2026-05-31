use std::path::PathBuf;

use crate::{
    calibration::{
        calibration_report_store::CalibrationReportStore,
        manual_apply_dryrun_validator::{DryRunStatus, ManualApplyDryRunValidator},
        manual_apply_runbook::ManualApplyRunbookStore,
        manual_parameter_export::ManualParameterExportStore,
        parameter_patch_diff::ParameterPatchDiffStore,
        parameter_recommendation_review_store::{ParameterRecommendationReviewStore, ReviewStatus},
    },
    config::AppConfig,
};

#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum ManualStartupStatus {
    ReadyForManualApply,
    Blocked,
    NeedsReview,
    MissingReport,
}

#[derive(Debug, Clone, serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ManualStartupCheckItem {
    pub name: String,
    pub ok: bool,
    pub severity: String,
    pub message: String,
}

#[derive(Debug, Clone, serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ManualStartupCheckResponse {
    pub ok: bool,
    pub status: ManualStartupStatus,
    pub read_only: bool,
    pub checks: Vec<ManualStartupCheckItem>,
    pub next_action: String,
}

pub struct ManualStartupCheckStore {
    config: AppConfig,
    report_store: CalibrationReportStore,
    review_store: ParameterRecommendationReviewStore,
    export_store: ManualParameterExportStore,
    diff_store: ParameterPatchDiffStore,
    runbook_store: ManualApplyRunbookStore,
    dry_run_validator: ManualApplyDryRunValidator,
}

impl ManualStartupCheckStore {
    pub fn new(report_dir: impl Into<PathBuf>, config: AppConfig) -> Self {
        let report_dir = report_dir.into();
        Self {
            report_store: CalibrationReportStore::new(report_dir.clone()),
            review_store: ParameterRecommendationReviewStore::new(report_dir.clone()),
            export_store: ManualParameterExportStore::new(report_dir.clone()),
            diff_store: ParameterPatchDiffStore::new(report_dir.clone(), Some(config.clone())),
            runbook_store: ManualApplyRunbookStore::new(report_dir.clone(), Some(config.clone())),
            dry_run_validator: ManualApplyDryRunValidator::new(report_dir, Some(config.clone())),
            config,
        }
    }

    pub fn run_check(&self) -> anyhow::Result<ManualStartupCheckResponse> {
        let mut checks = vec![ManualStartupCheckItem {
            name: "service_online".to_string(),
            ok: true,
            severity: "info".to_string(),
            message: "manual startup check API is reachable".to_string(),
        }];

        if !self.config.read_only {
            checks.push(ManualStartupCheckItem {
                name: "read_only_guard".to_string(),
                ok: false,
                severity: "error".to_string(),
                message: "readOnly=false blocks manual apply readiness checks".to_string(),
            });
            return Ok(blocked_response(
                self.config.read_only,
                checks,
                "Restore readOnly=true before any manual apply review.".to_string(),
            ));
        }

        let Some(report) = self.report_store.latest_report()? else {
            checks.push(ManualStartupCheckItem {
                name: "calibration_report".to_string(),
                ok: false,
                severity: "warning".to_string(),
                message: "latest calibration report is missing".to_string(),
            });
            return Ok(ManualStartupCheckResponse {
                ok: false,
                status: ManualStartupStatus::MissingReport,
                read_only: true,
                checks,
                next_action: "Run calibration offline or place a calibration report under .runtime/reports first.".to_string(),
            });
        };
        checks.push(ManualStartupCheckItem {
            name: "calibration_report".to_string(),
            ok: true,
            severity: "info".to_string(),
            message: format!("latest calibration report found: {}", report.summary.id),
        });

        let latest_recommendations = self.review_store.latest_recommendations()?;
        let pending_count = latest_recommendations
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
            checks.push(ManualStartupCheckItem {
                name: "parameter_recommendation_review".to_string(),
                ok: false,
                severity: "warning".to_string(),
                message: format!("{pending_count} recommendation(s) still need review"),
            });
        } else {
            checks.push(ManualStartupCheckItem {
                name: "parameter_recommendation_review".to_string(),
                ok: true,
                severity: "info".to_string(),
                message: "latest recommendations have no pending review items".to_string(),
            });
        }

        let Some(export) = self.export_store.latest_export()? else {
            checks.push(ManualStartupCheckItem {
                name: "manual_patch_export".to_string(),
                ok: false,
                severity: "warning".to_string(),
                message: "manual patch export is missing".to_string(),
            });
            return Ok(needs_review_response(
                checks,
                "Generate a manual patch export from approved recommendations before manual apply."
                    .to_string(),
            ));
        };
        checks.push(ManualStartupCheckItem {
            name: "manual_patch_export".to_string(),
            ok: true,
            severity: "info".to_string(),
            message: format!(
                "manual patch export available: {}",
                export.summary.export_id
            ),
        });

        match self.diff_store.latest_diff() {
            Ok(Some(_)) => checks.push(ManualStartupCheckItem {
                name: "parameter_diff".to_string(),
                ok: true,
                severity: "info".to_string(),
                message: "parameter diff is readable".to_string(),
            }),
            Ok(None) => {
                checks.push(ManualStartupCheckItem {
                    name: "parameter_diff".to_string(),
                    ok: false,
                    severity: "warning".to_string(),
                    message: "parameter diff is unavailable for the latest export".to_string(),
                });
                return Ok(needs_review_response(
                    checks,
                    "Open the latest export and regenerate the diff/audit layer.".to_string(),
                ));
            }
            Err(err) if err.to_string() == "current_config_unavailable" => {
                checks.push(ManualStartupCheckItem {
                    name: "parameter_diff".to_string(),
                    ok: false,
                    severity: "error".to_string(),
                    message: "current config is unavailable for diff generation".to_string(),
                });
                return Ok(blocked_response(
                    true,
                    checks,
                    "Restore current config visibility before manual apply review.".to_string(),
                ));
            }
            Err(err) => return Err(err),
        }

        match self.runbook_store.latest_runbook() {
            Ok(Some(_)) => checks.push(ManualStartupCheckItem {
                name: "manual_runbook".to_string(),
                ok: true,
                severity: "info".to_string(),
                message: "runbook can be generated".to_string(),
            }),
            Ok(None) => {
                checks.push(ManualStartupCheckItem {
                    name: "manual_runbook".to_string(),
                    ok: false,
                    severity: "warning".to_string(),
                    message: "manual runbook is unavailable".to_string(),
                });
                return Ok(needs_review_response(
                    checks,
                    "Generate or inspect the manual apply runbook before proceeding.".to_string(),
                ));
            }
            Err(err) if err.to_string() == "current_config_unavailable" => {
                checks.push(ManualStartupCheckItem {
                    name: "manual_runbook".to_string(),
                    ok: false,
                    severity: "error".to_string(),
                    message: "current config is unavailable for runbook generation".to_string(),
                });
                return Ok(blocked_response(
                    true,
                    checks,
                    "Restore current config visibility before manual apply review.".to_string(),
                ));
            }
            Err(err) => return Err(err),
        }

        let Some(dry_run) = self.dry_run_validator.latest_report()? else {
            checks.push(ManualStartupCheckItem {
                name: "dryrun_validator".to_string(),
                ok: false,
                severity: "warning".to_string(),
                message: "manual apply dry-run validation is unavailable".to_string(),
            });
            return Ok(needs_review_response(
                checks,
                "Generate a dry-run report before entering manual apply.".to_string(),
            ));
        };

        match dry_run.status {
            DryRunStatus::Failed => {
                checks.push(ManualStartupCheckItem {
                    name: "dryrun_validator".to_string(),
                    ok: false,
                    severity: "error".to_string(),
                    message: "manual apply dry-run validation failed".to_string(),
                });
                Ok(blocked_response(
                    true,
                    checks,
                    "Fix blocking dry-run issues before manual apply.".to_string(),
                ))
            }
            DryRunStatus::PassedWithWarnings => {
                checks.push(ManualStartupCheckItem {
                    name: "dryrun_validator".to_string(),
                    ok: true,
                    severity: "warning".to_string(),
                    message: "dry-run validation passed with warnings".to_string(),
                });
                Ok(ManualStartupCheckResponse {
                    ok: false,
                    status: ManualStartupStatus::NeedsReview,
                    read_only: true,
                    checks,
                    next_action: "Review dry-run warnings, then open the manual apply runbook for human review.".to_string(),
                })
            }
            DryRunStatus::Passed => {
                checks.push(ManualStartupCheckItem {
                    name: "dryrun_validator".to_string(),
                    ok: true,
                    severity: "info".to_string(),
                    message: "dry-run validation passed".to_string(),
                });
                let has_review_warnings = checks
                    .iter()
                    .any(|check| !check.ok && check.severity == "warning");
                if has_review_warnings {
                    Ok(needs_review_response(
                        checks,
                        "Review warnings before manual apply.".to_string(),
                    ))
                } else {
                    Ok(ManualStartupCheckResponse {
                        ok: true,
                        status: ManualStartupStatus::ReadyForManualApply,
                        read_only: true,
                        checks,
                        next_action: "Open manual apply runbook and execute manually if approved."
                            .to_string(),
                    })
                }
            }
        }
    }
}

fn needs_review_response(
    checks: Vec<ManualStartupCheckItem>,
    next_action: String,
) -> ManualStartupCheckResponse {
    ManualStartupCheckResponse {
        ok: false,
        status: ManualStartupStatus::NeedsReview,
        read_only: true,
        checks,
        next_action,
    }
}

fn blocked_response(
    read_only: bool,
    checks: Vec<ManualStartupCheckItem>,
    next_action: String,
) -> ManualStartupCheckResponse {
    ManualStartupCheckResponse {
        ok: false,
        status: ManualStartupStatus::Blocked,
        read_only,
        checks,
        next_action,
    }
}
