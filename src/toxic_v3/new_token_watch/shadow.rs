//! Read-only outcome classification for later L2 model calibration.
//!
//! It deliberately contains no notifier, order, or exchange dependency.

use std::collections::{BTreeMap, BTreeSet};

use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

use super::intent::{IntentAssessment, IntentState};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ShadowOutcomeLabel {
    InsufficientEvidence,
    Aligned,
    Conflicted,
    Neutral,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ShadowOutcome {
    pub label: ShadowOutcomeLabel,
    pub shadow_only: bool,
    pub discord_eligible: bool,
    pub execution_enabled: bool,
    pub reason: String,
}

/// A single delayed, read-only observation of a public-L2 intent event.
/// It is deliberately not a trading signal and never drives notifications.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ShadowOutcomeObservation {
    pub event_id: String,
    pub symbol: String,
    pub intent_state: IntentState,
    pub observed_at_ms: i64,
    pub horizon_seconds: u32,
    pub entry_price: f64,
    pub observed_price: f64,
    pub price_move_bps: f64,
    pub outcome: ShadowOutcome,
}

#[derive(Debug, Clone)]
struct PendingShadowOutcome {
    event_id: String,
    symbol: String,
    intent_state: IntentState,
    observed_at_ms: i64,
    entry_price: f64,
    evaluated_horizons: BTreeSet<u32>,
}

/// Holds only in-memory, delayed shadow observations. The runtime persists
/// compact completed observations; no raw depth frames are retained here.
#[derive(Debug, Default)]
pub struct ShadowOutcomeTracker {
    active_event_by_symbol: BTreeMap<String, String>,
    pending: BTreeMap<String, PendingShadowOutcome>,
}

impl ShadowOutcomeTracker {
    pub fn observe_intent(
        &mut self,
        raw_symbol: &str,
        observed_at_ms: i64,
        entry_price: f64,
        intent: &IntentAssessment,
    ) {
        if !entry_price.is_finite()
            || entry_price <= 0.0
            || !intent.intent_assessment_available
            || !matches!(
                intent.state,
                IntentState::BidPressure | IntentState::AskPressure
            )
        {
            self.active_event_by_symbol
                .remove(&raw_symbol.trim().to_ascii_uppercase());
            return;
        }
        let symbol = raw_symbol.trim().to_ascii_uppercase();
        if let Some(event_id) = self.active_event_by_symbol.get(&symbol) {
            if self
                .pending
                .get(event_id)
                .is_some_and(|pending| pending.intent_state == intent.state)
            {
                return;
            }
        }
        let event_id = stable_event_id(&symbol, observed_at_ms, intent.state);
        self.pending.insert(
            event_id.clone(),
            PendingShadowOutcome {
                event_id: event_id.clone(),
                symbol: symbol.clone(),
                intent_state: intent.state,
                observed_at_ms,
                entry_price,
                evaluated_horizons: BTreeSet::new(),
            },
        );
        self.active_event_by_symbol.insert(symbol, event_id);
    }

    pub fn observe_price(
        &mut self,
        raw_symbol: &str,
        now_ms: i64,
        observed_price: f64,
    ) -> Vec<ShadowOutcomeObservation> {
        if !observed_price.is_finite() || observed_price <= 0.0 {
            return vec![];
        }
        let symbol = raw_symbol.trim().to_ascii_uppercase();
        let horizons = [10_u32, 30, 120, 300];
        let mut observations = Vec::new();
        let mut completed = Vec::new();
        for pending in self
            .pending
            .values_mut()
            .filter(|pending| pending.symbol == symbol)
        {
            for horizon_seconds in horizons {
                if pending.evaluated_horizons.contains(&horizon_seconds)
                    || now_ms < pending.observed_at_ms + i64::from(horizon_seconds) * 1_000
                {
                    continue;
                }
                let price_move_bps =
                    (observed_price - pending.entry_price) / pending.entry_price * 10_000.0;
                let intent = IntentAssessment {
                    state: pending.intent_state,
                    confidence: 0.0,
                    intent_assessment_available: true,
                    reason: "l2_shadow_event".to_string(),
                    evidence: vec!["public_l2_orderbook".to_string()],
                    read_only: true,
                };
                observations.push(ShadowOutcomeObservation {
                    event_id: pending.event_id.clone(),
                    symbol: pending.symbol.clone(),
                    intent_state: pending.intent_state,
                    observed_at_ms: pending.observed_at_ms,
                    horizon_seconds,
                    entry_price: pending.entry_price,
                    observed_price,
                    price_move_bps,
                    outcome: evaluate_shadow_outcome(&intent, price_move_bps),
                });
                pending.evaluated_horizons.insert(horizon_seconds);
            }
            if pending.evaluated_horizons.len() == horizons.len() {
                completed.push(pending.event_id.clone());
            }
        }
        for event_id in completed {
            if self.active_event_by_symbol.get(&symbol) == Some(&event_id) {
                self.active_event_by_symbol.remove(&symbol);
            }
            self.pending.remove(&event_id);
        }
        observations
    }
}

fn stable_event_id(symbol: &str, observed_at_ms: i64, state: IntentState) -> String {
    let mut hasher = Sha256::new();
    hasher.update(symbol.as_bytes());
    hasher.update(b"|");
    hasher.update(observed_at_ms.to_le_bytes());
    hasher.update(b"|");
    hasher.update(format!("{state:?}").as_bytes());
    format!("ntl2-{:x}", hasher.finalize())
}

pub fn evaluate_shadow_outcome(intent: &IntentAssessment, price_move_bps: f64) -> ShadowOutcome {
    let label = if !intent.intent_assessment_available {
        ShadowOutcomeLabel::InsufficientEvidence
    } else if price_move_bps.abs() < 5.0 {
        ShadowOutcomeLabel::Neutral
    } else if matches!(intent.state, IntentState::BidPressure) && price_move_bps > 0.0
        || matches!(intent.state, IntentState::AskPressure) && price_move_bps < 0.0
    {
        ShadowOutcomeLabel::Aligned
    } else {
        ShadowOutcomeLabel::Conflicted
    };
    ShadowOutcome {
        label,
        shadow_only: true,
        discord_eligible: false,
        execution_enabled: false,
        reason: "l2_shadow_evaluation_only".to_string(),
    }
}
