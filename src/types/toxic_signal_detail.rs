use serde::{Deserialize, Serialize};

use super::{
    toxic_governance_ledger::ToxicGovernanceDecision, toxic_markout::ToxicMarkoutSignal,
    toxic_quality_scorecard::ToxicQualityScorecardBucket, toxic_replay::ToxicReplaySignalSummary,
    toxic_signal::ToxicSignal, toxic_signal_group::ToxicSignalGroup,
    toxic_weight_recommendation::ToxicWeightRecommendationItem,
};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ToxicSignalDetailOperatorAction {
    ReviewEvidence,
    ReviewMarkout,
    ReviewQuality,
    WatchSignalOnly,
    NoTradeWarning,
    NeedsMoreData,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ToxicSignalDetailSource {
    pub inbox_available: bool,
    pub group_available: bool,
    pub group_id: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ToxicSignalDetailTimelineStage {
    pub stage: String,
    pub label: String,
    pub available: bool,
    pub summary: String,
    pub timestamp_ms: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ToxicSignalDetailEvidence {
    pub fusion: Option<ToxicSignal>,
    pub replay: Option<ToxicReplaySignalSummary>,
    pub markout: Option<ToxicMarkoutSignal>,
    pub quality: Option<ToxicQualityScorecardBucket>,
    pub recommendation: Option<ToxicWeightRecommendationItem>,
    pub governance: Option<ToxicGovernanceDecision>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ToxicSignalDetailOperatorNarrative {
    pub why_signal_fired: Vec<String>,
    pub what_confirmed_it: Vec<String>,
    pub what_conflicted: Vec<String>,
    pub why_no_execution: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ToxicSignalDetailPayload {
    pub signal_id: String,
    pub symbol: String,
    pub signal_kind: String,
    pub direction_bias: String,
    pub severity: String,
    pub confidence: f64,
    pub created_at_ms: u64,
    pub source: ToxicSignalDetailSource,
    pub timeline: Vec<ToxicSignalDetailTimelineStage>,
    pub evidence: ToxicSignalDetailEvidence,
    pub operator_narrative: ToxicSignalDetailOperatorNarrative,
    pub operator_action: ToxicSignalDetailOperatorAction,
    pub no_execution_reason: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ToxicSignalDetailMemberSummary {
    pub signal_id: String,
    pub severity: String,
    pub confidence: f64,
    pub created_at_ms: u64,
    pub operator_action: ToxicSignalDetailOperatorAction,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ToxicSignalGroupDrilldownPayload {
    pub representative_signal: ToxicSignalDetailPayload,
    pub group: ToxicSignalGroup,
    pub members: Vec<ToxicSignalDetailMemberSummary>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ToxicSignalDetailStatusResponse {
    pub read_only: bool,
    pub runtime_modified: bool,
    pub analysis_only: bool,
    pub execution_enabled: bool,
    pub manual_review_required: bool,
    pub runtime_weight_modified: bool,
    pub config_modified: bool,
    pub enabled: bool,
    pub mode: String,
    pub selected_symbol: String,
    pub status: String,
    pub signal_count: usize,
    pub group_count: usize,
    pub last_signal_at_ms: Option<u64>,
    pub safety_boundary: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ToxicSignalDetailResponse {
    pub read_only: bool,
    pub runtime_modified: bool,
    pub analysis_only: bool,
    pub execution_enabled: bool,
    pub manual_review_required: bool,
    pub runtime_weight_modified: bool,
    pub config_modified: bool,
    pub mode: String,
    pub selected_symbol: String,
    pub available: bool,
    pub reason: Option<String>,
    pub detail: Option<ToxicSignalDetailPayload>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ToxicSignalDetailGroupResponse {
    pub read_only: bool,
    pub runtime_modified: bool,
    pub analysis_only: bool,
    pub execution_enabled: bool,
    pub manual_review_required: bool,
    pub runtime_weight_modified: bool,
    pub config_modified: bool,
    pub mode: String,
    pub selected_symbol: String,
    pub available: bool,
    pub reason: Option<String>,
    pub detail: Option<ToxicSignalGroupDrilldownPayload>,
}
