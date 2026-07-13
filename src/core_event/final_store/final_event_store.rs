use std::collections::BTreeSet;

use crate::contract_whale_monitor::types::{
    ContractWhaleEventStatus, ContractWhaleLatestResponse, ContractWhaleOiContextTag,
    ContractWhaleSignal, ContractWhaleSignalType,
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
    pub total_volume_btc: f64,
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
    #[serde(default)]
    pub buy_volume_btc: Option<f64>,
    #[serde(default)]
    pub sell_volume_btc: Option<f64>,
    #[serde(default)]
    pub display_volume_btc: f64,
    #[serde(default)]
    pub display_volume_label: String,
    #[serde(default)]
    pub volume_semantics: String,
    #[serde(default)]
    pub is_bidirectional_volume: bool,
    #[serde(default)]
    pub is_cross_exchange_aggregated: bool,
    #[serde(default)]
    pub is_lifecycle_accumulated: bool,
    #[serde(default)]
    pub merged_signal_count: usize,
    #[serde(default)]
    pub source_exchange_count: Option<usize>,
    #[serde(default)]
    pub source_exchanges: Vec<String>,
    #[serde(default)]
    pub merged_windows_sec: Vec<u64>,
    #[serde(default)]
    pub oi_context: ContractWhaleOiContextTag,
    #[serde(default)]
    pub oi_context_label: String,
    #[serde(default)]
    pub oi_delta: Option<f64>,
    #[serde(default)]
    pub oi_delta_pct: Option<f64>,
    #[serde(default)]
    pub oi_available: bool,
    #[serde(default)]
    pub oi_reason: Option<String>,
    pub source_signal: ContractWhaleSignal,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum VolumeDisplayContext {
    SingleWindowSignal,
    ContractEventStream,
    FinalLifecycleEvent,
}

impl VolumeDisplayContext {
    pub fn display_label(self) -> &'static str {
        match self {
            Self::SingleWindowSignal => "窗口总流量 BTC",
            Self::ContractEventStream => "峰值窗口流量 BTC",
            Self::FinalLifecycleEvent => "峰值窗口流量 BTC",
        }
    }

    pub fn semantics(self) -> &'static str {
        match self {
            Self::SingleWindowSignal => "single_window_bidirectional_cross_exchange",
            Self::ContractEventStream => "multi_exchange_bidirectional_peak_window",
            Self::FinalLifecycleEvent => "multi_exchange_bidirectional_peak_window",
        }
    }

    pub fn is_lifecycle_accumulated(self) -> bool {
        false
    }

    fn supports_unique_turnover(self) -> bool {
        !matches!(self, Self::SingleWindowSignal)
    }
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct VolumeDisplayMeta {
    pub display_volume_btc: f64,
    pub display_volume_label: String,
    pub volume_semantics: String,
    pub is_bidirectional_volume: bool,
    pub is_cross_exchange_aggregated: bool,
    pub is_lifecycle_accumulated: bool,
    pub merged_signal_count: usize,
    pub source_exchange_count: Option<usize>,
    pub source_exchanges: Vec<String>,
    pub merged_windows_sec: Vec<u64>,
    pub buy_volume_btc: Option<f64>,
    pub sell_volume_btc: Option<f64>,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct FinalEventStoreResponse {
    pub items: Vec<FinalEvent>,
    pub count: usize,
}

pub fn build_final_events_from_contract_whale_signals(
    signals: &[ContractWhaleSignal],
    context: VolumeDisplayContext,
) -> Vec<FinalEvent> {
    let baseline =
        MarketImpactBaseline::from_volumes(signals.iter().map(|signal| signal.total_volume_btc));
    signals
        .iter()
        .map(|signal| {
            FinalEvent::from_contract_signal_with_impact(
                signal,
                baseline.normalize(signal.total_volume_btc),
                context,
            )
        })
        .collect()
}

pub fn build_final_event_store_response_from_contract_whale_response(
    response: &ContractWhaleLatestResponse,
) -> FinalEventStoreResponse {
    let items = build_final_events_from_contract_whale_signals(
        &response.items,
        VolumeDisplayContext::FinalLifecycleEvent,
    );
    FinalEventStoreResponse {
        count: items.len(),
        items,
    }
}

impl FinalEvent {
    pub fn from_contract_signal(signal: &ContractWhaleSignal) -> Self {
        let baseline = MarketImpactBaseline::from_volumes([signal.total_volume_btc]);
        Self::from_contract_signal_with_impact(
            signal,
            baseline.normalize(signal.total_volume_btc),
            VolumeDisplayContext::FinalLifecycleEvent,
        )
    }

    pub fn from_contract_signal_with_impact(
        signal: &ContractWhaleSignal,
        impact: MarketImpactNormalization,
        context: VolumeDisplayContext,
    ) -> Self {
        let mut source_signal_ids = vec![signal.id.clone()];
        for id in &signal.merged_from {
            if !id.is_empty() && !source_signal_ids.iter().any(|existing| existing == id) {
                source_signal_ids.push(id.clone());
            }
        }
        let volume_meta = build_volume_display_meta(signal, &source_signal_ids, context);
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
            total_volume_btc: signal.total_volume_btc,
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
            buy_volume_btc: volume_meta.buy_volume_btc,
            sell_volume_btc: volume_meta.sell_volume_btc,
            display_volume_btc: volume_meta.display_volume_btc,
            display_volume_label: volume_meta.display_volume_label,
            volume_semantics: volume_meta.volume_semantics,
            is_bidirectional_volume: volume_meta.is_bidirectional_volume,
            is_cross_exchange_aggregated: volume_meta.is_cross_exchange_aggregated,
            is_lifecycle_accumulated: volume_meta.is_lifecycle_accumulated,
            merged_signal_count: volume_meta.merged_signal_count,
            source_exchange_count: volume_meta.source_exchange_count,
            source_exchanges: volume_meta.source_exchanges,
            merged_windows_sec: volume_meta.merged_windows_sec,
            oi_context: signal.classification_v2.oi_context,
            oi_context_label: signal.classification_v2.oi_context_label.clone(),
            oi_delta: signal.classification_v2.oi_delta,
            oi_delta_pct: signal.classification_v2.oi_delta_pct,
            oi_available: signal.classification_v2.oi_available,
            oi_reason: signal.classification_v2.oi_reason.clone(),
            source_signal: signal.clone(),
        }
    }

    pub fn with_volume_context(mut self, context: VolumeDisplayContext) -> Self {
        self.display_volume_label = context.display_label().to_string();
        self.volume_semantics = context.semantics().to_string();
        self.is_lifecycle_accumulated = context.is_lifecycle_accumulated();
        self
    }
}

