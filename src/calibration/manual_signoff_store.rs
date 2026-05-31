use std::{
    collections::hash_map::DefaultHasher,
    fs::{self, OpenOptions},
    hash::{Hash, Hasher},
    io::Write,
    path::{Path, PathBuf},
};

use anyhow::Context;
use chrono::Local;

use crate::{
    calibration::{
        calibration_report_store::CalibrationReportStore,
        manual_apply_evidence_pack::ManualApplyEvidencePackStore,
        manual_parameter_export::ManualParameterExportStore,
        manual_startup_check::{ManualStartupCheckStore, ManualStartupStatus},
        parameter_recommendation_review_store::{
            review_ledger_timestamp, ParameterRecommendationReviewStore,
        },
    },
    config::AppConfig,
};

pub const DEFAULT_MANUAL_SIGNOFF_TTL_MS: i64 = 30 * 60 * 1000;

#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub enum ManualSignoffDecision {
    Approved,
    Rejected,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum ManualSignoffGateStatus {
    NoSignoff,
    SignedOff,
    Rejected,
    SignoffStale,
    SignoffExpired,
    ReadinessNotReady,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ManualSignoffRecord {
    pub signoff_id: String,
    pub created_at_ms: i64,
    pub operator: String,
    pub decision: ManualSignoffDecision,
    pub note: Option<String>,
    pub readiness_status: String,
    pub evidence_fingerprint: String,
    pub calibration_report_id: Option<String>,
    pub recommendation_review_id: Option<String>,
    pub manual_patch_id: Option<String>,
    pub diff_id: Option<String>,
    pub runbook_id: Option<String>,
    pub dryrun_id: Option<String>,
    pub read_only: bool,
}

#[derive(Debug, Clone, serde::Deserialize)]
pub struct ManualSignoffInput {
    pub operator: String,
    pub decision: ManualSignoffDecision,
    pub note: Option<String>,
}

#[derive(Debug, Clone, serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ManualSignoffStatusResponse {
    pub read_only: bool,
    pub status: ManualSignoffGateStatus,
    pub latest_readiness_status: String,
    pub current_evidence_fingerprint: String,
    pub latest_signoff: Option<ManualSignoffRecord>,
    pub signoff_allowed: bool,
    pub next_action: String,
}

#[derive(Debug, Clone, serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ManualSignoffCreateResponse {
    pub ok: bool,
    pub read_only: bool,
    pub signoff_id: String,
    pub status: ManualSignoffGateStatus,
    pub evidence_fingerprint: String,
    pub next_action: String,
}

pub struct ManualSignoffStore {
    config: AppConfig,
    startup_store: ManualStartupCheckStore,
    evidence_pack_store: ManualApplyEvidencePackStore,
    report_store: CalibrationReportStore,
    review_store: ParameterRecommendationReviewStore,
    export_store: ManualParameterExportStore,
    ledger_path: PathBuf,
}

impl ManualSignoffStore {
    pub fn new(report_dir: impl Into<PathBuf>, config: AppConfig) -> Self {
        let report_dir = report_dir.into();
        let runtime_dir = report_dir
            .parent()
            .map(Path::to_path_buf)
            .unwrap_or_else(|| PathBuf::from(".runtime"));
        let ledger_path = runtime_dir.join("reviews").join("manual-signoffs.jsonl");
        Self {
            startup_store: ManualStartupCheckStore::new(report_dir.clone(), config.clone()),
            evidence_pack_store: ManualApplyEvidencePackStore::new(
                report_dir.clone(),
                Some(config.clone()),
            ),
            report_store: CalibrationReportStore::new(report_dir.clone()),
            review_store: ParameterRecommendationReviewStore::new(report_dir.clone()),
            export_store: ManualParameterExportStore::new(report_dir),
            ledger_path,
            config,
        }
    }

