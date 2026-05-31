use crate::types::liq_hunt::{LiqHuntDirection, LiqHuntResult, LiqHuntSignalLevel};

#[derive(Debug, Clone)]
pub struct LiqHuntReplaySummary {
    pub max_score: f64,
    pub active_count: usize,
    pub likely_count: usize,
    pub watch_count: usize,
    pub short_squeeze_count: usize,
    pub long_squeeze_count: usize,
    pub top_signals: Vec<LiqHuntResult>,
}

#[derive(Debug, Clone, Default)]
pub struct LiqHuntReplayAccumulator {
    max_score: f64,
    active_count: usize,
    likely_count: usize,
    watch_count: usize,
    short_squeeze_count: usize,
    long_squeeze_count: usize,
    top_signals: Vec<LiqHuntResult>,
}

impl LiqHuntReplayAccumulator {
    pub fn observe(&mut self, result: &LiqHuntResult) {
        self.max_score = self.max_score.max(result.score);
        match result.level {
            LiqHuntSignalLevel::Active => self.active_count += 1,
            LiqHuntSignalLevel::Likely => self.likely_count += 1,
            LiqHuntSignalLevel::Watch => self.watch_count += 1,
            LiqHuntSignalLevel::None => {}
        }
        match result.direction {
            LiqHuntDirection::ShortSqueeze => self.short_squeeze_count += 1,
            LiqHuntDirection::LongSqueeze => self.long_squeeze_count += 1,
            LiqHuntDirection::None => {}
        }
        if result.level != LiqHuntSignalLevel::None {
            self.top_signals.push(result.clone());
        }
    }

    pub fn finalize(mut self) -> Option<LiqHuntReplaySummary> {
        if self.max_score <= 0.0 && self.top_signals.is_empty() {
            return None;
        }
        self.top_signals
            .sort_by(|left, right| right.score.total_cmp(&left.score));
        self.top_signals.truncate(20);

        Some(LiqHuntReplaySummary {
            max_score: self.max_score,
            active_count: self.active_count,
            likely_count: self.likely_count,
            watch_count: self.watch_count,
            short_squeeze_count: self.short_squeeze_count,
            long_squeeze_count: self.long_squeeze_count,
            top_signals: self.top_signals,
        })
    }
}
