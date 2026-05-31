use std::collections::BTreeSet;

use crate::{
    config::AppConfig,
    market_data::quality::MarketDataQualitySnapshot,
    toxicity::cross_venue_confirmation::same_direction_venue_count,
    types::{
        flow::{FlowState, FlowWindow},
        liquidation::{
            LiquidationToxicDirection, LiquidationToxicSignalType,
            LiquidationToxicityRecentResponse,
        },
        market::VenueConnectionStatus,
        orderbook_wall::{
            OrderbookWallCandidateType, OrderbookWallInterpretationReport,
            OrderbookWallInterpretationType, OrderbookWallLifecycleReport,
        },
        status::VenueHealthMap,
        structural_toxicity::{
            StructuralToxicDirection, StructuralToxicSignalType, StructuralToxicityRecentResponse,
        },
        sweep::{SweepResult, SweepState},
        toxic::ToxicDirection,
        toxic_flow::{
            ActiveTradeToxicSignalType, ActiveTradeToxicityRecentResponse, ToxicConfidence,
            ToxicSide,
        },
        toxic_signal::{ToxicSignalDirection, ToxicSignalRecentResponse, ToxicSignalType},
        whale_flow_signal::{
            WhaleFlowBaselineQuality, WhaleFlowCandidate, WhaleFlowCandidateDiagnostics,
            WhaleFlowCandidateType, WhaleFlowDataQualitySummary, WhaleFlowRecentResponse,
            WhaleFlowThresholds, WhaleFlowVenueCoverage,
        },
    },
};

const ONE_HOUR_WINDOW_MS: u64 = 3_600_000;
const HISTORY_RATIO_THRESHOLD: f64 = 5.0;
const DIRECTION_BIAS_THRESHOLD: f64 = 0.70;
const DEPTH_DROP_THRESHOLD: f64 = 0.30;
const PRICE_IMPACT_THRESHOLD_BPS: f64 = 1.0;
const MIN_MULTI_VENUE_COUNT: usize = 2;

pub struct WhaleFlowAnalysisInputs<'a> {
    pub requested_symbol: &'a str,
    pub config: &'a AppConfig,
    pub venue_health: &'a VenueHealthMap,
    pub flow_state: &'a FlowState,
    pub sweep_state: &'a SweepState,
    pub market_data_quality: MarketDataQualitySnapshot,
    pub active_trade_recent: &'a ActiveTradeToxicityRecentResponse,
    pub liquidation_recent: &'a LiquidationToxicityRecentResponse,
    pub wall_lifecycle_report: &'a OrderbookWallLifecycleReport,
    pub wall_interpretation_report: &'a OrderbookWallInterpretationReport,
    pub structural_recent: &'a StructuralToxicityRecentResponse,
    pub fusion_recent: &'a ToxicSignalRecentResponse,
}

