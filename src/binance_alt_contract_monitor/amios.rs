use std::collections::VecDeque;

use super::types::{
    AltContractAtcaReport, AltContractMarketOsProcess, AltContractMarketOsReport,
    AltContractMarketOsState, AltContractSeverity, AltContractSignal, AltContractSmafReport,
    AltContractSmllReport,
};

const OS_LOOKBACK_MS: i64 = 60 * 60_000;
const MAX_CURRENT_STATES: usize = 4;

pub fn run_market_intelligence_os(
    now_ms: i64,
    signals: &VecDeque<AltContractSignal>,
    smaf_report: &AltContractSmafReport,
    smll_report: &AltContractSmllReport,
    atca_report: &AltContractAtcaReport,
) -> AltContractMarketOsReport {
    let mut recent = signals
        .iter()
        .filter(|signal| now_ms.saturating_sub(signal.ts) <= OS_LOOKBACK_MS)
        .collect::<Vec<_>>();
    recent.sort_by(|left, right| right.ts.cmp(&left.ts));

    let latest = recent.first().copied();
    let current_states = recent
        .iter()
        .take(MAX_CURRENT_STATES)
        .map(|signal| market_os_state(signal))
        .collect::<Vec<_>>();

    let kernel_load = latest.map(kernel_load_for_signal).unwrap_or_default();
    let confidence = confidence_for(&recent, smaf_report, atca_report);
    let risk = os_risk(latest, smaf_report, smll_report, confidence);
    let market_state = os_market_state(latest);

    AltContractMarketOsReport {
        enabled: true,
        protected_realtime: true,
        os_status: if recent.is_empty() {
            "idle".to_string()
        } else if smaf_report.smaf_score < 60.0 || smll_report.drift_report.drift_detected {
            "degraded".to_string()
        } else {
            "running".to_string()
        },
        market_state,
        kernel_load,
        signal_throughput: signal_throughput(recent.len()),
        confidence,
        risk,
        active_processes: active_processes(latest, smaf_report, smll_report, atca_report),
        current_states,
        scheduler_decision: scheduler_decision(latest, confidence),
        audit_summary: audit_summary(smaf_report, smll_report, atca_report),
        read_only: true,
        direct_discord_gate: false,
    }
}

fn active_processes(
    latest: Option<&AltContractSignal>,
    smaf_report: &AltContractSmafReport,
    smll_report: &AltContractSmllReport,
    atca_report: &AltContractAtcaReport,
) -> Vec<AltContractMarketOsProcess> {
    vec![
        process(
            "BACM",
            "kernel",
            latest.map(signal_activity_status).unwrap_or("standby"),
            latest
                .map(|signal| {
                    (signal.abnormal_score as f64 * 0.5) + (signal.build_score as f64 * 0.5)
                })
                .unwrap_or_default(),
            "market_event_interrupts",
        ),
        process(
            "MCSS",
            "kernel",
            latest.map(signal_activity_status).unwrap_or("standby"),
            latest
                .map(|signal| signal.master_capital_strength.mcss)
                .unwrap_or_default(),
            "capital_strength",
        ),
        process(
            "LME",
            "graph",
            latest.map(signal_activity_status).unwrap_or("standby"),
            latest
                .map(|signal| signal.liquidity_microstructure.lms_score)
                .unwrap_or_default(),
            "microstructure_read",
        ),
        process(
            "SMLE",
            "process",
            latest
                .map(|signal| non_empty_status(&signal.smart_money_lifecycle.lifecycle_state))
                .unwrap_or("standby"),
            latest
                .map(|signal| signal.smart_money_lifecycle.lifecycle_score)
                .unwrap_or_default(),
            "behavior_lifecycle",
        ),
        process(
            "SMP",
            "process",
            latest
                .map(|signal| non_empty_status(&signal.smart_money_prediction.next_state))
                .unwrap_or("standby"),
            latest
                .map(|signal| signal.smart_money_prediction.confidence)
                .unwrap_or_default(),
            "state_transition_prediction",
        ),
        process(
            "MCG",
            "graph",
            latest
                .map(|signal| non_empty_status(&signal.market_control_graph.control_type))
                .unwrap_or("standby"),
            latest
                .map(|signal| signal.market_control_graph.control_strength)
                .unwrap_or_default(),
            "control_graph",
        ),
        process(
            "SMAF",
            "audit",
            if smaf_report.smaf_score >= 70.0 {
                "stable"
            } else {
                "watch"
            },
            smaf_report.smaf_score,
            "system_reliability",
        ),
        process(
            "SMLL",
            "scheduler",
            if smll_report.drift_report.drift_detected {
                "drift_watch"
            } else if smll_report.sample_size > 0 {
                "observing"
            } else {
                "collecting"
            },
            smll_report.learning_score,
            "delayed_learning",
        ),
        process(
            "ATCA",
            "scheduler",
            atca_report.cognition_status.as_str(),
            agent_confidence(atca_report),
            "read_only_decision_routing",
        ),
        process(
            "SCC",
            "audit",
            latest
                .map(|signal| {
                    if signal.signal_confidence.confidence_score > 0.0 {
                        "calibrated"
                    } else {
                        "standby"
                    }
                })
                .unwrap_or("standby"),
            latest
                .map(|signal| signal.signal_confidence.confidence_score)
                .unwrap_or_default(),
            "confidence_calibration",
        ),
    ]
}

