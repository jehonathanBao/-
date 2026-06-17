use std::{collections::BTreeMap, sync::Arc};

use parking_lot::RwLock;
use thiserror::Error;

use super::{
    collector::ContractFlowCollector,
    engine::NewTokenFlowEngine,
    types::{ContractTick, TokenWatchItem, TokenWatchListResponse, MAX_ACTIVE_TOKENS},
};
use crate::normalizers::trade::now_ms;

#[derive(Debug, Error, PartialEq, Eq)]
pub enum TokenWatchError {
    #[error("invalid_symbol")]
    InvalidSymbol,
    #[error("max_active_tokens_reached")]
    MaxActiveTokensReached,
    #[error("token_not_found")]
    TokenNotFound,
}

#[derive(Debug, Clone, Default)]
pub struct TokenWatchManager {
    items: Arc<RwLock<BTreeMap<String, TokenWatchItem>>>,
}

impl TokenWatchManager {
    pub fn add_token(&self, raw_symbol: &str) -> Result<TokenWatchItem, TokenWatchError> {
        let symbol = normalize_symbol(raw_symbol)?;
        let mut guard = self.items.write();
        if let Some(existing) = guard.get(&symbol) {
            return Ok(existing.clone());
        }
        if guard.len() >= MAX_ACTIVE_TOKENS {
            return Err(TokenWatchError::MaxActiveTokensReached);
        }
        let now = now_ms();
        let ticks = ContractFlowCollector::deterministic_probe_ticks(&symbol, now as u64);
        let item = TokenWatchItem {
            symbol: symbol.clone(),
            added_at_ms: now,
            stream_status: "read_only_probe".to_string(),
            last_signal: NewTokenFlowEngine::analyze_ticks(&symbol, &ticks),
            read_only: true,
        };
        guard.insert(symbol, item.clone());
        Ok(item)
    }

    pub fn remove_token(&self, raw_symbol: &str) -> Result<TokenWatchItem, TokenWatchError> {
        let symbol = normalize_symbol(raw_symbol)?;
        self.items
            .write()
            .remove(&symbol)
            .ok_or(TokenWatchError::TokenNotFound)
    }

    pub fn list_active_tokens(&self) -> TokenWatchListResponse {
        let now = now_ms();
        let mut guard = self.items.write();
        for item in guard.values_mut() {
            let ticks = ContractFlowCollector::deterministic_probe_ticks(&item.symbol, now as u64);
            item.last_signal = NewTokenFlowEngine::analyze_ticks(&item.symbol, &ticks);
            item.stream_status = "read_only_probe".to_string();
        }
        let items = guard.values().cloned().collect::<Vec<_>>();
        TokenWatchListResponse {
            active_count: items.len(),
            items,
            max_active_tokens: MAX_ACTIVE_TOKENS,
            read_only: true,
        }
    }

    pub fn record_tick(&self, tick: ContractTick) -> Result<TokenWatchItem, TokenWatchError> {
        let symbol = normalize_symbol(&tick.symbol)?;
        let mut guard = self.items.write();
        let item = guard
            .get_mut(&symbol)
            .ok_or(TokenWatchError::TokenNotFound)?;
        item.last_signal = NewTokenFlowEngine::analyze_ticks(&symbol, &[tick]);
        item.stream_status = "test_tick_observed".to_string();
        Ok(item.clone())
    }

    #[cfg(test)]
    pub fn clear(&self) {
        self.items.write().clear();
    }
}

pub fn normalize_symbol(raw_symbol: &str) -> Result<String, TokenWatchError> {
    let compact = raw_symbol
        .trim()
        .chars()
        .filter(|c| c.is_ascii_alphanumeric())
        .collect::<String>()
        .to_ascii_uppercase();
    if compact.len() < 2 || compact.len() > 24 {
        return Err(TokenWatchError::InvalidSymbol);
    }
    let symbol = if compact.ends_with("USDT") {
        compact
    } else {
        format!("{compact}USDT")
    };
    if symbol.len() > 28 {
        return Err(TokenWatchError::InvalidSymbol);
    }
    Ok(symbol)
}