pub fn analyze_whale_flow(inputs: &WhaleFlowAnalysisInputs<'_>) -> WhaleFlowRecentResponse {
    let requested_symbol = inputs.requested_symbol.trim();
    let flow_windows = relevant_windows(requested_symbol, inputs.flow_state);
    let flow_windows_populated = flow_windows
        .iter()
        .any(|window| window.trade_count > 0 || window.data_quality.has_trades);
    let venue_coverage = build_venue_coverage(inputs, &flow_windows);
    let connected_venues = venue_coverage.connected_venues;
    let lagged_events = inputs.market_data_quality.flow_window_lagged_events
        + inputs.market_data_quality.markout_lagged_events
        + inputs.market_data_quality.vpin_lagged_events;
    let dropped_events = inputs.market_data_quality.event_bus_dropped_events;
    let thresholds = whale_flow_thresholds();

    let mut warnings = Vec::new();
    let mut no_trade_reasons = Vec::new();
    let mut no_candidate_reasons = Vec::new();
    let mut degradation_warnings = Vec::new();
    let mut history_baseline_mode = "unavailable".to_string();
    let mut fallback_baseline_quality = WhaleFlowBaselineQuality {
        relative_volume_multiple: None,
        baseline_source: "insufficient_history".to_string(),
        baseline_window_ms: None,
        fallback_used: false,
        insufficient_history: true,
        operator_warning: Some(
            "Relative volume baseline is unavailable for the current whale-flow review."
                .to_string(),
        ),
    };
    let mut fallback_window_score = f64::MIN;
    let mut primary_candidate_baseline_quality: Option<(f64, WhaleFlowBaselineQuality)> = None;
    let mut candidates = Vec::new();

    for window_ms in [1_000_u64, 5_000, 15_000, 60_000] {
        let Some(window) = flow_windows
            .iter()
            .find(|window| window.window_ms == window_ms)
        else {
            no_trade_reasons.push(format!(
                "{} window is unavailable for whale-flow review.",
                format_window(window_ms)
            ));
            continue;
        };

        if window.trade_count == 0 || !window.data_quality.has_trades {
            no_trade_reasons.push(format!(
                "{} has no confirmed public trade flow yet.",
                format_window(window_ms)
            ));
            continue;
        }

        let gross_volume_btc = total_aggressive_volume(window);
        let direction = dominant_side(window);
        if direction == ToxicSide::Neutral || gross_volume_btc <= 0.0 {
            no_trade_reasons.push(format!(
                "{} remained directionally neutral.",
                format_window(window_ms)
            ));
            continue;
        }

        let dominant_volume_btc = dominant_volume(window, direction);
        let volume_threshold_btc = volume_threshold(window_ms);
        let direction_bias = dominant_volume_btc / gross_volume_btc;
        let venue_count =
            same_direction_venue_count(toxic_direction(direction), &window.venue_breakdown);

        let baseline = historical_baseline(window_ms, flow_windows.as_slice());
        let replace_baseline_mode = history_baseline_mode == "unavailable"
            || (history_baseline_mode == "one_hour_normalized"
                && baseline.mode != "one_hour_normalized");
        if replace_baseline_mode {
            history_baseline_mode = baseline.mode.to_string();
        }
        let baseline_warning = baseline.warning.clone();
        if let Some(warning) = baseline_warning.clone() {
            warnings.push(warning);
        }

        let historical_volume_ratio = baseline
            .baseline_volume_btc
            .filter(|baseline_volume| *baseline_volume > 0.0)
            .map(|baseline_volume| dominant_volume_btc / baseline_volume);
        let depth_drop_ratio =
            matching_depth_drop_ratio(inputs.sweep_state, requested_symbol, window_ms, direction);
        let price_impact_bps =
            matching_price_impact_bps(inputs.sweep_state, requested_symbol, window_ms)
                .or(window.price_move_bps.map(f64::abs));

        let volume_ok = dominant_volume_btc >= volume_threshold_btc;
        let bias_ok = direction_bias >= DIRECTION_BIAS_THRESHOLD;
        let history_ok =
            historical_volume_ratio.is_some_and(|ratio| ratio >= HISTORY_RATIO_THRESHOLD);
        let multi_venue_ok = venue_count >= MIN_MULTI_VENUE_COUNT;
        let candidate_gate = price_impact_bps
            .is_some_and(|value| value >= PRICE_IMPACT_THRESHOLD_BPS)
            || depth_drop_ratio.is_some_and(|value| value >= DEPTH_DROP_THRESHOLD);
        let baseline_quality = build_baseline_quality(
            historical_volume_ratio,
            baseline.mode,
            baseline.source_window_ms,
            baseline_warning,
        );
        let missing_inputs =
            collect_missing_inputs(window, inputs.sweep_state, requested_symbol, window_ms);
        let degradation_reasons = collect_degradation_reasons(
            &missing_inputs,
            inputs.market_data_quality,
            &baseline_quality,
            venue_count,
        );
        let confidence_modifiers =
            collect_confidence_modifiers(&baseline_quality, venue_count, missing_inputs.is_empty());
        let why_candidate = collect_why_candidate(
            window_ms,
            dominant_volume_btc,
            direction_bias,
            historical_volume_ratio,
            venue_count,
            price_impact_bps,
            depth_drop_ratio,
        );
        let candidate_diagnostics = WhaleFlowCandidateDiagnostics {
            data_quality: derive_candidate_data_quality(&missing_inputs, &degradation_reasons),
            why_candidate,
            missing_inputs,
            degradation_reasons,
            confidence_modifiers,
        };
        let window_score = dominant_volume_btc;
        if window_score >= fallback_window_score {
            fallback_window_score = window_score;
            fallback_baseline_quality = baseline_quality.clone();
        }

        if !(volume_ok && bias_ok && history_ok && multi_venue_ok && candidate_gate) {
            if !volume_ok {
                no_trade_reasons.push(format!(
                    "{} dominant volume {:.1} BTC stayed below {:.0} BTC.",
                    format_window(window_ms),
                    dominant_volume_btc,
                    volume_threshold_btc
                ));
            }
            if !bias_ok {
                no_trade_reasons.push(format!(
                    "{} direction bias {:.0}% stayed below 70%.",
                    format_window(window_ms),
                    direction_bias * 100.0
                ));
            }
            if !history_ok {
                no_trade_reasons.push(format!(
                    "{} historical ratio was {}.",
                    format_window(window_ms),
                    historical_volume_ratio
                        .map(|ratio| format!("{ratio:.2}x"))
                        .unwrap_or_else(|| "Unavailable".to_string())
                ));
            }
            if !multi_venue_ok {
                no_trade_reasons.push(format!(
                    "{} had {} same-direction venue confirmations; need at least 2.",
                    format_window(window_ms),
                    venue_count
                ));
            }
            if !candidate_gate {
                no_trade_reasons.push(format!(
                    "{} had no material price impact or >=30% depth drop.",
                    format_window(window_ms)
                ));
            }
            no_candidate_reasons.extend(build_no_candidate_reasons(
                volume_ok,
                bias_ok,
                history_ok,
                multi_venue_ok,
                candidate_gate,
                &baseline_quality,
                &candidate_diagnostics.missing_inputs,
            ));
            degradation_warnings.extend(candidate_diagnostics.degradation_reasons.clone());
            continue;
        }

        let classification = classify_candidate(
            direction,
            inputs.active_trade_recent,
            inputs.liquidation_recent,
            inputs.wall_lifecycle_report,
            inputs.wall_interpretation_report,
            inputs.structural_recent,
            inputs.fusion_recent,
        );

        let mut reason = vec![
            format!(
                "{} dominant flow reached {:.1} BTC against a {:.0} BTC threshold.",
                format_window(window_ms),
                dominant_volume_btc,
                volume_threshold_btc
            ),
            format!("direction bias held at {:.0}%.", direction_bias * 100.0),
            format!(
                "historical normalized volume expanded to {:.2}x.",
                historical_volume_ratio.unwrap_or_default()
            ),
            format!("{} venues confirmed the same direction.", venue_count),
        ];
        if let Some(price_impact_bps) = price_impact_bps {
            if price_impact_bps >= PRICE_IMPACT_THRESHOLD_BPS {
                reason.push(format!("price impact reached {:.2} bps.", price_impact_bps));
            }
        }
        if let Some(depth_drop_ratio) = depth_drop_ratio {
            if depth_drop_ratio >= DEPTH_DROP_THRESHOLD {
                reason.push(format!(
                    "depth drop reached {:.0}%.",
                    depth_drop_ratio * 100.0
                ));
            }
        }
        reason.extend(classification.reason.clone());

        candidates.push(WhaleFlowCandidate {
            candidate_id: format!(
                "whale-flow-{}-{}-{}",
                requested_symbol.to_ascii_lowercase(),
                window_ms,
                window.now_ts
            ),
            symbol: requested_symbol.to_string(),
            ts_ms: window.now_ts.max(0) as u64,
            window: format_window(window_ms).to_string(),
            window_ms,
            volume_btc: dominant_volume_btc,
            gross_volume_btc,
            direction,
            direction_bias,
            historical_volume_ratio,
            historical_baseline_window_ms: baseline.source_window_ms,
            price_impact_bps,
            depth_drop_ratio,
            same_direction_venues: venue_count,
            candidate_type: classification.candidate_type,
            toxicity_score: classification.toxicity_score,
            confidence: classification.confidence,
            primary_reason: classification.primary_reason,
            reason,
            linked_active_trade_signal_ids: classification.linked_active_trade_signal_ids,
            linked_liquidation_signal_ids: classification.linked_liquidation_signal_ids,
            linked_wall_candidate_ids: classification.linked_wall_candidate_ids,
            linked_wall_interpretation_signal_ids: classification
                .linked_wall_interpretation_signal_ids,
            linked_structural_signal_ids: classification.linked_structural_signal_ids,
            linked_fusion_signal_ids: classification.linked_fusion_signal_ids,
            diagnostics: candidate_diagnostics,
            read_only: true,
        });
        match primary_candidate_baseline_quality.as_ref() {
            Some((score, _)) if *score > dominant_volume_btc => {}
            _ => {
                primary_candidate_baseline_quality =
                    Some((dominant_volume_btc, baseline_quality.clone()));
            }
        }
    }

    dedup_strings(&mut warnings);
    dedup_strings(&mut no_trade_reasons);
    dedup_strings(&mut no_candidate_reasons);
    dedup_strings(&mut degradation_warnings);
    candidates.sort_by(|left, right| {
        right
            .toxicity_score
            .cmp(&left.toxicity_score)
            .then_with(|| right.volume_btc.total_cmp(&left.volume_btc))
            .then_with(|| right.ts_ms.cmp(&left.ts_ms))
    });

    let primary_baseline_quality = primary_candidate_baseline_quality
        .map(|(_, baseline_quality)| baseline_quality)
        .unwrap_or(fallback_baseline_quality);

    let status = if !flow_windows_populated {
        "data_insufficient"
    } else if candidates.is_empty() {
        "no_whale_flow"
    } else {
        "candidate_active"
    };

    if status == "data_insufficient" {
        warnings.push(
            "Data insufficient: flow windows are not populated enough to classify whale flow."
                .to_string(),
        );
    } else if status == "no_whale_flow" {
        warnings.push(
            "No Whale Flow: current public trade flow does not satisfy the bounded candidate gates."
                .to_string(),
        );
    }

    if status != "no_whale_flow" {
        no_candidate_reasons.clear();
    } else if no_candidate_reasons.is_empty() {
        no_candidate_reasons
            .push("No Whale Flow candidate met the bounded read-only gates.".to_string());
    }

    let data_quality = build_data_quality_summary(
        flow_windows_populated,
        &venue_coverage,
        &primary_baseline_quality,
        inputs.sweep_state,
        &warnings,
        &degradation_warnings,
    );

    WhaleFlowRecentResponse {
        read_only: true,
        runtime_modified: false,
        analysis_only: true,
        execution_enabled: false,
        mode: "analysis_only".to_string(),
        selected_symbol: requested_symbol.to_string(),
        status: status.to_string(),
        history_baseline_mode,
        lagged_events,
        dropped_events,
        flow_windows_populated,
        connected_venues,
        data_quality,
        venue_coverage,
        baseline_quality: primary_baseline_quality,
        thresholds,
        warnings,
        no_trade_reasons,
        no_candidate_reasons,
        degradation_warnings,
        candidates,
    }
}