    pub fn status(&self) -> anyhow::Result<ManualSignoffStatusResponse> {
        let readiness = self.startup_store.run_check()?;
        let latest_readiness_status = startup_status_label(readiness.status).to_string();
        let current_evidence_fingerprint =
            self.current_evidence_fingerprint(&latest_readiness_status)?;
        let latest_signoff = self.latest_record()?;
        let signoff_allowed = readiness.status == ManualStartupStatus::ReadyForManualApply;

        let (status, next_action) = match latest_signoff.as_ref() {
            None => {
                if signoff_allowed {
                    (
                        ManualSignoffGateStatus::NoSignoff,
                        "Operator must review evidence and sign off manually.".to_string(),
                    )
                } else {
                    (
                        ManualSignoffGateStatus::ReadinessNotReady,
                        "Resolve readiness issues before operator sign-off.".to_string(),
                    )
                }
            }
            Some(record) if record.evidence_fingerprint != current_evidence_fingerprint => (
                ManualSignoffGateStatus::SignoffStale,
                "Evidence changed after the latest sign-off. Review again before manual apply."
                    .to_string(),
            ),
            Some(record) if self.is_expired(record.created_at_ms) => (
                ManualSignoffGateStatus::SignoffExpired,
                "Latest sign-off expired. Refresh evidence and sign off again.".to_string(),
            ),
            Some(record)
                if record.decision == ManualSignoffDecision::Rejected
                    && signoff_allowed =>
            (
                ManualSignoffGateStatus::Rejected,
                "Latest operator decision rejected manual apply. Review evidence before retrying."
                    .to_string(),
            ),
            Some(record)
                if record.decision == ManualSignoffDecision::Rejected
                    && !signoff_allowed =>
            (
                ManualSignoffGateStatus::ReadinessNotReady,
                "Readiness is no longer ready and the latest sign-off was rejected.".to_string(),
            ),
            Some(_record) if !signoff_allowed => (
                ManualSignoffGateStatus::ReadinessNotReady,
                "Readiness is no longer ready. Do not proceed with manual apply.".to_string(),
            ),
            Some(_record) => (
                ManualSignoffGateStatus::SignedOff,
                "Manual apply may be performed outside this system according to the runbook."
                    .to_string(),
            ),
        };

        Ok(ManualSignoffStatusResponse {
            read_only: self.config.read_only,
            status,
            latest_readiness_status,
            current_evidence_fingerprint,
            latest_signoff,
            signoff_allowed,
            next_action,
        })
    }

    pub fn create_signoff(
        &self,
        input: ManualSignoffInput,
    ) -> anyhow::Result<ManualSignoffCreateResponse> {
        self.create_signoff_at(input, Local::now().timestamp_millis())
    }

    pub fn create_signoff_at(
        &self,
        input: ManualSignoffInput,
        now_ms: i64,
    ) -> anyhow::Result<ManualSignoffCreateResponse> {
        let readiness = self.startup_store.run_check()?;
        if input.decision == ManualSignoffDecision::Approved
            && readiness.status != ManualStartupStatus::ReadyForManualApply
        {
            anyhow::bail!("readiness_not_ready");
        }

        let latest_readiness_status = startup_status_label(readiness.status).to_string();
        let fingerprint = self.current_evidence_fingerprint(&latest_readiness_status)?;
        let context = self.current_context()?;
        let signoff_id = format!("manual-signoff-{}", Local::now().format("%Y%m%d-%H%M%S"));

        let record = ManualSignoffRecord {
            signoff_id: signoff_id.clone(),
            created_at_ms: now_ms,
            operator: input.operator,
            decision: input.decision,
            note: input.note,
            readiness_status: latest_readiness_status,
            evidence_fingerprint: fingerprint.clone(),
            calibration_report_id: context.calibration_report_id,
            recommendation_review_id: context.recommendation_review_id,
            manual_patch_id: context.manual_patch_id.clone(),
            diff_id: context.manual_patch_id.clone(),
            runbook_id: context.manual_patch_id.clone(),
            dryrun_id: context.manual_patch_id,
            read_only: self.config.read_only,
        };
        self.append_record(&record)?;

        let (status, next_action) = match record.decision {
            ManualSignoffDecision::Approved => (
                ManualSignoffGateStatus::SignedOff,
                "Manual apply may be performed outside this system according to runbook."
                    .to_string(),
            ),
            ManualSignoffDecision::Rejected => (
                ManualSignoffGateStatus::Rejected,
                "Manual apply remains blocked until a later operator sign-off approves it."
                    .to_string(),
            ),
        };

        Ok(ManualSignoffCreateResponse {
            ok: true,
            read_only: self.config.read_only,
            signoff_id,
            status,
            evidence_fingerprint: fingerprint,
            next_action,
        })
    }

