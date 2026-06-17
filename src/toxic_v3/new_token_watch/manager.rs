use std::{collections::BTreeMap, sync::Arc};

use parking_lot::RwLock;
use thiserror::Error;

use super::{
    collector::ContractFlowCollector,
    engine::NewTokenFlowEngine,
    types::{
        BehaviorProbabilities, CapitalPhase, ContractTick, CostDistributionBand,
        PhaseTimelineSegment, SmartLevel, SmartMoneyChartResponse,
        SmartMoneyReconstructionResponse, TokenChartMarker, TokenChartPoint, TokenWatchItem,
        TokenWatchListResponse, MAX_ACTIVE_TOKENS,
    },
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

    pub fn get_reconstruction(
        &self,
        raw_symbol: &str,
        timeframe: &str,
    ) -> Result<SmartMoneyReconstructionResponse, TokenWatchError> {
        let item = self.refresh_item(raw_symbol)?;
        Ok(build_reconstruction_response(&item, timeframe))
    }

    pub fn get_chart(
        &self,
        raw_symbol: &str,
        timeframe: &str,
    ) -> Result<SmartMoneyChartResponse, TokenWatchError> {
        let item = self.refresh_item(raw_symbol)?;
        Ok(build_chart_response(&item, timeframe))
    }

    fn refresh_item(&self, raw_symbol: &str) -> Result<TokenWatchItem, TokenWatchError> {
        let symbol = normalize_symbol(raw_symbol)?;
        let now = now_ms();
        let mut guard = self.items.write();
        let item = guard
            .get_mut(&symbol)
            .ok_or(TokenWatchError::TokenNotFound)?;
        let ticks = ContractFlowCollector::deterministic_probe_ticks(&item.symbol, now as u64);
        item.last_signal = NewTokenFlowEngine::analyze_ticks(&item.symbol, &ticks);
        item.stream_status = "read_only_probe".to_string();
        Ok(item.clone())
    }

    #[cfg(test)]
    pub fn clear(&self) {
        self.items.write().clear();
    }
}

fn build_reconstruction_response(
    item: &TokenWatchItem,
    timeframe: &str,
) -> SmartMoneyReconstructionResponse {
    let tf = normalize_timeframe(timeframe);
    let signal = &item.last_signal;
    let capital = &signal.capital_structure;
    let reconstruction = &signal.position_reconstruction;
    let cost = &capital.cost_basis;
    let position = &capital.estimated_position;
    let current_price = current_price(item).max(0.0);
    let estimated_net_position_base = reconstruction
        .latent_position
        .last()
        .map(|point| point.impact_adjusted_position)
        .unwrap_or_else(|| {
            if current_price > 0.0 {
                position.lower_usd / current_price
            } else {
                0.0
            }
        });
    let estimated_net_position_usdt = estimated_net_position_base * current_price;
    let floating_pnl_low_pct = pct_change(current_price, cost.lower);
    let floating_pnl_high_pct = pct_change(current_price, cost.upper);
    let phase_timeline = build_phase_timeline(item);
    SmartMoneyReconstructionResponse {
        symbol: item.symbol.clone(),
        timeframe: tf,
        current_phase: capital.phase,
        current_price,
        change_24h_pct: None,
        volume_24h_usd: None,
        high_24h: None,
        low_24h: None,
        market_cap_usd: None,
        cost_basis_low: cost.lower,
        cost_basis_high: cost.upper,
        vwap_anchor: cost.vwap_anchor,
        estimated_total_position_usdt_low: position.lower_usd,
        estimated_total_position_usdt_high: position.upper_usd,
        estimated_net_position_usdt,
        floating_pnl_low_pct,
        floating_pnl_high_pct,
        accumulation_path: reconstruction.accumulation_path.clone(),
        last_accumulation_node: reconstruction.last_accumulation_node.clone(),
        distribution_path: reconstruction.distribution_path.clone(),
        distribution_completion_pct: distribution_completion(reconstruction),
        distribution_intensity_score: capital.distribution_risk.score * 100.0,
        short_term_behavior_probabilities: build_behavior_probabilities(item),
        phase_timeline,
        cost_distribution: build_cost_distribution(item),
        smart_levels: build_smart_levels(item),
        confidence: reconstruction.confidence.max(capital.phase_confidence),
        read_only: true,
    }
}

