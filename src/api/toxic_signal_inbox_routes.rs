use axum::{
    extract::{Path, Query, State},
    Json,
};
use serde::Deserialize;

use crate::{
    api::{
        contract_whale_routes::build_contract_whale_response_with_runtime,
        discord_notification_routes::{
            discord_alert_status_for_key, evaluate_discord_alert_gate,
            preferred_discord_alert_family, DiscordAlertMode, DiscordNotificationRequest,
        },
        market_structure_transport::{
            market_structure_evidence_available, SplitRiskSystemsTransport,
        },
        toxic_quality_scorecard_routes::build_fusion_recent,
    },
    app::AppState,
    contract_whale_monitor::{config::contract_whale_runtime_config, types::ContractWhaleSignal},
    runtime::{
        advanced_tof_metrics::{build_advanced_tof_metrics, AdvancedTofInput},
        cwm_risk_fusion::{
            build_cwm_risk_contribution_for_candidate, build_split_risk_systems,
            cwm_signal_is_fresh_at, SplitRiskSystemsInput,
        },
        perp_tof_metrics::{
            build_perp_tof_metrics, build_perp_tof_metrics_from_observed,
            observed_perp_snapshot_from_cwm, PerpTofInput,
        },
        tof_metrics::{
            enhance_signal_summary, enhance_signal_with_observed, ObservedTofSnapshot,
            TofSummaryInput,
        },
    },
    storage::contract_whale_repo::{ContractWhaleRepo, ContractWhaleSignalQuery},
    toxicity::{
        toxic_governance_ledger_service::toxic_governance_ledger_summary,
        toxic_markout_service::toxic_markout_recent,
        toxic_quality_scorecard_service::toxic_quality_scorecard_summary,
        toxic_replay_service::replay_recent,
        toxic_signal_inbox_service::{
            toxic_signal_inbox_by_signal_id, toxic_signal_inbox_recent, toxic_signal_inbox_status,
        },
        toxic_weight_recommendation_service::toxic_weight_recommendation_summary,
    },
    types::toxic_signal_inbox::ToxicSignalInboxRecentResponse,
};

#[derive(Debug, Deserialize)]
pub struct ToxicSignalInboxQuery {
    symbol: Option<String>,
}

pub async fn toxic_signal_inbox_status_route(
    State(state): State<AppState>,
    Query(query): Query<ToxicSignalInboxQuery>,
) -> Json<serde_json::Value> {
    let requested_symbol = normalize_symbol_query(query.symbol, &state.config().symbol);
    let recent = build_recent(&state, &requested_symbol);
    Json(with_runtime_contract(
        with_filter_contract(
            serde_json::json!(toxic_signal_inbox_status(&recent)),
            &requested_symbol,
        ),
        &state,
    ))
}

pub async fn toxic_signal_inbox_recent_route(
    State(state): State<AppState>,
    Query(query): Query<ToxicSignalInboxQuery>,
) -> Json<serde_json::Value> {
    let requested_symbol = normalize_symbol_query(query.symbol, &state.config().symbol);
    let cwm_signal = latest_cwm_signal_for_state(&state, &requested_symbol);
    Json(with_runtime_contract(
        with_filter_contract(
            with_tof_metrics_contract(
                with_trigger_prices(
                    serde_json::json!(build_recent(&state, &requested_symbol)),
                    &state,
                ),
                &state,
                cwm_signal.as_ref(),
            ),
            &requested_symbol,
        ),
        &state,
    ))
}

pub async fn toxic_signal_inbox_for_symbol(
    State(state): State<AppState>,
    Path(symbol): Path<String>,
) -> Json<serde_json::Value> {
    let requested_symbol = normalize_symbol_text(&symbol, &state.config().symbol);
    let cwm_signal = latest_cwm_signal_for_state(&state, &requested_symbol);
    Json(with_runtime_contract(
        with_filter_contract(
            with_tof_metrics_contract(
                with_trigger_prices(
                    serde_json::json!(build_recent(&state, &requested_symbol)),
                    &state,
                ),
                &state,
                cwm_signal.as_ref(),
            ),
            &requested_symbol,
        ),
        &state,
    ))
}

pub async fn toxic_signal_inbox_for_signal(
    State(state): State<AppState>,
    Path(signal_id): Path<String>,
    Query(query): Query<ToxicSignalInboxQuery>,
) -> Json<serde_json::Value> {
    let requested_symbol = normalize_symbol_query(query.symbol, &state.config().symbol);
    let recent = build_recent(&state, &requested_symbol);
    let cwm_signal = latest_cwm_signal_for_state(&state, &requested_symbol);
    Json(with_runtime_contract(
        with_tof_metrics_contract(
            with_trigger_prices(
                serde_json::json!(toxic_signal_inbox_by_signal_id(
                    &requested_symbol,
                    &signal_id,
                    &recent,
                )),
                &state,
            ),
            &state,
            cwm_signal.as_ref(),
        ),
        &state,
    ))
}

pub(crate) fn build_recent(
    state: &AppState,
    requested_symbol: &str,
) -> ToxicSignalInboxRecentResponse {
    let fusion_recent = build_fusion_recent(state, requested_symbol);
    let replay_recent = replay_recent(requested_symbol, &fusion_recent);
    let markout_recent = toxic_markout_recent(
        requested_symbol,
        &fusion_recent,
        |ts| state.price_snapshot_at_or_before_for_symbol(ts, requested_symbol),
        |ts| state.price_snapshots_since_for_symbol(ts, requested_symbol),
    );
    let quality_summary = toxic_quality_scorecard_summary(
        requested_symbol,
        &fusion_recent,
        |ts| state.price_snapshot_at_or_before_for_symbol(ts, requested_symbol),
        |ts| state.price_snapshots_since_for_symbol(ts, requested_symbol),
    );
    let recommendation_summary = toxic_weight_recommendation_summary(
        requested_symbol,
        &fusion_recent,
        |ts| state.price_snapshot_at_or_before_for_symbol(ts, requested_symbol),
        |ts| state.price_snapshots_since_for_symbol(ts, requested_symbol),
    );
    let governance_summary = toxic_governance_ledger_summary(Some(requested_symbol));

    toxic_signal_inbox_recent(
        requested_symbol,
        &fusion_recent,
        &replay_recent,
        &markout_recent,
        &quality_summary,
        &recommendation_summary,
        &governance_summary,
    )
}

