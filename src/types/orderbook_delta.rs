use serde::{Deserialize, Serialize};

use super::{
    market::Venue,
    orderbook_wall::OrderbookWallSide,
    toxic_flow::ToxicConfidence,
    toxic_signal::{
        ScoreBreakdown, SignalEvidence, ToxicChaseRisk, ToxicSignal, ToxicSignalDirection,
        ToxicSignalType,
    },
};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum OrderBookDeltaType {
    Add,
    Cancel,
    Amend,
    Fill,
    Reduce,
    Remove,
    Refill,
    SnapshotReset,
    Unknown,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum OrderBookDeltaEvidenceSource {
    NativeOrderEvent,
    InferredFromL2Delta,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct OrderBookDeltaEvent {
    pub venue: Venue,
    pub symbol: String,
    pub side: OrderbookWallSide,
    pub price: f64,
    pub qty_before: f64,
    pub qty_after: f64,
    pub delta_qty: f64,
    pub delta_type: OrderBookDeltaType,
    pub ts: i64,
    pub sequence: u64,
    pub order_id: Option<String>,
    pub lifetime_ms: Option<u64>,
    pub fill_qty: Option<f64>,
    pub cancel_qty: Option<f64>,
    pub evidence_source: OrderBookDeltaEvidenceSource,
    pub distance_to_touch_bps: Option<f64>,
    pub depth_before: Option<f64>,
    pub depth_after: Option<f64>,
}

impl OrderBookDeltaEvent {
    pub fn semantic_dedupe_key(&self) -> String {
        format!(
            "{}:{}:{:?}:{:.2}:{}:{}",
            self.venue.as_key(),
            self.symbol,
            self.side,
            self.price,
            delta_type_key(self.delta_type),
            self.sequence
        )
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ManipulationSignalType {
    SpoofingCandidate,
    LayeringCandidate,
    IcebergCandidate,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ManipulationResolutionStatus {
    Candidate,
    DataInsufficient,
    ConfirmedByReplay,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ManipulationScoreBreakdown {
    pub toxicity_score: u8,
    pub confidence_score: u8,
    pub data_quality_score: u8,
    pub markout_evidence_score: u8,
    pub venue_reliability_score: u8,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ManipulationEvidenceChecklist {
    pub large_wall_appeared: bool,
    pub near_touch: bool,
    pub low_fill_participation: bool,
    pub wall_removed: bool,
    pub post_remove_markout: bool,
    pub opposite_aggressive_flow: bool,
    pub synchronized_levels: bool,
    pub high_cancel_ratio: bool,
    pub repeated_refill: bool,
    pub stable_refill_interval: bool,
    pub hidden_liquidity_ratio: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ManipulationSignalV2 {
    pub signal_id: String,
    pub detector_version: String,
    pub signal_type: ManipulationSignalType,
    pub venue: Venue,
    pub symbol: String,
    pub side: OrderbookWallSide,
    pub window_ms: u64,
    pub observed_start_ms: i64,
    pub observed_end_ms: i64,
    pub price: Option<f64>,
    pub add_qty: f64,
    pub cancel_qty: f64,
    pub fill_qty: f64,
    pub cancel_to_trade_ratio: Option<f64>,
    pub depth_before: Option<f64>,
    pub depth_after: Option<f64>,
    pub price_impact_bps: Option<f64>,
    pub markout_1s_bps: Option<f64>,
    pub markout_5s_bps: Option<f64>,
    pub markout_30s_bps: Option<f64>,
    pub risk_score: u8,
    pub confidence: ToxicConfidence,
    pub score_breakdown: ManipulationScoreBreakdown,
    pub data_quality: String,
    pub dedupe_key: String,
    pub raw_evidence_links: Vec<String>,
    pub resolution_status: ManipulationResolutionStatus,
    pub evidence_source: OrderBookDeltaEvidenceSource,
    pub evidence_checklist: ManipulationEvidenceChecklist,
    pub reasons: Vec<String>,
    pub read_only: bool,
}

#[derive(Debug, Clone, Copy)]
pub struct VenueReliability {
    pub venue: Venue,
    pub reliability_score: f64,
}

impl ManipulationSignalV2 {
    pub fn to_toxic_signal(&self) -> ToxicSignal {
        ToxicSignal {
            signal_id: self.signal_id.clone(),
            symbol: self.symbol.clone(),
            ts_ms: self.observed_end_ms.max(0) as u64,
            signal_type: match self.signal_type {
                ManipulationSignalType::SpoofingCandidate => ToxicSignalType::SpoofingCandidate,
                ManipulationSignalType::LayeringCandidate => ToxicSignalType::LayeringCandidate,
                ManipulationSignalType::IcebergCandidate => ToxicSignalType::IcebergCandidate,
            },
            direction: ToxicSignalDirection::Neutral,
            toxicity_score: self.risk_score,
            confidence: self.confidence,
            primary_reason: self
                .reasons
                .first()
                .cloned()
                .unwrap_or_else(|| "toxic order candidate evidence".to_string()),
            reason: self.reasons.clone(),
            supporting_evidence: Vec::new(),
            invalidation_price: None,
            suggested_stop_distance_usd: None,
            chase_risk: ToxicChaseRisk::Medium,
            no_trade_reasons: vec!["candidate_only_not_confirmed".to_string()],
            linked_active_trade_signal_ids: Vec::new(),
            linked_liquidation_signal_ids: Vec::new(),
            linked_wall_lifecycle_signal_ids: Vec::new(),
            linked_wall_interpretation_signal_ids: Vec::new(),
            linked_structural_signal_ids: Vec::new(),
            read_only: true,
            detector_version: Some(self.detector_version.clone()),
            score_breakdown: Some(ScoreBreakdown {
                toxicity_score: self.score_breakdown.toxicity_score as f64,
                confidence: self.score_breakdown.confidence_score as f64,
                data_quality: self.score_breakdown.data_quality_score as f64,
                markout_evidence: self.score_breakdown.markout_evidence_score as f64,
                liquidity_impact: self
                    .depth_before
                    .zip(self.depth_after)
                    .map(|(before, after)| ((before - after).max(0.0) / before.max(1.0)) * 100.0)
                    .unwrap_or(0.0),
            }),
            evidence: Some(SignalEvidence {
                venue: self.venue.as_key().to_string(),
                symbol: self.symbol.clone(),
                window_ms: self.window_ms as i64,
                observed_start_ms: self.observed_start_ms,
                observed_end_ms: self.observed_end_ms,
                add_qty: self.add_qty,
                cancel_qty: self.cancel_qty,
                fill_qty: self.fill_qty,
                cancel_to_trade_ratio: self.cancel_to_trade_ratio,
                depth_before: self.depth_before,
                depth_after: self.depth_after,
                depth_impact: self
                    .depth_before
                    .zip(self.depth_after)
                    .map(|(before, after)| (before - after).max(0.0) / before.max(1.0)),
                price_impact_bps: self.price_impact_bps,
                markout_1s_bps: self.markout_1s_bps,
                markout_5s_bps: self.markout_5s_bps,
                markout_30s_bps: self.markout_30s_bps,
                raw_evidence_links: self.raw_evidence_links.clone(),
            }),
            data_quality: Some(self.score_breakdown.data_quality_score as f64),
            dedupe_key: Some(self.dedupe_key.clone()),
            resolution_status: Some("candidate".to_string()),
        }
    }
}

pub fn delta_type_key(delta_type: OrderBookDeltaType) -> &'static str {
    match delta_type {
        OrderBookDeltaType::Add => "add",
        OrderBookDeltaType::Cancel => "cancel",
        OrderBookDeltaType::Amend => "amend",
        OrderBookDeltaType::Fill => "fill",
        OrderBookDeltaType::Reduce => "reduce",
        OrderBookDeltaType::Remove => "remove",
        OrderBookDeltaType::Refill => "refill",
        OrderBookDeltaType::SnapshotReset => "snapshot_reset",
        OrderBookDeltaType::Unknown => "unknown",
    }
}
