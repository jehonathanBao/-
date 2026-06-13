use std::collections::{BTreeMap, BTreeSet, VecDeque};

use super::types::{
    AltContractBehaviorAuditResult, AltContractDataAuditResult, AltContractExchangeStatus,
    AltContractPredictionAuditResult, AltContractSeverity, AltContractSignal,
    AltContractSignalAuditResult, AltContractSmafReport,
};

const RECENT_SIGNAL_WINDOW_MS: i64 = 60 * 60_000;
const FRESH_TRADE_MS: i64 = 30_000;
const STALE_TRADE_MS: i64 = 120_000;
const FRESH_CONTEXT_MS: i64 = 90_000;
const STALE_CONTEXT_MS: i64 = 5 * 60_000;
const DUPLICATE_WINDOW_MS: i64 = 60_000;

pub struct SmafAuditInput<'a> {
    pub enabled: bool,
    pub now_ms: i64,
    pub exchanges: &'a BTreeMap<String, AltContractExchangeStatus>,
    pub signals: &'a VecDeque<AltContractSignal>,
    pub last_oi_poll_at: Option<i64>,
    pub last_force_order_at: Option<i64>,
    pub last_mark_price_at: Option<i64>,
    pub last_ticker_at: Option<i64>,
    pub errors1h: usize,
}

pub fn audit_smart_money_system(input: SmafAuditInput<'_>) -> AltContractSmafReport {
    let recent_signals = input
        .signals
        .iter()
        .filter(|signal| input.now_ms.saturating_sub(signal.ts) <= RECENT_SIGNAL_WINDOW_MS)
        .collect::<Vec<_>>();
    let data_audit = audit_data(&input);
    let signal_audit = audit_signals(&recent_signals);
    let behavior_audit = audit_behavior(&recent_signals);
    let prediction_audit = audit_predictions(&recent_signals);
    let smaf_score = round2(
        (data_audit.integrity_score
            + signal_audit.integrity_score
            + behavior_audit.structural_integrity
            + prediction_audit.integrity_score)
            / 4.0,
    );
    let mut critical_issues = Vec::new();
    if data_audit.integrity_score < 60.0 {
        critical_issues.push("data_integrity_low".to_string());
    }
    if signal_audit.single_source_dependency >= 80.0 {
        critical_issues.push("single_source_dependency_high".to_string());
    }
    if signal_audit.duplication_rate >= 30.0 {
        critical_issues.push("duplicate_signal_rate_high".to_string());
    }
    if behavior_audit.transition_entropy >= 50.0 {
        critical_issues.push("lifecycle_transition_entropy_high".to_string());
    }
    if behavior_audit.manipulation_noise >= 40.0 {
        critical_issues.push("manipulation_noise_high".to_string());
    }
    if prediction_audit.flip_rate >= 50.0 {
        critical_issues.push("prediction_flip_rate_high".to_string());
    }
    if prediction_audit.overfitting_score >= 50.0 {
        critical_issues.push("prediction_overfitting_risk".to_string());
    }
    AltContractSmafReport {
        data_audit,
        signal_audit,
        behavior_audit,
        prediction_audit,
        smaf_score,
        risk_level: smaf_risk_level(smaf_score).to_string(),
        critical_issues,
    }
}

fn audit_data(input: &SmafAuditInput<'_>) -> AltContractDataAuditResult {
    if !input.enabled {
        return AltContractDataAuditResult {
            freshness_score: 0.0,
            completeness_score: 0.0,
            consistency_score: 0.0,
            integrity_score: 0.0,
            data_risk_level: "disabled".to_string(),
        };
    }

    let active_exchanges = input
        .exchanges
        .values()
        .filter(|status| status.status != "disabled")
        .collect::<Vec<_>>();
    let exchange_freshness = average(
        active_exchanges
            .iter()
            .map(|status| freshness_from_last_seen(input.now_ms, status.last_trade_at)),
    );
    let context_freshness = average([
        optional_context_freshness(input.now_ms, input.last_oi_poll_at),
        optional_context_freshness(input.now_ms, input.last_mark_price_at),
        optional_context_freshness(input.now_ms, input.last_ticker_at),
        optional_context_freshness(input.now_ms, input.last_force_order_at),
    ]);
    let freshness_score = round2((exchange_freshness * 0.65) + (context_freshness * 0.35));

    let connected_exchanges = active_exchanges
        .iter()
        .filter(|status| status.connected && status.last_trade_at.is_some())
        .count();
    let exchange_coverage = ratio_score(connected_exchanges, active_exchanges.len());
    let context_coverage = ratio_score(
        [
            input.last_oi_poll_at,
            input.last_mark_price_at,
            input.last_ticker_at,
            input.last_force_order_at,
        ]
        .into_iter()
        .filter(Option::is_some)
        .count(),
        4,
    );
    let completeness_score = round2((exchange_coverage * 0.6) + (context_coverage * 0.4));

    let stale_sources = active_exchanges
        .iter()
        .filter(|status| freshness_from_last_seen(input.now_ms, status.last_trade_at) < 60.0)
        .count();
    let consistency_penalty = (input.errors1h as f64 * 4.0) + (stale_sources as f64 * 12.0);
    let consistency_score = round2(clamp_score(100.0 - consistency_penalty));
    let integrity_score = round2((freshness_score + completeness_score + consistency_score) / 3.0);

    AltContractDataAuditResult {
        freshness_score,
        completeness_score,
        consistency_score,
        integrity_score,
        data_risk_level: data_risk_level(integrity_score).to_string(),
    }
}

