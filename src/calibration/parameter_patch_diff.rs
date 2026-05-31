use std::{collections::BTreeMap, path::PathBuf};

use crate::{
    calibration::manual_parameter_export::{
        ManualParameterExportEntry, ManualParameterExportStore, ManualParameterPatchItem,
    },
    config::AppConfig,
};

#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ParameterPatchChangeType {
    Unchanged,
    Changed,
    MissingInCurrentConfig,
    MissingInPatch,
    TypeMismatch,
    UnsafeOrUnknownField,
}

#[derive(Debug, Clone, serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ParameterPatchDiffSummary {
    pub approved_recommendations_count: usize,
    pub changed_fields_count: usize,
    pub unchanged_fields_count: usize,
    pub warning_count: usize,
}

#[derive(Debug, Clone, serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ParameterPatchDiffEntry {
    pub key: String,
    pub current_value: Option<f64>,
    pub recommended_value: Option<f64>,
    pub current_display: String,
    pub recommended_display: String,
    pub change_type: ParameterPatchChangeType,
    pub numeric_delta: Option<f64>,
    pub percent_delta: Option<f64>,
    pub source_recommendation_id: String,
    pub review_decision_id: String,
    pub severity: String,
    pub notes: Vec<String>,
}

#[derive(Debug, Clone, serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ParameterPatchDiff {
    pub export_id: String,
    pub export_path: String,
    pub generated_at: Option<String>,
    pub generated_at_ms: Option<i64>,
    pub apply_mode: String,
    pub runtime_modified: bool,
    pub summary: ParameterPatchDiffSummary,
    pub entries: Vec<ParameterPatchDiffEntry>,
    pub warnings: Vec<String>,
}

#[derive(Debug, Clone, serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ParameterPatchAudit {
    pub export_id: String,
    pub export_path: String,
    pub generated_at: Option<String>,
    pub apply_mode: String,
    pub runtime_modified: bool,
    pub summary: ParameterPatchDiffSummary,
    pub warnings: Vec<String>,
    pub manual_apply_checklist: Vec<String>,
    pub entries: Vec<ParameterPatchDiffEntry>,
}

pub struct ParameterPatchDiffStore {
    export_store: ManualParameterExportStore,
    config: Option<AppConfig>,
}

impl ParameterPatchDiffStore {
    pub fn new(report_dir: impl Into<PathBuf>, config: Option<AppConfig>) -> Self {
        Self {
            export_store: ManualParameterExportStore::new(report_dir),
            config,
        }
    }

    pub fn latest_diff(&self) -> anyhow::Result<Option<ParameterPatchDiff>> {
        let Some(export) = self.export_store.latest_export()? else {
            return Ok(None);
        };
        Ok(Some(self.build_diff(export)?))
    }

    pub fn diff_by_id(&self, export_id: &str) -> anyhow::Result<Option<ParameterPatchDiff>> {
        let Some(export) = self.export_store.get_export(export_id)? else {
            return Ok(None);
        };
        Ok(Some(self.build_diff(export)?))
    }

    pub fn latest_audit(&self) -> anyhow::Result<Option<ParameterPatchAudit>> {
        let Some(diff) = self.latest_diff()? else {
            return Ok(None);
        };
        Ok(Some(build_audit(diff)))
    }

    pub fn audit_by_id(&self, export_id: &str) -> anyhow::Result<Option<ParameterPatchAudit>> {
        let Some(diff) = self.diff_by_id(export_id)? else {
            return Ok(None);
        };
        Ok(Some(build_audit(diff)))
    }

