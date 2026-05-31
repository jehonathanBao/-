use std::{
    collections::hash_map::DefaultHasher,
    hash::{Hash, Hasher},
    path::PathBuf,
};

use chrono::Local;
use serde::Serialize;

use crate::{
    calibration::{
        calibration_report_store::CalibrationReportStore,
        manual_apply_dryrun_validator::ManualApplyDryRunValidator,
        manual_apply_runbook::ManualApplyRunbookStore,
        manual_parameter_export::ManualParameterExportStore,
        manual_signoff_store::{
            ManualSignoffRecord, ManualSignoffStore, DEFAULT_MANUAL_SIGNOFF_TTL_MS,
        },
        manual_startup_check::{ManualStartupCheckStore, ManualStartupStatus},
        parameter_patch_diff::ParameterPatchDiffStore,
        parameter_recommendation_review_store::{
            review_ledger_timestamp, ParameterRecommendationReviewStore,
        },
    },
    config::AppConfig,
};

#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum ManualEvidenceFreshnessStatus {
    Fresh,
    Stale,
    Expired,
    NoSignoff,
    ReadinessNotReady,
    MissingEvidence,
}

#[derive(Debug, Clone, serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct EvidenceFreshnessCheck {
    pub name: String,
    pub present: bool,
    pub fresh: bool,
    pub changed_since_signoff: bool,
    pub current_hash: Option<String>,
    pub signoff_hash: Option<String>,
    pub current_modified_at_ms: Option<i64>,
    pub signoff_modified_at_ms: Option<i64>,
    pub message: String,
}

#[derive(Debug, Clone, serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ManualEvidenceFreshnessResponse {
    pub read_only: bool,
    pub status: ManualEvidenceFreshnessStatus,
    pub current_evidence_fingerprint: String,
    pub latest_signoff_fingerprint: Option<String>,
    pub ttl_ms: i64,
    pub age_ms: Option<i64>,
    pub expires_in_ms: Option<i64>,
    pub changed_evidence: Vec<String>,
    pub checks: Vec<EvidenceFreshnessCheck>,
    pub next_action: String,
}

pub struct ManualEvidenceFreshnessStore {
    config: AppConfig,
    startup_store: ManualStartupCheckStore,
    signoff_store: ManualSignoffStore,
    report_store: CalibrationReportStore,
    review_store: ParameterRecommendationReviewStore,
    export_store: ManualParameterExportStore,
    diff_store: ParameterPatchDiffStore,
    runbook_store: ManualApplyRunbookStore,
    dry_run_validator: ManualApplyDryRunValidator,
}

impl ManualEvidenceFreshnessStore {
    pub fn new(report_dir: impl Into<PathBuf>, config: AppConfig) -> Self {
        let report_dir = report_dir.into();
        Self {
            startup_store: ManualStartupCheckStore::new(report_dir.clone(), config.clone()),
            signoff_store: ManualSignoffStore::new(report_dir.clone(), config.clone()),
            report_store: CalibrationReportStore::new(report_dir.clone()),
            review_store: ParameterRecommendationReviewStore::new(report_dir.clone()),
            export_store: ManualParameterExportStore::new(report_dir.clone()),
            diff_store: ParameterPatchDiffStore::new(report_dir.clone(), Some(config.clone())),
            runbook_store: ManualApplyRunbookStore::new(report_dir.clone(), Some(config.clone())),
            dry_run_validator: ManualApplyDryRunValidator::new(report_dir, Some(config.clone())),
            config,
        }
    }