pub fn build_volume_display_meta(
    signal: &ContractWhaleSignal,
    source_signal_ids: &[String],
    context: VolumeDisplayContext,
) -> VolumeDisplayMeta {
    let unique_turnover = context.supports_unique_turnover()
        && signal.event_lifecycle.unique_turnover_available
        && signal
            .event_lifecycle
            .unique_turnover_btc
            .is_some_and(|value| value.is_finite() && value >= 0.0);
    let display_volume_btc = if unique_turnover {
        signal
            .event_lifecycle
            .unique_turnover_btc
            .unwrap_or_default()
    } else {
        display_volume_for_context(signal, context)
    };
    let (buy_volume_btc, sell_volume_btc) =
        derive_buy_sell_from_total_net(Some(signal.total_volume_btc), Some(signal.net_volume_btc));
    let source_exchanges = unique_source_exchanges(signal);
    let source_exchange_count = (!source_exchanges.is_empty()).then_some(source_exchanges.len());
    let merged_windows_sec = merged_windows_from_signal_ids(source_signal_ids, signal.window_sec);
    let merged_signal_count = source_signal_ids.len().max(1);
    VolumeDisplayMeta {
        display_volume_btc,
        display_volume_label: if unique_turnover {
            "事件真实换手 BTC".to_string()
        } else {
            context.display_label().to_string()
        },
        volume_semantics: if unique_turnover {
            "unique_1s_turnover_per_exchange".to_string()
        } else {
            context.semantics().to_string()
        },
        is_bidirectional_volume: true,
        is_cross_exchange_aggregated: source_exchange_count.is_some_and(|count| count > 1),
        is_lifecycle_accumulated: unique_turnover || context.is_lifecycle_accumulated(),
        merged_signal_count,
        source_exchange_count,
        source_exchanges,
        merged_windows_sec,
        buy_volume_btc,
        sell_volume_btc,
    }
}

fn display_volume_for_context(signal: &ContractWhaleSignal, context: VolumeDisplayContext) -> f64 {
    if !matches!(context, VolumeDisplayContext::SingleWindowSignal)
        && signal.event_lifecycle.volume_accumulated > f64::EPSILON
    {
        signal.event_lifecycle.volume_accumulated
    } else {
        signal.total_volume_btc
    }
}