    pub fn history(&self) -> anyhow::Result<Vec<ManualSignoffRecord>> {
        let mut records = self.load_records()?;
        records.sort_by_key(|record| std::cmp::Reverse(record.created_at_ms));
        Ok(records)
    }

    pub fn ledger_path(&self) -> &Path {
        &self.ledger_path
    }

    fn latest_record(&self) -> anyhow::Result<Option<ManualSignoffRecord>> {
        Ok(self.history()?.into_iter().next())
    }

    fn load_records(&self) -> anyhow::Result<Vec<ManualSignoffRecord>> {
        if !self.ledger_path.exists() {
            return Ok(Vec::new());
        }
        let content = fs::read_to_string(&self.ledger_path)
            .with_context(|| format!("read {}", self.ledger_path.display()))?;
        let mut records = Vec::new();
        for (index, line) in content.lines().enumerate() {
            if line.trim().is_empty() {
                continue;
            }
            let record = serde_json::from_str::<ManualSignoffRecord>(line).with_context(|| {
                format!("parse {} line {}", self.ledger_path.display(), index + 1)
            })?;
            records.push(record);
        }
        Ok(records)
    }

    fn append_record(&self, record: &ManualSignoffRecord) -> anyhow::Result<()> {
        if let Some(parent) = self.ledger_path.parent() {
            fs::create_dir_all(parent).with_context(|| format!("create {}", parent.display()))?;
        }
        let mut file = OpenOptions::new()
            .create(true)
            .append(true)
            .open(&self.ledger_path)
            .with_context(|| format!("open {}", self.ledger_path.display()))?;
        writeln!(file, "{}", serde_json::to_string(record)?)
            .with_context(|| format!("write {}", self.ledger_path.display()))?;
        Ok(())
    }

    fn current_evidence_fingerprint(&self, readiness_status: &str) -> anyhow::Result<String> {
        let payload = serde_json::json!({
            "readOnly": self.config.read_only,
            "readinessStatus": readiness_status,
            "latestReport": self.report_store.latest_report()?.map(|entry| entry.summary),
            "latestReviewTimestamp": review_ledger_timestamp(self.review_store.ledger_path()),
            "latestReview": self.review_store.list_reviews()?.into_iter().next(),
            "latestExport": self.export_store.latest_export()?.map(|entry| entry.summary),
            "latestEvidencePack": self.evidence_pack_store.latest_pack()?,
        });
        let text = serde_json::to_string(&payload)?;
        let mut hasher = DefaultHasher::new();
        text.hash(&mut hasher);
        Ok(format!("{:016x}", hasher.finish()))
    }

    fn current_context(&self) -> anyhow::Result<SignoffContext> {
        let latest_report = self.report_store.latest_report()?;
        let latest_review = self.review_store.list_reviews()?.into_iter().next();
        let latest_export = self.export_store.latest_export()?;
        Ok(SignoffContext {
            calibration_report_id: latest_report.map(|entry| entry.summary.id),
            recommendation_review_id: latest_review
                .map(|review| format!("{}::{}", review.recommendation_id, review.updated_at)),
            manual_patch_id: latest_export.map(|entry| entry.summary.export_id),
        })
    }

    fn is_expired(&self, created_at_ms: i64) -> bool {
        Local::now()
            .timestamp_millis()
            .saturating_sub(created_at_ms)
            > DEFAULT_MANUAL_SIGNOFF_TTL_MS
    }
}

struct SignoffContext {
    calibration_report_id: Option<String>,
    recommendation_review_id: Option<String>,
    manual_patch_id: Option<String>,
}

fn startup_status_label(status: ManualStartupStatus) -> &'static str {
    match status {
        ManualStartupStatus::ReadyForManualApply => "READY_FOR_MANUAL_APPLY",
        ManualStartupStatus::Blocked => "BLOCKED",
        ManualStartupStatus::NeedsReview => "NEEDS_REVIEW",
        ManualStartupStatus::MissingReport => "MISSING_REPORT",
    }
}
