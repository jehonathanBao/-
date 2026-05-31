use std::path::PathBuf;

use chrono::Local;

use crate::{
    calibration::{
        manual_evidence_freshness::{
            ManualEvidenceFreshnessResponse, ManualEvidenceFreshnessStatus,
            ManualEvidenceFreshnessStore,
        },
        manual_signoff_store::{
            ManualSignoffGateStatus, ManualSignoffStatusResponse, ManualSignoffStore,
        },
        manual_startup_check::{
            ManualStartupCheckResponse, ManualStartupCheckStore, ManualStartupStatus,
        },
    },
    config::AppConfig,
};

#[derive(Debug, Clone, serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ManualAuditStoryResponse {
    pub read_only: bool,
    pub apply_mode: String,
    pub runtime_modified: bool,
    pub generated_at: String,
    pub final_gate: String,
    pub readiness_status: String,
    pub signoff_status: String,
    pub freshness_status: String,
    pub current_evidence_fingerprint: String,
    pub latest_signoff_fingerprint: Option<String>,
    pub changed_evidence: Vec<String>,
    pub current_blocker: Option<String>,
    pub blockers: Vec<String>,
    pub warnings: Vec<String>,
    pub next_owner: String,
    pub next_action: String,
    pub handoff_summary: String,
    pub remediation_checklist: Vec<String>,
    pub safety_boundary: Vec<String>,
    pub ttl_ms: Option<i64>,
    pub age_ms: Option<i64>,
    pub expires_in_ms: Option<i64>,
    pub timeline: Vec<ManualAuditTimelineItem>,
    pub markdown: String,
}

#[derive(Debug, Clone, serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ManualAuditTimelineItem {
    pub key: String,
    pub title: String,
    pub status: String,
    pub severity: String,
    pub summary: String,
    pub evidence_ref: Option<ManualAuditEvidenceRef>,
    pub missing_reason: Option<String>,
    pub blocking_reason: Option<String>,
    pub remediation_hint: Option<String>,
    pub observed_at: String,
}

#[derive(Debug, Clone, serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ManualAuditEvidenceRef {
    pub kind: String,
    pub label: String,
    pub source_endpoint: String,
    pub markdown_endpoint: Option<String>,
    pub export_id: Option<String>,
    pub report_id: Option<String>,
    pub observed_at: String,
    pub status: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum ManualAuditStoryReason {
    Ready,
    NoSignoff,
    Stale,
    Expired,
    MissingReport,
    DryRunFailed,
    ReadinessNotReady,
}

#[derive(Clone, Copy)]
struct ReasonProfile {
    current_blocker: Option<&'static str>,
    severity: &'static str,
    next_owner: &'static str,
    next_action: &'static str,
    handoff_summary: &'static str,
    remediation: &'static [&'static str],
    warning: Option<&'static str>,
}

pub struct ManualAuditStoryStore {
    config: AppConfig,
    startup_store: ManualStartupCheckStore,
    signoff_store: ManualSignoffStore,
    freshness_store: ManualEvidenceFreshnessStore,
}

impl ManualAuditStoryStore {
    pub fn new(report_dir: impl Into<PathBuf>, config: AppConfig) -> Self {
        let report_dir = report_dir.into();
        Self {
            startup_store: ManualStartupCheckStore::new(report_dir.clone(), config.clone()),
            signoff_store: ManualSignoffStore::new(report_dir.clone(), config.clone()),
            freshness_store: ManualEvidenceFreshnessStore::new(report_dir, config.clone()),
            config,
        }
    }

    pub fn build_story(&self) -> anyhow::Result<ManualAuditStoryResponse> {
        let readiness = self.startup_store.run_check()?;
        let signoff = self.signoff_store.status()?;
        let freshness = self.freshness_store.freshness()?;

        let generated_at = Local::now().to_rfc3339();
        let readiness_status = startup_status_label(readiness.status).to_string();
        let signoff_status = signoff_status_label(signoff.status).to_string();
        let freshness_status = freshness_status_label(freshness.status).to_string();
        let final_gate = if readiness.status == ManualStartupStatus::ReadyForManualApply
            && signoff.status == ManualSignoffGateStatus::SignedOff
            && freshness.status == ManualEvidenceFreshnessStatus::Fresh
        {
            "READY".to_string()
        } else {
            "BLOCKED".to_string()
        };

        let reason = derive_reason(&readiness, &signoff, &freshness);
        let profile = reason_profile(reason);
        let blockers = build_blockers(reason, &freshness.changed_evidence);
        let warnings = build_warnings(reason, &freshness.changed_evidence);
        let remediation_checklist = profile
            .remediation
            .iter()
            .map(|value| (*value).to_string())
            .collect::<Vec<_>>();
        let timeline = build_timeline(
            &generated_at,
            reason,
            &readiness,
            &signoff,
            &freshness,
            profile,
        );

        let mut response = ManualAuditStoryResponse {
            read_only: self.config.read_only,
            apply_mode: "read_only_audit_story".to_string(),
            runtime_modified: false,
            generated_at,
            final_gate,
            readiness_status,
            signoff_status,
            freshness_status,
            current_evidence_fingerprint: freshness.current_evidence_fingerprint.clone(),
            latest_signoff_fingerprint: freshness.latest_signoff_fingerprint.clone(),
            changed_evidence: freshness.changed_evidence.clone(),
            current_blocker: profile.current_blocker.map(str::to_string),
            blockers,
            warnings,
            next_owner: profile.next_owner.to_string(),
            next_action: profile.next_action.to_string(),
            handoff_summary: handoff_summary(reason, &freshness.changed_evidence),
            remediation_checklist,
            safety_boundary: safety_boundary(),
            ttl_ms: Some(freshness.ttl_ms),
            age_ms: freshness.age_ms,
            expires_in_ms: freshness.expires_in_ms,
            timeline,
            markdown: String::new(),
        };
        response.markdown = render_markdown(&response);
        Ok(response)
    }

