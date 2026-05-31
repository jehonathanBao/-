use crate::types::{
    durable_archive_dryrun::{
        DurableArchiveDryRunEvidenceRefs, DurableArchiveDryRunFieldContract,
        DurableArchiveDryRunRecord, DurableArchiveDryRunResponse,
        DurableArchiveDryRunReviewPackResponse, DurableArchiveDryRunReviewPackSummary,
        DurableArchiveDryRunSafetyFlags, DurableArchiveDryRunValidation,
    },
    toxic_signal_history::{
        ToxicSignalHistoryAlertItem, ToxicSignalHistoryRecentResponse, ToxicSignalHistoryReportItem,
    },
};
use serde_json::Value;
use std::collections::{BTreeSet, HashSet};
use std::fmt::Write;

pub const DURABLE_ARCHIVE_DRYRUN_SCHEMA_VERSION: u32 = 1;
const MAX_DRYRUN_VALIDATION_RECORDS: usize = 100;
const MAX_DRYRUN_NOTE_CHARS: usize = 4_096;
const MAX_DRYRUN_EVIDENCE_REFS: usize = 20;

const REQUIRED_REQUEST_FIELDS: &[&str] = &[
    "signalId",
    "symbol",
    "signalKind",
    "createdAtMs",
    "schemaVersion",
    "sourceModule",
];

const FORBIDDEN_FIELDS: &[&str] = &[
    "privateKey",
    "seedPhrase",
    "apiSecret",
    "exchangeCredentials",
    "signedTransaction",
    "unsignedTransactionPayload",
    "orderPayload",
    "cancelPayload",
    "amendPayload",
    "walletAddressForSigning",
    "telegramBotToken",
    "webhookSecret",
    "liveTradingInstruction",
];

const EXECUTION_LIKE_FIELDS: &[&str] = &[
    "execute",
    "placeOrder",
    "cancelOrder",
    "amendOrder",
    "openPosition",
    "closePosition",
    "applyWeight",
    "reloadStrategy",
    "sendTelegram",
    "sendWebhook",
];

const UNSAFE_NOTIFICATION_FIELDS: &[&str] = &[
    "notificationSent",
    "executionTriggered",
    "webhookSent",
    "telegramSent",
];

pub fn build_durable_archive_dryrun_response(
    selected_symbol: &str,
    history_recent: &ToxicSignalHistoryRecentResponse,
    alert_items: &[ToxicSignalHistoryAlertItem],
    report_items: &[ToxicSignalHistoryReportItem],
) -> DurableArchiveDryRunResponse {
    let field_contract = DurableArchiveDryRunFieldContract {
        source_snapshot_fields: vec![
            "sourceSignalId".to_string(),
            "sourceSignalType".to_string(),
            "symbol".to_string(),
            "signalTsMs".to_string(),
            "signalLayer".to_string(),
            "direction".to_string(),
            "confidence".to_string(),
            "replayRef".to_string(),
            "markoutRef".to_string(),
            "governanceRef".to_string(),
        ],
        derived_fields: vec![
            "archiveRecordId".to_string(),
            "schemaVersion".to_string(),
            "createdAtMs".to_string(),
            "toxicityScore".to_string(),
            "safetyFlags".to_string(),
            "writeMode".to_string(),
            "archiveWriteEnabled".to_string(),
        ],
        evidence_reference_fields: vec![
            "evidenceRefs".to_string(),
            "replayRef".to_string(),
            "markoutRef".to_string(),
            "governanceRef".to_string(),
        ],
    };

    let records = history_recent
        .items
        .iter()
        .map(|item| {
            let matching_alert = alert_items
                .iter()
                .find(|alert| alert.signal_id == item.signal_id);
            let matching_report = report_items
                .iter()
                .find(|report| report.symbol.eq_ignore_ascii_case(&item.symbol));
            build_record(item, matching_alert, matching_report)
        })
        .collect::<Vec<_>>();

    DurableArchiveDryRunResponse {
        ok: true,
        read_only: true,
        runtime_modified: false,
        analysis_only: true,
        execution_enabled: false,
        dry_run: true,
        action: "dry_run_write".to_string(),
        selected_symbol: selected_symbol.to_string(),
        schema_version: DURABLE_ARCHIVE_DRYRUN_SCHEMA_VERSION,
        records_prepared: records.len(),
        archive_write_enabled: false,
        durable_storage_enabled: false,
        database_write_enabled: false,
        jsonl_write_enabled: false,
        sqlite_write_enabled: false,
        notification_sent: false,
        execution_triggered: false,
        write_mode: "dry_run_only".to_string(),
        field_contract,
        validation: DurableArchiveDryRunValidation {
            valid: records.iter().all(validate_record_field_types)
                && records.iter().all(validate_record_evidence_refs),
            field_types_valid: records.iter().all(validate_record_field_types),
            source_snapshot_fields_valid: true,
            derived_fields_valid: true,
            evidence_refs_valid: records.iter().all(validate_record_evidence_refs),
            persistence_attempted: false,
            errors: Vec::new(),
            warnings: if records.is_empty() {
                vec!["no_records".to_string()]
            } else {
                Vec::new()
            },
            unsafe_fields_detected: Vec::new(),
            duplicate_signal_ids: Vec::new(),
            missing_required_fields: Vec::new(),
            forbidden_fields: Vec::new(),
            unsafe_execution_field_detected: false,
        },
        records,
        safety_boundary: vec![
            "archiveWriteEnabled=false".to_string(),
            "durableStorageEnabled=false".to_string(),
            "databaseWriteEnabled=false".to_string(),
            "jsonlWriteEnabled=false".to_string(),
            "sqliteWriteEnabled=false".to_string(),
            "runtimeModified=false".to_string(),
            "executionEnabled=false".to_string(),
            "notificationSent=false".to_string(),
            "executionTriggered=false".to_string(),
            "No order placement".to_string(),
            "No wallet/signing".to_string(),
            "No live trading".to_string(),
        ],
        operator_notes: vec![
            "Dry-run only. Archive payloads are validated but not persisted.".to_string(),
            "This contract respects the S15A schema draft and keeps all archive write flags disabled."
                .to_string(),
            "No DB, JSONL, or SQLite write is attempted from this endpoint.".to_string(),
        ],
    }
}