fn build_chart_response(item: &TokenWatchItem, timeframe: &str) -> SmartMoneyChartResponse {
    let reconstruction = &item.last_signal.position_reconstruction;
    let mut previous_position = 0.0;
    let points = reconstruction
        .latent_position
        .iter()
        .map(|point| {
            let volume = (point.impact_adjusted_position - previous_position).abs();
            previous_position = point.impact_adjusted_position;
            TokenChartPoint {
                ts: point.timestamp,
                price: point.price,
                volume,
                net_position: point.impact_adjusted_position,
            }
        })
        .collect::<Vec<_>>();
    let markers = build_chart_markers(item);
    SmartMoneyChartResponse {
        symbol: item.symbol.clone(),
        timeframe: normalize_timeframe(timeframe),
        points,
        phase_segments: build_phase_timeline(item),
        markers,
        read_only: true,
    }
}

fn normalize_timeframe(timeframe: &str) -> String {
    match timeframe {
        "1m" | "5m" | "15m" | "1h" => timeframe.to_string(),
        _ => "15m".to_string(),
    }
}

fn current_price(item: &TokenWatchItem) -> f64 {
    item.last_signal
        .position_reconstruction
        .latent_position
        .last()
        .map(|point| point.price)
        .filter(|price| *price > 0.0)
        .unwrap_or(item.last_signal.capital_structure.cost_basis.vwap_anchor)
}

fn pct_change(current: f64, basis: f64) -> f64 {
    if basis <= 0.0 || current <= 0.0 {
        0.0
    } else {
        ((current - basis) / basis) * 100.0
    }
}

fn distribution_completion(reconstruction: &super::types::SmartMoneyPositionReconstruction) -> f64 {
    let accumulation_volume = reconstruction
        .accumulation_path
        .iter()
        .map(|segment| segment.cumulative_delta.abs())
        .sum::<f64>();
    let distribution_volume = reconstruction
        .distribution_path
        .iter()
        .map(|segment| segment.cumulative_delta.abs())
        .sum::<f64>();
    let total = accumulation_volume + distribution_volume;
    if total <= 0.0 {
        0.0
    } else {
        ((distribution_volume / total) * 100.0).clamp(0.0, 100.0)
    }
}

fn build_behavior_probabilities(item: &TokenWatchItem) -> BehaviorProbabilities {
    let phase = item.last_signal.capital_structure.phase;
    let distribution = item
        .last_signal
        .capital_structure
        .distribution_risk
        .score
        .clamp(0.0, 1.0);
    let confidence = item
        .last_signal
        .capital_structure
        .phase_confidence
        .clamp(0.0, 1.0);
    let (distribution_bias, range_bias, rebound_bias, accumulation_bias) = match phase {
        CapitalPhase::Distribution | CapitalPhase::Breakdown => {
            (0.55 + distribution * 0.35, 0.15, 0.1, 0.2)
        }
        CapitalPhase::Markup => (0.15 + distribution * 0.2, 0.15, 0.5, 0.2),
        CapitalPhase::Accumulation => (distribution * 0.2, 0.2, 0.25, 0.55),
        CapitalPhase::Neutral => (distribution * 0.25, 0.45, 0.15, 0.15 + confidence * 0.2),
    };
    let total = distribution_bias + range_bias + rebound_bias + accumulation_bias;
    BehaviorProbabilities {
        continue_distribution: distribution_bias / total,
        range_consolidation: range_bias / total,
        rebound_markup: rebound_bias / total,
        secondary_accumulation: accumulation_bias / total,
    }
}