    pub fn markdown(&self) -> anyhow::Result<String> {
        Ok(self.build_story()?.markdown)
    }
}

fn derive_reason(
    readiness: &ManualStartupCheckResponse,
    signoff: &ManualSignoffStatusResponse,
    freshness: &ManualEvidenceFreshnessResponse,
) -> ManualAuditStoryReason {
    if readiness.status == ManualStartupStatus::ReadyForManualApply
        && signoff.status == ManualSignoffGateStatus::SignedOff
        && freshness.status == ManualEvidenceFreshnessStatus::Fresh
    {
        return ManualAuditStoryReason::Ready;
    }
    if readiness.status == ManualStartupStatus::MissingReport {
        return ManualAuditStoryReason::MissingReport;
    }
    if readiness.status == ManualStartupStatus::Blocked
        && readiness
            .checks
            .iter()
            .any(|check| check.name == "dryrun_validator" && !check.ok)
    {
        return ManualAuditStoryReason::DryRunFailed;
    }
    if signoff.status == ManualSignoffGateStatus::NoSignoff
        || freshness.status == ManualEvidenceFreshnessStatus::NoSignoff
    {
        return ManualAuditStoryReason::NoSignoff;
    }
    if signoff.status == ManualSignoffGateStatus::SignoffStale
        || freshness.status == ManualEvidenceFreshnessStatus::Stale
    {
        return ManualAuditStoryReason::Stale;
    }
    if signoff.status == ManualSignoffGateStatus::SignoffExpired
        || freshness.status == ManualEvidenceFreshnessStatus::Expired
    {
        return ManualAuditStoryReason::Expired;
    }
    ManualAuditStoryReason::ReadinessNotReady
}

fn reason_profile(reason: ManualAuditStoryReason) -> ReasonProfile {
    match reason {
        ManualAuditStoryReason::Ready => ReasonProfile {
            current_blocker: None,
            severity: "ok",
            next_owner: "External Manual Executor",
            next_action: "Ready for external manual execution by runbook.",
            handoff_summary:
                "Manual gate is ready. Follow the manual runbook outside this system.",
            remediation: &["Ready for external manual execution by runbook."],
            warning: None,
        },
        ManualAuditStoryReason::NoSignoff => ReasonProfile {
            current_blocker: Some("NO_SIGNOFF"),
            severity: "warning",
            next_owner: "Reviewer / Approver",
            next_action: "Review evidence pack and sign off before external manual apply.",
            handoff_summary:
                "Evidence exists but has not been signed off. Review evidence pack first and approve or reject before any external manual apply.",
            remediation: &[
                "Review evidence pack.",
                "Approve or reject through the manual sign-off flow.",
                "Do not proceed without a valid sign-off.",
            ],
            warning: Some("Operator sign-off is missing."),
        },
        ManualAuditStoryReason::Stale => ReasonProfile {
            current_blocker: Some("STALE"),
            severity: "warning",
            next_owner: "Reviewer",
            next_action: "Review changedEvidence and re-sign.",
            handoff_summary:
                "Existing sign-off no longer matches current evidence. Review changedEvidence and re-sign.",
            remediation: &[
                "Refresh evidence freshness.",
                "Review changedEvidence.",
                "Create a new sign-off after confirming the updated evidence.",
            ],
            warning: Some(
                "Existing sign-off no longer matches current evidence and requires a new sign-off.",
            ),
        },
        ManualAuditStoryReason::Expired => ReasonProfile {
            current_blocker: Some("EXPIRED"),
            severity: "warning",
            next_owner: "Operator",
            next_action: "Refresh evidence, re-run Manual Startup Check, and sign off again.",
            handoff_summary:
                "Startup check or sign-off freshness window has expired. Refresh evidence, re-run Manual Startup Check, and sign off again.",
            remediation: &[
                "Refresh evidence.",
                "Re-run Manual Startup Check.",
                "Re-sign before external manual execution.",
            ],
            warning: Some("Latest sign-off expired and must be refreshed."),
        },
        ManualAuditStoryReason::MissingReport => ReasonProfile {
            current_blocker: Some("MISSING_REPORT"),
            severity: "error",
            next_owner: "Operator",
            next_action:
                "Restore or regenerate missing report / export / diff / runbook / dry-run evidence.",
            handoff_summary:
                "Required manual-apply evidence is incomplete. Restore the missing report and rebuild downstream evidence before any external manual apply.",
            remediation: &[
                "Run Manual Startup Check.",
                "Restore or regenerate missing report / export / diff / runbook / dry-run evidence.",
                "Re-check readiness after evidence is restored.",
            ],
            warning: None,
        },
        ManualAuditStoryReason::DryRunFailed => ReasonProfile {
            current_blocker: Some("DRY_RUN_FAILED"),
            severity: "error",
            next_owner: "Operator",
            next_action: "Fix dry-run validation failures before manual apply review.",
            handoff_summary:
                "Dry-run validation failed. Resolve blocking dry-run issues before any external manual apply.",
            remediation: &[
                "Run Manual Startup Check.",
                "Review failed dry-run checks.",
                "Fix blocking dry-run issues before continuing.",
            ],
            warning: None,
        },
        ManualAuditStoryReason::ReadinessNotReady => ReasonProfile {
            current_blocker: Some("READINESS_NOT_READY"),
            severity: "error",
            next_owner: "Operator",
            next_action:
                "Run Manual Startup Check and fix failed readiness items before sign-off.",
            handoff_summary:
                "Required manual-apply evidence is incomplete or readiness failed. Fix readiness items before any external manual apply.",
            remediation: &[
                "Run Manual Startup Check.",
                "Review failed readiness checks.",
                "Fix missing report / export / diff / runbook / dry-run evidence.",
            ],
            warning: None,
        },
    }
}