pub fn build_durable_archive_dryrun_response_from_payload(
    selected_symbol: &str,
    payload: &Value,
) -> DurableArchiveDryRunResponse {
    let mut validation = RequestValidation::default();
    let records = payload
        .get("records")
        .and_then(Value::as_array)
        .cloned()
        .unwrap_or_default();

    collect_unsafe_fields(payload, "", &mut validation);
    validate_top_level_payload(payload, &records, &mut validation);

    let mut seen_signal_ids = HashSet::new();
    let mut prepared_records = Vec::new();
    for record in records.iter().take(MAX_DRYRUN_VALIDATION_RECORDS) {
        validate_request_record(record, &mut validation);

        let signal_id = record
            .get("signalId")
            .and_then(Value::as_str)
            .unwrap_or_default();
        if !signal_id.is_empty() && !seen_signal_ids.insert(signal_id.to_string()) {
            validation
                .duplicate_signal_ids
                .insert(signal_id.to_string());
            validation
                .warnings
                .insert("duplicate_signal_id".to_string());
        }

        if request_record_is_preparable(record) {
            prepared_records.push(build_record_from_request(record));
        }
    }

    if records.len() > MAX_DRYRUN_VALIDATION_RECORDS {
        validation.warnings.insert("payload_too_large".to_string());
    }

    let field_contract = default_field_contract();
    let errors = validation.errors.iter().cloned().collect::<Vec<_>>();
    let warnings = validation.warnings.iter().cloned().collect::<Vec<_>>();
    let unsafe_fields_detected = validation
        .unsafe_fields_detected
        .iter()
        .cloned()
        .collect::<Vec<_>>();
    let duplicate_signal_ids = validation
        .duplicate_signal_ids
        .iter()
        .cloned()
        .collect::<Vec<_>>();
    let missing_required_fields = validation
        .missing_required_fields
        .iter()
        .cloned()
        .collect::<Vec<_>>();
    let forbidden_fields = validation
        .forbidden_fields
        .iter()
        .cloned()
        .collect::<Vec<_>>();
    let valid = errors.is_empty();

    DurableArchiveDryRunResponse {
        ok: true,
        read_only: true,
        runtime_modified: false,
        analysis_only: true,
        execution_enabled: false,
        dry_run: true,
        action: "dry_run_write".to_string(),
        selected_symbol: selected_symbol.to_string(),
        schema_version: DURABLE_ARCHIVE_DRYRUN_SCHEMA_VERSION,
        records_prepared: if valid { prepared_records.len() } else { 0 },
        archive_write_enabled: false,
        durable_storage_enabled: false,
        database_write_enabled: false,
        jsonl_write_enabled: false,
        sqlite_write_enabled: false,
        notification_sent: false,
        execution_triggered: false,
        write_mode: "dry_run_only".to_string(),
        field_contract,
        validation: DurableArchiveDryRunValidation {
            valid,
            field_types_valid: !validation.errors.contains("missing_required_field")
                && !validation.errors.contains("invalid_schema_version"),
            source_snapshot_fields_valid: !validation.errors.contains("missing_required_field"),
            derived_fields_valid: !validation.errors.contains("invalid_schema_version"),
            evidence_refs_valid: !validation.errors.contains("invalid_evidence_ref"),
            persistence_attempted: false,
            errors,
            warnings,
            unsafe_fields_detected,
            duplicate_signal_ids,
            missing_required_fields,
            forbidden_fields,
            unsafe_execution_field_detected: validation.unsafe_execution_field_detected,
        },
        records: if valid { prepared_records } else { Vec::new() },
        safety_boundary: safety_boundary(),
        operator_notes: vec![
            "Dry-run validation matrix only. No archive writer is active.".to_string(),
            "Bad payloads are reported as validation errors without persistence.".to_string(),
            "No DB, JSONL, SQLite, file, notification, or execution side effect is attempted."
                .to_string(),
        ],
    }
}