fn market_os_state(signal: &AltContractSignal) -> AltContractMarketOsState {
    AltContractMarketOsState {
        symbol: signal.product_id.clone(),
        market_state: os_market_state(Some(signal)),
        kernel_load: kernel_load_for_signal(signal),
        confidence: signal_confidence(signal),
        regime: non_empty_value(&signal.market_regime.regime, "Unknown"),
        lifecycle_state: non_empty_value(&signal.smart_money_lifecycle.lifecycle_state, "Unknown"),
        prediction: non_empty_value(&signal.smart_money_prediction.next_state, "Unknown"),
        control: control_label(signal),
        risk: signal_risk(signal, signal_confidence(signal)),
        explanation: state_explanation(signal),
    }
}

fn process(
    name: &str,
    layer: &str,
    status: &str,
    load: f64,
    role: &str,
) -> AltContractMarketOsProcess {
    AltContractMarketOsProcess {
        name: name.to_string(),
        layer: layer.to_string(),
        status: status.to_string(),
        load: round2(load.clamp(0.0, 100.0)),
        role: role.to_string(),
    }
}

fn os_market_state(latest: Option<&AltContractSignal>) -> String {
    let Some(signal) = latest else {
        return "CALM".to_string();
    };
    if signal.liquidation_suspected || signal.force_order_snapshot {
        return "INTERRUPT_LIQUIDATION_MODE".to_string();
    }
    if signal.market_control_graph.control_strength >= 70.0 {
        return "ACTIVE_CONTROL_MODE".to_string();
    }
    if signal.smart_money_lifecycle.lifecycle_score >= 60.0
        || signal.master_capital_strength.mcss >= 70.0
    {
        return "BEHAVIOR_PROCESS_MODE".to_string();
    }
    "OBSERVATION_MODE".to_string()
}

fn scheduler_decision(latest: Option<&AltContractSignal>, confidence: f64) -> String {
    let Some(signal) = latest else {
        return "standby".to_string();
    };
    if signal.liquidation_suspected || signal.force_order_snapshot {
        "interrupt_priority".to_string()
    } else if signal.severity.rank() >= AltContractSeverity::Critical.rank() && confidence >= 75.0 {
        "monitor_high_confidence".to_string()
    } else if signal.severity.rank() >= AltContractSeverity::High.rank() {
        "observe_candidate".to_string()
    } else {
        "standby".to_string()
    }
}

fn os_risk(
    latest: Option<&AltContractSignal>,
    smaf_report: &AltContractSmafReport,
    smll_report: &AltContractSmllReport,
    confidence: f64,
) -> String {
    if smaf_report.smaf_score < 50.0 || !smaf_report.critical_issues.is_empty() {
        return "system_risk".to_string();
    }
    if smll_report.drift_report.drift_detected {
        return "model_drift_watch".to_string();
    }
    match latest {
        Some(signal) if signal.liquidation_suspected || signal.force_order_snapshot => {
            "liquidation_interrupt".to_string()
        }
        Some(signal) if signal.severity == AltContractSeverity::S || confidence >= 85.0 => {
            "high_market_risk".to_string()
        }
        Some(signal) if signal.severity.rank() >= AltContractSeverity::Critical.rank() => {
            "market_risk".to_string()
        }
        _ => "normal".to_string(),
    }
}

fn signal_throughput(count: usize) -> String {
    if count >= 30 {
        "high".to_string()
    } else if count >= 8 {
        "normal".to_string()
    } else if count > 0 {
        "low".to_string()
    } else {
        "quiet".to_string()
    }
}