fn handoff_summary(reason: ManualAuditStoryReason, changed_evidence: &[String]) -> String {
    match reason {
        ManualAuditStoryReason::Stale if !changed_evidence.is_empty() => format!(
            "Existing sign-off no longer matches current evidence. Review changedEvidence ({}) and re-sign.",
            changed_evidence.join(", ")
        ),
        _ => reason_profile(reason).handoff_summary.to_string(),
    }
}

fn build_blockers(reason: ManualAuditStoryReason, changed_evidence: &[String]) -> Vec<String> {
    match reason {
        ManualAuditStoryReason::Ready => Vec::new(),
        ManualAuditStoryReason::Stale if !changed_evidence.is_empty() => vec![format!(
            "STALE: existing sign-off no longer matches current evidence. Changed evidence: {}.",
            changed_evidence.join(", ")
        )],
        _ => {
            let profile = reason_profile(reason);
            vec![format!(
                "{}: {}",
                profile.current_blocker.unwrap_or("BLOCKED"),
                profile.handoff_summary
            )]
        }
    }
}

fn build_warnings(reason: ManualAuditStoryReason, changed_evidence: &[String]) -> Vec<String> {
    let mut warnings = Vec::new();
    let profile = reason_profile(reason);
    if let Some(warning) = profile.warning {
        warnings.push(warning.to_string());
    }
    if reason == ManualAuditStoryReason::Stale && !changed_evidence.is_empty() {
        warnings.push(format!(
            "Changed evidence detected since sign-off: {}.",
            changed_evidence.join(", ")
        ));
    }
    warnings
}

#[allow(clippy::too_many_arguments)]
fn build_evidence_ref(
    kind: &str,
    label: impl Into<String>,
    source_endpoint: &str,
    markdown_endpoint: Option<&str>,
    export_id: Option<String>,
    report_id: Option<String>,
    observed_at: &str,
    status: &str,
) -> ManualAuditEvidenceRef {
    ManualAuditEvidenceRef {
        kind: kind.to_string(),
        label: label.into(),
        source_endpoint: source_endpoint.to_string(),
        markdown_endpoint: markdown_endpoint.map(str::to_string),
        export_id,
        report_id,
        observed_at: observed_at.to_string(),
        status: status.to_string(),
    }
}