fn default_field_contract() -> DurableArchiveDryRunFieldContract {
    DurableArchiveDryRunFieldContract {
        source_snapshot_fields: vec![
            "sourceSignalId".to_string(),
            "sourceSignalType".to_string(),
            "symbol".to_string(),
            "signalTsMs".to_string(),
            "signalLayer".to_string(),
            "direction".to_string(),
            "confidence".to_string(),
            "replayRef".to_string(),
            "markoutRef".to_string(),
            "governanceRef".to_string(),
        ],
        derived_fields: vec![
            "archiveRecordId".to_string(),
            "schemaVersion".to_string(),
            "createdAtMs".to_string(),
            "toxicityScore".to_string(),
            "safetyFlags".to_string(),
            "writeMode".to_string(),
            "archiveWriteEnabled".to_string(),
        ],
        evidence_reference_fields: vec![
            "evidenceRefs".to_string(),
            "replayRef".to_string(),
            "markoutRef".to_string(),
            "governanceRef".to_string(),
        ],
    }
}

fn safety_boundary() -> Vec<String> {
    vec![
        "archiveWriteEnabled=false".to_string(),
        "durableStorageEnabled=false".to_string(),
        "databaseWriteEnabled=false".to_string(),
        "jsonlWriteEnabled=false".to_string(),
        "sqliteWriteEnabled=false".to_string(),
        "runtimeModified=false".to_string(),
        "executionEnabled=false".to_string(),
        "notificationSent=false".to_string(),
        "executionTriggered=false".to_string(),
        "No order placement".to_string(),
        "No wallet/signing".to_string(),
        "No live trading".to_string(),
    ]
}

#[derive(Default)]
struct RequestValidation {
    errors: BTreeSet<String>,
    warnings: BTreeSet<String>,
    unsafe_fields_detected: BTreeSet<String>,
    duplicate_signal_ids: BTreeSet<String>,
    missing_required_fields: BTreeSet<String>,
    forbidden_fields: BTreeSet<String>,
    unsafe_execution_field_detected: bool,
}

fn validate_top_level_payload(
    payload: &Value,
    records: &[Value],
    validation: &mut RequestValidation,
) {
    if payload.get("records").is_none() || records.is_empty() {
        validation.errors.insert("no_records".to_string());
    }

    if payload.to_string().len() > MAX_DRYRUN_NOTE_CHARS * 4 {
        validation.warnings.insert("payload_too_large".to_string());
    }
}