fn audit_signals(signals: &[&AltContractSignal]) -> AltContractSignalAuditResult {
    if signals.is_empty() {
        return AltContractSignalAuditResult {
            noise_ratio: 0.0,
            duplication_rate: 0.0,
            single_source_dependency: 0.0,
            false_signal_estimate: 0.0,
            integrity_score: 100.0,
        };
    }
    let total = signals.len() as f64;
    let noisy = signals
        .iter()
        .filter(|signal| {
            signal.severity.rank() <= AltContractSeverity::Medium.rank()
                || signal.discord_reason == "low_score"
                || signal.discord_reason == "low_notional"
        })
        .count() as f64;
    let duplicates = duplicate_count(signals) as f64;
    let single_source = signals
        .iter()
        .filter(|signal| {
            let active_sources = signal
                .active_sources
                .iter()
                .filter(|source| source.enabled && source.status == "active")
                .count();
            active_sources <= 1 && signal.exchanges.len() <= 1
        })
        .count() as f64;
    let false_like = signals
        .iter()
        .filter(|signal| {
            signal.liquidation_suspected
                || signal.data_quality < 70
                || signal.post_signal_status == "failed"
                || signal.market_wide_move
        })
        .count() as f64;
    let noise_ratio = round2(noisy / total * 100.0);
    let duplication_rate = round2(duplicates / total * 100.0);
    let single_source_dependency = round2(single_source / total * 100.0);
    let false_signal_estimate = round2(false_like / total * 100.0);
    let integrity_score = round2(clamp_score(
        100.0
            - (noise_ratio * 0.25)
            - (duplication_rate * 0.25)
            - (single_source_dependency * 0.25)
            - (false_signal_estimate * 0.25),
    ));

    AltContractSignalAuditResult {
        noise_ratio,
        duplication_rate,
        single_source_dependency,
        false_signal_estimate,
        integrity_score,
    }
}

fn audit_behavior(signals: &[&AltContractSignal]) -> AltContractBehaviorAuditResult {
    if signals.len() < 2 {
        return AltContractBehaviorAuditResult {
            state_stability: 100.0,
            transition_entropy: 0.0,
            manipulation_noise: 0.0,
            structural_integrity: 100.0,
        };
    }
    let states = signals
        .iter()
        .map(|signal| normalize_state(&signal.smart_money_lifecycle.lifecycle_state))
        .collect::<Vec<_>>();
    let transitions = states
        .windows(2)
        .filter(|pair| pair[0] != pair[1])
        .collect::<Vec<_>>();
    let transition_ratio = transitions.len() as f64 / (states.len().saturating_sub(1)) as f64;
    let state_stability = round2(clamp_score(100.0 - (transition_ratio * 100.0)));
    let unique_transition_count = transitions
        .iter()
        .map(|pair| format!("{}>{}", pair[0], pair[1]))
        .collect::<BTreeSet<_>>()
        .len();
    let transition_entropy = if transitions.is_empty() {
        0.0
    } else {
        round2(unique_transition_count as f64 / transitions.len() as f64 * 100.0)
    };
    let manipulation_noise = round2(
        signals
            .iter()
            .filter(|signal| {
                normalize_state(&signal.market_regime.regime).contains("manipulation")
                    || signal
                        .smart_money_lifecycle
                        .explanation_tags
                        .iter()
                        .any(|tag| tag.contains("manipulation"))
            })
            .count() as f64
            / signals.len() as f64
            * 100.0,
    );
    let avg_confidence = average(
        signals
            .iter()
            .map(|signal| signal.smart_money_lifecycle.state_confidence),
    );
    let structural_integrity = round2(clamp_score(
        (state_stability * 0.35)
            + (avg_confidence * 0.25)
            + ((100.0 - transition_entropy) * 0.2)
            + ((100.0 - manipulation_noise) * 0.2),
    ));

    AltContractBehaviorAuditResult {
        state_stability,
        transition_entropy,
        manipulation_noise,
        structural_integrity,
    }
}

