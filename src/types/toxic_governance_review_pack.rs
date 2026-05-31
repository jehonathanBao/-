use serde::{Deserialize, Serialize};

use crate::types::{
    toxic_governance_ledger::ToxicGovernanceDecisionKind,
    toxic_governance_proposal::{ToxicGovernanceProposalAction, ToxicGovernanceProposalByAction},
    toxic_weight_recommendation::ToxicWeightRecommendationKind,
};

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ToxicGovernanceReviewPackDecisionSummary {
    pub accepted_count: usize,
    pub rejected_count: usize,
    pub watch_more_count: usize,
    pub needs_more_samples_count: usize,
    pub suppress_for_now_count: usize,
    pub escalate_review_count: usize,
    pub pending_governance_review_count: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ToxicGovernanceReviewPackItem {
    pub symbol: String,
    pub signal_type: String,
    pub recommended_action: ToxicWeightRecommendationKind,
    pub governance_decision: Option<ToxicGovernanceDecisionKind>,
    pub proposed_action: ToxicGovernanceProposalAction,
    pub proposal_status: String,
    pub sample_count: usize,
    pub aligned_ratio: f64,
    pub adverse_ratio: f64,
    pub neutral_ratio: f64,
    pub confidence: String,
    pub reason_codes: Vec<String>,
    pub evidence_summary: Vec<String>,
    pub governance_notes: Vec<String>,
    pub manual_review_required: bool,
    pub read_only: bool,
    pub review_pack_only: bool,
    pub proposal_only: bool,
    pub runtime_weight_modified: bool,
    pub config_modified: bool,
    pub runtime_modified: bool,
    pub auto_apply_enabled: bool,
    pub strategy_reloaded: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ToxicGovernanceReviewPackSignalTypeSummary {
    pub signal_type: String,
    pub total_items: usize,
    pub by_action: ToxicGovernanceProposalByAction,
    pub by_decision: ToxicGovernanceReviewPackDecisionSummary,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ToxicGovernanceReviewPackSymbolSummary {
    pub symbol: String,
    pub total_items: usize,
    pub by_action: ToxicGovernanceProposalByAction,
    pub by_decision: ToxicGovernanceReviewPackDecisionSummary,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ToxicGovernanceReviewPackSummaryResponse {
    pub read_only: bool,
    pub proposal_only: bool,
    pub review_pack_only: bool,
    pub runtime_weight_modified: bool,
    pub config_modified: bool,
    pub runtime_modified: bool,
    pub auto_apply_enabled: bool,
    pub strategy_reloaded: bool,
    pub mode: String,
    pub selected_symbol: String,
    pub status: String,
    pub ready_for_manual_review: bool,
    pub warnings: Vec<String>,
    pub total_items: usize,
    pub by_action: ToxicGovernanceProposalByAction,
    pub by_decision: ToxicGovernanceReviewPackDecisionSummary,
    pub recent_governance_notes: Vec<String>,
    pub items: Vec<ToxicGovernanceReviewPackItem>,
    pub by_signal_type: Vec<ToxicGovernanceReviewPackSignalTypeSummary>,
    pub by_symbol: Vec<ToxicGovernanceReviewPackSymbolSummary>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ToxicGovernanceReviewPackStatusResponse {
    pub read_only: bool,
    pub proposal_only: bool,
    pub review_pack_only: bool,
    pub runtime_weight_modified: bool,
    pub config_modified: bool,
    pub runtime_modified: bool,
    pub auto_apply_enabled: bool,
    pub strategy_reloaded: bool,
    pub enabled: bool,
    pub mode: String,
    pub selected_symbol: String,
    pub status: String,
    pub ready_for_manual_review: bool,
    pub total_items: usize,
    pub safety_boundary: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ToxicGovernanceReviewPackExportResponse {
    pub read_only: bool,
    pub proposal_only: bool,
    pub review_pack_only: bool,
    pub runtime_weight_modified: bool,
    pub config_modified: bool,
    pub runtime_modified: bool,
    pub auto_apply_enabled: bool,
    pub strategy_reloaded: bool,
    pub mode: String,
    pub selected_symbol: String,
    pub status: String,
    pub ready_for_manual_review: bool,
    pub total_items: usize,
    pub by_action: ToxicGovernanceProposalByAction,
    pub by_decision: ToxicGovernanceReviewPackDecisionSummary,
    pub recent_governance_notes: Vec<String>,
    pub items: Vec<ToxicGovernanceReviewPackItem>,
    pub markdown_report: String,
}
