use std::collections::{BTreeMap, VecDeque};

use super::types::{
    AltContractAgentDecision, AltContractAgentView, AltContractAtcaReport, AltContractDirection,
    AltContractMarketStateSnapshot, AltContractSeverity, AltContractSignal, AltContractSignalType,
    AltContractSmafReport, AltContractSmllReport,
};

const AGENT_LOOKBACK_MS: i64 = 24 * 60 * 60_000;
const MAX_AGENT_VIEWS: usize = 12;

pub fn run_trading_cognition_agent(
    now_ms: i64,
    signals: &VecDeque<AltContractSignal>,
    smaf_report: &AltContractSmafReport,
    smll_report: &AltContractSmllReport,
) -> AltContractAtcaReport {
    let latest_by_symbol = latest_signal_by_symbol(now_ms, signals);
    let mut agents = latest_by_symbol
        .values()
        .map(agent_view)
        .collect::<Vec<_>>();
    agents.sort_by(|left, right| {
        decision_rank(&right.decision)
            .cmp(&decision_rank(&left.decision))
            .then_with(|| right.confidence.total_cmp(&left.confidence))
    });
    agents.truncate(MAX_AGENT_VIEWS);

    let degraded = smaf_report.smaf_score < 60.0 || smll_report.drift_report.drift_detected;
    AltContractAtcaReport {
        enabled: true,
        protected_realtime: true,
        cognition_status: if agents.is_empty() {
            "waiting_for_signals".to_string()
        } else if degraded {
            "degraded_cognition".to_string()
        } else {
            "active_cognition".to_string()
        },
        memory_summary: memory_summary(agents.len(), smaf_report, smll_report),
        perception_count: agents.len(),
        interpretation_count: agents.len(),
        intention_count: agents.len(),
        prediction_count: agents
            .iter()
            .filter(|agent| agent.prediction != "unknown")
            .count(),
        decision_count: agents
            .iter()
            .filter(|agent| agent.decision.severity != "Ignore")
            .count(),
        agents,
    }
}

fn latest_signal_by_symbol(
    now_ms: i64,
    signals: &VecDeque<AltContractSignal>,
) -> BTreeMap<String, AltContractSignal> {
    let mut latest = BTreeMap::new();
    for signal in signals
        .iter()
        .filter(|signal| now_ms.saturating_sub(signal.ts) <= AGENT_LOOKBACK_MS)
    {
        latest
            .entry(signal.symbol.clone())
            .and_modify(|existing: &mut AltContractSignal| {
                if signal.ts > existing.ts {
                    *existing = signal.clone();
                }
            })
            .or_insert_with(|| signal.clone());
    }
    latest
}

fn agent_view(signal: &AltContractSignal) -> AltContractAgentView {
    let market_state = market_state(signal);
    let state = interpreted_state(signal);
    let intent = infer_intent(signal, &state);
    let prediction = signal.smart_money_prediction.next_state.clone();
    let confidence = cognition_confidence(signal);
    let risk = risk_level(signal, confidence);
    let decision = action_decision(signal, &intent, confidence);

    AltContractAgentView {
        symbol: signal.product_id.clone(),
        state,
        intent,
        prediction,
        decision,
        confidence,
        risk,
        market_state,
    }
}

fn market_state(signal: &AltContractSignal) -> AltContractMarketStateSnapshot {
    AltContractMarketStateSnapshot {
        symbol: signal.product_id.clone(),
        price_structure: price_structure(signal),
        volume_flow: volume_flow(signal),
        oi_movement: oi_movement(signal),
        liquidation_pressure: if signal.liquidation_suspected || signal.force_order_snapshot {
            "elevated".to_string()
        } else {
            "normal".to_string()
        },
        market_imbalance: round2(signal.dominance * 100.0),
    }
}

fn interpreted_state(signal: &AltContractSignal) -> String {
    if signal
        .market_regime
        .regime
        .eq_ignore_ascii_case("manipulation")
    {
        return "Manipulation".to_string();
    }
    let lifecycle_state = signal.smart_money_lifecycle.lifecycle_state.as_str();
    if lifecycle_state.is_empty() {
        signal.market_regime.regime.clone()
    } else {
        lifecycle_state.to_string()
    }
}