fn validate_request_record(record: &Value, validation: &mut RequestValidation) {
    for field in REQUIRED_REQUEST_FIELDS {
        let missing = record.get(*field).is_none()
            || record
                .get(*field)
                .and_then(Value::as_str)
                .is_some_and(|value| value.trim().is_empty());
        if missing {
            validation
                .errors
                .insert("missing_required_field".to_string());
            validation
                .missing_required_fields
                .insert((*field).to_string());
        }
    }

    if record.get("schemaVersion").and_then(Value::as_u64)
        != Some(DURABLE_ARCHIVE_DRYRUN_SCHEMA_VERSION as u64)
    {
        validation
            .errors
            .insert("invalid_schema_version".to_string());
    }

    validate_evidence_refs(record.get("evidenceRefs"), validation);

    if record
        .get("note")
        .and_then(Value::as_str)
        .is_some_and(|note| note.len() > MAX_DRYRUN_NOTE_CHARS)
    {
        validation.warnings.insert("payload_too_large".to_string());
    }
}

fn validate_evidence_refs(evidence_refs: Option<&Value>, validation: &mut RequestValidation) {
    let Some(evidence_refs) = evidence_refs else {
        validation.errors.insert("invalid_evidence_ref".to_string());
        return;
    };

    let refs = match evidence_refs {
        Value::Array(items) => items
            .iter()
            .filter_map(Value::as_str)
            .map(str::to_string)
            .collect::<Vec<_>>(),
        Value::Object(map) => map
            .values()
            .filter_map(Value::as_str)
            .map(str::to_string)
            .collect::<Vec<_>>(),
        Value::String(value) => vec![value.clone()],
        _ => Vec::new(),
    };

    if refs.is_empty() {
        validation.errors.insert("invalid_evidence_ref".to_string());
    }

    if refs.len() > MAX_DRYRUN_EVIDENCE_REFS {
        validation.warnings.insert("payload_too_large".to_string());
    }

    if refs.iter().any(|value| evidence_ref_is_unsafe(value)) {
        validation.errors.insert("invalid_evidence_ref".to_string());
    }
}

fn evidence_ref_is_unsafe(value: &str) -> bool {
    let lower = value.to_ascii_lowercase();
    lower.contains("private")
        || lower.contains("secret")
        || lower.contains("seed")
        || lower.contains("wallet")
        || lower.contains("signing")
        || lower.contains("credential")
        || lower.contains("telegram")
        || lower.contains("webhook")
        || lower.starts_with("file:")
        || lower.starts_with("c:\\")
        || lower.starts_with("/")
}

fn collect_unsafe_fields(value: &Value, path: &str, validation: &mut RequestValidation) {
    match value {
        Value::Object(map) => {
            for (key, child) in map {
                let child_path = if path.is_empty() {
                    key.to_string()
                } else {
                    format!("{path}.{key}")
                };
                if contains_case_insensitive(FORBIDDEN_FIELDS, key) {
                    validation
                        .errors
                        .insert("forbidden_field_present".to_string());
                    validation.forbidden_fields.insert(key.to_string());
                    validation.unsafe_fields_detected.insert(child_path.clone());
                }
                if contains_case_insensitive(EXECUTION_LIKE_FIELDS, key) {
                    validation
                        .errors
                        .insert("unsafe_execution_field_detected".to_string());
                    validation.unsafe_execution_field_detected = true;
                    validation.unsafe_fields_detected.insert(child_path.clone());
                }
                if contains_case_insensitive(UNSAFE_NOTIFICATION_FIELDS, key)
                    && child.as_bool() == Some(true)
                {
                    validation
                        .errors
                        .insert("unsafe_notification_field_present".to_string());
                    validation.unsafe_fields_detected.insert(child_path.clone());
                }
                collect_unsafe_fields(child, &child_path, validation);
            }
        }
        Value::Array(items) => {
            for (index, child) in items.iter().enumerate() {
                collect_unsafe_fields(child, &format!("{path}[{index}]"), validation);
            }
        }
        _ => {}
    }
}

fn contains_case_insensitive(items: &[&str], needle: &str) -> bool {
    items.iter().any(|item| item.eq_ignore_ascii_case(needle))
}

fn request_record_is_preparable(record: &Value) -> bool {
    REQUIRED_REQUEST_FIELDS
        .iter()
        .all(|field| record.get(*field).is_some())
        && record.get("schemaVersion").and_then(Value::as_u64)
            == Some(DURABLE_ARCHIVE_DRYRUN_SCHEMA_VERSION as u64)
        && record
            .get("evidenceRefs")
            .is_some_and(|refs| !refs.to_string().is_empty())
}