fn build_timeline(
    observed_at: &str,
    reason: ManualAuditStoryReason,
    readiness: &ManualStartupCheckResponse,
    signoff: &ManualSignoffStatusResponse,
    freshness: &ManualEvidenceFreshnessResponse,
    profile: ReasonProfile,
) -> Vec<ManualAuditTimelineItem> {
    let latest_signoff = signoff.latest_signoff.as_ref();
    let export_id = latest_signoff.and_then(|record| record.manual_patch_id.clone());
    let report_id = latest_signoff.and_then(|record| record.calibration_report_id.clone());
    let review_id = latest_signoff.and_then(|record| record.recommendation_review_id.clone());
    let signoff_id = latest_signoff.map(|record| record.signoff_id.clone());
    let stale_reason = if freshness.changed_evidence.is_empty() {
        None
    } else {
        Some(format!(
            "Changed evidence: {}",
            freshness.changed_evidence.join(", ")
        ))
    };

    vec![
        ManualAuditTimelineItem {
            key: "review".to_string(),
            title: "Recommendation Review".to_string(),
            status: if matches!(reason, ManualAuditStoryReason::MissingReport) {
                "missing".to_string()
            } else {
                "passed".to_string()
            },
            severity: if matches!(reason, ManualAuditStoryReason::MissingReport) {
                "error".to_string()
            } else {
                "info".to_string()
            },
            summary: "Recommendation review evidence is available for this manual apply story."
                .to_string(),
            evidence_ref: Some(build_evidence_ref(
                "recommendation_review",
                review_id
                    .clone()
                    .unwrap_or_else(|| "latest recommendation review".to_string()),
                "/api/parameter-review/reviews",
                Some("/api/calibration/reports/latest"),
                export_id.clone(),
                report_id.clone(),
                observed_at,
                "available",
            )),
            missing_reason: if matches!(reason, ManualAuditStoryReason::MissingReport) {
                Some(
                    "Calibration report is missing, so recommendation review cannot be trusted."
                        .to_string(),
                )
            } else {
                None
            },
            blocking_reason: None,
            remediation_hint: None,
            observed_at: observed_at.to_string(),
        },
        ManualAuditTimelineItem {
            key: "export".to_string(),
            title: "Manual Parameter Export".to_string(),
            status: if matches!(reason, ManualAuditStoryReason::MissingReport) {
                "missing".to_string()
            } else {
                "passed".to_string()
            },
            severity: if matches!(reason, ManualAuditStoryReason::MissingReport) {
                "error".to_string()
            } else {
                "info".to_string()
            },
            summary: "Manual export artifact is available for read-only review.".to_string(),
            evidence_ref: Some(build_evidence_ref(
                "manual_export",
                export_id
                    .clone()
                    .unwrap_or_else(|| "latest manual export".to_string()),
                "/api/parameter-review/exports/latest",
                Some("/api/parameter-review/exports/latest"),
                export_id.clone(),
                report_id.clone(),
                observed_at,
                "available",
            )),
            missing_reason: if matches!(reason, ManualAuditStoryReason::MissingReport) {
                Some("Manual export cannot be trusted until the upstream calibration report is restored.".to_string())
            } else {
                None
            },
            blocking_reason: None,
            remediation_hint: None,
            observed_at: observed_at.to_string(),
        },
        ManualAuditTimelineItem {
            key: "diff".to_string(),
            title: "Patch Diff / Audit".to_string(),
            status: if matches!(reason, ManualAuditStoryReason::MissingReport) {
                "missing".to_string()
            } else {
                "passed".to_string()
            },
            severity: if matches!(reason, ManualAuditStoryReason::MissingReport) {
                "error".to_string()
            } else {
                "info".to_string()
            },
            summary: "Patch diff and audit evidence are ready for inspection.".to_string(),
            evidence_ref: Some(build_evidence_ref(
                "patch_diff_audit",
                export_id
                    .clone()
                    .unwrap_or_else(|| "latest patch diff".to_string()),
                "/api/parameter-review/exports/latest/diff",
                Some("/api/parameter-review/exports/latest/audit"),
                export_id.clone(),
                report_id.clone(),
                observed_at,
                "available",
            )),
            missing_reason: if matches!(reason, ManualAuditStoryReason::MissingReport) {
                Some(
                    "Diff and audit are incomplete because upstream evidence is missing."
                        .to_string(),
                )
            } else {
                None
            },
            blocking_reason: None,
            remediation_hint: None,
            observed_at: observed_at.to_string(),
        },
        ManualAuditTimelineItem {
            key: "runbook".to_string(),
            title: "Manual Apply Runbook".to_string(),
            status: if matches!(reason, ManualAuditStoryReason::MissingReport) {
                "missing".to_string()
            } else {
                "passed".to_string()
            },
            severity: if matches!(reason, ManualAuditStoryReason::MissingReport) {
                "error".to_string()
            } else {
                "info".to_string()
            },
            summary: "Manual runbook exists for external execution guidance.".to_string(),
            evidence_ref: Some(build_evidence_ref(
                "manual_runbook",
                export_id
                    .clone()
                    .unwrap_or_else(|| "latest runbook".to_string()),
                "/api/parameter-review/exports/latest/runbook",
                Some("/api/parameter-review/exports/latest/runbook.md"),
                export_id.clone(),
                report_id.clone(),
                observed_at,
                "available",
            )),
            missing_reason: if matches!(reason, ManualAuditStoryReason::MissingReport) {
                Some(
                    "Runbook cannot be relied on until the missing report is restored.".to_string(),
                )
            } else {
                None
            },
            blocking_reason: None,
            remediation_hint: None,
            observed_at: observed_at.to_string(),
        },
        ManualAuditTimelineItem {
            key: "dryrun".to_string(),
            title: "Dry-run Validation".to_string(),
            status: match reason {
                ManualAuditStoryReason::DryRunFailed => "blocked".to_string(),
                ManualAuditStoryReason::ReadinessNotReady => "warning".to_string(),
                ManualAuditStoryReason::MissingReport => "missing".to_string(),
                _ => "passed".to_string(),
            },
            severity: match reason {
                ManualAuditStoryReason::DryRunFailed => "error".to_string(),
                ManualAuditStoryReason::ReadinessNotReady => "warning".to_string(),
                ManualAuditStoryReason::MissingReport => "error".to_string(),
                _ => "info".to_string(),
            },
            summary: if matches!(reason, ManualAuditStoryReason::DryRunFailed) {
                "Dry-run validation failed and is blocking manual apply.".to_string()
            } else {
                "Dry-run evidence is available for review.".to_string()
            },
            evidence_ref: Some(build_evidence_ref(
                "dry_run",
                export_id
                    .clone()
                    .unwrap_or_else(|| "latest dry-run".to_string()),
                "/api/parameter-review/exports/latest/dry-run",
                Some("/api/parameter-review/exports/latest/dry-run.md"),
                export_id.clone(),
                report_id.clone(),
                observed_at,
                if matches!(reason, ManualAuditStoryReason::DryRunFailed) {
                    "failed"
                } else {
                    "available"
                },
            )),
            missing_reason: if matches!(reason, ManualAuditStoryReason::MissingReport) {
                Some("Dry-run evidence is incomplete because upstream report/export evidence is missing.".to_string())
            } else {
                None
            },
            blocking_reason: if matches!(reason, ManualAuditStoryReason::DryRunFailed) {
                Some("Dry-run validation reported blocking issues.".to_string())
            } else {
                None
            },
            remediation_hint: if matches!(reason, ManualAuditStoryReason::DryRunFailed) {
                Some("Fix dry-run validation failures before continuing.".to_string())
            } else {
                None
            },
            observed_at: observed_at.to_string(),
        },
        ManualAuditTimelineItem {
            key: "evidence_pack".to_string(),
            title: "Evidence Pack".to_string(),
            status: match reason {
                ManualAuditStoryReason::Stale => "stale".to_string(),
                ManualAuditStoryReason::Expired => "expired".to_string(),
                ManualAuditStoryReason::MissingReport => "missing".to_string(),
                _ => "passed".to_string(),
            },
            severity: match reason {
                ManualAuditStoryReason::Stale | ManualAuditStoryReason::Expired => {
                    "warning".to_string()
                }
                ManualAuditStoryReason::MissingReport => "error".to_string(),
                _ => "info".to_string(),
            },
            summary: if freshness.changed_evidence.is_empty() {
                "Evidence pack is available.".to_string()
            } else {
                format!(
                    "Changed evidence detected: {}.",
                    freshness.changed_evidence.join(", ")
                )
            },
            evidence_ref: Some(build_evidence_ref(
                "evidence_pack",
                export_id
                    .clone()
                    .unwrap_or_else(|| "latest evidence pack".to_string()),
                "/api/parameter-review/exports/latest/evidence-pack",
                Some("/api/parameter-review/exports/latest/evidence-pack.md"),
                export_id.clone(),
                report_id.clone(),
                observed_at,
                if matches!(reason, ManualAuditStoryReason::Stale) {
                    "stale"
                } else if matches!(reason, ManualAuditStoryReason::Expired) {
                    "expired"
                } else {
                    "available"
                },
            )),
            missing_reason: if matches!(reason, ManualAuditStoryReason::MissingReport) {
                Some("Evidence pack is incomplete because upstream report/export evidence is missing.".to_string())
            } else {
                None
            },
            blocking_reason: stale_reason.clone(),
            remediation_hint: if matches!(reason, ManualAuditStoryReason::Stale) {
                Some("Review changedEvidence and re-sign.".to_string())
            } else if matches!(reason, ManualAuditStoryReason::Expired) {
                Some("Refresh evidence and sign off again.".to_string())
            } else {
                None
            },
            observed_at: observed_at.to_string(),
        },
        ManualAuditTimelineItem {
            key: "signoff".to_string(),
            title: "Operator Sign-off".to_string(),
            status: match reason {
                ManualAuditStoryReason::Ready => "passed".to_string(),
                ManualAuditStoryReason::NoSignoff => "blocked".to_string(),
                ManualAuditStoryReason::Stale => "stale".to_string(),
                ManualAuditStoryReason::Expired => "expired".to_string(),
                _ => {
                    if signoff.status == ManualSignoffGateStatus::Rejected {
                        "blocked".to_string()
                    } else {
                        "warning".to_string()
                    }
                }
            },
            severity: match reason {
                ManualAuditStoryReason::Ready => "info".to_string(),
                ManualAuditStoryReason::NoSignoff
                | ManualAuditStoryReason::Stale
                | ManualAuditStoryReason::Expired => "warning".to_string(),
                _ => "error".to_string(),
            },
            summary: format!(
                "Current sign-off status is {}.",
                signoff_status_label(signoff.status)
            ),
            evidence_ref: Some(build_evidence_ref(
                "operator_signoff",
                signoff_id
                    .clone()
                    .unwrap_or_else(|| "latest sign-off".to_string()),
                "/api/calibration/manual-signoff/status",
                None,
                export_id.clone(),
                report_id.clone(),
                observed_at,
                signoff_status_label(signoff.status),
            )),
            missing_reason: if matches!(reason, ManualAuditStoryReason::NoSignoff) {
                Some("No operator sign-off exists for the current evidence.".to_string())
            } else {
                None
            },
            blocking_reason: if matches!(
                reason,
                ManualAuditStoryReason::NoSignoff
                    | ManualAuditStoryReason::Stale
                    | ManualAuditStoryReason::Expired
            ) {
                Some(profile.next_action.to_string())
            } else {
                None
            },
            remediation_hint: if matches!(
                reason,
                ManualAuditStoryReason::NoSignoff
                    | ManualAuditStoryReason::Stale
                    | ManualAuditStoryReason::Expired
            ) {
                Some(profile.next_action.to_string())
            } else {
                None
            },
            observed_at: observed_at.to_string(),
        },
        ManualAuditTimelineItem {
            key: "startup_check".to_string(),
            title: "Manual Startup Check".to_string(),
            status: match readiness.status {
                ManualStartupStatus::ReadyForManualApply => "passed".to_string(),
                ManualStartupStatus::NeedsReview => "warning".to_string(),
                ManualStartupStatus::MissingReport => "missing".to_string(),
                ManualStartupStatus::Blocked => "blocked".to_string(),
            },
            severity: match readiness.status {
                ManualStartupStatus::ReadyForManualApply => "info".to_string(),
                ManualStartupStatus::NeedsReview => "warning".to_string(),
                ManualStartupStatus::MissingReport | ManualStartupStatus::Blocked => {
                    "error".to_string()
                }
            },
            summary: format!(
                "Startup readiness is {}.",
                startup_status_label(readiness.status)
            ),
            evidence_ref: Some(build_evidence_ref(
                "startup_check",
                "latest startup readiness",
                "/api/calibration/manual-startup/check",
                None,
                export_id.clone(),
                report_id.clone(),
                observed_at,
                startup_status_label(readiness.status),
            )),
            missing_reason: if readiness.status == ManualStartupStatus::MissingReport {
                Some(
                    "Startup readiness is missing the calibration report required to continue."
                        .to_string(),
                )
            } else {
                None
            },
            blocking_reason: if readiness.status != ManualStartupStatus::ReadyForManualApply {
                Some(readiness.next_action.clone())
            } else {
                None
            },
            remediation_hint: if readiness.status != ManualStartupStatus::ReadyForManualApply {
                Some(readiness.next_action.clone())
            } else {
                None
            },
            observed_at: observed_at.to_string(),
        },
        ManualAuditTimelineItem {
            key: "manual_gate".to_string(),
            title: "Manual Apply Gate".to_string(),
            status: if reason == ManualAuditStoryReason::Ready {
                "ready".to_string()
            } else {
                "blocked".to_string()
            },
            severity: profile.severity.to_string(),
            summary: if reason == ManualAuditStoryReason::Ready {
                "Ready for external manual execution by runbook.".to_string()
            } else {
                format!(
                    "Current blocker is {}.",
                    profile.current_blocker.unwrap_or("BLOCKED")
                )
            },
            evidence_ref: Some(build_evidence_ref(
                "manual_gate",
                "manual gate aggregate",
                "/api/calibration/manual-audit-story",
                Some("/api/calibration/manual-audit-story.md"),
                export_id.clone(),
                report_id.clone(),
                observed_at,
                if reason == ManualAuditStoryReason::Ready {
                    "ready"
                } else {
                    "blocked"
                },
            )),
            missing_reason: None,
            blocking_reason: profile.current_blocker.map(str::to_string),
            remediation_hint: Some(profile.next_action.to_string()),
            observed_at: observed_at.to_string(),
        },
        ManualAuditTimelineItem {
            key: "remediation".to_string(),
            title: "Remediation Checklist".to_string(),
            status: if reason == ManualAuditStoryReason::Ready {
                "ready".to_string()
            } else {
                "blocked".to_string()
            },
            severity: profile.severity.to_string(),
            summary: if reason == ManualAuditStoryReason::Ready {
                "No remediation required.".to_string()
            } else {
                profile
                    .remediation
                    .first()
                    .copied()
                    .unwrap_or("Review remediation steps before proceeding.")
                    .to_string()
            },
            evidence_ref: Some(build_evidence_ref(
                "remediation_checklist",
                "manual gate remediation",
                "/api/calibration/manual-audit-story",
                Some("/api/calibration/manual-audit-story.md"),
                export_id.clone(),
                report_id.clone(),
                observed_at,
                if reason == ManualAuditStoryReason::Ready {
                    "ready"
                } else {
                    "blocked"
                },
            )),
            missing_reason: None,
            blocking_reason: profile.current_blocker.map(str::to_string),
            remediation_hint: Some(profile.next_action.to_string()),
            observed_at: observed_at.to_string(),
        },
        ManualAuditTimelineItem {
            key: "handoff".to_string(),
            title: "Operator Handoff Note".to_string(),
            status: if reason == ManualAuditStoryReason::Ready {
                "ready".to_string()
            } else {
                "blocked".to_string()
            },
            severity: profile.severity.to_string(),
            summary: handoff_summary(reason, &freshness.changed_evidence),
            evidence_ref: Some(build_evidence_ref(
                "operator_handoff_note",
                "operator handoff note",
                "/api/calibration/manual-audit-story",
                Some("/api/calibration/manual-audit-story.md"),
                export_id,
                report_id,
                observed_at,
                if reason == ManualAuditStoryReason::Ready {
                    "ready"
                } else {
                    "blocked"
                },
            )),
            missing_reason: None,
            blocking_reason: profile.current_blocker.map(str::to_string),
            remediation_hint: Some(profile.next_action.to_string()),
            observed_at: observed_at.to_string(),
        },
    ]
}