#[derive(Debug, Clone)]
struct HistoricalBaseline<'a> {
    mode: &'a str,
    source_window_ms: Option<u64>,
    baseline_volume_btc: Option<f64>,
    warning: Option<String>,
}

#[derive(Debug, Clone)]
struct CandidateClassification {
    candidate_type: WhaleFlowCandidateType,
    toxicity_score: u8,
    confidence: ToxicConfidence,
    primary_reason: String,
    reason: Vec<String>,
    linked_active_trade_signal_ids: Vec<String>,
    linked_liquidation_signal_ids: Vec<String>,
    linked_wall_candidate_ids: Vec<String>,
    linked_wall_interpretation_signal_ids: Vec<String>,
    linked_structural_signal_ids: Vec<String>,
    linked_fusion_signal_ids: Vec<String>,
}

fn build_venue_coverage(
    inputs: &WhaleFlowAnalysisInputs<'_>,
    flow_windows: &[&FlowWindow],
) -> WhaleFlowVenueCoverage {
    let configured_venues = inputs.config.venues.all();
    let configured_venue_keys: Vec<String> = configured_venues
        .iter()
        .map(|config| config.venue.as_key().to_string())
        .collect();
    let enabled_venue_keys: Vec<String> = configured_venues
        .iter()
        .filter(|config| config.enabled)
        .map(|config| config.venue.as_key().to_string())
        .collect();
    let connected_venue_keys = inputs
        .venue_health
        .values()
        .filter(|health| {
            matches!(
                health.status,
                VenueConnectionStatus::Connected | VenueConnectionStatus::Degraded
            )
        })
        .map(|health| health.venue.as_key().to_string())
        .collect::<BTreeSet<_>>();
    let trade_venues = active_venues(flow_windows);
    let book_venues = inputs
        .sweep_state
        .quality
        .active_venues
        .iter()
        .map(|venue| venue.as_key().to_string())
        .collect::<BTreeSet<_>>();
    let enabled_venue_set = enabled_venue_keys.iter().cloned().collect::<BTreeSet<_>>();
    let venues_missing_trades = enabled_venue_set
        .difference(&trade_venues)
        .cloned()
        .collect::<Vec<_>>();
    let venues_missing_books = enabled_venue_set
        .difference(&book_venues)
        .cloned()
        .collect::<Vec<_>>();
    let max_same_direction_venues = flow_windows
        .iter()
        .map(|window| {
            let direction = dominant_side(window);
            same_direction_venue_count(toxic_direction(direction), &window.venue_breakdown)
        })
        .max()
        .unwrap_or(0);

    WhaleFlowVenueCoverage {
        configured_venues: configured_venue_keys.len(),
        enabled_venues: enabled_venue_keys.len(),
        connected_venues: connected_venue_keys.len(),
        active_trade_venues: trade_venues.len(),
        active_book_venues: book_venues.len(),
        venues_with_recent_trades: trade_venues.into_iter().collect(),
        venues_with_recent_books: book_venues.into_iter().collect(),
        venues_missing_trades,
        venues_missing_books,
        min_venue_confluence_required: MIN_MULTI_VENUE_COUNT,
        venue_confluence_satisfied: max_same_direction_venues >= MIN_MULTI_VENUE_COUNT,
    }
}