    pub fn freshness(&self) -> anyhow::Result<ManualEvidenceFreshnessResponse> {
        let startup = self.startup_store.run_check()?;
        let signoff_status = self.signoff_store.status()?;
        let latest_signoff = signoff_status.latest_signoff.clone();
        let current_fingerprint = signoff_status.current_evidence_fingerprint.clone();
        let latest_signoff_fingerprint = latest_signoff
            .as_ref()
            .map(|record| record.evidence_fingerprint.clone());

        let checks = self.build_checks(latest_signoff.as_ref(), startup.status)?;
        let changed_evidence = checks
            .iter()
            .filter(|check| check.changed_since_signoff)
            .map(|check| check.name.clone())
            .collect::<Vec<_>>();
        let missing_evidence = checks.iter().any(|check| !check.present);

        let age_ms = latest_signoff.as_ref().map(|record| {
            Local::now()
                .timestamp_millis()
                .saturating_sub(record.created_at_ms)
        });
        let expires_in_ms = age_ms.map(|age| DEFAULT_MANUAL_SIGNOFF_TTL_MS.saturating_sub(age));
        let expired = age_ms
            .map(|age| age > DEFAULT_MANUAL_SIGNOFF_TTL_MS)
            .unwrap_or(false);

        let (status, next_action) = match (latest_signoff.as_ref(), startup.status) {
            (None, _) => (
                ManualEvidenceFreshnessStatus::NoSignoff,
                "Operator must create a sign-off before manual apply.".to_string(),
            ),
            (_, _) if missing_evidence => (
                ManualEvidenceFreshnessStatus::MissingEvidence,
                "Restore missing evidence before manual apply review.".to_string(),
            ),
            (_, status) if status != ManualStartupStatus::ReadyForManualApply => (
                ManualEvidenceFreshnessStatus::ReadinessNotReady,
                "Refresh startup readiness before trusting the latest sign-off.".to_string(),
            ),
            (_, _) if expired => (
                ManualEvidenceFreshnessStatus::Expired,
                "Latest sign-off exceeded TTL. Re-run startup check and sign off again."
                    .to_string(),
            ),
            (_, _)
                if current_fingerprint
                    != latest_signoff_fingerprint.clone().unwrap_or_default()
                    || !changed_evidence.is_empty() =>
            {
                (
                    ManualEvidenceFreshnessStatus::Stale,
                    "Evidence changed after sign-off. Re-run startup check and sign off again."
                        .to_string(),
                )
            }
            _ => (
                ManualEvidenceFreshnessStatus::Fresh,
                "Evidence is fresh and still matches the latest operator sign-off.".to_string(),
            ),
        };

        Ok(ManualEvidenceFreshnessResponse {
            read_only: self.config.read_only,
            status,
            current_evidence_fingerprint: current_fingerprint,
            latest_signoff_fingerprint,
            ttl_ms: DEFAULT_MANUAL_SIGNOFF_TTL_MS,
            age_ms,
            expires_in_ms,
            changed_evidence,
            checks,
            next_action,
        })
    }

    fn build_checks(
        &self,
        latest_signoff: Option<&ManualSignoffRecord>,
        startup_status: ManualStartupStatus,
    ) -> anyhow::Result<Vec<EvidenceFreshnessCheck>> {
        let current_report = self.report_store.latest_report()?;
        let current_review = self.review_store.list_reviews()?.into_iter().next();
        let current_review_ts = review_ledger_timestamp(self.review_store.ledger_path());
        let current_export = self.export_store.latest_export()?;
        let current_diff = self.diff_store.latest_diff()?;
        let current_runbook = self.runbook_store.latest_runbook()?;
        let current_dry_run = self.dry_run_validator.latest_report()?;

        let signoff_report = latest_signoff
            .and_then(|record| record.calibration_report_id.as_deref())
            .map(|id| self.report_store.get_report(id))
            .transpose()?
            .flatten();
        let signoff_export = latest_signoff
            .and_then(|record| record.manual_patch_id.as_deref())
            .map(|id| self.export_store.get_export(id))
            .transpose()?
            .flatten();
        let signoff_diff = latest_signoff
            .and_then(|record| record.diff_id.as_deref())
            .map(|id| self.diff_store.diff_by_id(id))
            .transpose()?
            .flatten();
        let signoff_runbook = latest_signoff
            .and_then(|record| record.runbook_id.as_deref())
            .map(|id| self.runbook_store.runbook_by_id(id))
            .transpose()?
            .flatten();
        let signoff_dry_run = latest_signoff
            .and_then(|record| record.dryrun_id.as_deref())
            .map(|id| self.dry_run_validator.report_by_id(id))
            .transpose()?
            .flatten();

        Ok(vec![
            build_check(
                "calibration_report",
                current_report
                    .as_ref()
                    .map(|entry| serial_hash(&entry.summary))
                    .transpose()?,
                signoff_report
                    .as_ref()
                    .map(|entry| serial_hash(&entry.summary))
                    .transpose()?,
                current_report
                    .as_ref()
                    .and_then(|entry| entry.summary.created_at_ms),
                signoff_report
                    .as_ref()
                    .and_then(|entry| entry.summary.created_at_ms),
                "Calibration report",
            ),
            build_check(
                "recommendation_review",
                current_review.as_ref().map(|review| {
                    hash_text(&format!(
                        "{}::{}",
                        review.recommendation_id, review.updated_at
                    ))
                }),
                latest_signoff
                    .and_then(|record| record.recommendation_review_id.as_deref())
                    .map(hash_text),
                current_review_ts,
                latest_signoff.map(|record| {
                    review_timestamp_from_id(record.recommendation_review_id.as_deref())
                }),
                "Recommendation review",
            ),
            build_check(
                "manual_export_patch",
                current_export
                    .as_ref()
                    .map(|entry| serial_hash(&entry.export))
                    .transpose()?,
                signoff_export
                    .as_ref()
                    .map(|entry| serial_hash(&entry.export))
                    .transpose()?,
                current_export
                    .as_ref()
                    .and_then(|entry| entry.summary.created_at_ms),
                signoff_export
                    .as_ref()
                    .and_then(|entry| entry.summary.created_at_ms),
                "Manual patch",
            ),
            build_check(
                "parameter_diff",
                current_diff.as_ref().map(serial_hash).transpose()?,
                signoff_diff.as_ref().map(serial_hash).transpose()?,
                current_diff.as_ref().and_then(|diff| diff.generated_at_ms),
                signoff_diff.as_ref().and_then(|diff| diff.generated_at_ms),
                "Parameter diff",
            ),
            build_check(
                "manual_runbook",
                current_runbook.as_ref().map(serial_hash).transpose()?,
                signoff_runbook.as_ref().map(serial_hash).transpose()?,
                current_export
                    .as_ref()
                    .and_then(|entry| entry.summary.created_at_ms),
                signoff_export
                    .as_ref()
                    .and_then(|entry| entry.summary.created_at_ms),
                "Manual runbook",
            ),
            build_check(
                "dryrun_validator",
                current_dry_run.as_ref().map(serial_hash).transpose()?,
                signoff_dry_run.as_ref().map(serial_hash).transpose()?,
                current_export
                    .as_ref()
                    .and_then(|entry| entry.summary.created_at_ms),
                signoff_export
                    .as_ref()
                    .and_then(|entry| entry.summary.created_at_ms),
                "Dry-run",
            ),
            build_check(
                "startup_check",
                Some(hash_text(startup_status_label(startup_status))),
                latest_signoff.map(|record| hash_text(&record.readiness_status)),
                None,
                latest_signoff.map(|record| record.created_at_ms),
                "Startup check",
            ),
            build_check(
                "signoff_record",
                latest_signoff.map(|record| hash_text(&record.signoff_id)),
                latest_signoff.map(|record| hash_text(&record.signoff_id)),
                latest_signoff.map(|record| record.created_at_ms),
                latest_signoff.map(|record| record.created_at_ms),
                "Signoff record",
            ),
        ])
    }
}