pub(crate) fn latest_cwm_signal_for_state(
    state: &AppState,
    requested_symbol: &str,
) -> Option<ContractWhaleSignal> {
    let now_ms = crate::normalizers::trade::now_ms();
    let cwm_symbol = cwm_symbol_from_requested_symbol(requested_symbol);
    if !state.config().contract_whale_monitor.enabled
        || !contract_whale_runtime_config().symbol_enabled(&cwm_symbol)
    {
        return None;
    }
    if let Some(signal) = state.contract_whale_store().and_then(|store| {
        store
            .query_contract_whale_signals(&ContractWhaleSignalQuery {
                symbol: Some(cwm_symbol.clone()),
                limit: 1,
                ..ContractWhaleSignalQuery::default()
            })
            .ok()
            .and_then(|signals| signals.into_iter().next())
    }) {
        if cwm_signal_is_fresh_at(signal.ts, now_ms) {
            return Some(signal);
        }
    }
    let response = build_contract_whale_response_with_runtime(
        &state.flow_state_for_symbol(requested_symbol),
        &cwm_symbol,
        1,
        None,
        state.config().contract_whale_monitor.enabled,
        state.config().contract_whale_monitor.dry_run,
        Some(&state.venue_health()),
    );
    response
        .items
        .into_iter()
        .find(|signal| cwm_signal_is_fresh_at(signal.ts, now_ms))
}

fn cwm_symbol_from_requested_symbol(symbol: &str) -> String {
    crate::normalizers::symbol::canonical_base_asset(symbol).unwrap_or_else(|| "BTC".to_string())
}

pub(crate) fn normalize_symbol_query(symbol: Option<String>, default_symbol: &str) -> String {
    match symbol {
        Some(symbol) => normalize_symbol_text(&symbol, default_symbol),
        None => normalize_symbol_text(default_symbol, default_symbol),
    }
}

pub(crate) fn normalize_symbol_text(symbol: &str, default_symbol: &str) -> String {
    let trimmed = symbol.trim();
    if trimmed.is_empty() {
        default_symbol.trim().to_ascii_uppercase()
    } else {
        trimmed.to_ascii_uppercase()
    }
}

pub(crate) fn with_filter_contract(
    mut payload: serde_json::Value,
    requested_symbol: &str,
) -> serde_json::Value {
    if let Some(object) = payload.as_object_mut() {
        object.insert(
            "filter".to_string(),
            serde_json::json!({
                "symbol": requested_symbol,
                "viewOnly": true,
                "persistentWatchlistEnabled": false,
                "runtimeMonitorModified": false,
            }),
        );
    }
    payload
}

fn with_runtime_contract(mut payload: serde_json::Value, state: &AppState) -> serde_json::Value {
    if let Some(object) = payload.as_object_mut() {
        object.insert(
            "monitoringStarted".to_string(),
            serde_json::json!(state.runtime_started()),
        );
    }
    payload
}

pub(crate) fn with_tof_metrics_contract(
    mut payload: serde_json::Value,
    state: &AppState,
    cwm_signal: Option<&ContractWhaleSignal>,
) -> serde_json::Value {
    if let Some(items) = payload
        .get_mut("items")
        .and_then(|value| value.as_array_mut())
    {
        for item in items {
            decorate_item_with_tof(item, state, cwm_signal);
        }
    }
    if let Some(item) = payload.get_mut("item") {
        decorate_item_with_tof(item, state, cwm_signal);
    }
    payload
}

pub(crate) fn with_trigger_prices(
    mut payload: serde_json::Value,
    state: &AppState,
) -> serde_json::Value {
    if let Some(items) = payload
        .get_mut("items")
        .and_then(|value| value.as_array_mut())
    {
        for item in items {
            decorate_item_with_trigger_price(item, state);
        }
    }
    if let Some(item) = payload.get_mut("item") {
        decorate_item_with_trigger_price(item, state);
    }
    payload
}

fn decorate_item_with_trigger_price(item: &mut serde_json::Value, state: &AppState) {
    let Some(object) = item.as_object_mut() else {
        return;
    };
    let monitoring_started = state.runtime_started();
    object.insert(
        "monitoringStarted".to_string(),
        serde_json::json!(monitoring_started),
    );
    if object
        .get("triggerPriceUsd")
        .and_then(positive_number_from_value)
        .is_some()
    {
        return;
    }
    let Some(created_at_ms) = object.get("createdAtMs").and_then(i64_from_json_number) else {
        return;
    };
    let symbol = object
        .get("symbol")
        .and_then(|value| value.as_str())
        .unwrap_or(&state.config().symbol);
    let Some(price) = state
        .price_snapshot_at_or_before_for_symbol(created_at_ms, symbol)
        .map(|snapshot| snapshot.index_mid)
        .filter(|value| value.is_finite() && *value > 0.0)
    else {
        return;
    };
    object.insert(
        "triggerPriceUsd".to_string(),
        serde_json::json!(round_price(price)),
    );
}

fn i64_from_json_number(value: &serde_json::Value) -> Option<i64> {
    value
        .as_i64()
        .or_else(|| value.as_u64().and_then(|number| i64::try_from(number).ok()))
}

fn positive_number_from_value(value: &serde_json::Value) -> Option<f64> {
    value
        .as_f64()
        .filter(|number| number.is_finite() && *number > 0.0)
}

fn round_price(value: f64) -> f64 {
    (value * 100.0).round() / 100.0
}