fn build_baseline_quality(
    relative_volume_multiple: Option<f64>,
    baseline_mode: &str,
    baseline_window_ms: Option<u64>,
    operator_warning: Option<String>,
) -> WhaleFlowBaselineQuality {
    let baseline_source = normalize_baseline_source(baseline_mode);
    WhaleFlowBaselineQuality {
        relative_volume_multiple,
        baseline_source: baseline_source.to_string(),
        baseline_window_ms,
        fallback_used: matches!(
            baseline_source,
            "sixty_second_fallback" | "longer_window_fallback"
        ),
        insufficient_history: baseline_source == "insufficient_history",
        operator_warning,
    }
}

fn build_data_quality_summary(
    flow_windows_populated: bool,
    venue_coverage: &WhaleFlowVenueCoverage,
    baseline_quality: &WhaleFlowBaselineQuality,
    sweep_state: &SweepState,
    warnings: &[String],
    degradation_warnings: &[String],
) -> WhaleFlowDataQualitySummary {
    let latest_trade_available = !venue_coverage.venues_with_recent_trades.is_empty();
    let latest_book_available = !venue_coverage.venues_with_recent_books.is_empty();
    let venue_coverage_status = if venue_coverage.enabled_venues == 0 {
        "no_data"
    } else if venue_coverage.venues_missing_trades.is_empty()
        && venue_coverage.venues_missing_books.is_empty()
    {
        "healthy"
    } else if latest_trade_available || latest_book_available {
        "degraded"
    } else {
        "no_data"
    };
    let baseline_status = if baseline_quality.insufficient_history {
        "insufficient"
    } else if baseline_quality.fallback_used {
        "fallback"
    } else {
        "healthy"
    };
    let status = if !flow_windows_populated && !sweep_state.quality.has_books {
        "no_data"
    } else if latest_trade_available
        && latest_book_available
        && venue_coverage_status == "healthy"
        && baseline_status == "healthy"
    {
        "healthy"
    } else if latest_trade_available || latest_book_available {
        "partial"
    } else {
        "degraded"
    };
    let operator_warning = baseline_quality
        .operator_warning
        .clone()
        .or_else(|| degradation_warnings.first().cloned())
        .or_else(|| warnings.first().cloned());

    WhaleFlowDataQualitySummary {
        status: status.to_string(),
        venue_coverage_status: venue_coverage_status.to_string(),
        baseline_status: baseline_status.to_string(),
        latest_trade_available,
        latest_book_available,
        operator_warning,
    }
}

fn whale_flow_thresholds() -> WhaleFlowThresholds {
    WhaleFlowThresholds {
        one_second_btc: volume_threshold(1_000),
        five_second_btc: volume_threshold(5_000),
        fifteen_second_btc: volume_threshold(15_000),
        sixty_second_btc: volume_threshold(60_000),
        direction_ratio_min: DIRECTION_BIAS_THRESHOLD,
        relative_volume_multiple_min: HISTORY_RATIO_THRESHOLD,
        min_venue_confirmations: MIN_MULTI_VENUE_COUNT,
    }
}

