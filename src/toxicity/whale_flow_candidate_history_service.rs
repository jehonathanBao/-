use std::{collections::VecDeque, sync::Arc};

use parking_lot::RwLock;

use crate::types::whale_flow_signal::{WhaleFlowCandidate, WhaleFlowRecentResponse};

const DEFAULT_MAX_CANDIDATES: usize = 500;
pub const WHALE_FLOW_CANDIDATE_HISTORY_RETENTION_MODE: &str = "in_memory_bounded";

#[derive(Debug)]
struct WhaleFlowCandidateHistoryStore {
    max_candidates: usize,
    candidates: VecDeque<WhaleFlowCandidate>,
    recorded_count: u64,
    deduplicated_count: u64,
    evicted_count: u64,
}

impl WhaleFlowCandidateHistoryStore {
    fn new(max_candidates: usize) -> Self {
        Self {
            max_candidates,
            candidates: VecDeque::new(),
            recorded_count: 0,
            deduplicated_count: 0,
            evicted_count: 0,
        }
    }
}

#[derive(Debug, Clone, Copy)]
pub struct WhaleFlowCandidateHistorySnapshot {
    pub current_candidates: usize,
    pub max_candidates: usize,
    pub recorded_count: u64,
    pub deduplicated_count: u64,
    pub evicted_count: u64,
}

#[derive(Clone, Debug)]
pub struct WhaleFlowCandidateHistoryService {
    store: Arc<RwLock<WhaleFlowCandidateHistoryStore>>,
}

impl Default for WhaleFlowCandidateHistoryService {
    fn default() -> Self {
        Self::new(DEFAULT_MAX_CANDIDATES)
    }
}

impl WhaleFlowCandidateHistoryService {
    pub fn new(max_candidates: usize) -> Self {
        Self {
            store: Arc::new(RwLock::new(WhaleFlowCandidateHistoryStore::new(
                max_candidates,
            ))),
        }
    }

    pub fn record_report(&self, report: &WhaleFlowRecentResponse) {
        if report.candidates.is_empty() {
            return;
        }

        let mut store = self.store.write();
        let max_candidates = store.max_candidates;
        for candidate in &report.candidates {
            store.recorded_count += 1;
            upsert_candidate(&mut store, max_candidates, candidate.clone());
        }
    }

    pub fn recent_candidates(&self, selected_symbol: &str) -> Vec<WhaleFlowCandidate> {
        let store = self.store.read();
        store
            .candidates
            .iter()
            .filter(|candidate| symbol_matches(&candidate.symbol, selected_symbol))
            .cloned()
            .collect()
    }

    pub fn len(&self) -> usize {
        self.store.read().candidates.len()
    }

    pub fn is_empty(&self) -> bool {
        self.store.read().candidates.is_empty()
    }

    pub fn snapshot(&self) -> WhaleFlowCandidateHistorySnapshot {
        let store = self.store.read();
        WhaleFlowCandidateHistorySnapshot {
            current_candidates: store.candidates.len(),
            max_candidates: store.max_candidates,
            recorded_count: store.recorded_count,
            deduplicated_count: store.deduplicated_count,
            evicted_count: store.evicted_count,
        }
    }
}

fn upsert_candidate(
    store: &mut WhaleFlowCandidateHistoryStore,
    max_candidates: usize,
    candidate: WhaleFlowCandidate,
) {
    let before_len = store.candidates.len();
    store
        .candidates
        .retain(|existing| existing.candidate_id != candidate.candidate_id);
    if store.candidates.len() != before_len {
        store.deduplicated_count += 1;
    }
    store.candidates.push_front(candidate);
    while store.candidates.len() > max_candidates {
        let _ = store.candidates.pop_back();
        store.evicted_count += 1;
    }
}

fn symbol_matches(candidate_symbol: &str, selected_symbol: &str) -> bool {
    selected_symbol.eq_ignore_ascii_case("ALL")
        || candidate_symbol.eq_ignore_ascii_case(selected_symbol)
}