fn audit_predictions(signals: &[&AltContractSignal]) -> AltContractPredictionAuditResult {
    if signals.len() < 2 {
        return AltContractPredictionAuditResult {
            accuracy: 100.0,
            flip_rate: 0.0,
            overfitting_score: 0.0,
            follow_through_rate: 100.0,
            integrity_score: 100.0,
        };
    }
    let matches = signals
        .windows(2)
        .filter(|pair| {
            normalize_state(&pair[0].smart_money_prediction.next_state)
                == normalize_state(&pair[1].smart_money_lifecycle.lifecycle_state)
        })
        .count();
    let pairs = signals.len().saturating_sub(1);
    let accuracy = round2(matches as f64 / pairs as f64 * 100.0);
    let prediction_states = signals
        .iter()
        .map(|signal| normalize_state(&signal.smart_money_prediction.next_state))
        .collect::<Vec<_>>();
    let flips = prediction_states
        .windows(2)
        .filter(|pair| pair[0] != pair[1])
        .count();
    let flip_rate = round2(flips as f64 / pairs as f64 * 100.0);
    let avg_prediction_confidence = average(
        signals
            .iter()
            .map(|signal| signal.smart_money_prediction.confidence),
    );
    let overfitting_score = round2(clamp_score(if avg_prediction_confidence >= 80.0 {
        (flip_rate * 0.7) + ((100.0 - accuracy) * 0.3)
    } else {
        flip_rate * 0.35
    }));
    let follow_through_rate = accuracy;
    let integrity_score = round2(clamp_score(
        (accuracy * 0.35)
            + ((100.0 - flip_rate) * 0.25)
            + ((100.0 - overfitting_score) * 0.2)
            + (follow_through_rate * 0.2),
    ));

    AltContractPredictionAuditResult {
        accuracy,
        flip_rate,
        overfitting_score,
        follow_through_rate,
        integrity_score,
    }
}

fn freshness_from_last_seen(now_ms: i64, last_seen_at: Option<i64>) -> f64 {
    let Some(last_seen_at) = last_seen_at else {
        return 0.0;
    };
    let age = now_ms.saturating_sub(last_seen_at);
    if age <= FRESH_TRADE_MS {
        100.0
    } else if age <= STALE_TRADE_MS {
        70.0
    } else {
        20.0
    }
}

fn optional_context_freshness(now_ms: i64, last_seen_at: Option<i64>) -> f64 {
    let Some(last_seen_at) = last_seen_at else {
        return 70.0;
    };
    let age = now_ms.saturating_sub(last_seen_at);
    if age <= FRESH_CONTEXT_MS {
        100.0
    } else if age <= STALE_CONTEXT_MS {
        70.0
    } else {
        20.0
    }
}

fn duplicate_count(signals: &[&AltContractSignal]) -> usize {
    let mut duplicates = 0;
    for (index, signal) in signals.iter().enumerate() {
        if signals[..index].iter().any(|previous| {
            previous.product_id == signal.product_id
                && previous.signal_type == signal.signal_type
                && previous.direction == signal.direction
                && signal.ts.saturating_sub(previous.ts) <= DUPLICATE_WINDOW_MS
        }) {
            duplicates += 1;
        }
    }
    duplicates
}

fn ratio_score(count: usize, total: usize) -> f64 {
    if total == 0 {
        return 0.0;
    }
    count as f64 / total as f64 * 100.0
}

fn average(values: impl IntoIterator<Item = f64>) -> f64 {
    let mut count = 0;
    let sum = values.into_iter().fold(0.0, |acc, value| {
        count += 1;
        acc + value
    });
    if count == 0 {
        0.0
    } else {
        sum / count as f64
    }
}

fn data_risk_level(score: f64) -> &'static str {
    if score >= 85.0 {
        "low"
    } else if score >= 65.0 {
        "medium"
    } else {
        "high"
    }
}

fn smaf_risk_level(score: f64) -> &'static str {
    if score >= 90.0 {
        "Production Ready"
    } else if score >= 75.0 {
        "Stable but tuning needed"
    } else if score >= 60.0 {
        "Risky"
    } else {
        "Not reliable"
    }
}

fn normalize_state(value: &str) -> String {
    value
        .chars()
        .filter(|ch| ch.is_ascii_alphanumeric())
        .collect::<String>()
        .to_ascii_lowercase()
}

fn clamp_score(value: f64) -> f64 {
    value.clamp(0.0, 100.0)
}

fn round2(value: f64) -> f64 {
    (clamp_score(value) * 100.0).round() / 100.0
}
