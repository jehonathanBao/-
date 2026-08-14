//! Read-only detector for persistent, sub-threshold directional flow.
//!
//! Shadow is deliberately separate from Impact and Behavior.  It is a
//! candidate lane only: a candidate is never promoted to an execution signal
//! or used to place an order.

use std::collections::HashMap;

use serde::{Deserialize, Serialize};

const MAX_GAP_MS: i64 = 120_000;
const MIN_DOMINANCE: f64 = 0.55;
const MAX_PRICE_MOVE_PCT: f64 = 0.12;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ShadowState {
    Suspect,
    Watching,
    Corroborated,
    Invalidated,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ShadowDirection {
    Buy,
    Sell,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ShadowObservation {
    pub symbol: String,
    pub ts: i64,
    pub total_volume_btc: f64,
    pub net_volume_btc: f64,
    pub high_threshold_btc: f64,
    pub price_move_pct: Option<f64>,
    pub oi_change_pct: Option<f64>,
    pub data_quality: u8,
    pub multi_exchange_confirmed: bool,
    pub live_liquidation_btc: f64,
    pub trade_count: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ShadowCandidate {
    pub lane: String,
    pub symbol: String,
    pub direction: ShadowDirection,
    pub state: ShadowState,
    pub first_seen_ts: i64,
    pub last_seen_ts: i64,
    pub observation_count: u32,
    pub cumulative_volume_btc: f64,
    pub cumulative_net_volume_btc: f64,
    pub evidence: Vec<String>,
    pub invalidation_reason: Option<String>,
    pub read_only: bool,
    pub analysis_only: bool,
    pub execution_enabled: bool,
}

#[derive(Debug, Default)]
pub struct ShadowTracker {
    episodes: HashMap<(String, ShadowDirection), Episode>,
}

#[derive(Debug, Clone)]
struct Episode {
    first_seen_ts: i64,
    last_seen_ts: i64,
    observation_count: u32,
    cumulative_volume_btc: f64,
    cumulative_net_volume_btc: f64,
    oi_complete: bool,
    multi_exchange: bool,
    low_efficiency: bool,
    data_quality: u8,
}

impl ShadowTracker {
    pub fn observe(&mut self, observation: ShadowObservation) -> ShadowCandidate {
        let direction = if observation.net_volume_btc >= 0.0 {
            ShadowDirection::Buy
        } else {
            ShadowDirection::Sell
        };
        let key = (observation.symbol.clone(), direction);
        let hard_invalid = observation.total_volume_btc <= 0.0
            || observation.total_volume_btc >= observation.high_threshold_btc
            || observation.data_quality < 70
            || observation.live_liquidation_btc > 0.0
            || observation.net_volume_btc.abs() / observation.total_volume_btc < MIN_DOMINANCE;

        if let Some(previous) = self.episodes.get(&key) {
            if observation.ts.saturating_sub(previous.last_seen_ts) > MAX_GAP_MS {
                let candidate =
                    Self::invalidated(&observation, direction, previous, "stale_gap".to_string());
                self.episodes.remove(&key);
                return candidate;
            }
        }

        let episode = self.episodes.entry(key).or_insert_with(|| Episode {
            first_seen_ts: observation.ts,
            last_seen_ts: observation.ts,
            observation_count: 0,
            cumulative_volume_btc: 0.0,
            cumulative_net_volume_btc: 0.0,
            oi_complete: true,
            multi_exchange: true,
            low_efficiency: true,
            data_quality: observation.data_quality,
        });
        episode.last_seen_ts = observation.ts;
        episode.observation_count = episode.observation_count.saturating_add(1);
        episode.cumulative_volume_btc += observation.total_volume_btc.max(0.0);
        episode.cumulative_net_volume_btc += observation.net_volume_btc;
        episode.oi_complete &= observation.oi_change_pct.is_some();
        episode.multi_exchange &= observation.multi_exchange_confirmed;
        episode.low_efficiency &= observation
            .price_move_pct
            .map(|value| value.abs() <= MAX_PRICE_MOVE_PCT)
            .unwrap_or(false);
        episode.data_quality = episode.data_quality.min(observation.data_quality);

        if hard_invalid {
            return Self::invalidated(
                &observation,
                direction,
                episode,
                if observation.total_volume_btc >= observation.high_threshold_btc {
                    "above_high_volume_gate"
                } else if observation.live_liquidation_btc > 0.0 {
                    "liquidation_present"
                } else if observation.data_quality < 70 {
                    "low_data_quality"
                } else {
                    "insufficient_directional_dominance"
                }
                .to_string(),
            );
        }

        let state = if episode.observation_count >= 3
            && episode.oi_complete
            && episode.multi_exchange
            && episode.low_efficiency
            && episode.data_quality >= 70
        {
            ShadowState::Corroborated
        } else if episode.observation_count >= 2 {
            ShadowState::Watching
        } else {
            ShadowState::Suspect
        };
        Self::candidate(&observation, direction, episode, state, None)
    }

    fn candidate(
        observation: &ShadowObservation,
        direction: ShadowDirection,
        episode: &Episode,
        state: ShadowState,
        invalidation_reason: Option<String>,
    ) -> ShadowCandidate {
        let mut evidence = vec![
            "sub_high_volume".to_string(),
            "directional_persistence".to_string(),
        ];
        if episode.low_efficiency {
            evidence.push("low_price_efficiency".to_string());
        }
        if episode.oi_complete {
            evidence.push("oi_context_present".to_string());
        }
        if episode.multi_exchange {
            evidence.push("multi_exchange_confirmed".to_string());
        }
        ShadowCandidate {
            lane: "shadow".to_string(),
            symbol: observation.symbol.clone(),
            direction,
            state,
            first_seen_ts: episode.first_seen_ts,
            last_seen_ts: episode.last_seen_ts,
            observation_count: episode.observation_count,
            cumulative_volume_btc: episode.cumulative_volume_btc,
            cumulative_net_volume_btc: episode.cumulative_net_volume_btc,
            evidence,
            invalidation_reason,
            read_only: true,
            analysis_only: true,
            execution_enabled: false,
        }
    }

    fn invalidated(
        observation: &ShadowObservation,
        direction: ShadowDirection,
        episode: &Episode,
        reason: String,
    ) -> ShadowCandidate {
        Self::candidate(
            observation,
            direction,
            episode,
            ShadowState::Invalidated,
            Some(reason),
        )
    }
}
