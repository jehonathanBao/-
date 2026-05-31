use std::collections::HashMap;

#[derive(Debug, Clone)]
pub struct AlertDeduper {
    window_ms: i64,
    last_sent: HashMap<String, i64>,
}

impl AlertDeduper {
    pub fn new(window_ms: i64) -> Self {
        Self {
            window_ms: window_ms.max(0),
            last_sent: HashMap::new(),
        }
    }

    pub fn should_send(&mut self, key: &str, now_ts: i64) -> bool {
        self.prune(now_ts);
        match self.last_sent.get(key) {
            Some(last_ts) => now_ts - *last_ts >= self.window_ms,
            None => true,
        }
    }

    pub fn mark_sent(&mut self, key: &str, now_ts: i64) {
        self.last_sent.insert(key.to_string(), now_ts);
    }

    pub fn prune(&mut self, now_ts: i64) {
        let window_ms = self.window_ms;
        self.last_sent
            .retain(|_, last_ts| now_ts - *last_ts <= window_ms);
    }
}
