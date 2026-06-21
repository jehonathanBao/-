use crate::contract_whale_monitor::types::{
    ContractWhaleEventStatus, ContractWhaleLatestResponse, ContractWhaleSignal,
    ContractWhaleSignalType,
};
use crate::normalization::market_impact::{MarketImpactBaseline, MarketImpactNormalization};

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct FinalEvent {
    pub event_id: String,
    pub symbol: String,
    pub event_type: String,
    pub start_time: i64,
    pub end_time: i64,
    pub status: String,
    pub window_sec: u64,
    pub raw_volume: f64,
    pub impact_score: f64,
    pub z_score: f64,
    pub percentile: f64,
    pub normalized_score: f64,
    pub normalized_strength: String,
    pub impact_level: String,
    pub signal_level: String,
    pub signal_label: String,
    pub volume: f64,
    pub net_volume: f64,
    pub notional: f64,
    pub price: Option<f64>,
    pub price_move_pct: Option<f64>,
    pub direction_bias: String,
    pub dominance: f64,
    pub quality_score: f64,
    pub merge_similarity_score: f64,
    pub false_event_flags: Vec<String>,
    pub source_signal_ids: Vec<String>,
    pub source_signal: ContractWhaleSignal,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct FinalEventStoreResponse {
    pub items: Vec<FinalEvent>,
    pub count: usize,
}

pub fn build_final_events_from_contract_whale_signals(
    signals: &[ContractWhaleSignal],
) -> Vec<FinalEvent> {
    let baseline =
        MarketImpactBaseline::from_volumes(signals.iter().map(|signal| signal.total_volume_btc));
    signals
        .iter()
        .map(|signal| {
            FinalEvent::from_contract_signal_with_impact(
                signal,
                baseline.normalize(signal.total_volume_btc),
            )
        })
        .collect()
}

pub fn build_final_event_store_response_from_contract_whale_response(
    response: &ContractWhaleLatestResponse,
) -> FinalEventStoreResponse {
    let items = build_final_events_from_contract_whale_signals(&response.items);
    FinalEventStoreResponse {
        count: items.len(),
        items,
    }
}

impl FinalEvent {
    pub fn from_contract_signal(signal: &ContractWhaleSignal) -> Self {
        let baseline = MarketImpactBaseline::from_volumes([signal.total_volume_btc]);
        Self::from_contract_signal_with_impact(signal, baseline.normalize(signal.total_volume_btc))
    }

    pub fn from_contract_signal_with_impact(
        signal: &ContractWhaleSignal,
        impact: MarketImpactNormalization,
    ) -> Self {
        let mut source_signal_ids = vec![signal.id.clone()];
        for id in &signal.merged_from {
            if !id.is_empty() && !source_signal_ids.iter().any(|existing| existing == id) {
                source_signal_ids.push(id.clone());
            }
        }
        let event_id = if signal.event_lifecycle.event_id.is_empty() {
            signal.id.clone()
        } else {
            signal.event_lifecycle.event_id.clone()
        };
        Self {
            event_id,
            symbol: signal.symbol.clone(),
            event_type: signal_type_key(signal.signal_type).to_string(),
            start_time: signal.event_lifecycle.start_time,
            end_time: signal.event_lifecycle.last_update_time,
            status: event_status_key(signal.event_lifecycle.status).to_string(),
            window_sec: signal.window_sec,
            raw_volume: impact.raw_volume,
            impact_score: impact.impact_score,
            z_score: impact.z_score,
            percentile: impact.percentile,
            normalized_score: impact.normalized_score,
            normalized_strength: impact.normalized_strength,
            impact_level: impact.impact_level,
            signal_level: impact.signal_level,
            signal_label: impact.signal_label,
            volume: signal.total_volume_btc,
            net_volume: signal.net_volume_btc,
            notional: signal.total_notional_usd,
            price: signal
                .order_price_usd
                .or(signal.current_market_price_usd)
                .or_else(|| average_price(signal)),
            price_move_pct: signal.price_move_pct,
            direction_bias: direction_bias(signal.net_volume_btc).to_string(),
            dominance: signal.dominance,
            quality_score: signal.event_quality.quality_score,
            merge_similarity_score: signal.event_quality.merge_similarity_score,
            false_event_flags: signal.event_quality.false_event_flags.clone(),
            source_signal_ids,
            source_signal: signal.clone(),
        }
    }
}

fn average_price(signal: &ContractWhaleSignal) -> Option<f64> {
    (signal.total_volume_btc > f64::EPSILON && signal.total_notional_usd > 0.0)
        .then(|| signal.total_notional_usd / signal.total_volume_btc)
        .filter(|value| value.is_finite() && *value > 0.0)
}

fn direction_bias(net_volume: f64) -> &'static str {
    if net_volume > 0.0 {
        "buy"
    } else if net_volume < 0.0 {
        "sell"
    } else {
        "neutral"
    }
}

fn event_status_key(status: ContractWhaleEventStatus) -> &'static str {
    match status {
        ContractWhaleEventStatus::Active => "active",
        ContractWhaleEventStatus::Closed => "closed",
    }
}

fn signal_type_key(signal_type: ContractWhaleSignalType) -> &'static str {
    match signal_type {
        ContractWhaleSignalType::AggressiveBuy => "aggressive_buy",
        ContractWhaleSignalType::AggressiveSell => "aggressive_sell",
        ContractWhaleSignalType::DownsideAbsorption => "downside_absorption",
        ContractWhaleSignalType::UpsideSuppression => "upside_suppression",
    }
}