fn build_record_from_request(record: &Value) -> DurableArchiveDryRunRecord {
    let signal_id = required_string(record, "signalId");
    let signal_kind = required_string(record, "signalKind");
    let symbol = required_string(record, "symbol");
    let source_module = required_string(record, "sourceModule");
    let direction = record
        .get("direction")
        .or_else(|| record.get("directionBias"))
        .and_then(Value::as_str)
        .unwrap_or("neutral")
        .to_string();
    let confidence = record
        .get("confidence")
        .and_then(Value::as_f64)
        .unwrap_or(0.0)
        .clamp(0.0, 1.0);
    let signal_ts_ms = record
        .get("createdAtMs")
        .and_then(Value::as_u64)
        .unwrap_or_default();
    let replay_ref = Some(format!("replay:{symbol}:{signal_id}"));
    let markout_ref = Some(format!("markout:{signal_id}"));
    let governance_ref = Some(format!("governance:{symbol}"));
    let evidence_refs = DurableArchiveDryRunEvidenceRefs {
        signal_history_ref: format!("signal_history:{signal_id}"),
        replay_ref: replay_ref.clone(),
        markout_ref: markout_ref.clone(),
        governance_ref: governance_ref.clone(),
        alert_preview_ref: Some(format!("alert_preview:{signal_id}:dry_run_validation")),
        report_ref: Some(format!("signal_report:dry_run:{symbol}")),
    };

    DurableArchiveDryRunRecord {
        archive_record_id: format!("archive-dryrun-{signal_id}"),
        schema_version: DURABLE_ARCHIVE_DRYRUN_SCHEMA_VERSION,
        created_at_ms: signal_ts_ms,
        source_signal_id: signal_id,
        source_signal_type: signal_kind,
        symbol,
        signal_ts_ms,
        signal_layer: source_module,
        direction,
        toxicity_score: derive_toxicity_score("medium", confidence),
        confidence,
        evidence_refs,
        replay_ref,
        markout_ref,
        governance_ref,
        safety_flags: fixed_safety_flags(),
        write_mode: "dry_run_only".to_string(),
        archive_write_enabled: false,
    }
}

fn required_string(record: &Value, field: &str) -> String {
    record
        .get(field)
        .and_then(Value::as_str)
        .unwrap_or_default()
        .to_string()
}

pub fn build_durable_archive_dryrun_review_pack(
    payload: &DurableArchiveDryRunResponse,
) -> DurableArchiveDryRunReviewPackResponse {
    let dry_run_id = build_dry_run_id(payload);
    let summary = DurableArchiveDryRunReviewPackSummary {
        records_prepared: payload.records_prepared,
        validation_error_count: payload.validation.errors.len(),
        validation_warning_count: payload.validation.warnings.len(),
        unsafe_field_count: payload.validation.unsafe_fields_detected.len(),
        duplicate_signal_id_count: payload.validation.duplicate_signal_ids.len(),
        missing_required_field_count: payload.validation.missing_required_fields.len(),
        forbidden_field_count: payload.validation.forbidden_fields.len(),
    };
    let markdown = build_review_pack_markdown(&dry_run_id, payload, &summary);

    DurableArchiveDryRunReviewPackResponse {
        found: true,
        read_only: payload.read_only,
        runtime_modified: payload.runtime_modified,
        analysis_only: payload.analysis_only,
        execution_enabled: payload.execution_enabled,
        manual_review_required: true,
        archive_write_enabled: payload.archive_write_enabled,
        durable_storage_enabled: payload.durable_storage_enabled,
        database_write_enabled: payload.database_write_enabled,
        jsonl_write_enabled: payload.jsonl_write_enabled,
        sqlite_write_enabled: payload.sqlite_write_enabled,
        notification_sent: payload.notification_sent,
        execution_triggered: payload.execution_triggered,
        dry_run: payload.dry_run,
        review_pack_type: "durable_archive_dryrun_review_pack".to_string(),
        dry_run_id,
        selected_symbol: payload.selected_symbol.clone(),
        source_action: payload.action.clone(),
        summary,
        field_contract: payload.field_contract.clone(),
        validation: payload.validation.clone(),
        records: payload.records.clone(),
        safety_boundary: payload.safety_boundary.clone(),
        operator_notes: review_pack_notes(payload),
        markdown,
    }
}

