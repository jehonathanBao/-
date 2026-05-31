use crate::{
    toxicity::durable_archive_dryrun::{
        build_dry_run_id, build_durable_archive_dryrun_response,
        build_durable_archive_dryrun_response_from_payload,
        build_durable_archive_dryrun_review_pack,
        build_durable_archive_dryrun_review_pack_not_found,
    },
    types::{
        durable_archive_dryrun::{
            DurableArchiveDryRunResponse, DurableArchiveDryRunReviewPackResponse,
        },
        toxic_signal_history::{
            ToxicSignalHistoryAlertRecentResponse, ToxicSignalHistoryRecentResponse,
            ToxicSignalHistoryReportRecentResponse,
        },
    },
};
use serde_json::Value;

pub fn durable_archive_dryrun_write(
    selected_symbol: &str,
    history_recent: &ToxicSignalHistoryRecentResponse,
    alert_recent: &ToxicSignalHistoryAlertRecentResponse,
    report_recent: &ToxicSignalHistoryReportRecentResponse,
) -> DurableArchiveDryRunResponse {
    build_durable_archive_dryrun_response(
        selected_symbol,
        history_recent,
        &alert_recent.items,
        &report_recent.items,
    )
}

pub fn durable_archive_dryrun_validate_payload(
    selected_symbol: &str,
    payload: &Value,
) -> DurableArchiveDryRunResponse {
    build_durable_archive_dryrun_response_from_payload(selected_symbol, payload)
}

pub fn durable_archive_dryrun_review_pack_latest(
    payload: &DurableArchiveDryRunResponse,
) -> DurableArchiveDryRunReviewPackResponse {
    build_durable_archive_dryrun_review_pack(payload)
}

pub fn durable_archive_dryrun_review_pack_by_id(
    selected_symbol: &str,
    payload: &DurableArchiveDryRunResponse,
    dry_run_id: &str,
) -> DurableArchiveDryRunReviewPackResponse {
    let latest_id = build_dry_run_id(payload);
    if latest_id == dry_run_id {
        build_durable_archive_dryrun_review_pack(payload)
    } else {
        build_durable_archive_dryrun_review_pack_not_found(selected_symbol, dry_run_id)
    }
}