pub(crate) fn observed_tof_snapshot_for_state(
    state: &AppState,
    requested_symbol: &str,
) -> Option<ObservedTofSnapshot> {
    let flow = state.flow_state_for_symbol(requested_symbol);
    if !flow.symbol.eq_ignore_ascii_case(requested_symbol) {
        return None;
    }
    let window = flow
        .windows
        .values()
        .filter(|window| window.symbol.eq_ignore_ascii_case(requested_symbol))
        .filter(|window| window.data_quality.has_trades || window.data_quality.has_books)
        .min_by_key(|window| window.window_ms)?;

    let vpin = state.vpin_state();
    let vpin_matches = vpin.symbol.eq_ignore_ascii_case(requested_symbol);
    let sweep = state.sweep_state();
    let sweep_matches = sweep.symbol.eq_ignore_ascii_case(requested_symbol);
    let sweep_result = sweep_matches
        .then(|| {
            sweep
                .results
                .values()
                .filter(|result| result.symbol.eq_ignore_ascii_case(requested_symbol))
                .min_by_key(|result| result.window_ms)
        })
        .flatten();
    let liquidity = sweep_result.and_then(|result| result.liquidity.as_ref());
    let has_l2 = window.data_quality.has_books || sweep.quality.has_books;

    let mut source_times = vec![flow.updated_at];
    if vpin_matches {
        source_times.push(vpin.updated_at);
    }
    if sweep_matches {
        source_times.push(sweep.updated_at);
    }
    let observed_at_ms = source_times
        .into_iter()
        .filter(|value| *value > 0)
        .min()
        .unwrap_or(flow.updated_at);
    let vpin_window_volume = if vpin_matches {
        vpin.recent_buckets
            .iter()
            .rev()
            .take(vpin.metrics.lookback_buckets)
            .map(|bucket| bucket.total_btc)
            .sum()
    } else {
        0.0
    };

    Some(ObservedTofSnapshot {
        symbol: requested_symbol.trim().to_ascii_uppercase(),
        observed_at_ms,
        buy_volume: window.aggressive_buy_btc,
        sell_volume: window.aggressive_sell_btc,
        trade_count: window.trade_count,
        window_ms: window.window_ms,
        vpin: vpin_matches.then_some(vpin.metrics.vpin).flatten(),
        vpin_zscore: vpin_matches.then_some(vpin.metrics.vpin_zscore).flatten(),
        vpin_percentile: vpin_matches
            .then_some(vpin.metrics.vpin_percentile)
            .flatten(),
        vpin_bucket_count: if vpin_matches {
            vpin.metrics.completed_bucket_count
        } else {
            0
        },
        vpin_window_volume,
        per_venue_vpin: if vpin_matches {
            vpin.metrics.per_venue_vpin.clone()
        } else {
            std::collections::BTreeMap::new()
        },
        bid_depth_withdrawal: has_l2
            .then(|| liquidity.and_then(|value| value.bid_depth_drop_ratio))
            .flatten()
            .map(|value| value * 100.0),
        ask_depth_withdrawal: has_l2
            .then(|| liquidity.and_then(|value| value.ask_depth_drop_ratio))
            .flatten()
            .map(|value| value * 100.0),
        spread_bps: window
            .data_quality
            .has_books
            .then_some(window.spread_bps_median)
            .flatten(),
        book_update_rate: None,
        sweep_score: sweep_result.map(|result| if result.sweep_detected { 100.0 } else { 0.0 }),
    })
}

fn decorate_item_with_tof(
    item: &mut serde_json::Value,
    state: &AppState,
    cwm_signal: Option<&ContractWhaleSignal>,
) {
    decorate_item_with_tof_observation(item, state, cwm_signal, None);
}