fn safety_boundary() -> Vec<String> {
    vec![
        "This audit story is read-only.".to_string(),
        "readOnly = true".to_string(),
        "runtime_modified = false".to_string(),
        "No runtime config was changed.".to_string(),
        "No runtime reload was triggered.".to_string(),
        "No calibration runner was triggered.".to_string(),
        "No apply/update-config/reload endpoint was called.".to_string(),
        "Flow / liquidation / toxic realtime pipelines were not modified.".to_string(),
        "No private key was used.".to_string(),
        "No signing was performed.".to_string(),
        "No swap / approve / mint / transfer action was performed.".to_string(),
        "No transaction construction was performed.".to_string(),
        "No live execution path was touched.".to_string(),
    ]
}

fn render_evidence_reference(item: &ManualAuditTimelineItem) -> String {
    if let Some(evidence_ref) = item.evidence_ref.as_ref() {
        format!(
            "{} ({}) | source={}{}",
            evidence_ref.label,
            evidence_ref.kind,
            evidence_ref.source_endpoint,
            evidence_ref
                .markdown_endpoint
                .as_ref()
                .map(|value| format!(" | markdown={value}"))
                .unwrap_or_default()
        )
    } else {
        item.missing_reason
            .clone()
            .unwrap_or_else(|| "Unavailable".to_string())
    }
}

