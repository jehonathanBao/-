use serde::{Deserialize, Serialize};

use crate::types::toxic_weight_recommendation::ToxicWeightRecommendationKind;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ToxicGovernanceDecisionKind {
    AcceptRecommendation,
    RejectRecommendation,
    WatchMore,
    NeedsMoreSamples,
    SuppressForNow,
    EscalateReview,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ToxicGovernanceDecision {
    pub id: String,
    pub symbol: String,
    pub signal_type: String,
    pub recommendation: ToxicWeightRecommendationKind,
    pub decision: ToxicGovernanceDecisionKind,
    pub reviewer: String,
    pub reason: String,
    pub notes: String,
    pub confidence: f64,
    pub evidence_summary: Vec<String>,
    pub created_at_ms: u64,
    pub read_only: bool,
    pub governance_ledger_only: bool,
    pub runtime_weight_modified: bool,
    pub config_modified: bool,
    pub runtime_modified: bool,
    pub auto_apply_enabled: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ToxicGovernanceSymbolSummary {
    pub symbol: String,
    pub total_decisions: usize,
    pub accept_count: usize,
    pub reject_count: usize,
    pub watch_more_count: usize,
    pub needs_more_samples_count: usize,
    pub suppress_for_now_count: usize,
    pub escalate_review_count: usize,
    pub consensus_status: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ToxicGovernanceSignalTypeSummary {
    pub signal_type: String,
    pub total_decisions: usize,
    pub accept_count: usize,
    pub reject_count: usize,
    pub watch_more_count: usize,
    pub needs_more_samples_count: usize,
    pub suppress_for_now_count: usize,
    pub escalate_review_count: usize,
    pub consensus_status: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ToxicGovernanceLedgerSummaryResponse {
    pub read_only: bool,
    pub analysis_only: bool,
    pub execution_enabled: bool,
    pub manual_review_required: bool,
    pub governance_ledger_only: bool,
    pub runtime_weight_modified: bool,
    pub config_modified: bool,
    pub runtime_modified: bool,
    pub auto_apply_enabled: bool,
    pub strategy_reloaded: bool,
    pub mode: String,
    pub selected_symbol: String,
    pub status: String,
    pub governance_status: String,
    pub manual_review_decision_placeholder: String,
    pub evidence_lineage: Vec<String>,
    pub warnings: Vec<String>,
    pub total_decisions: usize,
    pub accept_count: usize,
    pub reject_count: usize,
    pub watch_more_count: usize,
    pub needs_more_samples_count: usize,
    pub suppress_for_now_count: usize,
    pub escalate_review_count: usize,
    pub consensus_status: String,
    pub recent_governance_notes: Vec<String>,
    pub decisions: Vec<ToxicGovernanceDecision>,
    pub by_symbol: Vec<ToxicGovernanceSymbolSummary>,
    pub by_signal_type: Vec<ToxicGovernanceSignalTypeSummary>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ToxicGovernanceLedgerStatusResponse {
    pub read_only: bool,
    pub analysis_only: bool,
    pub execution_enabled: bool,
    pub manual_review_required: bool,
    pub governance_ledger_only: bool,
    pub runtime_weight_modified: bool,
    pub config_modified: bool,
    pub runtime_modified: bool,
    pub auto_apply_enabled: bool,
    pub strategy_reloaded: bool,
    pub enabled: bool,
    pub mode: String,
    pub selected_symbol: String,
    pub status: String,
    pub total_decisions: usize,
    pub consensus_status: String,
    pub safety_boundary: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ToxicGovernanceLedgerExportResponse {
    pub read_only: bool,
    pub analysis_only: bool,
    pub execution_enabled: bool,
    pub manual_review_required: bool,
    pub governance_ledger_only: bool,
    pub runtime_weight_modified: bool,
    pub config_modified: bool,
    pub runtime_modified: bool,
    pub auto_apply_enabled: bool,
    pub strategy_reloaded: bool,
    pub mode: String,
    pub selected_symbol: String,
    pub status: String,
    pub governance_status: String,
    pub manual_review_decision_placeholder: String,
    pub evidence_lineage: Vec<String>,
    pub total_decisions: usize,
    pub consensus_status: String,
    pub recent_governance_notes: Vec<String>,
    pub decisions: Vec<ToxicGovernanceDecision>,
    pub markdown_report: String,
}