fn collect_missing_inputs(
    window: &FlowWindow,
    sweep_state: &SweepState,
    requested_symbol: &str,
    window_ms: u64,
) -> Vec<String> {
    let mut missing_inputs = Vec::new();
    if !window.data_quality.has_books {
        missing_inputs.push(format!(
            "{} orderbook depth unavailable.",
            format_window(window_ms)
        ));
    }
    let sweep_result = matching_sweep_result(sweep_state, requested_symbol, window_ms);
    if sweep_result
        .and_then(|result| result.liquidity.as_ref())
        .is_none()
    {
        missing_inputs.push(format!(
            "{} sweep liquidity depth unavailable.",
            format_window(window_ms)
        ));
    }
    missing_inputs
}

fn collect_degradation_reasons(
    missing_inputs: &[String],
    market_data_quality: MarketDataQualitySnapshot,
    baseline_quality: &WhaleFlowBaselineQuality,
    venue_count: usize,
) -> Vec<String> {
    let mut reasons = Vec::new();
    if !missing_inputs.is_empty() {
        reasons.extend(missing_inputs.iter().cloned());
    }
    if venue_count < MIN_MULTI_VENUE_COUNT {
        reasons.push(format!(
            "Only {} venue confirmed the same-side flow.",
            venue_count
        ));
    }
    if baseline_quality.fallback_used {
        reasons.push("Relative volume baseline used fallback history.".to_string());
    }
    if baseline_quality.insufficient_history {
        reasons.push("Relative volume baseline history is insufficient.".to_string());
    }
    if market_data_quality.event_bus_dropped_events > 0
        || market_data_quality.flow_window_lagged_events > 0
    {
        reasons.push("Market-data lag or drops were observed in the current process.".to_string());
    }
    reasons
}

fn collect_confidence_modifiers(
    baseline_quality: &WhaleFlowBaselineQuality,
    venue_count: usize,
    depth_available: bool,
) -> Vec<String> {
    let mut modifiers = Vec::new();
    if venue_count >= MIN_MULTI_VENUE_COUNT {
        modifiers.push("venue_confluence_confirmed".to_string());
    }
    if baseline_quality.fallback_used {
        modifiers.push("baseline_fallback_used".to_string());
    }
    if baseline_quality.insufficient_history {
        modifiers.push("baseline_insufficient_history".to_string());
    }
    if !depth_available {
        modifiers.push("depth_unavailable".to_string());
    }
    modifiers
}

fn collect_why_candidate(
    window_ms: u64,
    dominant_volume_btc: f64,
    _direction_bias: f64,
    historical_volume_ratio: Option<f64>,
    venue_count: usize,
    price_impact_bps: Option<f64>,
    depth_drop_ratio: Option<f64>,
) -> Vec<String> {
    let mut reasons = vec![
        format!(
            "{} active volume exceeded {:.0} BTC.",
            format_window(window_ms),
            volume_threshold(window_ms)
        ),
        format!(
            "direction ratio exceeded {:.0}%.",
            DIRECTION_BIAS_THRESHOLD * 100.0
        ),
    ];
    if let Some(ratio) = historical_volume_ratio {
        reasons.push(format!(
            "relative volume expanded to {:.2}x against baseline.",
            ratio
        ));
    }
    reasons.push(format!("{} venues confirmed same-side flow.", venue_count));
    if let Some(price_impact_bps) = price_impact_bps {
        if price_impact_bps >= PRICE_IMPACT_THRESHOLD_BPS {
            reasons.push(format!("price impact reached {:.2} bps.", price_impact_bps));
        }
    }
    if let Some(depth_drop_ratio) = depth_drop_ratio {
        if depth_drop_ratio >= DEPTH_DROP_THRESHOLD {
            reasons.push(format!(
                "depth drop reached {:.0}%.",
                depth_drop_ratio * 100.0
            ));
        }
    }
    if dominant_volume_btc > 0.0 && reasons.is_empty() {
        reasons.push("bounded whale-flow gate passed.".to_string());
    }
    reasons
}

fn derive_candidate_data_quality(
    missing_inputs: &[String],
    degradation_reasons: &[String],
) -> String {
    if missing_inputs.is_empty() && degradation_reasons.is_empty() {
        "healthy".to_string()
    } else if !missing_inputs.is_empty() {
        "partial".to_string()
    } else {
        "degraded".to_string()
    }
}

fn build_no_candidate_reasons(
    volume_ok: bool,
    bias_ok: bool,
    history_ok: bool,
    multi_venue_ok: bool,
    candidate_gate: bool,
    baseline_quality: &WhaleFlowBaselineQuality,
    missing_inputs: &[String],
) -> Vec<String> {
    let mut reasons = Vec::new();
    if !volume_ok {
        reasons.push("active volume below threshold".to_string());
    }
    if !bias_ok {
        reasons.push("direction ratio below 70%".to_string());
    }
    if !multi_venue_ok {
        reasons.push("venue confluence below required minimum".to_string());
    }
    if !history_ok {
        reasons.push("relative volume baseline insufficient".to_string());
    }
    if !candidate_gate {
        reasons.push("price impact or depth-drop candidate gate not satisfied".to_string());
    }
    if baseline_quality.insufficient_history {
        reasons.push("insufficient baseline history".to_string());
    }
    if !missing_inputs.is_empty() {
        reasons.extend(missing_inputs.iter().cloned());
    }
    reasons
}

