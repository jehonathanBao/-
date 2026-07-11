//! Conservative lifecycle tracking for unusually large visible L2 levels.
//!
//! A pull is observable. A spoof is not confirmed without order-level
//! identity and execution evidence, which public depth feeds do not provide.

use std::collections::{BTreeMap, BTreeSet};

use serde::{Deserialize, Serialize};

use super::l2::{BookSide, LocalOrderBook};

const WALL_MULTIPLE: f64 = 3.0;
const PERSISTENT_OBSERVATIONS: u32 = 3;
const MAX_EVIDENCE: usize = 20;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum WallLifecycle {
    Visible,
    Persistent,
    Pulled,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct WallEvidence {
    pub side: BookSide,
    pub price: f64,
    pub quantity: f64,
    pub relative_size: f64,
    pub observations: u32,
    pub lifecycle: WallLifecycle,
    pub first_seen_ms: i64,
    pub last_seen_ms: i64,
    pub label: String,
    pub probabilistic: bool,
    pub participant_identified: bool,
}

#[derive(Debug, Clone)]
struct WallState {
    evidence: WallEvidence,
}

#[derive(Debug, Default)]
pub struct WallTracker {
    walls: BTreeMap<String, WallState>,
}

impl WallTracker {
    pub fn observe(&mut self, book: &LocalOrderBook, now_ms: i64) {
        let levels = book.top_levels(20);
        if levels.is_empty() {
            return;
        }
        let mut quantities = levels
            .iter()
            .map(|level| level.quantity)
            .collect::<Vec<_>>();
        quantities
            .sort_by(|left, right| left.partial_cmp(right).unwrap_or(std::cmp::Ordering::Equal));
        let median = quantities[quantities.len() / 2].max(f64::MIN_POSITIVE);
        let mut seen = BTreeSet::new();
        for level in levels
            .into_iter()
            .filter(|level| level.quantity >= median * WALL_MULTIPLE)
        {
            let key = format!("{:?}:{:.8}", level.side, level.price);
            seen.insert(key.clone());
            let relative_size = level.quantity / median;
            let state = self.walls.entry(key).or_insert_with(|| WallState {
                evidence: WallEvidence {
                    side: level.side,
                    price: level.price,
                    quantity: level.quantity,
                    relative_size,
                    observations: 0,
                    lifecycle: WallLifecycle::Visible,
                    first_seen_ms: now_ms,
                    last_seen_ms: now_ms,
                    label: "visible_large_l2_level".to_string(),
                    probabilistic: true,
                    participant_identified: false,
                },
            });
            state.evidence.quantity = level.quantity;
            state.evidence.relative_size = relative_size;
            state.evidence.observations = state.evidence.observations.saturating_add(1);
            state.evidence.last_seen_ms = now_ms;
            if state.evidence.observations >= PERSISTENT_OBSERVATIONS {
                state.evidence.lifecycle = WallLifecycle::Persistent;
                state.evidence.label = "persistent_large_l2_level".to_string();
            }
        }
        for (key, state) in &mut self.walls {
            if !seen.contains(key) && state.evidence.lifecycle != WallLifecycle::Pulled {
                state.evidence.lifecycle = WallLifecycle::Pulled;
                state.evidence.label = "visible_l2_level_pulled".to_string();
                state.evidence.last_seen_ms = now_ms;
            }
        }
        self.prune();
    }

    pub fn evidence(&self) -> Vec<WallEvidence> {
        let mut result = self
            .walls
            .values()
            .map(|state| state.evidence.clone())
            .collect::<Vec<_>>();
        result.sort_by(|left, right| right.last_seen_ms.cmp(&left.last_seen_ms));
        result.truncate(MAX_EVIDENCE);
        result
    }

    fn prune(&mut self) {
        if self.walls.len() <= MAX_EVIDENCE * 3 {
            return;
        }
        let mut items = self
            .walls
            .iter()
            .map(|(key, state)| (key.clone(), state.evidence.last_seen_ms))
            .collect::<Vec<_>>();
        items.sort_by_key(|(_, last_seen)| *last_seen);
        for (key, _) in items.into_iter().take(self.walls.len() - MAX_EVIDENCE * 2) {
            self.walls.remove(&key);
        }
    }
}
