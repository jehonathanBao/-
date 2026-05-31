use crate::types::{
    liquidation::{LiquidationToxicDirection, LiquidationToxicityRecentResponse},
    orderbook_wall::{OrderbookWallInterpretationReport, OrderbookWallLifecycleReport},
    structural_toxicity::StructuralToxicityRecentResponse,
    toxic_flow::{ActiveTradeToxicSignal, ActiveTradeToxicityRecentResponse, ToxicConfidence},
    toxic_replay::{
        ToxicReplayBookContext, ToxicReplayContext, ToxicReplayDetail, ToxicReplayDetailResponse,
        ToxicReplayEvidenceBreakdown, ToxicReplayFlowContext, ToxicReplayLiquidationContext,
        ToxicReplayMarkoutPreview, ToxicReplayOperatorNarrative, ToxicReplayPriceContext,
        ToxicReplayRecentResponse, ToxicReplayReferenceLevels, ToxicReplaySignalSummary,
        ToxicReplayStatusResponse, ToxicReplayStructureContext,
    },
    toxic_signal::{ToxicChaseRisk, ToxicSignal, ToxicSignalRecentResponse},
};

pub fn build_toxic_replay_recent(
    requested_symbol: &str,
    fusion_recent: &ToxicSignalRecentResponse,
) -> ToxicReplayRecentResponse {
    let mut signals = fusion_recent
        .signals
        .iter()
        .filter(|signal| signal.symbol.eq_ignore_ascii_case(requested_symbol))
        .cloned()
        .collect::<Vec<_>>();
    signals.sort_by(|left, right| {
        right
            .ts_ms
            .cmp(&left.ts_ms)
            .then_with(|| right.toxicity_score.cmp(&left.toxicity_score))
    });

    ToxicReplayRecentResponse {
        read_only: true,
        runtime_modified: false,
        analysis_only: true,
        execution_enabled: false,
        mode: "analysis_only".to_string(),
        selected_symbol: requested_symbol.to_string(),
        status: if signals.is_empty() {
            "no_replay_signal".to_string()
        } else {
            "replay_ready".to_string()
        },
        warnings: fusion_recent.warnings.clone(),
        no_trade_reasons: fusion_recent.no_trade_reasons.clone(),
        signals: signals
            .into_iter()
            .map(|signal| {
                let signal_kind = signal_type_key(&signal);
                ToxicReplaySignalSummary {
                    signal_id: signal.signal_id,
                    signal_kind,
                    confidence: confidence_value(signal.confidence),
                    severity: severity_label(signal.toxicity_score).to_string(),
                    created_at: signal.ts_ms,
                    primary_reason: signal.primary_reason,
                    read_only: signal.read_only,
                }
            })
            .collect(),
    }
}

pub fn build_toxic_replay_status(
    requested_symbol: &str,
    fusion_recent: &ToxicSignalRecentResponse,
) -> ToxicReplayStatusResponse {
    let recent = build_toxic_replay_recent(requested_symbol, fusion_recent);
    ToxicReplayStatusResponse {
        read_only: true,
        runtime_modified: false,
        analysis_only: true,
        execution_enabled: false,
        enabled: true,
        mode: "analysis_only".to_string(),
        signal_count: recent.signals.len(),
        last_signal_at_ms: recent.signals.iter().map(|signal| signal.created_at).max(),
        safety_boundary: vec![
            "readOnly=true".to_string(),
            "runtimeModified=false".to_string(),
            "analysis_only".to_string(),
            "No order execution".to_string(),
            "No cancel/amend".to_string(),
            "No wallet".to_string(),
            "No signing".to_string(),
            "No transaction construction".to_string(),
        ],
    }
}

#[allow(clippy::too_many_arguments)]
pub fn build_toxic_replay_latest(
    requested_symbol: &str,
    fusion_recent: &ToxicSignalRecentResponse,
    active_trade_recent: &ActiveTradeToxicityRecentResponse,
    liquidation_recent: &LiquidationToxicityRecentResponse,
    wall_lifecycle_report: &OrderbookWallLifecycleReport,
    wall_interpretation_report: &OrderbookWallInterpretationReport,
    structural_recent: &StructuralToxicityRecentResponse,
) -> ToxicReplayDetailResponse {
    let latest_signal_id = fusion_recent
        .signals
        .iter()
        .filter(|signal| signal.symbol.eq_ignore_ascii_case(requested_symbol))
        .max_by(|left, right| {
            left.ts_ms
                .cmp(&right.ts_ms)
                .then_with(|| left.toxicity_score.cmp(&right.toxicity_score))
        })
        .map(|signal| signal.signal_id.clone());
    match latest_signal_id {
        Some(signal_id) => build_toxic_replay_by_signal_id(
            requested_symbol,
            &signal_id,
            fusion_recent,
            active_trade_recent,
            liquidation_recent,
            wall_lifecycle_report,
            wall_interpretation_report,
            structural_recent,
        ),
        None => unavailable_response(requested_symbol, "latest_signal_unavailable"),
    }
}

