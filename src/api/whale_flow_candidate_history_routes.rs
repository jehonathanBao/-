use axum::{
    extract::{Path, Query, State},
    Json,
};
use serde::Deserialize;

use crate::{
    api::{
        toxic_markout_routes::build_recent as build_markout_recent,
        whale_flow_routes::build_current_report as build_current_whale_flow_report,
    },
    app::AppState,
    toxicity::{
        whale_flow_calibration::{
            calibration_max_not_enough_data_rate_for_tuning, calibration_min_candidates_required,
            calibration_min_resolved_evidence_required, resolve_whale_candidate_markout,
        },
        whale_flow_calibration_service::build_whale_flow_threshold_calibration_report,
        whale_flow_candidate_history_service::WHALE_FLOW_CANDIDATE_HISTORY_RETENTION_MODE,
    },
    types::{
        whale_flow_calibration::WhaleFlowCalibrationReportResponse,
        whale_flow_candidate_history::{
            WhaleFlowCandidateHistoryItem, WhaleFlowCandidateHistoryRecentResponse,
            WhaleFlowCandidateHistoryStatusResponse,
        },
        whale_flow_signal::WhaleFlowCandidate,
    },
};

#[derive(Debug, Deserialize)]
pub struct WhaleFlowCandidateHistoryQuery {
    symbol: Option<String>,
}

pub async fn whale_flow_candidate_history_status(
    State(state): State<AppState>,
    Query(query): Query<WhaleFlowCandidateHistoryQuery>,
) -> Json<serde_json::Value> {
    let requested_symbol = query
        .symbol
        .unwrap_or_else(|| state.config().symbol.clone());
    Json(serde_json::json!(build_history_status(
        &state,
        &requested_symbol
    )))
}

pub async fn whale_flow_candidate_history_recent(
    State(state): State<AppState>,
    Query(query): Query<WhaleFlowCandidateHistoryQuery>,
) -> Json<serde_json::Value> {
    let requested_symbol = query
        .symbol
        .unwrap_or_else(|| state.config().symbol.clone());
    Json(serde_json::json!(build_history_recent(
        &state,
        &requested_symbol
    )))
}

pub async fn whale_flow_candidate_history_for_symbol(
    State(state): State<AppState>,
    Path(symbol): Path<String>,
) -> Json<serde_json::Value> {
    Json(serde_json::json!(build_history_recent(&state, &symbol)))
}

fn build_history_status(
    state: &AppState,
    requested_symbol: &str,
) -> WhaleFlowCandidateHistoryStatusResponse {
    let history_service = state.whale_flow_candidate_history_service();
    let snapshot = history_service.snapshot();
    let candidates = history_service.recent_candidates(requested_symbol);
    let current_candidates = candidates.len();
    let oldest_candidate_at_ms = candidates.iter().map(|candidate| candidate.ts_ms).min();
    let latest_candidate_at_ms = candidates.iter().map(|candidate| candidate.ts_ms).max();
    let calibration_report = build_history_calibration_report(state, requested_symbol, &candidates);

    let (resolved_markout_evidence_count, unresolved_candidate_count, calibration_blocked_reasons) =
        match calibration_report.as_ref() {
            Some(report) => (
                report.sample_status.resolved_markout_evidence_count,
                report.sample_status.unresolved_markout_count,
                report.sample_status.blocked_reasons.clone(),
            ),
            None => (
                0,
                current_candidates,
                vec!["candidate_history_empty".to_string()],
            ),
        };
    let calibration_ready = calibration_report
        .as_ref()
        .is_some_and(|report| report.sample_status.enough_data);

    WhaleFlowCandidateHistoryStatusResponse {
        read_only: true,
        analysis_only: true,
        execution_enabled: false,
        runtime_modified: false,
        selected_symbol: requested_symbol.to_string(),
        retention_mode: WHALE_FLOW_CANDIDATE_HISTORY_RETENTION_MODE.to_string(),
        durable_storage_enabled: false,
        database_write_enabled: false,
        jsonl_write_enabled: false,
        sqlite_write_enabled: false,
        archive_write_enabled: false,
        current_candidates,
        max_candidates: snapshot.max_candidates,
        oldest_candidate_at_ms,
        latest_candidate_at_ms,
        deduplicated_count: snapshot.deduplicated_count,
        evicted_count: snapshot.evicted_count,
        recorded_count: snapshot.recorded_count,
        resolved_markout_evidence_count,
        unresolved_candidate_count,
        not_enough_data_count: unresolved_candidate_count,
        min_candidates_required: calibration_min_candidates_required(),
        min_resolved_evidence_required: calibration_min_resolved_evidence_required(),
        max_not_enough_data_rate_for_tuning: calibration_max_not_enough_data_rate_for_tuning(),
        calibration_ready,
        calibration_blocked_reasons,
        operator_notes: build_operator_notes(calibration_ready),
    }
}