fn infer_intent(signal: &AltContractSignal, state: &str) -> String {
    if signal.liquidation_suspected || signal.force_order_snapshot {
        return "exit_liquidity".to_string();
    }
    if signal
        .market_regime
        .regime
        .eq_ignore_ascii_case("manipulation")
        || state.eq_ignore_ascii_case("manipulation")
    {
        return if signal.price_move_pct.unwrap_or_default() >= 0.0 {
            "trap".to_string()
        } else {
            "stop_hunt".to_string()
        };
    }
    match signal.signal_type {
        AltContractSignalType::MainForceLongBuild | AltContractSignalType::DownsideAbsorption => {
            "accumulate".to_string()
        }
        AltContractSignalType::MainForceShortBuild | AltContractSignalType::UpsideResistance => {
            "distribute".to_string()
        }
        AltContractSignalType::AbnormalPump | AltContractSignalType::AbnormalDump => {
            "trend_drive".to_string()
        }
        AltContractSignalType::LiquidationCascade => "exit_liquidity".to_string(),
        AltContractSignalType::UnclearContractAnomaly => "monitor".to_string(),
    }
}

fn action_decision(
    signal: &AltContractSignal,
    intent: &str,
    confidence: f64,
) -> AltContractAgentDecision {
    let notify = signal.discord_would_send
        || (signal.severity.rank() >= AltContractSeverity::Critical.rank()
            && signal.data_quality >= 70
            && confidence >= 70.0
            && intent != "monitor");
    AltContractAgentDecision {
        notify,
        severity: if notify {
            format!("{:?}", signal.severity)
        } else {
            "Ignore".to_string()
        },
        reason: if notify {
            format!("{} intent with {:.0}% confidence", intent, confidence)
        } else {
            "agent_filtered_low_confidence_or_display_only".to_string()
        },
    }
}

fn cognition_confidence(signal: &AltContractSignal) -> f64 {
    round2(
        (signal.data_quality as f64 * 0.25)
            + (signal.master_capital_strength.mcss * 0.25)
            + (signal.smart_money_lifecycle.state_confidence * 0.20)
            + (signal.smart_money_prediction.confidence * 0.15)
            + (signal.market_regime.confidence * 0.15),
    )
}

fn risk_level(signal: &AltContractSignal, confidence: f64) -> String {
    if signal.liquidation_suspected {
        "liquidation_risk".to_string()
    } else if confidence >= 85.0 || signal.severity == AltContractSeverity::S {
        "high".to_string()
    } else if confidence >= 70.0 || signal.severity.rank() >= AltContractSeverity::Critical.rank() {
        "medium".to_string()
    } else {
        "low".to_string()
    }
}

fn price_structure(signal: &AltContractSignal) -> String {
    match signal.price_move_pct {
        Some(value) if value > 0.15 => "breakout_up".to_string(),
        Some(value) if value < -0.15 => "breakdown_down".to_string(),
        Some(value) if value.abs() <= 0.05 => "flat".to_string(),
        Some(_) => "slow_move".to_string(),
        None => "unknown".to_string(),
    }
}

fn volume_flow(signal: &AltContractSignal) -> String {
    if signal.direction == AltContractDirection::Buy {
        "aggressive_buy".to_string()
    } else if signal.direction == AltContractDirection::Sell {
        "aggressive_sell".to_string()
    } else if signal.direction == AltContractDirection::Absorption {
        "absorption".to_string()
    } else if signal.direction == AltContractDirection::Suppression {
        "suppression".to_string()
    } else {
        "mixed".to_string()
    }
}

fn oi_movement(signal: &AltContractSignal) -> String {
    match signal.oi_change_pct {
        Some(value) if value > 0.2 => "expanding".to_string(),
        Some(value) if value < -0.2 => "contracting".to_string(),
        Some(_) => "flat".to_string(),
        None => "unknown".to_string(),
    }
}

fn memory_summary(
    agent_count: usize,
    smaf_report: &AltContractSmafReport,
    smll_report: &AltContractSmllReport,
) -> String {
    format!(
        "short_memory={} symbols · smaf={:.0} · learning_samples={}",
        agent_count, smaf_report.smaf_score, smll_report.sample_size
    )
}

fn decision_rank(decision: &AltContractAgentDecision) -> u8 {
    match decision.severity.as_str() {
        "S" => 4,
        "Critical" => 3,
        "High" => 2,
        _ => 0,
    }
}

fn round2(value: f64) -> f64 {
    (value * 100.0).round() / 100.0
}
