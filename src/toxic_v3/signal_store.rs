use std::collections::VecDeque;

use super::signal::SignalEvent;

pub trait SignalStore {
    fn record(&mut self, signal: &SignalEvent);
    fn recent(&self, limit: usize) -> Vec<SignalEvent>;
    fn len(&self) -> usize;

    fn is_empty(&self) -> bool {
        self.len() == 0
    }
}

#[derive(Debug, Clone)]
pub struct InMemorySignalStore {
    max_signals: usize,
    signals: VecDeque<SignalEvent>,
}

impl InMemorySignalStore {
    pub fn new(max_signals: usize) -> Self {
        Self {
            max_signals: max_signals.max(1),
            signals: VecDeque::new(),
        }
    }
}

impl Default for InMemorySignalStore {
    fn default() -> Self {
        Self::new(10_000)
    }
}

impl SignalStore for InMemorySignalStore {
    fn record(&mut self, signal: &SignalEvent) {
        self.signals.push_back(signal.clone());
        while self.signals.len() > self.max_signals {
            self.signals.pop_front();
        }
    }

    fn recent(&self, limit: usize) -> Vec<SignalEvent> {
        self.signals.iter().rev().take(limit).cloned().collect()
    }

    fn len(&self) -> usize {
        self.signals.len()
    }
}