fn normalize_baseline_source(mode: &str) -> &'static str {
    match mode {
        "one_hour_normalized" => "one_hour_normalized",
        "minute_normalized" => "sixty_second_fallback",
        "longer_window_normalized" | "fallback_window_normalized" => "longer_window_fallback",
        _ => "insufficient_history",
    }
}

fn relevant_windows<'a>(requested_symbol: &str, flow_state: &'a FlowState) -> Vec<&'a FlowWindow> {
    flow_state
        .windows
        .values()
        .filter(|window| window.symbol.eq_ignore_ascii_case(requested_symbol))
        .collect()
}

fn active_venues(flow_windows: &[&FlowWindow]) -> BTreeSet<String> {
    let mut venues = BTreeSet::new();
    for window in flow_windows {
        for venue in &window.data_quality.active_venues {
            venues.insert(venue.to_ascii_lowercase());
        }
    }
    venues
}

fn total_aggressive_volume(window: &FlowWindow) -> f64 {
    (window.aggressive_buy_btc + window.aggressive_sell_btc).max(window.abs_aggressive_btc)
}

fn dominant_side(window: &FlowWindow) -> ToxicSide {
    if window.net_aggressive_btc > 0.0 {
        ToxicSide::Buy
    } else if window.net_aggressive_btc < 0.0 {
        ToxicSide::Sell
    } else {
        ToxicSide::Neutral
    }
}

fn dominant_volume(window: &FlowWindow, direction: ToxicSide) -> f64 {
    match direction {
        ToxicSide::Buy => window.aggressive_buy_btc,
        ToxicSide::Sell => window.aggressive_sell_btc,
        ToxicSide::Neutral => 0.0,
    }
}

fn toxic_direction(direction: ToxicSide) -> ToxicDirection {
    match direction {
        ToxicSide::Buy => ToxicDirection::Buy,
        ToxicSide::Sell => ToxicDirection::Sell,
        ToxicSide::Neutral => ToxicDirection::Neutral,
    }
}

fn historical_baseline<'a>(
    window_ms: u64,
    flow_windows: &[&'a FlowWindow],
) -> HistoricalBaseline<'a> {
    if let Some(window) = flow_windows
        .iter()
        .find(|window| window.window_ms == ONE_HOUR_WINDOW_MS)
    {
        return HistoricalBaseline {
            mode: "one_hour_normalized",
            source_window_ms: Some(window.window_ms),
            baseline_volume_btc: Some(normalize_volume(window, window_ms)),
            warning: None,
        };
    }

    if window_ms < 60_000 {
        if let Some(window) = flow_windows
            .iter()
            .find(|window| window.window_ms == 60_000)
        {
            return HistoricalBaseline {
                mode: "minute_normalized",
                source_window_ms: Some(window.window_ms),
                baseline_volume_btc: Some(normalize_volume(window, window_ms)),
                warning: Some(format!(
                    "1h baseline is unavailable; {} fell back to normalized 60s volume.",
                    format_window(window_ms)
                )),
            };
        }
    }

    if let Some(window) = flow_windows
        .iter()
        .filter(|window| window.window_ms > window_ms)
        .min_by_key(|window| window.window_ms)
    {
        return HistoricalBaseline {
            mode: "longer_window_normalized",
            source_window_ms: Some(window.window_ms),
            baseline_volume_btc: Some(normalize_volume(window, window_ms)),
            warning: Some(format!(
                "1h baseline is unavailable; {} fell back to normalized {} volume.",
                format_window(window_ms),
                format_window(window.window_ms)
            )),
        };
    }

    if let Some(window) = flow_windows
        .iter()
        .filter(|window| window.window_ms != window_ms)
        .max_by_key(|window| window.window_ms)
    {
        return HistoricalBaseline {
            mode: "fallback_window_normalized",
            source_window_ms: Some(window.window_ms),
            baseline_volume_btc: Some(normalize_volume(window, window_ms)),
            warning: Some(format!(
                "Historical baseline for {} used fallback normalized {} volume.",
                format_window(window_ms),
                format_window(window.window_ms)
            )),
        };
    }

    HistoricalBaseline {
        mode: "unavailable",
        source_window_ms: None,
        baseline_volume_btc: None,
        warning: Some(format!(
            "Historical baseline is unavailable for {}.",
            format_window(window_ms)
        )),
    }
}

fn normalize_volume(window: &FlowWindow, target_window_ms: u64) -> f64 {
    if window.window_ms == 0 {
        return 0.0;
    }
    total_aggressive_volume(window) * target_window_ms as f64 / window.window_ms as f64
}

fn matching_sweep_result<'a>(
    sweep_state: &'a SweepState,
    requested_symbol: &str,
    window_ms: u64,
) -> Option<&'a SweepResult> {
    sweep_state.results.values().find(|result| {
        result.symbol.eq_ignore_ascii_case(requested_symbol) && result.window_ms == window_ms
    })
}

fn matching_price_impact_bps(
    sweep_state: &SweepState,
    requested_symbol: &str,
    window_ms: u64,
) -> Option<f64> {
    matching_sweep_result(sweep_state, requested_symbol, window_ms)
        .and_then(|result| result.price_impact_bps)
        .map(f64::abs)
}