    fn build_diff(&self, export: ManualParameterExportEntry) -> anyhow::Result<ParameterPatchDiff> {
        let Some(config) = self.config.as_ref() else {
            anyhow::bail!("current_config_unavailable");
        };

        let snapshot = current_config_snapshot(config);
        let mut warnings = Vec::new();
        let entries: Vec<ParameterPatchDiffEntry> = export
            .export
            .items
            .iter()
            .map(|item| build_diff_entry(item, &snapshot))
            .inspect(|entry| {
                if matches!(
                    entry.change_type,
                    ParameterPatchChangeType::MissingInCurrentConfig
                        | ParameterPatchChangeType::TypeMismatch
                        | ParameterPatchChangeType::UnsafeOrUnknownField
                ) {
                    warnings.push(format!(
                        "{} -> {}",
                        entry.key,
                        change_type_label(entry.change_type)
                    ));
                }
            })
            .collect();

        let changed_fields_count = entries
            .iter()
            .filter(|entry| entry.change_type == ParameterPatchChangeType::Changed)
            .count();
        let unchanged_fields_count = entries
            .iter()
            .filter(|entry| entry.change_type == ParameterPatchChangeType::Unchanged)
            .count();
        let warning_count = entries
            .iter()
            .filter(|entry| {
                matches!(
                    entry.change_type,
                    ParameterPatchChangeType::MissingInCurrentConfig
                        | ParameterPatchChangeType::TypeMismatch
                        | ParameterPatchChangeType::UnsafeOrUnknownField
                )
            })
            .count();

        Ok(ParameterPatchDiff {
            export_id: export.summary.export_id,
            export_path: export.summary.json_path,
            generated_at: Some(export.export.created_at.clone()),
            generated_at_ms: export.summary.created_at_ms,
            apply_mode: "manual_only".to_string(),
            runtime_modified: false,
            summary: ParameterPatchDiffSummary {
                approved_recommendations_count: export.export.items.len(),
                changed_fields_count,
                unchanged_fields_count,
                warning_count,
            },
            entries,
            warnings,
        })
    }
}

fn build_audit(diff: ParameterPatchDiff) -> ParameterPatchAudit {
    ParameterPatchAudit {
        export_id: diff.export_id.clone(),
        export_path: diff.export_path.clone(),
        generated_at: diff.generated_at.clone(),
        apply_mode: diff.apply_mode.clone(),
        runtime_modified: diff.runtime_modified,
        summary: diff.summary.clone(),
        warnings: diff.warnings.clone(),
        manual_apply_checklist: vec![
            "Confirm every changed field against the calibration report.".to_string(),
            "Copy the patch manually into config after a second human review.".to_string(),
            "Do not reload runtime from the dashboard.".to_string(),
            "Run replay or dry-run verification again after any manual config edit.".to_string(),
        ],
        entries: diff.entries,
    }
}

fn current_config_snapshot(config: &AppConfig) -> BTreeMap<String, f64> {
    BTreeMap::from([
        (
            "toxicity.threshold_btc".to_string(),
            config.toxic_volume_alert_btc,
        ),
        (
            "vpin.bucket_size_btc".to_string(),
            config.vpin_bucket_size_btc,
        ),
        (
            "vpin.lookback_buckets".to_string(),
            config.vpin_lookback_buckets as f64,
        ),
        ("vpin.spike_zscore".to_string(), config.vpin_spike_zscore),
        (
            "liq_hunt.likely_score".to_string(),
            config.liq_hunt_likely_score,
        ),
        (
            "liq_hunt.active_score".to_string(),
            config.liq_hunt_active_score,
        ),
    ])
}