fn build_check(
    name: &str,
    current_hash: Option<String>,
    signoff_hash: Option<String>,
    current_modified_at_ms: Option<i64>,
    signoff_modified_at_ms: Option<i64>,
    label: &str,
) -> EvidenceFreshnessCheck {
    let present = current_hash.is_some();
    let changed_since_signoff = match (&current_hash, &signoff_hash) {
        (Some(current), Some(previous)) => current != previous,
        _ => false,
    };
    let fresh = present && !changed_since_signoff;
    let message = match (&current_hash, &signoff_hash, changed_since_signoff) {
        (None, _, _) => format!("{label} evidence is missing."),
        (Some(_), None, _) => format!("{label} has no signoff baseline yet."),
        (Some(_), Some(_), true) => format!("{label} changed after the latest signoff."),
        (Some(_), Some(_), false) => format!("{label} matches latest signoff evidence."),
    };

    EvidenceFreshnessCheck {
        name: name.to_string(),
        present,
        fresh,
        changed_since_signoff,
        current_hash,
        signoff_hash,
        current_modified_at_ms,
        signoff_modified_at_ms,
        message,
    }
}

fn serial_hash<T: Serialize>(value: &T) -> anyhow::Result<String> {
    Ok(hash_text(&serde_json::to_string(value)?))
}

fn hash_text(value: &str) -> String {
    let mut hasher = DefaultHasher::new();
    value.hash(&mut hasher);
    format!("{:016x}", hasher.finish())
}

fn review_timestamp_from_id(value: Option<&str>) -> i64 {
    value
        .and_then(|raw| raw.rsplit_once("::"))
        .and_then(|(_, ts)| ts.parse::<i64>().ok())
        .unwrap_or_default()
}

fn startup_status_label(status: ManualStartupStatus) -> &'static str {
    match status {
        ManualStartupStatus::ReadyForManualApply => "READY_FOR_MANUAL_APPLY",
        ManualStartupStatus::Blocked => "BLOCKED",
        ManualStartupStatus::NeedsReview => "NEEDS_REVIEW",
        ManualStartupStatus::MissingReport => "MISSING_REPORT",
    }
}