#[allow(clippy::too_many_arguments)]
pub fn build_toxic_replay_by_signal_id(
    requested_symbol: &str,
    signal_id: &str,
    fusion_recent: &ToxicSignalRecentResponse,
    active_trade_recent: &ActiveTradeToxicityRecentResponse,
    liquidation_recent: &LiquidationToxicityRecentResponse,
    wall_lifecycle_report: &OrderbookWallLifecycleReport,
    wall_interpretation_report: &OrderbookWallInterpretationReport,
    structural_recent: &StructuralToxicityRecentResponse,
) -> ToxicReplayDetailResponse {
    let Some(source_signal) = fusion_recent
        .signals
        .iter()
        .find(|signal| {
            signal.signal_id == signal_id && signal.symbol.eq_ignore_ascii_case(requested_symbol)
        })
        .cloned()
    else {
        return unavailable_response(requested_symbol, "signal_not_found");
    };

    let active_trade = active_trade_recent
        .signals
        .iter()
        .filter(|signal| {
            source_signal
                .linked_active_trade_signal_ids
                .iter()
                .any(|id| id == &signal.signal_id)
        })
        .cloned()
        .collect::<Vec<_>>();
    let liquidation = liquidation_recent
        .signals
        .iter()
        .filter(|signal| {
            source_signal
                .linked_liquidation_signal_ids
                .iter()
                .any(|id| id == &signal.signal_id)
        })
        .cloned()
        .collect::<Vec<_>>();
    let orderbook = wall_lifecycle_report
        .recent_events
        .iter()
        .filter(|event| {
            source_signal
                .linked_wall_lifecycle_signal_ids
                .iter()
                .any(|id| id == &event.event_id)
        })
        .cloned()
        .collect::<Vec<_>>();
    let wall_interpretation = wall_interpretation_report
        .signals
        .iter()
        .filter(|signal| {
            source_signal
                .linked_wall_interpretation_signal_ids
                .iter()
                .any(|id| id == &signal.signal_id)
        })
        .cloned()
        .collect::<Vec<_>>();
    let structural = structural_recent
        .signals
        .iter()
        .filter(|signal| {
            source_signal
                .linked_structural_signal_ids
                .iter()
                .any(|id| id == &signal.signal_id)
        })
        .cloned()
        .collect::<Vec<_>>();

    let evidence_breakdown = ToxicReplayEvidenceBreakdown {
        active_trade,
        liquidation,
        orderbook,
        wall_interpretation,
        structural,
    };
    let current_price_reference = evidence_breakdown
        .structural
        .first()
        .map(|signal| signal.current_price)
        .or_else(|| {
            evidence_breakdown
                .liquidation
                .first()
                .map(|signal| signal.current_price)
        })
        .or_else(|| {
            evidence_breakdown
                .active_trade
                .first()
                .and_then(|signal| signal.close)
        });
    let context = ToxicReplayContext {
        price: ToxicReplayPriceContext {
            current_price_reference,
            invalidation_price: source_signal.invalidation_price,
            suggested_stop_distance_usd: source_signal.suggested_stop_distance_usd,
        },
        book: ToxicReplayBookContext {
            status: wall_lifecycle_report.status.clone(),
            tracked_wall_count: wall_lifecycle_report.tracked_walls.len(),
            recent_event_count: evidence_breakdown.orderbook.len(),
        },
        flow: ToxicReplayFlowContext {
            status: active_trade_recent.status.clone(),
            signal_count: evidence_breakdown.active_trade.len(),
            side_bias: active_trade_recent.side_bias.clone(),
        },
        liquidation: ToxicReplayLiquidationContext {
            status: liquidation_recent.mode.clone(),
            signal_count: evidence_breakdown.liquidation.len(),
            dominant_bias: liquidation_bias_label(liquidation_recent),
        },
        structure: ToxicReplayStructureContext {
            status: structural_recent.status.clone(),
            signal_count: evidence_breakdown.structural.len(),
        },
    };
    let operator_narrative = ToxicReplayOperatorNarrative {
        why_signal_fired: std::iter::once(source_signal.primary_reason.clone())
            .chain(source_signal.reason.iter().cloned())
            .collect(),
        supporting_evidence: source_signal
            .supporting_evidence
            .iter()
            .map(|item| format!("{}: {}", item.source, item.summary))
            .collect(),
        conflicting_evidence: source_signal.no_trade_reasons.clone(),
        why_not_entry_signal: vec![
            "This replay surface is analysis_only and does not place, amend, or cancel orders."
                .to_string(),
            "Reference levels remain informational only and are not order instructions."
                .to_string(),
        ],
        risk_warnings: build_risk_warnings(&source_signal),
    };
    let reference_levels = ToxicReplayReferenceLevels {
        invalidation_price: source_signal.invalidation_price,
        suggested_stop_distance_usd: source_signal.suggested_stop_distance_usd,
        wording: "Reference only. No order instruction.".to_string(),
    };
    let replay = ToxicReplayDetail {
        read_only: true,
        runtime_modified: false,
        analysis_only: true,
        execution_enabled: false,
        mode: "analysis_only".to_string(),
        symbol: requested_symbol.to_string(),
        signal_id: source_signal.signal_id.clone(),
        signal_kind: signal_type_key(&source_signal),
        confidence: confidence_value(source_signal.confidence),
        severity: severity_label(source_signal.toxicity_score).to_string(),
        created_at: source_signal.ts_ms,
        source_signal,
        evidence_breakdown,
        context,
        operator_narrative,
        reference_levels,
        markout_preview: ToxicReplayMarkoutPreview {
            available: false,
            note: "Reserved for T8 markout evaluation.".to_string(),
        },
    };

    ToxicReplayDetailResponse {
        read_only: true,
        runtime_modified: false,
        analysis_only: true,
        execution_enabled: false,
        mode: "analysis_only".to_string(),
        symbol: requested_symbol.to_string(),
        available: true,
        reason: None,
        replay: Some(replay),
    }
}