fn build_diff_entry(
    item: &ManualParameterPatchItem,
    snapshot: &BTreeMap<String, f64>,
) -> ParameterPatchDiffEntry {
    let review_decision_id = format!("{}::{}", item.recommendation_id, item.review.updated_at);
    let known_key = known_parameter_key(&item.parameter_key);
    let current_value = snapshot.get(&item.parameter_key).copied();
    let recommended_value = item.recommended_value;

    let change_type = if !known_key {
        ParameterPatchChangeType::UnsafeOrUnknownField
    } else if recommended_value.is_none() {
        ParameterPatchChangeType::MissingInPatch
    } else if current_value.is_none() {
        ParameterPatchChangeType::MissingInCurrentConfig
    } else if approximately_equal(current_value, recommended_value) {
        ParameterPatchChangeType::Unchanged
    } else {
        ParameterPatchChangeType::Changed
    };

    let numeric_delta = match (current_value, recommended_value) {
        (Some(current), Some(recommended))
            if matches!(
                change_type,
                ParameterPatchChangeType::Changed | ParameterPatchChangeType::Unchanged
            ) =>
        {
            Some(recommended - current)
        }
        _ => None,
    };
    let percent_delta = match (current_value, numeric_delta) {
        (Some(current), Some(delta)) if current.abs() > f64::EPSILON => Some(delta / current),
        _ => None,
    };

    let mut notes = Vec::new();
    notes.push(item.reason.clone());
    if let Some(effect) = &item.expected_effect {
        notes.push(format!("expected_effect: {effect}"));
    }
    if let Some(risk) = &item.risk_note {
        notes.push(format!("risk: {risk}"));
    }
    notes.push(format!(
        "review_status: {}",
        serde_variant_name(&item.review.status)
    ));
    if !known_key {
        notes.push("field is not part of the current runtime config snapshot".to_string());
    }

    ParameterPatchDiffEntry {
        key: item.parameter_key.clone(),
        current_value,
        recommended_value,
        current_display: current_value
            .map(format_scalar)
            .unwrap_or_else(|| "Unavailable".to_string()),
        recommended_display: recommended_value
            .map(format_scalar)
            .unwrap_or_else(|| "Unavailable".to_string()),
        change_type,
        numeric_delta,
        percent_delta,
        source_recommendation_id: item.recommendation_id.clone(),
        review_decision_id,
        severity: severity_for_change(change_type).to_string(),
        notes,
    }
}

fn known_parameter_key(key: &str) -> bool {
    matches!(
        key,
        "toxicity.threshold_btc"
            | "vpin.bucket_size_btc"
            | "vpin.lookback_buckets"
            | "vpin.spike_zscore"
            | "liq_hunt.likely_score"
            | "liq_hunt.active_score"
            | "toxicity.min_toxic_ratio"
    )
}

fn severity_for_change(change_type: ParameterPatchChangeType) -> &'static str {
    match change_type {
        ParameterPatchChangeType::Changed => "medium",
        ParameterPatchChangeType::Unchanged => "low",
        ParameterPatchChangeType::MissingInCurrentConfig
        | ParameterPatchChangeType::TypeMismatch
        | ParameterPatchChangeType::UnsafeOrUnknownField => "high",
        ParameterPatchChangeType::MissingInPatch => "medium",
    }
}

fn change_type_label(change_type: ParameterPatchChangeType) -> &'static str {
    match change_type {
        ParameterPatchChangeType::Unchanged => "unchanged",
        ParameterPatchChangeType::Changed => "changed",
        ParameterPatchChangeType::MissingInCurrentConfig => "missing_in_current_config",
        ParameterPatchChangeType::MissingInPatch => "missing_in_patch",
        ParameterPatchChangeType::TypeMismatch => "type_mismatch",
        ParameterPatchChangeType::UnsafeOrUnknownField => "unsafe_or_unknown_field",
    }
}

fn format_scalar(value: f64) -> String {
    if (value.fract()).abs() < f64::EPSILON {
        format!("{value:.0}")
    } else {
        format!("{value:.2}")
    }
}

fn approximately_equal(left: Option<f64>, right: Option<f64>) -> bool {
    match (left, right) {
        (Some(left), Some(right)) => (left - right).abs() < f64::EPSILON,
        (None, None) => true,
        _ => false,
    }
}

fn serde_variant_name(
    status: &super::parameter_recommendation_review_store::ReviewStatus,
) -> &'static str {
    match status {
        super::parameter_recommendation_review_store::ReviewStatus::Pending => "pending",
        super::parameter_recommendation_review_store::ReviewStatus::ApprovedForManualApply => {
            "approved_for_manual_apply"
        }
        super::parameter_recommendation_review_store::ReviewStatus::Rejected => "rejected",
        super::parameter_recommendation_review_store::ReviewStatus::Watch => "watch",
        super::parameter_recommendation_review_store::ReviewStatus::NeedsMoreData => {
            "needs_more_data"
        }
        super::parameter_recommendation_review_store::ReviewStatus::Archived => "archived",
    }
}