fn decorate_item_with_tof_observation(
    item: &mut serde_json::Value,
    state: &AppState,
    cwm_signal: Option<&ContractWhaleSignal>,
    observed_tof_override: Option<&ObservedTofSnapshot>,
) {
    let Some(object) = item.as_object_mut() else {
        return;
    };
    let monitoring_started = state.runtime_started();
    let signal_kind = object
        .get("signalKind")
        .and_then(|value| value.as_str())
        .unwrap_or("unknown")
        .to_string();
    let direction_bias = object
        .get("directionBias")
        .and_then(|value| value.as_str())
        .unwrap_or("neutral")
        .to_string();
    let severity = object
        .get("severity")
        .and_then(|value| value.as_str())
        .unwrap_or("low")
        .to_string();
    let confidence = object
        .get("confidence")
        .and_then(|value| value.as_f64())
        .unwrap_or(0.35);
    let quality_bucket = object
        .get("quality")
        .and_then(|value| value.get("qualityBucket"))
        .and_then(|value| value.as_str())
        .unwrap_or("not_enough_data")
        .to_string();
    let summary = object
        .get("fusion")
        .and_then(|value| value.get("summary"))
        .and_then(|value| value.as_str())
        .unwrap_or("candidate signal")
        .to_string();
    let symbol = object
        .get("symbol")
        .and_then(|value| value.as_str())
        .unwrap_or("UNKNOWN")
        .to_string();
    let existing_risk_score = object
        .get("riskScore")
        .and_then(|value| value.as_u64())
        .and_then(|value| u8::try_from(value).ok())
        .unwrap_or(0);
    let existing_data_quality = object
        .get("dataQualityScore")
        .and_then(|value| value.as_f64())
        .filter(|value| value.is_finite() && *value >= 0.0 && *value <= 100.0);
    let summary_input = TofSummaryInput {
        signal_kind: &signal_kind,
        direction_bias: &direction_bias,
        severity: &severity,
        confidence,
        quality_bucket: &quality_bucket,
        summary: &summary,
        existing_risk_score,
        existing_data_quality: existing_data_quality.unwrap_or(0.0),
    };
    let created_at_ms = object
        .get("createdAtMs")
        .and_then(i64_from_json_number)
        .unwrap_or_else(crate::normalizers::trade::now_ms);
    let now_ms = crate::normalizers::trade::now_ms();
    let observed_tof = observed_tof_override
        .cloned()
        .or_else(|| observed_tof_snapshot_for_state(state, &symbol));
    let enhancement = observed_tof
        .as_ref()
        .map(|snapshot| {
            enhance_signal_with_observed(&summary_input, snapshot, created_at_ms, now_ms)
        })
        .unwrap_or_else(|| enhance_signal_summary(&summary_input));
    let candidate_type = enhancement.candidate_type.clone();
    let explain_tags = enhancement.explain_tags.clone();
    let direction_label = enhancement.direction_label.clone();
    let direction_source = enhancement.direction_source.clone();
    let perp_input = PerpTofInput {
        symbol: &symbol,
        spot_candidate_type: &candidate_type,
        spot_direction: enhancement.direction,
        spot_risk_score: enhancement.final_risk_score,
        spot_data_quality: existing_data_quality.unwrap_or(0.0),
        spot_confidence: confidence,
        summary: &summary,
    };
    let observed_perp =
        cwm_signal.and_then(|signal| observed_perp_snapshot_from_cwm(&symbol, signal));
    let perp_metrics = observed_perp
        .as_ref()
        .map(|snapshot| {
            build_perp_tof_metrics_from_observed(snapshot, &symbol, created_at_ms, now_ms)
        })
        .unwrap_or_else(|| build_perp_tof_metrics(&perp_input));
    let advanced_metrics = build_advanced_tof_metrics(&AdvancedTofInput {
        symbol: &symbol,
        spot_candidate_type: &candidate_type,
        spot_direction: enhancement.direction,
        spot_risk_score: existing_risk_score,
        spot_data_quality: existing_data_quality.unwrap_or(0.0),
        spot_confidence: confidence,
        tof_metrics: &enhancement.tof_metrics,
        spot_tags: &explain_tags,
        perp_metrics: &perp_metrics,
        summary: &summary,
    });
    let merged_tags = advanced_metrics.explain_tags.clone();
    let advanced_score = advanced_metrics.final_risk_score;
    let authoritative_data_quality = existing_data_quality;
    let advanced_candidate_type = advanced_metrics.candidate_type.clone();
    let perp_score = perp_metrics.risk_score;
    let perp_candidate_type = perp_metrics.candidate_type.clone();
    let final_candidate_type = advanced_metrics.final_candidate_type.clone();
    let metrics_direction = serde_json::to_value(advanced_metrics.metrics_direction)
        .ok()
        .and_then(|value| value.as_str().map(str::to_string));
    let cwm_contribution =
        build_cwm_risk_contribution_for_candidate(&symbol, cwm_signal, created_at_ms, now_ms);
    let behavior_type = cwm_signal.and_then(|signal| {
        serde_json::to_value(signal.behavior_assessment.behavior_type)
            .ok()
            .and_then(|value| value.as_str().map(str::to_string))
    });
    let behavior_state = cwm_signal.and_then(|signal| {
        serde_json::to_value(signal.behavior_assessment.state)
            .ok()
            .and_then(|value| value.as_str().map(str::to_string))
    });
    let behavior_confidence = cwm_signal.map(|signal| signal.behavior_assessment.confidence);
    let behavior_main_force_confirmed =
        cwm_signal.map(|signal| signal.behavior_assessment.main_force_confirmed);
    let risk_systems = build_split_risk_systems(SplitRiskSystemsInput {
        ts_ms: created_at_ms,
        symbol: &symbol,
        short_toxic_score: enhancement.final_risk_score,
        short_tof_score: enhancement.tof_score,
        short_direction: enhancement.direction,
        toxic_type: &candidate_type,
        data_quality: authoritative_data_quality.unwrap_or(0.0),
        detector_confidence: (confidence * 100.0).clamp(0.0, 100.0),
        direction_confidence: enhancement.direction_confidence,
        direction_source: &direction_source,
        tof_metrics: &enhancement.tof_metrics,
        advanced_score: advanced_metrics
            .lineage
            .alert_eligible
            .then_some(advanced_score),
        perp_score: perp_metrics.lineage.alert_eligible.then_some(perp_score),
        metrics_direction: advanced_metrics.metrics_direction,
        cwm_contribution: cwm_contribution.clone(),
    });
    let market_structure_available =
        market_structure_evidence_available(&enhancement.tof_metrics, &cwm_contribution);
    let risk_systems_transport =
        SplitRiskSystemsTransport::from_core(&risk_systems, market_structure_available);
    let market_structure = risk_systems_transport.main_force_structure.as_ref();
    let toxic_score = risk_systems.short_term_toxic.toxic_score;
    object.insert(
        "tofMetrics".to_string(),
        serde_json::to_value(&enhancement.tof_metrics).unwrap_or(serde_json::Value::Null),
    );
    object.insert(
        "candidateType".to_string(),
        serde_json::json!(advanced_candidate_type.clone()),
    );
    object.insert(
        "explainTags".to_string(),
        serde_json::json!(merged_tags.clone()),
    );
    object.insert(
        "direction".to_string(),
        serde_json::json!(enhancement.direction),
    );
    object.insert(
        "directionLabel".to_string(),
        serde_json::json!(direction_label),
    );
    object.insert(
        "directionConfidence".to_string(),
        serde_json::json!(enhancement.direction_confidence),
    );
    object.insert(
        "directionSource".to_string(),
        serde_json::json!(direction_source),
    );
    object.insert(
        "tofScore".to_string(),
        serde_json::json!(enhancement
            .tof_metrics
            .lineage
            .available
            .then_some(enhancement.tof_score)),
    );
    object.insert(
        "perpTofMetrics".to_string(),
        serde_json::to_value(&perp_metrics).unwrap_or(serde_json::Value::Null),
    );
    object.insert(
        "perpScore".to_string(),
        serde_json::json!(perp_metrics.lineage.available.then_some(perp_score)),
    );
    object.insert(
        "perpCandidateType".to_string(),
        serde_json::json!(perp_candidate_type.clone()),
    );
    object.insert(
        "finalCandidateType".to_string(),
        serde_json::json!(final_candidate_type.clone()),
    );
    object.insert(
        "metricsDirection".to_string(),
        serde_json::json!(advanced_metrics
            .lineage
            .available
            .then_some(advanced_metrics.metrics_direction)),
    );
    object.insert(
        "mergedConfidence".to_string(),
        serde_json::json!(advanced_metrics
            .lineage
            .available
            .then_some(advanced_metrics.confidence)),
    );
    object.insert(
        "advancedTofMetrics".to_string(),
        serde_json::to_value(&advanced_metrics).unwrap_or(serde_json::Value::Null),
    );
    object.insert(
        "advancedScore".to_string(),
        serde_json::json!(advanced_metrics.lineage.available.then_some(advanced_score)),
    );
    object.insert(
        "advancedCandidateType".to_string(),
        serde_json::json!(advanced_candidate_type.clone()),
    );
    object.insert(
        "cwmContribution".to_string(),
        serde_json::to_value(&cwm_contribution).unwrap_or(serde_json::Value::Null),
    );
    object.insert(
        "riskSystems".to_string(),
        serde_json::to_value(&risk_systems_transport).unwrap_or(serde_json::Value::Null),
    );
    object.insert(
        "toxicShortScore".to_string(),
        serde_json::to_value(&risk_systems.short_term_toxic).unwrap_or(serde_json::Value::Null),
    );
    object.insert("toxicScore".to_string(), serde_json::json!(toxic_score));
    object.insert(
        "shortPressure".to_string(),
        serde_json::json!(risk_systems.short_term_toxic.short_pressure),
    );
    object.insert(
        "toxicSeverity".to_string(),
        serde_json::json!(risk_systems.short_term_toxic.severity.clone()),
    );
    object.insert(
        "toxicType".to_string(),
        serde_json::json!(risk_systems.short_term_toxic.toxic_type.clone()),
    );
    object.insert(
        "toxicTtlSec".to_string(),
        serde_json::json!(risk_systems.short_term_toxic.ttl_sec),
    );
    object.insert(
        "toxicExpiresAt".to_string(),
        serde_json::json!(risk_systems.short_term_toxic.expires_at),
    );
    object.insert(
        "toxicHalfLifeSec".to_string(),
        serde_json::json!(risk_systems.short_term_toxic.half_life_sec),
    );
    object.insert(
        "toxicMaxTtlSec".to_string(),
        serde_json::json!(risk_systems.short_term_toxic.max_ttl_sec),
    );
    object.insert(
        "toxicDecayedScore".to_string(),
        serde_json::json!(risk_systems.short_term_toxic.decayed_score),
    );
    object.insert(
        "toxicDecayFormula".to_string(),
        serde_json::json!(risk_systems.short_term_toxic.decay_formula.clone()),
    );
    object.insert(
        "toxicReasons".to_string(),
        serde_json::to_value(&risk_systems.short_term_toxic.reasons)
            .unwrap_or(serde_json::Value::Null),
    );
    object.insert(
        "marketStructureScore".to_string(),
        serde_json::to_value(&risk_systems_transport.market_structure_score)
            .unwrap_or(serde_json::Value::Null),
    );
    object.insert(
        "mainForceScore".to_string(),
        serde_json::json!(market_structure.map(|score| score.main_force_score)),
    );
    object.insert(
        "mainForceConfirmed".to_string(),
        serde_json::json!(market_structure.map(|score| score.main_force_confirmed)),
    );
    object.insert(
        "mainForceConfirmationCount".to_string(),
        serde_json::json!(market_structure.map(|score| score.main_force_confirmation_count)),
    );
    object.insert(
        "mainForceConfirmationTotal".to_string(),
        serde_json::json!(market_structure.map(|score| score.main_force_confirmation_total)),
    );
    object.insert(
        "mainForceConfirmationThreshold".to_string(),
        serde_json::json!(market_structure.map(|score| score.main_force_confirmation_threshold)),
    );
    object.insert(
        "structureBias".to_string(),
        serde_json::json!(market_structure.map(|score| score.structure_bias)),
    );
    object.insert(
        "extremeImpactScore".to_string(),
        serde_json::json!(market_structure.map(|score| score.extreme_impact_score)),
    );
    object.insert(
        "extremeImpactConfirmed".to_string(),
        serde_json::json!(market_structure.map(|score| score.extreme_impact_confirmed)),
    );
    object.insert(
        "regimeType".to_string(),
        serde_json::json!(market_structure.map(|score| score.regime_type.as_str())),
    );
    object.insert(
        "marketStructureSeverity".to_string(),
        serde_json::json!(market_structure.map(|score| score.severity.as_str())),
    );
    object.insert(
        "marketStructureConfidence".to_string(),
        serde_json::json!(market_structure.map(|score| score.confidence)),
    );
    object.insert(
        "marketStructureDataQuality".to_string(),
        serde_json::json!(market_structure.map(|score| score.data_quality)),
    );
    object.insert(
        "structureRaw".to_string(),
        serde_json::json!(market_structure.map(|score| score.structure_raw)),
    );
    object.insert(
        "spotContractFloor".to_string(),
        serde_json::json!(market_structure.map(|score| score.spot_contract_floor)),
    );
    object.insert(
        "durationScore".to_string(),
        serde_json::json!(market_structure.map(|score| score.duration_score)),
    );
    object.insert(
        "liquidationPenalty".to_string(),
        serde_json::json!(market_structure.map(|score| score.liquidation_penalty)),
    );
    object.insert(
        "crowdingPenalty".to_string(),
        serde_json::json!(market_structure.map(|score| score.crowding_penalty)),
    );
    object.insert(
        "spotScore".to_string(),
        serde_json::json!(market_structure.map(|score| score.spot_score)),
    );
    object.insert(
        "spotCvdScore".to_string(),
        serde_json::json!(market_structure.map(|score| score.spot_cvd_score)),
    );
    object.insert(
        "spotVolumeAnomaly".to_string(),
        serde_json::json!(market_structure.map(|score| score.spot_volume_anomaly)),
    );
    object.insert(
        "spotAbsorption".to_string(),
        serde_json::json!(market_structure.map(|score| score.spot_absorption)),
    );
    object.insert(
        "spotLiquidityShift".to_string(),
        serde_json::json!(market_structure.map(|score| score.spot_liquidity_shift)),
    );
    object.insert(
        "spotPriceResponse".to_string(),
        serde_json::json!(market_structure.map(|score| score.spot_price_response)),
    );
    object.insert(
        "contractScore".to_string(),
        serde_json::json!(market_structure.map(|score| score.contract_score)),
    );
    object.insert(
        "cwmAggressiveFlow".to_string(),
        serde_json::json!(market_structure.map(|score| score.cwm_aggressive_flow)),
    );
    object.insert(
        "oiImpulse".to_string(),
        serde_json::json!(market_structure.map(|score| score.oi_impulse)),
    );
    object.insert(
        "liquidationContext".to_string(),
        serde_json::json!(market_structure.map(|score| score.liquidation_context)),
    );
    object.insert(
        "fundingCrowding".to_string(),
        serde_json::json!(market_structure.map(|score| score.funding_crowding)),
    );
    object.insert(
        "basisPremium".to_string(),
        serde_json::json!(market_structure.map(|score| score.basis_premium)),
    );
    object.insert(
        "activeExchangeConfirmation".to_string(),
        serde_json::json!(market_structure.map(|score| score.active_exchange_confirmation)),
    );
    object.insert(
        "crossConfirmScore".to_string(),
        serde_json::json!(market_structure.map(|score| score.cross_confirm_score)),
    );
    object.insert(
        "spotContractDirectionConsistency".to_string(),
        serde_json::json!(market_structure.map(|score| score.spot_contract_direction_consistency)),
    );
    object.insert(
        "multiWindowConsistency".to_string(),
        serde_json::json!(market_structure.map(|score| score.multi_window_consistency)),
    );
    object.insert(
        "priceResponseConsistency".to_string(),
        serde_json::json!(market_structure.map(|score| score.price_response_consistency)),
    );
    object.insert(
        "sourceCoverage".to_string(),
        serde_json::json!(market_structure.map(|score| score.source_coverage)),
    );
    object.insert(
        "signalAgreement".to_string(),
        serde_json::json!(market_structure.map(|score| score.signal_agreement)),
    );
    object.insert(
        "oiScore".to_string(),
        serde_json::json!(market_structure.map(|score| score.oi_score)),
    );
    object.insert(
        "liquidationScore".to_string(),
        serde_json::json!(market_structure.map(|score| score.liquidation_score)),
    );
    object.insert(
        "fundingCrowdingScore".to_string(),
        serde_json::json!(market_structure.map(|score| score.funding_crowding_score)),
    );
    object.insert(
        "cwmScore".to_string(),
        serde_json::json!(market_structure.map(|score| score.cwm_score)),
    );
    object.insert(
        "behaviorType".to_string(),
        serde_json::json!(behavior_type.clone()),
    );
    object.insert(
        "behaviorState".to_string(),
        serde_json::json!(behavior_state.clone()),
    );
    object.insert(
        "behaviorConfidence".to_string(),
        serde_json::json!(behavior_confidence),
    );
    object.insert(
        "behaviorMainForceConfirmed".to_string(),
        serde_json::json!(behavior_main_force_confirmed),
    );
    object.insert(
        "marketStructureReasons".to_string(),
        serde_json::to_value(market_structure.map(|score| &score.reasons))
            .unwrap_or(serde_json::Value::Null),
    );
    object.insert(
        "finalRiskScore".to_string(),
        serde_json::json!(existing_risk_score),
    );
    object.insert(
        "riskScore".to_string(),
        serde_json::json!(existing_risk_score),
    );
    object.insert(
        "dataQuality".to_string(),
        serde_json::json!(authoritative_data_quality),
    );
    let signal_id = object
        .get("signalId")
        .and_then(|value| value.as_str())
        .map(str::to_string);
    let tof_score_for_alert = enhancement
        .tof_metrics
        .lineage
        .alert_eligible
        .then_some(enhancement.tof_score);
    let perp_score_for_alert = perp_metrics.lineage.alert_eligible.then_some(perp_score);
    let advanced_score_for_alert = advanced_metrics
        .lineage
        .alert_eligible
        .then_some(advanced_score);
    let server_evidence_verified = monitoring_started
        && authoritative_data_quality.is_some()
        && object
            .get("readOnly")
            .and_then(|value| value.as_bool())
            .unwrap_or(false)
        && !object
            .get("runtimeModified")
            .and_then(|value| value.as_bool())
            .unwrap_or(true)
        && object
            .get("analysisOnly")
            .and_then(|value| value.as_bool())
            .unwrap_or(false)
        && !object
            .get("executionEnabled")
            .and_then(|value| value.as_bool())
            .unwrap_or(true);
    object.insert(
        "alertEligible".to_string(),
        serde_json::json!(server_evidence_verified),
    );
    let mut alert_request = DiscordNotificationRequest {
        server_evidence_verified,
        alert_family: None,
        signal_id: signal_id.clone(),
        id: signal_id.clone(),
        dedupe_key: signal_id.clone(),
        exchange: Some("Runtime".to_string()),
        symbol: Some(symbol),
        signal_type: Some(signal_kind),
        level: Some(severity),
        side: Some(enhancement.direction_label.clone()),
        score: Some(existing_risk_score),
        confidence: Some((confidence * 100.0).clamp(0.0, 100.0)),
        data_quality: authoritative_data_quality,
        reason: Some(summary),
        impact: None,
        impact_level: None,
        time: None,
        price_range: None,
        add_qty: None,
        cancel_qty: None,
        fill_qty: None,
        cancel_to_trade_ratio: None,
        depth_before: None,
        depth_after: None,
        depth_impact: None,
        price_impact_bps: None,
        markout_1s_bps: None,
        markout_5s_bps: None,
        markout_30s_bps: None,
        tof_metrics: Some(enhancement.tof_metrics),
        tof_score: tof_score_for_alert,
        candidate_type: Some(advanced_candidate_type.clone()),
        explain_tags: Some(merged_tags),
        direction_confidence: Some(enhancement.direction_confidence),
        perp_tof_metrics: Some(perp_metrics),
        perp_score: perp_score_for_alert,
        perp_candidate_type: Some(perp_candidate_type),
        final_candidate_type: Some(final_candidate_type),
        metrics_direction,
        advanced_tof_metrics: Some(advanced_metrics),
        advanced_score: advanced_score_for_alert,
        advanced_candidate_type: Some(advanced_candidate_type),
        main_force_score: market_structure.map(|score| score.main_force_score),
        extreme_impact_score: market_structure.map(|score| score.extreme_impact_score),
        structure_bias: market_structure.map(|score| score.structure_bias),
        market_structure_confidence: market_structure.map(|score| score.confidence),
        market_structure_data_quality: market_structure.map(|score| score.data_quality),
        market_structure_severity: market_structure.map(|score| score.severity.clone()),
        behavior_type,
        behavior_state,
        behavior_confidence,
        behavior_main_force_confirmed,
        regime_type: market_structure.map(|score| score.regime_type.clone()),
        spot_score: market_structure.map(|score| score.spot_score),
        contract_score: market_structure.map(|score| score.contract_score),
        cross_confirm_score: market_structure.map(|score| score.cross_confirm_score),
        main_force_confirmed: market_structure.map(|score| score.main_force_confirmed),
        signal_agreement: market_structure.map(|score| score.signal_agreement),
        source_coverage: market_structure.map(|score| score.source_coverage),
        oi_score: market_structure.map(|score| score.oi_score),
        liquidation_score: market_structure.map(|score| score.liquidation_score),
        test: None,
    };
    alert_request.alert_family = Some(preferred_discord_alert_family(&alert_request).to_string());
    if alert_request.alert_family.as_deref() == Some("market_structure")
        && !cwm_contribution.available
    {
        alert_request.server_evidence_verified = false;
        object.insert("alertEligible".to_string(), serde_json::json!(false));
    }
    let alert_decision = evaluate_discord_alert_gate(&alert_request, DiscordAlertMode::Auto);
    let stored_alert = signal_id.as_deref().and_then(discord_alert_status_for_key);
    let alert_status = stored_alert
        .as_ref()
        .map(|status| status.last_decision.clone())
        .unwrap_or_else(|| {
            alert_status_from_reason(alert_decision.allowed, alert_decision.reason).to_string()
        });
    let alert_reason = stored_alert
        .as_ref()
        .map(|status| status.reason.clone())
        .unwrap_or_else(|| alert_decision.reason.to_string());
    object.insert(
        "alertStatus".to_string(),
        serde_json::json!(alert_status.clone()),
    );
    object.insert(
        "alertReason".to_string(),
        serde_json::json!(alert_reason.clone()),
    );
    let discord_alert = stored_alert
        .map(serde_json::to_value)
        .and_then(Result::ok)
        .unwrap_or_else(|| {
            serde_json::json!({
            "autoEligible": alert_decision.allowed,
            "autoSent": false,
            "lastDecision": alert_status,
            "reason": alert_reason,
            "sentAt": null,
            "manualSentAt": null,
            })
        });
    object.insert("discordAlert".to_string(), discord_alert);
}