fn unavailable_response(symbol: &str, reason: &str) -> ToxicReplayDetailResponse {
    ToxicReplayDetailResponse {
        read_only: true,
        runtime_modified: false,
        analysis_only: true,
        execution_enabled: false,
        mode: "analysis_only".to_string(),
        symbol: symbol.to_string(),
        available: false,
        reason: Some(reason.to_string()),
        replay: None,
    }
}

fn confidence_value(confidence: ToxicConfidence) -> f64 {
    match confidence {
        ToxicConfidence::High => 0.85,
        ToxicConfidence::Medium => 0.65,
        ToxicConfidence::Low => 0.45,
    }
}

fn severity_label(score: u8) -> &'static str {
    if score >= 80 {
        "high"
    } else if score >= 60 {
        "medium"
    } else {
        "low"
    }
}

fn liquidation_bias_label(recent: &LiquidationToxicityRecentResponse) -> String {
    recent
        .signals
        .first()
        .map(|signal| match signal.direction {
            LiquidationToxicDirection::Upside => "upside".to_string(),
            LiquidationToxicDirection::Downside => "downside".to_string(),
            LiquidationToxicDirection::Neutral => "neutral".to_string(),
        })
        .unwrap_or_else(|| "neutral".to_string())
}

fn build_risk_warnings(signal: &ToxicSignal) -> Vec<String> {
    let mut warnings = signal.no_trade_reasons.clone();
    warnings.push(match signal.chase_risk {
        ToxicChaseRisk::High => "Chase risk is high.".to_string(),
        ToxicChaseRisk::Medium => "Chase risk is medium.".to_string(),
        ToxicChaseRisk::Low => "Chase risk is low.".to_string(),
    });
    warnings.push("High toxicity does not turn this into an execution signal.".to_string());
    warnings.sort();
    warnings.dedup();
    warnings
}

fn signal_type_key(signal: &ToxicSignal) -> String {
    let mut output = String::new();
    let debug = format!("{:?}", signal.signal_type);
    for (index, ch) in debug.chars().enumerate() {
        if ch.is_uppercase() && index > 0 {
            output.push('_');
        }
        output.extend(ch.to_lowercase());
    }
    output
}

#[allow(dead_code)]
fn _side_bias_from_active(signal: &ActiveTradeToxicSignal) -> &'static str {
    match signal.side {
        crate::types::toxic_flow::ToxicSide::Buy => "buy",
        crate::types::toxic_flow::ToxicSide::Sell => "sell",
        crate::types::toxic_flow::ToxicSide::Neutral => "neutral",
    }
}
