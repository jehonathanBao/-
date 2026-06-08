use serde::{Deserialize, Serialize};

use crate::contract_whale_monitor::types::{
    ContractWhaleDirection, ContractWhaleSeverity, ContractWhaleSignal, ContractWhaleSignalType,
};

const CWM_WEIGHT: f64 = 0.15;

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CwmRiskContribution {
    pub available: bool,
    pub source: String,
    pub formula: String,
    pub contribution_weight: f64,
    pub score: Option<u8>,
    pub weighted_contribution: f64,
    pub signal_id: Option<String>,
    pub severity: Option<ContractWhaleSeverity>,
    pub signal_type: Option<ContractWhaleSignalType>,
    pub direction: Option<ContractWhaleDirection>,
    pub window_sec: Option<u64>,
    pub data_quality: Option<u8>,
    pub dominance: Option<f64>,
    pub summary: String,
    pub discord_gate_independent: bool,
}

impl CwmRiskContribution {
    pub fn unavailable(symbol: &str) -> Self {
        Self {
            available: false,
            source: "contract_whale_monitor".to_string(),
            formula: formula_label().to_string(),
            contribution_weight: CWM_WEIGHT,
            score: None,
            weighted_contribution: 0.0,
            signal_id: None,
            severity: None,
            signal_type: None,
            direction: None,
            window_sec: None,
            data_quality: None,
            dominance: None,
            summary: format!("No recent CWM signal for {symbol}; existing TOF score kept."),
            discord_gate_independent: true,
        }
    }
}

pub fn build_cwm_risk_contribution(
    symbol: &str,
    signal: Option<&ContractWhaleSignal>,
) -> CwmRiskContribution {
    let Some(signal) = signal else {
        return CwmRiskContribution::unavailable(symbol);
    };
    CwmRiskContribution {
        available: true,
        source: "contract_whale_monitor".to_string(),
        formula: formula_label().to_string(),
        contribution_weight: CWM_WEIGHT,
        score: Some(signal.score),
        weighted_contribution: round2(signal.score as f64 * CWM_WEIGHT),
        signal_id: Some(signal.id.clone()),
        severity: Some(signal.severity),
        signal_type: Some(signal.signal_type),
        direction: Some(signal.direction),
        window_sec: Some(signal.window_sec),
        data_quality: Some(signal.data_quality),
        dominance: Some(round4(signal.dominance)),
        summary: signal.final_result.clone(),
        discord_gate_independent: true,
    }
}

pub fn fused_risk_score_with_cwm(
    spot_risk_score: u8,
    spot_tof_score: f64,
    perp_score: u8,
    cwm_score: Option<u8>,
) -> u8 {
    if let Some(cwm_score) = cwm_score {
        clamp_score(
            0.35 * spot_risk_score as f64
                + 0.25 * spot_tof_score
                + 0.25 * perp_score as f64
                + CWM_WEIGHT * cwm_score as f64,
        )
        .round() as u8
    } else {
        crate::runtime::advanced_tof_metrics::fused_risk_score(
            spot_risk_score,
            spot_tof_score,
            perp_score,
        )
    }
}

fn formula_label() -> &'static str {
    "finalRiskScore = spotRisk*0.35 + TOF-lite*0.25 + perpMetrics*0.25 + CWM*0.15"
}

fn clamp_score(value: f64) -> f64 {
    if value.is_finite() {
        value.clamp(0.0, 100.0)
    } else {
        0.0
    }
}

fn round2(value: f64) -> f64 {
    (value * 100.0).round() / 100.0
}

fn round4(value: f64) -> f64 {
    (value * 10_000.0).round() / 10_000.0
}