fn matching_depth_drop_ratio(
    sweep_state: &SweepState,
    requested_symbol: &str,
    window_ms: u64,
    direction: ToxicSide,
) -> Option<f64> {
    let liquidity = matching_sweep_result(sweep_state, requested_symbol, window_ms)?
        .liquidity
        .as_ref()?;
    match direction {
        ToxicSide::Buy => liquidity.ask_depth_drop_ratio,
        ToxicSide::Sell => liquidity.bid_depth_drop_ratio,
        ToxicSide::Neutral => None,
    }
}

fn classify_candidate(
    direction: ToxicSide,
    active_trade_recent: &ActiveTradeToxicityRecentResponse,
    liquidation_recent: &LiquidationToxicityRecentResponse,
    wall_lifecycle_report: &OrderbookWallLifecycleReport,
    wall_interpretation_report: &OrderbookWallInterpretationReport,
    structural_recent: &StructuralToxicityRecentResponse,
    fusion_recent: &ToxicSignalRecentResponse,
) -> CandidateClassification {
    let linked_liquidation_signal_ids: Vec<String> = liquidation_recent
        .signals
        .iter()
        .filter(|signal| liquidation_matches_direction(direction, signal.direction))
        .filter(|signal| {
            matches!(
                signal.signal_type,
                LiquidationToxicSignalType::LongSqueezeRisk
                    | LiquidationToxicSignalType::ShortSqueezeRisk
                    | LiquidationToxicSignalType::LiquidationCascadeCandidate
                    | LiquidationToxicSignalType::LiquidationDeltaConfluence
            )
        })
        .map(|signal| signal.signal_id.clone())
        .collect();
    if !linked_liquidation_signal_ids.is_empty() {
        return CandidateClassification {
            candidate_type: WhaleFlowCandidateType::LiquidationSweep,
            toxicity_score: 93,
            confidence: ToxicConfidence::High,
            primary_reason: "large flow aligned with liquidation pressure".to_string(),
            reason: vec![
                "liquidation toxicity already flagged same-direction squeeze or cascade pressure"
                    .to_string(),
            ],
            linked_active_trade_signal_ids: Vec::new(),
            linked_liquidation_signal_ids,
            linked_wall_candidate_ids: Vec::new(),
            linked_wall_interpretation_signal_ids: Vec::new(),
            linked_structural_signal_ids: Vec::new(),
            linked_fusion_signal_ids: Vec::new(),
        };
    }

    let linked_active_trade_signal_ids: Vec<String> = active_trade_recent
        .signals
        .iter()
        .filter(|signal| active_trade_matches_direction(direction, signal.side))
        .filter(|signal| signal.signal_type == ActiveTradeToxicSignalType::AbsorptionCandidate)
        .map(|signal| signal.signal_id.clone())
        .collect();
    let linked_wall_candidate_ids: Vec<String> = wall_lifecycle_report
        .toxicity_candidates
        .iter()
        .filter(|candidate| wall_candidate_matches_direction(direction, candidate.candidate_type))
        .map(|candidate| candidate.candidate_id.clone())
        .collect();
    let linked_wall_interpretation_signal_ids: Vec<String> = wall_interpretation_report
        .signals
        .iter()
        .filter(|signal| wall_interpretation_matches_direction(direction, signal.signal_type))
        .map(|signal| signal.signal_id.clone())
        .collect();
    if !linked_active_trade_signal_ids.is_empty()
        || !linked_wall_candidate_ids.is_empty()
        || !linked_wall_interpretation_signal_ids.is_empty()
    {
        return CandidateClassification {
            candidate_type: WhaleFlowCandidateType::Absorption,
            toxicity_score: 88,
            confidence: ToxicConfidence::High,
            primary_reason: "large flow met live absorption evidence".to_string(),
            reason: vec![
                "absorption evidence suggests size traded without clean continuation".to_string(),
            ],
            linked_active_trade_signal_ids,
            linked_liquidation_signal_ids: Vec::new(),
            linked_wall_candidate_ids,
            linked_wall_interpretation_signal_ids,
            linked_structural_signal_ids: Vec::new(),
            linked_fusion_signal_ids: Vec::new(),
        };
    }

    let linked_structural_signal_ids: Vec<String> = structural_recent
        .signals
        .iter()
        .filter(|signal| structural_matches_direction(direction, signal.direction))
        .filter(|signal| {
            matches!(
                signal.signal_type,
                StructuralToxicSignalType::BullTrapCandidate
                    | StructuralToxicSignalType::BearTrapCandidate
                    | StructuralToxicSignalType::SupportTrap
                    | StructuralToxicSignalType::ResistanceTrap
                    | StructuralToxicSignalType::StopHuntUpside
                    | StructuralToxicSignalType::StopHuntDownside
            )
        })
        .map(|signal| signal.signal_id.clone())
        .collect();
    let linked_fusion_signal_ids: Vec<String> = fusion_recent
        .signals
        .iter()
        .filter(|signal| fusion_matches_direction(direction, signal.direction))
        .filter(|signal| {
            matches!(
                signal.signal_type,
                ToxicSignalType::TrapRisk
                    | ToxicSignalType::BullTrapRisk
                    | ToxicSignalType::BearTrapRisk
            )
        })
        .map(|signal| signal.signal_id.clone())
        .collect();
    if !linked_structural_signal_ids.is_empty() || !linked_fusion_signal_ids.is_empty() {
        return CandidateClassification {
            candidate_type: WhaleFlowCandidateType::Trap,
            toxicity_score: 84,
            confidence: ToxicConfidence::Medium,
            primary_reason: "large flow lined up with trap evidence".to_string(),
            reason: vec![
                "structural or fusion trap signals suggest inducement rather than clean follow-through"
                    .to_string(),
            ],
            linked_active_trade_signal_ids: Vec::new(),
            linked_liquidation_signal_ids: Vec::new(),
            linked_wall_candidate_ids: Vec::new(),
            linked_wall_interpretation_signal_ids: Vec::new(),
            linked_structural_signal_ids,
            linked_fusion_signal_ids,
        };
    }

    let (candidate_type, primary_reason) = match direction {
        ToxicSide::Buy => (
            WhaleFlowCandidateType::AggressiveBuy,
            "large aggressive buy flow crossed the whale gate".to_string(),
        ),
        ToxicSide::Sell => (
            WhaleFlowCandidateType::AggressiveSell,
            "large aggressive sell flow crossed the whale gate".to_string(),
        ),
        ToxicSide::Neutral => (
            WhaleFlowCandidateType::Trap,
            "direction was neutral after gating".to_string(),
        ),
    };
    CandidateClassification {
        candidate_type,
        toxicity_score: 80,
        confidence: ToxicConfidence::Medium,
        primary_reason,
        reason: vec![
            "bounded whale-flow gate passed without stronger structural overrides".to_string(),
        ],
        linked_active_trade_signal_ids: Vec::new(),
        linked_liquidation_signal_ids: Vec::new(),
        linked_wall_candidate_ids: Vec::new(),
        linked_wall_interpretation_signal_ids: Vec::new(),
        linked_structural_signal_ids: Vec::new(),
        linked_fusion_signal_ids: Vec::new(),
    }
}