fn build_phase_timeline(item: &TokenWatchItem) -> Vec<PhaseTimelineSegment> {
    let reconstruction = &item.last_signal.position_reconstruction;
    let mut cursor = item.added_at_ms.max(0) as u64;
    let mut segments = reconstruction
        .accumulation_path
        .iter()
        .chain(reconstruction.distribution_path.iter())
        .map(|segment| {
            let start_ms = cursor;
            let duration_ms = segment.duration_sec.saturating_mul(1000).max(1000);
            cursor = cursor.saturating_add(duration_ms);
            PhaseTimelineSegment {
                phase: segment.phase,
                label: segment.label.clone(),
                start_ms,
                end_ms: cursor,
                duration_sec: segment.duration_sec.max(1),
                lower: segment.start_price.min(segment.end_price),
                upper: segment.start_price.max(segment.end_price),
            }
        })
        .collect::<Vec<_>>();
    if segments.is_empty() {
        let price = current_price(item);
        segments.push(PhaseTimelineSegment {
            phase: item.last_signal.capital_structure.phase,
            label: item.last_signal.capital_structure.phase_label.clone(),
            start_ms: cursor,
            end_ms: cursor.saturating_add(60_000),
            duration_sec: 60,
            lower: price * 0.998,
            upper: price * 1.002,
        });
    }
    segments
}

fn build_cost_distribution(item: &TokenWatchItem) -> Vec<CostDistributionBand> {
    let cost = &item.last_signal.capital_structure.cost_basis;
    let width = (cost.upper - cost.lower)
        .abs()
        .max(cost.vwap_anchor.abs() * 0.002);
    vec![
        CostDistributionBand {
            label: "核心成本区".to_string(),
            lower: cost.lower,
            upper: cost.upper,
            pct: 0.62,
        },
        CostDistributionBand {
            label: "早期吸筹区".to_string(),
            lower: (cost.lower - width).max(0.0),
            upper: cost.lower,
            pct: 0.23,
        },
        CostDistributionBand {
            label: "浮动追仓区".to_string(),
            lower: cost.upper,
            upper: cost.upper + width,
            pct: 0.15,
        },
    ]
}

fn build_smart_levels(item: &TokenWatchItem) -> Vec<SmartLevel> {
    let cost = &item.last_signal.capital_structure.cost_basis;
    let mut levels = vec![
        SmartLevel {
            label: "成本下沿".to_string(),
            price: cost.lower,
            role: "support".to_string(),
        },
        SmartLevel {
            label: "VWAP锚点".to_string(),
            price: cost.vwap_anchor,
            role: "anchor".to_string(),
        },
        SmartLevel {
            label: "成本上沿".to_string(),
            price: cost.upper,
            role: "resistance".to_string(),
        },
    ];
    if let Some(node) = &item
        .last_signal
        .position_reconstruction
        .last_accumulation_node
    {
        levels.push(SmartLevel {
            label: "最后吸筹点".to_string(),
            price: (node.lower + node.upper) / 2.0,
            role: "last_accumulation".to_string(),
        });
    }
    levels
}

fn build_chart_markers(item: &TokenWatchItem) -> Vec<TokenChartMarker> {
    let reconstruction = &item.last_signal.position_reconstruction;
    let mut markers = Vec::new();
    if let Some(node) = &reconstruction.last_accumulation_node {
        markers.push(TokenChartMarker {
            ts: item.added_at_ms.max(0) as u64,
            price: (node.lower + node.upper) / 2.0,
            label: "最后吸筹点".to_string(),
            kind: "last_accumulation".to_string(),
        });
    }
    if let Some(segment) = reconstruction.distribution_path.first() {
        markers.push(TokenChartMarker {
            ts: item.added_at_ms.max(0) as u64 + segment.duration_sec.saturating_mul(1000),
            price: segment.end_price,
            label: "出货确认".to_string(),
            kind: "distribution".to_string(),
        });
    }
    markers
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