pub fn build_durable_archive_dryrun_review_pack_not_found(
    selected_symbol: &str,
    dry_run_id: &str,
) -> DurableArchiveDryRunReviewPackResponse {
    let payload = DurableArchiveDryRunResponse {
        ok: true,
        read_only: true,
        runtime_modified: false,
        analysis_only: true,
        execution_enabled: false,
        dry_run: true,
        action: "dry_run_write".to_string(),
        selected_symbol: selected_symbol.to_string(),
        schema_version: DURABLE_ARCHIVE_DRYRUN_SCHEMA_VERSION,
        records_prepared: 0,
        archive_write_enabled: false,
        durable_storage_enabled: false,
        database_write_enabled: false,
        jsonl_write_enabled: false,
        sqlite_write_enabled: false,
        notification_sent: false,
        execution_triggered: false,
        write_mode: "dry_run_only".to_string(),
        field_contract: default_field_contract(),
        validation: DurableArchiveDryRunValidation {
            valid: false,
            field_types_valid: false,
            source_snapshot_fields_valid: false,
            derived_fields_valid: false,
            evidence_refs_valid: false,
            persistence_attempted: false,
            errors: vec!["review_pack_not_found".to_string()],
            warnings: Vec::new(),
            unsafe_fields_detected: Vec::new(),
            duplicate_signal_ids: Vec::new(),
            missing_required_fields: Vec::new(),
            forbidden_fields: Vec::new(),
            unsafe_execution_field_detected: false,
        },
        records: Vec::new(),
        safety_boundary: safety_boundary(),
        operator_notes: vec![
            "Requested review pack id was not found for the current dry-run snapshot.".to_string(),
        ],
    };
    let summary = DurableArchiveDryRunReviewPackSummary {
        records_prepared: 0,
        validation_error_count: 1,
        validation_warning_count: 0,
        unsafe_field_count: 0,
        duplicate_signal_id_count: 0,
        missing_required_field_count: 0,
        forbidden_field_count: 0,
    };
    let markdown = format!(
        "# Durable Archive Dry-run Review Pack\n\nDry Run ID: {dry_run_id}\n\n## Status\n- Found: false\n- Selected symbol: {selected_symbol}\n- readOnly=true\n- analysisOnly=true\n- executionEnabled=false\n- archiveWriteEnabled=false\n- notificationSent=false\n- executionTriggered=false\n\n## Validation Errors\n- review_pack_not_found\n\n## Safety Boundary\n- No order placement\n- No wallet/signing\n- No live trading\n"
    );

    DurableArchiveDryRunReviewPackResponse {
        found: false,
        read_only: payload.read_only,
        runtime_modified: payload.runtime_modified,
        analysis_only: payload.analysis_only,
        execution_enabled: payload.execution_enabled,
        manual_review_required: true,
        archive_write_enabled: payload.archive_write_enabled,
        durable_storage_enabled: payload.durable_storage_enabled,
        database_write_enabled: payload.database_write_enabled,
        jsonl_write_enabled: payload.jsonl_write_enabled,
        sqlite_write_enabled: payload.sqlite_write_enabled,
        notification_sent: payload.notification_sent,
        execution_triggered: payload.execution_triggered,
        dry_run: payload.dry_run,
        review_pack_type: "durable_archive_dryrun_review_pack".to_string(),
        dry_run_id: dry_run_id.to_string(),
        selected_symbol: selected_symbol.to_string(),
        source_action: payload.action,
        summary,
        field_contract: payload.field_contract,
        validation: payload.validation,
        records: payload.records,
        safety_boundary: payload.safety_boundary,
        operator_notes: payload.operator_notes,
        markdown,
    }
}

pub fn build_dry_run_id(payload: &DurableArchiveDryRunResponse) -> String {
    format!(
        "dryrun-{}-v{}-r{}-e{}-w{}",
        payload.selected_symbol.to_ascii_lowercase(),
        payload.schema_version,
        payload.records_prepared,
        payload.validation.errors.len(),
        payload.validation.warnings.len()
    )
}

fn review_pack_notes(payload: &DurableArchiveDryRunResponse) -> Vec<String> {
    let mut notes = payload.operator_notes.clone();
    notes.push(
        "Review pack only. Manual review required before any future archive design work."
            .to_string(),
    );
    notes.push(
        "This pack does not write DB, JSONL, SQLite, files, notifications, or execution state."
            .to_string(),
    );
    notes
}