pub fn derive_buy_sell_from_total_net(
    total_volume_btc: Option<f64>,
    net_volume_btc: Option<f64>,
) -> (Option<f64>, Option<f64>) {
    let (Some(total_volume_btc), Some(net_volume_btc)) = (total_volume_btc, net_volume_btc) else {
        return (None, None);
    };
    let buy_volume_btc = (total_volume_btc + net_volume_btc) / 2.0;
    let sell_volume_btc = (total_volume_btc - net_volume_btc) / 2.0;
    if !buy_volume_btc.is_finite()
        || !sell_volume_btc.is_finite()
        || buy_volume_btc < 0.0
        || sell_volume_btc < 0.0
    {
        return (None, None);
    }
    (Some(buy_volume_btc), Some(sell_volume_btc))
}

fn unique_source_exchanges(signal: &ContractWhaleSignal) -> Vec<String> {
    let exchanges: BTreeSet<String> = signal
        .exchanges
        .iter()
        .map(|entry| entry.exchange.trim().to_string())
        .filter(|exchange| !exchange.is_empty())
        .collect();
    exchanges.into_iter().collect()
}

fn merged_windows_from_signal_ids(signal_ids: &[String], fallback_window_sec: u64) -> Vec<u64> {
    let windows: BTreeSet<u64> = signal_ids
        .iter()
        .filter_map(|signal_id| signal_id.split(':').nth(2))
        .filter_map(|window| window.parse::<u64>().ok())
        .chain(std::iter::once(fallback_window_sec))
        .collect();
    windows.into_iter().collect()
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::contract_whale_monitor::types::{
        ContractWhaleAction, ContractWhaleActiveSources, ContractWhaleDirection,
        ContractWhaleEventLifecycle, ContractWhaleEventQuality, ContractWhaleForcedFlowAttribution,
        ContractWhaleLiquidationForce, ContractWhaleLiquidationZone, ContractWhaleMarketDriver,
        ContractWhaleMarketDriverComponent, ContractWhaleMarketType, ContractWhalePersistenceState,
        ContractWhalePriceImpactAttribution, ContractWhalePriceResponseType, ContractWhaleSeverity,
        ContractWhaleSignal, ContractWhaleSignalCluster, ContractWhaleSourceRole,
        ContractWhaleSpotConfirmationContext, ContractWhaleStealthProfile, ContractWhaleTrajectory,
        ExchangeFlowContribution,
    };

    #[test]
    fn derives_buy_sell_from_total_and_net_volume() {
        assert_eq!(
            derive_buy_sell_from_total_net(Some(3_400.0), Some(200.0)),
            (Some(1_800.0), Some(1_600.0))
        );
        assert_eq!(
            derive_buy_sell_from_total_net(Some(3_400.0), Some(-200.0)),
            (Some(1_600.0), Some(1_800.0))
        );
        assert_eq!(
            derive_buy_sell_from_total_net(Some(100.0), Some(300.0)),
            (None, None)
        );
        assert_eq!(
            derive_buy_sell_from_total_net(None, Some(10.0)),
            (None, None)
        );
    }

    #[test]
    fn builds_lifecycle_volume_meta_with_contextual_labels() {
        let signal = sample_signal();
        let source_signal_ids = vec![
            signal.id.clone(),
            "contract-whale:BTC:5:1700000000010:downside_absorption".to_string(),
            "contract-whale:BTC:60:1700000000060:downside_absorption".to_string(),
        ];

        let meta = build_volume_display_meta(
            &signal,
            &source_signal_ids,
            VolumeDisplayContext::FinalLifecycleEvent,
        );

        assert_eq!(meta.display_volume_btc, 4_280.0);
        assert_eq!(meta.display_volume_label, "峰值窗口流量 BTC");
        assert_eq!(
            meta.volume_semantics,
            "multi_exchange_bidirectional_peak_window"
        );
        assert!(meta.is_bidirectional_volume);
        assert!(meta.is_cross_exchange_aggregated);
        assert!(!meta.is_lifecycle_accumulated);
        assert_eq!(meta.merged_signal_count, 3);
        assert_eq!(meta.source_exchange_count, Some(3));
        assert_eq!(meta.source_exchanges, vec!["binance", "bitfinex", "okx"]);
        assert_eq!(meta.merged_windows_sec, vec![5, 15, 60]);
        assert_eq!(meta.buy_volume_btc, Some(1_830.0));
        assert_eq!(meta.sell_volume_btc, Some(2_450.0));

        let window_meta = build_volume_display_meta(
            &signal,
            &source_signal_ids,
            VolumeDisplayContext::SingleWindowSignal,
        );
        assert_eq!(window_meta.display_volume_label, "窗口总流量 BTC");
        assert_eq!(
            window_meta.volume_semantics,
            "single_window_bidirectional_cross_exchange"
        );
        assert!(!window_meta.is_lifecycle_accumulated);
    }

    #[test]
    fn final_event_exposes_volume_semantics_fields() {
        let signal = sample_signal();
        let final_event = FinalEvent::from_contract_signal_with_impact(
            &signal,
            MarketImpactNormalization {
                raw_volume: 4_280.0,
                impact_score: 0.82,
                z_score: 2.14,
                percentile: 93.0,
                normalized_score: 0.88,
                normalized_strength: "EXTREME".to_string(),
                impact_level: "A".to_string(),
                signal_level: "L3".to_string(),
                signal_label: "HIGH IMPACT EVENT".to_string(),
            },
            VolumeDisplayContext::ContractEventStream,
        );

        assert_eq!(final_event.total_volume_btc, 4_280.0);
        assert_eq!(final_event.display_volume_btc, 4_280.0);
        assert_eq!(final_event.display_volume_label, "峰值窗口流量 BTC");
        assert_eq!(
            final_event.volume_semantics,
            "multi_exchange_bidirectional_peak_window"
        );
        assert!(final_event.is_bidirectional_volume);
        assert!(final_event.is_cross_exchange_aggregated);
        assert!(!final_event.is_lifecycle_accumulated);
        assert_eq!(final_event.merged_signal_count, 2);
        assert_eq!(final_event.source_exchange_count, Some(3));
        assert_eq!(
            final_event.source_exchanges,
            vec!["binance", "bitfinex", "okx"]
        );
        assert_eq!(final_event.merged_windows_sec, vec![5, 15]);
        assert_eq!(final_event.buy_volume_btc, Some(1_830.0));
        assert_eq!(final_event.sell_volume_btc, Some(2_450.0));
    }

    fn sample_signal() -> ContractWhaleSignal {
        ContractWhaleSignal {
            id: "contract-whale:BTC:15:1700000000000:buy".to_string(),
            ts: 1_700_000_000_000,
            symbol: "BTC".to_string(),
            window_sec: 15,
            signal_type:
                crate::contract_whale_monitor::types::ContractWhaleSignalType::AggressiveBuy,
            direction: ContractWhaleDirection::Buy,
            severity: ContractWhaleSeverity::High,
            score: 94,
            main_force_score: Some(87),
            spot_score: Some(81),
            contract_score: Some(94),
            base_asset: "BTC".to_string(),
            quantity_unit: "BTC".to_string(),
            total_volume: 4_280.0,
            net_volume: -620.0,
            total_volume_btc: 4_280.0,
            net_volume_btc: -620.0,
            total_notional_usd: 337_000_000.0,
            dominance: 0.676,
            order_price_usd: Some(69_917.0),
            current_market_price_usd: Some(70_000.0),
            price_deviation_pct: Some(0.1186),
            price_deviation_filtered: false,
            price_move_pct: Some(0.31),
            price_move_5s_pct: Some(0.31),
            price_move_15s_pct: Some(0.31),
            price_move_30s_pct: None,
            price_response_type: ContractWhalePriceResponseType::TrendFollowUp,
            classification_v2: Default::default(),
            main_exchange: Some("binance".to_string()),
            market_type: ContractWhaleMarketType::Perp,
            source_role: ContractWhaleSourceRole::Primary,
            exchanges: vec![
                ExchangeFlowContribution {
                    exchange: "binance".to_string(),
                    buy_volume_btc: 1_830.0,
                    sell_volume_btc: 200.0,
                    total_volume_btc: 2_030.0,
                    buy_share: 0.901,
                    sell_share: 0.099,
                    buy_notional_usd: 144_000_000.0,
                    sell_notional_usd: 16_000_000.0,
                    total_notional_usd: 160_000_000.0,
                    net_volume_btc: 1_630.0,
                    dominance: 0.803,
                    net_contribution_share: 0.66,
                    trade_count: 12,
                },
                ExchangeFlowContribution {
                    exchange: "bitfinex".to_string(),
                    buy_volume_btc: 0.0,
                    sell_volume_btc: 1_250.0,
                    total_volume_btc: 1_250.0,
                    buy_share: 0.0,
                    sell_share: 1.0,
                    buy_notional_usd: 0.0,
                    sell_notional_usd: 100_000_000.0,
                    total_notional_usd: 100_000_000.0,
                    net_volume_btc: -1_250.0,
                    dominance: 1.0,
                    net_contribution_share: 0.34,
                    trade_count: 8,
                },
                ExchangeFlowContribution {
                    exchange: "okx".to_string(),
                    buy_volume_btc: 0.0,
                    sell_volume_btc: 1_000.0,
                    total_volume_btc: 1_000.0,
                    buy_share: 0.0,
                    sell_share: 1.0,
                    buy_notional_usd: 0.0,
                    sell_notional_usd: 73_000_000.0,
                    total_notional_usd: 73_000_000.0,
                    net_volume_btc: -1_000.0,
                    dominance: 1.0,
                    net_contribution_share: 0.0,
                    trade_count: 6,
                },
            ],
            dominant_venue_net_contribution_share: Some(0.986),
            dynamic_multiple: Some(9.4),
            dynamic_baseline_btc: Some(512.0),
            dynamic_threshold_level: "critical".to_string(),
            percentile_level: Some(99.9),
            impact_level: Some("S".to_string()),
            signal_level: Some("S".to_string()),
            signal_label: Some("SHOCK IMPACT EVENT".to_string()),
            normalized_strength: Some("EXTREME".to_string()),
            impact_score: Some(9.4),
            impact_z_score: Some(9.4),
            multi_exchange_confirmed: true,
            liquidation_suspected: true,
            liquidation_long_btc: 420.0,
            liquidation_short_btc: 0.0,
            liquidation_notional_usd: 29_400_000.0,
            liquidation_ratio: Some(0.087),
            price_reversal_ratio: Some(0.41),
            oi_change_1m_btc: Some(250.0),
            oi_change_5m_btc: Some(900.0),
            oi_change_pct: Some(1.2),
            oi_bias: Some("rising".to_string()),
            funding_rate: Some(0.00018),
            funding_bias: Some("long".to_string()),
            data_quality: 91,
            score_breakdown: Default::default(),
            threshold_profile: "binance_bitfinex".to_string(),
            threshold_profile_reason: "active_contract_sources=binance,bitfinex".to_string(),
            configured_contract_sources: vec!["binance".to_string(), "bitfinex".to_string()],
            eligible_contract_sources: vec!["binance".to_string(), "bitfinex".to_string()],
            active_contract_sources: vec!["binance".to_string(), "bitfinex".to_string()],
            active_sources: ContractWhaleActiveSources::default(),
            spot_confirmation: ContractWhaleSpotConfirmationContext::default(),
            discord_eligible: true,
            discord_sent: true,
            discord_sent_at: Some(1_700_000_000_050),
            discord_reason: "critical_or_s_gate".to_string(),
            discord_would_send: true,
            final_result: "多平台主动买入爆发，疑似主力合约拉盘".to_string(),
            cluster: ContractWhaleSignalCluster {
                cluster_id: "cwm-cluster:BTC:buy:14166666".to_string(),
                signal_count: 3,
                dominant_intent: "liquidity_probe_buy".to_string(),
                started_at: 1_700_000_000_000,
                updated_at: 1_700_000_090_000,
                duration_ms: 90_000,
                intensity: 0.91,
                price_range_pct: Some(0.18),
            },
            persistence: ContractWhalePersistenceState {
                persistence_score: 0.82,
                signal_half_life_ms: 60_000,
                regime_stability: 0.67,
                redundant_with_previous: true,
                redundant_reason: "same_intent_within_60s".to_string(),
            },
            whale_action: ContractWhaleAction {
                ts: 1_700_000_000_000,
                symbol: "BTC".to_string(),
                action_type: "aggressive_buy".to_string(),
                volume: 3_260.0,
                price_impact: 0.31,
                exchange: "binance".to_string(),
            },
            trajectory: ContractWhaleTrajectory {
                trajectory_id: "whale-trajectory:cwm-cluster:BTC:buy:14166666".to_string(),
                start_ts: 1_700_000_000_000,
                end_ts: 1_700_000_090_000,
                duration_ms: 90_000,
                actions: vec![
                    ContractWhaleAction {
                        ts: 1_700_000_000_000,
                        symbol: "BTC".to_string(),
                        action_type: "liquidity_probe".to_string(),
                        volume: 1_000.0,
                        price_impact: 0.08,
                        exchange: "binance".to_string(),
                    },
                    ContractWhaleAction {
                        ts: 1_700_000_090_000,
                        symbol: "BTC".to_string(),
                        action_type: "aggressive_buy".to_string(),
                        volume: 3_260.0,
                        price_impact: 0.31,
                        exchange: "bitfinex".to_string(),
                    },
                ],
                intent: "accumulation".to_string(),
                regime_path: vec!["manipulation".to_string(), "accumulation".to_string()],
                stealth_profile: ContractWhaleStealthProfile {
                    gamma: 0.73,
                    fragmentation: 0.66,
                    entropy: 0.82,
                    cross_exchange_dispersion: 0.33,
                },
                aggressiveness_curve: vec![0.41, 0.94],
                conclusion: "连续买方压力和承接行为占优，疑似主力分批吸筹。".to_string(),
            },
            liquidation_force: ContractWhaleLiquidationForce {
                active_zone: "short_squeeze_zone".to_string(),
                primary_driver: "liquidation_cascade".to_string(),
                long_liquidation_pressure: 12,
                short_squeeze_pressure: 78,
                stop_hunt_probability: 66,
                cascade_intensity: 73,
                estimated_forced_size_usd: 29_400_000.0,
                zones: vec![ContractWhaleLiquidationZone {
                    side: "short".to_string(),
                    low_price_usd: Some(69_500.0),
                    high_price_usd: Some(70_200.0),
                    estimated_size_usd: 29_400_000.0,
                    intensity: 73,
                    reason: "short squeeze cluster".to_string(),
                }],
                flow_attribution: ContractWhaleForcedFlowAttribution {
                    whale_pct: 0.42,
                    retail_pct: 0.12,
                    liquidation_pct: 0.46,
                    dominant_driver: "liquidation_cascade".to_string(),
                },
                price_impact: ContractWhalePriceImpactAttribution {
                    whale_impact: 0.31,
                    liquidation_cascade: 1.42,
                    stop_loss_sweep: 0.2,
                    passive_absorption: -0.12,
                },
            },
            market_driver: ContractWhaleMarketDriver {
                primary_driver: "liquidity_forcing".to_string(),
                market_state: "liquidity_squeeze_regime".to_string(),
                whale_intent_pct: 0.28,
                liquidity_forcing_pct: 0.52,
                derivatives_pressure_pct: 0.15,
                reflexivity_pct: 0.05,
                components: vec![ContractWhaleMarketDriverComponent {
                    key: "whale_intent".to_string(),
                    score: 28,
                    weight_pct: 0.28,
                }],
                interpretation: "市场主要由主动资金流驱动。".to_string(),
            },
            event_lifecycle: ContractWhaleEventLifecycle {
                event_id: "cwm-event:BTC:downside_absorption:1700000000000".to_string(),
                start_time: 1_700_000_000_000,
                last_update_time: 1_700_000_015_000,
                status: crate::contract_whale_monitor::types::ContractWhaleEventStatus::Closed,
                latest_window_volume_btc: 4_280.0,
                peak_window_volume_btc: 4_280.0,
                unique_turnover_btc: None,
                unique_turnover_available: false,
                unique_turnover_reason: Some("raw_flow_not_enriched".to_string()),
                net_oi_delta_btc: None,
                peak_abs_oi_delta_btc: Some(0.0),
                latest_snapshot_ts: 1_700_000_015_000,
                peak_snapshot_ts: 1_700_000_015_000,
                display_snapshot_kind: "peak".to_string(),
                latest_snapshot: None,
                peak_snapshot: None,
                volume_accumulated: 4_280.0,
                oi_accumulated: 0.0,
                update_count: 3,
            },
            event_quality: ContractWhaleEventQuality {
                quality_score: 0.81,
                merge_similarity_score: 0.84,
                valid: true,
                false_event_flags: Vec::new(),
            },
            read_only: true,
            analysis_only: true,
            execution_enabled: false,
            merged_from: vec!["contract-whale:BTC:5:1700000000010:buy".to_string()],
        }
    }
}