fn render_markdown(story: &ManualAuditStoryResponse) -> String {
    let blockers = if story.blockers.is_empty() {
        "- None".to_string()
    } else {
        story
            .blockers
            .iter()
            .map(|item| format!("- {item}"))
            .collect::<Vec<_>>()
            .join("\n")
    };
    let warnings = if story.warnings.is_empty() {
        "- None".to_string()
    } else {
        story
            .warnings
            .iter()
            .map(|item| format!("- {item}"))
            .collect::<Vec<_>>()
            .join("\n")
    };
    let changed_evidence = if story.changed_evidence.is_empty() {
        "- None".to_string()
    } else {
        story
            .changed_evidence
            .iter()
            .map(|item| format!("- {item}"))
            .collect::<Vec<_>>()
            .join("\n")
    };
    let remediation = story
        .remediation_checklist
        .iter()
        .map(|item| format!("- {item}"))
        .collect::<Vec<_>>()
        .join("\n");
    let evidence_refs = story
        .timeline
        .iter()
        .map(|item| {
            let mut parts = vec![format!(
                "- {}: {}",
                item.title,
                render_evidence_reference(item)
            )];
            if let Some(reason) = item.missing_reason.as_ref() {
                parts.push(format!("  Missing reason: {reason}"));
            }
            if let Some(reason) = item.blocking_reason.as_ref() {
                parts.push(format!("  Blocking reason: {reason}"));
            }
            if let Some(hint) = item.remediation_hint.as_ref() {
                parts.push(format!("  Remediation: {hint}"));
            }
            parts.join("\n")
        })
        .collect::<Vec<_>>()
        .join("\n");
    let timeline = story
        .timeline
        .iter()
        .enumerate()
        .map(|(index, item)| {
            format!(
                "### {}. {}\n\n- Status: {}\n- Severity: {}\n- Summary: {}\n- Evidence: {}\n- Source: {}\n- Markdown: {}\n- Blocking reason: {}\n- Remediation: {}\n- Observed at: {}",
                index + 1,
                item.title,
                item.status,
                item.severity,
                item.summary,
                item
                    .evidence_ref
                    .as_ref()
                    .map(|reference| format!("{} ({})", reference.label, reference.kind))
                    .unwrap_or_else(|| "Unavailable".to_string()),
                item
                    .evidence_ref
                    .as_ref()
                    .map(|reference| reference.source_endpoint.clone())
                    .unwrap_or_else(|| "Unavailable".to_string()),
                item
                    .evidence_ref
                    .as_ref()
                    .and_then(|reference| reference.markdown_endpoint.clone())
                    .unwrap_or_else(|| "Unavailable".to_string()),
                item
                    .blocking_reason
                    .clone()
                    .or_else(|| item.missing_reason.clone())
                    .unwrap_or_else(|| "None".to_string()),
                item
                    .remediation_hint
                    .clone()
                    .unwrap_or_else(|| "None".to_string()),
                item.observed_at
            )
        })
        .collect::<Vec<_>>()
        .join("\n\n");
    let safety = story
        .safety_boundary
        .iter()
        .map(|item| format!("- {item}"))
        .collect::<Vec<_>>()
        .join("\n");
    let ops_log_snippet = format!(
        "Manual apply audit story generated.\n\n- Export ID: {}\n- Overall Status: {}\n- Current Blocker: {}\n- Next Owner: {}\n- Next Action: {}\n- Runtime Modified: {}\n- Apply Mode: {}\n- Safety: no apply / reload / runtime config mutation performed",
        story
            .timeline
            .iter()
            .find_map(|item| item.evidence_ref.as_ref().and_then(|reference| reference.export_id.clone()))
            .unwrap_or_else(|| "Unavailable".to_string()),
        story.final_gate,
        story
            .current_blocker
            .clone()
            .unwrap_or_else(|| "None".to_string()),
        story.next_owner,
        story.next_action,
        story.runtime_modified,
        story.apply_mode
    );

    format!(
        "# Manual Apply Audit Story\n\n\
> Handoff status: {}\n\
> Current blocker: {}\n\
> Next owner: {}\n\
> Next action: {}\n\
> Apply mode: {}\n\
> Runtime modified: {}\n\n\
## Manual Gate Summary\n\
- Final Gate: {}\n\
- Current Blocker: {}\n\
- Next Owner: {}\n\
- Next Action: {}\n\
- Runtime Modified: {}\n\
- Apply Mode: {}\n\n\
## Startup Readiness\n\
- Status: {}\n\n\
## Operator Sign-off\n\
- Status: {}\n\
- Latest Signoff Fingerprint: {}\n\n\
## Evidence Freshness / TTL\n\
- Status: {}\n\
- Current Evidence Fingerprint: {}\n\
- TTL ms: {}\n\
- Age ms: {}\n\
- Expires In ms: {}\n\n\
## Changed Evidence\n\
{}\n\n\
## Timeline\n\
{}\n\n\
## Blockers\n\
{}\n\n\
## Warnings\n\
{}\n\n\
## Evidence References\n\
{}\n\n\
## Remediation Checklist\n\
{}\n\n\
## Operator Handoff Note\n\
- Summary: {}\n\
- Next Owner: {}\n\
- Next Action: {}\n\n\
## Safety Boundary\n\
{}\n\n\
## Generated At\n\
- {}\n\n\
## Ops Log Snippet\n\
{}\n",
        story.final_gate,
        story
            .current_blocker
            .clone()
            .unwrap_or_else(|| "None".to_string()),
        story.next_owner,
        story.next_action,
        story.apply_mode,
        story.runtime_modified,
        story.final_gate,
        story
            .current_blocker
            .clone()
            .unwrap_or_else(|| "None".to_string()),
        story.next_owner,
        story.next_action,
        story.runtime_modified,
        story.apply_mode,
        story.readiness_status,
        story.signoff_status,
        story
            .latest_signoff_fingerprint
            .clone()
            .unwrap_or_else(|| "Unavailable".to_string()),
        story.freshness_status,
        story.current_evidence_fingerprint,
        story
            .ttl_ms
            .map(|value| value.to_string())
            .unwrap_or_else(|| "Unavailable".to_string()),
        story
            .age_ms
            .map(|value| value.to_string())
            .unwrap_or_else(|| "Unavailable".to_string()),
        story
            .expires_in_ms
            .map(|value| value.to_string())
            .unwrap_or_else(|| "Unavailable".to_string()),
        changed_evidence,
        timeline,
        blockers,
        warnings,
        evidence_refs,
        remediation,
        story.handoff_summary,
        story.next_owner,
        story.next_action,
        safety,
        story.generated_at,
        ops_log_snippet
    )
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