fn alert_status_from_reason(allowed: bool, reason: &str) -> &'static str {
    if allowed {
        "eligible"
    } else if matches!(
        reason,
        "score_below_threshold"
            | "confidence_below_threshold"
            | "data_quality_below_threshold"
            | "non_high_risk"
    ) {
        "rejected"
    } else {
        "skipped"
    }
}

#[cfg(test)]
mod tests {
    use super::{cwm_symbol_from_requested_symbol, decorate_item_with_tof_observation};
    use crate::{
        api::toxic_signal_ws_routes::build_ws_snapshot_with_authoritative_state,
        app::AppState,
        config::AppConfig,
        contract_whale_monitor::types::{
            ContractWhaleActiveSources, ContractWhaleDirection, ContractWhaleEvidenceState,
            ContractWhaleLiquidationForce, ContractWhaleMarketType, ContractWhalePriceResponseType,
            ContractWhaleScoreBreakdown, ContractWhaleSeverity, ContractWhaleSignal,
            ContractWhaleSignalType, ContractWhaleSourceRole, ContractWhaleSpotConfirmationContext,
        },
        runtime::tof_metrics::ObservedTofSnapshot,
        types::toxic_signal_inbox::ToxicSignalInboxRecentResponse,
    };
    use std::collections::BTreeMap;