fn build_review_pack_markdown(
    dry_run_id: &str,
    payload: &DurableArchiveDryRunResponse,
    summary: &DurableArchiveDryRunReviewPackSummary,
) -> String {
    let mut markdown = String::new();
    let _ = writeln!(markdown, "# Durable Archive Dry-run Review Pack");
    let _ = writeln!(markdown);
    let _ = writeln!(markdown, "Dry Run ID: {dry_run_id}");
    let _ = writeln!(markdown, "Selected Symbol: {}", payload.selected_symbol);
    let _ = writeln!(markdown);
    let _ = writeln!(markdown, "## Status");
    let _ = writeln!(markdown, "- readOnly=true");
    let _ = writeln!(markdown, "- runtimeModified=false");
    let _ = writeln!(markdown, "- analysisOnly=true");
    let _ = writeln!(markdown, "- executionEnabled=false");
    let _ = writeln!(markdown, "- manualReviewRequired=true");
    let _ = writeln!(markdown, "- archiveWriteEnabled=false");
    let _ = writeln!(markdown, "- durableStorageEnabled=false");
    let _ = writeln!(markdown, "- databaseWriteEnabled=false");
    let _ = writeln!(markdown, "- jsonlWriteEnabled=false");
    let _ = writeln!(markdown, "- sqliteWriteEnabled=false");
    let _ = writeln!(markdown, "- notificationSent=false");
    let _ = writeln!(markdown, "- executionTriggered=false");
    let _ = writeln!(markdown);
    let _ = writeln!(markdown, "## Summary");
    let _ = writeln!(markdown, "- Records prepared: {}", summary.records_prepared);
    let _ = writeln!(
        markdown,
        "- Validation errors: {}",
        summary.validation_error_count
    );
    let _ = writeln!(
        markdown,
        "- Validation warnings: {}",
        summary.validation_warning_count
    );
    let _ = writeln!(markdown, "- Unsafe fields: {}", summary.unsafe_field_count);
    let _ = writeln!(
        markdown,
        "- Duplicate signal IDs: {}",
        summary.duplicate_signal_id_count
    );
    let _ = writeln!(
        markdown,
        "- Missing required fields: {}",
        summary.missing_required_field_count
    );
    let _ = writeln!(
        markdown,
        "- Forbidden fields: {}",
        summary.forbidden_field_count
    );
    let _ = writeln!(markdown);
    let _ = writeln!(markdown, "## Validation Errors");
    if payload.validation.errors.is_empty() {
        let _ = writeln!(markdown, "- None");
    } else {
        for item in &payload.validation.errors {
            let _ = writeln!(markdown, "- {item}");
        }
    }
    let _ = writeln!(markdown);
    let _ = writeln!(markdown, "## Validation Warnings");
    if payload.validation.warnings.is_empty() {
        let _ = writeln!(markdown, "- None");
    } else {
        for item in &payload.validation.warnings {
            let _ = writeln!(markdown, "- {item}");
        }
    }
    let _ = writeln!(markdown);
    let _ = writeln!(markdown, "## Unsafe Fields");
    if payload.validation.unsafe_fields_detected.is_empty() {
        let _ = writeln!(markdown, "- None");
    } else {
        for item in &payload.validation.unsafe_fields_detected {
            let _ = writeln!(markdown, "- {item}");
        }
    }
    let _ = writeln!(markdown);
    let _ = writeln!(markdown, "## Field Contract");
    let _ = writeln!(
        markdown,
        "- Source snapshot fields: {}",
        payload.field_contract.source_snapshot_fields.join(", ")
    );
    let _ = writeln!(
        markdown,
        "- Derived fields: {}",
        payload.field_contract.derived_fields.join(", ")
    );
    let _ = writeln!(
        markdown,
        "- Evidence reference fields: {}",
        payload.field_contract.evidence_reference_fields.join(", ")
    );
    let _ = writeln!(markdown);
    let _ = writeln!(markdown, "## Prepared Records");
    if payload.records.is_empty() {
        let _ = writeln!(markdown, "- None");
    } else {
        for record in &payload.records {
            let _ = writeln!(
                markdown,
                "- {} | {} | {} | confidence={:.2} | toxicityScore={:.1}",
                record.archive_record_id,
                record.source_signal_id,
                record.symbol,
                record.confidence,
                record.toxicity_score
            );
        }
    }
    let _ = writeln!(markdown);
    let _ = writeln!(markdown, "## Safety Boundary");
    for item in &payload.safety_boundary {
        let _ = writeln!(markdown, "- {item}");
    }
    let _ = writeln!(markdown);
    let _ = writeln!(markdown, "## Operator Notes");
    for item in review_pack_notes(payload) {
        let _ = writeln!(markdown, "- {item}");
    }
    markdown
}

