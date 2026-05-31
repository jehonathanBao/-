use serde::{Deserialize, Serialize};

use crate::types::{
    toxic_governance_ledger::ToxicGovernanceDecisionKind,
    toxic_governance_proposal::{ToxicGovernanceProposalAction, ToxicGovernanceProposalByAction},
    toxic_governance_review_pack::ToxicGovernanceReviewPackDecisionSummary,
    toxic_weight_recommendation::ToxicWeightRecommendationKind,
};

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ToxicGovernanceSignoffPackItem {
    pub symbol: String,
    pub signal_type: String,
    pub recommended_action: ToxicWeightRecommendationKind,
    pub governance_decision: Option<ToxicGovernanceDecisionKind>,
    pub proposed_action: ToxicGovernanceProposalAction,
    pub proposal_status: String,
    pub signoff_recommendation: String,
    pub blocked_reason: Option<String>,
    pub sample_count: usize,
    pub aligned_ratio: f64,
    pub adverse_ratio: f64,
    pub neutral_ratio: f64,
    pub confidence: String,
    pub reason_codes: Vec<String>,
    pub evidence_summary: Vec<String>,
    pub governance_notes: Vec<String>,
    pub read_only: bool,
    pub proposal_only: bool,
    pub review_pack_only: bool,
    pub signoff_pack_only: bool,
    pub runtime_weight_modified: bool,
    pub config_modified: bool,
    pub runtime_modified: bool,
    pub auto_apply_enabled: bool,
    pub strategy_reloaded: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ToxicGovernanceSignoffPackSignalTypeSummary {
    pub signal_type: String,
    pub total_items: usize,
    pub ready_for_signoff_count: usize,
    pub hold_count: usize,
    pub by_action: ToxicGovernanceProposalByAction,
    pub by_decision: ToxicGovernanceReviewPackDecisionSummary,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ToxicGovernanceSignoffPackSymbolSummary {
    pub symbol: String,
    pub total_items: usize,
    pub ready_for_signoff_count: usize,
    pub hold_count: usize,
    pub by_action: ToxicGovernanceProposalByAction,
    pub by_decision: ToxicGovernanceReviewPackDecisionSummary,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ToxicGovernanceSignoffPackSummaryResponse {
    pub read_only: bool,
    pub proposal_only: bool,
    pub review_pack_only: bool,
    pub signoff_pack_only: bool,
    pub runtime_weight_modified: bool,
    pub config_modified: bool,
    pub runtime_modified: bool,
    pub auto_apply_enabled: bool,
    pub strategy_reloaded: bool,
    pub mode: String,
    pub selected_symbol: String,
    pub status: String,
    pub ready_for_manual_signoff: bool,
    pub blocked_reasons: Vec<String>,
    pub warnings: Vec<String>,
    pub total_items: usize,
    pub ready_for_signoff_count: usize,
    pub hold_count: usize,
    pub by_action: ToxicGovernanceProposalByAction,
    pub by_decision: ToxicGovernanceReviewPackDecisionSummary,
    pub recent_governance_notes: Vec<String>,
    pub items: Vec<ToxicGovernanceSignoffPackItem>,
    pub by_signal_type: Vec<ToxicGovernanceSignoffPackSignalTypeSummary>,
    pub by_symbol: Vec<ToxicGovernanceSignoffPackSymbolSummary>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ToxicGovernanceSignoffPackStatusResponse {
    pub read_only: bool,
    pub proposal_only: bool,
    pub review_pack_only: bool,
    pub signoff_pack_only: bool,
    pub runtime_weight_modified: bool,
    pub config_modified: bool,
    pub runtime_modified: bool,
    pub auto_apply_enabled: bool,
    pub strategy_reloaded: bool,
    pub enabled: bool,
    pub mode: String,
    pub selected_symbol: String,
    pub status: String,
    pub ready_for_manual_signoff: bool,
    pub blocked_reasons: Vec<String>,
    pub total_items: usize,
    pub safety_boundary: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ToxicGovernanceSignoffPackExportResponse {
    pub read_only: bool,
    pub proposal_only: bool,
    pub review_pack_only: bool,
    pub signoff_pack_only: bool,
    pub runtime_weight_modified: bool,
    pub config_modified: bool,
    pub runtime_modified: bool,
    pub auto_apply_enabled: bool,
    pub strategy_reloaded: bool,
    pub mode: String,
    pub selected_symbol: String,
    pub status: String,
    pub ready_for_manual_signoff: bool,
    pub blocked_reasons: Vec<String>,
    pub total_items: usize,
    pub ready_for_signoff_count: usize,
    pub hold_count: usize,
    pub by_action: ToxicGovernanceProposalByAction,
    pub by_decision: ToxicGovernanceReviewPackDecisionSummary,
    pub recent_governance_notes: Vec<String>,
    pub items: Vec<ToxicGovernanceSignoffPackItem>,
    pub markdown_report: String,
}