    #[test]
    fn cwm_symbol_uses_canonical_base_for_common_aliases() {
        for alias in ["BTC-PERP", "btcusdt", "BTC/USDT", "BTC-USDT-SWAP"] {
            assert_eq!(cwm_symbol_from_requested_symbol(alias), "BTC");
        }
        assert_eq!(cwm_symbol_from_requested_symbol("ETHPERP"), "ETH");
    }

    #[test]
    fn rest_and_ws_market_structure_require_bilateral_tof_and_fresh_cwm_evidence() {
        let mut config =
            AppConfig::from_env_with_config_file("config/default").expect("test config");
        config.sqlite_enabled = false;
        config.replay_enabled = false;
        config.contract_whale_monitor.enabled = false;
        config.spot_whale_monitor.enabled = false;
        let state = AppState::new(config);
        let now_ms = crate::normalizers::trade::now_ms();
        let tof = observed_tof(now_ms);
        let cwm = sample_cwm_signal(now_ms);

        for (name, use_tof, use_cwm, expected_available) in [
            ("neither", false, false, false),
            ("tof_only", true, false, false),
            ("cwm_only", false, true, false),
            ("both", true, true, true),
        ] {
            let recent = summary_only_recent_at(now_ms);
            let tof = use_tof.then_some(&tof);
            let cwm = use_cwm.then_some(&cwm);
            let mut rest_item = serde_json::to_value(&recent.items[0]).expect("REST item json");
            decorate_item_with_tof_observation(&mut rest_item, &state, cwm, tof);
            let ws = serde_json::to_value(build_ws_snapshot_with_authoritative_state(
                &recent, cwm, tof, false,
            ))
            .expect("WS json");
            let ws_item = &ws["signals"][0];

            assert_market_transport(&rest_item, expected_available, "REST", name);
            assert_market_transport(ws_item, expected_available, "WS", name);
            for field in [
                "riskScore",
                "dataQuality",
                "confidence",
                "direction",
                "directionConfidence",
                "directionSource",
                "shortPressure",
            ] {
                assert_eq!(
                    rest_item[field], ws_item[field],
                    "detector field {field} for {name}"
                );
            }
            assert_eq!(rest_item["riskScore"], 82);
            assert_eq!(rest_item["dataQuality"], 82.0);
        }
    }