fn build_record(
    item: &crate::types::toxic_signal_history::ToxicSignalHistorySignalItem,
    matching_alert: Option<&ToxicSignalHistoryAlertItem>,
    matching_report: Option<&ToxicSignalHistoryReportItem>,
) -> DurableArchiveDryRunRecord {
    let replay_ref = Some(format!("replay:{}:{}", item.symbol, item.signal_id));
    let markout_ref = Some(format!("markout:{}", item.signal_id));
    let governance_ref = Some(format!("governance:{}", item.symbol));
    let evidence_refs = DurableArchiveDryRunEvidenceRefs {
        signal_history_ref: format!("signal_history:{}", item.signal_id),
        replay_ref: replay_ref.clone(),
        markout_ref: markout_ref.clone(),
        governance_ref: governance_ref.clone(),
        alert_preview_ref: matching_alert
            .map(|alert| format!("alert_preview:{}:{}", alert.signal_id, alert.preview_status)),
        report_ref: matching_report
            .map(|report| format!("signal_report:{}:{}", report.date, report.symbol)),
    };

    DurableArchiveDryRunRecord {
        archive_record_id: format!("archive-dryrun-{}", item.signal_id),
        schema_version: DURABLE_ARCHIVE_DRYRUN_SCHEMA_VERSION,
        created_at_ms: item.history_recorded_at_ms,
        source_signal_id: item.signal_id.clone(),
        source_signal_type: item.signal_kind.clone(),
        symbol: item.symbol.clone(),
        signal_ts_ms: item.created_at_ms,
        signal_layer: item.source.clone(),
        direction: item.direction_bias.clone(),
        toxicity_score: derive_toxicity_score(item.severity.as_str(), item.confidence),
        confidence: item.confidence,
        evidence_refs,
        replay_ref,
        markout_ref,
        governance_ref,
        safety_flags: fixed_safety_flags(),
        write_mode: "dry_run_only".to_string(),
        archive_write_enabled: false,
    }
}

fn fixed_safety_flags() -> DurableArchiveDryRunSafetyFlags {
    DurableArchiveDryRunSafetyFlags {
        archive_write_enabled: false,
        durable_storage_enabled: false,
        database_write_enabled: false,
        jsonl_write_enabled: false,
        sqlite_write_enabled: false,
        runtime_modified: false,
        execution_enabled: false,
        notification_sent: false,
        execution_triggered: false,
    }
}

fn derive_toxicity_score(severity: &str, confidence: f64) -> f64 {
    let severity_weight = match severity.to_ascii_lowercase().as_str() {
        "high" => 1.0,
        "medium" => 0.7,
        "low" => 0.4,
        _ => 0.2,
    };
    ((severity_weight + confidence.clamp(0.0, 1.0)) / 2.0 * 100.0 * 10.0).round() / 10.0
}

fn validate_record_field_types(record: &DurableArchiveDryRunRecord) -> bool {
    !record.archive_record_id.trim().is_empty()
        && !record.source_signal_id.trim().is_empty()
        && !record.source_signal_type.trim().is_empty()
        && !record.symbol.trim().is_empty()
        && !record.signal_layer.trim().is_empty()
        && !record.direction.trim().is_empty()
        && record.schema_version == DURABLE_ARCHIVE_DRYRUN_SCHEMA_VERSION
        && record.confidence.is_finite()
        && record.toxicity_score.is_finite()
}

fn validate_record_evidence_refs(record: &DurableArchiveDryRunRecord) -> bool {
    record.evidence_refs.signal_history_ref == format!("signal_history:{}", record.source_signal_id)
        && record.evidence_refs.replay_ref == record.replay_ref
        && record.evidence_refs.markout_ref == record.markout_ref
        && record.evidence_refs.governance_ref == record.governance_ref
}