fn build_history_recent(
    state: &AppState,
    requested_symbol: &str,
) -> WhaleFlowCandidateHistoryRecentResponse {
    let candidates = state
        .whale_flow_candidate_history_service()
        .recent_candidates(requested_symbol);
    let markout_recent = build_markout_recent(state, requested_symbol);
    let items = candidates
        .iter()
        .map(|candidate| build_history_item(candidate, &markout_recent))
        .collect::<Vec<_>>();
    WhaleFlowCandidateHistoryRecentResponse {
        read_only: true,
        analysis_only: true,
        execution_enabled: false,
        runtime_modified: false,
        selected_symbol: requested_symbol.to_string(),
        retention_mode: WHALE_FLOW_CANDIDATE_HISTORY_RETENTION_MODE.to_string(),
        status: if items.is_empty() {
            "empty_candidate_history"
        } else {
            "candidate_history_available"
        }
        .to_string(),
        items,
        operator_notes: if candidates.is_empty() {
            vec!["No whale candidates have been recorded in bounded memory yet.".to_string()]
        } else {
            build_operator_notes(false)
        },
    }
}

fn build_history_calibration_report(
    state: &AppState,
    requested_symbol: &str,
    candidates: &[WhaleFlowCandidate],
) -> Option<WhaleFlowCalibrationReportResponse> {
    if candidates.is_empty() {
        return None;
    }

    let mut whale_flow_recent = build_current_whale_flow_report(state, requested_symbol);
    whale_flow_recent.candidates = candidates.to_vec();
    whale_flow_recent.history_baseline_mode = "whale_candidate_history".to_string();
    whale_flow_recent
        .warnings
        .push("Calibration is using bounded in-memory whale candidate history.".to_string());
    let markout_recent = build_markout_recent(state, requested_symbol);
    let history_status = state.signal_history_service().status();
    Some(build_whale_flow_threshold_calibration_report(
        requested_symbol,
        &whale_flow_recent,
        &markout_recent,
        &history_status,
    ))
}

fn build_history_item(
    candidate: &WhaleFlowCandidate,
    markout_recent: &crate::types::toxic_markout::ToxicMarkoutRecentResponse,
) -> WhaleFlowCandidateHistoryItem {
    let (outcome_status, markout_status) =
        resolve_whale_candidate_markout(candidate, markout_recent);
    WhaleFlowCandidateHistoryItem {
        candidate_id: candidate.candidate_id.clone(),
        symbol: candidate.symbol.clone(),
        classification: candidate_type_key(candidate.candidate_type).to_string(),
        window_ms: candidate.window_ms,
        volume_btc: candidate.volume_btc,
        direction_bias: direction_key(candidate.direction).to_string(),
        direction_ratio: candidate.direction_bias,
        relative_volume_multiple: candidate.historical_volume_ratio,
        venue_confluence_count: candidate.same_direction_venues,
        baseline_source: baseline_source_key(candidate).to_string(),
        data_quality: candidate.diagnostics.data_quality.clone(),
        created_at_ms: candidate.ts_ms,
        outcome_status: outcome_status.to_string(),
        markout_status: markout_status.to_string(),
    }
}

fn build_operator_notes(calibration_ready: bool) -> Vec<String> {
    let mut notes = vec![
        "Whale candidate history is bounded in-memory only.".to_string(),
        "History is not durable and may be lost after restart.".to_string(),
        "No DB write. No file write. No persistent whale history write.".to_string(),
        "Calibration readiness depends on resolved markout evidence, not candidate count alone."
            .to_string(),
    ];
    if !calibration_ready {
        notes.push(
            "Calibration not ready until the resolved evidence gate is satisfied.".to_string(),
        );
    }
    notes
}

fn candidate_type_key(
    candidate_type: crate::types::whale_flow_signal::WhaleFlowCandidateType,
) -> &'static str {
    match candidate_type {
        crate::types::whale_flow_signal::WhaleFlowCandidateType::AggressiveBuy => "aggressive_buy",
        crate::types::whale_flow_signal::WhaleFlowCandidateType::AggressiveSell => {
            "aggressive_sell"
        }
        crate::types::whale_flow_signal::WhaleFlowCandidateType::Absorption => "absorption",
        crate::types::whale_flow_signal::WhaleFlowCandidateType::LiquidationSweep => {
            "liquidation_sweep"
        }
        crate::types::whale_flow_signal::WhaleFlowCandidateType::Trap => "trap",
    }
}

fn direction_key(direction: crate::types::toxic_flow::ToxicSide) -> &'static str {
    match direction {
        crate::types::toxic_flow::ToxicSide::Buy => "buy",
        crate::types::toxic_flow::ToxicSide::Sell => "sell",
        crate::types::toxic_flow::ToxicSide::Neutral => "neutral",
    }
}

fn baseline_source_key(candidate: &WhaleFlowCandidate) -> &'static str {
    match candidate.historical_baseline_window_ms {
        Some(3_600_000) => "one_hour_normalized",
        Some(60_000) => "sixty_second_fallback",
        Some(_) => "longer_window_fallback",
        None => "insufficient_history",
    }
}
