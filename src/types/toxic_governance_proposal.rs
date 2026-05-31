use serde::{Deserialize, Serialize};

use crate::types::{
    toxic_governance_ledger::ToxicGovernanceDecisionKind,
    toxic_weight_recommendation::ToxicWeightRecommendationKind,
};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ToxicGovernanceProposalAction {
    Keep,
    SlightUpgradeCandidate,
    SlightDowngradeCandidate,
    DowngradeCandidate,
    NoTradeOnlyCandidate,
    DisableCandidate,
    NeedsMoreSamples,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ToxicGovernanceProposalItem {
    pub symbol: String,
    pub signal_type: String,
    pub recommended_action: ToxicWeightRecommendationKind,
    pub governance_decision: Option<ToxicGovernanceDecisionKind>,
    pub proposed_action: ToxicGovernanceProposalAction,
    pub sample_count: usize,
    pub aligned_ratio: f64,
    pub adverse_ratio: f64,
    pub neutral_ratio: f64,
    pub confidence: String,
    pub reason_codes: Vec<String>,
    pub evidence_summary: Vec<String>,
    pub governance_notes: Vec<String>,
    pub proposal_status: String,
    pub manual_review_required: bool,
    pub read_only: bool,
    pub proposal_only: bool,
    pub runtime_weight_modified: bool,
    pub config_modified: bool,
    pub runtime_modified: bool,
    pub auto_apply_enabled: bool,
    pub strategy_reloaded: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ToxicGovernanceProposalByAction {
    pub keep: usize,
    pub slight_upgrade_candidate: usize,
    pub slight_downgrade_candidate: usize,
    pub downgrade_candidate: usize,
    pub no_trade_only_candidate: usize,
    pub disable_candidate: usize,
    pub needs_more_samples: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ToxicGovernanceProposalSignalTypeSummary {
    pub signal_type: String,
    pub total_proposals: usize,
    pub by_action: ToxicGovernanceProposalByAction,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ToxicGovernanceProposalSymbolSummary {
    pub symbol: String,
    pub total_proposals: usize,
    pub by_action: ToxicGovernanceProposalByAction,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ToxicGovernanceProposalSummaryResponse {
    pub read_only: bool,
    pub proposal_only: bool,
    pub runtime_weight_modified: bool,
    pub config_modified: bool,
    pub runtime_modified: bool,
    pub auto_apply_enabled: bool,
    pub strategy_reloaded: bool,
    pub mode: String,
    pub selected_symbol: String,
    pub status: String,
    pub warnings: Vec<String>,
    pub total_proposals: usize,
    pub by_action: ToxicGovernanceProposalByAction,
    pub items: Vec<ToxicGovernanceProposalItem>,
    pub by_signal_type: Vec<ToxicGovernanceProposalSignalTypeSummary>,
    pub by_symbol: Vec<ToxicGovernanceProposalSymbolSummary>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ToxicGovernanceProposalStatusResponse {
    pub read_only: bool,
    pub proposal_only: bool,
    pub runtime_weight_modified: bool,
    pub config_modified: bool,
    pub runtime_modified: bool,
    pub auto_apply_enabled: bool,
    pub strategy_reloaded: bool,
    pub enabled: bool,
    pub mode: String,
    pub selected_symbol: String,
    pub status: String,
    pub total_proposals: usize,
    pub safety_boundary: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ToxicGovernanceProposalExportResponse {
    pub read_only: bool,
    pub proposal_only: bool,
    pub runtime_weight_modified: bool,
    pub config_modified: bool,
    pub runtime_modified: bool,
    pub auto_apply_enabled: bool,
    pub strategy_reloaded: bool,
    pub mode: String,
    pub selected_symbol: String,
    pub status: String,
    pub total_proposals: usize,
    pub by_action: ToxicGovernanceProposalByAction,
    pub items: Vec<ToxicGovernanceProposalItem>,
    pub markdown_report: String,
}
