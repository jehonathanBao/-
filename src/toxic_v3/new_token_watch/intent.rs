//! Conservative L2 intent assessment.
//!
//! The result is a public-orderbook interpretation, never a claim about a
//! specific trader, market maker, or whale.

use serde::{Deserialize, Serialize};

use super::l2::{OrderBookMetrics, OrderBookReadiness};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum IntentState {
    Unavailable,
    Neutral,
    BidPressure,
    AskPressure,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct IntentAssessment {
    pub state: IntentState,
    pub confidence: f64,
    pub intent_assessment_available: bool,
    pub reason: String,
    pub evidence: Vec<String>,
    pub read_only: bool,
}

/// Small state machine that prevents a single L2 frame from becoming a
/// directional claim.  It is reset on every stale or sequence-gap state.
#[derive(Debug, Default)]
pub struct IntentFsm {
    candidate: Option<IntentState>,
    consecutive: u8,
}

impl IntentFsm {
    pub fn observe(&mut self, metrics: &OrderBookMetrics) -> IntentAssessment {
        let raw = evaluate_intent(metrics);
        if !raw.intent_assessment_available {
            self.candidate = None;
            self.consecutive = 0;
            return raw;
        }
        if raw.state == IntentState::Neutral {
            self.candidate = None;
            self.consecutive = 0;
            return raw;
        }
        if self.candidate == Some(raw.state) {
            self.consecutive = self.consecutive.saturating_add(1);
        } else {
            self.candidate = Some(raw.state);
            self.consecutive = 1;
        }
        if self.consecutive < 2 {
            return IntentAssessment {
                state: IntentState::Unavailable,
                confidence: 0.0,
                intent_assessment_available: false,
                reason: "intent_warmup".to_string(),
                evidence: vec!["awaiting_consecutive_l2_observations".to_string()],
                read_only: true,
            };
        }
        raw
    }
}

pub fn evaluate_intent(metrics: &OrderBookMetrics) -> IntentAssessment {
    if metrics.readiness != OrderBookReadiness::Ready || !metrics.orderbook_evidence_available {
        return IntentAssessment {
            state: IntentState::Unavailable,
            confidence: 0.0,
            intent_assessment_available: false,
            reason: "orderbook_not_ready".to_string(),
            evidence: vec!["l2_evidence_unavailable".to_string()],
            read_only: true,
        };
    }
    let confidence = metrics.imbalance.abs().clamp(0.0, 1.0);
    let state = if metrics.imbalance >= 0.18 {
        IntentState::BidPressure
    } else if metrics.imbalance <= -0.18 {
        IntentState::AskPressure
    } else {
        IntentState::Neutral
    };
    IntentAssessment {
        state,
        confidence,
        intent_assessment_available: true,
        reason: "l2_top_depth_imbalance".to_string(),
        evidence: vec![
            "public_l2_orderbook".to_string(),
            "probabilistic_interpretation".to_string(),
        ],
        read_only: true,
    }
}
