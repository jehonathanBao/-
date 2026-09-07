//! V4 multi-horizon impact outcomes and empirical forecast snapshots.
//!
//! This module is intentionally deterministic.  It describes the historical
//! effect of a contract-whale event; it is not an execution signal and it
//! never uses observations at or after the event timestamp when building a
//! forecast for that event.

use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};

use super::{
    behavior_assessment::{
        build_detection_behavior, BehaviorDirectionBias, ContractWhaleBehaviorHypothesis,
    },
    types::{
        ContractFlowBucket, ContractFundingSnapshot, ContractLiquidationBucket,
        ContractOiSnapshot, ContractReferencePriceSnapshot, ContractWhaleMarketType,
        ContractWhaleSignal,
    },
};

pub const CONTRACT_WHALE_IMPACT_FORECAST_VERSION: &str = "cwm_impact_v4_1";
pub const HORIZONS_SEC: [u64; 4] = [900, 3_600, 14_400, 86_400];
pub const V4_1_DEFAULT_FEE_BPS: f64 = 6.0;
pub const V4_1_DEFAULT_SLIPPAGE_BPS: f64 = 4.0;
pub const V4_1_DEFAULT_SAFETY_MARGIN_BPS: f64 = 5.0;

fn next_maturity_threshold(samples: usize) -> usize {
    match samples {
        0..=9 => 10,
        10..=29 => 30,
        30..=99 => 100,
        100..=299 => 300,
        _ => samples,
    }
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ContractWhaleStructureBreakEvidence {
    pub structure_break: Option<bool>,
    pub break_direction: Option<String>,
    pub broken_level: Option<f64>,
    pub confirmed_close: Option<f64>,
    pub atr_at_event: Option<f64>,
    pub distance_atr: Option<f64>,
    pub structure_state_before: Option<String>,
    pub structure_state_after: Option<String>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ContractWhaleAnomalyIntensity {
    pub volume_btc: f64,
    pub notional_usd: f64,
    pub percentile: Option<f64>,
    pub robust_z: Option<f64>,
    pub active_direction_share: f64,
    pub duration_sec: u64,
    pub peak_volume_btc: f64,
    pub cumulative_volume_btc: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ContractWhaleDataStreamStatus {
    pub stream: String,
    pub status: String,
    pub last_event_ts: Option<i64>,
    pub last_received_at_ms: Option<i64>,
    pub freshness_ms: Option<i64>,
    pub reconnect_count: Option<u64>,
    pub gap_count: Option<u64>,
    pub parse_failure_count: Option<u64>,
    pub degraded_reason: Option<String>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ContractWhaleTradePlan {
    pub state: String,
    pub reference_price: Option<f64>,
    pub event_high: Option<f64>,
    pub event_low: Option<f64>,
    pub event_vwap: Option<f64>,
    pub structure_high: Option<f64>,
    pub structure_low: Option<f64>,
    pub confirmation_price: Option<f64>,
    pub invalidation_price: Option<f64>,
    pub latest_confirmation_ts: Option<i64>,
    pub confirmation_oi_condition: String,
    pub confirmation_flow_condition: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ContractWhaleV4DecisionState {
    pub event_id: String,
    pub forecast_version: String,
    pub state: String,
    pub reason: String,
    pub updated_at_ms: i64,
    pub decided_at_ms: Option<i64>,
}

pub struct ContractWhaleOutcomeInputs<'a> {
    pub flow_buckets: &'a [ContractFlowBucket],
    pub reference_prices: &'a [ContractReferencePriceSnapshot],
    pub oi_snapshots: &'a [ContractOiSnapshot],
    pub funding_snapshots: &'a [ContractFundingSnapshot],
    pub liquidation_buckets: &'a [ContractLiquidationBucket],
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ContractWhaleHorizonOutcome {
    pub event_id: String,
    pub episode_id: String,
    pub signal_id: String,
    pub symbol: String,
    pub event_ts: i64,
    pub horizon_sec: u64,
    pub direction: String,
    pub behavior: String,
    pub market_regime: String,
    pub intensity_bucket: String,
    pub entry_price: Option<f64>,
    pub end_price: Option<f64>,
    #[serde(default)]
    pub entry_price_source: Option<String>,
    #[serde(default)]
    pub end_price_source: Option<String>,
    pub signed_markout_bps: Option<f64>,
    pub mfe_bps: Option<f64>,
    pub mae_bps: Option<f64>,
    pub follow_through: Option<bool>,
    pub structure_break: Option<bool>,
    #[serde(default)]
    pub structure: ContractWhaleStructureBreakEvidence,
    #[serde(default)]
    pub oi_change_btc: Option<f64>,
    #[serde(default)]
    pub oi_change_pct: Option<f64>,
    #[serde(default)]
    pub funding_change: Option<f64>,
    #[serde(default)]
    pub liquidation_long_btc: Option<f64>,
    #[serde(default)]
    pub liquidation_short_btc: Option<f64>,
    #[serde(default)]
    pub liquidation_observed: Option<bool>,
    #[serde(default)]
    pub liquidation_stream_available: bool,
    #[serde(default)]
    pub reference_price_available: bool,
    #[serde(default)]
    pub price_coverage: f64,
    #[serde(default)]
    pub price_data_degraded: bool,
    #[serde(default)]
    pub missing_evidence: Vec<String>,
    #[serde(default)]
    pub degraded_reasons: Vec<String>,
    pub state: String,
    pub data_quality: u8,
    pub evaluated_at: i64,
    pub outcome_version: String,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ContractWhaleHorizonStats {
    pub horizon: String,
    pub horizon_sec: u64,
    pub sample_count: usize,
    pub median_bps: Option<f64>,
    #[serde(default)]
    pub net_median_bps: Option<f64>,
    pub p25_bps: Option<f64>,
    pub p75_bps: Option<f64>,
    pub mfe_bps: Option<f64>,
    pub mae_bps: Option<f64>,
    pub follow_through_rate: Option<f64>,
    pub structure_break_rate: Option<f64>,
    pub state: String,
    pub baseline_level: String,
    pub cohort_sample_count: usize,
    #[serde(default)]
    pub exact_sample_count: usize,
    #[serde(default)]
    pub behavior_regime_sample_count: usize,
    #[serde(default)]
    pub behavior_sample_count: usize,
    #[serde(default)]
    pub direction_sample_count: usize,
    #[serde(default)]
    pub global_sample_count: usize,
    #[serde(default)]
    pub fallback_sample_count: usize,
    #[serde(default)]
    pub sample_from_ts: Option<i64>,
    #[serde(default)]
    pub sample_to_ts: Option<i64>,
    #[serde(default)]
    pub source_policy: String,
    #[serde(default)]
    pub expected_low_price: Option<f64>,
    #[serde(default)]
    pub expected_high_price: Option<f64>,
    #[serde(default)]
    pub rating: String,
    /// V4.2 separates the live signal from the maturity of the historical
    /// cohort.  These fields are optional so V3/V4.1 payloads remain readable.
    #[serde(default)]
    pub model_median_bps: Option<f64>,
    #[serde(default)]
    pub model_p25_bps: Option<f64>,
    #[serde(default)]
    pub model_p75_bps: Option<f64>,
    #[serde(default)]
    pub model_weight: f64,
    #[serde(default)]
    pub sample_weight: f64,
    #[serde(default)]
    pub raw_sample_count: usize,
    #[serde(default)]
    pub effective_sample_count: f64,
    #[serde(default)]
    pub direction_probability: Option<f64>,
    #[serde(default)]
    pub reversal_probability: Option<f64>,
    #[serde(default)]
    pub structure_break_probability: Option<f64>,
    #[serde(default)]
    pub confirmation_rule: String,
    #[serde(default)]
    pub invalidation_rule: String,
    #[serde(default)]
    pub data_quality: u8,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ContractWhaleMultiHorizonImpactForecast {
    pub forecast_version: String,
    pub source_policy: String,
    #[serde(default)]
    pub data_streams: Vec<String>,
    #[serde(default)]
    pub data_stream_health: Vec<ContractWhaleDataStreamStatus>,
    pub event_id: String,
    pub episode_id: String,
    pub symbol: String,
    pub event_ts: i64,
    #[serde(default)]
    pub local_received_at_ms: i64,
    #[serde(default)]
    pub freshness_ms: i64,
    pub market_regime: String,
    pub behavior: String,
    pub direction: String,
    pub impact_grade: String,
    pub impact_score: f64,
    pub dominant_horizon: String,
    pub scenario_type: String,
    #[serde(default)]
    pub anomaly_intensity: ContractWhaleAnomalyIntensity,
    #[serde(default)]
    pub evidence_strength: u8,
    #[serde(default)]
    pub evidence_complete: bool,
    #[serde(default)]
    pub binance_evidence_complete: bool,
    #[serde(default)]
    pub perp_flow_confirmed: Option<bool>,
    #[serde(default)]
    pub spot_flow_confirmed: Option<bool>,
    #[serde(default)]
    pub oi_confirmed: Option<bool>,
    #[serde(default)]
    pub price_response_confirmed: Option<bool>,
    #[serde(default)]
    pub liquidation_observed: Option<bool>,
    #[serde(default)]
    pub liquidation_stream_available: bool,
    #[serde(default)]
    pub reference_price_available: bool,
    #[serde(default)]
    pub missing_evidence: Vec<String>,
    #[serde(default)]
    pub degraded_reasons: Vec<String>,
    pub exact_sample_count: usize,
    pub effective_sample_count: usize,
    pub baseline_level: String,
    pub maturity_state: String,
    pub status: String,
    pub horizons: Vec<ContractWhaleHorizonStats>,
    pub confirmation: String,
    pub invalidation: String,
    #[serde(default)]
    pub trade_plan: ContractWhaleTradePlan,
    #[serde(default)]
    pub decision_state: String,
    #[serde(default)]
    pub transaction_cost_bps: f64,
    #[serde(default)]
    pub strategy_mode: String,
    #[serde(default)]
    pub production_ready: bool,
    pub training_cutoff_ts: i64,
    pub computed_at_ms: i64,
    /// V4.2 dual-rating fields. `impact_grade` remains populated for older
    /// clients, while `signal_severity` is the immediate, sample-independent
    /// event grade and `maturity_level` describes empirical support.
    #[serde(default)]
    pub signal_severity: String,
    #[serde(default)]
    pub maturity_level: String,
    #[serde(default)]
    pub prediction_source: String,
    #[serde(default)]
    pub model_weight: f64,
    #[serde(default)]
    pub sample_weight: f64,
    #[serde(default)]
    pub raw_sample_count: usize,
    #[serde(default)]
    pub next_maturity_threshold: usize,
    #[serde(default)]
    pub samples_until_next_maturity: usize,
    #[serde(default)]
    pub direction_probability: Option<f64>,
    #[serde(default)]
    pub early_warning: bool,
    #[serde(default)]
    pub external_alert_enabled: bool,
    #[serde(default)]
    pub prior_strength: f64,
}

pub fn event_id(signal: &ContractWhaleSignal) -> String {
    if signal.event_lifecycle.event_id.trim().is_empty() {
        signal.id.clone()
    } else {
        signal.event_lifecycle.event_id.clone()
    }
}

pub fn evaluate_horizon_outcomes(
    signal: &ContractWhaleSignal,
    inputs: ContractWhaleOutcomeInputs<'_>,
    now_ms: i64,
) -> Vec<ContractWhaleHorizonOutcome> {
    let behavior = build_detection_behavior(signal, None, signal.ts);
    let behavior_key = enum_key(behavior.hypothesis);
    let direction = match behavior.direction_bias {
        BehaviorDirectionBias::Bullish => ("bullish", 1.0),
        BehaviorDirectionBias::Bearish => ("bearish", -1.0),
        BehaviorDirectionBias::Neutral => ("neutral", 0.0),
        BehaviorDirectionBias::Unknown => ("unknown", 0.0),
    };
    let flow_prices = weighted_prices(signal, inputs.flow_buckets, now_ms);
    let preferred_source = if matches!(
        behavior.hypothesis,
        ContractWhaleBehaviorHypothesis::LongLiquidationCascade
            | ContractWhaleBehaviorHypothesis::ShortSqueeze
    ) {
        "mark"
    } else {
        "index"
    };
    let reference_path = preferred_reference_path(
        signal,
        inputs.reference_prices,
        preferred_source,
        &flow_prices,
    );
    let entry = reference_price_at_or_before(&reference_path, signal.ts, 120_000)
        .or_else(|| reference_price_near(&reference_path, signal.ts, 120_000))
        .or_else(|| {
            signal
                .order_price_usd
                .filter(|value| value.is_finite() && *value > 0.0)
                .map(|price| (price, "event_vwap".to_string()))
        });
    let entry_price = entry.as_ref().map(|(price, _)| *price);
    let entry_price_source = entry.as_ref().map(|(_, source)| source.clone());
    let regime = market_regime(&signal.market_driver.market_state);
    let intensity = intensity_bucket(signal.score);
    let id = event_id(signal);
    let episode_id = id.clone();

    HORIZONS_SEC
        .into_iter()
        .filter_map(|horizon_sec| {
            let target_ts = signal
                .ts
                .saturating_add((horizon_sec as i64).saturating_mul(1_000));
            if now_ms < target_ts {
                return None;
            }
            let end = reference_price_near(&reference_path, target_ts, freshness_ms(horizon_sec));
            let end_price = end.as_ref().map(|(price, _)| *price);
            let end_price_source = end.as_ref().map(|(_, source)| source.clone());
            let path = reference_path
                .iter()
                .filter(|(ts, _)| *ts >= signal.ts && *ts <= target_ts)
                .map(|(_, (price, _))| *price)
                .collect::<Vec<_>>();
            let signed = |price: Option<f64>| match (entry_price, price, direction.1) {
                (Some(entry), Some(price), direction) if direction != 0.0 => {
                    Some(((price / entry) - 1.0) * 10_000.0 * direction)
                }
                _ => None,
            };
            let signed_path = entry_price.and_then(|entry| {
                (direction.1 != 0.0).then(|| {
                    path.iter()
                        .map(|price| ((price / entry) - 1.0) * 10_000.0 * direction.1)
                        .collect::<Vec<_>>()
                })
            });
            let markout = signed(end_price);
            let mfe = signed_path.as_ref().and_then(|values| {
                values
                    .iter()
                    .copied()
                    .reduce(f64::max)
                    .filter(|value| value.is_finite())
            });
            let mae = signed_path.as_ref().and_then(|values| {
                values
                    .iter()
                    .copied()
                    .reduce(f64::min)
                    .filter(|value| value.is_finite())
            });
            let complete = markout.is_some();
            let structure = structure_break_evidence(
                &reference_path,
                signal.ts,
                target_ts,
                direction.1,
            );
            let (oi_change_btc, oi_change_pct) = oi_change(
                inputs.oi_snapshots,
                &signal.symbol,
                signal.ts,
                target_ts,
            );
            let funding_change = funding_change(
                inputs.funding_snapshots,
                &signal.symbol,
                signal.ts,
                target_ts,
            );
            let liquidation_stream_available = market_stream_available(
                signal,
                ContractWhaleMarketType::Liquidation,
            );
            let (liquidation_long_btc, liquidation_short_btc) = liquidation_sum(
                inputs.liquidation_buckets,
                &signal.symbol,
                signal.ts,
                target_ts,
            );
            let liquidation_observed = liquidation_stream_available.then_some(
                liquidation_long_btc.unwrap_or(0.0) > 0.0
                    || liquidation_short_btc.unwrap_or(0.0) > 0.0,
            );
            let expected_points = (horizon_sec / 60).max(1) as usize;
            let price_coverage = (path.len() as f64 / expected_points as f64).clamp(0.0, 1.0);
            let reference_price_available = entry_price_source
                .as_deref()
                .is_some_and(|source| matches!(source, "index" | "mark"))
                && end_price_source
                    .as_deref()
                    .is_some_and(|source| matches!(source, "index" | "mark"));
            let mut missing_evidence = Vec::new();
            let mut degraded_reasons = Vec::new();
            if !reference_price_available {
                missing_evidence.push("mark_index_reference_price".to_string());
                degraded_reasons.push("reference_price_fallback".to_string());
            }
            if oi_change_btc.is_none() {
                missing_evidence.push("oi_change".to_string());
            }
            if funding_change.is_none() {
                missing_evidence.push("funding_change".to_string());
            }
            if !liquidation_stream_available {
                missing_evidence.push("liquidation_stream".to_string());
            }
            if price_coverage < 0.80 {
                degraded_reasons.push("price_coverage_below_80pct".to_string());
            }
            Some(ContractWhaleHorizonOutcome {
                event_id: id.clone(),
                episode_id: episode_id.clone(),
                signal_id: signal.id.clone(),
                symbol: signal.symbol.clone(),
                event_ts: signal.ts,
                horizon_sec,
                direction: direction.0.to_string(),
                behavior: behavior_key.clone(),
                market_regime: regime.clone(),
                intensity_bucket: intensity.clone(),
                entry_price,
                end_price,
                entry_price_source: entry_price_source.clone(),
                end_price_source,
                signed_markout_bps: markout,
                mfe_bps: mfe,
                mae_bps: mae,
                follow_through: markout.map(|value| value > 0.0),
                structure_break: structure.structure_break,
                structure,
                oi_change_btc,
                oi_change_pct,
                funding_change,
                liquidation_long_btc,
                liquidation_short_btc,
                liquidation_observed,
                liquidation_stream_available,
                reference_price_available,
                price_coverage,
                price_data_degraded: !degraded_reasons.is_empty(),
                missing_evidence,
                degraded_reasons,
                state: if complete {
                    "complete".to_string()
                } else {
                    "insufficient_price_data".to_string()
                },
                data_quality: signal.data_quality,
                evaluated_at: now_ms,
                outcome_version: CONTRACT_WHALE_IMPACT_FORECAST_VERSION.to_string(),
            })
        })
        .collect()
}

pub fn build_forecast(
    signal: &ContractWhaleSignal,
    historical: &[ContractWhaleHorizonOutcome],
    reference_prices: &[ContractReferencePriceSnapshot],
    computed_at_ms: i64,
) -> ContractWhaleMultiHorizonImpactForecast {
    let behavior = build_detection_behavior(signal, None, signal.ts);
    let behavior_key = enum_key(behavior.hypothesis);
    let direction = direction_key(behavior.direction_bias);
    let regime = market_regime(&signal.market_driver.market_state);
    let intensity = intensity_bucket(signal.score);
    // Strictly before the event.  This is the core anti-leakage boundary.
    let historical = historical
        .iter()
        .filter(|outcome| {
            outcome.symbol.eq_ignore_ascii_case(&signal.symbol)
                && outcome.event_ts < signal.ts
                && outcome
                    .event_ts
                    .saturating_add((outcome.horizon_sec as i64).saturating_mul(1_000))
                    <= signal.ts
                && outcome.outcome_version == CONTRACT_WHALE_IMPACT_FORECAST_VERSION
        })
        .collect::<Vec<_>>();
    let transaction_cost_bps = v4_transaction_cost_bps(signal.funding_rate);
    let event_reference = reference_for_forecast(signal, reference_prices);
    let exact_total = historical
        .iter()
        .filter(|outcome| {
            outcome.behavior == behavior_key
                && outcome.market_regime == regime
                && outcome.intensity_bucket == intensity
        })
        .count();

    let mut horizons = Vec::with_capacity(HORIZONS_SEC.len());
    for horizon_sec in HORIZONS_SEC {
        let all_for_horizon = historical
            .iter()
            .filter(|outcome| {
                outcome.horizon_sec == horizon_sec && outcome.signed_markout_bps.is_some()
            })
            .copied()
            .collect::<Vec<_>>();
        let exact_for_horizon = all_for_horizon
            .iter()
            .filter(|outcome| {
                outcome.behavior == behavior_key
                    && outcome.market_regime == regime
                    && outcome.intensity_bucket == intensity
            })
            .copied()
            .collect::<Vec<_>>();
        let behavior_regime_for_horizon =
            filter_group(&all_for_horizon, &behavior_key, &regime, None);
        let behavior_for_horizon =
            filter_group(&all_for_horizon, &behavior_key, &regime, Some("*"));
        let direction_for_horizon = all_for_horizon
            .iter()
            .filter(|outcome| outcome.direction == direction)
            .copied()
            .collect::<Vec<_>>();
        let candidates = [
            ("behavior_regime_intensity", exact_for_horizon.clone()),
            ("behavior_regime", behavior_regime_for_horizon.clone()),
            ("behavior", behavior_for_horizon.clone()),
            ("direction", direction_for_horizon.clone()),
            ("global", all_for_horizon.clone()),
        ];
        let (baseline_level, sample) = choose_cohort(&candidates);
        horizons.push(stats_for(
            horizon_sec,
            sample,
            baseline_level,
            exact_for_horizon.len(),
            event_reference.as_ref().map(|(price, _)| *price),
            transaction_cost_bps,
            direction.as_str(),
            behavior_regime_for_horizon.len(),
            behavior_for_horizon.len(),
            direction_for_horizon.len(),
            all_for_horizon.len(),
        ));
    }

    // Effective N is the sample size of the selected decision horizon, not
    // the largest sample size among all horizons. Otherwise a mature 15m
    // cohort could incorrectly unlock a 4h/1d conclusion with no support.
    let dominant_horizon = dominant_horizon(&horizons);
    let effective_sample_count = horizons
        .iter()
        .find(|item| item.horizon == dominant_horizon)
        .map(|item| item.sample_count)
        .unwrap_or(0);
    let exact_sample_count = exact_total / HORIZONS_SEC.len().max(1);
    let maturity_state = maturity_state(effective_sample_count).to_string();
    let impact_score = impact_score(&horizons, direction.as_str());
    let scenario_type = scenario_type(&behavior.hypothesis, &horizons, direction.as_str());
    let liquidation_stream_available = market_stream_available(
        signal,
        ContractWhaleMarketType::Liquidation,
    );
    let perp_flow_confirmed = Some(
        signal
            .main_exchange
            .as_deref()
            .is_some_and(|exchange| exchange.eq_ignore_ascii_case("binance"))
            || signal
                .active_contract_sources
                .iter()
                .any(|exchange| exchange.eq_ignore_ascii_case("binance")),
    );
    let spot_flow_confirmed = market_stream_available(signal, ContractWhaleMarketType::Spot)
        .then_some(signal.spot_confirmation.score > 0);
    let oi_confirmed = signal
        .classification_v2
        .oi_available
        .then_some(signal.classification_v2.oi_delta_pct.is_some());
    let price_response_confirmed = Some(
        signal.price_response_type
            != super::types::ContractWhalePriceResponseType::NoClearResponse,
    );
    let liquidation_observed = liquidation_stream_available.then_some(
        signal.liquidation_long_btc > 0.0 || signal.liquidation_short_btc > 0.0,
    );
    let reference_price_available = event_reference
        .as_ref()
        .is_some_and(|(_, source)| matches!(source.as_str(), "index" | "mark"));
    let mut missing_evidence = Vec::new();
    if spot_flow_confirmed.is_none() {
        missing_evidence.push("binance_spot_flow".to_string());
    }
    if oi_confirmed.is_none() {
        missing_evidence.push("binance_oi".to_string());
    }
    if !liquidation_stream_available {
        missing_evidence.push("binance_force_order".to_string());
    }
    if !reference_price_available {
        missing_evidence.push("binance_mark_index".to_string());
    }
    let mut degraded_reasons = Vec::new();
    if signal.classification_v2.evidence.evidence_degraded
        || signal.classification_v2.oi_evidence_degraded
    {
        degraded_reasons.push("signal_evidence_degraded".to_string());
    }
    if signal.data_quality < 70 {
        degraded_reasons.push("data_quality_below_70".to_string());
    }
    let evidence_complete = perp_flow_confirmed == Some(true)
        && spot_flow_confirmed.is_some()
        && oi_confirmed.is_some()
        && price_response_confirmed.is_some()
        && liquidation_stream_available
        && reference_price_available
        && degraded_reasons.is_empty();
    let impact_grade = impact_grade(
        impact_score,
        effective_sample_count,
        &horizons,
        direction.as_str(),
        evidence_complete,
    );
    let status = if effective_sample_count == 0 {
        "insufficient_data".to_string()
    } else {
        maturity_state.clone()
    };
    let baseline_level = horizons
        .iter()
        .filter(|item| item.sample_count > 0)
        .max_by_key(|item| item.sample_count)
        .map(|item| item.baseline_level.clone())
        .unwrap_or_else(|| "none".to_string());
    let prior_levels = prior_structure_levels(reference_prices, signal.ts);
    let trade_plan = build_trade_plan(
        signal,
        event_reference.as_ref().map(|(price, _)| *price),
        prior_levels,
        &direction,
        effective_sample_count,
        evidence_complete,
        computed_at_ms,
    );
    let behavior_assessment = build_detection_behavior(signal, None, signal.ts);
    let confirmation = confirmation_text(&behavior_key, &direction, &trade_plan);
    let invalidation = invalidation_text(&behavior_key, &direction, &trade_plan);
    ContractWhaleMultiHorizonImpactForecast {
        forecast_version: CONTRACT_WHALE_IMPACT_FORECAST_VERSION.to_string(),
        source_policy: "binance_only".to_string(),
        data_streams: forecast_data_streams(signal, reference_price_available),
        data_stream_health: forecast_data_stream_health(signal, reference_prices),
        event_id: event_id(signal),
        episode_id: event_id(signal),
        symbol: signal.symbol.clone(),
        event_ts: signal.ts,
        local_received_at_ms: signal
            .event_lifecycle
            .latest_snapshot_ts
            .max(signal.ts),
        freshness_ms: signal
            .event_lifecycle
            .latest_snapshot_ts
            .max(signal.ts)
            .saturating_sub(signal.ts),
        market_regime: regime,
        behavior: behavior_key,
        direction,
        impact_grade: impact_grade.clone(),
        impact_score,
        dominant_horizon,
        scenario_type,
        anomaly_intensity: anomaly_intensity(signal),
        evidence_strength: behavior_assessment.confidence_score,
        evidence_complete,
        binance_evidence_complete: evidence_complete,
        perp_flow_confirmed,
        spot_flow_confirmed,
        oi_confirmed,
        price_response_confirmed,
        liquidation_observed,
        liquidation_stream_available,
        reference_price_available,
        missing_evidence,
        degraded_reasons,
        exact_sample_count,
        effective_sample_count,
        baseline_level,
        maturity_state: maturity_state.clone(),
        status,
        horizons,
        confirmation,
        invalidation,
        trade_plan,
        decision_state: if effective_sample_count == 0 {
            "no_trade".to_string()
        } else {
            "awaiting_confirmation".to_string()
        },
        transaction_cost_bps,
        strategy_mode: "experimental_shadow".to_string(),
        production_ready: false,
        training_cutoff_ts: signal.ts.saturating_sub(1),
        computed_at_ms,
        signal_severity: impact_grade.clone(),
        maturity_level: maturity_state.clone(),
        prediction_source: "historical_validated".to_string(),
        model_weight: 0.0,
        sample_weight: if effective_sample_count > 0 { 1.0 } else { 0.0 },
        raw_sample_count: effective_sample_count,
        next_maturity_threshold: next_maturity_threshold(effective_sample_count),
        samples_until_next_maturity: next_maturity_threshold(effective_sample_count).saturating_sub(effective_sample_count),
        direction_probability: None,
        early_warning: false,
        external_alert_enabled: false,
        prior_strength: 0.0,
    }
}

/// Evaluate the event's executable state from concrete price/structure/OI
/// conditions. This state is persisted separately; the T0 forecast payload
/// itself remains immutable.
pub fn evaluate_trade_plan_state(
    forecast: &ContractWhaleMultiHorizonImpactForecast,
    outcomes: &[ContractWhaleHorizonOutcome],
    now_ms: i64,
) -> (String, String) {
    if forecast.effective_sample_count == 0 || forecast.direction == "unknown" {
        return ("no_trade".to_string(), "sample_or_direction_insufficient".to_string());
    }
    if matches!(forecast.behavior.as_str(), "long_liquidation_cascade" | "short_squeeze") {
        return ("no_trade".to_string(), "forced_flow_is_risk_warning_only".to_string());
    }
    let plan = &forecast.trade_plan;
    let latest = outcomes
        .iter()
        .filter(|outcome| outcome.state == "complete" && outcome.end_price.is_some())
        .max_by_key(|outcome| outcome.horizon_sec);
    let Some(latest) = latest else {
        if now_ms >= forecast.event_ts.saturating_add(86_400_000) {
            return ("expired_unconfirmed".to_string(), "confirmation_window_expired".to_string());
        }
        return ("awaiting_confirmation".to_string(), "waiting_for_closed_reference_price".to_string());
    };
    let price = latest.end_price.unwrap_or_default();
    let confirmation = plan.confirmation_price;
    let invalidation = plan.invalidation_price;
    let bullish = forecast.direction == "bullish";
    let oi_aligned = latest.oi_change_btc.is_some_and(|value| value >= 0.0);
    let structure_confirmed = latest.structure_break == Some(true);
    if let Some(level) = invalidation {
        if (bullish && price <= level) || (!bullish && price >= level) {
            return ("invalidated".to_string(), "price_crossed_invalidation_boundary".to_string());
        }
    }
    if let Some(level) = confirmation {
        let price_confirmed = (bullish && price >= level) || (!bullish && price <= level);
        if price_confirmed && oi_aligned && structure_confirmed && forecast.evidence_complete {
            return ("confirmed".to_string(), "closed_boundary_oi_structure_evidence_aligned".to_string());
        }
    }
    if now_ms >= forecast.event_ts.saturating_add(86_400_000) {
        ("expired_unconfirmed".to_string(), "confirmation_window_expired".to_string())
    } else {
        ("awaiting_confirmation".to_string(), "confirmation_conditions_not_all_met".to_string())
    }
}

fn weighted_prices(
    signal: &ContractWhaleSignal,
    buckets: &[ContractFlowBucket],
    now_ms: i64,
) -> Vec<(i64, f64)> {
    let max_horizon = HORIZONS_SEC.iter().copied().max().unwrap_or(0) as i64 * 1_000;
    let structure_lookback = 60 * 60 * 1_000;
    let mut grouped = BTreeMap::<i64, (f64, f64)>::new();
    for bucket in buckets {
        if !bucket.symbol.eq_ignore_ascii_case(&signal.symbol)
            || !bucket.exchange.eq_ignore_ascii_case("binance")
            || bucket.ts_bucket < signal.ts.saturating_sub(structure_lookback)
            || bucket.ts_bucket > now_ms.min(signal.ts.saturating_add(max_horizon))
        {
            continue;
        }
        let Some(price) = bucket
            .vwap
            .filter(|value| value.is_finite() && *value > 0.0)
        else {
            continue;
        };
        let volume = (bucket.buy_volume_btc + bucket.sell_volume_btc).max(0.0);
        if volume <= f64::EPSILON {
            continue;
        }
        let entry = grouped.entry(bucket.ts_bucket).or_default();
        entry.0 += price * volume;
        entry.1 += volume;
    }
    grouped
        .into_iter()
        .filter(|(_, (_, volume))| *volume > f64::EPSILON)
        .map(|(ts, (weighted, volume))| (ts, weighted / volume))
        .collect()
}

type ReferencePath = Vec<(i64, (f64, String))>;

fn preferred_reference_path(
    signal: &ContractWhaleSignal,
    snapshots: &[ContractReferencePriceSnapshot],
    preferred_source: &str,
    flow_prices: &[(i64, f64)],
) -> ReferencePath {
    let mut grouped = BTreeMap::<i64, BTreeMap<String, f64>>::new();
    for snapshot in snapshots.iter().filter(|snapshot| {
        snapshot.exchange == super::types::ContractExchange::Binance
            && snapshot.symbol.eq_ignore_ascii_case(&signal.symbol)
            && snapshot.price.is_finite()
            && snapshot.price > 0.0
    }) {
        grouped
            .entry(snapshot.ts_bucket)
            .or_default()
            .insert(snapshot.price_source.clone(), snapshot.price);
    }
    let source_order = if preferred_source == "mark" {
        ["mark", "index", "futures_last", "spot"]
    } else {
        ["index", "mark", "futures_last", "spot"]
    };
    let mut path = grouped
        .into_iter()
        .filter_map(|(ts, values)| {
            source_order.iter().find_map(|source| {
                values
                    .get(*source)
                    .copied()
                    .map(|price| (ts, (price, (*source).to_string())))
            })
        })
        .collect::<BTreeMap<_, _>>();
    for (ts, price) in flow_prices {
        path.entry(*ts)
            .or_insert((*price, "perp_vwap".to_string()));
    }
    path.into_iter().collect()
}

fn reference_price_at_or_before(
    path: &ReferencePath,
    target_ts: i64,
    freshness_ms: i64,
) -> Option<(f64, String)> {
    path.iter()
        .filter(|(ts, (price, _))| {
            *ts <= target_ts
                && target_ts.saturating_sub(*ts) <= freshness_ms
                && price.is_finite()
                && *price > 0.0
        })
        .max_by_key(|(ts, _)| *ts)
        .map(|(_, (price, source))| (*price, source.clone()))
}

fn reference_price_near(
    path: &ReferencePath,
    target_ts: i64,
    freshness_ms: i64,
) -> Option<(f64, String)> {
    path.iter()
        .filter(|(ts, (price, _))| {
            ts.abs_diff(target_ts) <= freshness_ms.max(0) as u64
                && price.is_finite()
                && *price > 0.0
        })
        .min_by_key(|(ts, _)| ts.abs_diff(target_ts))
        .map(|(_, (price, source))| (*price, source.clone()))
}

/// A conservative, deterministic structure label based only on the preceding
/// Binance reference-price window.  This is deliberately a label, not a
/// prediction: it records whether the post-event path broke the prior local
/// range by a small volatility-aware buffer.
fn structure_break_evidence(
    prices: &ReferencePath,
    event_ts: i64,
    target_ts: i64,
    direction: f64,
) -> ContractWhaleStructureBreakEvidence {
    if direction == 0.0 {
        return ContractWhaleStructureBreakEvidence::default();
    }
    let prior = prices
        .iter()
        .filter(|(ts, (price, _))| {
            *ts < event_ts
                && *ts >= event_ts.saturating_sub(4 * 60 * 60 * 1_000)
                && price.is_finite()
                && *price > 0.0
        })
        .map(|(_, (price, _))| *price)
        .collect::<Vec<_>>();
    let post = prices
        .iter()
        .filter(|(ts, (price, _))| {
            *ts >= event_ts
                && *ts <= target_ts
                && price.is_finite()
                && *price > 0.0
        })
        .map(|(_, (price, _))| *price)
        .collect::<Vec<_>>();
    if prior.len() < 20 || post.len() < 2 {
        return ContractWhaleStructureBreakEvidence::default();
    }
    let Some(prior_high) = prior.iter().copied().reduce(f64::max) else {
        return ContractWhaleStructureBreakEvidence::default();
    };
    let Some(prior_low) = prior.iter().copied().reduce(f64::min) else {
        return ContractWhaleStructureBreakEvidence::default();
    };
    let atr = prior
        .windows(2)
        .map(|window| (window[1] - window[0]).abs())
        .filter(|value| value.is_finite())
        .sum::<f64>()
        / prior.len().saturating_sub(1).max(1) as f64;
    let buffer = atr.max((prior_high + prior_low) * 0.00025);
    let threshold = if direction > 0.0 {
        prior_high + buffer
    } else {
        prior_low - buffer
    };
    let confirmed_close = post
        .iter()
        .copied()
        .find(|price| if direction > 0.0 { *price >= threshold } else { *price <= threshold });
    let did_break = confirmed_close.is_some();
    let final_close = post.last().copied();
    let prior_change = prior
        .first()
        .zip(prior.last())
        .map(|(first, last)| (last / first - 1.0).abs())
        .unwrap_or(0.0);
    let state_before = if atr / prior.last().copied().unwrap_or(1.0) > 0.002 {
        "high_volatility"
    } else if prior_change > 0.006 {
        "trend"
    } else {
        "range"
    };
    let distance_atr = confirmed_close.map(|close| {
        if atr > f64::EPSILON {
            (close - if direction > 0.0 { prior_high } else { prior_low }).abs() / atr
        } else {
            0.0
        }
    });
    ContractWhaleStructureBreakEvidence {
        structure_break: Some(did_break),
        break_direction: did_break.then(|| if direction > 0.0 { "up" } else { "down" }.to_string()),
        broken_level: did_break.then_some(if direction > 0.0 { prior_high } else { prior_low }),
        confirmed_close,
        atr_at_event: atr.is_finite().then_some(atr),
        distance_atr,
        structure_state_before: Some(state_before.to_string()),
        structure_state_after: Some(if did_break {
            if direction > 0.0 { "bullish_break" } else { "bearish_break" }
        } else if final_close.is_some() {
            "range_held"
        } else {
            "unavailable"
        }.to_string()),
    }
}

fn freshness_ms(horizon_sec: u64) -> i64 {
    if horizon_sec >= 14_400 { 180_000 } else { 120_000 }
}

fn nearest_oi<'a>(
    values: &'a [ContractOiSnapshot],
    symbol: &str,
    target_ts: i64,
    max_gap_ms: i64,
) -> Option<&'a ContractOiSnapshot> {
    values
        .iter()
        .filter(|value| {
            value.exchange == super::types::ContractExchange::Binance
                && value.symbol.eq_ignore_ascii_case(symbol)
                && value.ts.abs_diff(target_ts) <= max_gap_ms.max(0) as u64
        })
        .min_by_key(|value| value.ts.abs_diff(target_ts))
}

fn oi_change(
    values: &[ContractOiSnapshot],
    symbol: &str,
    event_ts: i64,
    target_ts: i64,
) -> (Option<f64>, Option<f64>) {
    let start = nearest_oi(values, symbol, event_ts, 120_000);
    let end = nearest_oi(values, symbol, target_ts, 120_000);
    match (start, end) {
        (Some(start), Some(end)) if start.oi_btc > 0.0 => {
            let delta = end.oi_btc - start.oi_btc;
            (Some(delta), Some(delta / start.oi_btc * 100.0))
        }
        _ => (None, None),
    }
}

fn funding_change(
    values: &[ContractFundingSnapshot],
    symbol: &str,
    event_ts: i64,
    target_ts: i64,
) -> Option<f64> {
    let nearest = |target: i64| {
        values
            .iter()
            .filter(|value| {
                value.exchange == super::types::ContractExchange::Binance
                    && value.symbol.eq_ignore_ascii_case(symbol)
                    && value.ts.abs_diff(target) <= 15 * 60 * 1_000
            })
            .min_by_key(|value| value.ts.abs_diff(target))
    };
    nearest(event_ts)
        .zip(nearest(target_ts))
        .map(|(start, end)| end.funding_rate - start.funding_rate)
}

fn liquidation_sum(
    values: &[ContractLiquidationBucket],
    symbol: &str,
    event_ts: i64,
    target_ts: i64,
) -> (Option<f64>, Option<f64>) {
    let matching = values.iter().filter(|value| {
        value.exchange.eq_ignore_ascii_case("binance")
            && value.symbol.eq_ignore_ascii_case(symbol)
            && value.ts_bucket >= event_ts
            && value.ts_bucket <= target_ts
    });
    let (mut long, mut short, mut count) = (0.0, 0.0, 0usize);
    for value in matching {
        long += value.long_liq_btc.max(0.0);
        short += value.short_liq_btc.max(0.0);
        count += 1;
    }
    if count == 0 {
        (None, None)
    } else {
        (Some(long), Some(short))
    }
}

fn market_stream_available(signal: &ContractWhaleSignal, market: ContractWhaleMarketType) -> bool {
    signal
        .active_sources
        .contract
        .iter()
        .chain(signal.active_sources.spot.iter())
        .any(|entry| {
            entry.exchange.eq_ignore_ascii_case("binance")
                && entry.market_type == market
                && entry.enabled
                && !matches!(entry.status.as_str(), "disabled" | "unavailable" | "stale")
        })
}

fn reference_for_forecast(
    signal: &ContractWhaleSignal,
    reference_prices: &[ContractReferencePriceSnapshot],
) -> Option<(f64, String)> {
    let path = preferred_reference_path(signal, reference_prices, "index", &[]);
    reference_price_at_or_before(&path, signal.ts, 120_000)
        .or_else(|| reference_price_near(&path, signal.ts, 120_000))
        .or_else(|| {
            signal
                .order_price_usd
                .filter(|value| value.is_finite() && *value > 0.0)
                .map(|value| (value, "event_vwap".to_string()))
        })
}

fn prior_structure_levels(
    reference_prices: &[ContractReferencePriceSnapshot],
    event_ts: i64,
) -> (Option<f64>, Option<f64>) {
    let values = reference_prices
        .iter()
        .filter(|value| {
            value.exchange == super::types::ContractExchange::Binance
                && matches!(value.price_source.as_str(), "index" | "mark")
                && value.ts_bucket < event_ts
                && value.ts_bucket >= event_ts.saturating_sub(4 * 60 * 60 * 1_000)
                && value.price.is_finite()
                && value.price > 0.0
        })
        .map(|value| value.price)
        .collect::<Vec<_>>();
    (
        values.iter().copied().reduce(f64::max),
        values.iter().copied().reduce(f64::min),
    )
}

fn build_trade_plan(
    signal: &ContractWhaleSignal,
    reference_price: Option<f64>,
    (structure_high, structure_low): (Option<f64>, Option<f64>),
    direction: &str,
    samples: usize,
    evidence_complete: bool,
    computed_at_ms: i64,
) -> ContractWhaleTradePlan {
    let event_vwap = signal.order_price_usd.or(reference_price);
    let event_move = signal.price_move_pct.unwrap_or(0.0).abs() / 100.0;
    let event_high = event_vwap.map(|price| price * (1.0 + event_move));
    let event_low = event_vwap.map(|price| price * (1.0 - event_move));
    let bullish = direction == "bullish";
    let confirmation_price = if bullish {
        structure_high.or(event_high)
    } else {
        structure_low.or(event_low)
    };
    let invalidation_price = if bullish {
        structure_low.or(event_low)
    } else {
        structure_high.or(event_high)
    };
    ContractWhaleTradePlan {
        state: if samples < 30 {
            "observe"
        } else if !evidence_complete {
            "awaiting_confirmation"
        } else {
            "awaiting_confirmation"
        }
        .to_string(),
        reference_price,
        event_high,
        event_low,
        event_vwap,
        structure_high,
        structure_low,
        confirmation_price,
        invalidation_price,
        latest_confirmation_ts: Some(computed_at_ms.saturating_add(15 * 60 * 1_000)),
        confirmation_oi_condition: if bullish {
            "OI不下降且与价格突破方向一致"
        } else {
            "OI不下降且与价格跌破方向一致"
        }
        .to_string(),
        confirmation_flow_condition: "Binance现货与永续主动流同向，且参考价格数据完整"
            .to_string(),
    }
}

fn confirmation_text(
    behavior: &str,
    direction: &str,
    plan: &ContractWhaleTradePlan,
) -> String {
    let boundary = plan
        .confirmation_price
        .map(|value| format!("{value:.2}"))
        .unwrap_or_else(|| "结构边界不可用".to_string());
    if matches!(behavior, "downside_absorption" | "upside_suppression") {
        format!("已收盘价格收复/跌破吸收区边界 {boundary}，后续主动流减弱，OI不再逆向扩张")
    } else {
        format!(
            "已收盘价格{} {boundary}，OI与方向一致，Binance现货和永续同向",
            if direction == "bullish" { "突破" } else { "跌破" }
        )
    }
}

fn invalidation_text(
    behavior: &str,
    direction: &str,
    plan: &ContractWhaleTradePlan,
) -> String {
    let boundary = plan
        .invalidation_price
        .map(|value| format!("{value:.2}"))
        .unwrap_or_else(|| "事件区间边界不可用".to_string());
    if matches!(behavior, "downside_absorption" | "upside_suppression") {
        format!("吸收区被有效击穿 {boundary}，或OI继续沿主动流方向扩张")
    } else {
        format!(
            "已收盘价格{} {boundary}，或OI/现货与行为假设反向，或数据质量降级",
            if direction == "bullish" { "跌破" } else { "突破" }
        )
    }
}

fn anomaly_intensity(signal: &ContractWhaleSignal) -> ContractWhaleAnomalyIntensity {
    ContractWhaleAnomalyIntensity {
        volume_btc: signal.total_volume_btc,
        notional_usd: signal.total_notional_usd,
        percentile: signal.percentile_level,
        robust_z: signal.impact_z_score,
        active_direction_share: signal.dominance.clamp(0.0, 1.0),
        duration_sec: signal.window_sec,
        peak_volume_btc: signal.event_lifecycle.peak_window_volume_btc,
        cumulative_volume_btc: signal.event_lifecycle.volume_accumulated,
    }
}

fn forecast_data_streams(
    signal: &ContractWhaleSignal,
    reference_price_available: bool,
) -> Vec<String> {
    let mut streams = vec!["binance_futures_agg_trade".to_string()];
    if market_stream_available(signal, ContractWhaleMarketType::Spot) {
        streams.push("binance_spot_agg_trade".to_string());
    }
    if signal.classification_v2.oi_available {
        streams.push("binance_open_interest".to_string());
    }
    if signal.funding_rate.is_some() {
        streams.push("binance_funding".to_string());
    }
    if market_stream_available(signal, ContractWhaleMarketType::Liquidation) {
        streams.push("binance_force_order".to_string());
    }
    if reference_price_available {
        streams.extend(["binance_mark_price".to_string(), "binance_index_price".to_string()]);
    }
    streams
}

fn forecast_data_stream_health(
    signal: &ContractWhaleSignal,
    reference_prices: &[ContractReferencePriceSnapshot],
) -> Vec<ContractWhaleDataStreamStatus> {
    let received_at = signal
        .event_lifecycle
        .latest_snapshot_ts
        .max(signal.ts);
    let mut health = Vec::new();
    let mut push = |stream: &str, available: bool, last_event_ts: Option<i64>, reason: Option<&str>| {
        health.push(ContractWhaleDataStreamStatus {
            stream: stream.to_string(),
            status: if available { "available" } else { "unavailable" }.to_string(),
            last_event_ts,
            last_received_at_ms: available.then_some(received_at),
            freshness_ms: last_event_ts.map(|ts| received_at.saturating_sub(ts).max(0)),
            reconnect_count: None,
            gap_count: None,
            parse_failure_count: None,
            degraded_reason: reason.map(str::to_string),
        });
    };
    let binance_perp = signal.active_contract_sources.iter().any(|source| {
        source.eq_ignore_ascii_case("binance")
    }) || signal
        .main_exchange
        .as_deref()
        .is_some_and(|source| source.eq_ignore_ascii_case("binance"));
    let binance_spot = signal.active_sources.spot.iter().any(|source| {
        source.exchange.eq_ignore_ascii_case("binance") && source.enabled
    });
    let last_reference = reference_prices
        .iter()
        .filter(|row| matches!(row.price_source.as_str(), "mark" | "index"))
        .map(|row| row.event_time_ms)
        .max();
    push(
        "binance_futures_agg_trade",
        binance_perp,
        binance_perp.then_some(signal.ts),
        (!binance_perp).then_some("binance_perp_source_unavailable"),
    );
    push(
        "binance_spot_agg_trade",
        binance_spot,
        signal.spot_confirmation.latest_signal_at,
        (!binance_spot).then_some("binance_spot_source_unavailable"),
    );
    push(
        "binance_mark_index",
        last_reference.is_some(),
        last_reference,
        last_reference.is_none().then_some("mark_index_not_available"),
    );
    push(
        "binance_open_interest",
        signal.classification_v2.oi_available,
        signal.oi_change_1m_btc.map(|_| signal.ts),
        (!signal.classification_v2.oi_available).then_some("oi_unavailable"),
    );
    push(
        "binance_funding",
        signal.funding_rate.is_some(),
        signal.funding_rate.map(|_| signal.ts),
        signal.funding_rate.is_none().then_some("funding_unavailable"),
    );
    let force_available = market_stream_available(signal, ContractWhaleMarketType::Liquidation);
    push(
        "binance_force_order",
        force_available,
        force_available.then_some(signal.ts),
        (!force_available).then_some("force_order_stream_unavailable"),
    );
    push(
        "binance_book_ticker",
        false,
        None,
        Some("book_ticker_not_ingested_by_v4_1"),
    );
    health
}

fn v4_transaction_cost_bps(funding_rate: Option<f64>) -> f64 {
    let config = super::config::contract_whale_runtime_config().impact_v4_1;
    config.fee_bps
        + config.slippage_bps
        + config.safety_margin_bps
        + funding_rate.unwrap_or(0.0).abs() * 10_000.0
}

fn enum_key<T: Serialize>(value: T) -> String {
    serde_json::to_string(&value)
        .unwrap_or_else(|_| "unknown".to_string())
        .trim_matches('"')
        .to_string()
}

fn direction_key(value: BehaviorDirectionBias) -> String {
    match value {
        BehaviorDirectionBias::Bullish => "bullish",
        BehaviorDirectionBias::Bearish => "bearish",
        BehaviorDirectionBias::Neutral => "neutral",
        BehaviorDirectionBias::Unknown => "unknown",
    }
    .to_string()
}

pub fn market_regime(value: &str) -> String {
    let value = value.to_ascii_lowercase();
    if value.contains("liquid") || value.contains("cascade") || value.contains("squeeze") {
        "liquidation"
    } else if value.contains("high_vol") || value.contains("volatile") {
        "high_volatility"
    } else if value.contains("bear") || value.contains("distribution") || value.contains("dump") {
        "bear_trend"
    } else if value.contains("bull") || value.contains("expansion") || value.contains("push") {
        "bull_trend"
    } else if value.contains("range") || value.contains("neutral") {
        "range"
    } else {
        "unclear"
    }
    .to_string()
}

fn intensity_bucket(score: u8) -> String {
    match score {
        0..=49 => "normal",
        50..=79 => "high",
        _ => "extreme",
    }
    .to_string()
}

fn filter_group<'a>(
    values: &[&'a ContractWhaleHorizonOutcome],
    behavior: &str,
    regime: &str,
    regime_wildcard: Option<&str>,
) -> Vec<&'a ContractWhaleHorizonOutcome> {
    values
        .iter()
        .filter(|outcome| {
            outcome.behavior == behavior && regime_wildcard.is_some_and(|wildcard| wildcard == "*")
                || (outcome.behavior == behavior
                    && regime_wildcard.is_none()
                    && outcome.market_regime == regime)
        })
        .copied()
        .collect()
}

fn choose_cohort<'a>(
    candidates: &[(&'static str, Vec<&'a ContractWhaleHorizonOutcome>)],
) -> (String, Vec<&'a ContractWhaleHorizonOutcome>) {
    candidates
        .iter()
        .find(|(_, values)| values.len() >= 30)
        .or_else(|| candidates.iter().find(|(_, values)| !values.is_empty()))
        .map(|(level, values)| ((*level).to_string(), values.clone()))
        .unwrap_or_else(|| ("none".to_string(), Vec::new()))
}

fn stats_for(
    horizon_sec: u64,
    values: Vec<&ContractWhaleHorizonOutcome>,
    baseline_level: String,
    exact_count: usize,
    reference_price: Option<f64>,
    transaction_cost_bps: f64,
    direction: &str,
    behavior_regime_count: usize,
    behavior_count: usize,
    direction_count: usize,
    global_count: usize,
) -> ContractWhaleHorizonStats {
    let markouts = values
        .iter()
        .filter_map(|value| value.signed_markout_bps)
        .collect::<Vec<_>>();
    let follow = values
        .iter()
        .filter_map(|value| value.follow_through)
        .collect::<Vec<_>>();
    let mfes = values
        .iter()
        .filter_map(|value| value.mfe_bps)
        .collect::<Vec<_>>();
    let maes = values
        .iter()
        .filter_map(|value| value.mae_bps)
        .collect::<Vec<_>>();
    let structure = values
        .iter()
        .filter_map(|value| value.structure_break)
        .collect::<Vec<_>>();
    let sample_count = markouts.len();
    let sample_from_ts = values.iter().map(|value| value.event_ts).min();
    let sample_to_ts = values.iter().map(|value| value.event_ts).max();
    let median_bps = quantile(markouts.clone(), 0.50);
    let net_median_bps = median_bps.map(|value| value - transaction_cost_bps);
    let p25_bps = quantile(markouts.clone(), 0.25);
    let p75_bps = quantile(markouts, 0.75);
    let price_direction = if direction == "bearish" { -1.0 } else { 1.0 };
    let expected_prices = reference_price
        .zip(p25_bps)
        .zip(p75_bps)
        .map(|((price, p25), p75)| {
            let first = price * (1.0 + price_direction * p25 / 10_000.0);
            let second = price * (1.0 + price_direction * p75 / 10_000.0);
            (first.min(second), first.max(second))
        });
    let expected_low_price = expected_prices.map(|(low, _)| low);
    let expected_high_price = expected_prices.map(|(_, high)| high);
    let follow_through_rate = rate(&follow, true);
    let rating = horizon_rating(sample_count, net_median_bps, follow_through_rate);
    ContractWhaleHorizonStats {
        horizon: horizon_label(horizon_sec).to_string(),
        horizon_sec,
        sample_count,
        median_bps,
        net_median_bps,
        p25_bps,
        p75_bps,
        mfe_bps: quantile(mfes, 0.50),
        mae_bps: quantile(maes, 0.50),
        follow_through_rate,
        structure_break_rate: rate(&structure, true),
        state: maturity_state(sample_count).to_string(),
        baseline_level,
        cohort_sample_count: exact_count,
        exact_sample_count: exact_count,
        behavior_regime_sample_count: behavior_regime_count,
        behavior_sample_count: behavior_count,
        direction_sample_count: direction_count,
        global_sample_count: global_count,
        fallback_sample_count: sample_count.saturating_sub(exact_count),
        sample_from_ts,
        sample_to_ts,
        source_policy: "binance_only".to_string(),
        expected_low_price,
        expected_high_price,
        rating,
        model_median_bps: None,
        model_p25_bps: None,
        model_p75_bps: None,
        model_weight: 0.0,
        sample_weight: if sample_count > 0 { 1.0 } else { 0.0 },
        raw_sample_count: sample_count,
        effective_sample_count: sample_count as f64,
        direction_probability: None,
        reversal_probability: None,
        structure_break_probability: None,
        confirmation_rule: String::new(),
        invalidation_rule: String::new(),
        data_quality: 0,
    }
}

fn horizon_rating(
    sample_count: usize,
    net_median_bps: Option<f64>,
    follow_through_rate: Option<f64>,
) -> String {
    let min_follow_through =
        super::config::contract_whale_runtime_config().impact_v4_1.min_follow_through_rate;
    if sample_count < 30 {
        return "U".to_string();
    }
    if net_median_bps.unwrap_or(f64::NEG_INFINITY) <= 0.0
        || follow_through_rate.unwrap_or(0.0) < min_follow_through
    {
        return "C".to_string();
    }
    if sample_count < 100 {
        "B"
    } else if sample_count < 300 {
        "A"
    } else {
        "S"
    }
    .to_string()
}

fn quantile(mut values: Vec<f64>, probability: f64) -> Option<f64> {
    values.retain(|value| value.is_finite());
    if values.is_empty() {
        return None;
    }
    values.sort_by(f64::total_cmp);
    let index = ((values.len() - 1) as f64 * probability).round() as usize;
    values.get(index).copied()
}

fn rate(values: &[bool], positive: bool) -> Option<f64> {
    if values.is_empty() {
        return None;
    }
    Some(values.iter().filter(|value| **value == positive).count() as f64 / values.len() as f64)
}

pub fn maturity_state(sample_count: usize) -> &'static str {
    match sample_count {
        0..=29 => "insufficient_data",
        30..=99 => "warming_up",
        100..=299 => "provisional",
        _ => "stable",
    }
}

fn impact_score(horizons: &[ContractWhaleHorizonStats], direction: &str) -> f64 {
    if matches!(direction, "unknown" | "neutral") {
        return 0.0;
    }
    horizons
        .iter()
        .filter(|item| item.sample_count >= 30)
        .filter_map(|item| Some(item.median_bps?.abs() * item.follow_through_rate.unwrap_or(0.0)))
        .max_by(f64::total_cmp)
        .map(|value| (value / 30.0 * 100.0).clamp(0.0, 100.0))
        .unwrap_or(0.0)
}

fn dominant_horizon(horizons: &[ContractWhaleHorizonStats]) -> String {
    horizons
        .iter()
        .filter(|item| {
            item.sample_count >= 30 && item.median_bps.is_some() && item.follow_through_rate.is_some()
        })
        .max_by(|left, right| {
            let left_score =
                left.median_bps.unwrap_or(0.0).abs() * left.follow_through_rate.unwrap_or(0.0);
            let right_score =
                right.median_bps.unwrap_or(0.0).abs() * right.follow_through_rate.unwrap_or(0.0);
            left_score.total_cmp(&right_score)
        })
        .map(|item| item.horizon.clone())
        .unwrap_or_else(|| "unknown".to_string())
}

fn scenario_type(
    hypothesis: &ContractWhaleBehaviorHypothesis,
    horizons: &[ContractWhaleHorizonStats],
    direction: &str,
) -> String {
    // A behavior label is only a hypothesis until at least one horizon has a
    // usable cohort. Keep the UI from presenting a mechanism label as a
    // validated market-impact conclusion while the model is warming up.
    if !horizons.iter().any(|item| item.sample_count >= 30) {
        return "no_clear_edge".to_string();
    }
    match hypothesis {
        ContractWhaleBehaviorHypothesis::LongLiquidationCascade => {
            return "liquidation_cascade".to_string()
        }
        ContractWhaleBehaviorHypothesis::ShortSqueeze => return "short_squeeze".to_string(),
        ContractWhaleBehaviorHypothesis::DownsideAbsorption
        | ContractWhaleBehaviorHypothesis::UpsideSuppression => {
            return "absorption_reversal".to_string()
        }
        _ => {}
    }
    let valid = horizons
        .iter()
        .filter(|item| item.sample_count >= 30)
        .filter_map(|item| item.median_bps)
        .collect::<Vec<_>>();
    if valid.is_empty() || matches!(direction, "unknown" | "neutral") {
        return "no_clear_edge".to_string();
    }
    let adverse = valid.iter().filter(|value| **value < 0.0).count();
    let positive = valid.iter().filter(|value| **value > 0.0).count();
    let adjacent_reversal = horizons.windows(2).any(|pair| {
        let min_structure =
            super::config::contract_whale_runtime_config().impact_v4_1.min_structure_break_rate;
        pair.iter().all(|item| {
            item.sample_count >= 30
                && item.median_bps.is_some_and(|value| value < 0.0)
                && item.structure_break_rate.unwrap_or(0.0) >= min_structure
        })
    });
    if adverse >= 2 && adjacent_reversal {
        "structural_reversal_candidate".to_string()
    } else if adverse > positive {
        "trend_exhaustion".to_string()
    } else if positive >= 2 {
        "trend_continuation".to_string()
    } else {
        "local_pullback".to_string()
    }
}

fn impact_grade(
    score: f64,
    samples: usize,
    horizons: &[ContractWhaleHorizonStats],
    direction: &str,
    evidence_complete: bool,
) -> String {
    let min_follow_through =
        super::config::contract_whale_runtime_config().impact_v4_1.min_follow_through_rate;
    if samples < 30 || score <= 0.0 || matches!(direction, "unknown" | "neutral") {
        return "U".to_string();
    }
    let has_net_edge = horizons.iter().any(|item| {
        item.sample_count >= 30
            && item.net_median_bps.is_some_and(|value| value > 0.0)
            && item.follow_through_rate.unwrap_or(0.0) >= min_follow_through
    });
    if !has_net_edge {
        return "C".to_string();
    }
    let grade = if score >= 82.0 {
        "S"
    } else if score >= 68.0 {
        "A"
    } else if score >= 48.0 {
        "B"
    } else {
        "C"
    };
    let max_grade = if samples < 100 {
        "B"
    } else if samples < 300 {
        "A"
    } else {
        "S"
    };
    let order = ["U", "C", "B", "A", "S"];
    let grade_index = order.iter().position(|value| *value == grade).unwrap_or(0);
    let cap_index = order
        .iter()
        .position(|value| *value == max_grade)
        .unwrap_or(0);
    if grade == "S"
        && horizons
            .iter()
            .filter(|item| item.sample_count >= 300 && item.median_bps.is_some())
            .count()
            < 2
    {
        return "A".to_string();
    }
    if grade == "S" && !evidence_complete {
        return "A".to_string();
    }
    order[grade_index.min(cap_index)].to_string()
}

pub fn horizon_label(horizon_sec: u64) -> &'static str {
    match horizon_sec {
        900 => "15m",
        3_600 => "1h",
        14_400 => "4h",
        86_400 => "1d",
        _ => "unknown",
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn maturity_caps_are_explicit() {
        assert_eq!(maturity_state(0), "insufficient_data");
        assert_eq!(maturity_state(30), "warming_up");
        assert_eq!(maturity_state(100), "provisional");
        assert_eq!(maturity_state(300), "stable");
    }

    #[test]
    fn sell_direction_signs_markout_and_excursions() {
        let entry = 100.0;
        let path: [f64; 3] = [99.0, 98.0, 101.0];
        let signed = path
            .iter()
            .map(|price| (price / entry - 1.0) * 10_000.0 * -1.0)
            .collect::<Vec<_>>();
        assert!((signed[0] - 100.0).abs() < 1e-9);
        assert!((signed[2] + 100.0).abs() < 1e-9);
        assert!((signed.iter().copied().reduce(f64::max).unwrap_or_default() - 200.0).abs() < 1e-9);
        assert!((signed.iter().copied().reduce(f64::min).unwrap_or_default() + 100.0).abs() < 1e-9);
    }

    #[test]
    fn forecast_history_is_strictly_before_event() {
        let signal = serde_json::from_value::<ContractWhaleSignal>(serde_json::json!({
            "id":"v4-test", "ts":100000, "symbol":"BTC", "windowSec":15,
            "signalType":"aggressive_sell", "direction":"sell", "severity":"high", "score":80,
            "totalVolumeBtc":100, "netVolumeBtc":-80, "totalNotionalUsd":8000000, "dominance":0.8,
            "mainExchange":null, "exchanges":[], "dataQuality":90,
            "discordEligible":false, "discordSent":false, "discordReason":"", "finalResult":"",
            "readOnly":true, "analysisOnly":true, "executionEnabled":false
        }))
        .unwrap();
        let future = ContractWhaleHorizonOutcome {
            event_id: "future".into(),
            episode_id: "future".into(),
            signal_id: "future".into(),
            symbol: "BTC".into(),
            event_ts: 100000,
            horizon_sec: 900,
            direction: "bearish".into(),
            behavior: "active_sell_pressure".into(),
            market_regime: "unclear".into(),
            intensity_bucket: "extreme".into(),
            entry_price: Some(100.0),
            end_price: Some(99.0),
            entry_price_source: Some("index".into()),
            end_price_source: Some("index".into()),
            signed_markout_bps: Some(100.0),
            mfe_bps: Some(100.0),
            mae_bps: Some(0.0),
            follow_through: Some(true),
            structure_break: None,
            structure: ContractWhaleStructureBreakEvidence::default(),
            oi_change_btc: None,
            oi_change_pct: None,
            funding_change: None,
            liquidation_long_btc: None,
            liquidation_short_btc: None,
            liquidation_observed: None,
            liquidation_stream_available: false,
            reference_price_available: true,
            price_coverage: 1.0,
            price_data_degraded: false,
            missing_evidence: Vec::new(),
            degraded_reasons: Vec::new(),
            state: "complete".into(),
            data_quality: 90,
            evaluated_at: 101000,
            outcome_version: CONTRACT_WHALE_IMPACT_FORECAST_VERSION.into(),
        };
        let forecast = build_forecast(&signal, &[future], &[], 200000);
        assert_eq!(forecast.effective_sample_count, 0);
        assert_eq!(forecast.training_cutoff_ts, 99999);
    }

    #[test]
    fn forecast_rejects_prior_event_whose_horizon_ends_after_signal() {
        let signal = serde_json::from_value::<ContractWhaleSignal>(serde_json::json!({
            "id":"anti-leak", "ts":1_000_000, "symbol":"BTC", "windowSec":15,
            "signalType":"aggressive_sell", "direction":"sell", "severity":"high", "score":80,
            "totalVolumeBtc":100, "netVolumeBtc":-80, "totalNotionalUsd":8000000, "dominance":0.8,
            "mainExchange":"binance", "exchanges":[], "dataQuality":90,
            "discordEligible":false, "discordSent":false, "discordReason":"", "finalResult":"",
            "readOnly":true, "analysisOnly":true, "executionEnabled":false
        }))
        .unwrap();
        let leaking = ContractWhaleHorizonOutcome {
            event_id: "prior-but-unfinished".into(),
            episode_id: "prior-but-unfinished".into(),
            signal_id: "prior-but-unfinished".into(),
            symbol: "BTC".into(),
            event_ts: 900_000,
            horizon_sec: 900,
            direction: "bearish".into(),
            behavior: "active_sell_pressure".into(),
            market_regime: "unclear".into(),
            intensity_bucket: "extreme".into(),
            entry_price: Some(100.0),
            end_price: Some(99.0),
            entry_price_source: Some("index".into()),
            end_price_source: Some("index".into()),
            signed_markout_bps: Some(100.0),
            mfe_bps: Some(100.0),
            mae_bps: Some(0.0),
            follow_through: Some(true),
            structure_break: Some(true),
            structure: ContractWhaleStructureBreakEvidence::default(),
            oi_change_btc: None,
            oi_change_pct: None,
            funding_change: None,
            liquidation_long_btc: None,
            liquidation_short_btc: None,
            liquidation_observed: None,
            liquidation_stream_available: false,
            reference_price_available: true,
            price_coverage: 1.0,
            price_data_degraded: false,
            missing_evidence: Vec::new(),
            degraded_reasons: Vec::new(),
            state: "complete".into(),
            data_quality: 90,
            evaluated_at: 1_800_000,
            outcome_version: CONTRACT_WHALE_IMPACT_FORECAST_VERSION.into(),
        };
        let forecast = build_forecast(&signal, &[leaking], &[], 1_000_001);
        assert_eq!(forecast.effective_sample_count, 0);
        assert!(forecast
            .horizons
            .iter()
            .all(|horizon| horizon.sample_count == 0));
    }
}