fn active_trade_matches_direction(direction: ToxicSide, side: ToxicSide) -> bool {
    matches!(
        (direction, side),
        (ToxicSide::Buy, ToxicSide::Buy) | (ToxicSide::Sell, ToxicSide::Sell)
    )
}

fn liquidation_matches_direction(
    direction: ToxicSide,
    liquidation_direction: LiquidationToxicDirection,
) -> bool {
    matches!(
        (direction, liquidation_direction),
        (ToxicSide::Buy, LiquidationToxicDirection::Upside)
            | (ToxicSide::Sell, LiquidationToxicDirection::Downside)
    )
}

fn wall_candidate_matches_direction(
    direction: ToxicSide,
    candidate_type: OrderbookWallCandidateType,
) -> bool {
    match direction {
        ToxicSide::Buy => matches!(
            candidate_type,
            OrderbookWallCandidateType::ResistanceAbsorption
                | OrderbookWallCandidateType::WallDeltaConfluence
        ),
        ToxicSide::Sell => matches!(
            candidate_type,
            OrderbookWallCandidateType::SupportAbsorption
                | OrderbookWallCandidateType::WallDeltaConfluence
        ),
        ToxicSide::Neutral => false,
    }
}

fn wall_interpretation_matches_direction(
    direction: ToxicSide,
    signal_type: OrderbookWallInterpretationType,
) -> bool {
    match direction {
        ToxicSide::Buy => matches!(
            signal_type,
            OrderbookWallInterpretationType::AskAbsorption
                | OrderbookWallInterpretationType::LiquidityInducementAbove
                | OrderbookWallInterpretationType::ResistanceWallFailure
        ),
        ToxicSide::Sell => matches!(
            signal_type,
            OrderbookWallInterpretationType::BidAbsorption
                | OrderbookWallInterpretationType::LiquidityInducementBelow
                | OrderbookWallInterpretationType::SupportWallFailure
        ),
        ToxicSide::Neutral => false,
    }
}

fn structural_matches_direction(
    direction: ToxicSide,
    structural_direction: StructuralToxicDirection,
) -> bool {
    matches!(
        (direction, structural_direction),
        (ToxicSide::Buy, StructuralToxicDirection::UpsideTrap)
            | (
                ToxicSide::Buy,
                StructuralToxicDirection::BullishReversalCandidate
            )
            | (ToxicSide::Sell, StructuralToxicDirection::DownsideTrap)
            | (
                ToxicSide::Sell,
                StructuralToxicDirection::BearishReversalCandidate
            )
    )
}

fn fusion_matches_direction(direction: ToxicSide, signal_direction: ToxicSignalDirection) -> bool {
    matches!(
        (direction, signal_direction),
        (ToxicSide::Buy, ToxicSignalDirection::LongBias)
            | (ToxicSide::Buy, ToxicSignalDirection::TrapRisk)
            | (ToxicSide::Sell, ToxicSignalDirection::ShortBias)
            | (ToxicSide::Sell, ToxicSignalDirection::TrapRisk)
    )
}

fn volume_threshold(window_ms: u64) -> f64 {
    match window_ms {
        1_000 => 100.0,
        5_000 => 300.0,
        15_000 => 800.0,
        60_000 => 2_000.0,
        _ => f64::MAX,
    }
}

fn format_window(window_ms: u64) -> &'static str {
    match window_ms {
        1_000 => "1s",
        5_000 => "5s",
        15_000 => "15s",
        60_000 => "60s",
        3_600_000 => "1h",
        _ => "custom",
    }
}

fn dedup_strings(items: &mut Vec<String>) {
    let mut seen = BTreeSet::new();
    items.retain(|item| seen.insert(item.clone()));
}
