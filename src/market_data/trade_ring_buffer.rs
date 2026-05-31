use std::collections::HashSet;

use crate::types::market::NormalizedTrade;

#[derive(Debug)]
pub struct TradeRingBuffer {
    trades: Vec<NormalizedTrade>,
    seen_trade_ids: HashSet<String>,
    max_age_ms: i64,
}

impl TradeRingBuffer {
    pub fn new(max_age_ms: i64) -> Self {
        Self {
            trades: Vec::new(),
            seen_trade_ids: HashSet::new(),
            max_age_ms,
        }
    }

    pub fn add_trade(&mut self, trade: NormalizedTrade) {
        if let Some(key) = dedupe_key(&trade) {
            if self.seen_trade_ids.contains(&key) {
                return;
            }
            self.seen_trade_ids.insert(key);
        }
        let ts = trade.ts;
        self.trades.push(trade);
        self.prune(ts);
    }

    pub fn get_trades_since(&self, ts: i64) -> Vec<NormalizedTrade> {
        self.trades
            .iter()
            .filter(|trade| trade.ts >= ts)
            .cloned()
            .collect()
    }

    pub fn prune(&mut self, now_ts: i64) {
        let cutoff = now_ts - self.max_age_ms;
        let mut removed = Vec::new();
        self.trades.retain(|trade| {
            let keep = trade.ts >= cutoff;
            if !keep {
                removed.push(trade.clone());
            }
            keep
        });
        for trade in removed {
            if let Some(key) = dedupe_key(&trade) {
                self.seen_trade_ids.remove(&key);
            }
        }
    }

    pub fn len(&self) -> usize {
        self.trades.len()
    }

    pub fn is_empty(&self) -> bool {
        self.trades.is_empty()
    }
}

fn dedupe_key(trade: &NormalizedTrade) -> Option<String> {
    trade
        .trade_id
        .as_ref()
        .map(|trade_id| format!("{}:{trade_id}", trade.venue))
}