fn confidence_for(
    recent: &[&AltContractSignal],
    smaf_report: &AltContractSmafReport,
    atca_report: &AltContractAtcaReport,
) -> f64 {
    let signal_scores = recent
        .iter()
        .take(MAX_CURRENT_STATES)
        .map(|signal| signal_confidence(signal))
        .filter(|score| *score > 0.0)
        .collect::<Vec<_>>();
    if !signal_scores.is_empty() {
        return round2(signal_scores.iter().sum::<f64>() / signal_scores.len() as f64);
    }
    let agent_score = agent_confidence(atca_report);
    if agent_score > 0.0 {
        agent_score
    } else {
        smaf_report.smaf_score
    }
}

fn signal_confidence(signal: &AltContractSignal) -> f64 {
    if signal.signal_confidence.confidence_score > 0.0 {
        signal.signal_confidence.confidence_score
    } else {
        round2(
            (signal.data_quality as f64 * 0.25)
                + (signal.master_capital_strength.mcss * 0.25)
                + (signal.smart_money_lifecycle.state_confidence * 0.20)
                + (signal.smart_money_prediction.confidence * 0.15)
                + (signal.market_control_graph.control_strength * 0.15),
        )
    }
}

fn kernel_load_for_signal(signal: &AltContractSignal) -> f64 {
    round2(
        (signal.abnormal_score as f64 * 0.15)
            + (signal.build_score as f64 * 0.15)
            + (signal.master_capital_strength.mcss * 0.20)
            + (signal.liquidity_microstructure.lms_score * 0.15)
            + (signal.market_control_graph.control_strength * 0.15)
            + (signal
                .signal_confidence
                .confidence_score
                .max(signal.data_quality as f64)
                * 0.20),
    )
}

fn signal_activity_status(signal: &AltContractSignal) -> &'static str {
    if signal.severity.rank() >= AltContractSeverity::Critical.rank() {
        "interrupt"
    } else {
        "active"
    }
}

fn non_empty_status(value: &str) -> &'static str {
    if value.trim().is_empty() || value.eq_ignore_ascii_case("unknown") {
        "standby"
    } else {
        "active"
    }
}

fn non_empty_value(value: &str, fallback: &str) -> String {
    if value.trim().is_empty() {
        fallback.to_string()
    } else {
        value.to_string()
    }
}

fn control_label(signal: &AltContractSignal) -> String {
    let side = signal.market_control_graph.dominant_side.as_str();
    let side = if side.is_empty() { "neutral" } else { side };
    format!(
        "{}:{}",
        side,
        non_empty_value(&signal.market_control_graph.control_type, "NoClearControl")
    )
}

fn signal_risk(signal: &AltContractSignal, confidence: f64) -> String {
    if signal.liquidation_suspected || signal.force_order_snapshot {
        "liquidation_risk".to_string()
    } else if confidence >= 85.0 || signal.severity == AltContractSeverity::S {
        "high".to_string()
    } else if confidence >= 70.0 || signal.severity.rank() >= AltContractSeverity::Critical.rank() {
        "medium".to_string()
    } else {
        "low".to_string()
    }
}

fn state_explanation(signal: &AltContractSignal) -> String {
    if signal.liquidation_suspected || signal.force_order_snapshot {
        return "清算或强平快照触发 OS interrupt，AMIOS 仅提升观察优先级。".to_string();
    }
    if signal.market_control_graph.control_strength >= 70.0 {
        return format!(
            "控制图谱显示 {}，控制强度 {:.0}/100。",
            non_empty_value(&signal.market_control_graph.control_type, "NoClearControl"),
            signal.market_control_graph.control_strength
        );
    }
    if !signal.smart_money_lifecycle.lifecycle_state.is_empty() {
        return format!(
            "生命周期进程处于 {}，用于解释市场行为阶段。",
            signal.smart_money_lifecycle.lifecycle_state
        );
    }
    "候选信号进入观察队列，未改变任何推送或执行边界。".to_string()
}

fn agent_confidence(atca_report: &AltContractAtcaReport) -> f64 {
    if atca_report.agents.is_empty() {
        0.0
    } else {
        round2(
            atca_report
                .agents
                .iter()
                .map(|agent| agent.confidence)
                .sum::<f64>()
                / atca_report.agents.len() as f64,
        )
    }
}

fn audit_summary(
    smaf_report: &AltContractSmafReport,
    smll_report: &AltContractSmllReport,
    atca_report: &AltContractAtcaReport,
) -> String {
    format!(
        "smaf={:.0} smll_samples={} atca={} read_only=true direct_discord_gate=false",
        smaf_report.smaf_score, smll_report.sample_size, atca_report.cognition_status
    )
}

fn round2(value: f64) -> f64 {
    (value * 100.0).round() / 100.0
}