    fn summary_only_recent_at(created_at_ms: i64) -> ToxicSignalInboxRecentResponse {
        serde_json::from_value(serde_json::json!({
            "readOnly": true,
            "runtimeModified": false,
            "analysisOnly": true,
            "executionEnabled": false,
            "manualReviewRequired": true,
            "runtimeWeightModified": false,
            "configModified": false,
            "mode": "analysis_only",
            "selectedSymbol": "BTC-PERP",
            "status": "signal_inbox_ready",
            "warnings": [],
            "items": [{
                "signalId": "sig_transport_no_source",
                "symbol": "BTC-PERP",
                "signalKind": "spoofing_candidate",
                "directionBias": "short_bias",
                "severity": "high",
                "riskScore": 82,
                "dataQualityScore": 82.0,
                "confidence": 0.82,
                "createdAtMs": created_at_ms,
                "fusion": { "available": true, "summary": "large ask wall removed" },
                "replay": { "available": true, "evidenceCount": 3 },
                "markout": {
                    "available": true,
                    "oneMinute": "adverse",
                    "fiveMinute": "adverse",
                    "fifteenMinute": "not_enough_data",
                    "oneHour": "not_enough_data"
                },
                "quality": {
                    "available": true,
                    "qualityBucket": "good",
                    "alignedRatio": 0.8,
                    "adverseRatio": 0.2
                },
                "recommendation": {
                    "available": true,
                    "action": "review_evidence",
                    "noTradeOnly": false,
                    "manualReviewRequired": true
                },
                "governance": {
                    "ledgerAvailable": false,
                    "latestDecision": "missing_ledger_evidence"
                },
                "operatorAction": "review_evidence",
                "readOnly": true,
                "runtimeModified": false,
                "analysisOnly": true,
                "executionEnabled": false
            }]
        }))
        .expect("summary-only recent")
    }

