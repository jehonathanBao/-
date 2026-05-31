use crate::{
    toxicity::whale_flow_monitor::{analyze_whale_flow, WhaleFlowAnalysisInputs},
    types::whale_flow_signal::{WhaleFlowRecentResponse, WhaleFlowStatusResponse},
};

pub fn build_whale_flow_recent(inputs: &WhaleFlowAnalysisInputs<'_>) -> WhaleFlowRecentResponse {
    analyze_whale_flow(inputs)
}

pub fn build_whale_flow_status(inputs: &WhaleFlowAnalysisInputs<'_>) -> WhaleFlowStatusResponse {
    let recent = build_whale_flow_recent(inputs);
    WhaleFlowStatusResponse {
        read_only: true,
        runtime_modified: false,
        analysis_only: true,
        execution_enabled: false,
        enabled: true,
        mode: "analysis_only".to_string(),
        selected_symbol: inputs.requested_symbol.to_string(),
        status: recent.status.clone(),
        candidate_count: recent.candidates.len(),
        last_candidate_at_ms: recent
            .candidates
            .iter()
            .map(|candidate| candidate.ts_ms)
            .max(),
        lagged_events: recent.lagged_events,
        dropped_events: recent.dropped_events,
        flow_windows_populated: recent.flow_windows_populated,
        connected_venues: recent.connected_venues,
        data_quality: recent.data_quality.clone(),
        venue_coverage: recent.venue_coverage.clone(),
        baseline_quality: recent.baseline_quality.clone(),
        thresholds: recent.thresholds.clone(),
        safety_boundary: vec![
            "readOnly=true".to_string(),
            "analysisOnly=true".to_string(),
            "executionEnabled=false".to_string(),
            "No order placement".to_string(),
            "No wallet".to_string(),
            "No signing".to_string(),
            "No runtime mutation".to_string(),
            "No DB / JSONL / SQLite / archive write".to_string(),
        ],
    }
}