    fn assert_market_transport(
        item: &serde_json::Value,
        expected_available: bool,
        surface: &str,
        case: &str,
    ) {
        for field in [
            "marketStructureScore",
            "mainForceScore",
            "mainForceConfirmed",
            "mainForceConfirmationCount",
            "mainForceConfirmationTotal",
            "mainForceConfirmationThreshold",
            "structureBias",
            "extremeImpactScore",
            "extremeImpactConfirmed",
            "regimeType",
            "marketStructureSeverity",
            "marketStructureConfidence",
            "marketStructureDataQuality",
            "structureRaw",
            "spotContractFloor",
            "durationScore",
            "liquidationPenalty",
            "crowdingPenalty",
            "spotScore",
            "spotCvdScore",
            "spotVolumeAnomaly",
            "spotAbsorption",
            "spotLiquidityShift",
            "spotPriceResponse",
            "contractScore",
            "cwmAggressiveFlow",
            "oiImpulse",
            "liquidationContext",
            "fundingCrowding",
            "basisPremium",
            "activeExchangeConfirmation",
            "crossConfirmScore",
            "spotContractDirectionConsistency",
            "multiWindowConsistency",
            "priceResponseConsistency",
            "sourceCoverage",
            "signalAgreement",
            "oiScore",
            "liquidationScore",
            "fundingCrowdingScore",
            "cwmScore",
        ] {
            assert_eq!(
                !item[field].is_null(),
                expected_available,
                "{surface} {field} availability for {case}"
            );
        }
        assert_eq!(
            !item["riskSystems"]["marketStructureScore"].is_null(),
            expected_available,
            "{surface} nested marketStructureScore for {case}"
        );
        assert_eq!(
            !item["riskSystems"]["mainForceStructure"].is_null(),
            expected_available,
            "{surface} nested mainForceStructure for {case}"
        );
        if surface == "REST" {
            assert_eq!(
                !item["marketStructureReasons"].is_null(),
                expected_available,
                "REST marketStructureReasons for {case}"
            );
        }
    }

    fn observed_tof(observed_at_ms: i64) -> ObservedTofSnapshot {
        ObservedTofSnapshot {
            symbol: "BTC-PERP".to_string(),
            observed_at_ms,
            buy_volume: 300.0,
            sell_volume: 100.0,
            trade_count: 40,
            window_ms: 5_000,
            vpin: Some(0.82),
            vpin_zscore: Some(2.4),
            vpin_percentile: Some(0.96),
            vpin_bucket_count: 20,
            vpin_window_volume: 2_000.0,
            per_venue_vpin: BTreeMap::from([("binance".to_string(), 0.82)]),
            bid_depth_withdrawal: Some(12.0),
            ask_depth_withdrawal: Some(62.0),
            spread_bps: Some(9.0),
            book_update_rate: Some(80.0),
            sweep_score: Some(70.0),
        }
    }

    fn sample_cwm_signal(ts: i64) -> ContractWhaleSignal {
        let mut signal = ContractWhaleSignal {
            id: format!("contract-whale:BTC:15:{ts}:buy"),
            ts,
            symbol: "BTC".to_string(),
            window_sec: 15,
            signal_type: ContractWhaleSignalType::AggressiveBuy,
            direction: ContractWhaleDirection::Buy,
            severity: ContractWhaleSeverity::S,
            score: 94,
            main_force_score: Some(86),
            spot_score: Some(78),
            contract_score: Some(94),
            base_asset: "BTC".to_string(),
            quantity_unit: "BTC".to_string(),
            total_volume: 4_820.0,
            net_volume: 3_260.0,
            total_volume_btc: 4_820.0,
            net_volume_btc: 3_260.0,
            total_notional_usd: 337_000_000.0,
            dominance: 0.676,
            order_price_usd: Some(70_000.0),
            current_market_price_usd: Some(70_000.0),
            price_deviation_pct: Some(0.0),
            price_deviation_filtered: false,
            price_move_pct: Some(0.31),
            price_move_5s_pct: None,
            price_move_15s_pct: Some(0.31),
            price_move_30s_pct: None,
            price_response_type: ContractWhalePriceResponseType::TrendFollowUp,
            classification_v2: Default::default(),
            behavior_assessment: Default::default(),
            main_exchange: Some("binance".to_string()),
            market_type: ContractWhaleMarketType::Perp,
            source_role: ContractWhaleSourceRole::Primary,
            exchanges: Vec::new(),
            dominant_venue_net_contribution_share: Some(0.72),
            dynamic_multiple: Some(9.4),
            dynamic_baseline_btc: Some(512.0),
            dynamic_threshold_level: "critical".to_string(),
            percentile_level: Some(99.9),
            impact_level: None,
            signal_level: None,
            signal_label: None,
            normalized_strength: None,
            impact_score: None,
            impact_z_score: None,
            multi_exchange_confirmed: true,
            liquidation_suspected: false,
            liquidation_long_btc: 0.0,
            liquidation_short_btc: 0.0,
            liquidation_notional_usd: 0.0,
            liquidation_ratio: None,
            price_reversal_ratio: None,
            oi_change_1m_btc: None,
            oi_change_5m_btc: None,
            oi_change_pct: Some(1.4),
            oi_bias: Some("long_increase".to_string()),
            funding_rate: Some(0.00076),
            funding_bias: Some("long".to_string()),
            data_quality: 91,
            score_breakdown: ContractWhaleScoreBreakdown::default(),
            threshold_profile: "binance_bitfinex".to_string(),
            threshold_profile_reason: "active_contract_sources=binance,bitfinex".to_string(),
            configured_contract_sources: vec!["binance".to_string(), "bitfinex".to_string()],
            eligible_contract_sources: vec!["binance".to_string(), "bitfinex".to_string()],
            active_contract_sources: vec!["binance".to_string(), "bitfinex".to_string()],
            active_sources: ContractWhaleActiveSources::default(),
            spot_confirmation: ContractWhaleSpotConfirmationContext::default(),
            discord_eligible: true,
            discord_sent: false,
            discord_sent_at: None,
            discord_reason: "dry_run".to_string(),
            discord_would_send: true,
            final_result: "multi-platform aggressive buy burst".to_string(),
            cluster: Default::default(),
            persistence: Default::default(),
            whale_action: Default::default(),
            trajectory: Default::default(),
            liquidation_force: ContractWhaleLiquidationForce::default(),
            market_driver: Default::default(),
            event_lifecycle: Default::default(),
            event_quality: Default::default(),
            read_only: true,
            analysis_only: true,
            execution_enabled: false,
            merged_from: Vec::new(),
        };
        signal.classification_v2.evidence.oi = ContractWhaleEvidenceState::Available(1.4);
        signal.classification_v2.evidence.funding = ContractWhaleEvidenceState::Available(0.00076);
        signal.classification_v2.evidence.liquidation_status = "unavailable".to_string();
        signal.classification_v2.evidence.liquidation_reason =
            Some("no_live_liquidation_samples".to_string());
        signal
    }
}
